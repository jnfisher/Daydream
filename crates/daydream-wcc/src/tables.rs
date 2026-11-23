//! Which WCC file holds which record type, and decoding a record of any of them by name.
//!
//! Daydream supports MajorMUD 1.11p (the DOS build's byte-packed layouts) at launch, so every
//! table here uses [`LayoutSet::Dos`]. A realm's files are named by their stem: `WCCITEMS`
//! is `WCCITEMS.DAT` on a running realm, `WCCITEMS.VIR` in the virgin set and `WCCITEMS.db`
//! once wbtrv32 (or Daydream) has converted it.

use serde_json::{Map, Value};

use crate::decode::{
    self, ActionRecord, ActmonRecord, BankbookRecord, ClassRecord, DecodeError, GangRecord,
    ItemRecord, ItemownRecord, MonsterRecord, MudmsgRecord, PlayerRecord, RaceRecord, Record,
    RoomRecord, ShopRecord, SpellRecord, TextblkRecord,
};
use crate::json::ExportRecord;

/// A record type of the 1.11p data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Table {
    Rooms,
    Items,
    Monsters,
    ActiveMonsters,
    Spells,
    Classes,
    Races,
    Shops,
    Messages,
    TextBlocks,
    Actions,
    Gangs,
    BankBooks,
    ItemOwnership,
    Users,
}

/// One WCC file of a realm: its stem and the record type it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WccFile {
    /// The file name without its extension (`WCCITEMS`).
    pub stem: &'static str,
    /// What its records are.
    pub table: Table,
}

/// Every file Daydream decodes, in the order NMR lists its editors. `WCCMP001` is the
/// stock map; a realm may have more map files (`WCCMP002`, ...), which hold rooms too
/// (see [`table_for_stem`]).
pub const FILES: &[WccFile] = &[
    WccFile {
        stem: "WCCITEMS",
        table: Table::Items,
    },
    WccFile {
        stem: "WCCKNMSR",
        table: Table::Monsters,
    },
    WccFile {
        stem: "WCCSPELS",
        table: Table::Spells,
    },
    WccFile {
        stem: "WCCCLASS",
        table: Table::Classes,
    },
    WccFile {
        stem: "WCCRACE",
        table: Table::Races,
    },
    WccFile {
        stem: "WCCSHOPS",
        table: Table::Shops,
    },
    WccFile {
        stem: "WCCMP001",
        table: Table::Rooms,
    },
    WccFile {
        stem: "WCCMSG",
        table: Table::Messages,
    },
    WccFile {
        stem: "WCCTEXT",
        table: Table::TextBlocks,
    },
    WccFile {
        stem: "WCCACTS",
        table: Table::Actions,
    },
    WccFile {
        stem: "WCCUSERS",
        table: Table::Users,
    },
    WccFile {
        stem: "WCCBANKS",
        table: Table::BankBooks,
    },
    WccFile {
        stem: "WCCGANGS",
        table: Table::Gangs,
    },
    WccFile {
        stem: "WCCACMSR",
        table: Table::ActiveMonsters,
    },
    WccFile {
        stem: "WCCITOWN",
        table: Table::ItemOwnership,
    },
];

/// The record type of the file with this stem (any case), or `None` for a file Daydream
/// keeps but does not decode (`WCCUPDAT`, `WCCVACN`, ...). Map files are `WCCMPnnn`.
pub fn table_for_stem(stem: &str) -> Option<Table> {
    let upper = stem.to_ascii_uppercase();
    if let Some(n) = upper.strip_prefix("WCCMP")
        && n.len() == 3
        && n.bytes().all(|b| b.is_ascii_digit())
    {
        return Some(Table::Rooms);
    }
    FILES.iter().find(|f| f.stem == upper).map(|f| f.table)
}

