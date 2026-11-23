//! NMR's Universal Modifier: one change applied to many records at once, planned first so
//! every changed value can be shown before anything is written.
//!
//! A [`Modify`] names a table, an [`Action`], an optional range of records and "only if"
//! [`Condition`]s, as NMR's form does. [`plan`] works out every change from the realm's
//! rows without writing; [`apply`] writes a plan's rows through the strict import, so only
//! the changed values reach the records.
//!
//! The arithmetic is NMR's `DoMath`: `+ - * /`, `%` (the value times percent / 100) and
//! `=`; then the limit ("Limit to <= n" or ">= n"); then VB's `Round`, which rounds halves to
//! even; then the value is held inside its element's type (a 16-bit Integer, a 32-bit
//! Long, a Byte, or an unsigned 32-bit value), as NMR clamps it.

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::data::{NmrData, NmrRows};
use crate::export::export_table;
use crate::import::{ImportError, ImportOptions, Range, TableReport, import};
use crate::schema::{self, NmrTable, Source, Value, Vb};

/// Arithmetic of the modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    /// The value times `value` percent.
    Percent,
    Set,
}

/// A comparison of the "only if" and limit boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cmp {
    Eq,
    Le,
    Ge,
    Ne,
}

impl Cmp {
    fn test(self, a: f64, b: f64) -> bool {
        match self {
            Cmp::Eq => a == b,
            Cmp::Le => a <= b,
            Cmp::Ge => a >= b,
            Cmp::Ne => a != b,
        }
    }
}

/// What to do to each record.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// `columns op value` (one column, or several, such as every `Item Drop % n`), then held
    /// under or over `limit`.
    Math {
        columns: Vec<String>,
        op: Op,
        value: f64,
        #[serde(default)]
        limit: Option<(Cmp, f64)>,
    },
    /// Put `ability` with `value` in the first empty ability slot, unless a slot already
    /// has that ability with that value.
    GiveAbility { ability: i64, value: i64 },
    /// Empty every slot holding `ability` (NMR leaves the value; `clear_value` zeroes it too).
    TakeAbility {
        ability: i64,
        #[serde(default)]
        clear_value: bool,
    },
    /// Apply the arithmetic to the value of every slot holding `ability`.
    ChangeAbility {
        ability: i64,
        op: Op,
        value: f64,
        #[serde(default)]
        limit: Option<(Cmp, f64)>,
    },
    /// Set each column to its value: NMR's Mass Room Editor (and any table's). Text and
    /// numbers alike; a key column cannot be set.
    Set { values: Vec<(String, Value)> },
}

/// An "only if" box.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    /// `column cmp value`.
    Column {
        column: String,
        cmp: Cmp,
        value: f64,
    },
    /// Some slot holds `ability` with a value that passes `cmp value`.
    HasAbility { ability: i64, cmp: Cmp, value: f64 },
    /// No slot does.
    LacksAbility { ability: i64, cmp: Cmp, value: f64 },
}

/// One run of the modifier.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Modify {
    pub table: String,
    pub action: Action,
    #[serde(default)]
    pub range: Option<Range>,
    #[serde(default)]
    pub only_if: Vec<Condition>,
}

/// One changed value.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Change {
    pub key: Vec<Value>,
    pub column: String,
    pub before: Value,
    pub after: Value,
}

/// What a run would do.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Plan {
    pub table: String,
    /// Every value that changes.
    pub changes: Vec<Change>,
    /// Records in range the "only if" boxes left out.
    pub skipped: usize,
    /// Records in range the action changed nothing in.
    pub unchanged: usize,
    /// Things worth a look that the run could not change, as NMR logs them.
    pub notes: Vec<String>,
    /// The changed and new rows, whole, for [`apply`].
    #[serde(skip)]
    pub(crate) rows: Option<NmrRows>,
    /// The keys of records to delete.
    #[serde(skip)]
    pub(crate) deletes: Vec<Vec<Value>>,
}

impl Plan {
    /// How many records change, are added or go.
    pub fn records(&self) -> usize {
        self.rows.as_ref().map_or(0, |r| r.rows.len()) + self.deletes.len()
    }
}

