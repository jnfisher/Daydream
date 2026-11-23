//! Moving a realm's world into silvermere's database and back.
//!
//! Each world table is a [`Kind`]: the realm file it lives in, its flat record, silvermere's
//! row and the way between them. [`export`] decodes the realm's records into rows and
//! writes them as `silvermere import` does (`put_many`, one transaction per table, inside
//! `world_import`'s begin and finish so a cut-off export leaves a world silvermere refuses to
//! serve), deletes the rows the realm no longer has, and keeps each record's bytes in
//! `daydream_records` ([`crate::raw`]). [`import`] reads the rows back and rebuilds each
//! record over its base ([`crate::records`]).

use std::collections::{BTreeMap, HashMap};

use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::{
    Action, Class, Item, KnownMonster, LayoutSet, Message, Race, Room, Shop, Spell, TextBlock,
};
use daydream_wcc::decode::{
    self, ActionRecord, ClassRecord, DecodeError, ItemRecord, LegacyClock, MonsterRecord,
    MudmsgRecord, RaceRecord, Record, RoomRecord, ShopRecord, SpellRecord, TextblkRecord,
};
use daydream_wcc::tables::Table;
use jiff::tz::TimeZone;
use serde::Serialize;

use crate::db::repo::StateWrite;
use crate::db::{Db, Result as DbResult};
use crate::records as rec;
use crate::{Error, Result, raw};

/// How a table's rows are moved.
pub(crate) struct Ctx {
    pub set: LayoutSet,
    pub clock: LegacyClock,
}

impl Ctx {
    pub fn new(set: LayoutSet, tz: TimeZone) -> Self {
        Self {
            set,
            clock: LegacyClock::new(tz, jiff::Timestamp::now()),
        }
    }
}

/// One world table: a realm record type and silvermere's row for it.
pub(crate) trait Kind {
    /// silvermere's table (`rooms`), which also names the kind in reports.
    const NAME: &'static str;
    /// The realm's record type.
    const TABLE: Table;
    type Row: Clone + PartialEq + Serialize;
    type Rec: Record;

    /// The row's key, as text: `map/room`, `number`, `number/part` or `name`.
    fn key(row: &Self::Row) -> String;
    fn to_row(r: &Self::Rec, ctx: &Ctx) -> std::result::Result<Self::Row, DecodeError>;
    fn apply(r: &mut Self::Rec, new: &Self::Row, old: &Self::Row, ctx: &Ctx);
    async fn all(db: &Db) -> DbResult<Vec<Self::Row>>;
    async fn put_many(db: &Db, rows: &[Self::Row], state: StateWrite) -> DbResult<()>;
    /// `DELETE ... WHERE` and its arguments for one key.
    fn delete(row: &Self::Row) -> (String, Vec<KeyArg>);

    /// Decodes a stored record.
    fn decode(raw: &[u8], set: LayoutSet) -> std::result::Result<Self::Rec, DecodeError> {
        Self::Rec::decode_as(raw, set)
    }
    /// An empty record, for a row that has no record to start from.
    fn blank(set: LayoutSet) -> Vec<u8> {
        vec![0; Self::Rec::layout(set).len]
    }
    /// `base` with `new` applied where it differs from `old`.
    fn rebuild(
        base: &[u8],
        new: &Self::Row,
        old: &Self::Row,
        ctx: &Ctx,
    ) -> std::result::Result<Vec<u8>, DecodeError> {
        let mut r = Self::decode(base, ctx.set)?;
        Self::apply(&mut r, new, old, ctx);
        Ok(encode_over(&r, base, ctx.set))
    }
}

/// A bound key value of a delete.
#[derive(Debug, Clone)]
pub(crate) enum KeyArg {
    Int(i64),
    Text(String),
}

fn by_number(table: &str, n: i64) -> (String, Vec<KeyArg>) {
    (
        format!("DELETE FROM {table} WHERE number = $1"),
        vec![KeyArg::Int(n)],
    )
}