impl Table {
    /// The name used in URLs and file names (`items`, `text-blocks`, ...).
    pub fn slug(self) -> &'static str {
        match self {
            Self::Rooms => "rooms",
            Self::Items => "items",
            Self::Monsters => "monsters",
            Self::ActiveMonsters => "active-monsters",
            Self::Spells => "spells",
            Self::Classes => "classes",
            Self::Races => "races",
            Self::Shops => "shops",
            Self::Messages => "messages",
            Self::TextBlocks => "text-blocks",
            Self::Actions => "actions",
            Self::Gangs => "gangs",
            Self::BankBooks => "bank-books",
            Self::ItemOwnership => "item-ownership",
            Self::Users => "users",
        }
    }

    /// Whether the file has variable-length records (only `WCCTEXT`).
    pub fn is_variable_length(self) -> bool {
        self == Self::TextBlocks
    }

    /// The 1.11p record length (for `WCCTEXT`, the full 2022-byte buffer).
    pub fn record_len(self) -> usize {
        macro_rules! len {
            ($r:ty) => {
                <$r as Record>::LAYOUT.len
            };
        }
        match self {
            Self::Rooms => len!(RoomRecord),
            Self::Items => len!(ItemRecord),
            Self::Monsters => len!(MonsterRecord),
            Self::ActiveMonsters => len!(ActmonRecord),
            Self::Spells => len!(SpellRecord),
            Self::Classes => len!(ClassRecord),
            Self::Races => len!(RaceRecord),
            Self::Shops => len!(ShopRecord),
            Self::Messages => len!(MudmsgRecord),
            Self::TextBlocks => len!(TextblkRecord),
            Self::Actions => len!(ActionRecord),
            Self::Gangs => len!(GangRecord),
            Self::BankBooks => len!(BankbookRecord),
            Self::ItemOwnership => len!(ItemownRecord),
            Self::Users => len!(PlayerRecord),
        }
    }

    /// Record `id`'s bytes as the JSON object [`crate::json`] writes: `_id`, `_raw_len` and
    /// every member by name, `unk_*` spans included.
    pub fn to_json(self, id: u32, raw: &[u8]) -> Result<Map<String, Value>, DecodeError> {
        macro_rules! json {
            ($r:ty) => {
                <$r>::decode(raw)?.export(id)
            };
        }
        Ok(match self {
            Self::Rooms => json!(RoomRecord),
            Self::Items => json!(ItemRecord),
            Self::Monsters => json!(MonsterRecord),
            Self::ActiveMonsters => json!(ActmonRecord),
            Self::Spells => json!(SpellRecord),
            Self::Classes => json!(ClassRecord),
            Self::Races => json!(RaceRecord),
            Self::Shops => json!(ShopRecord),
            Self::Messages => json!(MudmsgRecord),
            Self::TextBlocks => decode::decode_textblk(raw)?.export(id),
            Self::Actions => json!(ActionRecord),
            Self::Gangs => json!(GangRecord),
            Self::BankBooks => json!(BankbookRecord),
            Self::ItemOwnership => json!(ItemownRecord),
            Self::Users => json!(PlayerRecord),
        })
    }

    /// Decodes `raw` and encodes it again. For a fixed-length table the result must equal
    /// `raw`; for `WCCTEXT` it is `raw` zero-filled to the full record buffer. Used by the
    /// round-trip tests to prove the decoders lose nothing.
    pub fn reencode(self, raw: &[u8]) -> Result<Vec<u8>, DecodeError> {
        macro_rules! re {
            ($r:ty) => {
                <$r>::decode(raw)?.encode()
            };
        }
        Ok(match self {
            Self::Rooms => re!(RoomRecord),
            Self::Items => re!(ItemRecord),
            Self::Monsters => re!(MonsterRecord),
            Self::ActiveMonsters => re!(ActmonRecord),
            Self::Spells => re!(SpellRecord),
            Self::Classes => re!(ClassRecord),
            Self::Races => re!(RaceRecord),
            Self::Shops => re!(ShopRecord),
            Self::Messages => re!(MudmsgRecord),
            Self::TextBlocks => decode::decode_textblk(raw)?.encode(),
            Self::Actions => re!(ActionRecord),
            Self::Gangs => re!(GangRecord),
            Self::BankBooks => re!(BankbookRecord),
            Self::ItemOwnership => re!(ItemownRecord),
            Self::Users => re!(PlayerRecord),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_files_are_rooms() {
        assert_eq!(table_for_stem("wccmp001"), Some(Table::Rooms));
        assert_eq!(table_for_stem("WCCMP017"), Some(Table::Rooms));
        assert_eq!(table_for_stem("WCCMPX"), None);
        assert_eq!(table_for_stem("WCCUPDAT"), None);
        assert_eq!(table_for_stem("wccitems"), Some(Table::Items));
    }

    #[test]
    fn every_table_has_one_file_and_a_unique_slug() {
        let mut slugs: Vec<_> = FILES.iter().map(|f| f.table.slug()).collect();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), FILES.len());
    }
}
