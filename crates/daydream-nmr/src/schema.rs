//! The shape of NMR's `.mdb` and where each column's value lives in a record.
//!
//! [`crate::columns::TABLES`] is generated from Nightmare Redux's own source
//! (`tools/nmr/generate.sh`): the tables and columns of `CreateAccessTables`, and for each
//! column the record element NMR's Access export writes into it. NMR edits the Worldgroup 3
//! build's files, so its elements are spans of the aligned (`WG_*`) layouts; a [`Piece`] names
//! the `daydream-types` member a span falls in, so the same value is found in a realm of
//! either layout. A few NMR elements cover two members and the alignment padding between
//! them (`WornOn` is `worn` plus the pad byte after it); padding has no bytes in the DOS
//! layout and reads as zero.

use daydream_types::records::{Layout, LayoutSet};
use daydream_wcc::tables::Table;

use crate::text;

/// One table of NMR's `.mdb`.
#[derive(Debug)]
pub struct NmrTable {
    /// The Access table name.
    pub name: &'static str,
    /// The WCC record type its rows are, or `None` for `Info`.
    pub table: Option<Table>,
    /// The index NMR's import seeks by (a primary key, or `idxRooms` and `idxTextblocks`).
    pub index: Option<Index>,
    /// Every column, in `CreateAccessTables` order.
    pub columns: &'static [Column],
}

/// A named index over one or more columns.
#[derive(Debug)]
pub struct Index {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    /// A primary key (unique); otherwise a plain index that allows duplicates.
    pub primary: bool,
}

/// One column of a table.
#[derive(Debug)]
pub struct Column {
    pub name: &'static str,
    pub ty: ColumnType,
    /// NMR creates it only under an option (`Exp Multiplier`); a file may lack it.
    pub optional: bool,
    pub source: Source,
}

/// An Access column type, as NMR creates it through ADOX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    /// `adInteger`: Long Integer.
    Long,
    /// `adDouble`.
    Double,
    /// `adVarWChar(n)`: Text of at most `n` characters.
    Text(u16),
    /// `adLongVarWChar`: Memo.
    Memo,
}

/// Where a column's value comes from.
#[derive(Debug)]
pub enum Source {
    /// A record element read as NMR's VB type reads it.
    Element { vb: Vb, pieces: &'static [Piece] },
    /// A 32-bit element NMR writes unsigned, as a Double (`SLong2ULong`).
    Unsigned(&'static [Piece]),
    /// Always 0 (`CurrentRoomMon`, which NMR never exports).
    Zero,
    /// A text block's text, unshifted (`DecryptTextblock`).
    TextblockData,
    /// The `Info` table's version row.
    Info,
}

/// How NMR's VB struct holds an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vb {
    /// `Long`: signed 32-bit.
    Long,
    /// `Integer`: signed 16-bit.
    Integer,
    /// `Byte`: unsigned 8-bit.
    Byte,
    /// `String * n`: every byte of the span, NULs included, as Windows-1252 characters.
    Text,
}

/// Bytes `at..at + len` of a record member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub member: &'static str,
    pub at: usize,
    pub len: usize,
}

/// Shorthand for the generated table.
pub const fn p(member: &'static str, at: usize, len: usize) -> Piece {
    Piece { member, at, len }
}

/// A value of one cell.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Long(i32),
    Double(f64),
    Text(String),
}

/// Why a value cannot go into a record.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ValueError {
    #[error("{value} does not fit NMR's {vb:?} element")]
    OutOfRange { value: f64, vb: Vb },
    #[error("expected a number, found {0:?}")]
    NotNumber(Value),
    #[error("expected text, found {0:?}")]
    NotText(Value),
    #[error("{0}")]
    Text(#[from] text::TextError),
}

impl NmrTable {
    /// The column with this name.
    pub fn column(&self, name: &str) -> Option<&'static Column> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

/// The table with this name (any case).
pub fn table(name: &str) -> Option<&'static NmrTable> {
    crate::columns::TABLES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
}

/// The record layout the WCC table's records are in, for `set`.
pub fn layout_of(table: Table, set: LayoutSet) -> &'static Layout {
    use daydream_types::records::layout as l;
    let dos: &'static Layout = match table {
        Table::Rooms => &l::ROOM,
        Table::Items => &l::ITEM,
        Table::Monsters => &l::MONSTER,
        Table::ActiveMonsters => &l::ACTMON,
        Table::Spells => &l::SPELL,
        Table::Classes => &l::CLASS,
        Table::Races => &l::RACE,
        Table::Shops => &l::SHOP,
        Table::Messages => &l::MUDMSG,
        Table::TextBlocks => &l::TEXTBLK,
        Table::Actions => &l::ACTION,
        Table::Gangs => &l::GANG,
        Table::BankBooks => &l::BANKBOOK,
        Table::ItemOwnership => &l::ITEMOWN,
        Table::Users => &l::PLAYER,
    };
    set.layout(dos)
}

/// Where a piece's bytes are in a record of `layout`, or `None` for padding the layout
/// does not have.
fn span(layout: &Layout, piece: &Piece) -> Option<std::ops::Range<usize>> {
    let f = layout.field(piece.member)?;
    assert!(
        piece.at + piece.len <= f.size,
        "{}.{}: piece {}+{} past the member",
        layout.decoder,
        piece.member,
        piece.at,
        piece.len
    );
    Some(f.offset + piece.at..f.offset + piece.at + piece.len)
}

