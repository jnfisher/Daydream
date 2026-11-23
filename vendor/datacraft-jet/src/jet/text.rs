//! Text decoding for Jet/ACE rows (split from `row.rs` to keep every file
//! under the size gate).
//!
//! Reference: `docs/FORMAT-JET.md` ("Text compression").

/// Text decoding: UCS-2LE or compressed unicode on Jet4, CP1252 on Jet3
/// (see `docs/FORMAT-JET.md`).
pub(crate) fn decode_text(bytes: &[u8], compressed: bool, jet3: bool) -> String {
    if jet3 {
        return crate::jet::jet3::cp1252(bytes);
    }
    if compressed {
        decode_compressed_text(bytes)
    } else {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .collect();
        String::from_utf16_lossy(&units)
    }
}

/// Jet4 text that may be compressed. As Jet (and Jackcess and mdbtools) store it: a value
/// starting with the `FF FE` marker is compressed, and from there each `0x00` byte toggles
/// between single-byte characters and UCS-2 pairs; a value without the marker is plain
/// UCS-2 (Jet stores a text it cannot compress, such as one holding NULs, that way even in
/// a compressed column). Daydream patch: 0.2.0 read a marker-less value as compressed and
/// toggled only once.
pub(crate) fn decode_compressed_text(bytes: &[u8]) -> String {
    let Some(body) = bytes.strip_prefix(&[0xFF, 0xFE][..]) else {
        return decode_text(bytes, false, false);
    };
    let mut units: Vec<u16> = Vec::with_capacity(body.len());
    let mut compressed = true;
    let mut at = 0usize;
    while at < body.len() {
        let b = body[at];
        if b == 0x00 {
            compressed = !compressed;
            at += 1;
        } else if compressed {
            units.push(u16::from(b));
            at += 1;
        } else if at + 1 < body.len() {
            units.push(u16::from_le_bytes([b, body[at + 1]]));
            at += 2;
        } else {
            break;
        }
    }
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod daydream_tests {
    use super::decode_compressed_text;

    #[test]
    fn unmarked_text_is_ucs2_and_marked_text_toggles() {
        // "\0 D" stored plainly, as Jet stores a text holding a NUL
        assert_eq!(decode_compressed_text(&[0, 0, 0x20, 0, 0x44, 0]), "\0 D");
        // "ab", then U+4E2D in UCS-2, then "c"
        assert_eq!(
            decode_compressed_text(&[0xFF, 0xFE, b'a', b'b', 0, 0x2D, 0x4E, 0, b'c']),
            "ab\u{4E2D}c"
        );
    }
}
