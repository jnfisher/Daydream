//! Getting an agent's edits in and showing what they do, at a cost in proportion to the change:
//! edits read from a file instead of retyped as tool arguments, `clone` edits that copy a row
//! (and, with `messages`, the messages it uses, its name swapped for the new one), and
//! previews that leave out what an insert leaves blank.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use daydream_nmr::rows::{self, Edit, Preview};
use daydream_nmr::{fields, messages};
use daydream_store::Realm;
use serde_json::{Map, Value, json};

use crate::{ToolError, bad};

/// The largest edits file read.
pub const MAX_FILE: u64 = 32 << 20;

/// A preview showing more changed values than this (an insert's blank columns not counted) is
/// shown as a summary unless asked.
pub const SUMMARY_OVER: usize = 400;

/// Where edits files may be read from: the realm's `.daydream/batches` (made when missing),
/// then the folders the server was given.
pub fn allowed_dirs(realm_dir: &Path, extra: &[PathBuf]) -> Vec<PathBuf> {
    let own = batch_dir(realm_dir);
    let _ = std::fs::create_dir_all(&own);
    std::iter::once(own).chain(extra.iter().cloned()).collect()
}

/// The realm's own folder for edits files.
pub fn batch_dir(realm_dir: &Path) -> PathBuf {
    realm_dir.join(".daydream").join("batches")
}

/// An edits file's contents, if it is a `.json` file inside one of `allowed` (a relative
/// path is taken from the first). Returns its path as found, and the JSON.
pub fn read_file(name: &str, allowed: &[PathBuf]) -> Result<(PathBuf, Value), ToolError> {
    let given = Path::new(name);
    let path = if given.is_absolute() {
        given.to_path_buf()
    } else {
        allowed
            .first()
            .ok_or_else(|| bad("no folder takes edits files"))?
            .join(given)
    };
    if given
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(bad(format!(
            "{name}: give the file's path without .. (it must be in a folder edits files are read from)"
        )));
    }
    if path
        .extension()
        .is_none_or(|x| !x.eq_ignore_ascii_case("json"))
    {
        return Err(bad("an edits file is a .json file"));
    }
    let real = path
        .canonicalize()
        .map_err(|e| bad(format!("{}: {e}", path.display())))?;
    let inside = allowed
        .iter()
        .filter_map(|d| d.canonicalize().ok())
        .any(|d| real.starts_with(d));
    if !inside {
        let dirs: Vec<String> = allowed.iter().map(|d| d.display().to_string()).collect();
        return Err(bad(format!(
            "{} is not in a folder edits files are read from: {}",
            path.display(),
            dirs.join(", ")
        )));
    }
    let len = std::fs::metadata(&real)
        .map_err(|e| ToolError::Io(e.to_string()))?
        .len();
    if len > MAX_FILE {
        return Err(bad(format!(
            "{} is {len} bytes; split it (at most {MAX_FILE})",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(&real).map_err(|e| ToolError::Io(e.to_string()))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| bad(format!("{} is not JSON: {e}", path.display())))?;
    Ok((real, v))
}

/// The edits of a file's JSON: a list of edits, or `{"edits": [...], "changeset": "..."}`.
/// Returns the edits and the file's changeset.
pub fn file_edits(v: Value) -> Result<(Value, Option<Value>), ToolError> {
    match v {
        Value::Array(_) => Ok((v, None)),
        Value::Object(mut o) => {
            let edits = o
                .remove("edits")
                .ok_or_else(|| bad("an edits file is a list of edits, or an object with edits"))?;
            Ok((edits, o.remove("changeset")))
        }
        _ => Err(bad(
            "an edits file is a list of edits, or an object with edits",
        )),
    }
}

/// Turns the agent's edits into the library's, writing out each `clone` as the insert it
/// stands for: the `from` row's values with `values` over them, after the inserts of the
/// messages it copies.
pub fn expand(realm: &Realm, edits: &Value) -> Result<Vec<Edit>, ToolError> {
    let list = edits
        .as_array()
        .ok_or_else(|| bad("edits must be a list"))?;
    let mut copies = Copies::new(list);
    let mut out = Vec::with_capacity(list.len());
    for (i, e) in list.iter().enumerate() {
        let at = |msg: String| bad(format!("edit {i}: {msg}"));
        if e.get("op").and_then(Value::as_str) == Some("clone") {
            let insert = clone_edit(realm, e).map_err(|err| at(err.to_string()))?;
            let made =
                copy_messages(realm, e, insert, &mut copies).map_err(|err| at(err.to_string()))?;
            out.extend(made);
        } else {
            out.push(serde_json::from_value(e.clone()).map_err(|err| at(err.to_string()))?);
        }
    }
    if out.is_empty() {
        return Err(bad("edits is empty"));
    }
    Ok(out)
}

fn clone_edit(realm: &Realm, e: &Value) -> Result<Edit, ToolError> {
    let table = e
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("a clone needs table"))?;
    let t = rows::record_table(table)?;
    let from = e
        .get("from")
        .ok_or_else(|| bad("a clone needs from: the key of the row to copy"))?;
    let key = rows::parse_key(t, from)?;
    let (_, all) = rows::load(realm, t)?;
    let index = rows::index_columns(t);
    let row = rows::find_row(&all, &index, &key)
        .ok_or_else(|| bad(format!("{} {} not found", t.name, rows::key_display(&key))))?;
    let mut values = rows::row_json(&all, row);
    let overrides = match e.get("values") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(o)) => o.clone(),
        Some(_) => return Err(bad("values must be an object")),
    };
    for c in &index {
        if !overrides.keys().any(|k| k.eq_ignore_ascii_case(c)) {
            return Err(bad(format!("a clone needs the new row's {c} in values")));
        }
    }
    for (k, v) in overrides {
        let name = values
            .keys()
            .find(|c| c.eq_ignore_ascii_case(&k))
            .cloned()
            .ok_or_else(|| bad(format!("{} has no column {k}", t.name)))?;
        values.insert(name, v);
    }
    Ok(Edit::Insert {
        table: t.name.to_string(),
        values,
    })
}

