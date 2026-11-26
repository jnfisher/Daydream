//! Key definitions: the 0x1E-byte segment specs in the FCR.
//!
//! Port of wbtrv32 `loadKeyDefinitions` (BtrieveDatabase.cc:468), `KeyDefinition.h`,
//! `AttributeMask.h` and `KeyDataType.h`. Keys are exposed as data only: there is no index
//! here, the database rebuilds them as constraints.

use super::acs::Acs;
use super::error::Fault;
use super::header::{Version, u16_at};

/// Bytes per key segment definition.
pub const KEY_DEF_LENGTH: usize = 0x1E;

/// Where the v5 key definitions start in the FCR; they follow one another, segments
/// included.
pub(crate) const V5_KEY_DEF_BASE: u64 = 0x110;

/// The attribute word of a key segment (`AttributeMask.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyAttributes(pub u16);

impl KeyAttributes {
    pub const DUPLICATES: u16 = 0x0001;
    pub const MODIFIABLE: u16 = 0x0002;
    pub const OLD_STYLE_BINARY: u16 = 0x0004;
    pub const NULL_ALL_SEGMENTS: u16 = 0x0008;
    pub const SEGMENTED: u16 = 0x0010;
    pub const NUMBERED_ACS: u16 = 0x0020;
    pub const DESCENDING: u16 = 0x0040;
    pub const REPEATING_DUPLICATES: u16 = 0x0080;
    pub const EXTENDED_TYPE: u16 = 0x0100;
    pub const NULL_ANY_SEGMENT: u16 = 0x0200;
    /// MultipleACS is 0x400 together with NumberedACS.
    pub const MULTIPLE_ACS: u16 = 0x0400 | Self::NUMBERED_ACS;

    /// True when every bit of `mask` is set.
    pub fn has(self, mask: u16) -> bool {
        self.0 & mask == mask
    }
}

/// The data type of a key segment (`KeyDataType.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyDataType {
    String,
    Integer,
    Float,
    Date,
    Time,
    Decimal,
    Money,
    Logical,
    Numeric,
    Bfloat,
    Lstring,
    Zstring,
    Unsigned,
    UnsignedBinary,
    AutoInc,
    /// No extended type and no binary flag: a plain string.
    OldAscii,
    /// No extended type, binary flag set: an unsigned integer.
    OldBinary,
    /// An extended type code wbtrv32 does not name.
    Other(u8),
}

impl KeyDataType {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::String,
            0x01 => Self::Integer,
            0x02 => Self::Float,
            0x03 => Self::Date,
            0x04 => Self::Time,
            0x05 => Self::Decimal,
            0x06 => Self::Money,
            0x07 => Self::Logical,
            0x08 => Self::Numeric,
            0x09 => Self::Bfloat,
            0x0A => Self::Lstring,
            0x0B => Self::Zstring,
            0x0D => Self::Unsigned,
            0x0E => Self::UnsignedBinary,
            0x0F => Self::AutoInc,
            0x20 => Self::OldAscii,
            0x21 => Self::OldBinary,
            other => Self::Other(other),
        }
    }

    /// String-like types (`KeyDefinition::isString`).
    /// The data type's code, as Btrieve stores it and wbtrv32 writes it in `keys_t`
    /// (Daydream addition).
    pub fn code(self) -> u8 {
        match self {
            Self::String => 0x00,
            Self::Integer => 0x01,
            Self::Float => 0x02,
            Self::Date => 0x03,
            Self::Time => 0x04,
            Self::Decimal => 0x05,
            Self::Money => 0x06,
            Self::Logical => 0x07,
            Self::Numeric => 0x08,
            Self::Bfloat => 0x09,
            Self::Lstring => 0x0A,
            Self::Zstring => 0x0B,
            Self::Unsigned => 0x0D,
            Self::UnsignedBinary => 0x0E,
            Self::AutoInc => 0x0F,
            Self::OldAscii => 0x20,
            Self::OldBinary => 0x21,
            Self::Other(code) => code,
        }
    }

    pub fn is_string(self) -> bool {
        matches!(self, Self::Zstring | Self::Lstring | Self::OldAscii)
    }
}