macro_rules! numbered {
    (
        $kind:ident, $name:literal, $table:ident, $row:ty, $rec:ty, $to:expr, $apply:expr,
        $repo:ident, |$db:ident, $rows:ident, $state:ident| $put:expr
    ) => {
        pub(crate) struct $kind;
        impl Kind for $kind {
            const NAME: &'static str = $name;
            const TABLE: Table = Table::$table;
            type Row = $row;
            type Rec = $rec;
            fn key(row: &$row) -> String {
                row.number.0.to_string()
            }
            fn to_row(r: &$rec, _ctx: &Ctx) -> std::result::Result<$row, DecodeError> {
                Ok($to(r))
            }
            fn apply(r: &mut $rec, new: &$row, old: &$row, _ctx: &Ctx) {
                $apply(r, new, old)
            }
            async fn all(db: &Db) -> DbResult<Vec<$row>> {
                db.$repo().all().await
            }
            #[allow(unused_variables)]
            async fn put_many($db: &Db, $rows: &[$row], $state: StateWrite) -> DbResult<()> {
                $put
            }
            fn delete(row: &$row) -> (String, Vec<KeyArg>) {
                by_number($name, i64::from(row.number.0))
            }
        }
    };
}

numbered!(
    Items,
    "items",
    Items,
    Item,
    ItemRecord,
    decode::to_item,
    rec::apply_item,
    items,
    |db, rows, state| db.items().put_many(rows, state).await
);
numbered!(
    Spells,
    "spells",
    Spells,
    Spell,
    SpellRecord,
    decode::to_spell,
    rec::apply_spell,
    spells,
    |db, rows, state| db.spells().put_many(rows).await
);
numbered!(
    Classes,
    "classes",
    Classes,
    Class,
    ClassRecord,
    decode::to_class,
    rec::apply_class,
    classes,
    |db, rows, state| db.classes().put_many(rows).await
);
numbered!(
    Races,
    "races",
    Races,
    Race,
    RaceRecord,
    decode::to_race,
    rec::apply_race,
    races,
    |db, rows, state| db.races().put_many(rows).await
);
numbered!(
    Shops,
    "shops",
    Shops,
    Shop,
    ShopRecord,
    decode::to_shop,
    rec::apply_shop,
    shops,
    |db, rows, state| db.shops().put_many(rows, state).await
);
numbered!(
    Messages,
    "messages",
    Messages,
    Message,
    MudmsgRecord,
    decode::to_message,
    rec::apply_message,
    messages,
    |db, rows, state| db.messages().put_many(rows).await
);

pub(crate) struct Monsters;
impl Kind for Monsters {
    const NAME: &'static str = "monsters";
    const TABLE: Table = Table::Monsters;
    type Row = KnownMonster;
    type Rec = MonsterRecord;
    fn key(row: &KnownMonster) -> String {
        row.number.0.to_string()
    }
    fn to_row(r: &MonsterRecord, ctx: &Ctx) -> std::result::Result<KnownMonster, DecodeError> {
        decode::to_monster(r, &ctx.clock)
    }
    fn apply(r: &mut MonsterRecord, new: &KnownMonster, old: &KnownMonster, ctx: &Ctx) {
        rec::apply_monster(r, new, old, &ctx.clock.tz)
    }
    async fn all(db: &Db) -> DbResult<Vec<KnownMonster>> {
        db.monsters().all().await
    }
    async fn put_many(db: &Db, rows: &[KnownMonster], state: StateWrite) -> DbResult<()> {
        db.monsters().put_many(rows, state).await
    }
    fn delete(row: &KnownMonster) -> (String, Vec<KeyArg>) {
        by_number("monsters", i64::from(row.number.0))
    }
}

pub(crate) struct Rooms;
impl Kind for Rooms {
    const NAME: &'static str = "rooms";
    const TABLE: Table = Table::Rooms;
    type Row = Room;
    type Rec = RoomRecord;
    fn key(row: &Room) -> String {
        format!("{}/{}", row.key.map.0, row.key.room.0)
    }
    fn to_row(r: &RoomRecord, _ctx: &Ctx) -> std::result::Result<Room, DecodeError> {
        Ok(decode::to_room(r))
    }
    fn apply(r: &mut RoomRecord, new: &Room, old: &Room, _ctx: &Ctx) {
        rec::apply_room(r, new, old)
    }
    async fn all(db: &Db) -> DbResult<Vec<Room>> {
        db.rooms().all().await
    }
    async fn put_many(db: &Db, rows: &[Room], state: StateWrite) -> DbResult<()> {
        db.rooms().put_many(rows, state).await
    }
    fn delete(row: &Room) -> (String, Vec<KeyArg>) {
        (
            "DELETE FROM rooms WHERE map = $1 AND room = $2".into(),
            vec![
                KeyArg::Int(i64::from(row.key.map.0)),
                KeyArg::Int(i64::from(row.key.room.0)),
            ],
        )
    }
}