/// The message numbers a batch's clones copy into: above the realm's highest and any the
/// batch's own inserts take.
struct Copies {
    /// Numbers the batch's inserts take.
    taken: HashSet<i64>,
    /// The realm's messages, by number (read when a clone first copies one).
    lines: Option<HashMap<i64, [String; 3]>>,
    /// The next number to give.
    next: i64,
}

impl Copies {
    fn new(list: &[Value]) -> Copies {
        let taken = list
            .iter()
            .filter(|e| {
                e.get("op").and_then(Value::as_str) == Some("insert")
                    && e.get("table")
                        .and_then(Value::as_str)
                        .is_some_and(|t| t.eq_ignore_ascii_case("Messages"))
            })
            .filter_map(|e| {
                e.get("values")?
                    .as_object()?
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("Number"))?
                    .1
                    .as_i64()
            })
            .collect();
        Copies {
            taken,
            lines: None,
            next: 0,
        }
    }

    /// The realm's messages, read once.
    fn lines(&mut self, realm: &Realm) -> Result<&HashMap<i64, [String; 3]>, ToolError> {
        if self.lines.is_none() {
            let t = rows::record_table("Messages")?;
            let (_, all) = rows::load(realm, t)?;
            let at = |c: &str| all.position(c);
            let (n, l1, l2, l3) = (at("Number"), at("Line 1"), at("Line 2"), at("Line 3"));
            let text = |row: &[daydream_nmr::Value], i: Option<usize>| match i.map(|i| &row[i]) {
                Some(daydream_nmr::Value::Text(s)) => {
                    s.split('\0').next().unwrap_or("").to_string()
                }
                _ => String::new(),
            };
            let mut map = HashMap::new();
            for row in &all.rows {
                if let Some(num) = n.and_then(|i| daydream_nmr::refs::num(&row[i])) {
                    map.insert(num, [text(row, l1), text(row, l2), text(row, l3)]);
                }
            }
            let top = map.keys().chain(&self.taken).copied().max().unwrap_or(0);
            self.next = top + 1;
            self.lines = Some(map);
        }
        Ok(self.lines.as_ref().expect("read above"))
    }

    /// A number no message has and no insert of the batch takes.
    fn take(&mut self) -> i64 {
        while self.taken.contains(&self.next) {
            self.next += 1;
        }
        self.taken.insert(self.next);
        self.next += 1;
        self.next - 1
    }
}

