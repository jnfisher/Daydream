// MajorMUD 1.11p's melee and monster-cast arithmetic, as silvermere plays it: energy per blow
// and blows per round, the attack and defence values of both sides, hit, dodge, critical and
// damage, and a monster's cast at a player (resistance, strength, magic resistance). Ported
// from silvermere-rules (combat/energy.rs, combat/attack.rs, combat/fighter.rs, spells/resist.rs,
// spells/apply_monster.rs) and silvermere-engine (player/secondary.rs, combat/attack.rs,
// monsters/cast.rs); UPSTREAM.md names the commit. The game's integer order and truncation
// toward zero are kept, and so are its (sic) quirks: they are what players meet.
//
// Pure: no DOM, so `node --test` runs its tests (rules.test.ts).

/** The game's random source: `genrdn(lo, hi)`, a whole number in `[lo, hi)` (lo when the range is empty). */
export interface Rng {
  range(lo: number, hi: number): number;
}

/** A percentile roll, `genrdn(0, 100)`. */
export const pct = (rng: Rng) => rng.range(0, 100);

/** A seeded `Rng`, so a fight replays the same from its seed. */
export function seeded(seed: number): Rng {
  let s = seed | 0;
  const next = () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return { range: (lo, hi) => (hi > lo ? lo + Math.floor(next() * (hi - lo)) : lo) };
}

/** C's integer division: truncates toward zero. */
export const div = (a: number, b: number) => Math.trunc(a / b);
/** A value kept in 16 bits, read signed. */
export const i16 = (v: number) => ((v & 0xffff) << 16) >> 16;
/** A value kept in 16 bits, read unsigned. */
export const u16 = (v: number) => v & 0xffff;
/** A value kept in 32 bits, read unsigned. */
const u32 = (v: number) => v >>> 0;

// ---- energy ------------------------------------------------------------------------------

/** Every player's energy a round, and the floor of a blow's cost. */
export const MAXENERGY = 1000;
export const MIN_ENERGY = 200;
export const MAX_BLOWS = 6;

/**
 * The energy one blow costs (`compute_energy_used`): `skill = level*combat + 45`, `quick =
 * agility + 150`, `divisor = skill*quick*1500/9000` (32 bits, unsigned), `cost = extra +
 * speed*1000/divisor`; 50 when anything is 0.
 */
export function energyUsed(level: number, combat: number, agility: number, speed: number, extra: number): number {
  const skill = u32(i16(level) * combat + 45);
  const quick = u32(i16(agility) + 150);
  const divisor = Math.floor(u32(Number((BigInt(skill) * BigInt(quick) * 1500n) & 0xffffffffn)) / 9000);
  if (skill === 0 || quick === 0 || divisor === 0) return 50;
  return u32(extra + Math.floor(u32(speed * 1000) / divisor)) | 0;
}

/** One energy round: the tick adds 1,000 to what was carried, then blows of `cost` while it lasts, at most six. */
export function energyRound(carry: number, cost: number): { blows: number; leftover: number } {
  const c = cost < 1 ? MIN_ENERGY : cost;
  let energy = carry + MAXENERGY;
  let blows = 0;
  while (blows < MAX_BLOWS && energy >= c) {
    energy -= c;
    blows++;
  }
  return { blows, leftover: energy };
}

// ---- moves and one blow ------------------------------------------------------------------

/** The attack moves, by the game's numbers. */
export const PUNCH = 1;
export const KICK = 2;
export const JUMPKICK = 3;
export const BACKSTAB = 4;
export const ATTACK = 5;
export const BASH = 6;
export const SMASH = 7;

/** The move table: hit modifier, damage percent, whether it can be a critical. */
export function moveMods(mv: number): { hitmod: number; dmgpct: number; canCrit: boolean } {
  switch (mv) {
    case KICK:
      return { hitmod: 0, dmgpct: 33, canCrit: true };
    case JUMPKICK:
      return { hitmod: 0, dmgpct: 66, canCrit: true };
    case BASH:
      return { hitmod: -15, dmgpct: 10, canCrit: false };
    case SMASH:
      return { hitmod: -25, dmgpct: 20, canCrit: false };
    case 8:
      return { hitmod: -75, dmgpct: 125, canCrit: false };
    case BACKSTAB:
      return { hitmod: 0, dmgpct: 0, canCrit: false };
    default:
      return { hitmod: 0, dmgpct: 0, canCrit: true };
  }
}

