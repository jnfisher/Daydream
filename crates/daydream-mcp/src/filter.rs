//! `list_rows`' `where`: conditions on any column, so an agent finds rows on the server instead
//! of downloading a table to search it.
//!
//! A condition is `{"column": "Mon Type", "op": "=", "value": 33}`. A column name with `*`
//! matches every column it fits, and the condition holds when any of them does
//! (`"Room Item *"` is "the room holds the item"). Conditions all hold together.

use daydream_nmr::Value as Cell;
use daydream_nmr::data::NmrRows;
use serde_json::Value;

use crate::{ToolError, bad};

/// The operators, as the tool's schema lists them.
pub const OPS: &[&str] = &["=", "!=", "<", "<=", ">", ">=", "between", "in", "contains"];

#[derive(Debug)]
enum Test {
    Eq(Value),
    Ne(Value),
    Lt(f64),
    Le(f64),
    Gt(f64),
    Ge(f64),
    Between(f64, f64),
    In(Vec<Value>),
    Contains(String),
}

/// One condition, its columns found.
#[derive(Debug)]
pub struct Cond {
    columns: Vec<usize>,
    test: Test,
}

/// Whether a column name fits a pattern (any case), where `*` stands for one word: a run of
/// characters without a space, so `Room Item *` is `Room Item 3` but not `Room Item 3 QTY`.
fn glob(pattern: &str, name: &str) -> bool {
    fn go(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*', rest)) => (1..=n.len())
                .take_while(|&i| n[i - 1] != ' ')
                .any(|i| go(rest, &n[i..])),
            Some((c, rest)) => n
                .split_first()
                .is_some_and(|(d, n)| c.eq_ignore_ascii_case(d) && go(rest, n)),
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    go(&p, &n)
}

fn number(v: &Value, what: &str) -> Result<f64, ToolError> {
    v.as_f64()
        .ok_or_else(|| bad(format!("{what} needs a number, not {v}")))
}

/// Reads `where` against a table's columns.
pub fn parse(v: Option<&Value>, rows: &NmrRows, table: &str) -> Result<Vec<Cond>, ToolError> {
    let list = match v {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(l)) => l,
        Some(_) => return Err(bad("where is a list of conditions")),
    };
    list.iter()
        .map(|c| {
            let column = c
                .get("column")
                .and_then(Value::as_str)
                .ok_or_else(|| bad("a condition needs column"))?;
            let op = c.get("op").and_then(Value::as_str).unwrap_or("=");
            let value = c.get("value").cloned().unwrap_or(Value::Null);
            let columns: Vec<usize> = rows
                .columns
                .iter()
                .enumerate()
                .filter(|(_, n)| glob(column, n))
                .map(|(i, _)| i)
                .collect();
            if columns.is_empty() {
                return Err(bad(format!("{table} has no column {column}")));
            }
            let what = format!("{column} {op}");
            let test = match op {
                "=" => Test::Eq(value),
                "!=" => Test::Ne(value),
                "<" => Test::Lt(number(&value, &what)?),
                "<=" => Test::Le(number(&value, &what)?),
                ">" => Test::Gt(number(&value, &what)?),
                ">=" => Test::Ge(number(&value, &what)?),
                "between" => match value.as_array().map(Vec::as_slice) {
                    Some([lo, hi]) => Test::Between(number(lo, &what)?, number(hi, &what)?),
                    _ => return Err(bad(format!("{what} needs [low, high]"))),
                },
                "in" => match value {
                    Value::Array(l) => Test::In(l),
                    _ => return Err(bad(format!("{what} needs a list"))),
                },
                "contains" => Test::Contains(
                    value
                        .as_str()
                        .ok_or_else(|| bad(format!("{what} needs text")))?
                        .to_lowercase(),
                ),
                other => {
                    return Err(bad(format!(
                        "no operator {other}: use one of {}",
                        OPS.join(" ")
                    )));
                }
            };
            Ok(Cond { columns, test })
        })
        .collect()
}

fn as_number(c: &Cell) -> Option<f64> {
    match c {
        Cell::Long(n) => Some(f64::from(*n)),
        Cell::Double(d) => Some(*d),
        Cell::Text(s) => s.split('\0').next().unwrap_or("").trim().parse().ok(),
        Cell::Null => None,
    }
}

fn as_text(c: &Cell) -> String {
    match c {
        Cell::Text(s) => s.split('\0').next().unwrap_or("").to_string(),
        Cell::Long(n) => n.to_string(),
        Cell::Double(d) => d.to_string(),
        Cell::Null => String::new(),
    }
}

/// Whether a cell equals a JSON value: numbers by value, text ignoring case and spaces at
/// the ends.
fn equals(c: &Cell, v: &Value) -> bool {
    match v {
        Value::Number(n) => as_number(c).zip(n.as_f64()).is_some_and(|(a, b)| a == b),
        Value::String(s) => as_text(c).trim().eq_ignore_ascii_case(s.trim()),
        Value::Null => matches!(c, Cell::Null),
        _ => false,
    }
}

fn holds(test: &Test, c: &Cell) -> bool {
    let n = || as_number(c);
    match test {
        Test::Eq(v) => equals(c, v),
        Test::Ne(v) => !equals(c, v),
        Test::Lt(x) => n().is_some_and(|a| a < *x),
        Test::Le(x) => n().is_some_and(|a| a <= *x),
        Test::Gt(x) => n().is_some_and(|a| a > *x),
        Test::Ge(x) => n().is_some_and(|a| a >= *x),
        Test::Between(lo, hi) => n().is_some_and(|a| a >= *lo && a <= *hi),
        Test::In(l) => l.iter().any(|v| equals(c, v)),
        Test::Contains(s) => as_text(c).to_lowercase().contains(s),
    }
}

/// Whether a row meets every condition.
pub fn matches(conds: &[Cond], row: &[Cell]) -> bool {
    conds.iter().all(|cond| match cond.test {
        // "not equal" on a wildcard means none of the columns equals it
        Test::Ne(_) => cond.columns.iter().all(|&i| holds(&cond.test, &row[i])),
        _ => cond.columns.iter().any(|&i| holds(&cond.test, &row[i])),
    })
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn a_star_matches_any_run_of_characters() {
        assert!(glob("Room Item *", "Room Item 3"));
        assert!(glob("room item *", "Room Item 12"));
        assert!(!glob("Room Item *", "Room Item 3 QTY"));
        assert!(glob("Room Item * QTY", "Room Item 3 QTY"));
        assert!(glob("Exit *", "Exit 9"));
        assert!(!glob("Exit *", "Exit Type 9"));
        assert!(!glob("Room Item *", "Room Item "));
        assert!(!glob("Name", "Name 2"));
        assert!(glob("Name", "name"));
    }
}