/// One key segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyDef {
    /// Key number (0-based ordinal of the key, shared by all its segments).
    pub number: u16,
    /// Segment index within the key, 0 for the first.
    pub segment: u16,
    pub attributes: KeyAttributes,
    pub data_type: KeyDataType,
    /// 0-based byte offset into the record payload. The file stores it 0-based relative
    /// to the physical record, so v6 subtracts the 2-byte usage count: offsets are
    /// payload-relative in both versions.
    pub offset: u16,
    pub length: u16,
    pub null_value: u8,
    /// The collating sequence, set only when the segment has `NumberedACS`.
    pub acs_name: Option<String>,
    pub acs: Option<[u8; 256]>,
}

impl KeyDef {
    /// 1-based Btrieve key position (`KeyDefinition::getPosition`).
    pub fn position(&self) -> u16 {
        self.offset + 1
    }

    /// Another segment of the same key follows this one.
    pub fn continues(&self) -> bool {
        self.attributes.has(KeyAttributes::SEGMENTED)
    }

    pub fn allows_duplicates(&self) -> bool {
        self.attributes.0 & (KeyAttributes::DUPLICATES | KeyAttributes::REPEATING_DUPLICATES) != 0
    }

    pub fn is_modifiable(&self) -> bool {
        self.attributes.has(KeyAttributes::MODIFIABLE)
    }

    pub fn is_nullable(&self) -> bool {
        self.attributes.0 & (KeyAttributes::NULL_ALL_SEGMENTS | KeyAttributes::NULL_ANY_SEGMENT)
            != 0
            || self.data_type.is_string()
    }

    pub fn requires_acs(&self) -> bool {
        self.attributes.has(KeyAttributes::NUMBERED_ACS)
    }

    /// True when the segment collates through a case-folding ACS.
    pub fn is_case_insensitive(&self) -> bool {
        self.acs.is_some_and(|table| {
            (b'a'..=b'z').all(|c| table[usize::from(c)] == table[usize::from(c - 0x20)])
        })
    }

    /// This segment's bytes in a record payload, or `None` if the record is too short.
    pub fn bytes<'r>(&self, record: &'r [u8]) -> Option<&'r [u8]> {
        let start = usize::from(self.offset);
        record.get(start..start + usize::from(self.length))
    }
}

/// A key: one or more segments, concatenated in order (`Key.cc extractKeyDataFromRecord`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub number: u16,
    pub segments: Vec<KeyDef>,
}

impl Key {
    pub fn primary(&self) -> &KeyDef {
        &self.segments[0]
    }

    pub fn is_composite(&self) -> bool {
        self.segments.len() > 1
    }

    /// Total key length, the sum of the segment lengths.
    pub fn length(&self) -> usize {
        self.segments.iter().map(|s| usize::from(s.length)).sum()
    }

    pub fn allows_duplicates(&self) -> bool {
        self.primary().allows_duplicates()
    }

    /// The key value of a record: its segments' bytes concatenated, without ACS mapping.
    pub fn extract(&self, record: &[u8]) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(self.length());
        for segment in &self.segments {
            out.extend_from_slice(segment.bytes(record)?);
        }
        Some(out)
    }

    /// The key value of a record as Btrieve tells values apart: two records have the same
    /// key exactly when these bytes are equal. A zstring segment ends at its NUL and an
    /// lstring at its length, and text collates through the segment's ACS (so a
    /// case-insensitive key has one value however it is cased). `None` if the record is too
    /// short. (Daydream addition.)
    pub fn identity(&self, record: &[u8]) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(self.length());
        for s in &self.segments {
            let b = s.bytes(record)?;
            let text = match s.data_type {
                KeyDataType::Zstring => &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())],
                KeyDataType::Lstring if !b.is_empty() => {
                    &b[1..1 + usize::from(b[0]).min(b.len() - 1)]
                }
                KeyDataType::String | KeyDataType::OldAscii => b,
                _ => {
                    out.extend_from_slice(b);
                    continue;
                }
            };
            let start = out.len();
            match &s.acs {
                Some(table) => out.extend(text.iter().map(|&c| table[usize::from(c)])),
                None => out.extend_from_slice(text),
            }
            out.resize(start + b.len(), 0);
        }
        Some(out)
    }

    /// True when a record's key is null, so the index leaves the record out (and it takes
    /// no value of the key). (Daydream addition.)
    pub fn is_null(&self, record: &[u8]) -> bool {
        crate::btrieve::index::is_null(self, record)
    }
}

