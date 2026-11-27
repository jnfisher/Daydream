//! A realm through silvermere's database and back, on random records: every byte of every
//! world table, on SQLite and (with `DAYDREAM_POSTGRES_URL`) PostgreSQL.

use daydream_silvermere::{
    Error, ExportOptions, ImportOptions, State, TableReport, export, import, info,
};
use daydream_store::{Realm, WbtrvDb};
use daydream_types::records::LayoutSet;

use crate::support::{backends, contents, database, empty_realm, random_realm, records, stem};

fn quiet(reports: &[TableReport]) -> String {
    reports
        .iter()
        .map(|r| {
            format!(
                "{} +{} ~{} ={} -{}",
                r.table, r.inserted, r.updated, r.unchanged, r.deleted
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

async fn out_and_back(set: LayoutSet, postgres: bool) {
    let src = tempfile::tempdir().unwrap();
    random_realm(src.path(), set, 30, 7);
    let before = records(src.path());
    let url_db = database(postgres).await;
    let url = url_db.url.clone();
    let realm = Realm::open(src.path()).unwrap();
    let r = export(&realm, &url, &ExportOptions::default())
        .await
        .unwrap();
    assert_eq!(r.len(), 10, "{}", quiet(&r));
    for t in &r {
        assert_eq!((t.inserted, t.deleted), (30, 0), "{}", quiet(&r));
    }

    // back into the realm it came from: nothing changes
    let back = import(&realm, &url, &ImportOptions::default())
        .await
        .unwrap();
    for t in &back {
        assert_eq!(t.unchanged, 30, "{}", quiet(&back));
    }
    assert_eq!(records(src.path()), before);

    // into an empty realm: every record, byte for byte, from the bytes kept beside the rows
    let dest = tempfile::tempdir().unwrap();
    empty_realm(dest.path(), set);
    let empty = Realm::open(dest.path()).unwrap();
    let r = import(&empty, &url, &ImportOptions::default())
        .await
        .unwrap();
    for t in &r {
        assert_eq!(t.inserted, 30, "{}", quiet(&r));
    }
    assert_eq!(contents(dest.path()), contents(src.path()));
}

#[tokio::test]
async fn a_realm_comes_back_byte_for_byte() {
    for pg in backends() {
        out_and_back(LayoutSet::Dos, pg).await;
    }
}

#[tokio::test]
async fn a_worldgroup_realm_comes_back_byte_for_byte() {
    for pg in backends() {
        out_and_back(LayoutSet::Worldgroup, pg).await;
    }
}

/// Without the kept bytes, what silvermere stores still comes back: the realm rebuilt from
/// the rows alone writes the same rows.
#[tokio::test]
async fn without_the_bytes_every_stored_value_comes_back() {
    for pg in backends() {
        let src = tempfile::tempdir().unwrap();
        random_realm(src.path(), LayoutSet::Dos, 20, 11);
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        let opts = ExportOptions {
            raw: false,
            ..ExportOptions::default()
        };
        export(&Realm::open(src.path()).unwrap(), &url, &opts)
            .await
            .unwrap();
        let info = info(&url).await.unwrap();
        assert!(!info.raw);
        assert!(info.tables.iter().all(|(_, n)| *n == 20), "{info:?}");

        let dest = tempfile::tempdir().unwrap();
        empty_realm(dest.path(), LayoutSet::Dos);
        let rebuilt = Realm::open(dest.path()).unwrap();
        import(&rebuilt, &url, &ImportOptions::default())
            .await
            .unwrap();
        // a second database, from the rebuilt realm: the same rows
        let url2_db = database(pg).await;
        let url2 = url2_db.url.clone();
        export(&rebuilt, &url2, &opts).await.unwrap();
        // the rebuilt realm, exported and imported once more, is stable
        let again = import(&rebuilt, &url2, &ImportOptions::default())
            .await
            .unwrap();
        for t in &again {
            assert_eq!(t.unchanged, 20, "{}", quiet(&again));
        }
        // and the first realm, every byte there, finds the second database's rows to be
        // its own: every value silvermere stores came back
        let r = import(
            &Realm::open(src.path()).unwrap(),
            &url2,
            &ImportOptions::default(),
        )
        .await
        .unwrap();
        for t in &r {
            assert_eq!(t.unchanged, 20, "{}", quiet(&r));
        }
    }
}

/// A change made in silvermere changes only that member's bytes.
#[tokio::test]
async fn an_edit_in_silvermere_changes_only_its_bytes() {
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 10, 3);
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        let realm = Realm::open(dir.path()).unwrap();
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        let before = records(dir.path());

        sqlx::any::install_default_drivers();
        let pool = sqlx::AnyPool::connect(&url).await.unwrap();
        sqlx::query("UPDATE items SET name = 'Daydream sword', cost = 1234 WHERE number = 4")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM spells WHERE number = 2")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let r = import(&realm, &url, &ImportOptions::default())
            .await
            .unwrap();
        let items = r.iter().find(|t| t.table == "items").unwrap();
        assert_eq!((items.updated, items.unchanged), (1, 9));
        let spells = r.iter().find(|t| t.table == "spells").unwrap();
        assert_eq!((spells.deleted, spells.unchanged), (1, 9));

        let after = records(dir.path());
        let file = |recs: &[(String, crate::support::Records)], s: &str| {
            recs.iter().find(|(stem, _)| stem == s).unwrap().1.clone()
        };
        let (old, new) = (file(&before, "WCCITEMS"), file(&after, "WCCITEMS"));
        let layout = daydream_types::records::layout::ITEM;
        let name = layout.field("name").unwrap();
        let cost = layout.field("cost").unwrap();
        for ((_, a), (_, b)) in old.iter().zip(&new) {
            if a == b {
                continue;
            }
            assert_eq!(i32::from_le_bytes(a[..4].try_into().unwrap()), 4);
            for (at, (x, y)) in a.iter().zip(b).enumerate() {
                if x != y {
                    let in_name = (name.offset..name.offset + name.size).contains(&at);
                    let in_cost = (cost.offset..cost.offset + cost.size).contains(&at);
                    assert!(in_name || in_cost, "byte {at:#x} changed");
                }
            }
            assert_eq!(&b[name.offset..name.offset + 15], b"Daydream sword\0");
            assert_eq!(&b[cost.offset..cost.offset + 2], &1234u16.to_le_bytes());
        }
        assert_eq!(file(&after, "WCCSPELS").len(), 9);
    }
}

/// An export deletes the rows the realm no longer has; their state and bytes go with them.
#[tokio::test]
async fn deleted_records_leave_silvermere_too() {
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 6, 5);
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        let realm = Realm::open(dir.path()).unwrap();
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        let path = dir.path().join(format!(
            "{}.db",
            stem(daydream_wcc::tables::Table::Rooms, LayoutSet::Dos)
        ));
        let mut rooms = WbtrvDb::open(&path).unwrap();
        let first = rooms.records().unwrap()[0].0;
        rooms.delete(first).unwrap();
        drop(rooms);

        let dry = ExportOptions {
            dry_run: true,
            ..ExportOptions::default()
        };
        let r = export(&realm, &url, &dry).await.unwrap();
        assert_eq!(r[0].deleted, 1);
        assert_eq!(info(&url).await.unwrap().tables[0], ("rooms", 6));
        let r = export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        assert_eq!((r[0].table, r[0].deleted, r[0].updated), ("rooms", 1, 5));
        assert_eq!(info(&url).await.unwrap().tables[0], ("rooms", 5));
        let back = import(&realm, &url, &ImportOptions::default())
            .await
            .unwrap();
        assert!(
            back.iter().all(|t| t.inserted + t.updated + t.deleted == 0),
            "{}",
            quiet(&back)
        );
    }
}

