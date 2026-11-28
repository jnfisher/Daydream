// The Combat lab's fight: a character built from the world's records meets one of its
// monsters, and they trade rounds as 1.11p plays them (rules.ts holds the arithmetic). A
// round is the player's blows (energy carried from the last round, six at most) and then the
// monster's (its attacks picked by their cumulative percent, its energy spent as it goes,
// spell attacks cast with their chance, resisted, rolled and softened by magic resistance),
// then its between-rounds spell. Each blow sends the lines the player reads, in the game's
// colours.
//
// Left out, and said so on the page: regeneration, rest and meditation between rounds, a
// SMASH knocking the monster down, damage shields, a weapon's own spells, what a spell does
// besides damage, and groups. An area spell is played as if aimed at the player alone.
//
// Pure: no DOM, so `node --test` runs its tests (fight.test.ts).

import { blowLines, COLOUR, fmtMsg, upperFirst, type Lines, type Texts, type Victim } from '../library/combat.ts';
import {
  A,
  abilityHas,
  abilityValue,
  ATTACK,
  BASH,
  canHit,
  div,
  dodgeChance,
  dynamicStats,
  elementResist,
  energyRound,
  hitChance,
  JUMPKICK,
  KICK,
  MAX_BLOWS,
  mayResist,
  monsterAmountRange,
  monsterFighter,
  monsterGroup,
  moveMods,
  mrDamage,
  playerFighter,
  PUNCH,
  resistChance,
  resolve,
  secondary,
  seeded,
  SMASH,
  userGroup,
  type BlowResult,
  type Character,
  type ClassRec,
  type Fighter,
  type ItemRec,
  type Kit,
  type MonAttack,
  type MonsterRec,
  type RaceRec,
  type Rng,
  type Slot,
  type Source,
  type SpellRec,
} from './rules.ts';

type Get = (col: string) => unknown;
const n = (v: unknown) => (typeof v === 'number' ? v : Number(v) || 0);
const s = (v: unknown) => (v === null || v === undefined ? '' : String(v).split('\0')[0]!);

/** A record's ability slots that hold an ability. */
export function slotsOf(get: Get): Slot[] {
  const out: Slot[] = [];
  for (let i = 0; get(`Ability ${i}`) !== undefined; i++) {
    const a = n(get(`Ability ${i}`));
    if (a) out.push([a, n(get(`Ability Value ${i}`))]);
  }
  return out;
}

export const classOf = (get: Get): ClassRec => ({
  combat: n(get('Combat')),
  basehp: n(get('Min HP')),
  hplevel: n(get('Max HP')),
  magery: n(get('Magic Type')),
  magelvl: n(get('Magic LVL')),
  exp: n(get('EXP %')),
  slots: slotsOf(get),
});

export const raceOf = (get: Get): RaceRec => ({ hpbonus: n(get('HP Bonus')), exp: n(get('EXP %')), slots: slotsOf(get) });

export function itemOf(get: Get): ItemRec {
  const negates: number[] = [];
  for (let i = 0; get(`Negate ${i}`) !== undefined; i++) if (n(get(`Negate ${i}`))) negates.push(n(get(`Negate ${i}`)));
  return {
    number: n(get('Number')),
    name: s(get('Name')),
    type: n(get('Type')),
    ac: n(get('AC')),
    dr: n(get('DR')),
    accuracy: n(get('Accuracy')),
    armtype: n(get('Armour')),
    mindmg: n(get('Min Hit')),
    maxdmg: n(get('Max Hit')),
    speed: n(get('Speed')),
    strreq: n(get('Req Str')),
    hitmsg: n(get('Hit Msg')),
    missmsg: n(get('Miss Msg')),
    slots: slotsOf(get),
    negates,
  };
}