/// A segment as stored, before the ACS is attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawKeyDef {
    pub attributes: KeyAttributes,
    pub data_type: KeyDataType,
    pub offset: u16,
    pub length: u16,
    pub null_value: u8,
    /// Logical page of this segment's own ACS (MultipleACS only).
    pub acs_page: u32,
}

/// Decodes one 0x1E-byte segment definition for key `number`.
pub(crate) fn parse_key_def(
    def: &[u8],
    number: u16,
    version: Version,
    record_length: u16,
) -> Result<RawKeyDef, Fault> {
    let bad = |reason: String| Fault::BadKey {
        key: number,
        reason,
    };
    if def.len() < KEY_DEF_LENGTH {
        return Err(bad("key definition truncated".into()));
    }
    let attributes = KeyAttributes(u16_at(def, 0x08));
    let data_type = if attributes.has(KeyAttributes::EXTENDED_TYPE) {
        KeyDataType::from_code(def[0x1C])
    } else if attributes.has(KeyAttributes::OLD_STYLE_BINARY) {
        KeyDataType::OldBinary
    } else {
        KeyDataType::OldAscii
    };
    let stored = u16_at(def, 0x14);
    let offset = match version {
        Version::V5 => stored,
        Version::V6 => stored
            .checked_sub(2)
            .ok_or_else(|| bad(format!("v6 offset {stored} inside the usage count")))?,
    };
    let length = u16_at(def, 0x16);
    if data_type == KeyDataType::Float && length != 4 && length != 8 {
        return Err(bad(format!("float key of length {length}")));
    }
    // The decoders slice records by these, so a key outside the fixed part is a hard error.
    if length == 0 || usize::from(offset) + usize::from(length) > usize::from(record_length) {
        return Err(bad(format!(
            "segment at {offset} length {length} outside record length {record_length}"
        )));
    }
    Ok(RawKeyDef {
        attributes,
        data_type,
        offset,
        length,
        null_value: def[0x1D],
        acs_page: u32::from(def[0x19]) << 16 | u32::from(u16_at(def, 0x1A)),
    })
}

/// Builds the exposed segment from a decoded one and its ACS (if any).
pub(crate) fn key_def(raw: RawKeyDef, number: u16, segment: u16, acs: Option<&Acs>) -> KeyDef {
    KeyDef {
        number,
        segment,
        attributes: raw.attributes,
        data_type: raw.data_type,
        offset: raw.offset,
        length: raw.length,
        null_value: raw.null_value,
        acs_name: acs.map(|a| a.name.clone()),
        acs: acs.map(|a| a.table),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(offset: u16, length: u16, data_type: KeyDataType, acs: Option<[u8; 256]>) -> KeyDef {
        KeyDef {
            number: 0,
            segment: 0,
            attributes: KeyAttributes(0),
            data_type,
            offset,
            length,
            null_value: 0,
            acs_name: None,
            acs,
        }
    }

    #[test]
    fn identity_tells_values_apart_as_btrieve_does() {
        let mut upper = [0u8; 256];
        for (i, c) in upper.iter_mut().enumerate() {
            *c = (i as u8).to_ascii_uppercase();
        }
        let key = Key {
            number: 0,
            segments: vec![
                segment(0, 2, KeyDataType::Integer, None),
                segment(2, 6, KeyDataType::Zstring, Some(upper)),
            ],
        };
        let id = |r: &[u8]| key.identity(r).unwrap();
        // past the NUL nothing counts, and the case folds
        assert_eq!(id(b"\x01\x00abc\0xy"), id(b"\x01\x00ABC\0\0\0"));
        assert_ne!(id(b"\x01\x00abc\0xy"), id(b"\x02\x00abc\0xy"));
        assert_ne!(id(b"\x01\x00abc\0xy"), id(b"\x01\x00abd\0xy"));
        assert_eq!(id(b"\x01\x00abc\0xy").len(), key.length());
        assert_eq!(key.identity(b"\x01\x00abc"), None);
        // without an ACS the case counts
        let key = Key {
            number: 0,
            segments: vec![segment(0, 4, KeyDataType::Zstring, None)],
        };
        assert_ne!(key.identity(b"ab\0\0"), key.identity(b"AB\0\0"));
    }
}