/// Live state silvermere has is kept unless the export resets it.
#[tokio::test]
async fn live_state_is_kept_unless_reset() {
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 4, 9);
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        let realm = Realm::open(dir.path()).unwrap();
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        sqlx::any::install_default_drivers();
        let pool = sqlx::AnyPool::connect(&url).await.unwrap();
        sqlx::query("UPDATE room_state SET copper = 777")
            .execute(&pool)
            .await
            .unwrap();
        let copper = || async {
            let n: i64 = sqlx::query_scalar("SELECT MIN(copper) FROM room_state")
                .fetch_one(&pool)
                .await
                .unwrap();
            n
        };
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        assert_eq!(copper().await, 777);
        let reset = ExportOptions {
            state: State::Reset,
            ..ExportOptions::default()
        };
        export(&realm, &url, &reset).await.unwrap();
        assert_ne!(copper().await, 777);
        pool.close().await;
    }
}

/// A running silvermere holds the writer guard: nothing is read or written.
#[tokio::test]
async fn a_running_silvermere_is_left_alone() {
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 3, 1);
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        let realm = Realm::open(dir.path()).unwrap();
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        let held = match daydream_silvermere::db::WriterGuard::try_acquire(&url)
            .await
            .unwrap()
        {
            daydream_silvermere::db::TryAcquire::Taken(g) => g,
            daydream_silvermere::db::TryAcquire::Held(h) => panic!("held: {h}"),
        };
        let e = export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Running { .. }), "{e}");
        let e = import(&realm, &url, &ImportOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(e, Error::Running { .. }), "{e}");
        assert!(info(&url).await.unwrap().running.is_some());
        held.release().await;
        import(&realm, &url, &ImportOptions::default())
            .await
            .unwrap();
    }
}

