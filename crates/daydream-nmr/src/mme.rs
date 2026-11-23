//! NMR's Export to MMUD Explorer (`frmMME_Export`): the realm as the `.mdb` MME reads.
//!
//! MME shows players what is in the game, so the export is NMR's, value for value: the
//! same tables and columns (`CreateTables`), the same values from each record (`Export*`),
//! and the same rooms left out (NMR's default exclusion list of sysop and test rooms, and
//! a sysop's own list).
//!
//! It covers the record tables' own values, the rooms and their exits as MME shows them,
//! the version row, the walk that marks what is in the game and where it comes from
//! (`walk`: `In Game`, `Obtained From`, `References`, `Learned From`, `Casted By`,
//! `Summoned By`, `Assigned To`, `TBInfo`), and the lairs and their averages (`lairs`).
//! Still to come: NMR's monster attack simulation (`AvgDmg*`, `AttTrue%`, and from them
//! `ScriptValue`); until then those columns hold 0, as NMR writes them before it.

use std::collections::HashMap;

mod lairs;
mod walk;

use daydream_store::Realm;
use daydream_types::records::LayoutSet;

use crate::data::{NmrData, NmrRows};
use crate::export::{ExportError, export_table};
use crate::schema::{self, ColumnType, Value};

/// A table of MME's `.mdb`, as `CreateTables` makes it.
#[derive(Debug, Clone, PartialEq)]
pub struct MmeTable {
    pub name: &'static str,
    pub columns: Vec<(String, ColumnType)>,
    /// The key: its name, whether it is primary (else a plain index), its columns.
    pub key: (&'static str, bool, Vec<&'static str>),
}

/// `adVarWChar` with no size: Jet's 255.
const TEXT: ColumnType = ColumnType::Text(255);
const LONG: ColumnType = ColumnType::Long;
const DOUBLE: ColumnType = ColumnType::Double;
const MEMO: ColumnType = ColumnType::Memo;

/// The exits' columns, by exit number.
pub const DIRECTIONS: [&str; 10] = ["N", "S", "E", "W", "NE", "NW", "SE", "SW", "U", "D"];

fn cols(list: &[(&str, ColumnType)]) -> Vec<(String, ColumnType)> {
    list.iter().map(|(n, t)| (n.to_string(), *t)).collect()
}

fn abilities(out: &mut Vec<(String, ColumnType)>, n: usize) {
    for x in 0..n {
        out.push((format!("Abil-{x}"), LONG));
        out.push((format!("AbilVal-{x}"), LONG));
    }
}

