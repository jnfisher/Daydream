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
/// (`emu`), under `root`; and the workspace beside them.
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

    // the MBBSEmu game is edited where it is, and has nothing to write back
    let (status, body) = call(
        &app,
        "POST",
        "/api/games/open",
        Some(json!({"id": emu["id"]})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["current"]["live"], true);
    assert_eq!(
        body["current"]["realm_dir"],
        std::fs::canonicalize(m.root.join("emu"))
            .unwrap()
            .display()
            .to_string()
    );
    let (status, body) = call(
        &app,
        "POST",
        "/api/tools/write-dat",
        Some(json!({"dry_run": true})),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // a later start opens the game open last
    let last = daydream_server::games::workspace::Workspace::new(m.workspace.clone())
        .last()
        .unwrap();
    assert_eq!(
        last.realm_dir,
        std::fs::canonicalize(m.root.join("emu")).unwrap()
    );
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
}
