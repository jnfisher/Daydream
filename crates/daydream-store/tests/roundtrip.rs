//! The store half of the round-trip gate: every record of every stock file goes into a
//! `.db` and comes back byte for byte, in the reader's order (so `id` is the ordinal).

use daydream_store::WbtrvDb;
use daydream_wcc::btrieve::OpenOptions;
use daydream_wcc::tables::FILES;

use crate::support::stock_dir;

#[test]
fn every_stock_file_round_trips_through_a_db() {
    let dir = stock_dir();
    let tmp = tempfile::tempdir().unwrap();
    for f in FILES {
        let src = dir.join(format!("{}.VIR", f.stem));
        let dest = tmp.path().join(format!("{}.db", f.stem));
        let (db, report) = WbtrvDb::convert(&src, &dest).unwrap();
        assert_eq!(report.skipped, 0, "{}", f.stem);
        let stored = db.records().unwrap();
        let mut file = OpenOptions::new().variable_length(true).open(&src).unwrap();
        let mut records = file.records();
        let mut n = 0;
        while let Some(r) = records.next_record() {
            let r = r.unwrap();
            let (id, data) = &stored[n];
            assert_eq!(*id, i64::from(r.ordinal), "{}", f.stem);
            assert_eq!(data.as_slice(), r.data, "{} #{}", f.stem, r.ordinal);
            n += 1;
        }
        assert_eq!(n, stored.len(), "{}", f.stem);
        drop(db);
        // and it opens again as a realm file
        let db = WbtrvDb::open_read_only(&dest).unwrap();
        assert_eq!(db.count().unwrap() as usize, n);
    }
}