pub(crate) struct TextBlocks;
impl Kind for TextBlocks {
    const NAME: &'static str = "text_blocks";
    const TABLE: Table = Table::TextBlocks;
    type Row = TextBlock;
    type Rec = TextblkRecord;
    fn key(row: &TextBlock) -> String {
        format!("{}/{}", row.number.0, row.part)
    }
    fn to_row(r: &TextblkRecord, _ctx: &Ctx) -> std::result::Result<TextBlock, DecodeError> {
        Ok(decode::to_text_block(r))
    }
    fn apply(_r: &mut TextblkRecord, _new: &TextBlock, _old: &TextBlock, _ctx: &Ctx) {
        unreachable!("text blocks are rebuilt by rebuild()")
    }
    async fn all(db: &Db) -> DbResult<Vec<TextBlock>> {
        db.text_blocks().all().await
    }
    async fn put_many(db: &Db, rows: &[TextBlock], _state: StateWrite) -> DbResult<()> {
        db.text_blocks().put_many(rows).await
    }
    fn delete(row: &TextBlock) -> (String, Vec<KeyArg>) {
        (
            "DELETE FROM text_blocks WHERE number = $1 AND part = $2".into(),
            vec![
                KeyArg::Int(i64::from(row.number.0)),
                KeyArg::Int(i64::from(row.part)),
            ],
        )
    }
    fn decode(raw: &[u8], set: LayoutSet) -> std::result::Result<TextblkRecord, DecodeError> {
        decode::decode_textblk_as(raw, set)
    }
    fn blank(set: LayoutSet) -> Vec<u8> {
        // the fixed part and an empty text's NUL
        vec![0; text_offset(set) + 1]
    }
    fn rebuild(
        base: &[u8],
        new: &TextBlock,
        old: &TextBlock,
        ctx: &Ctx,
    ) -> std::result::Result<Vec<u8>, DecodeError> {
        let mut r = Self::decode(base, ctx.set)?;
        let fixed = text_offset(ctx.set);
        let len = rec::apply_text_block(&mut r, new, old, fixed, base.len());
        let mut out = encode_over(&r, base, ctx.set);
        if new.text == old.text {
            // the stored bytes after the text, as they were
            out.truncate(fixed);
            out.extend_from_slice(&base[fixed.min(base.len())..]);
        } else {
            out.truncate(len);
        }
        Ok(out)
    }
}

/// `r` written over `base`: the bytes no member covers (a Worldgroup layout's alignment
/// padding, and anything past the layout) stay as they were.
fn encode_over<R: Record>(r: &R, base: &[u8], set: LayoutSet) -> Vec<u8> {
    let len = R::layout(set).len;
    let mut out = base.to_vec();
    if out.len() < len {
        out.resize(len, 0);
    }
    r.write_fields(&mut out[..len], R::offsets(set));
    out
}

/// Where a text block's text starts: its record's fixed part.
fn text_offset(set: LayoutSet) -> usize {
    TextblkRecord::layout(set)
        .field("text")
        .map(|f| f.offset)
        .unwrap_or(22)
}

pub(crate) struct Actions;
impl Kind for Actions {
    const NAME: &'static str = "actions";
    const TABLE: Table = Table::Actions;
    type Row = Action;
    type Rec = ActionRecord;
    fn key(row: &Action) -> String {
        row.name.clone()
    }
    fn to_row(r: &ActionRecord, _ctx: &Ctx) -> std::result::Result<Action, DecodeError> {
        Ok(decode::to_action(r))
    }
    fn apply(r: &mut ActionRecord, new: &Action, old: &Action, _ctx: &Ctx) {
        rec::apply_action(r, new, old)
    }
    async fn all(db: &Db) -> DbResult<Vec<Action>> {
        db.actions().all().await
    }
    async fn put_many(db: &Db, rows: &[Action], _state: StateWrite) -> DbResult<()> {
        db.actions().put_many(rows).await
    }
    fn delete(row: &Action) -> (String, Vec<KeyArg>) {
        (
            "DELETE FROM actions WHERE name = $1".into(),
            vec![KeyArg::Text(row.name.clone())],
        )
    }
}

/// What happened to one table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TableReport {
    /// silvermere's table (`rooms`, `items`, ...).
    pub table: &'static str,
    /// Rows the destination did not have.
    pub inserted: usize,
    /// Rows the destination had, written again (an export) or changed (an import).
    pub updated: usize,
    /// Records that were already the same (an import only).
    pub unchanged: usize,
    /// Rows the destination had and the source does not.
    pub deleted: usize,
}

/// One realm record: where it is and its bytes.
struct Held {
    file: usize,
    id: i64,
    bytes: Vec<u8>,
}

