//! silvermere itself on what Daydream writes: it serves the export, and while it runs its
//! writer lock keeps Daydream out. Runs when `SILVERMERE_BIN` names a silvermere binary.

use daydream_silvermere::{Error, ExportOptions, ImportOptions, export, import};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{backends, database, random_realm, serve, silvermere_bin};

/// A random world has no room 2140 (silvermere's default start) and so on.
const ROOMS: [&str; 4] = [
    "game.start_room=1",
    "game.reinc_room=1",
    "game.evil_reinc_room=1",
    "game.jail_room=1",
];

#[tokio::test]
async fn silvermere_serves_an_export_and_locks_daydream_out() {
    let Some(bin) = silvermere_bin() else {
        eprintln!("SILVERMERE_BIN is not set: skipped");
        return;
    };
    for pg in backends() {
        let dir = tempfile::tempdir().unwrap();
        random_realm(dir.path(), LayoutSet::Dos, 30, 11);
        let db = database(pg).await;
        let realm = Realm::open(dir.path()).unwrap();
        export(&realm, &db.url, &ExportOptions::default())
            .await
            .unwrap();
        {
            let _server = serve(&bin, &db.url, &ROOMS);
            let e = export(&realm, &db.url, &ExportOptions::default()).await;
            assert!(matches!(e, Err(Error::Running { .. })), "{e:?}");
            let e = import(&realm, &db.url, &ImportOptions::default()).await;
            assert!(matches!(e, Err(Error::Running { .. })), "{e:?}");
        }
        // stopped: Daydream gets in again
        import(&realm, &db.url, &ImportOptions::default())
            .await
            .unwrap();
    }
}