/** One side of a blow: what `resolve` reads. */
export interface Fighter {
  av: number;
  dv: number;
  /** Defence against the foe's alignment, added to `dv` in the hit chance. */
  defalign: number;
  /** Damage resistance in tenths of a point. */
  dr: number;
  /** Experience for beating it: a monster's experience times its level. */
  expval: number;
  /** Energy one blow costs. */
  energy: number;
  mindmg: number;
  maxdmg: number;
  dodge: number;
  crit: number;
  /** Added to the attacker's hit modifier when this side defends. */
  defmod: number;
  /** The blow was fast enough for the critical bonus. */
  fast: boolean;
}

export const blankFighter = (): Fighter => ({ av: 0, dv: 0, defalign: 0, dr: 0, expval: 0, energy: 0, mindmg: 0, maxdmg: 0, dodge: 0, crit: 0, defmod: 0, fast: false });

/**
 * The hit chance without the negative-dodge roll: a backstab is `av - dv`; else
 * `atkpower = (av+hitmod)^2/14/10` and `100 - (dv+defalign)^2/atkpower` (5 with no power),
 * held to 10..99. The roll is 1..99, so 99 lands 98 times in 99.
 */
export function hitChance(av: number, hitmod: number, dv: number, defalign: number, backstab: boolean): number {
  let chance: number;
  if (backstab) chance = av - dv;
  else {
    const power = av + hitmod;
    const atkpower = div(div(power * power, 14), 10);
    if (atkpower === 0) chance = 5;
    else {
      const def = dv + defalign;
      chance = 100 - div(def * def, atkpower);
    }
  }
  return Math.min(99, Math.max(10, chance));
}

/** The critical chance rolled against: above 40 only a third of the rest counts. */
export const effectiveCrit = (crit: number) => (crit > 40 ? div(crit - 40, 3) + 40 : crit);

/** The defender's chance to dodge a landed blow: `dodge*10/(av/8)`, at most 95, a fifth against a backstab. */
export function dodgeChance(dodge: number, av: number, backstab: boolean): number {
  let chance = av > 8 ? div(dodge * 10, div(av, 8)) : 0;
  if (chance > 95) chance = 95;
  if (backstab) chance = div(chance, 5);
  return chance;
}

export type BlowResult =
  | { outcome: 'miss' | 'dodge' | 'armour'; damage: 0; critical: false }
  | { outcome: 'hit'; damage: number; critical: boolean };

/**
 * One blow of `atk` on `def` with move `mv` (`atkmove`). The attacker is a copy: its range and
 * critical chance change in place as the game's do. The rolls come in the game's order: hit,
 * negative dodge, critical, damage, dodge.
 */
export function resolve(attacker: Fighter, def: Fighter, mv: number, rng: Rng): BlowResult {
  const atk = { ...attacker };
  const mods = moveMods(mv);
  const backstab = mv === BACKSTAB;
  if (!mods.canCrit) atk.crit = 0;
  const hitmod = mods.hitmod + def.defmod;
  if (mods.dmgpct !== 0) {
    atk.mindmg = div(atk.mindmg * (mods.dmgpct + 100), 100);
    atk.maxdmg = div(atk.maxdmg * (mods.dmgpct + 100), 100);
  }
  const roll = rng.range(1, 100);
  const chance = def.dodge < 0 && pct(rng) > def.dodge + 100 ? 99 : hitChance(atk.av, hitmod, def.dv, def.defalign, backstab);
  if (atk.crit > 40) atk.crit = div(atk.crit - 40, 3) + 40;
  if (roll >= chance) return { outcome: 'miss', damage: 0, critical: false };
  let critical = false;
  if (pct(rng) < atk.crit && mv !== BASH && mv !== SMASH) {
    critical = true;
    atk.mindmg = atk.maxdmg * 2;
    atk.maxdmg *= 4;
  }
  if (atk.mindmg > atk.maxdmg) atk.maxdmg = atk.mindmg;
  let damage = rng.range(0, atk.maxdmg - atk.mindmg + 1) + atk.mindmg - div(def.dr, 10);
  const dodge = dodgeChance(def.dodge, atk.av, backstab);
  if (mv === BASH) damage *= 3;
  else if (mv === SMASH) damage *= 5;
  if (def.dodge > 0 && pct(rng) < dodge) return { outcome: 'dodge', damage: 0, critical: false };
  if (damage <= 0) return { outcome: 'armour', damage: 0, critical: false };
  return { outcome: 'hit', damage, critical };
}

