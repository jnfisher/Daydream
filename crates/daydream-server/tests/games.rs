//! The game picker: games found under a search folder, opened into working copies, written
//! back into, and what a token holder may not name.

use std::path::{Path, PathBuf};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use daydream_server::Options;
use daydream_server::games::Config;
use daydream_store::WbtrvDb;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "0123456789abcdef-token";

/// A machine with one Btrieve game (`bbs/WGSERV`) and one MBBSEmu game in `.db` files
/// (`emu`, its `.DAT` files converted as wbtrv32 converts them), under `root`; and the
/// workspace beside them.
struct Machine {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    workspace: PathBuf,
}

/// wbtrv32's test file, under MajorMUD's names: a stand-in for the world's files.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../daydream-wcc/tests/fixtures/btrieve/MBBSEMU.DAT")
}

fn machine() -> Machine {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("games");
    let wg = root.join("bbs").join("WGSERV");
    std::fs::create_dir_all(&wg).unwrap();
    for f in ["WCCITEMS.DAT", "WCCKNMSR.DAT"] {
        std::fs::copy(fixture(), wg.join(f)).unwrap();
    }
    // the BBS's own files are not the game's
    std::fs::copy(fixture(), wg.join("BBSUSR.DAT")).unwrap();
    let emu = root.join("emu");
    std::fs::create_dir_all(&emu).unwrap();
    for f in ["WCCITEMS", "WCCKNMSR"] {
        std::fs::copy(fixture(), emu.join(format!("{f}.DAT"))).unwrap();
        WbtrvDb::convert(&fixture(), &emu.join(format!("{f}.DB"))).unwrap();
    }
    // MBBSEmu's own settings, by which it is known; and a file of the BBS's, not the game's
    std::fs::write(emu.join("appsettings.json"), "{}").unwrap();
    WbtrvDb::convert(&fixture(), &emu.join("BBSUSR.DB")).unwrap();
    let workspace = tmp.path().join("workspace");
    Machine {
        root,
        workspace,
        _tmp: tmp,
    }
}

impl Machine {
    fn router(&self, token: Option<&str>, allow_any_source: bool) -> axum::Router {
        daydream_server::router_with(
            None,
            Options {
                token: token.map(str::to_string),
                games: Config {
                    workspace: Some(self.workspace.clone()),
                    search: vec![self.root.clone()],
                    default_search: false,
                    allow_any_source,
                    ..Config::default()
                },
                ..Options::default()
            },
        )
    }

    fn wg(&self) -> PathBuf {
        std::fs::canonicalize(self.root.join("bbs").join("WGSERV")).unwrap()
    }
}

async fn call(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(
            "host",
            if token.is_some() {
                "daydream.example.com"
            } else {
                "127.0.0.1:7340"
            },
        )
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = req
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let res = router.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn game<'a>(list: &'a Value, kind: &str) -> &'a Value {
    list["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} game in {list}"))
}

