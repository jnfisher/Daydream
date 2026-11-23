//! Btrieve keys as wbtrv32 stores them in SQLite (schema version 4).
//!
//! A port of wbtrv32's `Key.cc`, `KeyDefinition.h` and `SqliteUtil.cc` (upstream commit in
//! `UPSTREAM.md`): the column names and types of `data_t`'s key columns, its indexes and
//! non-modifiable trigger, and the value each key column holds for a record. wbtrv32 looks
//! records up by these columns, so a record Daydream writes must carry exactly the values
//! wbtrv32 would compute, or MBBSEmu and legacy NMR would not find it.
//!
//! Only whole records are converted here, never truncated search keys, so the "supplied
//! bytes" handling of upstream's search path is not needed: every segment is complete.

use rusqlite::types::{ToSql, ToSqlOutput, ValueRef};

/// Attribute bits (`AttributeMask.h`).
pub mod attr {
    pub const DUPLICATES: u16 = 1 << 0;
    pub const MODIFIABLE: u16 = 1 << 1;
    pub const NULL_ALL_SEGMENTS: u16 = 1 << 3;
    pub const SEGMENTED_KEY: u16 = 1 << 4;
    pub const NUMBERED_ACS: u16 = 1 << 5;
    pub const DESCENDING: u16 = 1 << 6;
    pub const REPEATING_DUPLICATES: u16 = 1 << 7;
    pub const NULL_ANY_SEGMENT: u16 = 1 << 9;
    pub const CASE_INSENSITIVE: u16 = 1 << 10;
}

/// Data type codes (`KeyDataType.h`).
pub mod ty {
    pub const STRING: u8 = 0x00;
    pub const INTEGER: u8 = 0x01;
    pub const FLOAT: u8 = 0x02;
    pub const DATE: u8 = 0x03;
    pub const TIME: u8 = 0x04;
    pub const DECIMAL: u8 = 0x05;
    pub const MONEY: u8 = 0x06;
    pub const NUMERIC: u8 = 0x08;
    pub const BFLOAT: u8 = 0x09;
    pub const LSTRING: u8 = 0x0A;
    pub const ZSTRING: u8 = 0x0B;
    pub const UNSIGNED: u8 = 0x0D;
    pub const UNSIGNED_BINARY: u8 = 0x0E;
    pub const AUTO_INC: u8 = 0x0F;
    pub const NUMERIC_STS: u8 = 0x11;
    pub const NUMERIC_SA: u8 = 0x12;
    pub const CURRENCY: u8 = 0x13;
    pub const TIMESTAMP: u8 = 0x14;
    pub const NULL_INDICATOR: u8 = 0xFF;
    pub const OLD_ASCII: u8 = 0x20;
    pub const OLD_BINARY: u8 = 0x21;
}

/// Bytes in a collating table.
pub const ACS_LENGTH: usize = 256;

/// One key segment: a row of `keys_t`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The key's number.
    pub number: u16,
    /// The segment's index within its key.
    pub index: u16,
    pub attributes: u16,
    /// The data type code ([`ty`]).
    pub data_type: u8,
    /// Offset in the record (after a v6 usage count).
    pub offset: u16,
    pub length: u16,
    pub null_value: u8,
    /// The collating sequence this segment sorts by, when it requires one.
    pub acs: Option<Vec<u8>>,
}

impl Segment {
    fn has(&self, mask: u16) -> bool {
        self.attributes & mask != 0
    }

    pub fn is_null_indicator(&self) -> bool {
        self.data_type == ty::NULL_INDICATOR
    }

    /// A null indicator segment ignores the ACS flag.
    pub fn requires_acs(&self) -> bool {
        self.has(attr::NUMBERED_ACS) && !self.is_null_indicator()
    }

    fn is_distinct_null_indicator(&self) -> bool {
        self.is_null_indicator() && self.has(attr::CASE_INSENSITIVE)
    }

    fn is_case_insensitive(&self) -> bool {
        self.has(attr::CASE_INSENSITIVE)
            && !self.requires_acs()
            && matches!(
                self.data_type,
                ty::STRING | ty::LSTRING | ty::ZSTRING | ty::OLD_ASCII
            )
    }