/// The element's bytes, padding as zero.
pub fn gather(layout: &Layout, pieces: &[Piece], record: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for piece in pieces {
        match span(layout, piece) {
            Some(r) => out.extend_from_slice(&record[r]),
            None => out.resize(out.len() + piece.len, 0),
        }
    }
    out
}

/// Writes the element's bytes back into the record; bytes for padding are dropped.
pub fn scatter(layout: &Layout, pieces: &[Piece], bytes: &[u8], record: &mut [u8]) {
    let mut at = 0;
    for piece in pieces {
        if let Some(r) = span(layout, piece) {
            record[r].copy_from_slice(&bytes[at..at + piece.len]);
        }
        at += piece.len;
    }
}

fn le(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .rev()
        .fold(0u32, |acc, &b| (acc << 8) | u32::from(b))
}

impl Vb {
    /// The element's value as NMR exports it.
    pub fn read(self, bytes: &[u8]) -> Value {
        match self {
            Vb::Long => Value::Long(le(bytes) as i32),
            Vb::Integer => Value::Long(i32::from(le(bytes) as u16 as i16)),
            Vb::Byte => Value::Long(i32::from(bytes[0])),
            Vb::Text => Value::Text(text::from_1252(bytes)),
        }
    }

    /// The element's bytes for `value`, `len` bytes long.
    pub fn write(self, value: &Value, len: usize) -> Result<Vec<u8>, ValueError> {
        if self == Vb::Text {
            let Value::Text(s) = value else {
                return Err(ValueError::NotText(value.clone()));
            };
            let mut bytes = text::to_1252(s)?;
            bytes.resize(len, 0);
            return Ok(bytes);
        }
        let n = number(value)?;
        let (lo, hi) = match self {
            Vb::Long => (f64::from(i32::MIN), f64::from(i32::MAX)),
            Vb::Integer => (f64::from(i16::MIN), f64::from(i16::MAX)),
            _ => (0.0, 255.0),
        };
        if n.fract() != 0.0 || n < lo || n > hi {
            return Err(ValueError::OutOfRange { value: n, vb: self });
        }
        let v = n as i64 as u32;
        let mut bytes = v.to_le_bytes()[..len.min(4)].to_vec();
        bytes.resize(len, 0);
        Ok(bytes)
    }
}

/// A numeric cell as f64 (NULL is 0, as DAO hands VB a Null that NMR's assignments reject;
/// Daydream treats it as 0).
pub fn number(value: &Value) -> Result<f64, ValueError> {
    match value {
        Value::Long(n) => Ok(f64::from(*n)),
        Value::Double(d) => Ok(*d),
        Value::Null => Ok(0.0),
        Value::Text(_) => Err(ValueError::NotNumber(value.clone())),
    }
}

/// `SLong2ULong`: a stored 32-bit value read unsigned.
pub fn read_unsigned(bytes: &[u8]) -> Value {
    Value::Double(f64::from(le(bytes)))
}

/// `ULong2SLong`: the 32-bit value for an unsigned cell, clamped to
/// -2147483648..=4294967295 as NMR clamps it (a negative value is stored as it is).
pub fn write_unsigned(value: &Value) -> Result<Vec<u8>, ValueError> {
    let n = number(value)?;
    if n.fract() != 0.0 {
        return Err(ValueError::OutOfRange {
            value: n,
            vb: Vb::Long,
        });
    }
    let v = n.clamp(f64::from(i32::MIN), f64::from(u32::MAX)) as i64 as u32;
    Ok(v.to_le_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_read_with_vb_signs() {
        assert_eq!(Vb::Integer.read(&[0xFF, 0xFF]), Value::Long(-1));
        assert_eq!(Vb::Long.read(&[1, 0, 0, 0x80]), Value::Long(i32::MIN + 1));
        assert_eq!(Vb::Byte.read(&[0xFF]), Value::Long(255));
        assert_eq!(read_unsigned(&[0xFF; 4]), Value::Double(4294967295.0));
    }

    #[test]
    fn integers_write_back_and_refuse_overflow() {
        assert_eq!(
            Vb::Integer.write(&Value::Long(-1), 2).unwrap(),
            [0xFF, 0xFF]
        );
        assert_eq!(
            Vb::Long.write(&Value::Long(-2), 4).unwrap(),
            [0xFE, 0xFF, 0xFF, 0xFF]
        );
        assert!(Vb::Integer.write(&Value::Long(40000), 2).is_err());
        assert!(Vb::Byte.write(&Value::Long(-1), 1).is_err());
        assert_eq!(write_unsigned(&Value::Double(5e9)).unwrap(), [0xFF; 4]);
        assert_eq!(write_unsigned(&Value::Double(-1.0)).unwrap(), [0xFF; 4]);
    }

    #[test]
    fn every_column_resolves_in_both_layouts() {
        for t in crate::columns::TABLES {
            let Some(table) = t.table else { continue };
            for set in [LayoutSet::Dos, LayoutSet::Worldgroup] {
                let layout = layout_of(table, set);
                for c in t.columns {
                    let pieces = match &c.source {
                        Source::Element { pieces, .. } | Source::Unsigned(pieces) => *pieces,
                        _ => continue,
                    };
                    for piece in pieces {
                        if span(layout, piece).is_none() {
                            assert!(
                                piece.member.starts_with("pad_") && set == LayoutSet::Dos,
                                "{}.{}: {} missing from the {set:?} layout",
                                t.name,
                                c.name,
                                piece.member
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn index_columns_exist() {
        for t in crate::columns::TABLES {
            if let Some(i) = &t.index {
                for c in i.columns {
                    assert!(t.column(c).is_some(), "{}: {c}", i.name);
                }
            }
        }
    }
}