/** A monster record, with its weapon's record when it wields one. */
export function monsterOf(get: Get, weapon: ItemRec | null): MonsterRec {
  const attacks: MonAttack[] = [];
  const spells: MonsterRec['spells'] = [];
  for (let i = 0; get(`Attack Type ${i}`) !== undefined; i++)
    attacks.push({
      type: n(get(`Attack Type ${i}`)),
      pct: n(get(`Attack % ${i}`)),
      acc: n(get(`Attack Accu/Spell ${i}`)),
      min: n(get(`Attack Min Hit/Cast % ${i}`)),
      max: n(get(`Attack Max Hit/Cast LVL ${i}`)),
      energy: n(get(`Attack Energy ${i}`)),
      hitmsg: n(get(`Attack Hit Msg ${i}`)),
      defmsg: n(get(`Attack Dodge Msg ${i}`)),
      missmsg: n(get(`Attack Miss Msg ${i}`)),
      hitspell: n(get(`Attack Hit Spell ${i}`)),
    });
  for (let i = 0; get(`Spell Number ${i}`) !== undefined; i++)
    spells.push({ spell: n(get(`Spell Number ${i}`)), pct: n(get(`Spell Cast % ${i}`)), level: n(get(`Spell Cast LVL ${i}`)) });
  const sources: Source[] = [{ slots: slotsOf(get) }];
  if (weapon) sources.push({ item: weapon.slots });
  return {
    number: n(get('Number')),
    name: s(get('Name')) || 'monster',
    ac: n(get('AC')),
    dr: n(get('DR')),
    mr: n(get('MR')),
    hp: n(get('Hit Points')),
    energy: n(get('Energy')),
    alignment: n(get('Alignment')),
    bsdefense: n(get('BS Defense')),
    exp: n(get('Experience')),
    level: n(get('Exp Multiplier')) || 1,
    weapon: n(get('Weapon Number')),
    attacks,
    spells,
    sources,
  };
}

export function spellOf(get: Get): SpellRec {
  return {
    number: n(get('Number')),
    name: s(get('Name')),
    level: n(get('Level')),
    resmode: n(get('Type of Resists')),
    type: n(get('Spell Type')),
    element: n(get('Attack Type')),
    target: n(get('Target')),
    min: n(get('Min')),
    max: n(get('Max')),
    minInc: n(get('Min Increase')),
    minLvls: n(get('LVLS Min Increase')),
    maxInc: n(get('Max Increase')),
    maxLvls: n(get('LVLS Max Increase')),
    msg: n(get('Cast MSG B')),
    alt: (n(get('MSG Style')) & 1) !== 0,
    slots: slotsOf(get),
  };
}

// ---- the player's kit --------------------------------------------------------------------

/** What the lab knows of the player: the character, the records it reads, the move it fights with. */
export interface Player {
  name: string;
  c: Character;
  cls: ClassRec;
  race: RaceRec;
  weapon: ItemRec | null;
  armour: ItemRec[];
  move: number;
}

/** The ability sources of a player, in the game's order: race, class, worn items, weapon. */
export function sourcesOf(p: Player): Source[] {
  return [{ slots: p.race.slots }, { slots: p.cls.slots }, ...p.armour.map((a) => ({ item: a.slots })), ...(p.weapon ? [{ item: p.weapon.slots }] : [])];
}

/** The player's sheet: the kit a blow is built from, and the figures the page shows. */
export function kitOf(p: Player): Kit & { maxhp: number; mr: number; antiMagic: boolean } {
  const sources = sourcesOf(p);
  const sec = secondary(p.c, p.cls, p.race, sources);
  return {
    c: p.c,
    cls: p.cls,
    weapon: p.weapon,
    armour: p.armour,
    sources,
    dyn: dynamicStats(sources),
    mabase: sec.mabase,
    hp: Math.max(1, sec.maxhp),
    maxhp: sec.maxhp,
    mr: sec.mr,
    antiMagic: abilityHas(sources, A.antiMagic),
  };
}

/** The moves the player may fight with: a plain attack, and the ones the kit's abilities give. */
export function movesOf(p: Player): { move: number; name: string }[] {
  const src = sourcesOf(p);
  const out = [{ move: ATTACK, name: p.weapon ? `Attack with ${p.weapon.name}` : 'Attack with fists' }];
  if (abilityHas(src, A.bash)) out.push({ move: BASH, name: 'Bash' });
  if (abilityHas(src, A.smash)) out.push({ move: SMASH, name: 'Smash' });
  if (abilityHas(src, A.punch)) out.push({ move: PUNCH, name: 'Punch' });
  if (abilityHas(src, A.kick)) out.push({ move: KICK, name: 'Kick' });
  if (abilityHas(src, A.jumpkick)) out.push({ move: JUMPKICK, name: 'Jumpkick' });
  return out;
}

/** The average hit points rolled in training: half of the class's random hit points a level, for every level after the first. */
export const averageRandomHp = (level: number, hplevel: number) => (level > 1 && hplevel > 0 ? Math.floor(((level - 1) * hplevel) / 2) : 0);