    pub fn is_descending(&self) -> bool {
        self.has(attr::DESCENDING)
    }

    fn is_string(&self) -> bool {
        matches!(self.data_type, ty::ZSTRING | ty::LSTRING | ty::OLD_ASCII)
    }

    fn is_nullable(&self) -> bool {
        self.has(attr::NULL_ALL_SEGMENTS | attr::NULL_ANY_SEGMENT) || self.is_string()
    }

    fn is_bfloat(&self) -> bool {
        self.data_type == ty::BFLOAT && (self.length == 4 || self.length == 8)
    }

    fn decimal_digits(&self) -> usize {
        let len = usize::from(self.length);
        match self.data_type {
            ty::DECIMAL | ty::MONEY => (len * 2).saturating_sub(1),
            ty::NUMERIC | ty::NUMERIC_SA => len,
            ty::NUMERIC_STS => len.saturating_sub(1),
            _ => 0,
        }
    }

    fn sqlite_type(&self) -> &'static str {
        let digits = self.decimal_digits();
        if self.is_null_indicator()
            || (is_binary_integer(self.data_type) && self.length <= 8)
            || self.is_bfloat()
            || (digits > 0 && digits <= MAX_INTEGER_DIGITS)
        {
            "INTEGER"
        } else if self.is_string() {
            "TEXT"
        } else if self.data_type == ty::FLOAT {
            "REAL"
        } else {
            "BLOB"
        }
    }

    /// The table mapping this segment's bytes to the ones it sorts by.
    fn collation(&self) -> Option<Collation<'_>> {
        if self.requires_acs() {
            self.acs.as_deref().map(Collation::Table)
        } else if self.is_case_insensitive() {
            Some(Collation::Upper)
        } else {
            None
        }
    }
}

enum Collation<'a> {
    Table(&'a [u8]),
    Upper,
}

impl Collation<'_> {
    fn map(&self, b: u8) -> u8 {
        match self {
            Self::Table(t) => t[usize::from(b)],
            Self::Upper => b.to_ascii_uppercase(),
        }
    }
}

const MAX_INTEGER_DIGITS: usize = 18;

fn is_signed_integer(t: u8) -> bool {
    matches!(t, ty::INTEGER | ty::AUTO_INC | ty::CURRENCY)
}

fn is_binary_integer(t: u8) -> bool {
    is_signed_integer(t)
        || matches!(
            t,
            ty::UNSIGNED
                | ty::UNSIGNED_BINARY
                | ty::OLD_BINARY
                | ty::DATE
                | ty::TIME
                | ty::TIMESTAMP
        )
}

/// A key and its segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub segments: Vec<Segment>,
    /// The ACS name and table wbtrv32 records in `keys_t` for every segment of this key: the
    /// key's own for a MultipleACS key, else the file's.
    pub acs_name: Option<String>,
    pub acs: Option<Vec<u8>>,
}

/// What can be wrong with a key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error("key {0} requires an ACS, but none was provided")]
    MissingAcs(u16),
    #[error("key {0}: a null indicator segment must be 1 byte")]
    NullIndicatorLength(u16),
    #[error("key {0}: a float segment must be 4 or 8 bytes")]
    FloatLength(u16),
    #[error("key {0}: the ACS is {1} bytes, not 256")]
    AcsLength(u16, usize),
    #[error("key {key}: segment at {offset}+{length} is past the record's {record} bytes")]
    OutOfRecord {
        key: u16,
        offset: u16,
        length: u16,
        record: usize,
    },
}

impl Key {
    pub fn primary(&self) -> &Segment {
        &self.segments[0]
    }

    pub fn number(&self) -> u16 {
        self.primary().number
    }

