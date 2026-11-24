//! What Daydream knows of NMR's columns beyond their names and types: the choice lists and
//! links the editors pick from, and notes on what the game does with a value
//! (`fields.json`, which the web editors read too).
//!
//! A column is described by its own name, or by its family's template: the name with its
//! number written `#` (`Room Item #` for `Room Item 3`, `Room Item # QTY` for
//! `Room Item 3 QTY`).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// The file, as the web bundle embeds it too.
pub const JSON: &str = include_str!("fields.json");

/// A column's description. Every part is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnInfo {
    /// The choice list its values are labelled from (a key of [`Fields::lists`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    /// The table whose record number it holds (0 is none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// It holds an ability number.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub abilities: bool,
    /// Another table it points at, that the editors do not pick from (`Rooms`: a room on
    /// the same map).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    /// What the game does with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Where the note was read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl ColumnInfo {
    /// The table a value of this column names, if any.
    pub fn target(&self) -> Option<&str> {
        self.r#ref.as_deref().or(self.links.as_deref())
    }
}

/// A table's description.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TableInfo {
    #[serde(default)]
    pub note: Option<String>,
    /// By column name or template.
    #[serde(default)]
    pub columns: BTreeMap<String, ColumnInfo>,
}

/// One parameter of an exit type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitParam {
    pub means: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExitParams {
    pub source: String,
    /// By exit type number, then `Para1` to `Para4`.
    pub types: BTreeMap<String, BTreeMap<String, ExitParam>>,
}

/// An ability's name and NMR's note on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ability {
    pub name: String,
    pub note: String,
}

/// The whole of `fields.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Fields {
    pub about: String,
    /// Choice lists: value to label.
    pub lists: BTreeMap<String, BTreeMap<String, String>>,
    pub exit_params: ExitParams,
    pub tables: BTreeMap<String, TableInfo>,
    pub abilities: BTreeMap<String, Ability>,
}

static FIELDS: LazyLock<Fields> =
    LazyLock::new(|| serde_json::from_str(JSON).expect("fields.json is valid"));

/// The descriptions.
pub fn fields() -> &'static Fields {
    &FIELDS
}

/// A column's family template: its last run of digits written `#` (`None` without one), as
/// the web editors group numbered columns (`Para1 3` is `Para1 #`).
pub fn template(column: &str) -> Option<String> {
    let end = column.rfind(|c: char| c.is_ascii_digit())? + 1;
    let start = column[..end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |s| s + 1);
    Some(format!("{}#{}", &column[..start], &column[end..]))
}

/// A column's description, by its name or its family's template.
pub fn column(table: &str, column: &str) -> Option<&'static ColumnInfo> {
    let t = FIELDS.tables.get(table)?;
    t.columns
        .get(column)
        .or_else(|| template(column).and_then(|tpl| t.columns.get(&tpl)))
}

/// A table's note.
pub fn table_note(table: &str) -> Option<&'static str> {
    FIELDS.tables.get(table)?.note.as_deref()
}

/// The label of `value` in the list `list`.
pub fn label(list: &str, value: i64) -> Option<&'static str> {
    FIELDS
        .lists
        .get(list)?
        .get(&value.to_string())
        .map(String::as_str)
}

/// What the parameters of exit type `kind` mean.
pub fn exit_params(kind: i64) -> Option<&'static BTreeMap<String, ExitParam>> {
    FIELDS.exit_params.types.get(&kind.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_reads_and_names_real_columns() {
        let f = fields();
        assert!(f.lists.contains_key("ROOM_TYPE"));
        assert_eq!(label("ROOM_TYPE", 2), Some("Arena"));
        assert_eq!(f.abilities["1"].name, "Damage");
        // every described column is a column of its table (or a family of them)
        for (table, info) in &f.tables {
            let t = crate::schema::table(table)
                .or_else(|| crate::players::TABLES.iter().find(|t| t.name == table))
                .unwrap_or_else(|| panic!("no table {table}"));
            for (name, c) in &info.columns {
                let found = t
                    .columns
                    .iter()
                    .any(|col| col.name == name || template(col.name).as_deref() == Some(name));
                assert!(found, "{table} has no column {name}");
                if let Some(list) = &c.options {
                    assert!(f.lists.contains_key(list), "{table}.{name}: no list {list}");
                }
                if let Some(target) = c.target() {
                    assert!(
                        crate::schema::table(target).is_some(),
                        "{table}.{name}: {target}"
                    );
                }
            }
        }
        for params in f.exit_params.types.values() {
            for p in params.values() {
                if let Some(target) = &p.links {
                    assert!(crate::schema::table(target).is_some(), "{target}");
                }
            }
        }
    }

    #[test]
    fn a_numbered_column_finds_its_family() {
        assert_eq!(
            template("Room Item 12 QTY").as_deref(),
            Some("Room Item # QTY")
        );
        assert_eq!(template("Name"), None);
        assert_eq!(template("Para1 3").as_deref(), Some("Para1 #"));
        assert_eq!(
            column("Rooms", "Room Item 3").and_then(|c| c.r#ref.as_deref()),
            Some("Items")
        );
        assert_eq!(
            column("Rooms", "Exit 9").and_then(|c| c.links.as_deref()),
            Some("Rooms")
        );
        assert_eq!(
            column("Rooms", "Type").and_then(|c| c.options.as_deref()),
            Some("ROOM_TYPE")
        );
        assert_eq!(
            column("Rooms", "Type 4").and_then(|c| c.options.as_deref()),
            Some("EXIT_TYPE")
        );
        assert_eq!(
            exit_params(2).unwrap()["Para1"].links.as_deref(),
            Some("Items")
        );
    }
}