/// MME's tables, in `CreateTables` order (1.11p: Monsters have `ExpMulti`).
pub fn tables() -> Vec<MmeTable> {
    let mut rooms = cols(&[
        ("Map Number", LONG),
        ("Room Number", LONG),
        ("Name", ColumnType::Text(55)),
        ("Light", LONG),
        ("Shop", LONG),
        ("NPC", LONG),
        ("CMD", LONG),
        ("Spell", LONG),
        ("Lair", TEXT),
        ("Delay", LONG),
        ("Placed", TEXT),
    ]);
    rooms.extend(DIRECTIONS.iter().map(|d| (d.to_string(), TEXT)));

    let lairs = cols(&[
        ("GroupIndex", TEXT),
        ("MobList", TEXT),
        ("Mobs", LONG),
        ("TotalLairs", LONG),
        ("AvgDelay", LONG),
        ("AvgWalk", DOUBLE),
        ("AvgExp", DOUBLE),
        ("AvgDmg", DOUBLE),
        ("AvgDmgPhys", DOUBLE),
        ("AvgDmgSpell", DOUBLE),
        ("AvgHP", DOUBLE),
        ("AvgAC", LONG),
        ("AvgDR", LONG),
        ("AvgMR", LONG),
        ("AvgDodge", LONG),
    ]);

    let mut classes = cols(&[
        ("Number", LONG),
        ("Name", ColumnType::Text(29)),
        ("MinHits", LONG),
        ("MaxHits", LONG),
        ("ExpTable", LONG),
        ("MageryType", LONG),
        ("MageryLVL", LONG),
        ("WeaponType", LONG),
        ("ArmourType", LONG),
        ("CombatLVL", LONG),
    ]);
    abilities(&mut classes, 10);

    let mut races = cols(&[("Number", LONG), ("Name", ColumnType::Text(29))]);
    for p in ["m", "x"] {
        for s in ["INT", "WIL", "STR", "HEA", "AGL", "CHM"] {
            races.push((format!("{p}{s}"), LONG));
        }
    }
    races.extend(cols(&[
        ("HPPerLVL", LONG),
        ("ExpTable", LONG),
        ("BaseCP", LONG),
    ]));
    abilities(&mut races, 10);

    let mut spells = cols(&[
        ("Number", LONG),
        ("Name", ColumnType::Text(29)),
        ("Short", ColumnType::Text(5)),
    ]);
    for c in [
        "ReqLevel",
        "EnergyCost",
        "ManaCost",
        "MinBase",
        "MaxBase",
        "Diff",
        "TypeOfResists",
        "Targets",
        "Dur",
        "AttType",
        "Magery",
        "MageryLVL",
        "Cap",
        "MaxIncLVLs",
        "MaxInc",
        "MinIncLVLs",
        "MinInc",
        "DurIncLVLs",
        "DurInc",
    ] {
        spells.push((c.to_string(), LONG));
    }
    abilities(&mut spells, 10);
    spells.extend(cols(&[
        ("Learnable", LONG),
        ("Learned From", TEXT),
        ("Casted By", TEXT),
        ("Classes", TEXT),
    ]));

    let mut monsters = cols(&[
        ("Number", LONG),
        ("Name", ColumnType::Text(29)),
        ("Weapon", LONG),
        ("ArmourClass", LONG),
        ("DamageResist", LONG),
        ("Follow%", LONG),
        ("MagicRes", LONG),
        ("BSDefense", LONG),
        ("EXP", DOUBLE),
        ("ExpMulti", DOUBLE),
        ("HP", LONG),
        ("Energy", LONG),
        ("AvgDmg", DOUBLE),
        ("AvgDmgPhys", DOUBLE),
        ("AvgDmgSpell", DOUBLE),
        ("GreetTXT", LONG),
        ("HPRegen", LONG),
        ("CharmLVL", LONG),
        ("Type", LONG),
        ("Undead", LONG),
        ("Align", LONG),
        ("RegenTime", LONG),
        ("GameLimit", LONG),
        ("AvgLairExp", DOUBLE),
        ("ScriptValue", DOUBLE),
        ("R", LONG),
        ("P", LONG),
        ("G", LONG),
        ("S", LONG),
        ("C", LONG),
        ("DeathSpell", LONG),
        ("CreateSpell", LONG),
    ]);
    for x in 0..5 {
        monsters.push((format!("AttName-{x}"), ColumnType::Text(50)));
        for (c, t) in [
            ("AttType", LONG),
            ("AttAcc", LONG),
            ("Att%", LONG),
            ("AttTrue%", DOUBLE),
            ("AttMin", LONG),
            ("AttMax", LONG),
            ("AttEnergy", LONG),
            ("AttHitSpell", LONG),
        ] {
            monsters.push((format!("{c}-{x}"), t));
        }
    }
    for x in 0..5 {
        for c in ["MidSpell", "MidSpell%", "MidSpellLVL"] {
            monsters.push((format!("{c}-{x}"), LONG));
        }
    }
    for x in 0..10 {
        monsters.push((format!("DropItem-{x}"), LONG));
        monsters.push((format!("DropItem%-{x}"), LONG));
    }
    abilities(&mut monsters, 10);
    monsters.extend(cols(&[("In Game", LONG), ("Summoned By", MEMO)]));

    let mut items = cols(&[("Number", LONG), ("Name", ColumnType::Text(29))]);
    for c in [
        "Limit",
        "Encum",
        "ItemType",
        "UseCount",
        "Price",
        "Currency",
        "Min",
        "Max",
        "ArmourClass",
        "DamageResist",
        "WeaponType",
        "ArmourType",
        "Worn",
        "Accy",
        "Gettable",
        "StrReq",
        "Speed",
        "Not Droppable",
        "Destroy On Death",
        "Retain After Uses",
    ] {
        items.push((c.to_string(), LONG));
    }
    for c in ["ClassRest", "RaceRest", "NegateSpell"] {
        for x in 0..10 {
            items.push((format!("{c}-{x}"), LONG));
        }
    }
    abilities(&mut items, 20);
    items.extend(cols(&[
        ("In Game", LONG),
        ("Obtained From", MEMO),
        ("References", MEMO),
    ]));

    let mut shops = cols(&[
        ("Number", LONG),
        ("Name", ColumnType::Text(39)),
        ("ShopType", LONG),
        ("MinLVL", LONG),
        ("MaxLVL", LONG),
        ("Markup%", LONG),
        ("ClassRest", LONG),
    ]);
    for x in 0..20 {
        for c in ["Item", "Max", "Time", "Amount", "%"] {
            shops.push((format!("{c}-{x}"), LONG));
        }
    }
    shops.extend(cols(&[("In Game", LONG), ("Assigned To", TEXT)]));

    let info = cols(&[
        ("NMR Version", TEXT),
        ("Dat File Version", TEXT),
        ("Date", TEXT),
        ("Time", TEXT),
        ("Custom", TEXT),
        ("Legit", LONG),
        ("DisableKai", LONG),
        ("UpdateURL", TEXT),
    ]);
    let tbinfo = cols(&[
        ("Number", LONG),
        ("LinkTo", LONG),
        ("Action", MEMO),
        ("Called From", TEXT),
    ]);

    let pk = |name, col| (name, true, vec![col]);
    vec![
        MmeTable {
            name: "Rooms",
            columns: rooms,
            key: ("idxRooms", false, vec!["Map Number", "Room Number"]),
        },
        MmeTable {
            name: "Lairs",
            columns: lairs,
            key: pk("pkLairs", "GroupIndex"),
        },
        MmeTable {
            name: "Classes",
            columns: classes,
            key: pk("pkClasses", "Number"),
        },
        MmeTable {
            name: "Races",
            columns: races,
            key: pk("pkRaces", "Number"),
        },
        MmeTable {
            name: "Spells",
            columns: spells,
            key: pk("pkSpells", "Number"),
        },
        MmeTable {
            name: "Monsters",
            columns: monsters,
            key: pk("pkMonsters", "Number"),
        },
        MmeTable {
            name: "Items",
            columns: items,
            key: pk("pkItems", "Number"),
        },
        MmeTable {
            name: "Shops",
            columns: shops,
            key: pk("pkShops", "Number"),
        },
        MmeTable {
            name: "Info",
            columns: info,
            key: ("", false, Vec::new()),
        },
        MmeTable {
            name: "TBInfo",
            columns: tbinfo,
            key: pk("pkTBInfo", "Number"),
        },
    ]
}

impl MmeTable {
    /// The table as a plan for Jet creates it.
    pub fn plan_table(&self) -> crate::jet::PlanTable<'_> {
        crate::jet::PlanTable {
            name: self.name,
            columns: self.columns.iter().map(|(c, t)| (c.as_str(), *t)).collect(),
            key: (!self.key.2.is_empty()).then(|| (self.key.0, self.key.1, self.key.2.clone())),
        }
    }
}