    /// The checks wbtrv32's `KeyDefinition` constructor makes.
    pub fn validate(&self) -> Result<(), KeyError> {
        for s in &self.segments {
            if s.requires_acs() && s.acs.is_none() {
                return Err(KeyError::MissingAcs(s.number));
            }
            if let Some(acs) = &s.acs
                && acs.len() != ACS_LENGTH
            {
                return Err(KeyError::AcsLength(s.number, acs.len()));
            }
            if s.is_null_indicator() && s.length != 1 {
                return Err(KeyError::NullIndicatorLength(s.number));
            }
            if s.data_type == ty::FLOAT && s.length != 4 && s.length != 8 {
                return Err(KeyError::FloatLength(s.number));
            }
        }
        Ok(())
    }

    pub fn is_composite(&self) -> bool {
        self.segments.len() > 1
    }

    pub fn is_modifiable(&self) -> bool {
        self.primary().has(attr::MODIFIABLE)
    }

    pub fn is_unique(&self) -> bool {
        !self
            .primary()
            .has(attr::DUPLICATES | attr::REPEATING_DUPLICATES)
    }

    fn has_null_attribute(&self) -> bool {
        self.primary()
            .has(attr::NULL_ALL_SEGMENTS | attr::NULL_ANY_SEGMENT)
    }

    fn is_nullable(&self) -> bool {
        if self.is_composite() {
            self.has_null_attribute()
        } else {
            self.primary().is_nullable()
        }
    }

    fn has_null_indicator(&self) -> bool {
        self.segments.iter().any(Segment::is_null_indicator)
    }

    fn len(&self) -> usize {
        self.segments.iter().map(|s| usize::from(s.length)).sum()
    }

    /// `key_N`.
    pub fn name(&self) -> String {
        format!("key_{}", self.number())
    }

    /// `key_N`, or `key_N_0`, `key_N_1`, ... for a composite key.
    pub fn column_names(&self) -> Vec<String> {
        if !self.is_composite() {
            return vec![self.name()];
        }
        (0..self.segments.len())
            .map(|i| format!("{}_{i}", self.name()))
            .collect()
    }

    /// The column definitions in `data_t`.
    pub fn column_definitions(&self) -> Vec<String> {
        let names = self.column_names();
        if !self.is_composite() && self.primary().data_type == ty::AUTO_INC {
            return vec![format!("{} INTEGER NOT NULL UNIQUE", names[0])];
        }
        names
            .iter()
            .zip(&self.segments)
            .map(|(name, s)| {
                let mut sql = format!("{name} {}", s.sqlite_type());
                if !self.is_nullable() {
                    sql.push_str(" NOT NULL");
                }
                if !self.is_composite() && self.is_unique() {
                    sql.push_str(" UNIQUE");
                }
                sql
            })
            .collect()
    }

    /// The `CREATE INDEX` statements for this key.
    pub fn index_sql(&self) -> Vec<String> {
        let names = self.column_names();
        let unique_without_nulls = self.is_unique() && self.has_null_indicator();
        let possibly_unique = if self.is_unique() && !unique_without_nulls {
            "UNIQUE"
        } else {
            ""
        };
        let index_columns: Vec<String> = names
            .iter()
            .zip(&self.segments)
            .map(|(n, s)| {
                if s.is_descending() {
                    format!("{n} DESC")
                } else {
                    n.clone()
                }
            })
            .collect();
        let mut out = vec![format!(
            "CREATE {possibly_unique} INDEX {}_index on data_t({})",
            self.name(),
            index_columns.join(", ")
        )];
        if unique_without_nulls {
            let terms: Vec<String> = names
                .iter()
                .zip(&self.segments)
                .filter(|(_, s)| s.is_null_indicator())
                .map(|(n, _)| format!("{n} = 0"))
                .collect();
            out.push(format!(
                "CREATE UNIQUE INDEX {}_unique on data_t({}) WHERE {}",
                self.name(),
                names.join(", "),
                terms.join(" AND ")
            ));
        }
        out
    }

    /// This key's column values for `record`, in [`Key::column_names`] order.
    pub fn values(&self, record: &[u8]) -> Result<Vec<KeyValue>, KeyError> {
        let mut data = Vec::with_capacity(self.len());
        for s in &self.segments {
            let start = usize::from(s.offset);
            let end = start + usize::from(s.length);
            let bytes = record.get(start..end).ok_or(KeyError::OutOfRecord {
                key: s.number,
                offset: s.offset,
                length: s.length,
                record: record.len(),
            })?;
            data.extend_from_slice(bytes);
        }
        Ok(self.key_data_values(data))
    }