// ---- the sheet ---------------------------------------------------------------------------

/** A monster attack slot's figures against the player. */
export interface AttackSheet {
  slot: number;
  type: number;
  /** Percent of blows that pick it. */
  share: number;
  hit: number;
  dodge: number;
  min: number;
  max: number;
  energy: number;
  blows: number;
}

/** Both sides' figures, before any roll. */
export interface Sheet {
  you: Fighter & { blows: number; hit: number; monDodge: number; maxhp: number; mr: number; canHit: boolean };
  mon: { dv: number; dr: number; dodge: number; hp: number; mr: number; exp: number; attacks: AttackSheet[] };
}

export function sheetOf(p: Player, m: MonsterRec): Sheet {
  const kit = kitOf(p);
  const style = monsterGroup(m.alignment);
  const atk = playerFighter(kit, p.move, style);
  const def = m.attacks.length ? monsterFighter(m, 0, userGroup(p.c.evil)) : null;
  const me = playerFighter(kit, ATTACK, style);
  const mods = moveMods(p.move);
  const hit = def ? hitChance(atk.av, mods.hitmod + def.defmod, def.dv, def.defalign, false) : 0;
  const attacks: AttackSheet[] = [];
  let before = 0;
  m.attacks.forEach((a, i) => {
    const share = Math.max(0, Math.min(100, a.pct) - before);
    before = Math.max(before, Math.min(100, a.pct));
    if (share === 0) return;
    const f = monsterFighter(m, i, userGroup(p.c.evil));
    const cost = a.energy < 1 ? 1 : a.energy;
    attacks.push({
      slot: i,
      type: a.type,
      share,
      hit: hitChance(f.av, me.defmod, me.dv, me.defalign, false),
      dodge: me.dodge > 0 ? dodgeChance(me.dodge, f.av, false) : 0,
      min: f.mindmg,
      max: f.maxdmg,
      energy: f.energy,
      blows: Math.min(MAX_BLOWS, Math.floor(m.energy / cost)),
    });
  });
  return {
    you: {
      ...atk,
      blows: blowsAverage(atk.energy),
      hit,
      monDodge: def && def.dodge > 0 ? dodgeChance(def.dodge, atk.av, false) : 0,
      maxhp: kit.maxhp,
      mr: kit.mr,
      canHit: canHitWith(p, m),
    },
    mon: { dv: def?.dv ?? m.ac, dr: def?.dr ?? m.dr * 10, dodge: def?.dodge ?? 0, hp: m.hp, mr: m.mr, exp: m.exp * m.level, attacks },
  };
}

/** Blows a round on average, energy carried from round to round (over 60 rounds). */
export function blowsAverage(cost: number): number {
  let carry = 0;
  let blows = 0;
  for (let i = 0; i < 60; i++) {
    const r = energyRound(carry, cost);
    blows += r.blows;
    carry = r.leftover;
  }
  return blows / 60;
}

/** Whether the player's blows can hurt the monster at all (Magical against Hit Magical). */
export function canHitWith(p: Player, m: MonsterRec): boolean {
  const magreq = abilityValue(m.sources, A.magical, true);
  if (magreq === 0) return true;
  const limbs = !p.weapon || p.move === PUNCH || p.move === KICK || p.move === JUMPKICK;
  const weaponMagical = limbs ? null : (p.weapon!.slots.find(([a]) => a === A.magical)?.[1] ?? 0);
  return canHit(abilityValue(sourcesOf(p), A.hitMagical), magreq, weaponMagical);
}

// ---- the words ---------------------------------------------------------------------------

/** What the lab reads beside the records it was given: a message's lines, a spell. */
export interface World {
  lines: (n: number) => Lines | null;
  spell: (n: number) => SpellRec | null;
  /** A monster's weapon: its name, hit and miss messages. */
  weapon: (n: number) => { name: string; hit: Lines | null; miss: Lines | null } | null;
}

/** Splits at `|` (at most five alternatives) and picks one, the same one for all three forms (`pick3`). */
function pick3(rng: Rng, forms: Lines): Lines {
  const alts = forms.map((f) => f.split('|').slice(0, 5));
  const i = rng.range(0, alts[0]!.length);
  return alts.map((a) => a[i] ?? '') as Lines;
}