/// A table's records in a realm, by key.
struct RealmTable<K: Kind> {
    files: Vec<std::path::PathBuf>,
    by_key: BTreeMap<String, Held>,
    rows: Vec<K::Row>,
}

fn read_realm<K: Kind>(realm: &Realm, ctx: &Ctx) -> Result<RealmTable<K>> {
    let mut t = RealmTable {
        files: Vec::new(),
        by_key: BTreeMap::new(),
        rows: Vec::new(),
    };
    let mut at: HashMap<String, usize> = HashMap::new();
    for f in realm.files_of(K::TABLE) {
        let file = t.files.len();
        t.files.push(f.path.clone());
        let db = WbtrvDb::open_read_only(&f.path)?;
        for (id, bytes) in db.records()? {
            let r = K::decode(&bytes, ctx.set)
                .map_err(|e| Error::Record(format!("{} record {id}: {e}", f.stem)))?;
            let row = K::to_row(&r, ctx)
                .map_err(|e| Error::Record(format!("{} record {id}: {e}", f.stem)))?;
            let key = K::key(&row);
            // a key held twice: the last record wins, as silvermere's import keeps it
            match at.get(&key) {
                Some(&i) => t.rows[i] = row,
                None => {
                    at.insert(key.clone(), t.rows.len());
                    t.rows.push(row);
                }
            }
            t.by_key.insert(key, Held { file, id, bytes });
        }
    }
    Ok(t)
}

/// What an export or import does with state (room floors, shop stock, monster kills,
/// items in play).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    /// Export: rows silvermere has keep its live state; new rows take the realm's.
    #[default]
    Keep,
    /// Export: every row's state is rewritten from the realm.
    Reset,
}

impl State {
    fn write(self) -> StateWrite {
        match self {
            Self::Keep => StateWrite::CreateMissing,
            Self::Reset => StateWrite::Reset,
        }
    }
}

pub(crate) async fn export_table<K: Kind>(
    realm: &Realm,
    db: &Db,
    ctx: &Ctx,
    opts: &crate::ExportOptions,
) -> Result<TableReport> {
    let t = read_realm::<K>(realm, ctx)?;
    let have = K::all(db).await?;
    let mut report = TableReport {
        table: K::NAME,
        ..TableReport::default()
    };
    let theirs: HashMap<String, &K::Row> = have.iter().map(|r| (K::key(r), r)).collect();
    for row in &t.rows {
        if theirs.contains_key(&K::key(row)) {
            report.updated += 1;
        } else {
            report.inserted += 1;
        }
    }
    let gone: Vec<&K::Row> = have
        .iter()
        .filter(|r| !t.by_key.contains_key(&K::key(r)))
        .collect();
    if opts.delete {
        report.deleted = gone.len();
    }
    if opts.dry_run {
        return Ok(report);
    }
    K::put_many(db, &t.rows, opts.state.write()).await?;
    if opts.delete && !gone.is_empty() {
        let mut tx = db.pool().begin().await.map_err(crate::db::DbError::from)?;
        for r in gone {
            let (sql, args) = K::delete(r);
            let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
            for a in args {
                q = match a {
                    KeyArg::Int(v) => q.bind(v),
                    KeyArg::Text(v) => q.bind(v),
                };
            }
            q.execute(&mut *tx)
                .await
                .map_err(crate::db::DbError::from)?;
        }
        tx.commit().await.map_err(crate::db::DbError::from)?;
    }
    let records: Vec<(String, &[u8])> = if opts.raw {
        t.by_key
            .iter()
            .map(|(k, h)| (k.clone(), h.bytes.as_slice()))
            .collect()
    } else {
        Vec::new()
    };
    raw::replace(db, K::NAME, &records).await?;
    Ok(report)
}

/// The rows `rows` read back as silvermere stores them: written to `scratch` (an empty
/// in-memory silvermere database) and read again, so a row compares equal to what the same
/// row read from silvermere's own database gives.
async fn as_stored<K: Kind>(scratch: &Db, rows: &[K::Row]) -> Result<HashMap<String, K::Row>> {
    if rows.is_empty() {
        return Ok(HashMap::new());
    }
    K::put_many(scratch, rows, StateWrite::Reset).await?;
    let back = K::all(scratch).await?;
    // emptied for the next use
    for r in &back {
        let (sql, args) = K::delete(r);
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for a in args {
            q = match a {
                KeyArg::Int(v) => q.bind(v),
                KeyArg::Text(v) => q.bind(v),
            };
        }
        q.execute(&scratch.pool())
            .await
            .map_err(crate::db::DbError::from)?;
    }
    Ok(back.into_iter().map(|r| (K::key(&r), r)).collect())
}