    /// `keyDataToSqliteObjects(keyData, allowNull = true)` for a complete key.
    fn key_data_values(&self, mut data: Vec<u8>) -> Vec<KeyValue> {
        if self.is_null_key_data(&data) {
            return vec![KeyValue::Null; self.segments.len()];
        }
        if self.is_composite() {
            // a true null's field reads as zero bytes
            let mut at = 0;
            for i in 0..self.segments.len() - 1 {
                let s = &self.segments[i];
                if s.is_null_indicator() && data[at] != 0 {
                    let field = at + usize::from(s.length);
                    let next = usize::from(self.segments[i + 1].length);
                    data[field..field + next].fill(0);
                }
                at += usize::from(s.length);
            }
        }
        self.apply_collation(&mut data);
        let mut out = Vec::with_capacity(self.segments.len());
        let mut at = 0;
        for s in &self.segments {
            let len = usize::from(s.length);
            out.push(segment_value(s, &data[at..at + len]));
            at += len;
        }
        out
    }

    fn is_null_key_data(&self, data: &[u8]) -> bool {
        if !self.has_null_attribute() {
            return false;
        }
        let mut null_segments = 0;
        let mut at = 0;
        for s in &self.segments {
            let len = usize::from(s.length);
            if data[at..at + len].iter().all(|&b| b == s.null_value) {
                null_segments += 1;
            }
            at += len;
        }
        if self.primary().has(attr::NULL_ANY_SEGMENT) {
            null_segments > 0
        } else {
            null_segments == self.segments.len()
        }
    }

    fn apply_collation(&self, data: &mut [u8]) {
        let mut at = 0;
        for s in &self.segments {
            let end = at + usize::from(s.length);
            if let Some(c) = s.collation() {
                // an LSTRING's first byte is its length, which isn't collated
                let start = at + usize::from(s.data_type == ty::LSTRING);
                for b in &mut data[start.min(end)..end] {
                    *b = c.map(*b);
                }
            }
            at = end;
        }
    }
}

/// The non-modifiable trigger for `keys`, if any key is non-modifiable.
pub fn trigger_sql(keys: &[Key]) -> Option<String> {
    let fixed: Vec<&Key> = keys.iter().filter(|k| !k.is_modifiable()).collect();
    if fixed.is_empty() {
        return None;
    }
    let mut sql =
        String::from("CREATE TRIGGER non_modifiable BEFORE UPDATE ON data_t BEGIN SELECT CASE ");
    for key in fixed {
        for column in key.column_names() {
            sql.push_str(&format!(
                "WHEN NEW.{column} IS NOT OLD.{column} THEN RAISE (ABORT,'You modified a non-modifiable {}!') ",
                key.name()
            ));
        }
    }
    sql.push_str("END; END;");
    Some(sql)
}

/// A key column's value.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyValue {
    Null,
    Integer(i64),
    Real(f64),
    /// Stored as TEXT; the bytes are the record's (CP437), not necessarily UTF-8.
    Text(Vec<u8>),
    Blob(Vec<u8>),
}

impl ToSql for KeyValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(match self {
            Self::Null => ValueRef::Null,
            Self::Integer(i) => ValueRef::Integer(*i),
            Self::Real(f) => ValueRef::Real(*f),
            Self::Text(t) => ValueRef::Text(t),
            Self::Blob(b) => ValueRef::Blob(b),
        }))
    }
}