/// The plan `tools/nmr/write-mdb.ps1` turns into MME's `.mdb`.
pub fn plan(data: &NmrData) -> String {
    let tables = tables();
    let plan: Vec<_> = tables.iter().map(MmeTable::plan_table).collect();
    crate::jet::plan_for(&plan, data)
}

/// A range of rooms left out of the export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Excluded {
    pub map: i64,
    pub from: i64,
    pub to: i64,
}

/// NMR's default exclusion list (`AddDefaultExcludes`): sysop rooms, test rooms and
/// a few rooms players cannot reach.
pub const DEFAULT_EXCLUDED: [(i64, i64, i64, &str); 20] = [
    (1, 288, 288, "fake silver river room"),
    (1, 289, 289, "halls of training, entrance"),
    (1, 2306, 2306, "small house, attic"),
    (4, 1, 1, "silvermere property office"),
    (1, 164, 164, "sysop support chamber"),
    (1, 238, 238, "sysop cheat room"),
    (1, 3347, 3347, "sysop support chamber"),
    (1, 2779, 2782, "sysop support module test rooms"),
    (1, 3343, 3346, "sysop support empty rooms"),
    (1, 3437, 3437, "sysop support rubbish room"),
    (17, 2867, 2867, "sysop training room"),
    (2, 2963, 2963, "mod test room, mod 1"),
    (3, 788, 788, "mod test room, mod 2"),
    (8, 1407, 1407, "mod test room, mod 3"),
    (6, 3275, 3275, "mod test room, mod 4"),
    (9, 1432, 1432, "mod test room, mod 5"),
    (12, 2258, 2258, "mod test room, mod 6"),
    (16, 2673, 2673, "mod test room, mod 7"),
    (15, 2055, 2055, "mod test room, mod 8"),
    (17, 2839, 2839, "mod test room, mod 9"),
];

/// NMR's export options (its form's boxes).
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct MmeOptions {
    /// "Legit" (the default): the export MME's own site hands out, with NMR's default
    /// exclusions and no custom name, URL or room list.
    pub legit: bool,
    /// "Disable Kai".
    pub disable_kai: bool,
    /// "Exclude default rooms".
    pub exclude_default: bool,
    /// "Exclude rooms", with `excluded` (not under `legit`).
    pub exclude_rooms: bool,
    pub excluded: Vec<Excluded>,
    /// "Hide excluded rooms": NMR's box keeps the sysop's own excluded rooms in the export,
    /// their details blanked; without it they are left out.
    pub hide_excluded: bool,
    /// "No rooms" (with `exclude_rooms`, not under `legit`): no Rooms table rows at all.
    pub no_rooms: bool,
    /// "Custom" name and update URL (not under `legit`).
    pub custom: String,
    pub update_url: String,
    /// "Dat File Version".
    pub dat_version: String,
    /// When the export was made, as NMR writes VB's `Date` and `Time` (`9/28/2025`,
    /// `3:01:29 PM`).
    pub date: String,
    pub time: String,
}

impl Default for MmeOptions {
    fn default() -> Self {
        MmeOptions {
            legit: true,
            disable_kai: false,
            exclude_default: true,
            exclude_rooms: false,
            excluded: Vec::new(),
            hide_excluded: false,
            no_rooms: false,
            custom: String::new(),
            update_url: String::new(),
            dat_version: crate::export::DAT_VERSION.to_string(),
            date: String::new(),
            time: String::new(),
        }
    }
}

/// The rooms left out (`ExcludedRooms`), and how many of them are NMR's defaults.
struct Exclusions {
    ranges: Vec<Excluded>,
    defaults: usize,
}

impl Exclusions {
    fn new(o: &MmeOptions) -> Exclusions {
        let mut ranges = Vec::new();
        if o.legit || o.exclude_default {
            ranges.extend(DEFAULT_EXCLUDED.iter().map(|&(map, from, to, _)| Excluded {
                map,
                from,
                to,
            }));
        }
        if o.exclude_rooms && !o.legit {
            ranges.extend(o.excluded.iter().copied());
        }
        Exclusions {
            ranges,
            defaults: DEFAULT_EXCLUDED.len(),
        }
    }

    /// `IsExcludedRoom`: with `default_only`, NMR checks only as many ranges as its default
    /// list has (the first ones, whatever they are).
    fn excludes(&self, map: i64, room: i64, default_only: bool) -> bool {
        let n = if default_only {
            self.defaults.min(self.ranges.len())
        } else {
            self.ranges.len()
        };
        self.ranges[..n]
            .iter()
            .any(|r| r.map == map && (r.from..=r.to).contains(&room))
    }
}

/// One source table's rows by column name.
struct Src {
    rows: NmrRows,
}

impl Src {
    fn load(realm: &Realm, name: &str, layout: LayoutSet) -> Result<Src, ExportError> {
        let t = schema::table(name).expect("an NMR table");
        Ok(Src {
            rows: export_table(realm, t, layout)?,
        })
    }

    fn at(&self, column: &str) -> usize {
        self.rows
            .position(column)
            .unwrap_or_else(|| panic!("{} has {column}", self.rows.name))
    }
}

fn num(v: &Value) -> i64 {
    match v {
        Value::Long(n) => i64::from(*n),
        Value::Double(d) => *d as i64,
        _ => 0,
    }
}

/// `ClipNull`: text up to its first NUL.
fn clip(v: &Value) -> String {
    match v {
        Value::Text(s) => s.split('\0').next().unwrap_or("").to_string(),
        _ => String::new(),
    }
}

