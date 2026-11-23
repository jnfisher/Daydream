//! WCCITOWN limited-item ownership: the `itemown` record, state. Not
//! carried: `unk_0x33`.

use daydream_types::enums::OwnerKind;
use daydream_types::ids::{ActiveMonsterId, ItemNo, MapId, RoomId};
use daydream_types::records::{ItemOwnership, layout};

use super::{Raw, Text, record};

record! {
    /// The `itemown` record, WCCITOWN: every member, in record order.
    pub struct ItemownRecord = layout::ITEMOWN {
        id: i32,
        itemnum: i32,
        owntype: u8,
        mapnum: i32,
        roomnum: i32,
        userid: Text<30>,
        monnum: i32,
        unk_0x33: Raw<14>,
    }
}

/// The schema's ownership row.
pub fn to_item_ownership(r: &ItemownRecord) -> ItemOwnership {
    ItemOwnership {
        id: r.id,
        itemnum: ItemNo(r.itemnum),
        owntype: OwnerKind::from_code(u16::from(r.owntype)),
        mapnum: MapId(r.mapnum as u32),
        roomnum: RoomId(r.roomnum as u32),
        userid: r.userid.trimmed(),
        monnum: ActiveMonsterId(r.monnum),
    }
}