/// `segmentDataToSqliteObject` for a complete, collated segment.
fn segment_value(s: &Segment, data: &[u8]) -> KeyValue {
    if s.is_bfloat() {
        return KeyValue::Integer(bfloat_to_integer(data));
    }
    if s.decimal_digits() > 0 {
        return decimal_value(s.data_type, data);
    }
    match s.data_type {
        ty::NULL_INDICATOR => {
            let indicator = data[0];
            if indicator == 0 || s.is_distinct_null_indicator() {
                KeyValue::Integer(i64::from(indicator))
            } else {
                KeyValue::Integer(1)
            }
        }
        t if is_binary_integer(t) => {
            if data.is_empty() || data.len() > 8 {
                // more than 8 bytes (or none): a BLOB, most significant byte first so memcmp sorts it
                let mut copy = data.to_vec();
                copy.reverse();
                return KeyValue::Blob(copy);
            }
            let mut value: u64 = if is_signed_integer(t) && data[data.len() - 1] & 0x80 != 0 {
                u64::MAX
            } else {
                0
            };
            for (i, &b) in data.iter().enumerate() {
                value &= !(0xFFu64 << (i * 8));
                value |= u64::from(b) << (i * 8);
            }
            if data.len() == 8 && !is_signed_integer(t) {
                value ^= 1 << 63;
            }
            KeyValue::Integer(value as i64)
        }
        ty::LSTRING => {
            let len = usize::from(data[0]).min(data.len() - 1);
            KeyValue::Text(data[1..1 + len].to_vec())
        }
        ty::ZSTRING | ty::OLD_ASCII => {
            let len = data.iter().position(|&b| b == 0).unwrap_or(data.len());
            KeyValue::Text(data[..len].to_vec())
        }
        ty::FLOAT => {
            let mut buf = [0u8; 8];
            let n = data.len().min(8);
            buf[..n].copy_from_slice(&data[..n]);
            if s.length == 4 {
                let f = f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                KeyValue::Real(f64::from(f))
            } else {
                KeyValue::Real(f64::from_le_bytes(buf))
            }
        }
        _ => KeyValue::Blob(data.to_vec()),
    }
}

/// A BFLOAT (Microsoft Binary Format) value as an INTEGER that sorts the same.
fn bfloat_to_integer(bytes: &[u8]) -> i64 {
    let mantissa_bytes = bytes.len() - 1;
    let exponent = bytes[mantissa_bytes];
    if exponent == 0 {
        return 0;
    }
    let mantissa_bits = (mantissa_bytes * 8 - 1) as u32;
    let mut mantissa: u64 = 0;
    for (i, &b) in bytes[..mantissa_bytes].iter().enumerate() {
        mantissa |= u64::from(b) << (i * 8);
    }
    let negative = (mantissa >> mantissa_bits) & 1 == 1;
    mantissa &= (1u64 << mantissa_bits) - 1;
    let magnitude = ((u64::from(exponent) << mantissa_bits) | mantissa) as i64;
    if negative { -magnitude } else { magnitude }
}

fn ascii_digit(c: u8) -> u8 {
    if c.is_ascii_digit() { c - b'0' } else { 0 }
}

