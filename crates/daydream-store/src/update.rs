//! MajorMUD's update file (`WCCUPDAT`; `WCCUPDA2` on Worldgroup 3), kept in step with the realm.
//!
//! The game merges its update file into its world at every start (and at MBBSEmu's nightly
//! cleanup): each record holds the image of one race, class, room, shop, item, spell, monster,
//! message, text block or action, and the world's record with the image's key takes the
//! image's design fields (one with no such record is added). The stock file holds an image of
//! every stock record, so a record changed in the world files alone is changed back at the
//! next start, and a deleted one comes back.
//!
//! So Daydream keeps the images in step with its edits ([`sync`]), from the realm's journal
//! ([`crate::journal::SYNC`]): an edited record's images become the record as it is now (an
//! image is added when it had none), and the images of a deleted or renumbered record's old key
//! are removed. The next start then applies the edits again instead of undoing them, and a
//! publish through the update file alone carries them into a running world. As Nightmare Redux
//! does, the file can also be written anew from the whole realm, or blank ([`compile`]).
//!
//! | | DOS (`WCCUPDAT`) | Worldgroup 3 (`WCCUPDA2`) |
//! |---|---|---|
//! | a record | `seqnum` (4 bytes), `type` (1), the image | `seqnum`, `type`, 3 bytes of padding, the image |
//! | its length | 2015 in the stock file (the game reads 2027) | 2032 |
//! | room for the image | 2010 bytes | 2024 bytes |
//!
//! The game applies the records in `seqnum` order from 1 (a record numbered 0 or below is never
//! applied), so an image added for a record that had none gets the next number. An image that
//! does not fit (a long text block in a DOS file) cannot go in: its images are removed instead,
//! so the game leaves the record as Daydream wrote it, and the report names it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use daydream_types::records::layout::{
    self, ACTION, CLASS, ITEM, Layout, LayoutSet, MONSTER, MUDMSG, RACE, ROOM, SHOP, SPELL,
    TEXTBLK, WG_UPDREC,
};
use daydream_wcc::tables::Table;

use crate::db::{StoreError, WbtrvDb};
use crate::journal;
use crate::realm::Realm;

/// The kind of record an update record holds (its `type`, `UPD_RACE` 1 .. `UPD_ACTION` 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Race = 1,
    Class = 2,
    Room = 3,
    Shop = 4,
    Item = 5,
    Spell = 6,
    Monster = 7,
    Message = 8,
    Text = 9,
    Action = 10,
}

/// The order Nightmare Redux compiles a full update file in: messages and text first, rooms
/// after the items and monsters they hold.
pub const COMPILE_ORDER: [Kind; 10] = [
    Kind::Message,
    Kind::Text,
    Kind::Race,
    Kind::Class,
    Kind::Spell,
    Kind::Item,
    Kind::Shop,
    Kind::Monster,
    Kind::Room,
    Kind::Action,
];

impl Kind {
    /// The kind of a table's records, for the ten the update file holds.
    pub fn of(table: Table) -> Option<Self> {
        Some(match table {
            Table::Races => Self::Race,
            Table::Classes => Self::Class,
            Table::Rooms => Self::Room,
            Table::Shops => Self::Shop,
            Table::Items => Self::Item,
            Table::Spells => Self::Spell,
            Table::Monsters => Self::Monster,
            Table::Messages => Self::Message,
            Table::TextBlocks => Self::Text,
            Table::Actions => Self::Action,
            _ => return None,
        })
    }

    pub fn from_code(code: u8) -> Option<Self> {
        COMPILE_ORDER.into_iter().find(|k| k.code() == code)
    }

    /// The `type` byte.
    pub fn code(self) -> u8 {
        self as u8
    }

    pub fn table(self) -> Table {
        match self {
            Self::Race => Table::Races,
            Self::Class => Table::Classes,
            Self::Room => Table::Rooms,
            Self::Shop => Table::Shops,
            Self::Item => Table::Items,
            Self::Spell => Table::Spells,
            Self::Monster => Table::Monsters,
            Self::Message => Table::Messages,
            Self::Text => Table::TextBlocks,
            Self::Action => Table::Actions,
        }
    }