/** The player's verbs (`player_verbs`): a weapon's hit and miss messages, the martial-arts words, or fists. */
function playerVerbs(p: Player, w: World, rng: Rng): { hit: Lines; miss: Lines } {
  const mv = p.move;
  if (mv === PUNCH) return both(pick3(rng, ['punch', 'punches', 'punches']));
  if (mv === KICK) return both(pick3(rng, ['kick', 'kicks', 'kicks']));
  if (mv === JUMPKICK) return both(pick3(rng, ['jumpkick', 'jumpkicks', 'jumpkicks']));
  if (!p.weapon) return { hit: pick3(rng, ['punch', 'punches', 'punches']), miss: pick3(rng, ['swing at', 'swings at', 'swings at']) };
  const hitmsg = p.weapon.hitmsg ? w.lines(p.weapon.hitmsg) : null;
  const hit = pick3(rng, hitmsg ?? ['attack', 'attacks', 'attacks']);
  const missmsg = p.weapon.missmsg ? w.lines(p.weapon.missmsg) : null;
  return { hit, miss: missmsg ? pick3(rng, missmsg) : hit };
}
const both = (l: Lines) => ({ hit: l, miss: l });

/** The line the player reads for their own blow (`player_on_monster`). */
export function youLine(verbs: { hit: Lines; miss: Lines }, mon: string, r: BlowResult): string {
  switch (r.outcome) {
    case 'hit':
      return `${COLOUR.hitRoom}You ${r.critical ? 'critically ' : ''}${verbs.hit[0]} ${mon} for ${r.damage} damage!`;
    case 'armour':
      return `${COLOUR.armour}Your ${verbs.miss[0]} ${mon} hits, but glances off its armour.`;
    case 'dodge':
      return `${COLOUR.miss}You ${verbs.miss[0]} ${mon} who dodges your attack!`;
    case 'miss':
      return `${COLOUR.miss}You ${verbs.miss[0]} ${mon}!`;
  }
}

// ---- a round -----------------------------------------------------------------------------

/** One line of a round: who it is about, the text in the game's colours, and what it did. */
export interface Line {
  who: 'you' | 'mon' | 'end';
  text: string;
  damage?: number;
  outcome?: BlowResult['outcome'] | 'cast' | 'resisted' | 'failed';
  /** What the lab did not play of it. */
  note?: string;
}

/** Where a fight stands between rounds. */
export interface State {
  hp: number;
  monHp: number;
  /** The player's energy carried into the next round. */
  carry: number;
  round: number;
}

export interface Fight {
  player: Player;
  monster: MonsterRec;
  world: World;
}

export const startOf = (f: Fight): State => ({ hp: kitOf(f.player).hp, monHp: f.monster.hp, carry: 0, round: 0 });

/** Ability slots that only shape a spell's words or targets: nothing to play. */
const QUIET = new Set([0, 101, 108, 109, 110, 111, 112, 113, 115, 120, 137]);

const ANSI_LTREDBLK = '\x1b[1;31;40m';

/** One round: the player's blows, then the monster's. Returns the state after it and the lines. */
export function roundOf(f: Fight, st: State, rng: Rng): { state: State; lines: Line[] } {
  const lines: Line[] = [];
  const state = { ...st, round: st.round + 1 };
  const { player: p, monster: m, world: w } = f;
  const kit = { ...kitOf(p), hp: state.hp };
  const victim: Victim = { name: p.name, poss: 'their', subj: 'they' };

  // the player's blows
  if (!canHitWith(p, m)) {
    const limbs = !p.weapon || p.move === PUNCH || p.move === KICK || p.move === JUMPKICK;
    lines.push({ who: 'you', text: limbs ? `Your ${p.move === KICK || p.move === JUMPKICK ? 'feet' : 'fists'} have no effect against this monster!` : 'Your weapon has no effect against this monster!' });
  } else if (m.attacks.length) {
    const atk = playerFighter(kit, p.move, monsterGroup(m.alignment));
    const def = monsterFighter(m, 0, userGroup(p.c.evil));
    const verbs = playerVerbs(p, w, rng);
    let energy = state.carry + 1000;
    for (let b = 0; b < MAX_BLOWS; b++) {
      const r = resolve(atk, def, p.move, rng);
      if (atk.energy > energy) break;
      energy -= atk.energy;
      const dmg = r.outcome === 'hit' ? Math.min(r.damage, state.monHp) : 0;
      state.monHp -= dmg;
      lines.push({ who: 'you', text: youLine(verbs, m.name, r), damage: dmg, outcome: r.outcome });
      if (state.monHp <= 0) {
        lines.push({ who: 'end', text: `${COLOUR.grey}You gain ${(m.exp * m.level).toLocaleString('en-US')} experience.` });
        state.carry = 0;
        return { state, lines };
      }
    }
    state.carry = energy;
  }

  // the monster's
  monsterRound(f, kit, state, lines, victim, rng);
  return { state, lines };
}