/// NMR writes `Chr(0)` into text it has nothing for.
fn nul() -> Value {
    Value::Text("\0".to_string())
}

fn long(n: i64) -> Value {
    Value::Long(n as i32)
}

fn text(s: impl Into<String>) -> Value {
    Value::Text(s.into())
}

/// Builds rows of an [`MmeTable`] by column name.
struct Out {
    rows: NmrRows,
    index: HashMap<String, usize>,
}

impl Out {
    fn new(t: &MmeTable) -> Out {
        Out {
            index: t
                .columns
                .iter()
                .enumerate()
                .map(|(i, (c, _))| (c.clone(), i))
                .collect(),
            rows: NmrRows {
                name: t.name.to_string(),
                columns: t.columns.iter().map(|(c, _)| c.clone()).collect(),
                rows: Vec::new(),
            },
        }
    }

    /// A row of NMR's defaults for what the export fills in later: 0, or NUL for text.
    fn blank(&self, t: &MmeTable) -> Vec<Value> {
        t.columns
            .iter()
            .map(|(_, ty)| match ty {
                ColumnType::Long => Value::Long(0),
                ColumnType::Double => Value::Double(0.0),
                _ => nul(),
            })
            .collect()
    }

    fn set(&self, row: &mut [Value], column: &str, v: Value) {
        let i = *self
            .index
            .get(column)
            .unwrap_or_else(|| panic!("{} has {column}", self.rows.name));
        row[i] = v;
    }
}