pub(crate) async fn import_table<K: Kind>(
    realm: &Realm,
    db: &Db,
    scratch: &Db,
    ctx: &Ctx,
    opts: &crate::ImportOptions,
) -> Result<TableReport> {
    let t = read_realm::<K>(realm, ctx)?;
    let mut report = TableReport {
        table: K::NAME,
        ..TableReport::default()
    };
    if t.files.is_empty() {
        return Err(Error::NoFile(K::TABLE));
    }
    let theirs = K::all(db).await?;
    let mut kept = raw::load(db, K::NAME).await?;

    // each row's base: the bytes it was exported from, else the realm's record, else blank
    let mut bases: Vec<Option<Vec<u8>>> = Vec::with_capacity(theirs.len());
    let mut known = Vec::new();
    for row in &theirs {
        let key = K::key(row);
        let base = kept
            .remove(&key)
            .or_else(|| t.by_key.get(&key).map(|h| h.bytes.clone()));
        if let Some(b) = &base {
            let r = K::decode(b, ctx.set)
                .map_err(|e| Error::Record(format!("{} {key}: {e}", K::NAME)))?;
            let old =
                K::to_row(&r, ctx).map_err(|e| Error::Record(format!("{} {key}: {e}", K::NAME)))?;
            // a base whose key no longer matches (a hand-edited table) is not used
            if K::key(&old) == key {
                known.push(old);
            } else {
                bases.push(None);
                continue;
            }
        }
        bases.push(base);
    }
    let olds = as_stored::<K>(scratch, &known).await?;

    let blank = K::blank(ctx.set);
    let blank_row = K::to_row(
        &K::decode(&blank, ctx.set).map_err(|e| Error::Record(e.to_string()))?,
        ctx,
    )
    .map_err(|e| Error::Record(e.to_string()))?;

    // the realm's files, opened for writing on the first change
    let mut open: Vec<Option<WbtrvDb>> = t.files.iter().map(|_| None).collect();
    let file_of = |i: usize, open: &mut Vec<Option<WbtrvDb>>| -> Result<()> {
        if open[i].is_none() {
            let mut db = WbtrvDb::open(&t.files[i])?;
            db.begin()?;
            open[i] = Some(db);
        }
        Ok(())
    };
    let mut seen = std::collections::HashSet::new();
    let mut inserts = Vec::new();
    for (row, base) in theirs.iter().zip(bases) {
        let key = K::key(row);
        seen.insert(key.clone());
        let bytes = match &base {
            Some(b) => K::rebuild(b, row, &olds[&key], ctx),
            None => K::rebuild(&blank, row, &blank_row, ctx),
        }
        .map_err(|e| Error::Record(format!("{} {key}: {e}", K::NAME)))?;
        match t.by_key.get(&key) {
            Some(h) if h.bytes == bytes => report.unchanged += 1,
            Some(h) => {
                report.updated += 1;
                if !opts.dry_run {
                    file_of(h.file, &mut open)?;
                    open[h.file]
                        .as_mut()
                        .expect("opened")
                        .update(h.id, &bytes)?;
                }
            }
            None => {
                report.inserted += 1;
                inserts.push((key, bytes));
            }
        }
    }
    if opts.delete {
        for (key, h) in &t.by_key {
            if !seen.contains(key) {
                report.deleted += 1;
                if !opts.dry_run {
                    file_of(h.file, &mut open)?;
                    open[h.file].as_mut().expect("opened").delete(h.id)?;
                }
            }
        }
    }
    if !opts.dry_run {
        for (key, bytes) in inserts {
            let i = file_for_new(&key, &t);
            file_of(i, &mut open)?;
            open[i].as_mut().expect("opened").insert(&bytes)?;
        }
        for db in open.into_iter().flatten() {
            db.commit()?;
        }
    }
    Ok(report)
}

/// The realm file a new record goes into: for rooms, the file that holds others of its map;
/// otherwise the table's first file.
fn file_for_new<K: Kind>(key: &str, t: &RealmTable<K>) -> usize {
    if K::TABLE == Table::Rooms
        && let Some((map, _)) = key.split_once('/')
    {
        let prefix = format!("{map}/");
        if let Some(h) = t
            .by_key
            .iter()
            .find(|(k, _)| k.starts_with(&prefix))
            .map(|(_, h)| h)
        {
            return h.file;
        }
    }
    0
}