// ---- abilities ---------------------------------------------------------------------------

/** The abilities the lab reads, by number. */
export const A = {
  damageNoArmour: 1,
  defence: 2,
  coldDefence: 3,
  damage: 4,
  hotDefence: 5,
  damageResist: 7,
  shadowStealth: 9,
  shield: 10,
  damageArmour: 17,
  poisonImmune: 21,
  avBless: 22,
  defVsEvil: 24,
  defVsGood: 25,
  magical: 28,
  punch: 29,
  kick: 30,
  bash: 31,
  smash: 32,
  dodge: 34,
  jumpkick: 35,
  magicResist: 36,
  antiMagic: 51,
  critical: 58,
  resistStone: 65,
  resistLightning: 66,
  slowness: 68,
  energyUse: 87,
  alterHp: 88,
  punchAv: 89,
  kickAv: 90,
  jumpkickAv: 91,
  punchDmg: 92,
  kickDmg: 93,
  jumpkickDmg: 94,
  drPercent: 99,
  defenceMod: 104,
  avCurse: 105,
  avThird: 106,
  blindUser: 107,
  spellImmune: 139,
  hitMagical: 142,
  resistWater: 147,
} as const;

/** An ability slot: its ability and value. */
export type Slot = [number, number];

/** Abilities whose total is the best value, not the sum (`BEST_VALUE`). */
const BEST = new Set([22, 71, 72, 87, 101, 105, 106]);
/** The monster's best-value abilities (`MONSTER_BEST_VALUE`): the elemental defences too. */
const MONSTER_BEST = new Set([...BEST, 3, 5, 65, 66]);

/**
 * Where a total reads from, in the game's order: a list of every slot (a race, a class, a
 * monster's own) or an item, which gives the value of its first slot with the ability (0 when
 * it has none, which still counts for a best value).
 */
export type Source = { slots: Slot[] } | { item: Slot[] };

/** The total of one ability over the sources (`AbilityLookup::value`). */
export function abilityValue(sources: readonly Source[], ability: number, monster = false): number {
  if (ability === 0 && !monster) return 0;
  const best = (monster ? MONSTER_BEST : BEST).has(ability);
  let total: number | null = best ? null : 0;
  const add = (v: number) => (total = total === null ? v : best ? Math.max(total, v) : total + v);
  for (const s of sources) {
    if ('slots' in s) for (const [a, v] of s.slots) a === ability && add(v);
    else add(s.item.find(([a]) => a === ability)?.[1] ?? 0);
  }
  return total ?? 0;
}

/** Whether any source carries the ability. */
export const abilityHas = (sources: readonly Source[], ability: number) =>
  sources.some((s) => ('slots' in s ? s.slots : s.item).some(([a]) => a === ability));

/** The ability-driven bonuses of a player record (`dynamic_stats`), and the status flags they set. */
export interface Dynamic {
  avbonus: number;
  acbonus: number;
  dmgbonus: number;
  critbonus: number;
  drbonus: number;
  drpct: number;
  energyuse: number;
  shield: number;
  defmod: number;
  blind: boolean;
  slowed: boolean;
}

/** The dynamic stats over the sources: every slot of every source counts (an item's too), the
 * three Alter AV abilities keep the best, and Energy Use averages as it goes. */
export function dynamicStats(sources: readonly Source[]): Dynamic {
  const d: Dynamic = { avbonus: 0, acbonus: 0, dmgbonus: 0, critbonus: 0, drbonus: 0, drpct: 0, energyuse: 0, shield: 0, defmod: 0, blind: false, slowed: false };
  let av: number | null = null;
  for (const s of sources)
    for (const [a, v] of 'slots' in s ? s.slots : s.item) {
      if (a === A.avBless || a === A.avCurse || a === A.avThird) av = av === null || av < v ? v : av;
      else if (a === A.defence) d.acbonus += v;
      else if (a === A.damage) d.dmgbonus += v;
      else if (a === A.critical) d.critbonus += v;
      else if (a === A.damageResist) d.drbonus += v;
      else if (a === A.drPercent) d.drpct += v;
      else if (a === A.energyUse) d.energyuse = d.energyuse !== 0 ? div(d.energyuse + v, 2) : v;
      else if (a === A.shield) d.shield += v;
      else if (a === A.defenceMod) d.defmod += v;
      else if (a === A.blindUser) d.blind = true;
      else if (a === A.slowness) d.slowed = true;
    }
  d.avbonus = av ?? 0;
  return d;
}

