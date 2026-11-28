//! silvermere itself on what Daydream writes: it serves the export, and while it runs its
//! writer lock keeps Daydream out. Runs when `SILVERMERE_BIN` names a silvermere binary.

use daydream_silvermere::{Error, ExportOptions, ImportOptions, export, export_held, import, info};
use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::support::{
    admin_call, backends, database, empty_realm, random_realm, serve, serve_admin, silvermere_bin,
};

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

/// A publish into silvermere while it runs (silvermere 1.0.5 and later): it holds its own
/// writes, Daydream writes the world, and silvermere reloads it, which ends the hold.
#[tokio::test]
async fn a_world_written_under_a_hold_is_taken_in_while_silvermere_runs() {
    let Some(bin) = silvermere_bin() else {
        eprintln!("SILVERMERE_BIN is not set: skipped");
        return;
    };
    const TOKEN: &str = "daydream-test-token";
    for pg in backends() {
        let first = tempfile::tempdir().unwrap();
        random_realm(first.path(), LayoutSet::Dos, 30, 11);
        // a second world, smaller: what the reload must show
        let second = tempfile::tempdir().unwrap();
        random_realm(second.path(), LayoutSet::Dos, 25, 12);
        let db = database(pg).await;
        export(
            &Realm::open(first.path()).unwrap(),
            &db.url,
            &ExportOptions::default(),
        )
        .await
        .unwrap();
        // nobody runs on it: there is no hold to write under
        let e = export_held(
            &Realm::open(second.path()).unwrap(),
            &db.url,
            &ExportOptions::default(),
        )
        .await;
        assert!(matches!(e, Err(Error::NotRunning(_))), "{e:?}");
        {
            let (_server, _data, admin) = serve_admin(&bin, &db.url, &ROOMS, Some(TOKEN));
            let admin = admin.expect("silvermere says where its admin API listens");
            let call = |m: &str, p: &str, b: Option<&str>| admin_call(&admin, TOKEN, m, p, b);
            let (status, held) = call("POST", "/v1/world-hold", Some(r#"{"secs":60}"#));
            if status == 404 {
                eprintln!("this silvermere has no /v1/world-hold (before 1.0.5): skipped");
                return;
            }
            assert_eq!(status, 200, "{held}");
            let id = held["hold"].as_str().unwrap().to_string();
            assert_eq!(call("POST", "/v1/world-hold", None).0, 409);
            let r = export_held(
                &Realm::open(second.path()).unwrap(),
                &db.url,
                &ExportOptions::default(),
            )
            .await
            .unwrap();
            assert!(r.iter().all(|t| t.deleted == 5), "{r:?}");
            let (status, reloaded) = call("POST", &format!("/v1/world-hold/{id}/reload"), None);
            assert_eq!(status, 200, "{reloaded}");
            assert_eq!(reloaded["reloaded"]["rooms"], 25, "{reloaded}");
            assert_eq!(reloaded["reloaded"]["items"], 25, "{reloaded}");
            let (_, now) = call("GET", "/v1/world-hold", None);
            assert!(now["hold"].is_null(), "{now}");
            // still running, so still locked out of a plain export
            let e = export(
                &Realm::open(second.path()).unwrap(),
                &db.url,
                &ExportOptions::default(),
            )
            .await;
            assert!(matches!(e, Err(Error::Running { .. })), "{e:?}");
        }
        // stopped: the second world is what silvermere has
        let i = info(&db.url).await.unwrap();
        assert!(i.tables.iter().all(|(_, n)| *n == 25), "{:?}", i.tables);
        let back = tempfile::tempdir().unwrap();
        empty_realm(back.path(), LayoutSet::Dos);
        let r = import(
            &Realm::open(back.path()).unwrap(),
            &db.url,
            &ImportOptions::default(),
        )
        .await
        .unwrap();
        assert!(r.iter().all(|t| t.inserted == 25), "{r:?}");
    }
}