function monsterRound(f: Fight, kit: ReturnType<typeof kitOf>, state: State, lines: Line[], victim: Victim, rng: Rng): void {
  const { player: p, monster: m, world: w } = f;
  if (!m.attacks.length) return;
  let energy = m.energy;
  let slot = 0;
  let stop = false;
  const hurt = (dmg: number) => {
    state.hp -= dmg;
    return state.hp <= 0;
  };
  const killed = () => lines.push({ who: 'end', text: `${ANSI_LTREDBLK}You have been killed!` });
  const cast = (spellNo: number, a: MonAttack | null): 'tired' | 'resisted' | 'failed' | 'applied' | 'nothing' | 'dead' => {
    const roll = rng.range(0, 100);
    const spl = w.spell(a ? a.acc : spellNo);
    if (!spl) return 'nothing';
    const chance = a ? a.min : 100;
    const cost = a ? a.energy : 0;
    let resisted = abilityValue(kit.sources, A.spellImmune) > spl.level;
    const castok = roll < chance;
    if (energy < cost) return 'tired';
    if (kit.armour.some((i) => i.negates.includes(spl.number))) resisted = true;
    else if (castok && mayResist(spl, kit.antiMagic) && rng.range(1, 100) <= resistChance(kit.mr)) resisted = true;
    if (resisted || !castok) {
      if (cost) energy -= Math.max(1, div(cost, 2));
      lines.push(
        resisted
          ? { who: 'mon', text: `${COLOUR.miss}You resisted ${m.name}'s cast of ${spl.name}.`, outcome: 'resisted' }
          : { who: 'mon', text: `${COLOUR.miss}The ${m.name} attempted to cast ${spl.name} at you, but failed.`, outcome: 'failed' },
      );
      return resisted ? 'resisted' : 'failed';
    }
    energy -= cost;
    const range = monsterAmountRange(spl, a ? a.max : 0);
    const rolled = rng.range(0, range.max - range.min + 1) + range.min;
    const power = div(rolled * (100 - elementResist(spl, kit.sources)), 100);
    let dmg = 0;
    let other = false;
    for (const [ab, v] of spl.slots) {
      const amount = v !== 0 ? v : power;
      if (ab === A.damageNoArmour) dmg += amount;
      else if (ab === A.damageArmour) dmg += mrDamage(amount, kit.mr, kit.antiMagic);
      else if (!QUIET.has(ab)) other = true;
    }
    const msg = spl.msg ? w.lines(spl.msg) : null;
    const fmt = msg?.[1] || '%s cast %s on you.';
    const text = spl.alt ? fmtMsg(upperFirst(fmt), [power]) : upperFirst(fmtMsg(fmt, [m.name, spl.name, power]));
    lines.push({ who: 'mon', text: `${COLOUR.hitRoom}${text}`, damage: dmg, outcome: 'cast', ...(other ? { note: dmg ? 'and more the lab does not play' : 'its effects are not played here' } : {}) });
    if (dmg > 0 && hurt(dmg)) {
      killed();
      return 'dead';
    }
    return 'applied';
  };

  for (let b = 0; b < MAX_BLOWS && !stop; b++) {
    const roll = rng.range(0, 100);
    const i = m.attacks.findIndex((a) => a.pct > roll);
    if (i >= 0) slot = i;
    let a = m.attacks[slot]!;
    if (a.type === 2) {
      const r = cast(0, a);
      if (r === 'dead') return;
      if (r !== 'nothing') continue;
      slot = 0;
      a = m.attacks[0]!;
      if (a.type !== 1) continue;
    } else if (a.type === 3) {
      // a rob is a stub in 1.11p: the first attack strikes instead
      slot = 0;
      a = m.attacks[0]!;
      if (a.type !== 1) continue;
    } else if (a.type !== 1) continue;

    const atk = monsterFighter(m, slot, userGroup(p.c.evil));
    const def = playerFighter(kit, ATTACK, monsterGroup(m.alignment));
    const r = resolve(atk, def, ATTACK, rng);
    if (atk.energy > energy) break;
    energy -= atk.energy;
    if (atk.energy > energy) stop = true;
    const texts: Texts = { hit: a.hitmsg ? w.lines(a.hitmsg) : null, def: a.defmsg ? w.lines(a.defmsg) : null, miss: a.missmsg ? w.lines(a.missmsg) : null };
    const verbs = monsterVerbs(m, w, rng);
    const shown = r.outcome === 'armour' ? 'armour' : r.outcome;
    const l = blowLines(m.name, verbs, texts, victim, shown, r.damage);
    lines.push({ who: 'mon', text: l.you, damage: r.damage, outcome: r.outcome });
    if (r.damage > 0) {
      if (hurt(r.damage)) {
        killed();
        return;
      }
      if (a.hitspell && cast(a.hitspell, null) === 'dead') return;
    }
  }
  // the spell after the round
  if (m.spells[0]?.spell) {
    const roll = rng.range(0, 100);
    const sp = m.spells.find((x) => x.pct > roll);
    if (sp?.spell) {
      const spl = w.spell(sp.spell);
      // on itself (Self, Self or Single, Monster), else at the player (or the room, which holds the player)
      if (spl && spl.target !== 1 && spl.target !== 2 && spl.target !== 4) cast(sp.spell, null);
      else if (spl) lines.push({ who: 'mon', text: `${COLOUR.grey}The ${m.name} casts ${spl.name} on itself.`, outcome: 'cast' });
    }
  }
}

