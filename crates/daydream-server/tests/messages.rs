//! Messages as the editors share them: who uses each one, what each use reads, and a
//! record's own copy of one it shares, over a realm built through the API.

use std::path::Path;

use axum::http::StatusCode;
use serde_json::{Value, json};

mod support;
use support::{add, empty_realm, get, key, send};

async fn death_msg(dir: &Path, monster: i64) -> Value {
    let row = get(
        dir,
        &format!("/api/tables/Monsters/row?key={}", key(json!([monster]))),
    )
    .await;
    row["values"]["Death Msg"].clone()
}

#[tokio::test]
async fn messages_say_who_shares_them_and_a_record_takes_its_own() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    empty_realm(dir);
    add(
        dir,
        "Messages",
        json!({ "Number": 9,
            "Line 1": "The %s bites you for %d damage!",
            "Line 2": "The %s bites %s for %s damage!",
            "Line 3": "The cave newt squeaks, and dies." }),
    )
    .await;
    add(
        dir,
        "Messages",
        json!({ "Number": 10, "Line 1": "A %s scurries in from %s." }),
    )
    .await;
    add(
        dir,
        "Monsters",
        json!({ "Number": 1, "Name": "cave newt", "Move Msg": 10, "Death Msg": 9,
            "Attack Type 0": 1, "Attack % 0": 100, "Attack Hit Msg 0": 9 }),
    )
    .await;
    add(
        dir,
        "Monsters",
        json!({ "Number": 2, "Name": "marsh toad", "Death Msg": 9 }),
    )
    .await;
    add(
        dir,
        "Textblocks",
        json!({ "Number": 5, "Part #": 0, "Data": "pet newt:message 9\n" }),
    )
    .await;

    // who uses each: a record once, however many of its columns name it
    let users = get(dir, "/api/messages/users?numbers=9,10,77").await;
    let of = |users: &Value, n: &str| -> Vec<String> {
        users["users"][n]["some"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| format!("{} {}", u["table"].as_str().unwrap(), u["key"]))
            .collect()
    };
    assert_eq!(
        of(&users, "9"),
        ["Monsters [1]", "Monsters [2]", "Textblocks [5,0]"],
        "{users}"
    );
    assert_eq!(users["users"]["9"]["others"], 3);
    assert_eq!(
        users["users"]["9"]["tables"],
        json!({ "Monsters": 2, "Textblocks": 1 })
    );
    assert_eq!(users["users"]["9"]["some"][1]["title"], "marsh toad");
    assert_eq!(of(&users, "10"), ["Monsters [1]"]);
    assert_eq!(of(&users, "77"), Vec::<String>::new());
    // the record asking is left out
    let mine = get(
        dir,
        &format!(
            "/api/messages/users?numbers=9&table=Monsters&key={}",
            key(json!([2]))
        ),
    )
    .await;
    assert_eq!(of(&mine, "9"), ["Monsters [1]", "Textblocks [5,0]"]);
    assert_eq!(mine["users"]["9"]["others"], 2);
    for bad in ["numbers=nine", "numbers=9&table=Monsters"] {
        let (status, _) = send(dir, "GET", &format!("/api/messages/users?{bad}"), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }

    // what each use reads
    let refs = get(
        dir,
        &format!("/api/references?table=Messages&key={}", key(json!([9]))),
    )
    .await;
    let death = refs["references"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["column"] == "Death Msg" && r["key"] == json!([2]))
        .unwrap();
    assert_eq!(death["reads"][0]["line"], 3);
    assert_eq!(death["reads"][0]["to"], "the room");
    let hit = refs["references"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["column"] == "Attack Hit Msg 0")
        .unwrap();
    assert_eq!(
        hit["reads"][0]["fills"],
        json!(["its name", "the damage", "a star (*)", "a star (*)"])
    );
    // a record's references say nothing of reads
    let rooms = get(
        dir,
        &format!("/api/references?table=Monsters&key={}", key(json!([1]))),
    )
    .await;
    assert!(rooms["references"].as_array().unwrap().is_empty());

    // the toad's own copy: the newt's name in its death line becomes the toad's
    let own = json!({ "table": "Monsters", "key": [2], "number": 9 });
    let mut dry = own.clone();
    dry["dry_run"] = json!(true);
    let (status, plan) = send(dir, "POST", "/api/messages/own", Some(dry)).await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["number"], 11);
    assert_eq!(plan["lines"][2], "The marsh toad squeaks, and dies.");
    assert_eq!(plan["columns"], json!(["Death Msg"]));
    assert_eq!(plan["swapped"], 1);
    assert_eq!(death_msg(dir, 2).await, 9, "a dry run writes nothing");

    let (status, done) = send(dir, "POST", "/api/messages/own", Some(own)).await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(death_msg(dir, 2).await, 11);
    let copy = get(
        dir,
        &format!("/api/tables/Messages/row?key={}", key(json!([11]))),
    )
    .await;
    assert_eq!(
        copy["values"]["Line 3"],
        "The marsh toad squeaks, and dies."
    );
    assert_eq!(copy["values"]["Line 1"], "The %s bites you for %d damage!");
    // the newt keeps the old one, and who uses each is read again
    assert_eq!(death_msg(dir, 1).await, 9);
    let users = get(dir, "/api/messages/users?numbers=9,11").await;
    assert_eq!(
        of(&users, "9"),
        ["Monsters [1]", "Textblocks [5,0]"],
        "{users}"
    );
    assert_eq!(of(&users, "11"), ["Monsters [2]"]);

    // one step: undo takes back the copy and the change together
    let history = get(dir, "/api/history").await;
    assert_eq!(
        history["steps"][0]["label"], "Give Monsters 2 its own message 9",
        "{history}"
    );
    assert_eq!(history["steps"][0]["records"], 2, "{history}");
    let (status, body) = send(dir, "POST", "/api/undo", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(death_msg(dir, 2).await, 9);
    let (status, _) = send(
        dir,
        "GET",
        &format!("/api/tables/Messages/row?key={}", key(json!([11]))),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // a message the record does not name, or one the world lacks
    let (status, body) = send(
        dir,
        "POST",
        "/api/messages/own",
        Some(json!({ "table": "Monsters", "key": [2], "number": 10 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("does not name message 10")
    );
    let (status, _) = send(
        dir,
        "POST",
        "/api/messages/own",
        Some(json!({ "table": "Monsters", "key": [7], "number": 9 })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
