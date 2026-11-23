//! The stock 1.11p world through silvermere's database: out and back into an empty realm
//! byte for byte, back into its own realm with nothing changed, and (with `SILVERMERE_BIN`)
//! served by silvermere itself.

use std::path::PathBuf;
use std::sync::OnceLock;

use daydream_silvermere::{ExportOptions, ImportOptions, export, import};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{
    WORLD, backends, contents, database, empty_realm, records, serve, silvermere_bin, stem,
};

/// The stock world as a realm, converted once per run.
fn stock_realm() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let vir = daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
            panic!(
                "the tests need the stock distribution and could not get it: {e}\n\
                 (cache directory {}; set {} to use another)",
                daydream_wcc::stock_files::cache_dir().display(),
                daydream_wcc::stock_files::CACHE_ENV
            )
        });
        let dir =
            std::env::temp_dir().join(format!("daydream-silvermere-stock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        daydream_store::realm::convert_dir(&vir, &dir).unwrap();
        dir
    })
    .clone()
}

/// A copy of the stock realm's world files.
fn copy_world(to: &std::path::Path) {
    for t in WORLD {
        let name = format!("{}.db", stem(t, LayoutSet::Dos));
        std::fs::copy(stock_realm().join(&name), to.join(&name)).unwrap();
    }
}

fn world(v: Vec<(String, Vec<Vec<u8>>)>) -> Vec<(String, Vec<Vec<u8>>)> {
    let stems: Vec<&str> = WORLD.iter().map(|&t| stem(t, LayoutSet::Dos)).collect();
    v.into_iter()
        .filter(|(s, _)| stems.contains(&s.as_str()))
        .collect()
}

#[tokio::test]
async fn the_stock_world_round_trips_through_silvermere() {
    for pg in backends() {
        let src = tempfile::tempdir().unwrap();
        copy_world(src.path());
        let before = records(src.path());
        let db = database(pg).await;
        let realm = Realm::open(src.path()).unwrap();
        let r = export(&realm, &db.url, &ExportOptions::default())
            .await
            .unwrap();
        for t in &r {
            assert!(t.inserted > 0, "{}: nothing written", t.table);
        }

        // back into its own realm: nothing changes
        let r = import(&realm, &db.url, &ImportOptions::default())
            .await
            .unwrap();
        for t in &r {
            assert_eq!((t.inserted, t.updated, t.deleted), (0, 0, 0), "{t:?}");
        }
        assert_eq!(records(src.path()), before);

        // into an empty realm: every record, byte for byte
        let dest = tempfile::tempdir().unwrap();
        empty_realm(dest.path(), LayoutSet::Dos);
        let empty = Realm::open(dest.path()).unwrap();
        import(&empty, &db.url, &ImportOptions::default())
            .await
            .unwrap();
        assert_eq!(world(contents(dest.path())), world(contents(src.path())));

        // and silvermere serves it
        if let Some(bin) = silvermere_bin() {
            drop(serve(&bin, &db.url, &[]));
        }
    }
}