/** A monster's weapon's verbs, one alternative for all three forms, or blank without one. */
function monsterVerbs(m: MonsterRec, w: World, rng: Rng) {
  const weapon = m.weapon ? w.weapon(m.weapon) : null;
  const none: Lines = ['', '', ''];
  if (!weapon) return { hit: none, miss: none, weapon: '' };
  const hit = weapon.hit ? pick3(rng, weapon.hit) : none;
  return { hit, miss: weapon.miss ? pick3(rng, weapon.miss) : hit, weapon: weapon.name };
}

// ---- many fights -------------------------------------------------------------------------

/** How a fight ended. */
export interface Ending {
  won: boolean;
  rounds: number;
  /** Hit points the player had left (won) or the monster had left (lost). */
  left: number;
  dealt: number;
  taken: number;
}

/** A fight to the end, at most `cap` rounds; the rounds' lines when `keep`. */
export function fightOut(f: Fight, rng: Rng, cap = 200, keep = false): Ending & { rounds_: Line[][] } {
  let st = startOf(f);
  const rounds: Line[][] = [];
  let dealt = 0;
  let taken = 0;
  while (st.round < cap && st.hp > 0 && st.monHp > 0) {
    const before = st;
    const r = roundOf(f, st, rng);
    dealt += before.monHp - r.state.monHp;
    taken += before.hp - r.state.hp;
    st = r.state;
    if (keep) rounds.push(r.lines);
  }
  const won = st.monHp <= 0;
  return { won, rounds: st.round, left: won ? st.hp : st.monHp, dealt, taken, rounds_: rounds };
}

/** Many fights from one seed: how often the player wins, how long it takes, what each side does a round. */
export interface Odds {
  fights: number;
  wins: number;
  rounds: number;
  /** Average damage a round, each way. */
  dealt: number;
  taken: number;
  /** Average hit points left after a win. */
  left: number;
  /** Fights that reached the round cap. */
  stalled: number;
}

export function oddsOf(f: Fight, fights = 1000, seed = 1, cap = 200): Odds {
  const rng = seeded(seed);
  let wins = 0;
  let rounds = 0;
  let dealt = 0;
  let taken = 0;
  let left = 0;
  let stalled = 0;
  for (let i = 0; i < fights; i++) {
    const e = fightOut(f, rng, cap);
    if (e.won) {
      wins++;
      left += e.left;
    } else if (e.rounds >= cap) stalled++;
    rounds += e.rounds;
    dealt += e.dealt;
    taken += e.taken;
  }
  return { fights, wins, rounds: rounds / fights, dealt: rounds ? dealt / rounds : 0, taken: rounds ? taken / rounds : 0, left: wins ? left / wins : 0, stalled };
}