/// A clone with `messages`: copies of the messages it uses as inserts, each line with the
/// copied row's name swapped for the clone's, then the clone pointing at them. `named` copies
/// the messages whose lines name the row, `all` every one; a message several columns use is
/// copied once.
fn copy_messages(
    realm: &Realm,
    e: &Value,
    insert: Edit,
    copies: &mut Copies,
) -> Result<Vec<Edit>, ToolError> {
    let all = match e.get("messages") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => return Ok(vec![insert]),
        Some(Value::Bool(true)) => false,
        Some(v) if v.as_str() == Some("named") => false,
        Some(v) if v.as_str() == Some("all") => true,
        Some(_) => return Err(bad("messages is named or all")),
    };
    let Edit::Insert { table, mut values } = insert else {
        return Ok(vec![insert]);
    };
    let t = rows::record_table(&table)?;
    // the copied row's name, and the clone's
    let from = rows::parse_key(t, e.get("from").unwrap_or(&Value::Null))?;
    let (_, rows_of) = rows::load(realm, t)?;
    let old_name = rows::find_row(&rows_of, &rows::index_columns(t), &from)
        .and_then(|row| rows_of.position("Name").map(|i| &row[i]))
        .and_then(|v| match v {
            daydream_nmr::Value::Text(s) => {
                Some(s.split('\0').next().unwrap_or("").trim().to_string())
            }
            _ => None,
        })
        .unwrap_or_default();
    let new_name = values
        .get("Name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    // the columns the agent set itself keep what it set
    let given: HashSet<String> = e
        .get("values")
        .and_then(Value::as_object)
        .map(|o| o.keys().map(|k| k.to_ascii_lowercase()).collect())
        .unwrap_or_default();
    let cols: Vec<&str> = t
        .columns
        .iter()
        .map(|c| c.name)
        .filter(|c| {
            fields::column(t.name, c).and_then(|i| i.target()) == Some("Messages")
                && !given.contains(&c.to_ascii_lowercase())
        })
        .collect();
    let mut copied: HashMap<i64, i64> = HashMap::new();
    let mut out = Vec::new();
    for c in cols {
        let Some(m) = values.get(c).and_then(Value::as_i64).filter(|m| *m > 0) else {
            continue;
        };
        if let Some(&n) = copied.get(&m) {
            values.insert(c.to_string(), json!(n));
            continue;
        }
        let Some(lines) = copies.lines(realm)?.get(&m).cloned() else {
            continue;
        };
        let swapped = lines.map(|l| messages::swap_name(&l, &old_name, &new_name));
        if !all && swapped.iter().all(|(_, n)| *n == 0) {
            continue;
        }
        let n = copies.take();
        copied.insert(m, n);
        values.insert(c.to_string(), json!(n));
        let [l1, l2, l3] = swapped.map(|(l, _)| l);
        out.push(Edit::Insert {
            table: "Messages".into(),
            values: Map::from_iter([
                ("Number".to_string(), json!(n)),
                ("Line 1".to_string(), json!(l1)),
                ("Line 2".to_string(), json!(l2)),
                ("Line 3".to_string(), json!(l3)),
            ]),
        });
    }
    out.push(Edit::Insert { table, values });
    Ok(out)
}