    /// What a person calls it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Race => "race",
            Self::Class => "class",
            Self::Room => "room",
            Self::Shop => "shop",
            Self::Item => "item",
            Self::Spell => "spell",
            Self::Monster => "monster",
            Self::Message => "message",
            Self::Text => "text block",
            Self::Action => "action",
        }
    }

    fn layout(self, set: LayoutSet) -> &'static Layout {
        set.layout(match self {
            Self::Race => &RACE,
            Self::Class => &CLASS,
            Self::Room => &ROOM,
            Self::Shop => &SHOP,
            Self::Item => &ITEM,
            Self::Spell => &SPELL,
            Self::Monster => &MONSTER,
            Self::Message => &MUDMSG,
            Self::Text => &TEXTBLK,
            Self::Action => &ACTION,
        })
    }
}

/// The bytes of member `name` of a record laid out as `layout`.
fn member<'a>(layout: &Layout, name: &str, data: &'a [u8]) -> Option<&'a [u8]> {
    let f = layout.field(name)?;
    data.get(f.offset..f.offset + f.size)
}

fn int(bytes: &[u8]) -> i64 {
    match *bytes {
        [a, b] => i64::from(i16::from_le_bytes([a, b])),
        [a, b, c, d] => i64::from(i32::from_le_bytes([a, b, c, d])),
        _ => 0,
    }
}

/// What the game finds a record of this kind by, as bytes: the kind, then the number (rooms:
/// map and room; text blocks: number and part; actions: the verb, in any case).
pub fn key_of(kind: Kind, set: LayoutSet, data: &[u8]) -> Option<Vec<u8>> {
    let layout = kind.layout(set);
    let mut key = vec![kind.code()];
    match kind {
        Kind::Room => {
            key.extend_from_slice(member(layout, "mapnum", data)?);
            key.extend_from_slice(member(layout, "roomnum", data)?);
        }
        Kind::Text => {
            key.extend_from_slice(member(layout, "number", data)?);
            key.extend_from_slice(member(layout, "part", data)?);
        }
        Kind::Action => {
            let name = member(layout, "name", data)?;
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            key.extend(name[..end].iter().map(u8::to_ascii_lowercase));
        }
        _ => key.extend_from_slice(member(layout, "number", data)?),
    }
    Some(key)
}

/// A record of this kind, for a person: `item 12`, `room 1/5`, `text block 3 part 2`.
pub fn describe(kind: Kind, set: LayoutSet, data: &[u8]) -> String {
    let layout = kind.layout(set);
    let n = |name| member(layout, name, data).map_or(0, int);
    match kind {
        Kind::Room => format!("room {}/{}", n("mapnum"), n("roomnum")),
        Kind::Text => format!("text block {} part {}", n("number"), n("part")),
        Kind::Action => {
            let name = member(layout, "name", data).unwrap_or(&[]);
            let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            format!("action {}", String::from_utf8_lossy(&name[..end]))
        }
        k => format!("{} {}", k.name(), n("number")),
    }
}

/// A realm's update file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFile {
    /// `WCCUPDAT`, or `WCCUPDA2`.
    pub stem: String,
    pub path: std::path::PathBuf,
    /// The layouts of its images: the Worldgroup 3 build's for a file of 2032-byte records.
    pub set: LayoutSet,
    record_length: usize,
    header: usize,
}

impl UpdateFile {
    /// The realm's update file, if it has one.
    pub fn find(realm: &Realm) -> Result<Option<Self>, StoreError> {
        for stem in ["WCCUPDA2", "WCCUPDAT"] {
            let Some(f) = realm.file(stem) else {
                continue;
            };
            let record_length = WbtrvDb::open_read_only(&f.path)?.metadata().record_length;
            let set = if record_length == WG_UPDREC.len {
                LayoutSet::Worldgroup
            } else {
                LayoutSet::Dos
            };
            let header = set
                .layout(&layout::UPDREC)
                .field("data")
                .map_or(5, |d| d.offset);
            return Ok(Some(Self {
                stem: f.stem.clone(),
                path: f.path.clone(),
                set,
                record_length,
                header,
            }));
        }
        Ok(None)
    }

