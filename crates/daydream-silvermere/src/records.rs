//! silvermere's rows back onto a realm's records.
//!
//! silvermere's schema holds the members of each record that the game uses, normalised
//! (`daydream-types`' record structs, as `daydream-wcc`'s `to_*` functions make them). It
//! leaves out the `unk_*` spans, cache bytes, the bytes after a text's NUL and slots it
//! stores as absent, so a record cannot be rebuilt from its row alone. These functions
//! instead change a *base* record (the bytes the row came from, when known) where the row
//! differs from what the base reads as (`old`): a member whose value is unchanged keeps its
//! base bytes exactly, and an unchanged row gives back its base record byte for byte.

use daydream_types::records::{
    AbilitySlot, Action, Class, Item, KnownMonster, Message, Race, Room, Shop, Spell, TextBlock,
};
use daydream_wcc::decode::textblk::TEXT_SHIFT;
use daydream_wcc::decode::{
    ActionRecord, ClassRecord, ItemRecord, MonsterRecord, MudmsgRecord, RaceRecord, RoomRecord,
    ShopRecord, SpellRecord, Text, TextblkRecord,
};
use jiff::tz::TimeZone;

/// `dst = conv(new)` when `new` differs from `old`.
macro_rules! put {
    ($dst:expr, $new:expr, $old:expr) => {
        if $new != $old {
            $dst = $new;
        }
    };
    ($dst:expr, $new:expr, $old:expr, |$v:ident| $conv:expr) => {
        if $new != $old {
            let $v = $new;
            $dst = $conv;
        }
    };
}

/// A text member: rewritten cleanly (text, NUL, zeros) only when its value changed.
fn text<const N: usize>(dst: &mut Text<N>, new: &str, old: &str) {
    if new != old {
        *dst = Text::new(new);
    }
}

fn texts<const N: usize, const M: usize>(dst: &mut [Text<N>; M], new: &[String], old: &[String]) {
    for i in 0..M {
        text(&mut dst[i], &new[i], &old[i]);
    }
}

fn abilities<const N: usize>(
    codes: &mut [i16; N],
    values: &mut [i16; N],
    new: &[AbilitySlot; N],
    old: &[AbilitySlot; N],
) {
    for i in 0..N {
        put!(codes[i], new[i].0, old[i].0, |a| a.code() as i16);
        put!(values[i], new[i].1, old[i].1);
    }
}

/// Unix seconds UTC (0 = never) as a DOS date and time in `tz`, the inverse of
/// `daydream_wcc::dos_time::dos_to_unix`. Odd seconds round down, as DOS keeps two-second
/// steps; a moment outside 1980-2107 is never.
pub fn unix_to_dos(at: i64, tz: &TimeZone) -> (u16, u16) {
    if at == 0 {
        return (0, 0);
    }
    let Ok(ts) = jiff::Timestamp::from_second(at) else {
        return (0, 0);
    };
    let dt = ts.to_zoned(tz.clone()).datetime();
    let year = dt.year() - 1980;
    if !(0..128).contains(&year) {
        return (0, 0);
    }
    let date = (year as u16) << 9 | (dt.month() as u16) << 5 | dt.day() as u16;
    let time = (dt.hour() as u16) << 11 | (dt.minute() as u16) << 5 | ((dt.second() as u16) / 2);
    (date, time)
}