#[tokio::test]
async fn games_are_found_opened_and_written_back() {
    let m = machine();
    let app = m.router(None, false);
    // nothing open yet: the editors ask for a game
    let (status, body) = call(&app, "GET", "/api/realm", None, None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    let (status, list) = call(&app, "GET", "/api/games", None, None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["current"], Value::Null);
    assert_eq!(list["custom"], true);
    let wg = game(&list, "wcc");
    assert_eq!(wg["label"], "WGSERV (MajorMUD)");
    assert_eq!(wg["location"], m.wg().display().to_string());
    assert_eq!(wg["copy"], Value::Null);
    let emu = game(&list, "db");
    assert_eq!(emu["live"], true);

    let (status, st) = call(
        &app,
        "GET",
        &format!("/api/games/status?id={}", wg["id"].as_str().unwrap()),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{st}");
    assert_eq!(st["running"], false);

    // opening the Btrieve game makes its working copy: the WCC files only
    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": wg["id"]})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let copy = PathBuf::from(body["current"]["realm_dir"].as_str().unwrap());
    assert!(copy.starts_with(&m.workspace), "{}", copy.display());
    let (_, realm) = call(&app, "GET", "/api/realm", None, None).await;
    let stems: Vec<&str> = realm["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["stem"].as_str().unwrap())
        .collect();
    assert_eq!(stems, ["WCCITEMS", "WCCKNMSR"]);
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    assert_eq!(game(&list, "wcc")["current"], true);
    assert!(game(&list, "wcc")["copy"]["pulled"].is_string());

    // an edit, written back with no folder named: the game's own
    WbtrvDb::open(&copy.join("WCCITEMS.db"))
        .unwrap()
        .delete(2)
        .unwrap();
    let before = std::fs::read(m.wg().join("WCCITEMS.DAT")).unwrap();
    let (status, body) = call(
        &app,
        "POST",
        "/api/tools/write-dat",
        Some(json!({"dry_run": false})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let items = body["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["stem"] == "WCCITEMS")
        .unwrap();
    assert_eq!(items["action"], "rebuilt");
    assert_ne!(std::fs::read(m.wg().join("WCCITEMS.DAT")).unwrap(), before);

    // opening it again keeps the copy; refreshing reads the game again, keeping the old one
    let (_, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": wg["id"]})),
        None,
    )
    .await;
    assert_eq!(body["current"]["realm_dir"], copy.display().to_string());
    std::fs::write(copy.join("marker"), b"x").unwrap();
    let (status, _) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": wg["id"], "refresh": true})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!copy.join("marker").exists());
    assert!(copy.with_extension("previous").join("marker").exists());

    // the MBBSEmu game is edited in a copy of its world's .db files, and published, not
    // written back as WCC files
    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": emu["id"]})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["current"]["live"], false);
    assert_eq!(body["current"]["copy"], true);
    let emu_copy = PathBuf::from(body["current"]["realm_dir"].as_str().unwrap());
    assert!(emu_copy.starts_with(&m.workspace), "{}", emu_copy.display());
    let (_, realm) = call(&app, "GET", "/api/realm", None, None).await;
    let stems: Vec<&str> = realm["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["stem"].as_str().unwrap())
        .collect();
    assert_eq!(stems, ["WCCITEMS", "WCCKNMSR"]);
    let (status, body) = call(
        &app,
        "POST",
        "/api/tools/write-dat",
        Some(json!({"dry_run": true})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("Publish"),
        "{body}"
    );
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    assert!(game(&list, "db")["copy"]["pulled"].is_string(), "{list}");

    // a later start opens the game open last
    let last = daydream_server::games::workspace::Workspace::new(m.workspace.clone())
        .last()
        .unwrap();
    assert_eq!(last.realm_dir, emu_copy);
}

/// The MBBSEmu game: its own files, opened where they are by an earlier Daydream or named at
/// start, are worked on in their copy from the next start.
#[tokio::test]
async fn a_games_own_db_files_given_at_start_give_way_to_their_copy() {
    let m = machine();
    let emu = std::fs::canonicalize(m.root.join("emu")).unwrap();
    let app = daydream_server::router_with(
        Some(&emu),
        Options {
            games: Config {
                workspace: Some(m.workspace.clone()),
                search: vec![m.root.clone()],
                default_search: false,
                ..Config::default()
            },
            ..Options::default()
        },
    );
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    let current = &list["current"];
    assert_eq!(current["copy"], true, "{list}");
    assert_eq!(current["live"], false, "{list}");
    assert_eq!(current["label"], "emu (MBBSEmu, .db files)");
    let copy = PathBuf::from(current["realm_dir"].as_str().unwrap());
    assert!(copy.starts_with(&m.workspace), "{}", copy.display());
    assert!(copy.join("WCCITEMS.db").is_file());
    // the BBS's own files stay out of it
    assert!(!copy.join("BBSUSR.db").exists());
    let last = daydream_server::games::workspace::Workspace::new(m.workspace.clone())
        .last()
        .unwrap();
    assert_eq!(last.realm_dir, copy);
}

/// The MBBSEmu game published: its copy's edit goes into its own .db file once it has
/// stopped, onto the record as play left it, and play's other changes stay.
#[tokio::test]
async fn an_mbbsemu_game_publishes_into_its_db_files() {
    let m = machine();
    let app = m.router(None, false);
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    let emu = game(&list, "db");
    let (_, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": emu["id"]})),
        None,
    )
    .await;
    let copy = PathBuf::from(body["current"]["realm_dir"].as_str().unwrap());
    let game_items = std::fs::canonicalize(m.root.join("emu"))
        .unwrap()
        .join("WCCITEMS.DB");

    let (status, plan) = call(&app, "GET", "/api/publish", None, None).await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["engine"], "mbbsemu");
    assert_eq!(plan["name"], "MBBSEmu");
    assert_eq!(plan["changes"], 0);
    assert_eq!(plan["running"]["running"], false);

    // an edit in the copy while the game plays on
    let mut ours = WbtrvDb::open(&copy.join("WCCITEMS.db")).unwrap();
    let mut edited = ours.get(2).unwrap().unwrap();
    edited[0] ^= 0xFF;
    ours.update(2, &edited).unwrap();
    drop(ours);
    let mut theirs = WbtrvDb::open(&game_items).unwrap();
    theirs.track(false);
    let mut played = theirs.get(3).unwrap().unwrap();
    played[0] ^= 0xFF;
    theirs.update(3, &played).unwrap();
    drop(theirs);

    let (_, plan) = call(&app, "GET", "/api/publish", None, None).await;
    assert_eq!(plan["changes"], 1, "{plan}");
    let ways: Vec<&str> = plan["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].as_str().unwrap())
        .collect();
    assert_eq!(ways, ["now"]);
    assert_eq!(plan["ways"][0]["ready"], true, "{plan}");
    let steps: Vec<&str> = plan["ways"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(steps, ["check", "stop", "backup", "write", "start"]);
    assert_eq!(plan["ways"][0]["steps"][1]["title"], "Stop MBBSEmu");

    let (status, job) = call(
        &app,
        "POST",
        "/api/publish",
        Some(json!({"way": "now"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{job}");
    let job = publish_ends(&app).await;
    assert_eq!(job["state"], "done", "{job}");
    assert_eq!(job["engine"], "mbbsemu");
    let states: Vec<&str> = job["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["state"].as_str().unwrap())
        .collect();
    assert_eq!(states, ["done", "done", "done", "done", "yours"], "{job}");
    assert!(
        job["steps"][3]["log"]
            .to_string()
            .contains("WCCITEMS.DB: 1 changed"),
        "{job}"
    );

    // the edit at the record's own id, play's change kept, the file as it was kept beside it
    let theirs = WbtrvDb::open_read_only(&game_items).unwrap();
    assert_eq!(theirs.get(2).unwrap().unwrap(), edited);
    assert_eq!(theirs.get(3).unwrap().unwrap(), played);
    let backup = std::fs::read_dir(game_items.parent().unwrap())
        .unwrap()
        .flatten()
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("daydream-backup-")
        })
        .expect("a backup folder");
    let saved = WbtrvDb::open_read_only(&backup.path().join("WCCITEMS.DB")).unwrap();
    assert_ne!(saved.get(2).unwrap().unwrap(), edited);
    assert_eq!(saved.get(3).unwrap().unwrap(), played);
    let (_, changes) = call(&app, "GET", "/api/changes", None, None).await;
    assert_eq!(changes["changes"], json!([]), "{changes}");
}

/// Follows the publish under way until it ends.
async fn publish_ends(app: &axum::Router) -> Value {
    for _ in 0..200 {
        let (status, job) = call(app, "GET", "/api/publish/job", None, None).await;
        assert_eq!(status, StatusCode::OK, "{job}");
        if job["state"] != "running" {
            return job;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("the publish did not end");
}

#[tokio::test]
async fn a_wcc_game_publishes_step_by_step() {
    let m = machine();
    let app = m.router(None, false);
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    let wg = game(&list, "wcc");
    let (_, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": wg["id"]})),
        None,
    )
    .await;
    let copy = PathBuf::from(body["current"]["realm_dir"].as_str().unwrap());

    // nothing changed yet: nothing to publish, either way
    let (status, plan) = call(&app, "GET", "/api/publish", None, None).await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["engine"], "majorbbs");
    assert_eq!(plan["changes"], 0);
    assert_eq!(plan["running"]["running"], false);
    assert!(
        plan["checks"].to_string().contains("Nothing is waiting"),
        "{plan}"
    );
    let ways: Vec<&Value> = plan["ways"].as_array().unwrap().iter().collect();
    assert_eq!(
        ways.iter()
            .map(|w| w["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["update", "now"]
    );
    assert!(ways.iter().all(|w| w["ready"] == false), "{plan}");
    // the game had no update file, so its working copy has none to write
    assert!(
        ways[0]["why"].as_str().unwrap().contains("no update file"),
        "{plan}"
    );
    let (status, body) = call(
        &app,
        "POST",
        "/api/publish",
        Some(json!({"way": "now"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "nothing is waiting to publish");

    // an edit: publishing now is ready, through the update file is not
    WbtrvDb::open(&copy.join("WCCITEMS.db"))
        .unwrap()
        .delete(2)
        .unwrap();
    let (_, plan) = call(&app, "GET", "/api/publish", None, None).await;
    assert_eq!(plan["changes"], 1);
    assert_eq!(plan["way"], "now");
    let steps: Vec<&str> = plan["ways"][1]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert_eq!(steps, ["check", "stop", "backup", "write", "start"]);
    let (status, body) = call(
        &app,
        "POST",
        "/api/publish",
        Some(json!({"way": "update"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    let before = std::fs::read(m.wg().join("WCCITEMS.DAT")).unwrap();
    let (status, job) = call(
        &app,
        "POST",
        "/api/publish",
        Some(json!({"way": "now"})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{job}");
    assert_eq!(job["state"], "running");
    let job = publish_ends(&app).await;
    assert_eq!(job["state"], "done", "{job}");
    let states: Vec<&str> = job["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["state"].as_str().unwrap())
        .collect();
    assert_eq!(states, ["done", "done", "done", "done", "yours"], "{job}");
    assert!(
        job["steps"][1]["log"][0]
            .as_str()
            .unwrap()
            .contains("not running"),
        "{job}"
    );
    assert!(
        job["steps"][3]["log"].to_string().contains("WCCITEMS.DAT"),
        "{job}"
    );

    // written, with the file as it was kept beside it, and nothing waits any more
    assert_ne!(std::fs::read(m.wg().join("WCCITEMS.DAT")).unwrap(), before);
    let backup = std::fs::read_dir(m.wg())
        .unwrap()
        .flatten()
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("daydream-backup-")
        })
        .expect("a backup folder");
    assert_eq!(
        std::fs::read(backup.path().join("WCCITEMS.DAT")).unwrap(),
        before
    );
    let (_, changes) = call(&app, "GET", "/api/changes", None, None).await;
    assert_eq!(changes["changes"], json!([]), "{changes}");
    let (_, plan) = call(&app, "GET", "/api/publish", None, None).await;
    assert_eq!(plan["job"]["state"], "done");
    // no publish is waiting, so none is cancelled
    let (_, body) = call(&app, "POST", "/api/publish/cancel", None, None).await;
    assert_eq!(body["cancelled"], false);
}

#[tokio::test]
async fn a_typed_folder_opens_here_but_not_for_a_token_holder() {
    let m = machine();
    let typed =
        json!({"kind": "folder", "location": m.root.join("bbs/WGSERV").display().to_string()});
    let (status, body) = call(
        &m.router(None, false),
        "POST",
        "/api/games/open",
        Some(typed.clone()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["current"]["kind"], "wcc");

    let app = m.router(Some(TOKEN), false);
    let (status, list) = call(&app, "GET", "/api/games", None, Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list["custom"], false);
    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(typed.clone()),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    // what it found itself it opens
    let id = game(&list, "wcc")["id"].clone();
    let (status, _) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": id})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // and it writes only there
    let elsewhere = m.root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    for body in [
        json!({"wcc": elsewhere.display().to_string(), "dry_run": true}),
        json!({"out": elsewhere.display().to_string(), "dry_run": true}),
    ] {
        let (status, res) = call(
            &app,
            "POST",
            "/api/tools/write-dat",
            Some(body),
            Some(TOKEN),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{res}");
    }
    let (status, res) = call(
        &app,
        "POST",
        "/api/tools/write-dat",
        Some(json!({"dry_run": true})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");
    let (status, res) = call(
        &app,
        "POST",
        "/api/silvermere/info",
        Some(json!({"database": "postgres://u:p@db.example.com/x"})),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{res}");

    // --allow-any-source lifts it
    let app = m.router(Some(TOKEN), true);
    let (status, body) = call(&app, "POST", "/api/games/open", Some(typed), Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_folder_without_the_game_is_refused() {
    let m = machine();
    let typed = json!({"kind": "folder", "location": m.root.display().to_string()});
    let (status, body) = call(
        &m.router(None, false),
        "POST",
        "/api/games/open",
        Some(typed),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("no MajorMUD files"),
        "{body}"
    );
}

/// A silvermere database found by its settings file opens into a working copy of its world,
/// and the silvermere card then works on it unasked. Needs the stock world (CI keeps it).
#[tokio::test]
async fn a_silvermere_database_opens_into_a_working_copy() {
    let vir = daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
        panic!("the test needs the stock distribution and could not get it: {e}")
    });
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    daydream_store::realm::convert_dir(&vir, &src).unwrap();
    let install = tmp.path().join("games").join("silvermere");
    std::fs::create_dir_all(install.join("data")).unwrap();
    std::fs::write(
        install.join("silvermere.toml"),
        "[database]\nurl = \"\"   # the default\n",
    )
    .unwrap();
    let url = daydream_silvermere::database_url(
        &install.join("data/silvermere.db").display().to_string(),
    );
    let realm = daydream_store::Realm::open(&src).unwrap();
    daydream_silvermere::export(&realm, &url, &daydream_silvermere::ExportOptions::default())
        .await
        .unwrap();

    let app = daydream_server::router_with(
        None,
        Options {
            games: Config {
                workspace: Some(tmp.path().join("workspace")),
                search: vec![tmp.path().join("games")],
                default_search: false,
                allow_any_source: false,
                ..Config::default()
            },
            ..Options::default()
        },
    );
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    let sm = game(&list, "silvermere");
    assert!(
        sm["found_by"]
            .as_str()
            .unwrap()
            .ends_with("silvermere.toml"),
        "{sm}"
    );
    let id = sm["id"].as_str().unwrap().to_string();
    let (_, st) = call(
        &app,
        "GET",
        &format!("/api/games/status?id={id}"),
        None,
        None,
    )
    .await;
    assert_eq!(st["running"], false, "{st}");

    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": id})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let copy = PathBuf::from(body["current"]["realm_dir"].as_str().unwrap());
    for stem in ["WCCITEMS", "WCCKNMSR", "WCCMP001", "WCCTEXT"] {
        let n = |dir: &Path| {
            WbtrvDb::open_read_only(&dir.join(format!("{stem}.db")))
                .unwrap()
                .records()
                .unwrap()
        };
        assert_eq!(n(&copy), n(&src), "{stem}");
    }
    // the silvermere card is on the open game's database without being told
    let (status, info) = call(&app, "POST", "/api/silvermere/info", Some(json!({})), None).await;
    assert_eq!(status, StatusCode::OK, "{info}");
    assert_eq!(info["running"], Value::Null);
    let (status, res) = call(
        &app,
        "POST",
        "/api/silvermere/export",
        Some(json!({"dry_run": true})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{res}");

    // with silvermere itself serving the database (SILVERMERE_BIN: CI keeps the release),
    // the picker sees it run, and neither a refresh nor the card's export goes under it
    if let Some(bin) = &silvermere_bin() {
        let serving = serve(bin, &url);
        let (_, st) = call(&app, "GET", "/api/games/status?id=current", None, None).await;
        assert_eq!(st["running"], true, "{st}");
        // reading it again would read a world silvermere is changing
        let (status, body) = call(
            &app,
            "POST",
            "/api/games/open",
            Some(json!({"id": id, "refresh": true})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert!(
            body["error"].as_str().unwrap().contains("Stop it first"),
            "{body}"
        );
        let (status, res) = call(
            &app,
            "POST",
            "/api/silvermere/export",
            Some(json!({})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{res}");
        drop(serving);
        let (status, res) = call(
            &app,
            "POST",
            "/api/silvermere/export",
            Some(json!({})),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{res}");
    }
}

/// The silvermere release to run, when given (`SILVERMERE_BIN`).
fn silvermere_bin() -> Option<PathBuf> {
    std::env::var_os("SILVERMERE_BIN")
        .filter(|b| !b.is_empty())
        .map(Into::into)
}

/// `silvermere serve` on a database, stopped when dropped.
struct Serving {
    child: std::process::Child,
    /// Its data folder, removed after it stops.
    _data: tempfile::TempDir,
}

impl Drop for Serving {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts `bin serve` on `url` and waits until it listens, or panics with its output.
fn serve(bin: &Path, url: &str) -> Serving {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let data = tempfile::tempdir().unwrap();
    let mut child = Command::new(bin)
        .arg("serve")
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
        .stderr(Stdio::piped())
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
    let serving = Serving { child, _data: data };
    let mut seen = Vec::new();
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(60)) {
            Ok(line) if line.contains("telnet listening") => return serving,
            Ok(line) => seen.push(line),
            Err(e) => panic!("silvermere did not start ({e:?}):\n{}", seen.join("\n")),
        }
    }
}

#[tokio::test]
async fn the_mcp_server_connects_with_no_game_open_and_opens_one() {
    let m = machine();
    let app = m.router(None, false);
    let rpc = |id: u64, method: &str, params: Value| serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
    let (status, init) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(1, "initialize", serde_json::json!({}))),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{init}");
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("open_game")
    );
    let (_, list) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(2, "tools/list", serde_json::json!({}))),
        None,
    )
    .await;
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"list_tables") && names.contains(&"open_game"),
        "{names:?}"
    );

    let call_tool = |id: u64, name: &str, args: Value| {
        rpc(
            id,
            "tools/call",
            serde_json::json!({ "name": name, "arguments": args }),
        )
    };
    let (_, closed) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(3, "list_tables", serde_json::json!({}))),
        None,
    )
    .await;
    assert_eq!(closed["result"]["isError"], true, "{closed}");

    let (_, games) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(4, "list_games", serde_json::json!({}))),
        None,
    )
    .await;
    assert_eq!(games["result"]["isError"], false, "{games}");
    let emu = game(&games["result"]["structuredContent"], "db")["id"].clone();
    let (_, opened) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(5, "open_game", serde_json::json!({ "id": emu }))),
        None,
    )
    .await;
    assert_eq!(opened["result"]["isError"], false, "{opened}");
    let (_, tables) = call(
        &app,
        "POST",
        "/mcp",
        Some(call_tool(6, "list_tables", serde_json::json!({}))),
        None,
    )
    .await;
    assert_eq!(tables["result"]["isError"], false, "{tables}");
}

/// silvermere published end to end, as a sysop with an admin token has it: told to warn the
/// players (nobody is in the game, so it stops at once), silvermere saves and stops, the
/// world is written, and Daydream starts it again with the command line it ran with. Needs
/// the stock world and `SILVERMERE_BIN` (CI keeps both); skipped without the binary.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn silvermere_is_published_and_started_again() {
    let Some(bin) = silvermere_bin() else {
        eprintln!("SILVERMERE_BIN is not set: skipped");
        return;
    };
    let vir = daydream_wcc::stock_files::vir_dir(None).unwrap_or_else(|e| {
        panic!("the test needs the stock distribution and could not get it: {e}")
    });
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    daydream_store::realm::convert_dir(&vir, &src).unwrap();
    let install = tmp.path().join("games").join("silvermere");
    let (token, admin_port) = silvermere_install(&bin, &install);
    let url = daydream_silvermere::database_url(
        &install.join("data/silvermere.db").display().to_string(),
    );
    let realm = daydream_store::Realm::open(&src).unwrap();
    daydream_silvermere::export(&realm, &url, &daydream_silvermere::ExportOptions::default())
        .await
        .unwrap();

    let app = daydream_server::router_with(
        None,
        Options {
            games: Config {
                workspace: Some(tmp.path().join("workspace")),
                search: vec![tmp.path().join("games")],
                default_search: false,
                allow_any_source: false,
                ..Config::default()
            },
            ..Options::default()
        },
    );
    let (_, list) = call(&app, "GET", "/api/games", None, None).await;
    let id = game(&list, "silvermere")["id"].clone();
    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({ "id": id })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    publish_with_silvermere_running(&app, &bin, &install, &token, admin_port).await;
}

/// A silvermere installation at `install`, as a sysop sets one up: `silvermere.toml` with an
/// admin API on a free port and an admin token for Daydream, which `silvermere admin token
/// new` made. The token, and the admin API's port.
fn silvermere_install(bin: &Path, install: &Path) -> (String, u16) {
    std::fs::create_dir_all(install.join("data")).unwrap();
    let made = std::process::Command::new(bin)
        .current_dir(install)
        .args(["admin", "token", "new", "daydream"])
        .args(["--scopes", "read,write,admin"])
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&made.stdout).into_owned();
    assert!(
        made.status.success(),
        "{out}{}",
        String::from_utf8_lossy(&made.stderr)
    );
    let token = out
        .lines()
        .find_map(|l| l.strip_prefix("Token (shown once, store it now):"))
        .unwrap_or_else(|| panic!("no token in {out}"))
        .trim()
        .to_string();
    let block = &out[out
        .find("[[admin.tokens]]")
        .expect("the [[admin.tokens]] block")..];
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::fs::write(
        install.join("silvermere.toml"),
        format!(
            "[server]\ntelnet_bind = \"127.0.0.1:0\"\nadmin_bind = \"127.0.0.1:{port}\"\n\
             web_bind = \"\"\n\n{block}"
        ),
    )
    .unwrap();
    (token, port)
}

/// Whether silvermere's admin API at `port` answers.
fn answers(port: u16) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .proxy(None)
        .timeout_global(Some(std::time::Duration::from_secs(2)))
        .build()
        .into();
    agent
        .get(&format!("http://127.0.0.1:{port}/healthz"))
        .call()
        .is_ok()
}

/// The silvermere a test started, and the one Daydream started again after it, stopped
/// when dropped (by the pid in its lock file).
struct Silvermere {
    first: std::process::Child,
    lock: PathBuf,
}

impl Drop for Silvermere {
    fn drop(&mut self) {
        let _ = self.first.kill();
        let _ = self.first.wait();
        let pid: String = std::fs::read_to_string(&self.lock)
            .unwrap_or_default()
            .chars()
            .filter(char::is_ascii_digit)
            .collect();
        if !pid.is_empty() {
            let _ = std::process::Command::new("kill").arg(&pid).status();
        }
    }
}

/// With the silvermere game at `install` open in `app` and stopped: starts it in its folder
/// as a sysop would, gives Daydream the admin token and lets it start silvermere again, then
/// edits an item and publishes, and checks every step went through and silvermere answers
/// again.
async fn publish_with_silvermere_running(
    app: &axum::Router,
    bin: &Path,
    install: &Path,
    token: &str,
    admin_port: u16,
) {
    let log = std::fs::File::create(install.join("serve.log")).unwrap();
    let first = std::process::Command::new(bin)
        .current_dir(install)
        .arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .unwrap();
    let running = Silvermere {
        first,
        lock: install.join("data/silvermere.db.lock"),
    };
    let started = std::time::Instant::now();
    while !answers(admin_port) {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(60),
            "silvermere did not start:\n{}",
            std::fs::read_to_string(install.join("serve.log")).unwrap_or_default()
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    // before the token, the plan waits for the sysop to stop it
    let (_, plan) = call(app, "GET", "/api/publish", None, None).await;
    assert_eq!(plan["control"]["token_state"], "none", "{plan}");
    let steps = |plan: &Value| -> Vec<String> {
        plan["ways"][0]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    };
    assert!(!steps(&plan).contains(&"warn".to_string()), "{plan}");

    let (status, control) = call(
        app,
        "PUT",
        "/api/control",
        Some(json!({ "token": token, "autostart": true })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{control}");
    assert_eq!(control["token_state"], "ok", "{control}");
    assert_eq!(control["start"]["kind"], "process", "{control}");
    assert_eq!(control["players"], json!([]), "{control}");
    // the token is never given back
    assert!(!control.to_string().contains(token));

    // an edit to publish
    let (_, rows) = call(app, "GET", "/api/tables/Items/rows", None, None).await;
    let key = rows["rows"][0][0].clone();
    let uri = format!("/api/tables/Items/row?key={}", url_escape(&key.to_string()));
    let (status, saved) = call(
        app,
        "PUT",
        &uri,
        Some(json!({"values": {"Name": "published sword"}})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");

    let (_, plan) = call(app, "GET", "/api/publish", None, None).await;
    assert_eq!(
        steps(&plan),
        ["check", "warn", "stop", "backup", "write", "start"],
        "{plan}"
    );
    let (status, job) = call(
        app,
        "POST",
        "/api/publish",
        Some(json!({"way": "now", "warn": {"in_secs": 300, "message": "a test"}})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{job}");
    let started = std::time::Instant::now();
    let job = loop {
        let (_, job) = call(app, "GET", "/api/publish/job", None, None).await;
        if job["state"] != "running" {
            break job;
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(240),
            "the publish did not end: {job}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    };
    assert_eq!(job["state"], "done", "{job}");
    for step in job["steps"].as_array().unwrap() {
        assert_eq!(step["state"], "done", "{job}");
    }
    assert!(
        job["result"].as_str().unwrap().contains("running again"),
        "{job}"
    );
    // nobody was in the game, so no countdown
    assert!(
        job["steps"][1]["log"][0]
            .as_str()
            .unwrap()
            .contains("Nobody is in the game"),
        "{job}"
    );
    assert!(answers(admin_port), "{job}");
    let (_, changes) = call(app, "GET", "/api/changes", None, None).await;
    assert_eq!(changes["changes"], json!([]), "{changes}");
    drop(running);
}

/// `text` made safe for a query string.
fn url_escape(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