/// Copies `pairs` (MME column, NMR column) from each source row, `extra` filling the rest.
fn copy_table(
    t: &MmeTable,
    src: &Src,
    pairs: &[(String, String)],
    mut extra: impl FnMut(&Out, &mut Vec<Value>, &[Value]),
) -> NmrRows {
    let mut out = Out::new(t);
    let at: Vec<(String, usize)> = pairs.iter().map(|(m, n)| (m.clone(), src.at(n))).collect();
    for r in &src.rows.rows {
        let mut row = out.blank(t);
        for (m, i) in &at {
            let v = match &r[*i] {
                Value::Text(_) => text(clip(&r[*i])),
                v => v.clone(),
            };
            out.set(&mut row, m, v);
        }
        extra(&out, &mut row, r);
        out.rows.rows.push(row);
    }
    out.rows
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn ability_pairs(out: &mut Vec<(String, String)>, n: usize) {
    for x in 0..n {
        out.push((format!("Abil-{x}"), format!("Ability {x}")));
        out.push((format!("AbilVal-{x}"), format!("Ability Value {x}")));
    }
}

/// The realm as MME's `.mdb` holds it: NMR's export with `o`'s options.
pub fn export(realm: &Realm, layout: LayoutSet, o: &MmeOptions) -> Result<NmrData, ExportError> {
    let tables = tables();
    let table = |name: &str| {
        tables
            .iter()
            .find(|t| t.name == name)
            .expect("an MME table")
    };
    let mut data = NmrData::default();
    let excl = Exclusions::new(o);

    let messages = Src::load(realm, "Messages", layout)?;
    let message: HashMap<i64, [String; 3]> = {
        let (n, l1, l2, l3) = (
            messages.at("Number"),
            messages.at("Line 1"),
            messages.at("Line 2"),
            messages.at("Line 3"),
        );
        messages
            .rows
            .rows
            .iter()
            .map(|r| (num(&r[n]), [clip(&r[l1]), clip(&r[l2]), clip(&r[l3])]))
            .collect()
    };
    let monsters = Src::load(realm, "Monsters", layout)?;
    data.tables.push(rooms(
        realm,
        layout,
        table("Rooms"),
        o,
        &excl,
        &message,
        &monsters,
    )?);
    data.tables.push(NmrRows {
        name: "Lairs".into(),
        columns: table("Lairs")
            .columns
            .iter()
            .map(|(c, _)| c.clone())
            .collect(),
        rows: Vec::new(),
    });

    let mut p = pairs(&[
        ("Number", "Number"),
        ("Name", "Name"),
        ("MinHits", "Min HP"),
        ("MaxHits", "Max HP"),
        ("ExpTable", "EXP %"),
        ("MageryType", "Magic Type"),
        ("MageryLVL", "Magic LVL"),
        ("WeaponType", "Weapon"),
        ("ArmourType", "Armour"),
        ("CombatLVL", "Combat"),
    ]);
    ability_pairs(&mut p, 10);
    let classes = Src::load(realm, "Classes", layout)?;
    data.tables
        .push(copy_table(table("Classes"), &classes, &p, |_, _, _| {}));

    let mut p = pairs(&[("Number", "Number"), ("Name", "Name")]);
    for (m, s) in [("m", "Min"), ("x", "Max")] {
        for st in ["INT", "WIL", "STR", "HEA", "AGL", "CHM"] {
            p.push((format!("{m}{st}"), format!("{s} {st}")));
        }
    }
    p.extend(pairs(&[
        ("HPPerLVL", "HP Bonus"),
        ("ExpTable", "EXP %"),
        ("BaseCP", "CP"),
    ]));
    ability_pairs(&mut p, 10);
    let races = Src::load(realm, "Races", layout)?;
    data.tables
        .push(copy_table(table("Races"), &races, &p, |_, _, _| {}));

    let mut p = pairs(&[
        ("Number", "Number"),
        ("Name", "Name"),
        ("Short", "Short Name"),
        ("ReqLevel", "Level"),
        ("EnergyCost", "Energy"),
        ("ManaCost", "Mana"),
        ("MinBase", "Min"),
        ("MaxBase", "Max"),
        ("Diff", "Difficulty"),
        ("TypeOfResists", "Type of Resists"),
        ("Targets", "Target"),
        ("Dur", "Duration"),
        ("AttType", "Attack Type"),
        ("Magery", "Magery A"),
        ("MageryLVL", "Magery B"),
        ("Cap", "Level Cap"),
        ("MaxIncLVLs", "LVLS Max Increase"),
        ("MaxInc", "Max Increase"),
        ("MinIncLVLs", "LVLS Min Increase"),
        ("MinInc", "Min Increase"),
        ("DurIncLVLs", "LVLS Dur Increase"),
        ("DurInc", "Dur Increase"),
    ]);
    ability_pairs(&mut p, 10);
    let spells = Src::load(realm, "Spells", layout)?;
    let spell_name: HashMap<i64, String> = {
        let (n, name) = (spells.at("Number"), spells.at("Name"));
        spells
            .rows
            .rows
            .iter()
            .map(|r| (num(&r[n]), clip(&r[name])))
            .collect()
    };
    data.tables
        .push(copy_table(table("Spells"), &spells, &p, |_, _, _| {}));

    data.tables.push(monster_table(
        table("Monsters"),
        &monsters,
        &message,
        &spell_name,
    ));

    let mut p = pairs(&[
        ("Number", "Number"),
        ("Name", "Name"),
        ("Limit", "Game Limit"),
        ("Encum", "Weight"),
        ("ItemType", "Type"),
        ("UseCount", "Uses"),
        ("Price", "Cost"),
        ("Currency", "Cost Type"),
        ("Min", "Min Hit"),
        ("Max", "Max Hit"),
        ("ArmourClass", "AC"),
        ("DamageResist", "DR"),
        ("WeaponType", "Weapon"),
        ("ArmourType", "Armour"),
        ("Worn", "Worn On"),
        ("Accy", "Accuracy"),
        ("Gettable", "Gettable"),
        ("StrReq", "Req Str"),
        ("Speed", "Speed"),
        ("Not Droppable", "Not Droppable"),
        ("Destroy On Death", "Destroy On Death"),
        ("Retain After Uses", "Retain After Uses"),
    ]);
    for x in 0..10 {
        p.push((format!("ClassRest-{x}"), format!("Class {x}")));
        p.push((format!("RaceRest-{x}"), format!("Race {x}")));
        // NMR's Negate columns are already its every other element
        p.push((format!("NegateSpell-{x}"), format!("Negate {x}")));
    }
    ability_pairs(&mut p, 20);
    let items = Src::load(realm, "Items", layout)?;
    data.tables
        .push(copy_table(table("Items"), &items, &p, |_, _, _| {}));

    let p = pairs(&[
        ("Number", "Number"),
        ("Name", "Name"),
        ("ShopType", "Type"),
        ("MinLVL", "Min Lvl"),
        ("MaxLVL", "Max Lvl"),
        ("Markup%", "MarkUp"),
        ("ClassRest", "Class Limit"),
    ]);
    let stock: Vec<[String; 5]> = (0..20)
        .map(|x| {
            [
                format!("Item {x}"),
                format!("Max {x}"),
                format!("Regen Time {x}"),
                format!("Regen Number{x}"),
                format!("Regen %{x}"),
            ]
        })
        .collect();
    let shops = Src::load(realm, "Shops", layout)?;
    let ty = shops.at("Type");
    let stock_at: Vec<[usize; 5]> = stock
        .iter()
        .map(|cs| [0, 1, 2, 3, 4].map(|i| shops.at(&cs[i])))
        .collect();
    data.tables
        .push(copy_table(table("Shops"), &shops, &p, |out, row, r| {
            // a gang shop's arrays hold its owner, not stock: NMR writes zeros
            let gang = num(&r[ty]) == 11;
            for (x, at) in stock_at.iter().enumerate() {
                for (c, &i) in ["Item", "Max", "Time", "Amount", "%"].iter().zip(at) {
                    let v = if gang { long(0) } else { long(num(&r[i])) };
                    out.set(row, &format!("{c}-{x}"), v);
                }
            }
        }));

    let info = table("Info");
    let legit = if o.legit { 1 } else { 0 };
    let (custom, url) = if o.legit {
        (
            "Default".to_string(),
            "http://www.mudinfo.net/mmudexp.php".to_string(),
        )
    } else {
        (o.custom.clone(), o.update_url.clone())
    };
    data.tables.push(NmrRows {
        name: "Info".into(),
        columns: info.columns.iter().map(|(c, _)| c.clone()).collect(),
        rows: vec![vec![
            text(crate::export::NMR_VERSION),
            text(o.dat_version.clone()),
            text(o.date.clone()),
            text(o.time.clone()),
            text(custom),
            long(legit),
            long(i64::from(o.disable_kai)),
            text(url),
        ]],
    });
    data.tables.push(NmrRows {
        name: "TBInfo".into(),
        columns: table("TBInfo")
            .columns
            .iter()
            .map(|(c, _)| c.clone())
            .collect(),
        rows: Vec::new(),
    });

    let rooms = Src::load(realm, "Rooms", layout)?;
    let textblocks = Src::load(realm, "Textblocks", layout)?;
    let world = walk::World::read(&rooms, &shops, &monsters, &spells, &items, &textblocks);
    let mut lairs = walk::locate(&mut data, &world, &excl, o.hide_excluded);
    lairs.script_values(named(&mut data, "Monsters"));
    lairs.export(named(&mut data, "Lairs"), &world.lair_rooms());
    Ok(data)
}

fn named<'a>(data: &'a mut NmrData, name: &str) -> &'a mut NmrRows {
    data.tables
        .iter_mut()
        .find(|t| t.name == name)
        .expect("an MME table")
}