pub fn apply_room(r: &mut RoomRecord, new: &Room, old: &Room) {
    put!(r.mapnum, new.key.map.0, old.key.map.0);
    put!(r.roomnum, new.key.room.0, old.key.room.0);
    text(&mut r.name, &new.name, &old.name);
    texts(&mut r.desc, &new.desc, &old.desc);
    text(&mut r.mapfile, &new.mapfile, &old.mapfile);
    for d in 0..new.exits.len() {
        let (n, o) = (&new.exits[d], &old.exits[d]);
        put!(r.exits[d], n.target.0, o.target.0);
        put!(r.exittypes[d], n.kind, o.kind, |k| k.code() as i16);
        put!(r.exitpar1s[d], n.par1, o.par1);
        put!(r.exitpar2s[d], n.par2, o.par2);
        put!(r.exitpar3s[d], n.par3, o.par3);
        put!(r.exitpar4s[d], n.par4, o.par4);
    }
    for i in 0..new.monsters.len() {
        put!(r.monsters[i], new.monsters[i].0, old.monsters[i].0, |m| m
            as u32);
    }
    put!(r.type_, new.room_type, old.room_type, |t| t.code() as i16);
    put!(r.shopnum, new.shopnum.0, old.shopnum.0);
    text(&mut r.owner, &new.owner, &old.owner);
    put!(r.minlevel, new.minlevel, old.minlevel);
    put!(r.maxlevel, new.maxlevel, old.maxlevel);
    put!(r.genmonster, new.genmonster.0, old.genmonster.0);
    put!(r.light, new.light, old.light);
    put!(r.ganghouse, new.ganghouse, old.ganghouse, |g| g.code());
    for i in 0..new.items.len() {
        let (n, o) = (&new.items[i], &old.items[i]);
        put!(r.items[i], n.item.0, o.item.0);
        put!(r.itemuses[i], n.uses, o.uses);
        put!(r.itemcounts[i], n.count, o.count);
    }
    for i in 0..new.hiditems.len() {
        let (n, o) = (&new.hiditems[i], &old.hiditems[i]);
        put!(r.hiditems[i], n.item.0, o.item.0);
        put!(r.hiduses[i], n.uses, o.uses);
        put!(r.hidcounts[i], n.count, o.count);
    }
    put!(r.runic, new.coins.runic, old.coins.runic);
    put!(r.platinum, new.coins.platinum, old.coins.platinum);
    put!(r.gold, new.coins.gold, old.coins.gold);
    put!(r.silver, new.coins.silver, old.coins.silver);
    put!(r.copper, new.coins.copper, old.coins.copper);
    put!(r.hidrunic, new.hidcoins.runic, old.hidcoins.runic);
    put!(r.hidplatinum, new.hidcoins.platinum, old.hidcoins.platinum);
    put!(r.hidgold, new.hidcoins.gold, old.hidcoins.gold);
    put!(r.hidsilver, new.hidcoins.silver, old.hidcoins.silver);
    put!(r.hidcopper, new.hidcoins.copper, old.hidcoins.copper);
    put!(r.maxmonsters, new.maxmonsters, old.maxmonsters);
    put!(r.mongroup, new.mongroup, old.mongroup);
    put!(r.lastkill, new.lastkill, old.lastkill);
    put!(r.flags, new.flags, old.flags, |f| f.bits());
    put!(r.foundmask, new.foundmask, old.foundmask);
    put!(r.deathroom, new.deathroom.0, old.deathroom.0);
    put!(r.huproom, new.huproom.0, old.huproom.0);
    put!(r.cmdtext, new.cmdtext.0, old.cmdtext.0);
    put!(r.regendelay, new.regendelay, old.regendelay);
    put!(r.areamax, new.areamax, old.areamax);
    put!(r.areacount, new.areacount, old.areacount);
    put!(r.ctlroom, new.ctlroom.0, old.ctlroom.0);
    put!(r.specmon, new.specmon.0, old.specmon.0);
    for i in 0..new.placed.len() {
        put!(r.placed[i], new.placed[i].0, old.placed[i].0);
    }
    put!(r.protmsg, new.protmsg.0, old.protmsg.0);
    put!(r.picktrap, new.picktrap, old.picktrap);
    put!(r.roomspell, new.roomspell, old.roomspell);
    put!(r.searchers, new.searchers, old.searchers);
    put!(r.nummonsters, new.nummonsters, old.nummonsters);
}

