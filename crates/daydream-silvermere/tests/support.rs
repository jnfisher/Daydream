//! Realms of random records, and the databases the tests write into.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use daydream_store::{Metadata, Realm, WbtrvDb};
use daydream_types::records::LayoutSet;
use daydream_wcc::decode::{
    ActionRecord, MonsterRecord, Record, RoomRecord, Text, TextblkRecord, decode_textblk_as,
};
use daydream_wcc::tables::{FILES, Table, WORLDGROUP_FILES};

/// The world tables the adapter moves.
pub const WORLD: [Table; 10] = [
    Table::Rooms,
    Table::Items,
    Table::Monsters,
    Table::Spells,
    Table::Classes,
    Table::Races,
    Table::Shops,
    Table::Messages,
    Table::TextBlocks,
    Table::Actions,
];

pub fn stem(table: Table, set: LayoutSet) -> &'static str {
    if table == Table::Rooms {
        return match set {
            LayoutSet::Dos => "WCCMP001",
            LayoutSet::Worldgroup => "WCCMP002",
        };
    }
    let files = match set {
        LayoutSet::Dos => FILES,
        LayoutSet::Worldgroup => WORLDGROUP_FILES,
    };
    files.iter().find(|f| f.table == table).unwrap().stem
}

/// The text's offset in a text block: the length of its fixed part.
pub fn text_offset(set: LayoutSet) -> usize {
    TextblkRecord::layout(set).field("text").unwrap().offset
}

/// An empty `.db` for every world table (no Btrieve keys: the adapter matches by its own).
pub fn empty_realm(dir: &Path, set: LayoutSet) {
    for table in WORLD {
        let variable = table.is_variable_length();
        let len = if variable {
            text_offset(set)
        } else {
            daydream_nmr_free_len(table, set)
        };
        let meta = Metadata {
            record_length: len,
            physical_record_length: len,
            page_length: 4096,
            variable_length_records: variable,
            version: 4,
        };
        WbtrvDb::create(
            &dir.join(format!("{}.db", stem(table, set))),
            meta,
            Vec::new(),
        )
        .unwrap();
    }
}

/// A table's record length in `set`.
fn daydream_nmr_free_len(table: Table, set: LayoutSet) -> usize {
    use daydream_wcc::decode::*;
    match table {
        Table::Rooms => RoomRecord::layout(set).len,
        Table::Items => ItemRecord::layout(set).len,
        Table::Monsters => MonsterRecord::layout(set).len,
        Table::Spells => SpellRecord::layout(set).len,
        Table::Classes => ClassRecord::layout(set).len,
        Table::Races => RaceRecord::layout(set).len,
        Table::Shops => ShopRecord::layout(set).len,
        Table::Messages => MudmsgRecord::layout(set).len,
        Table::TextBlocks => TextblkRecord::layout(set).len,
        Table::Actions => ActionRecord::layout(set).len,
        t => panic!("{t:?} is not a world table"),
    }
}

/// A small, deterministic generator.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub fn fill(&mut self, b: &mut [u8]) {
        for x in b {
            *x = self.next() as u8;
        }
    }
}

/// Writes `n` random records into every world table of the realm in `dir` (made empty
/// first): every byte random, except each record's key (distinct) and what silvermere's
/// schema requires of a value (a monster's kill date is a date).
pub fn random_realm(dir: &Path, set: LayoutSet, n: usize, seed: u64) {
    empty_realm(dir, set);
    let mut rng = Lcg(seed);
    for table in WORLD {
        let mut db = WbtrvDb::open(&dir.join(format!("{}.db", stem(table, set)))).unwrap();
        for i in 0..n {
            let len = daydream_nmr_free_len(table, set);
            let mut raw = vec![0u8; len];
            rng.fill(&mut raw);
            let number = (i as i32) + 1;
            let raw = match table {
                Table::Rooms => {
                    let mut r = RoomRecord::decode_as(&raw, set).unwrap();
                    r.mapnum = 1 + (i % 2) as u32;
                    r.roomnum = number as u32;
                    r.encode_as(set)
                }
                Table::Monsters => {
                    let mut r = MonsterRecord::decode_as(&raw, set).unwrap();
                    r.number = number;
                    // a valid date, or 0 ("never") for every other one
                    r.killdate = if i % 2 == 0 {
                        0
                    } else {
                        (40 << 9) | (6 << 5) | 15
                    };
                    r.killtime = if i % 2 == 0 {
                        r.killtime
                    } else {
                        (13 << 11) | (7 << 5) | 9
                    };
                    r.encode_as(set)
                }
                Table::Actions => {
                    let mut r = ActionRecord::decode_as(&raw, set).unwrap();
                    r.name = Text::new(format!("act{i}"));
                    r.encode_as(set)
                }
                Table::TextBlocks => {
                    let mut r = decode_textblk_as(&raw, set).unwrap();
                    r.number = number / 2 + 1;
                    r.part = (i % 2) as i16;
                    let mut out = r.encode_as(set);
                    // a stored text of random length, then the terminator
                    let used = (rng.next() as usize) % 300;
                    let at = text_offset(set);
                    out.truncate(at + used + 1);
                    for b in &mut out[at..at + used] {
                        // no stored 0x00 or 0x20 inside the text: both end it
                        if *b == 0 || *b == 0x20 {
                            *b = 0x41;
                        }
                    }
                    out[at + used] = 0;
                    out
                }
                _ => {
                    // every other world table's key 0 is its number, first, as an i32 or
                    // an i16/u16
                    let width = match table {
                        Table::Spells | Table::Classes | Table::Races => 2,
                        _ => 4,
                    };
                    raw[..width].copy_from_slice(&number.to_le_bytes()[..width]);
                    raw
                }
            };
            db.insert(&raw).unwrap();
        }
    }
}