// ---- the player --------------------------------------------------------------------------

/** A character as the lab builds one: stats as they stand (with what items add), level, and hit points. */
export interface Character {
  level: number;
  str: number;
  intel: number;
  wil: number;
  agi: number;
  hea: number;
  cha: number;
  /** Hit points rolled in training, on top of the formula. */
  randomhp: number;
  /** Percent of the most they can carry. */
  encumpct: number;
  /** Evil points: sort them into good, neutral or evil. */
  evil: number;
}

/** What a class record gives. */
export interface ClassRec {
  combat: number;
  basehp: number;
  hplevel: number;
  magery: number;
  magelvl: number;
  exp: number;
  slots: Slot[];
}

/** What a race record gives. */
export interface RaceRec {
  hpbonus: number;
  exp: number;
  slots: Slot[];
}

/** What an item record gives. */
export interface ItemRec {
  number: number;
  name: string;
  type: number;
  ac: number;
  dr: number;
  accuracy: number;
  armtype: number;
  mindmg: number;
  maxdmg: number;
  speed: number;
  strreq: number;
  hitmsg: number;
  missmsg: number;
  slots: Slot[];
  negates: number[];
}

export type Group = 'good' | 'neutral' | 'evil';

/** A player's alignment group by evil points (`user_align_group`). */
export const userGroup = (evil: number): Group => (evil <= -51 ? 'good' : evil >= 40 ? 'evil' : 'neutral');

/** A monster's alignment group (`monster_align_group`): Neutral Evil counts as neutral. */
export function monsterGroup(alignment: number): Group {
  if (alignment === 1 || alignment === 2 || alignment === 6) return 'evil';
  if (alignment === 0 || alignment === 4) return 'good';
  return 'neutral';
}

/** The figures the secondary-stats pass gives that the lab reads: hit points, martial-arts base, magic resistance. */
export function secondary(c: Character, cls: ClassRec, race: RaceRec, sources: readonly Source[]): { maxhp: number; mabase: number; mr: number } {
  const tmp = div(i16((c.hea - 50) * c.level), 16);
  const maxhp =
    div(c.hea, 2) + c.level * cls.basehp + c.level * race.hpbonus + tmp + c.randomhp + abilityValue(sources, A.alterHp);
  const mabase = Math.min(75, Math.max(1, i16(div(c.level, 10) + div(c.intel - 50, 10) + div(c.agi - 50, 20) + div(c.cha - 50, 30))));
  const mr = div(u16(c.intel + c.wil * 3), 4) + abilityValue(sources, A.magicResist);
  return { maxhp: i16(maxhp), mabase, mr: u16(mr) };
}

/** The speed figures of the martial-arts moves and of bare fists: normal and slowed. */
const SPEED: Record<number, [number, number]> = { [PUNCH]: [1150, 1750], [KICK]: [1400, 2000], [JUMPKICK]: [1900, 2650], 0: [1200, 1800] };

/** A player's whole kit for a blow. */
export interface Kit {
  c: Character;
  cls: ClassRec;
  weapon: ItemRec | null;
  armour: ItemRec[];
  sources: Source[];
  dyn: Dynamic;
  mabase: number;
  hp: number;
}

/**
 * The player's side of a blow (`player_fighter`): `mv` is the move, `style` the foe's group.
 * The same sheet serves the player as attacker and as defender. A spell was not cast this
 * round, the player is not in a group, and the light is ordinary.
 */