pub fn apply_item(r: &mut ItemRecord, new: &Item, old: &Item) {
    put!(r.number, new.number.0, old.number.0);
    put!(r.level, new.level, old.level);
    put!(r.limit, new.limit, old.limit);
    text(&mut r.name, &new.name, &old.name);
    texts(&mut r.desc, &new.desc, &old.desc);
    texts(&mut r.hiddesc, &new.hiddesc, &old.hiddesc);
    put!(r.active, new.active, old.active);
    put!(r.weight, new.weight, old.weight);
    put!(r.type_, new.item_type, old.item_type, |t| t.code() as i16);
    abilities(
        &mut r.abilities,
        &mut r.abilvals,
        &new.abilities,
        &old.abilities,
    );
    put!(r.charges, new.charges, old.charges);
    put!(r.energy, new.energy, old.energy);
    put!(r.cost, new.cost, old.cost);
    for i in 0..new.classes.len() {
        put!(r.classes[i], new.classes[i].0, old.classes[i].0, |c| c
            as i16);
    }
    put!(r.mindmg, new.mindmg, old.mindmg);
    put!(r.maxdmg, new.maxdmg, old.maxdmg);
    put!(r.ac, new.ac, old.ac);
    for i in 0..new.races.len() {
        put!(r.races[i], new.races[i].0, old.races[i].0, |c| i32::from(c));
    }
    for i in 0..new.negates.len() {
        put!(r.negates[i], new.negates[i].0, old.negates[i].0, |s| {
            i32::from(s)
        });
    }
    put!(r.wpntype, new.wpntype, old.wpntype, |t| t.code() as i16);
    put!(r.armtype, new.armtype, old.armtype, |t| t.code() as i16);
    put!(r.worn, new.worn, old.worn, |w| w.code() as u8);
    put!(r.accuracy, new.accuracy, old.accuracy);
    put!(r.dr, new.dr, old.dr);
    put!(r.gettable, new.gettable, old.gettable);
    put!(r.strreq, new.strreq, old.strreq);
    let (n, o) = (new.cash.to_runic_first(), old.cash.to_runic_first());
    for i in 0..5 {
        put!(r.cash[i], n[i], o[i], |c| c as i32);
    }
    put!(r.speed, new.speed, old.speed);
    put!(r.hitmsg, new.hitmsg.0, old.hitmsg.0);
    put!(r.missmsg, new.missmsg.0, old.missmsg.0);
    put!(r.textnum, new.textnum.0, old.textnum.0);
    put!(r.destroymsg, new.destroymsg.0, old.destroymsg.0);
    put!(r.nodrop, new.nodrop, old.nodrop);
    put!(r.currency, new.currency, old.currency, |c| c.code() as u8);
    put!(r.retain, new.retain, old.retain);
    put!(r.robable, new.robable, old.robable);
    put!(r.destroy, new.destroy, old.destroy);
}

pub fn apply_monster(r: &mut MonsterRecord, new: &KnownMonster, old: &KnownMonster, tz: &TimeZone) {
    put!(r.number, new.number.0, old.number.0);
    text(&mut r.name, &new.name, &old.name);
    put!(r.montype, new.montype, old.montype, |t| t.code() as i16);
    put!(r.level, new.level, old.level);
    put!(r.genindex, new.genindex, old.genindex);
    put!(r.wornitem, new.wornitem.0, old.wornitem.0);
    put!(r.weapon, new.weapon.0, old.weapon.0);
    put!(r.dr, new.dr, old.dr);
    put!(r.ac, new.ac, old.ac);
    put!(r.group, new.group, old.group);
    put!(r.followpct, new.followpct, old.followpct);
    put!(r.magicres, new.magicres, old.magicres);
    put!(r.bsdefense, new.bsdefense, old.bsdefense);
    put!(r.exp, new.exp, old.exp);
    put!(r.maxhp, new.maxhp, old.maxhp);
    put!(r.energy, new.energy, old.energy);
    put!(r.hpregen, new.hpregen, old.hpregen);
    abilities(
        &mut r.abilities,
        &mut r.abilvals,
        &new.abilities,
        &old.abilities,
    );
    put!(r.maxalive, new.maxalive, old.maxalive);
    put!(r.curalive, new.curalive, old.curalive);
    put!(r.grouprole, new.grouprole, old.grouprole, |g| g.code()
        as i16);
    put!(r.maxfollow, new.maxfollow, old.maxfollow);
    put!(r.undead, new.undead, old.undead);
    put!(r.alignment, new.alignment, old.alignment, |a| a.code()
        as i16);
    put!(r.regenhrs, new.regenhrs, old.regenhrs);
    if new.killed_at != old.killed_at {
        (r.killdate, r.killtime) = unix_to_dos(new.killed_at, tz);
    }
    put!(r.arrivemsg, new.arrivemsg.0, old.arrivemsg.0);
    put!(r.deathmsg, new.deathmsg.0, old.deathmsg.0);
    for i in 0..new.items.len() {
        let (n, o) = (&new.items[i], &old.items[i]);
        put!(r.items[i], n.item.0, o.item.0);
        put!(r.itemuses[i], n.uses, o.uses);
        put!(r.itempcts[i], n.pct, o.pct);
    }
    let (n, o) = (new.cash.to_runic_first(), old.cash.to_runic_first());
    for i in 0..5 {
        put!(r.cash[i], n[i], o[i]);
    }
    put!(r.talktext, new.talktext.0, old.talktext.0);
    put!(r.charmlvl, new.charmlvl, old.charmlvl);
    put!(r.namelist, new.namelist, old.namelist);
    for i in 0..new.attacks.len() {
        let (n, o) = (&new.attacks[i], &old.attacks[i]);
        put!(r.atktypes[i], n.kind, o.kind, |k| k.code() as u8);
        put!(r.atkaccs[i], n.accuracy, o.accuracy);
        put!(r.atkpcts[i], n.pct, o.pct);
        put!(r.atkmins[i], n.min, o.min);
        put!(r.atkmaxs[i], n.max, o.max);
        put!(r.hitmsgs[i], n.hitmsg.0, o.hitmsg.0);
        put!(r.defmsgs[i], n.defmsg.0, o.defmsg.0);
        put!(r.missmsgs[i], n.missmsg.0, o.missmsg.0);
        put!(r.atkenergies[i], n.energy, o.energy);
        put!(r.hitspells[i], n.hitspell.0, o.hitspell.0);
    }
    put!(r.randtext, new.randtext.0, old.randtext.0);
    put!(r.charmres, new.charmres, old.charmres);
    put!(r.deathspell, new.deathspell.0, old.deathspell.0);
    put!(r.createspell, new.createspell.0, old.createspell.0);
    for i in 0..new.spells.len() {
        let (n, o) = (&new.spells[i], &old.spells[i]);
        put!(r.spells[i], n.spell.0, o.spell.0);
        put!(r.spellpcts[i], n.pct, o.pct);
        put!(r.spelllvls[i], n.level, o.level);
    }
    texts(&mut r.desc, &new.desc, &old.desc);
    put!(r.gender, new.gender, old.gender, |g| g.code() as u8);
}