/// `GetMessages(n, which)`: one line, or (`which` 0) the lines that have text, joined.
fn message_text(messages: &HashMap<i64, [String; 3]>, n: i64, which: usize) -> String {
    if n == 0 {
        return "none".into();
    }
    let Some(lines) = messages.get(&n) else {
        return "MSG NOT IN DATABASE!".into();
    };
    if which == 0 {
        lines
            .iter()
            .filter(|l| !l.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        lines[which - 1].clone()
    }
}

/// `GetMonsterAttackName`: a physical attack's verb from its hit message, a spell's name.
fn attack_name(
    ty: i64,
    acc_spell: i64,
    hit_msg: i64,
    messages: &HashMap<i64, [String; 3]>,
    spells: &HashMap<i64, String>,
) -> String {
    let name = match ty {
        0 => "None".to_string(),
        1 => {
            let s = message_text(messages, hit_msg, 1);
            // VB's InStr, 1-based, case-insensitive
            let find = |from: usize, pat: &str| -> usize {
                let lower = s.to_ascii_lowercase();
                lower
                    .get(from.saturating_sub(1)..)
                    .and_then(|t| t.find(pat))
                    .map_or(0, |i| i + from.max(1))
            };
            let y = find(1, "%s ") + 3;
            let mut z = find(y, " for ");
            if z == 0 {
                z = find(y, " ");
            }
            if y > 3 && z > y {
                let word = &s[y - 1..z - 1];
                let after = find(y + "all-out".len() + 1, " ");
                if word == "all-out" && after > 0 {
                    s[y - 1..after - 1].to_string()
                } else {
                    word.to_string()
                }
            } else {
                "Physical".to_string()
            }
        }
        2 => match acc_spell {
            0 => "none".to_string(),
            n => spells
                .get(&n)
                .cloned()
                .unwrap_or_else(|| format!("Spell {n}")),
        },
        // NMR tests for 1 again where it means 3 (rob), so a rob is "Unknown"
        _ => "Unknown".to_string(),
    };
    if name.chars().count() > 49 {
        format!("{}...", name.chars().take(46).collect::<String>())
    } else {
        name
    }
}

fn monster_table(
    t: &MmeTable,
    src: &Src,
    messages: &HashMap<i64, [String; 3]>,
    spells: &HashMap<i64, String>,
) -> NmrRows {
    let mut p = pairs(&[
        ("Number", "Number"),
        ("Name", "Name"),
        ("Weapon", "Weapon Number"),
        ("ArmourClass", "AC"),
        ("DamageResist", "DR"),
        ("Follow%", "Follow"),
        ("MagicRes", "MR"),
        ("BSDefense", "BS Defense"),
        ("EXP", "Experience"),
        ("ExpMulti", "Exp Multiplier"),
        ("HP", "Hit Points"),
        ("Energy", "Energy"),
        ("GreetTXT", "Greet Txt"),
        ("HPRegen", "HP Regen"),
        ("CharmLVL", "Charm LvL"),
        ("Type", "Type"),
        ("Undead", "Undead"),
        ("Align", "Alignment"),
        ("RegenTime", "Regen Time"),
        ("GameLimit", "Game Limit"),
        ("R", "Runic"),
        ("P", "Platinum"),
        ("G", "Gold"),
        ("S", "Silver"),
        ("C", "Copper"),
        ("DeathSpell", "Death Spell"),
        ("CreateSpell", "Create Spell"),
    ]);
    for x in 0..5 {
        for (m, n) in [
            ("AttType", "Attack Type"),
            ("AttAcc", "Attack Accu/Spell"),
            ("Att%", "Attack %"),
            ("AttMin", "Attack Min Hit/Cast %"),
            ("AttMax", "Attack Max Hit/Cast LVL"),
            ("AttEnergy", "Attack Energy"),
            ("AttHitSpell", "Attack Hit Spell"),
            ("MidSpell", "Spell Number"),
            ("MidSpell%", "Spell Cast %"),
            ("MidSpellLVL", "Spell Cast LVL"),
        ] {
            p.push((format!("{m}-{x}"), format!("{n} {x}")));
        }
    }
    for x in 0..10 {
        p.push((format!("DropItem-{x}"), format!("Item Number {x}")));
        p.push((format!("DropItem%-{x}"), format!("Item Drop % {x}")));
    }
    ability_pairs(&mut p, 10);
    let att: Vec<[usize; 3]> = (0..5)
        .map(|x| {
            [
                src.at(&format!("Attack Type {x}")),
                src.at(&format!("Attack Accu/Spell {x}")),
                src.at(&format!("Attack Hit Msg {x}")),
            ]
        })
        .collect();
    copy_table(t, src, &p, |out, row, r| {
        for (x, [ty, acc, hit]) in att.iter().enumerate() {
            let name = attack_name(num(&r[*ty]), num(&r[*acc]), num(&r[*hit]), messages, spells);
            out.set(row, &format!("AttName-{x}"), text(name));
        }
    })
}

/// `GetAlignmentValue`.
fn alignment(evil: i64) -> String {
    match evil {
        ..=-201 => "Saint",
        -200..=-51 => "Good",
        -50..=29 => "Neutral",
        30..=39 => "Seedy",
        40..=79 => "Outlaw",
        80..=119 => "Criminal",
        120..=209 => "Villain",
        _ => "Fiend",
    }
    .to_string()
}

/// One room's exits as NMR reads them.
struct RoomExits {
    to: [i64; 10],
    ty: [i64; 10],
    para: [[i64; 4]; 10],
}

#[allow(clippy::too_many_arguments)]
fn rooms(
    realm: &Realm,
    layout: LayoutSet,
    t: &MmeTable,
    o: &MmeOptions,
    excl: &Exclusions,
    messages: &HashMap<i64, [String; 3]>,
    monsters: &Src,
) -> Result<NmrRows, ExportError> {
    let mut out = Out::new(t);
    if !o.legit && o.exclude_rooms && o.no_rooms {
        return Ok(out.rows);
    }
    let src = Src::load(realm, "Rooms", layout)?;
    let c = |name: &str| src.at(name);
    let (map, room, name, ty, shop, light, spell, npc, cmd, delay) = (
        c("Map Number"),
        c("Room Number"),
        c("Name"),
        c("Type"),
        c("Shop Number"),
        c("Light"),
        c("Spell"),
        c("Perm NPC"),
        c("Command Text"),
        c("Delay"),
    );
    let (group, min, max, regen) = (
        c("Mon Type"),
        c("Min Index"),
        c("Max Index"),
        c("Max Regen"),
    );
    let placed: Vec<usize> = (0..10).map(|x| c(&format!("Placed Item {x}"))).collect();
    let exit_cols: Vec<[usize; 6]> = (0..10)
        .map(|x| {
            [
                c(&format!("Exit {x}")),
                c(&format!("Type {x}")),
                c(&format!("Para1 {x}")),
                c(&format!("Para2 {x}")),
                c(&format!("Para3 {x}")),
                c(&format!("Para4 {x}")),
            ]
        })
        .collect();
    let exits_of = |r: &[Value]| {
        let mut e = RoomExits {
            to: [0; 10],
            ty: [0; 10],
            para: [[0; 4]; 10],
        };
        for (x, cs) in exit_cols.iter().enumerate() {
            e.to[x] = num(&r[cs[0]]);
            e.ty[x] = num(&r[cs[1]]);
            for k in 0..4 {
                e.para[x][k] = num(&r[cs[2 + k]]);
            }
        }
        e
    };
    let by_key: HashMap<(i64, i64), usize> = src
        .rows
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| ((num(&r[map]), num(&r[room])), i))
        .collect();
    let mgil = monster_group_index_list(monsters);

    for r in &src.rows.rows {
        let (m, n) = (num(&r[map]), num(&r[room]));
        let room_name = clip(&r[name]);
        if room_name
            .get(..11)
            .is_some_and(|p| p.eq_ignore_ascii_case("buffer room"))
        {
            continue;
        }
        if excl.excludes(m, n, o.hide_excluded) {
            continue;
        }
        let mut row = out.blank(t);
        out.set(&mut row, "Map Number", long(m));
        out.set(&mut row, "Room Number", long(n));
        out.set(&mut row, "Name", text(room_name));
        if excl.excludes(m, n, false) {
            // kept but blanked: NMR's "hide excluded rooms"
            out.rows.rows.push(row);
            continue;
        }
        let s = num(&r[shop]);
        out.set(
            &mut row,
            "Shop",
            long(if num(&r[ty]) == 1 && s > 0 { s } else { 0 }),
        );
        out.set(&mut row, "Light", long(num(&r[light])));
        out.set(&mut row, "Spell", long(num(&r[spell])));
        out.set(&mut row, "NPC", long(num(&r[npc])));
        out.set(&mut row, "CMD", long(num(&r[cmd])));
        let d = num(&r[delay]);
        out.set(&mut row, "Delay", long(if d == 0 { 5 } else { d }));
        let mut p: String = placed
            .iter()
            .map(|&i| num(&r[i]))
            .filter(|&x| x > 0)
            .map(|x| format!("{x},"))
            .collect();
        p.push('\0');
        out.set(&mut row, "Placed", text(p));

        let e = exits_of(r);
        for (x, v) in exit_texts(m, n, &e, messages, |room| {
            by_key.get(&(m, room)).map(|&i| exits_of(&src.rows.rows[i]))
        })
        .into_iter()
        .enumerate()
        {
            out.set(&mut row, DIRECTIONS[x], v.map_or(Value::Null, text));
        }

        let (g, lo, hi, most) = (num(&r[group]), num(&r[min]), num(&r[max]), num(&r[regen]));
        let mut mobs = String::new();
        if most > 0 && num(&r[ty]) == 3 && lo <= hi && hi > 0 {
            for i in lo..=hi {
                for &mon in mgil.get(&(g, i)).map(Vec::as_slice).unwrap_or_default() {
                    mobs.push_str(&format!("{mon},"));
                }
            }
        }
        let lair = if mobs.is_empty() {
            "\0".to_string()
        } else {
            format!("(Max {most}): {mobs}[{g}-{lo}-{hi}-{most}]")
        };
        out.set(&mut row, "Lair", text(lair));
        out.rows.rows.push(row);
    }
    Ok(out.rows)
}