export function playerFighter(k: Kit, mv: number, style: Group): Fighter {
  const { c, cls, weapon, armour, sources, dyn } = k;
  const { level, str, agi } = c;
  const combat = cls.combat;
  const encumpct = c.encumpct;
  const speed = (p: [number, number]) => (dyn.slowed ? p[1] : p[0]);
  const f: Fighter = { ...blankFighter(), energy: 5, mindmg: 1, maxdmg: 4 };
  const v = (a: number) => abilityValue(sources, a);

  if (userGroup(c.evil) !== style) {
    // (sic) against a good foe the Defence vs Evil value stands in
    if (style === 'good' || style === 'evil') f.defalign = v(A.defVsEvil);
  }
  if (abilityHas(sources, A.shadowStealth)) f.defalign += 10;
  f.crit = Math.max(1, k.mabase + dyn.critbonus);
  f.dodge = k.hp > 0 ? v(A.dodge) + div(c.cha - 50, 5) + div(level, 5) + div(agi - 50, 3) : -1;

  let ac = 0;
  let accuracy = 0;
  let tooweak = false;
  const lvl = Math.min(level, 20);
  if (mv === PUNCH || mv === KICK || mv === JUMPKICK) {
    f.energy = energyUsed(level, combat, agi, speed(SPEED[mv]!), 0);
    const n = v(mv === PUNCH ? A.punch : mv === KICK ? A.kick : A.jumpkick);
    f.mindmg = div(lvl * n, 8) + 2;
    f.maxdmg = mv === PUNCH ? div((lvl + 3) * n, 4) + 6 : div(lvl * n, 6) + (mv === KICK ? 7 : 8);
  } else if (weapon) {
    ac = weapon.ac;
    f.mindmg = weapon.mindmg;
    f.maxdmg = weapon.maxdmg;
    const wspeed = dyn.slowed ? div(weapon.speed * 3, 2) : weapon.speed;
    f.energy = energyUsed(level, combat, agi, wspeed, 0);
    if (weapon.strreq > str) {
      f.energy = Math.floor((f.energy * ((weapon.strreq - str) * 3 + 200)) / 200);
      tooweak = true;
    }
    accuracy = weapon.accuracy;
  } else f.energy = energyUsed(level, combat, agi, speed(SPEED[0]!), 0);

  f.maxdmg += div(str - 50, 10);
  const strmin = div(str - 100, 10) * 2;
  if (strmin > 0) f.mindmg += strmin;
  if (f.mindmg > f.maxdmg) f.mindmg = f.maxdmg;
  f.mindmg = Math.max(0, f.mindmg);
  f.maxdmg = Math.max(0, f.maxdmg);

  const kinds = { leather: false, chain: false, plate: false };
  for (const a of armour) {
    if (a.type !== 0) continue;
    ac += a.ac;
    f.dr += a.dr;
    accuracy += a.accuracy;
    if (a.armtype >= 3 && a.armtype <= 6) kinds.leather = true;
    else if (a.armtype === 7 || a.armtype === 8) kinds.chain = true;
    else if (a.armtype === 9) kinds.plate = true;
  }

  if (mv !== BACKSTAB) {
    if (accuracy === 0) accuracy = 1;
    if (encumpct < 33 && k.hp > 0) {
      accuracy += 15 - div(encumpct, 10);
      f.dodge += 10 - div(encumpct, 10);
    }
    const skill = (combat - 1) * 2 + div(level, 2) + div(accuracy, 2);
    const training = (combat - 1) * isqrt(level);
    f.av = i16((skill + training) * 2 + div(str - 50, 3) + div(agi - 50, 6));
  }
  f.dv = div(ac, 10);

  f.av += dyn.avbonus;
  f.dv += dyn.acbonus;
  f.maxdmg += dyn.dmgbonus;
  f.dr += dyn.drbonus;
  f.dr = div((dyn.drpct + 100) * f.dr, 100);

  if (mv === PUNCH || mv === KICK || mv === JUMPKICK) {
    const [av, dmg] = mv === PUNCH ? [A.punchAv, A.punchDmg] : mv === KICK ? [A.kickAv, A.kickDmg] : [A.jumpkickAv, A.jumpkickDmg];
    f.av += v(av);
    f.mindmg += v(dmg);
    f.maxdmg += v(dmg);
  }

  // Alter Energy Usage, read as an unsigned percent (sic: a negative one is huge)
  const eupct = u32(dyn.energyuse);
  if (eupct > 0) f.energy = Math.floor(u32(Number((BigInt(f.energy) * BigInt(eupct)) & 0xffffffffn)) / 100);

  if (dyn.blind) {
    f.av -= 10;
    f.defalign -= 10;
  }
  let shield = dyn.shield;
  if (shield !== 0) {
    if (kinds.plate) shield = div(shield, 4);
    else if (kinds.chain) shield = div(shield, 3);
    else if (kinds.leather) shield = div(shield, 2);
    f.dv += shield;
  }
  f.defmod = dyn.defmod;

  f.energy = Math.floor((f.energy * (div(encumpct, 2) + 75)) / 100);
  if (f.energy < 200 && encumpct <= 66 && !tooweak) {
    const n = Math.min(20, 200 - f.energy + div(agi - 50, 10));
    f.crit += encumpct < 33 ? n : div(n, 2);
    f.fast = true;
  }
  if (mv === BASH) f.energy *= 2;
  else if (mv === SMASH) f.energy = MAXENERGY;
  if (f.energy < MIN_ENERGY) f.energy = MIN_ENERGY;
  if (f.energy > MAXENERGY) f.energy = MAXENERGY;
  return f;
}

