//! Bytes to text the way NMR moves them into Access.
//!
//! NMR is a VB6 program: a record's fixed-length strings become VB strings through the
//! system ANSI code page, which on the Windows installs NMR runs on is Windows-1252. So a
//! stored byte becomes the 1252 character with that code, and importing turns it back.
//! Windows maps the five bytes 1252 leaves undefined (0x81, 0x8D, 0x8F, 0x90, 0x9D) to the
//! control characters of the same code, so every byte survives the trip. MajorMUD's own
//! text is CP437, so a high byte shows in Access as its 1252 character, as it does in NMR.

/// 0x80..=0x9F in Windows-1252, as Windows converts them.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// A character with no Windows-1252 byte.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} has no Windows-1252 byte")]
pub struct TextError(pub char);

/// One byte as its Windows-1252 character.
pub fn char_of(b: u8) -> char {
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

/// The Windows-1252 byte of a character.
pub fn byte_of(c: char) -> Result<u8, TextError> {
    match u32::from(c) {
        n @ (0..=0x7F | 0xA0..=0xFF) => Ok(n as u8),
        _ => HIGH
            .iter()
            .position(|&h| h == c)
            .map(|i| 0x80 + i as u8)
            .ok_or(TextError(c)),
    }
}

/// Every byte as its character (NULs included, as NMR's fixed-length strings hold them).
pub fn from_1252(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char_of(b)).collect()
}

/// The bytes of a string.
pub fn to_1252(s: &str) -> Result<Vec<u8>, TextError> {
    s.chars().map(byte_of).collect()
}

/// `DecryptTextblock`: each stored byte of 0x20 or more, less 0x20, up to the first that
/// becomes NUL; bytes below 0x20 are skipped.
pub fn decrypt_textblock(stored: &[u8]) -> String {
    let bytes: Vec<u8> = stored
        .iter()
        .filter(|&&b| b >= 0x20)
        .map(|&b| b - 0x20)
        .take_while(|&b| b != 0)
        .collect();
    from_1252(&bytes)
}

/// `EncryptTextblock`: each character's byte plus 0x20, characters above 0xDF dropped, cut
/// to `len` and NUL-filled.
pub fn encrypt_textblock(text: &str, len: usize) -> Result<Vec<u8>, TextError> {
    let mut out: Vec<u8> = to_1252(text)?
        .into_iter()
        .filter(|&b| b <= 0xDF)
        .map(|b| b + 0x20)
        .collect();
    out.resize(len, 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_round_trips() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(to_1252(&from_1252(&all)).unwrap(), all);
        assert_eq!(from_1252(&[0x80, 0x41, 0]), "\u{20AC}A\0");
        assert!(to_1252("\u{2500}").is_err());
    }

    #[test]
    fn text_blocks_shift_by_0x20() {
        let stored = encrypt_textblock("Hi\n", 8).unwrap();
        assert_eq!(stored, [0x68, 0x89, 0x2A, 0, 0, 0, 0, 0]);
        assert_eq!(decrypt_textblock(&stored), "Hi\n");
        // a stored 0x20 (a decoded NUL) ends the text; stored control bytes are skipped
        assert_eq!(decrypt_textblock(&[0x68, 0x01, 0x89, 0x20, 0x68]), "Hi");
    }
}