/// A database a newer silvermere migrated, or one that is not silvermere's, is refused.
#[tokio::test]
async fn unknown_schemas_are_refused() {
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 2, 1);
        let realm = Realm::open(dir.path()).unwrap();
        let url_db = database(pg).await;
        let url = url_db.url.clone();
        // nothing there to import
        let e = import(&realm, &url, &ImportOptions::default())
            .await
            .unwrap_err();
        assert!(
            matches!(e, Error::Schema { .. } | Error::NoDatabase(_)),
            "{e}"
        );
        export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap();
        sqlx::any::install_default_drivers();
        let pool = sqlx::AnyPool::connect(&url).await.unwrap();
        let checksum = if pg { "'\\x00'::bytea" } else { "x'00'" };
        let success = if pg { "TRUE" } else { "1" };
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, \
             execution_time) VALUES (99, 'future', {success}, {checksum}, 0)"
        )))
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
        let e = export(&realm, &url, &ExportOptions::default())
            .await
            .unwrap_err();
        assert!(e.to_string().contains("newer silvermere"), "{e}");
    }
}

#[test]
fn paths_become_sqlite_urls() {
    use daydream_silvermere::database_url;
    assert_eq!(
        database_url("world/silvermere.db"),
        "sqlite:world/silvermere.db"
    );
    assert_eq!(database_url("sqlite://x.db"), "sqlite://x.db");
    assert_eq!(database_url(" postgres://u@h/d "), "postgres://u@h/d");
}

/// A preview into a SQLite file not made yet: every record would be new, and no file is made.
#[tokio::test]
async fn a_preview_into_a_new_database_makes_no_file() {
    let dir = tempfile::tempdir().unwrap();
    random_realm(dir.path(), LayoutSet::Dos, 4, 3);
    let realm = Realm::open(dir.path()).unwrap();
    let file = dir.path().join("silvermere.db");
    let url = daydream_silvermere::database_url(&file.display().to_string());
    let dry = ExportOptions {
        dry_run: true,
        ..ExportOptions::default()
    };
    let r = export(&realm, &url, &dry).await.unwrap();
    assert_eq!(r.len(), 10, "{}", quiet(&r));
    for t in &r {
        assert_eq!(
            (t.inserted, t.updated, t.deleted),
            (4, 0, 0),
            "{}",
            quiet(&r)
        );
    }
    assert!(!file.exists());
}