/** The whole part of a square root, as the game counts the training term. */
export function isqrt(n: number): number {
  if (n <= 0) return 0;
  let r = 0;
  while ((r + 1) * (r + 1) <= n) r++;
  return r;
}

/** Hit Magical (`can_hit`): the player's total covers the monster's requirement first, the
 * weapon's Magical the rest; fists and feet are never magical (`weaponMagical` null). */
export function canHit(hitmag: number, magreq: number, weaponMagical: number | null): boolean {
  const left = hitmag >= magreq ? 0 : magreq - hitmag;
  if (left === 0) return true;
  return weaponMagical !== null && weaponMagical >= left;
}

// ---- the monster -------------------------------------------------------------------------

/** One of a monster's attack slots, as the fight reads it. */
export interface MonAttack {
  type: number;
  /** Cumulative percent. */
  pct: number;
  /** Accuracy, or the spell a spell attack casts. */
  acc: number;
  /** Least damage, or a spell's cast chance. */
  min: number;
  /** Most damage, or a spell's cast level. */
  max: number;
  energy: number;
  hitmsg: number;
  defmsg: number;
  missmsg: number;
  hitspell: number;
}

/** What a monster record gives. */
export interface MonsterRec {
  number: number;
  name: string;
  ac: number;
  dr: number;
  mr: number;
  hp: number;
  energy: number;
  alignment: number;
  bsdefense: number;
  exp: number;
  level: number;
  weapon: number;
  attacks: MonAttack[];
  /** Between-rounds spells: spell, cumulative percent, level. */
  spells: { spell: number; pct: number; level: number }[];
  /** The monster's own slots, then its weapon's (as an item). */
  sources: Source[];
}

/** The monster's side of a blow with attack slot `slot` (`monster_fighter`); `style` is the foe's group. */
export function monsterFighter(m: MonsterRec, slot: number, style: Group): Fighter {
  const a = m.attacks[slot]!;
  const v = (x: number) => abilityValue(m.sources, x, true);
  const f: Fighter = {
    ...blankFighter(),
    av: a.acc + v(A.avBless) + v(A.avCurse) + v(A.avThird),
    dv: m.ac + v(A.defence),
    dr: m.dr * 10 + v(A.damageResist),
    expval: u32(m.exp * m.level),
    energy: a.energy,
    mindmg: a.min + v(A.damage),
    maxdmg: a.max + v(A.damage),
    dodge: v(A.dodge),
    defmod: v(A.defenceMod),
  };
  if (monsterGroup(m.alignment) !== style && (style === 'good' || style === 'evil')) f.defalign = v(A.defVsEvil);
  if (abilityHas(m.sources, A.shadowStealth)) f.defalign += 10;
  const eu = u32(v(A.energyUse));
  if (eu > 0) {
    f.energy = Math.floor(u32(f.energy * eu) / 100);
    if (m.energy < f.energy) f.energy = m.energy;
  }
  return f;
}

// ---- a monster's cast at a player --------------------------------------------------------

/** What a spell record gives a cast. */
export interface SpellRec {
  number: number;
  name: string;
  level: number;
  /** `Type of Resists`: 0 never, 1 with Anti Magic, 2 always. */
  resmode: number;
  /** `Spell Type`: 0-2 are attacks with an element. */
  type: number;
  element: number;
  target: number;
  min: number;
  max: number;
  minInc: number;
  minLvls: number;
  maxInc: number;
  maxLvls: number;
  /** The message its target and the room read (`Cast MSG B`). */
  msg: number;
  /** Alternate messaging. */
  alt: boolean;
  slots: Slot[];
}

