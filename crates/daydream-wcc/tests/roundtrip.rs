//! The decoders lose nothing: every record of every stock 1.11p file decodes, and encodes
//! back to the same bytes. This is the record half of the round-trip gate; the store half is
//! in daydream-store.

use daydream_wcc::btrieve::OpenOptions;
use daydream_wcc::decode::{RaceRecord, Record, Text};
use daydream_wcc::tables::{FILES, Table};

use crate::stock;

#[test]
fn every_stock_record_reencodes_to_its_own_bytes() {
    let mut total = 0;
    let mut changed = Vec::new();
    for f in FILES {
        let path = stock::file(&format!("{}.VIR", f.stem));
        let mut file = OpenOptions::new()
            .variable_length(f.table.is_variable_length())
            .open(&path)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut records = file.records();
        while let Some(record) = records.next_record() {
            let record = record.unwrap();
            let raw = record.data;
            let again = f
                .table
                .reencode(raw)
                .unwrap_or_else(|e| panic!("{} #{}: {e}", f.stem, record.ordinal));
            let same = if f.table == Table::TextBlocks {
                // a stored text block is cut short; the decoder zero-fills it to the buffer
                again.len() >= raw.len()
                    && &again[..raw.len()] == raw
                    && again[raw.len()..].iter().all(|&b| b == 0)
            } else {
                again == raw
            };
            if !same {
                let at = again.iter().zip(raw).position(|(a, b)| a != b);
                changed.push(format!(
                    "{} #{} (first difference at {at:?})",
                    f.stem, record.ordinal
                ));
            }
            let json = f.table.to_json(record.ordinal, raw).unwrap();
            assert_eq!(json["_id"], record.ordinal);
            total += 1;
        }
    }
    assert!(
        changed.is_empty(),
        "{} records changed: {:?}",
        changed.len(),
        &changed[..changed.len().min(20)]
    );
    // the stock set's record counts (see stock_vir.rs)
    assert_eq!(
        total,
        13 + 15 + 26_720 + 178 + 1950 + 1379 + 1101 + 3867 + 3467 + 64
    );
}

/// The game leaves old bytes after a text's NUL; they survive a decode and encode, and an
/// edited text is written cleanly.
#[test]
fn bytes_after_a_texts_nul_survive_until_the_text_changes() {
    let mut raw = vec![0u8; RaceRecord::LAYOUT.len];
    raw[2..2 + 16].copy_from_slice(b"Elf\0Half-Ogre   ");
    let r = RaceRecord::decode(&raw).unwrap();
    assert_eq!(r.name.0, "Elf");
    assert_eq!(r.encode(), raw);

    let mut edited = r.clone();
    edited.name = Text::new("Gnome");
    let out = edited.encode();
    assert_eq!(&out[2..8], b"Gnome\0");
    assert!(out[8..32].iter().all(|&b| b == 0));
    assert_eq!(&out[32..], &raw[32..]);
}