/// Why a run cannot be planned.
#[derive(Debug, thiserror::Error)]
pub enum ModifyError {
    #[error("no table {0}")]
    NoTable(String),
    #[error("{table} has no column {column}")]
    NoColumn { table: String, column: String },
    #[error("{table}.{column} is not a number")]
    NotNumber { table: String, column: String },
    #[error("{0} has no abilities")]
    NoAbilities(String),
    #[error("{0} is part of the record's key and cannot be set")]
    KeyColumn(String),
    #[error("can't divide by zero")]
    DivideByZero,
    #[error(transparent)]
    Export(#[from] crate::export::ExportError),
    #[error(transparent)]
    Import(#[from] ImportError),
}

pub(crate) fn num(v: &Value) -> f64 {
    schema::number(v).unwrap_or(0.0)
}

/// The range a column's element holds, as NMR clamps to it.
pub(crate) fn bounds(t: &NmrTable, column: &str) -> Option<(f64, f64)> {
    let c = t.column(column)?;
    Some(match &c.source {
        Source::Element {
            vb: Vb::Integer, ..
        } => (f64::from(i16::MIN), f64::from(i16::MAX)),
        Source::Element { vb: Vb::Long, .. } => (f64::from(i32::MIN), f64::from(i32::MAX)),
        Source::Element { vb: Vb::Byte, .. } => (0.0, 255.0),
        Source::Unsigned(_) => (0.0, f64::from(u32::MAX)),
        _ => return None,
    })
}

/// NMR's `DoMath`.
fn math(value: f64, op: Op, by: f64, limit: Option<(Cmp, f64)>, (lo, hi): (f64, f64)) -> f64 {
    let mut out = match op {
        Op::Add => value + by,
        Op::Sub => value - by,
        Op::Mul => value * by,
        Op::Div => value / by,
        Op::Percent => value * (by / 100.0),
        Op::Set => by,
    };
    match limit {
        Some((Cmp::Le, cap)) if out > cap => out = cap,
        Some((Cmp::Ge, floor)) if out < floor => out = floor,
        _ => {}
    }
    out.round_ties_even().clamp(lo, hi)
}

/// A number in the cell's own form: a Double column stays a Double.
pub(crate) fn cell(like: &Value, n: f64) -> Value {
    match like {
        Value::Double(_) => Value::Double(n),
        _ => Value::Long(n as i32),
    }
}

/// The ability slots of a table: (ability column, value column) positions.
pub(crate) fn slots(rows: &NmrRows) -> Vec<(usize, usize)> {
    (0..)
        .map_while(|i| {
            Some((
                rows.position(&format!("Ability {i}"))?,
                rows.position(&format!("Ability Value {i}"))?,
            ))
        })
        .collect()
}

fn in_range(t: &NmrTable, rows: &NmrRows, row: &[Value], range: &Range) -> bool {
    let index = t.index.as_ref().map(|i| i.columns).unwrap_or(&[]);
    let at = |i: usize| {
        index
            .get(i)
            .and_then(|c| rows.position(c))
            .map(|p| num(&row[p]) as i64)
    };
    let (map, number) = if t.table == Some(daydream_wcc::tables::Table::Rooms) {
        (at(0), at(1).unwrap_or(0))
    } else {
        (None, at(0).unwrap_or(0))
    };
    range.map.is_none_or(|m| Some(m) == map) && (range.from..=range.to).contains(&number)
}

fn passes(rows: &NmrRows, slots: &[(usize, usize)], row: &[Value], c: &Condition) -> bool {
    match c {
        Condition::Column { column, cmp, value } => rows
            .position(column)
            .is_some_and(|i| cmp.test(num(&row[i]), *value)),
        Condition::HasAbility {
            ability,
            cmp,
            value,
        } => slots
            .iter()
            .any(|&(a, v)| num(&row[a]) as i64 == *ability && cmp.test(num(&row[v]), *value)),
        Condition::LacksAbility {
            ability,
            cmp,
            value,
        } => !slots
            .iter()
            .any(|&(a, v)| num(&row[a]) as i64 == *ability && cmp.test(num(&row[v]), *value)),
    }
}

/// Works out every change `m` makes to the realm, writing nothing.
pub fn plan(realm: &Realm, m: &Modify, layout: LayoutSet) -> Result<Plan, ModifyError> {
    let t = schema::table(&m.table)
        .filter(|t| t.table.is_some())
        .ok_or_else(|| ModifyError::NoTable(m.table.clone()))?;
    let rows = export_table(realm, t, layout)?;
    let slots = slots(&rows);
    let no_column = |column: &str| ModifyError::NoColumn {
        table: t.name.to_string(),
        column: column.to_string(),
    };
    // check the action and the boxes before touching a row
    match &m.action {
        Action::Math {
            columns, op, value, ..
        } => {
            for c in columns {
                rows.position(c).ok_or_else(|| no_column(c))?;
                bounds(t, c).ok_or_else(|| ModifyError::NotNumber {
                    table: t.name.to_string(),
                    column: c.clone(),
                })?;
            }
            if *op == Op::Div && *value == 0.0 {
                return Err(ModifyError::DivideByZero);
            }
        }
        Action::ChangeAbility { op, value, .. } if *op == Op::Div && *value == 0.0 => {
            return Err(ModifyError::DivideByZero);
        }
        Action::Set { values } => {
            let keys = t.index.as_ref().map(|i| i.columns).unwrap_or(&[]);
            for (c, v) in values {
                let col = t.column(c).ok_or_else(|| no_column(c))?;
                if keys.contains(&c.as_str()) {
                    return Err(ModifyError::KeyColumn(c.clone()));
                }
                let text = matches!(
                    col.ty,
                    schema::ColumnType::Text(_) | schema::ColumnType::Memo
                );
                if !text && schema::number(v).is_err() {
                    return Err(ModifyError::NotNumber {
                        table: t.name.to_string(),
                        column: c.clone(),
                    });
                }
            }
        }
        _ if slots.is_empty() => return Err(ModifyError::NoAbilities(t.name.to_string())),
        _ => {}
    }
    for c in &m.only_if {
        match c {
            Condition::Column { column, .. } => {
                rows.position(column).ok_or_else(|| no_column(column))?;
            }
            _ if slots.is_empty() => return Err(ModifyError::NoAbilities(t.name.to_string())),
            _ => {}
        }
    }
    let value_bounds = slots
        .first()
        .and_then(|&(_, v)| bounds(t, &rows.columns[v]))
        .unwrap_or((f64::from(i16::MIN), f64::from(i16::MAX)));
    let in_scope = |row: &[Value]| m.range.as_ref().is_none_or(|r| in_range(t, &rows, row, r));
    let only_if = |row: &[Value]| m.only_if.iter().all(|c| passes(&rows, &slots, row, c));
    let each = |row: &[Value]| -> Step {
        if !in_scope(row) {
            return Step::Out;
        }
        if !only_if(row) {
            return Step::Skip;
        }
        let mut new = row.to_vec();
        match &m.action {
            Action::Math {
                columns,
                op,
                value,
                limit,
            } => {
                for c in columns {
                    let i = rows.position(c).expect("checked");
                    let n = math(
                        num(&row[i]),
                        *op,
                        *value,
                        *limit,
                        bounds(t, c).expect("checked"),
                    );
                    new[i] = cell(&row[i], n);
                }
            }
            Action::GiveAbility { ability, value } => {
                let has = slots.iter().any(|&(a, v)| {
                    num(&row[a]) as i64 == *ability && num(&row[v]) as i64 == *value
                });
                if !has && let Some(&(a, v)) = slots.iter().find(|&&(a, _)| num(&row[a]) == 0.0) {
                    new[a] = cell(&row[a], *ability as f64);
                    new[v] = cell(&row[v], *value as f64);
                }
            }
            Action::TakeAbility {
                ability,
                clear_value,
            } => {
                for &(a, v) in &slots {
                    if num(&row[a]) as i64 == *ability {
                        new[a] = cell(&row[a], 0.0);
                        if *clear_value {
                            new[v] = cell(&row[v], 0.0);
                        }
                    }
                }
            }
            Action::ChangeAbility {
                ability,
                op,
                value,
                limit,
            } => {
                for &(a, v) in &slots {
                    if num(&row[a]) as i64 == *ability {
                        new[v] = cell(
                            &row[v],
                            math(num(&row[v]), *op, *value, *limit, value_bounds),
                        );
                    }
                }
            }
            Action::Set { values } => {
                for (c, v) in values {
                    let i = rows.position(c).expect("checked");
                    new[i] = match (v, &row[i]) {
                        // the same text up to the record's NUL is no change
                        (Value::Text(to), Value::Text(was))
                            if was.split('\0').next() == Some(to) =>
                        {
                            row[i].clone()
                        }
                        (Value::Text(_) | Value::Null, _) => v.clone(),
                        _ => cell(&row[i], num(v)),
                    };
                }
            }
        }
        Step::Row(new)
    };
    Ok(collect(t, &rows, each))
}

/// What a run does with one record.
pub(crate) enum Step {
    /// Outside the run's range: not counted.
    Out,
    /// Left out by an "only if" box.
    Skip,
    /// The record as the run leaves it.
    Row(Vec<Value>),
}

/// Runs `each` over every row and gathers the changes into a plan.
pub(crate) fn collect(
    t: &NmrTable,
    rows: &NmrRows,
    mut each: impl FnMut(&[Value]) -> Step,
) -> Plan {
    let index: Vec<usize> = t
        .index
        .as_ref()
        .map(|i| i.columns.iter().filter_map(|c| rows.position(c)).collect())
        .unwrap_or_default();
    let mut plan = Plan {
        table: t.name.to_string(),
        ..Plan::default()
    };
    let mut changed_rows = Vec::new();
    for row in &rows.rows {
        let new = match each(row) {
            Step::Out => continue,
            Step::Skip => {
                plan.skipped += 1;
                continue;
            }
            Step::Row(new) => new,
        };
        let key: Vec<Value> = index.iter().map(|&i| row[i].clone()).collect();
        let before = plan.changes.len();
        for (i, (a, b)) in row.iter().zip(&new).enumerate() {
            if a != b {
                plan.changes.push(Change {
                    key: key.clone(),
                    column: rows.columns[i].clone(),
                    before: a.clone(),
                    after: b.clone(),
                });
            }
        }
        if plan.changes.len() == before {
            plan.unchanged += 1;
        } else {
            changed_rows.push(new);
        }
    }
    plan.rows = Some(NmrRows {
        name: rows.name.clone(),
        columns: rows.columns.clone(),
        rows: changed_rows,
    });
    plan
}

/// Writes a plan's changed rows to the realm through the strict import.
pub fn apply(realm: &Realm, plan: &Plan, layout: LayoutSet) -> Result<TableReport, ModifyError> {
    let mut report = match &plan.rows {
        Some(rows) if !rows.rows.is_empty() => {
            let data = NmrData {
                tables: vec![rows.clone()],
            };
            let opts = ImportOptions {
                layout,
                ..ImportOptions::default()
            };
            import(realm, &data, &opts)?
                .into_iter()
                .next()
                .unwrap_or_default()
        }
        _ => TableReport {
            table: plan.table.clone(),
            ..TableReport::default()
        },
    };
    if !plan.deletes.is_empty() {
        let d = crate::import::delete(realm, &plan.table, &plan.deletes, layout)?;
        report.deleted += d.deleted;
        report.errors.extend(d.errors);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_is_nmrs() {
        let int = (f64::from(i16::MIN), f64::from(i16::MAX));
        assert_eq!(math(10.0, Op::Add, 5.0, None, int), 15.0);
        assert_eq!(math(10.0, Op::Percent, 150.0, None, int), 15.0);
        // VB's Round: halves to even
        assert_eq!(math(5.0, Op::Div, 2.0, None, int), 2.0);
        assert_eq!(math(7.0, Op::Div, 2.0, None, int), 4.0);
        assert_eq!(
            math(100.0, Op::Mul, 3.0, Some((Cmp::Le, 250.0)), int),
            250.0
        );
        assert_eq!(math(1.0, Op::Sub, 5.0, Some((Cmp::Ge, 0.0)), int), 0.0);
        assert_eq!(math(30000.0, Op::Mul, 2.0, None, int), 32767.0);
        assert_eq!(math(3.0, Op::Sub, 9.0, None, (0.0, 255.0)), 0.0);
    }
}