/// One file's records: id and bytes.
pub type Records = Vec<(i64, Vec<u8>)>;

/// Every record of every `.db` in a realm, by stem.
pub fn records(dir: &Path) -> Vec<(String, Records)> {
    Realm::open(dir)
        .unwrap()
        .files()
        .iter()
        .map(|f| {
            let db = WbtrvDb::open_read_only(&f.path).unwrap();
            (f.stem.clone(), db.records().unwrap())
        })
        .collect()
}

/// The records only, each file's sorted (ids differ after an insert).
pub fn contents(dir: &Path) -> Vec<(String, Vec<Vec<u8>>)> {
    records(dir)
        .into_iter()
        .map(|(s, r)| {
            let mut v: Vec<Vec<u8>> = r.into_iter().map(|(_, b)| b).collect();
            v.sort();
            (s, v)
        })
        .collect()
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// A database to test on: a SQLite file in a directory of its own, or with `postgres`, a
/// fresh schema on the PostgreSQL that `DAYDREAM_POSTGRES_URL` names.
pub struct TestDb {
    pub url: String,
    _dir: Option<tempfile::TempDir>,
}

pub async fn database(postgres: bool) -> TestDb {
    if !postgres {
        let dir = tempfile::tempdir().unwrap();
        return TestDb {
            url: format!("sqlite:{}", dir.path().join("silvermere.db").display()),
            _dir: Some(dir),
        };
    }
    let base = std::env::var("DAYDREAM_POSTGRES_URL").expect("DAYDREAM_POSTGRES_URL");
    let schema = format!(
        "dd_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    );
    sqlx::any::install_default_drivers();
    let pool = sqlx::AnyPool::connect(&base).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let sep = if base.contains('?') { '&' } else { '?' };
    TestDb {
        url: format!("{base}{sep}options=-c%20search_path%3D{schema}"),
        _dir: None,
    }
}

/// The backends to run on: SQLite, and PostgreSQL when `DAYDREAM_POSTGRES_URL` is set.
pub fn backends() -> Vec<bool> {
    let mut v = vec![false];
    if std::env::var("DAYDREAM_POSTGRES_URL").is_ok_and(|u| !u.is_empty()) {
        v.push(true);
    }
    v
}

/// A silvermere server: `SILVERMERE_BIN`'s `serve` on a database, until dropped.
pub struct Serving(std::process::Child);

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The silvermere binary to serve with, when `SILVERMERE_BIN` names one.
pub fn silvermere_bin() -> Option<std::path::PathBuf> {
    std::env::var_os("SILVERMERE_BIN")
        .filter(|b| !b.is_empty())
        .map(Into::into)
}

/// Starts `bin serve` on `url` and waits until it listens for telnet, or panics with its
/// output when it exits first or takes over a minute. `set` overrides config keys.
pub fn serve(bin: &Path, url: &str, set: &[&str]) -> (Serving, tempfile::TempDir) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let data = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(bin);
    cmd.arg("serve")
        .arg("--data-dir")
        .arg(data.path())
        .args(["--database-url", url])
        .args([
            "--telnet-bind",
            "127.0.0.1:0",
            "--admin-bind",
            "127.0.0.1:0",
        ])
        .args(["--set", "server.web_bind=\"\"", "--log-format", "pretty"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for s in set {
        cmd.args(["--set", s]);
    }
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("{}: {e}", bin.display()));
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    for out in [
        Box::new(child.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
        Box::new(child.stderr.take().unwrap()),
    ] {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);
    let serving = Serving(child);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut seen = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(left) {
            Ok(line) if line.contains("telnet listening") => return (serving, data),
            Ok(line) => seen.push(line),
            Err(e) => panic!(
                "silvermere did not start ({e:?}):\n{}",
                seen[seen.len().saturating_sub(40)..].join("\n")
            ),
        }
    }
}