/** The strength range of a monster's cast at `level` (`monster_amount_range`: no level cap). */
export function monsterAmountRange(s: SpellRec, level: number): { min: number; max: number } {
  const grow = (inc: number, lvls: number) => (lvls > 0 ? div(inc * level, lvls) : 0);
  const min = s.min + grow(s.minInc, s.minLvls);
  const max = s.max + grow(s.maxInc, s.maxLvls);
  return { min: Math.min(min, max), max };
}

/** Whether the spell lets the target roll to resist at all. */
export const mayResist = (s: SpellRec, antiMagic: boolean) => s.resmode === 2 || (s.resmode === 1 && antiMagic);

/** The player's chance to resist a monster's cast: half the magic resistance, at most 97. */
export const resistChance = (mr: number) => Math.min(div(mr, 2), 97);

/** The element resistance a player has to the spell, in percent (`element_resistance`). */
export function elementResist(s: SpellRec, sources: readonly Source[]): number {
  if (s.type > 2) return 0;
  const ab = { 0: A.coldDefence, 1: A.hotDefence, 2: A.resistStone, 3: A.resistLightning, 5: A.resistWater, 6: A.poisonImmune }[s.element];
  return ab ? abilityValue(sources, ab) : 0;
}

/**
 * "Inflict Damage on Armour" on a player after magic resistance (`mr_damage`): the cut is half
 * of what is over 50, at most 50 (75 with Anti Magic, half of all of it). (sic) The test is
 * unsigned 16-bit, so below 50 it wraps and the cut is the full 50.
 */
export function mrDamage(amount: number, mr: number, antiMagic: boolean): number {
  const m = u16(mr);
  let cut: number;
  if (antiMagic) cut = Math.min(75, Math.max(0, i16(div(m, 2))));
  else {
    const above = u16(m - 50);
    cut = Math.min(50, above > 0 ? i16(div(above, 2)) : 0);
  }
  if (cut === 0) return antiMagic ? amount : div(amount * i16(u16(50 - m)), 100) + amount;
  return amount - div(amount * cut, 100);
}

/** The target kinds a monster's attack casts at the one player (`single_kind`): Single, Self or
 * Single, Monster or Single, Any. The others are area casts, which reach everyone in the room. */
export const singleTarget = (target: number) => target === 0 || target === 2 || target === 6 || target === 8;

// ---- experience --------------------------------------------------------------------------

/** The growth factors of the first 26 steps of the experience table. */
const EXP_TABLE: [number, number][] = [
  [1, 1], [40, 20], [44, 24], [44, 24], [48, 28], [48, 28], [52, 32], [52, 32], [56, 36], [56, 36], [60, 40], [60, 40], [65, 45],
  [65, 45], [70, 50], [70, 50], [75, 55], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [50, 40], [23, 20],
];
const W32 = 2 ** 32;
const BILLION = 1_000_000_000;

/**
 * The experience a character must have to be `level` (`exp_needed_for_level`), for the race's
 * and class's experience percentages added (`modifier`): 1.11's billions-and-remainder table,
 * with its 32-bit rescaling by hundreds and (sic) the billions growing by numer/1000.
 */
export function expNeeded(level: number, modifier: number): number {
  if (level <= 1) return 0;
  let rem = u32(div((modifier + 100) * 1000, 100));
  let nbil = 0;
  for (let i = 0; i < level - 1; i++) {
    const [numer, denom] = i < 26 ? EXP_TABLE[i]! : i < 54 ? [115, 100] : i < 57 ? [109, 100] : [108, 100];
    let product = (rem * numer) % W32;
    if (product >= rem && Math.floor(product / numer) === rem) rem = Math.floor(product / denom);
    else {
      let scales = 0;
      while (product < rem || Math.floor(product / numer) !== rem) {
        scales++;
        rem = Math.floor(rem / 100);
        product = (rem * numer) % W32;
      }
      rem = scales <= 1 ? Math.floor(product / denom) : Math.floor((((rem * numer) % W32) * 100) % W32 / denom);
      for (let k = 0; k < scales; k++) rem = (rem * 100) % W32;
    }
    let carry = (((nbil * numer) % W32) * 1_000_000) % W32;
    while (carry >= BILLION) {
      carry -= BILLION;
      nbil++;
    }
    let total = (rem + carry) % W32;
    while (total >= BILLION) {
      total -= BILLION;
      nbil++;
    }
    rem = total;
  }
  return i16(nbil) * BILLION + rem;
}