/// `CreateMGIL`: the monsters of each group and index, at most 15, in number order, leaving
/// out names that start with "sdf" and negative indexes.
fn monster_group_index_list(src: &Src) -> HashMap<(i64, i64), Vec<i64>> {
    let (n, name, g, i) = (
        src.at("Number"),
        src.at("Name"),
        src.at("Group"),
        src.at("Index"),
    );
    let mut out: HashMap<(i64, i64), Vec<i64>> = HashMap::new();
    for r in &src.rows.rows {
        let index = num(&r[i]);
        if clip(&r[name]).starts_with("sdf") || index < 0 {
            continue;
        }
        let list = out.entry((num(&r[g]), index)).or_default();
        if list.len() < 15 {
            list.push(num(&r[n]));
        }
    }
    out
}

/// The ten exits' text, as `ExportRooms` writes them (`None`: not written, so Null); `room_on_map` finds another room of
/// the same map, for a remote action's count of actions.
fn exit_texts(
    map: i64,
    room: i64,
    e: &RoomExits,
    messages: &HashMap<i64, [String; 3]>,
    room_on_map: impl Fn(i64) -> Option<RoomExits>,
) -> [Option<String>; 10] {
    let mut out: [Option<String>; 10] = Default::default();
    // (target room, text) by exit and action number
    let mut actions: [[(i64, String); 10]; 10] = Default::default();
    let mut new_actions: [String; 10] = Default::default();
    let picklocks = |p: i64| {
        if p < 0 {
            (p - 1).abs().to_string()
        } else {
            "any".to_string()
        }
    };
    for x in 0..10 {
        let to = e.to[x];
        let [p1, p2, p3, p4] = e.para[x];
        if to == 0 {
            out[x] = Some("0".into());
            continue;
        }
        let here = format!("{map}/{to}");
        out[x] = match e.ty[x] {
            0 | 1 | 5 | 19 | 21 => here,
            2 => {
                let mut s = format!("{here} (Key: {p1}");
                if p3 != 0 {
                    s.push_str(&format!(" [or {} picklocks])", picklocks(p3)));
                } else {
                    s.push(')');
                }
                s
            }
            3 => format!("{here} (Item: {p1})"),
            4 => format!("{here} (Toll: {p1})"),
            6 => {
                let what = match p1 {
                    0 => {
                        out[x] = Some("0".into());
                        continue;
                    }
                    1 | 3 => " (Hidden/Passable)".to_string(),
                    2 | 4 => " (Hidden/Searchable)".to_string(),
                    _ if p2 < 0 => format!(" (Hidden/Needs {} Actions, any order)", p2.abs()),
                    _ if p2 > 0 => format!(" (Hidden/Needs {p2} Actions, specific order)"),
                    _ => " (Hidden/Unknown)".to_string(),
                };
                format!("{here}{what}")
            }
            7 | 11 => {
                let mut s = if p4 <= 0 {
                    format!("{here} (Door")
                } else {
                    format!("{here} (Key: {p4}")
                };
                if p2 != 0 {
                    let or = if p4 <= 0 { "" } else { "or " };
                    s.push_str(&format!(" [{or}{} picklocks/strength])", picklocks(p2)));
                } else {
                    s.push(')');
                }
                s
            }
            8 => format!("{p1}/{to}"),
            9 => format!("{here} (Trap, {p1} damage)"),
            10 => format!("{here} (Text: {})", message_text(messages, p1, 0)),
            12 => {
                // VB's Mod keeps the sign: a negative one names no exit
                let dir = if p2 >= 0 {
                    DIRECTIONS[(p2 % 10) as usize]
                } else {
                    ""
                };
                let mut s = if room == to {
                    format!("[on the {dir} exit of this room]: ")
                } else {
                    format!("[on the {dir} exit of room {map}/{to}]: ")
                };
                s.push_str(&message_text(messages, p1, 0));
                if p4 != 0 {
                    s.push_str(&format!(" (Item: {p4})"));
                }
                if p2 > 9 && p2 < 100 {
                    let k = (p2 % 10) as usize;
                    let total = if to != room {
                        // GetTotalActions
                        match room_on_map(to) {
                            None => 0,
                            Some(other) if other.ty[k] == 6 => {
                                let t = other.para[k][1].abs();
                                if t > 9 { -1 } else { t }
                            }
                            Some(_) => -1,
                        }
                    } else if e.ty[k] == 6 {
                        e.para[k][1].abs()
                    } else {
                        -1
                    };
                    if total > 0 && total < 10 {
                        let action = total - p2 / 10 + 1;
                        if (0..10).contains(&action) {
                            let label = format!("Action#{action} {s}");
                            actions[x][action as usize] = (to, label);
                            new_actions[x] = "open".into();
                        }
                        continue;
                    }
                }
                format!("Action {s}")
            }
            13 => format!("{here} (Class: {p1} OK, {p2} NO)"),
            14 => format!("{here} (Race: {p1} OK, {p2} NO)"),
            15 => format!("{here} (Level: {p1} to {p2})"),
            16 => format!("{here} (Timed: {p2}*5 minutes)"),
            17 => format!("{here} (Ticket/Item: {p1})"),
            18 => format!("{here} (Max Users: {p1})"),
            20 => format!("{here} (Alignment: {} to {})", alignment(p1), alignment(p2)),
            22 => format!("{here} (Cast: pre-{p1}, post-{p2})"),
            23 => format!("{here} (Ability: {p1} w/value {p2} to {p3})"),
            24 => format!("{here} (Spell Trap: {p1})"),
            // an exit type NMR does not know leaves the column unset
            _ => {
                out[x] = None;
                continue;
            }
        }
        .into();
    }
    // NMR's ordering of a room's remote actions: for each one, its target's actions in
    // order fill the exits still marked "open", first to last
    for x in 0..10 {
        for y in 0..10 {
            let target = actions[x][y].0;
            if target <= 0 {
                continue;
            }
            for z in 1..10 {
                if let Some(x2) = (0..10).find(|&x2| actions[x2][z].0 == target)
                    && let Some(slot) = new_actions.iter().position(|a| a == "open")
                {
                    new_actions[slot] = actions[x2][z].1.clone();
                }
            }
        }
    }
    for x in 0..10 {
        if !new_actions[x].is_empty() {
            out[x] = Some(new_actions[x].clone());
        }
    }
    out
}