    /// How many bytes of an image a record holds.
    pub fn room(&self) -> usize {
        self.record_length.saturating_sub(self.header)
    }

    /// An update record: `seqnum`, `type` and `image`, zero filled; `None` when the image does
    /// not fit. `like` is a record whose padding to keep.
    fn record(&self, seqnum: i32, kind: u8, image: &[u8], like: Option<&[u8]>) -> Option<Vec<u8>> {
        if image.len() > self.room() {
            return None;
        }
        let mut r = vec![0; self.record_length];
        if let Some(old) = like {
            let n = self.header.min(old.len());
            r[..n].copy_from_slice(&old[..n]);
        }
        r[..4].copy_from_slice(&seqnum.to_le_bytes());
        r[4] = kind;
        r[self.header..self.header + image.len()].copy_from_slice(image);
        Some(r)
    }

    /// A record's `seqnum`, `type` and image.
    fn parts<'a>(&self, record: &'a [u8]) -> Option<(i32, u8, &'a [u8])> {
        let seq = i32::from_le_bytes(record.get(..4)?.try_into().ok()?);
        Some((seq, *record.get(4)?, record.get(self.header..)?))
    }
}

/// What bringing the update file in step did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The update file's stem; `None` when the realm has none (nothing to do).
    pub file: Option<String>,
    /// Images replaced by the record as it is now.
    pub replaced: usize,
    /// Images added.
    pub added: usize,
    /// Images removed: of deleted or renumbered records, and of records too long for one.
    pub removed: usize,
    /// Records too long for an image, whose images were removed (`text block 3 part 1`).
    pub too_long: Vec<String>,
    /// The file's records afterwards.
    pub records: u64,
}

impl Report {
    /// Whether the file was written.
    pub fn changed(&self) -> bool {
        self.replaced + self.added + self.removed > 0
    }
}

/// The net change of one key: the record as it is now, or `None` when it is gone.
type Changes = BTreeMap<(usize, Vec<u8>), (Kind, Option<Vec<u8>>)>;

fn order(kind: Kind) -> usize {
    COMPILE_ORDER
        .iter()
        .position(|&k| k == kind)
        .unwrap_or(usize::MAX)
}

/// The keys the realm's journal says changed since the last sync, with what holds each now.
fn changes(
    realm: &Realm,
    set: LayoutSet,
    entries: &[journal::Entry],
) -> Result<Changes, StoreError> {
    let mut dbs: HashMap<&str, WbtrvDb> = HashMap::new();
    let mut gone: Vec<(Kind, Vec<u8>)> = Vec::new();
    let mut now: Changes = BTreeMap::new();
    for e in entries {
        let Some(f) = realm.file(&e.stem) else {
            continue;
        };
        let Some(kind) = f.table.and_then(Kind::of) else {
            continue;
        };
        if !dbs.contains_key(e.stem.as_str()) {
            dbs.insert(e.stem.as_str(), WbtrvDb::open_read_only(&f.path)?);
        }
        let after = dbs[e.stem.as_str()].get(e.id)?;
        if after == e.before {
            continue;
        }
        if let Some(key) = e.before.as_deref().and_then(|b| key_of(kind, set, b)) {
            gone.push((kind, key));
        }
        if let Some(a) = after
            && let Some(key) = key_of(kind, set, &a)
        {
            now.insert((order(kind), key), (kind, Some(a)));
        }
    }
    for (kind, key) in gone {
        now.entry((order(kind), key)).or_insert((kind, None));
    }
    Ok(now)
}

/// Update records by the key of their image: `(id, record)`.
type Images = HashMap<Vec<u8>, Vec<(i64, Vec<u8>)>>;

