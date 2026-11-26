//! Small realms for the unit tests.

use daydream_types::records::layout::ITEM;

use crate::key::{Key, Segment, attr, ty};
use crate::{Metadata, Realm, WbtrvDb, journal};

/// An item record: its number, name and weight.
pub fn item(number: i32, name: &[u8], weight: u8) -> Vec<u8> {
    let mut r = vec![0; ITEM.len];
    r[..4].copy_from_slice(&number.to_le_bytes());
    let n = ITEM.field("name").unwrap();
    r[n.offset..n.offset + name.len()].copy_from_slice(name);
    r[ITEM.field("weight").unwrap().offset] = weight;
    r
}

/// A realm of one items file (numbers unique), made from `items` and established.
pub fn items_realm(items: &[Vec<u8>]) -> (tempfile::TempDir, Realm) {
    let tmp = tempfile::tempdir().unwrap();
    let meta = Metadata {
        record_length: ITEM.len,
        physical_record_length: ITEM.len + 4,
        page_length: 4096,
        variable_length_records: false,
        version: 4,
    };
    let key = Key {
        segments: vec![Segment {
            number: 0,
            index: 0,
            attributes: attr::MODIFIABLE,
            data_type: ty::INTEGER,
            offset: 0,
            length: 4,
            null_value: 0,
            acs: None,
        }],
        acs_name: None,
        acs: None,
    };
    let mut db = WbtrvDb::create(&tmp.path().join("WCCITEMS.db"), meta, vec![key]).unwrap();
    for r in items {
        db.insert(r).unwrap();
    }
    drop(db);
    journal::reset(tmp.path()).unwrap();
    let realm = Realm::open(tmp.path()).unwrap();
    (tmp, realm)
}