/// Whether a value is what a new row starts with in that column.
fn is_blank(table: &str, column: &str, v: &daydream_nmr::Value) -> bool {
    let Ok(t) = rows::record_table(table) else {
        return false;
    };
    t.columns
        .iter()
        .find(|c| c.name == column)
        .is_some_and(|c| rows::blank(c.ty) == *v)
}

/// How much of a preview to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Rows per table and operation, the first few rows, and any warnings.
    Summary,
    /// Every changed value, leaving out the columns an insert leaves blank.
    Changes,
    /// Every changed value.
    Full,
}

impl Detail {
    pub fn parse(v: Option<&Value>) -> Result<Option<Self>, ToolError> {
        Ok(match v.and_then(Value::as_str) {
            None => None,
            Some("summary") => Some(Self::Summary),
            Some("changes") => Some(Self::Changes),
            Some("full") => Some(Self::Full),
            Some(other) => {
                return Err(bad(format!(
                    "detail is summary, changes or full, not {other}"
                )));
            }
        })
    }
}

/// One row's preview, as JSON.
fn preview_json(p: &Preview, full: bool) -> Value {
    let insert = p
        .changes
        .iter()
        .all(|c| c.before == daydream_nmr::Value::Null);
    let mut blank = 0;
    let changes: Vec<Value> = p
        .changes
        .iter()
        .filter(|c| {
            let skip = !full && insert && is_blank(&p.table, &c.column, &c.after);
            blank += usize::from(skip);
            !skip
        })
        .map(|c| {
            json!({
                "column": c.column,
                "before": rows::cell_json(&c.before),
                "after": rows::cell_json(&c.after),
            })
        })
        .collect();
    let key: Vec<Value> = p.key.iter().map(rows::cell_json).collect();
    let mut v = json!({ "table": p.table, "key": key, "changes": changes });
    if blank > 0 {
        v["blank_columns"] = json!(blank);
    }
    v
}

/// What an edit does to its row, in a word.
pub fn op(e: &Edit) -> &'static str {
    match e {
        Edit::Insert { .. } => "insert",
        Edit::Update { .. } => "update",
        Edit::Delete { .. } => "delete",
    }
}

/// Rows per table and operation (`{"Rooms": {"insert": 35}}`).
pub fn count(edits: &[Edit]) -> Value {
    let mut out = Map::new();
    for e in edits {
        let t = out
            .entry(e.table().to_string())
            .or_insert_with(|| json!({}));
        t[op(e)] = json!(t[op(e)].as_u64().unwrap_or(0) + 1);
    }
    Value::Object(out)
}

/// The preview's body (without the token): its rows as `detail` asks, or by size when not
/// asked.
pub fn shown(edits: &[Edit], previews: &[Preview], detail: Option<Detail>) -> Value {
    let values: usize = previews.iter().map(|p| p.changes.len()).sum();
    let set: usize = previews
        .iter()
        .flat_map(|p| p.changes.iter().map(move |c| (p, c)))
        .filter(|(p, c)| {
            c.before != daydream_nmr::Value::Null || !is_blank(&p.table, &c.column, &c.after)
        })
        .count();
    let detail = detail.unwrap_or(if set > SUMMARY_OVER {
        Detail::Summary
    } else {
        Detail::Changes
    });
    let full = detail == Detail::Full;
    let mut v = json!({ "rows": count(edits), "values_changed": values });
    match detail {
        Detail::Summary => {
            v["detail"] = json!("summary");
            v["first"] = previews
                .iter()
                .take(3)
                .map(|p| preview_json(p, false))
                .collect();
            v["note"] = json!(format!(
                "{} rows; showing the first {}. Call again with detail \"changes\" to see every row.",
                previews.len(),
                previews.len().min(3)
            ));
        }
        Detail::Changes | Detail::Full => {
            v["detail"] = json!(if full { "full" } else { "changes" });
            v["edits"] = previews.iter().map(|p| preview_json(p, full)).collect();
        }
    }
    v
}