/// The images of the keys wanted, by key, and the highest `seqnum` of the file.
fn scan(
    db: &WbtrvDb,
    file: &UpdateFile,
    wanted: &HashSet<&Vec<u8>>,
) -> Result<(Images, i32), StoreError> {
    let mut found = Images::new();
    let mut max_seq = 0;
    let mut after = 0;
    loop {
        let page = db.page(after, 1000)?;
        let Some((last, _)) = page.last() else {
            break;
        };
        after = *last;
        for (id, record) in page {
            let Some((seq, code, image)) = file.parts(&record) else {
                continue;
            };
            max_seq = max_seq.max(seq);
            let Some(key) = Kind::from_code(code).and_then(|k| key_of(k, file.set, image)) else {
                continue;
            };
            if wanted.contains(&key) {
                found.entry(key).or_default().push((id, record));
            }
        }
    }
    Ok((found, max_seq))
}

/// Brings the update file of the realm in `realm_dir` in step with the edits its journal
/// recorded since the last time, and clears them from the journal. Cheap when nothing changed:
/// one look at the journal.
pub fn sync(realm_dir: &Path) -> Result<Report, StoreError> {
    let pending = journal::pending(realm_dir, journal::SYNC)?;
    if pending.entries.is_empty() {
        return Ok(Report::default());
    }
    let realm = Realm::open(realm_dir)?;
    let report = match UpdateFile::find(&realm)? {
        Some(file) => {
            let changes = changes(&realm, file.set, &pending.entries)?;
            apply(&file, &changes)?
        }
        None => Report::default(),
    };
    journal::clear(realm_dir, journal::SYNC, Some(pending.mark))?;
    Ok(report)
}

fn apply(file: &UpdateFile, changes: &Changes) -> Result<Report, StoreError> {
    let mut report = Report {
        file: Some(file.stem.clone()),
        ..Report::default()
    };
    let mut db = WbtrvDb::open(&file.path)?;
    // the game never writes it: published whole, so nothing to journal
    db.track(false);
    if !changes.is_empty() {
        let wanted: HashSet<&Vec<u8>> = changes.keys().map(|(_, k)| k).collect();
        let (images, max_seq) = scan(&db, file, &wanted)?;
        let mut next = max_seq.max(0);
        db.begin()?;
        let written = (|| {
            for ((_, key), (kind, now)) in changes {
                let old = images.get(key).map_or(&[][..], Vec::as_slice);
                let Some(record) = now else {
                    for (id, _) in old {
                        db.delete(*id)?;
                        report.removed += 1;
                    }
                    continue;
                };
                if file.record(0, kind.code(), record, None).is_none() {
                    report.too_long.push(describe(*kind, file.set, record));
                    for (id, _) in old {
                        db.delete(*id)?;
                        report.removed += 1;
                    }
                    continue;
                }
                if old.is_empty() {
                    next += 1;
                    let r = file.record(next, kind.code(), record, None).expect("fits");
                    db.insert(&r)?;
                    report.added += 1;
                }
                for (id, was) in old {
                    let seq = file.parts(was).map_or(0, |p| p.0);
                    let r = file
                        .record(seq, kind.code(), record, Some(was))
                        .expect("fits");
                    if r != *was {
                        db.update(*id, &r)?;
                        report.replaced += 1;
                    }
                }
            }
            Ok(())
        })();
        match written {
            Ok(()) => db.commit()?,
            Err(e) => {
                let _ = db.rollback();
                return Err(e);
            }
        }
    }
    report.records = db.count()?;
    Ok(report)
}

/// How to write the update file anew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compile {
    /// An image of every record the file can hold, in [`COMPILE_ORDER`], numbered from 1:
    /// Nightmare Redux's "Compile Full Update File".
    Full,
    /// One record the game never applies (`seqnum` 0, `type` 0): its "Compile Blank Update
    /// File". The next start then changes nothing.
    Blank,
}

