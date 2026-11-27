//! The checks that read the wiring, on the stock 1.11p world: what they find there is known,
//! so each check is held to it (a price the smith's words contradict, a casino line that
//! charges ten times its twins, rooms that spawn nothing, a blank blow, a typo).

use daydream_nmr::wiring::{self, Check, Level, Rec, Report};

fn stock() -> std::sync::Arc<Report> {
    wiring::report(&crate::support::stock_realm()).unwrap()
}

/// The findings of one check at one level, as text.
fn said(r: &Report, check: Check, level: Level) -> Vec<String> {
    r.findings
        .iter()
        .filter(|f| f.check == check && f.level == level)
        .map(|f| format!("{}: {}", f.about.label(), r.text(f)))
        .collect()
}

#[test]
fn the_stock_world_s_prices() {
    let r = stock();
    assert_eq!(
        said(&r, Check::Prices, Level::Warn),
        [
            "text block 1338: Charges 200 gold, but its own words quote 2,000 gold: message 1711 (He says, \"It will cost you 2000 gold if you…). That is a tenth of the 2,000 gold item 967 (golden hide tunic) costs.",
            "text block 9135: Charges 10 gold (line 6), but lines 4, 5 and 7 charge 1 gold to do the same.",
        ]
    );
    // the dwarven smith's prices are as his words say, twice what his wares cost
    let ok = said(&r, Check::Prices, Level::Ok);
    assert!(
        ok.contains(&"text block 911: Charges 2,000 gold, as text block 909 quotes. That is twice the 1,000 gold item 868 (dwarven axe) costs.".to_string()),
        "{ok:#?}"
    );
    assert!(said(&r, Check::Prices, Level::Note).len() < 10);
}

#[test]
fn the_stock_world_s_spawns_and_placeholders() {
    let r = stock();
    let rooms = said(&r, Check::Spawns, Level::Warn);
    assert_eq!(rooms.len(), 8, "{rooms:#?}");
    assert!(rooms.contains(&"room 1/1324: Spawns from group 5 at index 20, but group 5 has none there (its nearest are 19 and 21), so it spawns nothing.".to_string()));
    // the merchant's bodyguard is summoned when he appears
    let guard = r
        .findings
        .iter()
        .find(|f| f.check == Check::Spawns && f.about == Rec::of("Monsters", 382))
        .unwrap();
    assert_eq!(guard.level, Level::Ok);
    assert!(r.text(guard).ends_with("Spell 483 (bodyguard) summons it, cast by monster 381 (dwarven merchant)'s Create Spell."), "{}", r.text(guard));
    // blank blows, with their fix
    let blows: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.check == Check::Placeholders && f.level == Level::Warn)
        .collect();
    assert_eq!(blows.len(), 9);
    assert_eq!(blows[0].about, Rec::of("Monsters", 39));
    assert_eq!(blows[0].fix.len(), 3);
}

#[test]
fn the_stock_world_s_spelling() {
    let r = stock();
    let typo = r
        .findings
        .iter()
        .find(|f| f.word.as_deref() == Some("exhange"))
        .unwrap();
    assert_eq!(typo.about, Rec::of("Messages", 1461));
    assert_eq!(typo.suggest, ["exchange"]);
    let n = r
        .findings
        .iter()
        .filter(|f| f.check == Check::Spelling)
        .count();
    assert!((400..800).contains(&n), "{n} spelling findings");
}
