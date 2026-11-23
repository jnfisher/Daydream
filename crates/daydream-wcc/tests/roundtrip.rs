//! The decoders lose nothing: every record of every stock 1.11p file decodes, and encodes
//! back to the same bytes. This is the record half of the round-trip gate; the store half is
//! in daydream-store.

use daydream_wcc::btrieve::OpenOptions;
use daydream_wcc::tables::{FILES, Table};

use crate::stock;

#[test]
fn every_stock_record_reencodes_to_its_own_bytes() {
    let mut total = 0;
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
            if f.table == Table::TextBlocks {
                // a stored text block is cut short; the decoder zero-fills it to the buffer
                assert!(again.len() >= raw.len(), "{} #{}", f.stem, record.ordinal);
                assert_eq!(&again[..raw.len()], raw, "{} #{}", f.stem, record.ordinal);
                assert!(again[raw.len()..].iter().all(|&b| b == 0));
            } else {
                assert_eq!(again, raw, "{} #{} changed", f.stem, record.ordinal);
            }
            let json = f.table.to_json(record.ordinal, raw).unwrap();
            assert_eq!(json["_id"], record.ordinal);
            total += 1;
        }
    }
    // the stock set's record counts (see stock_vir.rs)
    assert_eq!(
        total,
        13 + 15 + 26_720 + 178 + 1950 + 1379 + 1101 + 3867 + 3467 + 64
    );
}