/// A DECIMAL, MONEY or NUMERIC family value: an INTEGER of its digits when they fit, else
/// a BLOB that memcmp orders.
fn decimal_value(t: u8, bytes: &[u8]) -> KeyValue {
    let mut digits = Vec::new();
    let mut negative = false;
    let last = bytes.len() - 1;
    match t {
        ty::DECIMAL | ty::MONEY => {
            for (i, &b) in bytes.iter().enumerate() {
                digits.push(b >> 4);
                if i < last {
                    digits.push(b & 0xF);
                }
            }
            negative = bytes[last] & 0xF == 0xD;
        }
        ty::NUMERIC | ty::NUMERIC_SA => {
            digits.extend(bytes[..last].iter().map(|&b| ascii_digit(b)));
            let c = bytes[last];
            let mut digit = ascii_digit(c);
            if t == ty::NUMERIC {
                if (b'A'..=b'I').contains(&c) {
                    digit = c - b'A' + 1;
                } else if (b'J'..=b'R').contains(&c) {
                    digit = c - b'J' + 1;
                    negative = true;
                } else if c == b'}' {
                    negative = true;
                }
            } else if (b'P'..=b'Y').contains(&c) {
                digit = c - b'P';
            } else if (b'p'..=b'y').contains(&c) {
                digit = c - b'p';
                negative = true;
            }
            digits.push(digit);
        }
        ty::NUMERIC_STS => {
            digits.extend(bytes[..last].iter().map(|&b| ascii_digit(b)));
            negative = bytes[last] == b'-';
        }
        _ => {}
    }
    if digits.iter().all(|&d| d == 0) {
        negative = false;
    }
    if digits.len() <= MAX_INTEGER_DIGITS {
        let value = digits.iter().fold(0i64, |v, &d| v * 10 + i64::from(d));
        return KeyValue::Integer(if negative { -value } else { value });
    }
    let mut blob = vec![u8::from(!negative)];
    blob.extend(digits.iter().map(|&d| if negative { 0xFF - d } else { d }));
    KeyValue::Blob(blob)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(data_type: u8, attributes: u16, offset: u16, length: u16) -> Segment {
        Segment {
            number: 0,
            index: 0,
            attributes,
            data_type,
            offset,
            length,
            null_value: 0,
            acs: None,
        }
    }

    fn key(segments: Vec<Segment>) -> Key {
        Key {
            segments,
            acs_name: None,
            acs: None,
        }
    }

    #[test]
    fn integers_sign_extend_and_unsigned_do_not() {
        let k = key(vec![seg(ty::INTEGER, 0, 0, 2)]);
        assert_eq!(
            k.values(&[0xFF, 0xFF]).unwrap(),
            vec![KeyValue::Integer(-1)]
        );
        let k = key(vec![seg(ty::UNSIGNED_BINARY, 0, 0, 2)]);
        assert_eq!(
            k.values(&[0xFF, 0xFF]).unwrap(),
            vec![KeyValue::Integer(65535)]
        );
        let k = key(vec![seg(ty::UNSIGNED_BINARY, 0, 0, 8)]);
        assert_eq!(
            k.values(&[0; 8]).unwrap(),
            vec![KeyValue::Integer(i64::MIN)]
        );
    }

    #[test]
    fn zstrings_stop_at_nul_and_case_insensitive_keys_upper_case() {
        let k = key(vec![seg(ty::ZSTRING, attr::CASE_INSENSITIVE, 1, 4)]);
        assert_eq!(
            k.values(b"xab\0z").unwrap(),
            vec![KeyValue::Text(b"AB".to_vec())]
        );
    }

    #[test]
    fn a_null_key_is_null_only_when_it_opts_in() {
        let k = key(vec![seg(ty::INTEGER, 0, 0, 2)]);
        assert_eq!(k.values(&[0, 0]).unwrap(), vec![KeyValue::Integer(0)]);
        let k = key(vec![seg(ty::INTEGER, attr::NULL_ALL_SEGMENTS, 0, 2)]);
        assert_eq!(k.values(&[0, 0]).unwrap(), vec![KeyValue::Null]);
    }

    #[test]
    fn column_definitions_follow_wbtrv32() {
        let k = key(vec![seg(ty::ZSTRING, 0, 0, 4)]);
        assert_eq!(k.column_definitions(), vec!["key_0 TEXT UNIQUE"]);
        let k = key(vec![seg(ty::INTEGER, attr::DUPLICATES, 0, 4)]);
        assert_eq!(k.column_definitions(), vec!["key_0 INTEGER NOT NULL"]);
        let k = key(vec![seg(ty::AUTO_INC, 0, 0, 4)]);
        assert_eq!(
            k.column_definitions(),
            vec!["key_0 INTEGER NOT NULL UNIQUE"]
        );
        let k = key(vec![
            seg(ty::INTEGER, attr::SEGMENTED_KEY, 0, 2),
            seg(ty::ZSTRING, 0, 2, 4),
        ]);
        assert_eq!(
            k.column_definitions(),
            vec!["key_0_0 INTEGER NOT NULL", "key_0_1 TEXT NOT NULL"]
        );
    }

    #[test]
    fn bfloat_and_decimal_sort_as_integers() {
        assert_eq!(bfloat_to_integer(&[0, 0, 0, 0]), 0);
        assert_eq!(
            decimal_value(ty::DECIMAL, &[0x12, 0x3D]),
            KeyValue::Integer(-123)
        );
        assert_eq!(
            decimal_value(ty::NUMERIC_STS, b"042+"),
            KeyValue::Integer(42)
        );
    }
}
