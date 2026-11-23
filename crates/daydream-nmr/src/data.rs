//! The contents of an NMR `.mdb`, independent of how it is stored.
//!
//! [`NmrData`] is what [`crate::export`] makes from a realm, what [`crate::import`] applies to
//! one, what [`crate::mdb::read`] reads from a `.mdb`, and (as JSON) what the Windows Jet
//! writer turns into a `.mdb`.

use serde::{Deserialize, Serialize};

use crate::schema::{self, Value};

/// Every table's rows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NmrData {
    pub tables: Vec<NmrRows>,
}

/// One table: its column names and rows, each row one value per column.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NmrRows {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl NmrData {
    /// The table with this name (any case).
    pub fn table(&self, name: &str) -> Option<&NmrRows> {
        self.tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// The table with this name, mutable.
    pub fn table_mut(&mut self, name: &str) -> Option<&mut NmrRows> {
        self.tables
            .iter_mut()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }
}

impl NmrRows {
    /// Where a column is in each row (any case).
    pub fn position(&self, column: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.eq_ignore_ascii_case(column))
    }

    /// The schema of this table, when NMR has one by its name.
    pub fn schema(&self) -> Option<&'static schema::NmrTable> {
        schema::table(&self.name)
    }
}