pub fn apply_spell(r: &mut SpellRecord, new: &Spell, old: &Spell) {
    put!(r.number, new.number.0, old.number.0);
    text(&mut r.name, &new.name, &old.name);
    texts(&mut r.descs, &new.descs, &old.descs);
    put!(r.resist, new.resist, old.resist);
    put!(r.levelcap, new.levelcap, old.levelcap);
    put!(r.flags, new.flags, old.flags, |f| f.bits());
    abilities(
        &mut r.abilities,
        &mut r.abilvals,
        &new.abilities,
        &old.abilities,
    );
    put!(r.energy, new.energy, old.energy);
    put!(r.level, new.level, old.level);
    put!(r.minval, new.minval, old.minval);
    put!(r.maxval, new.maxval, old.maxval);
    put!(r.type_, new.spell_type, old.spell_type, |t| t.code() as i16);
    put!(r.resmode, new.resmode, old.resmode, |m| m.code() as u8);
    put!(r.diff, new.diff, old.diff);
    put!(r.durlvl, new.durlvl, old.durlvl);
    put!(r.target, new.target, old.target, |t| t.code() as i16);
    put!(r.duration, new.duration, old.duration);
    put!(r.element, new.element, old.element, |e| e.code() as i16);
    put!(r.resstat, new.resstat, old.resstat, |s| s.code() as u8);
    put!(r.magery, new.magery, old.magery, |m| m.code() as i16);
    put!(r.msgnum, new.msgnum.0, old.msgnum.0);
    put!(r.mana, new.mana, old.mana);
    put!(r.maxinc, new.maxinc, old.maxinc);
    put!(r.maxlvls, new.maxlvls, old.maxlvls);
    put!(r.magelvl, new.magelvl, old.magelvl);
    put!(r.mininc, new.mininc, old.mininc);
    put!(r.minlvls, new.minlvls, old.minlvls);
    put!(r.durinc, new.durinc, old.durinc);
    put!(r.durlvls, new.durlvls, old.durlvls);
    text(&mut r.shortnm, &new.shortnm, &old.shortnm);
}

pub fn apply_class(r: &mut ClassRecord, new: &Class, old: &Class) {
    put!(r.number, new.number.0, old.number.0);
    text(&mut r.name, &new.name, &old.name);
    put!(r.basehp, new.basehp, old.basehp);
    put!(r.hplevel, new.hplevel, old.hplevel);
    put!(r.exp, new.exp, old.exp);
    put!(r.attmult, new.attmult, old.attmult);
    put!(r.maxenergy, new.maxenergy, old.maxenergy);
    put!(r.restored, new.restored, old.restored);
    abilities(
        &mut r.abilities,
        &mut r.abilvals,
        &new.abilities,
        &old.abilities,
    );
    put!(r.magery, new.magery, old.magery, |m| m.code() as i16);
    put!(r.magelvl, new.magelvl, old.magelvl);
    put!(r.weapon, new.weapon, old.weapon, |w| w.code() as i16);
    put!(r.armour, new.armour, old.armour, |a| a.code() as i16);
    put!(r.combat, new.combat, old.combat);
    put!(r.titles, new.titles.0, old.titles.0);
}