/// Writes the update file of the realm in `realm_dir` anew, and clears the realm's
/// [`journal::SYNC`] journal (the file now holds every edit).
pub fn compile(realm_dir: &Path, how: Compile) -> Result<Report, StoreError> {
    let realm = Realm::open(realm_dir)?;
    let Some(file) = UpdateFile::find(&realm)? else {
        return Ok(Report::default());
    };
    let mut report = Report {
        file: Some(file.stem.clone()),
        ..Report::default()
    };
    let mut records = Vec::new();
    match how {
        Compile::Blank => {
            records.push(file.record(0, 0, &[], None).expect("an empty image fits"));
        }
        Compile::Full => {
            let mut seq = 0;
            for kind in COMPILE_ORDER {
                for f in realm.files_of(kind.table()) {
                    for (_, data) in WbtrvDb::open_read_only(&f.path)?.records()? {
                        match file.record(seq + 1, kind.code(), &data, None) {
                            Some(r) => {
                                seq += 1;
                                records.push(r);
                            }
                            None => report.too_long.push(describe(kind, file.set, &data)),
                        }
                    }
                }
            }
        }
    }
    let mut db = WbtrvDb::open(&file.path)?;
    db.track(false);
    db.begin()?;
    let written = (|| {
        report.removed = db.clear()? as usize;
        for r in &records {
            db.insert(r)?;
        }
        Ok(())
    })();
    match written {
        Ok(()) => db.commit()?,
        Err(e) => {
            let _ = db.rollback();
            return Err(e);
        }
    }
    report.added = records.len();
    report.records = db.count()?;
    journal::clear(realm_dir, journal::SYNC, None)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Metadata;
    use crate::key::{Key, Segment, attr, ty};

    fn key(offset: u16, length: u16, data_type: u8, attributes: u16) -> Key {
        Key {
            segments: vec![Segment {
                number: 0,
                index: 0,
                attributes,
                data_type,
                offset,
                length,
                null_value: 0,
                acs: None,
            }],
            acs_name: None,
            acs: None,
        }
    }

    fn meta(record_length: usize, variable: bool) -> Metadata {
        Metadata {
            record_length,
            physical_record_length: record_length + 4,
            page_length: 4096,
            variable_length_records: variable,
            version: 4,
        }
    }

    /// An item of `len` bytes: its number, and `mark` at 0x10.
    fn item(len: usize, number: i32, mark: &[u8]) -> Vec<u8> {
        let mut r = vec![0; len];
        r[..4].copy_from_slice(&number.to_le_bytes());
        r[0x10..0x10 + mark.len()].copy_from_slice(mark);
        r
    }

    fn text(number: i32, part: i16, text: &[u8]) -> Vec<u8> {
        let mut r = vec![0; 0x16 + text.len() + 1];
        r[..2].copy_from_slice(&part.to_le_bytes());
        r[0x0E..0x12].copy_from_slice(&number.to_le_bytes());
        r[0x16..0x16 + text.len()].copy_from_slice(text);
        r
    }

    struct Fixture {
        dir: tempfile::TempDir,
        set: LayoutSet,
        upd: &'static str,
        upd_len: usize,
        header: usize,
    }

    impl Fixture {
        /// A realm of items 1-4, one room and one text block, and an update file holding an
        /// image of each (items first, numbered 1-6).
        fn new(set: LayoutSet) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let (items, upd, upd_len, header) = match set {
                LayoutSet::Dos => ("WCCITEMS", "WCCUPDAT", 2015, 5),
                LayoutSet::Worldgroup => ("WCCITEM2", "WCCUPDA2", 2032, 8),
            };
            let f = Self {
                dir,
                set,
                upd,
                upd_len,
                header,
            };
            let ilen = set.layout(&ITEM).len;
            let mut db = WbtrvDb::create(
                &f.path(items),
                meta(ilen, false),
                vec![key(0, 4, ty::INTEGER, attr::MODIFIABLE)],
            )
            .unwrap();
            let mut images = Vec::new();
            for n in 1..=4 {
                let r = item(ilen, n, b"stock");
                db.insert(&r).unwrap();
                images.push((5, r));
            }
            let rlen = set.layout(&ROOM).len;
            let mut room = vec![0; rlen];
            room[..4].copy_from_slice(&1i32.to_le_bytes());
            room[4..8].copy_from_slice(&7i32.to_le_bytes());
            WbtrvDb::create(&f.path("WCCMP001"), meta(rlen, false), vec![])
                .unwrap()
                .insert(&room)
                .unwrap();
            images.push((3, room));
            if set == LayoutSet::Dos {
                let t = text(3, 1, b"short");
                WbtrvDb::create(&f.path("WCCTEXT"), meta(22, true), vec![])
                    .unwrap()
                    .insert(&t)
                    .unwrap();
                images.push((9, t));
            }
            let mut u = WbtrvDb::create(
                &f.path(upd),
                meta(upd_len, false),
                vec![key(0, 4, ty::AUTO_INC, 0)],
            )
            .unwrap();
            for (i, (kind, image)) in images.iter().enumerate() {
                u.insert(&f.image(i as i32 + 1, *kind, image)).unwrap();
            }
            f
        }

        fn path(&self, stem: &str) -> std::path::PathBuf {
            self.dir.path().join(format!("{stem}.db"))
        }

        fn items(&self) -> WbtrvDb {
            let stem = match self.set {
                LayoutSet::Dos => "WCCITEMS",
                LayoutSet::Worldgroup => "WCCITEM2",
            };
            WbtrvDb::open(&self.path(stem)).unwrap()
        }

        fn image(&self, seq: i32, kind: u8, image: &[u8]) -> Vec<u8> {
            let mut r = vec![0; self.upd_len];
            r[..4].copy_from_slice(&seq.to_le_bytes());
            r[4] = kind;
            r[self.header..self.header + image.len()].copy_from_slice(image);
            r
        }

        /// The update file's records as (seqnum, type, the image's first 4 bytes, mark).
        fn update_file(&self) -> Vec<(i32, u8, i32, Vec<u8>)> {
            let db = WbtrvDb::open_read_only(&self.path(self.upd)).unwrap();
            let mut out: Vec<_> = db
                .records()
                .unwrap()
                .into_iter()
                .map(|(_, r)| {
                    let h = self.header;
                    (
                        i32::from_le_bytes(r[..4].try_into().unwrap()),
                        r[4],
                        i32::from_le_bytes(r[h..h + 4].try_into().unwrap()),
                        r[h + 0x10..h + 0x15].to_vec(),
                    )
                })
                .collect();
            out.sort();
            out
        }
    }

    fn find(db: &WbtrvDb, number: i32) -> i64 {
        db.records()
            .unwrap()
            .into_iter()
            .find(|(_, r)| r[..4] == number.to_le_bytes())
            .unwrap()
            .0
    }

    fn edit_items(f: &Fixture) {
        let mut db = f.items();
        let len = db.metadata().record_length;
        // edited, deleted, added and renumbered
        db.update(find(&db, 2), &item(len, 2, b"EDIT2")).unwrap();
        db.delete(find(&db, 3)).unwrap();
        db.insert(&item(len, 9, b"new 9")).unwrap();
        db.update(find(&db, 4), &item(len, 40, b"stock")).unwrap();
    }

    #[test]
    fn edits_replace_add_and_remove_images() {
        for set in [LayoutSet::Dos, LayoutSet::Worldgroup] {
            let f = Fixture::new(set);
            edit_items(&f);
            let r = sync(f.dir.path()).unwrap();
            assert_eq!(r.file.as_deref(), Some(f.upd));
            assert_eq!((r.replaced, r.added, r.removed), (1, 2, 2), "{set:?}");
            let upd = f.update_file();
            let items: Vec<_> = upd
                .iter()
                .filter(|u| u.1 == 5)
                .map(|u| (u.0, u.2, u.3.as_slice()))
                .collect();
            let last = if set == LayoutSet::Dos { 6 } else { 5 };
            assert_eq!(
                items,
                vec![
                    (1, 1, &b"stock"[..]),
                    (2, 2, b"EDIT2"),
                    // new images follow the last: items before rooms
                    (last + 1, 9, b"new 9"),
                    (last + 2, 40, b"stock"),
                ],
                "{set:?}"
            );
            // the room's image was left alone
            assert!(upd.iter().any(|u| u.1 == 3 && u.2 == 1));
            // the journal was cleared: a second sync does nothing
            assert_eq!(sync(f.dir.path()).unwrap(), Report::default());
        }
    }

    #[test]
    fn a_change_undone_before_the_sync_changes_nothing() {
        let f = Fixture::new(LayoutSet::Dos);
        let mut db = f.items();
        let len = db.metadata().record_length;
        let id = find(&db, 1);
        db.update(id, &item(len, 1, b"twice")).unwrap();
        db.update(id, &item(len, 1, b"stock")).unwrap();
        let before = f.update_file();
        let r = sync(f.dir.path()).unwrap();
        assert!(!r.changed(), "{r:?}");
        assert_eq!(f.update_file(), before);
    }

    #[test]
    fn a_record_too_long_for_an_image_loses_its_image() {
        let f = Fixture::new(LayoutSet::Dos);
        let mut db = WbtrvDb::open(&f.path("WCCTEXT")).unwrap();
        let long = text(3, 1, &[b'x'; 1999]);
        assert!(long.len() > 2010);
        db.update(1, &long).unwrap();
        let r = sync(f.dir.path()).unwrap();
        assert_eq!(r.too_long, vec!["text block 3 part 1".to_string()]);
        assert_eq!(r.removed, 1);
        assert!(f.update_file().iter().all(|u| u.1 != 9));
    }

    #[test]
    fn a_realm_without_an_update_file_just_forgets_its_edits() {
        let f = Fixture::new(LayoutSet::Dos);
        std::fs::remove_file(f.path("WCCUPDAT")).unwrap();
        edit_items(&f);
        assert_eq!(sync(f.dir.path()).unwrap(), Report::default());
        assert!(
            journal::pending(f.dir.path(), journal::SYNC)
                .unwrap()
                .entries
                .is_empty()
        );
        // the base journal keeps them
        assert_eq!(
            journal::pending(f.dir.path(), journal::BASE)
                .unwrap()
                .entries
                .len(),
            4
        );
    }

    #[test]
    fn a_full_compile_holds_every_record_in_nmr_order() {
        let f = Fixture::new(LayoutSet::Dos);
        edit_items(&f);
        let r = compile(f.dir.path(), Compile::Full).unwrap();
        assert_eq!((r.removed, r.added, r.records), (6, 6, 6));
        let kinds: Vec<(i32, u8, i32)> = f.update_file().iter().map(|u| (u.0, u.1, u.2)).collect();
        // text (9; its first bytes are the part), items (5) in the file's order, then the room
        assert_eq!(
            kinds,
            vec![
                (1, 9, 1),
                (2, 5, 1),
                (3, 5, 2),
                (4, 5, 40),
                (5, 5, 9),
                (6, 3, 1)
            ]
        );
        assert!(
            journal::pending(f.dir.path(), journal::SYNC)
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn a_blank_compile_leaves_one_record_the_game_never_applies() {
        let f = Fixture::new(LayoutSet::Worldgroup);
        let r = compile(f.dir.path(), Compile::Blank).unwrap();
        assert_eq!(r.records, 1);
        assert_eq!(f.update_file(), vec![(0, 0, 0, vec![0; 5])]);
        // an edit afterwards still gets its image, numbered from 1
        let mut db = f.items();
        let len = db.metadata().record_length;
        db.update(find(&db, 1), &item(len, 1, b"after")).unwrap();
        let r = sync(f.dir.path()).unwrap();
        assert_eq!(r.added, 1);
        assert_eq!(f.update_file()[1], (1, 5, 1, b"after".to_vec()));
    }

    #[test]
    fn keys_follow_the_game_lookups() {
        let set = LayoutSet::Dos;
        let mut a = vec![0; ACTION.len];
        a[..5].copy_from_slice(b"SMILE");
        let mut b = a.clone();
        b[..5].copy_from_slice(b"smile");
        b[100] = 1;
        assert_eq!(key_of(Kind::Action, set, &a), key_of(Kind::Action, set, &b));
        assert_eq!(describe(Kind::Action, set, &a), "action SMILE");
        let t = text(12, 2, b"hi");
        assert_eq!(
            key_of(Kind::Text, set, &t).unwrap(),
            [&[9u8][..], &12i32.to_le_bytes(), &2i16.to_le_bytes()].concat()
        );
        let mut s = vec![0; SPELL.len];
        s[..2].copy_from_slice(&300i16.to_le_bytes());
        assert_eq!(describe(Kind::Spell, set, &s), "spell 300");
        assert_eq!(Kind::from_code(7), Some(Kind::Monster));
        assert_eq!(Kind::from_code(0), None);
    }
}