pub fn apply_race(r: &mut RaceRecord, new: &Race, old: &Race) {
    put!(r.number, new.number.0, old.number.0);
    text(&mut r.name, &new.name, &old.name);
    put!(r.minint, new.minint, old.minint);
    put!(r.minwil, new.minwil, old.minwil);
    put!(r.minstr, new.minstr, old.minstr);
    put!(r.minhea, new.minhea, old.minhea);
    put!(r.minagi, new.minagi, old.minagi);
    put!(r.mincha, new.mincha, old.mincha);
    put!(r.hpbonus, new.hpbonus, old.hpbonus);
    put!(r.maxenergy, new.maxenergy, old.maxenergy);
    put!(r.restored, new.restored, old.restored);
    abilities(
        &mut r.abilities,
        &mut r.abilvals,
        &new.abilities,
        &old.abilities,
    );
    put!(r.basecp, new.basecp, old.basecp);
    put!(r.exp, new.exp, old.exp);
    put!(r.maxint, new.maxint, old.maxint);
    put!(r.maxwil, new.maxwil, old.maxwil);
    put!(r.maxstr, new.maxstr, old.maxstr);
    put!(r.maxhea, new.maxhea, old.maxhea);
    put!(r.maxagi, new.maxagi, old.maxagi);
    put!(r.maxcha, new.maxcha, old.maxcha);
}

pub fn apply_shop(r: &mut ShopRecord, new: &Shop, old: &Shop) {
    put!(r.number, new.number.0, old.number.0);
    text(&mut r.name, &new.name, &old.name);
    texts(&mut r.desc, &new.desc, &old.desc);
    put!(r.type_, new.shop_type, old.shop_type, |t| t.code() as i16);
    put!(r.minlevel, new.minlevel, old.minlevel);
    put!(r.maxlevel, new.maxlevel, old.maxlevel);
    put!(r.markup, new.markup, old.markup);
    put!(r.ownable, new.ownable, old.ownable);
    put!(r.expandable, new.expandable, old.expandable);
    put!(r.trainclass, new.trainclass.0, old.trainclass.0);
    for i in 0..new.stock.len() {
        let (n, o) = (&new.stock[i], &old.stock[i]);
        put!(r.items[i], n.item.0, o.item.0);
        put!(r.maxqty[i], n.maxqty, o.maxqty);
        put!(r.qty[i], n.qty, o.qty);
        put!(r.regentime[i], n.regentime, o.regentime);
        put!(r.regennum[i], n.regennum, o.regennum);
        put!(r.regenpct[i], n.regenpct, o.regenpct);
    }
}

pub fn apply_message(r: &mut MudmsgRecord, new: &Message, old: &Message) {
    put!(r.number, new.number.0, old.number.0);
    texts(&mut r.msgs, &new.msgs, &old.msgs);
}

pub fn apply_action(r: &mut ActionRecord, new: &Action, old: &Action) {
    text(&mut r.name, &new.name, &old.name);
    texts(&mut r.texts, &new.texts, &old.texts);
    put!(r.targets, new.targets, old.targets, |t| t.bits() as i16);
}

/// A text block part. The stored text is variable length: when the text changed, the
/// record is cut to the fixed part, the shifted text and its NUL (`fixed` is the fixed
/// part's length in the realm's layout); otherwise the base's length is kept. Returns the
/// stored length.
pub fn apply_text_block(
    r: &mut TextblkRecord,
    new: &TextBlock,
    old: &TextBlock,
    fixed: usize,
    base_len: usize,
) -> usize {
    put!(r.part, new.part, old.part);
    put!(r.number, new.number.0, old.number.0);
    put!(r.link, new.link.0, old.link.0);
    if new.text == old.text {
        return base_len;
    }
    let stored = &mut r.text.0;
    stored.fill(0);
    let limit = stored.len() - 1;
    let mut n = 0;
    for ch in new.text.chars().take(limit) {
        // a decoded NUL would end the text: the game's rule (see `decode_text`)
        let b = daydream_types::fixed::char_to_cp437(ch);
        stored[n] = b.wrapping_add(TEXT_SHIFT);
        n += 1;
    }
    fixed + n + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_times_round_trip() {
        let tz = TimeZone::UTC;
        for (date, time) in [
            (0u16, 0u16),
            (0x5A21, 0x68E9),
            (0x2A6F, 0x0000),
            (0xFF9F, 0xBF7D),
        ] {
            let at = daydream_wcc::dos_time::dos_to_unix(date, time, &tz).unwrap();
            let back = unix_to_dos(at, &tz);
            if date == 0 {
                assert_eq!(back, (0, 0));
            } else {
                assert_eq!(back, (date, time), "{date:#06x} {time:#06x}");
            }
        }
    }
}
