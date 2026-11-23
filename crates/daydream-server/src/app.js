// Built from web/src by `npm run build` in web/. Do not edit.

// src/abilities.ts
var ABILITIES = {
  0: ["empty", 'The "nothing" attribute. New objects automatically have their attributes filled with this.'],
  1: ["Damage", "Does damage in the amount specified by the parameter. MR is not taken into account."],
  2: ["AC", "Modifies the AC value specified by the parameter, this AC is NOT affected by the targets weight. Used in barkskin and stoneskin."],
  3: ["Rcol", "Modifies resistance to cold by the value specified by the parameter."],
  4: ["Max Damage", "Modifies max damage to the value specified by the parameter."],
  5: ["Rfir", "Modifies resistance to fire by the value specified by the parameter."],
  6: ["Enslave", "Charms the target."],
  7: ["DR", "Modifies the DR value specified by the parameter. Note that DR is expressed in multiples of 10."],
  8: ["Drain", "Drains HP from the target to the caster. (usually seen in spells)"],
  9: ["Shadow", 'Displays "A shadowy figure!" when looked at by other players.'],
  10: ["AC(Blur)", "Modifies the AC value specified by the parameter, this AC IS affected by the targets encumberance. Used in blur, and the spell for the amethyst pendant."],
  11: ["EnergyLevel", "Modifies the Energy level specified by the parameter."],
  12: ["Summon", "Summons a monster specified by the parameter. The value in parameter is the monster number."],
  13: ["Illu", "Increases or decreases the light value for the target ONLY by the value specified in the parameter."],
  14: ["RoomIllu", "Increases or decreases the light value for the entire room by the value specified in the parameter."],
  15: ["Alterhunger", "Unknown"],
  16: ["Alter thirst", "Unknown"],
  17: ["Damage(-MR)", "Does damage which takes into account the targets MR (or so the name implies)."],
  18: ["Heal", "Heals damage specified by the value in the parameter."],
  19: ["Poison", "Poisons the target doing damage specified by the value in the parameter."],
  20: ["Cure Poison", "Cures poison (cure poison has a parameter of 8, as does antidote)."],
  21: ["ImmuPoison", "Used in duration spells to make the target immune to poison."],
  22: ["Accuracy", "Increases the targets accuracy by the value specified in the parameter. (bless increases this value by 3, divine favour increases it by 10)"],
  23: ["AffectsUndead", "Used to make a spell ONLY work on undead monsters."],
  24: ["ProtEvil", "Protection from evil."],
  25: ["ProtGood", "Protection from good."],
  26: ["DetectMagic", "Detects the magic in an item."],
  27: ["Stealth", "Modifies stealth by the value in the parameter."],
  28: ["Magical", "Defines the magical potency of an item. Witchunters cannot use items with this flag. To make a non-magical item hit highly magical creatures, use the HitMagic flag."],
  29: ["Punch", "Used in the Mystic class definition to grant the Punch ability. A parameter of +1 is used in their definition."],
  30: ["Kick", "Used in the Mystic class definition to grant the Kick ability. A parameter of +1 is used in their definition."],
  31: ["Bash", "Used in almost all class definition to grant the ability to bash attack monsters."],
  32: ["Smash", "Unknown"],
  33: ["Killblow", "Unknown"],
  34: ["Dodge", "Increases or decreases the targets ability to Dodge by the % value specified in the parameter."],
  35: ["JumpKick", "Used in the Mystic class definition to grant the JumpKick ability. A parameter of +1 is used in their definition."],
  36: ["M.R.", "Increases or decreases the targets MR by the value specified in the parameter."],
  37: ["Picklocks", "In Race and Class definitions, this grants the ability to Picklock, in items, this can be used to enhance the ability."],
  38: ["Tracking", "Used in Race and Class definitions to grant Tracking to a Race/Class."],
  39: ["Thievery", "In Race and Class definitions, this grants the ability to Rob, in items, this can be used to enhance the ability."],
  40: ["FindTraps", "In Race and Class definitions, this grants the ability to find traps, in items, this can be used to enhance the ability."],
  41: ["DisarmTraps", "In Race and Class definitions, this grants the ability to disarm traps, in items, this can be used to enhance the ability."],
  42: ["LearnSp", "Places the spell specified in the parameter into the players spells list. Also used for Kai powers and Bard songs. Use this in a\nscroll item. The parameter is the spell number."],
  43: ["CastsSp", "Casts the spell when an item is 'used', or, with the %Spell attribute, this is the spell that is cast when the % chance is met."],
  44: ["Intel", "Increases or decreases the Intel stat by the value specified in the parameter."],
  45: ["Wisdom", "Increases or decreases the Wisdom/Willpower stat by the value specified in the parameter."],
  46: ["Strength", "Increases or decreases the Strength stat by the value specified in the parameter."],
  47: ["Health", "Increases or decreases the Health stat by the value specified in the parameter."],
  48: ["Agility", "Increases or decreases the Agility stat by the value specified in the parameter."],
  49: ["Charm", "Increases or decreases the Charm stat by the value specified in the parameter."],
  50: ["MageBaneQuest", "This Determines if the player has completed the MageBane quest."],
  51: ["AntiMagic", "Used in the Witchunter class definition, apperently the only effect being that MR effects beneficial as well as combat spells cast on the target."],
  52: ["EvilInCombat", "Used in spells to generate evil warnings and give EP when used."],
  53: ["BlindingLight", "Unknown"],
  54: ["IlluTarget", "Increases or decreases the light level for the target specified by the value in the parameter."],
  55: ["AlterGeneralLightDuration", "Unknown"],
  56: ["RechargeItem", "Unknown"],
  57: ["SeeHidden", "Grants the ability to see hidden players. For monsters, this allows them to see hidden players and attack them while they are sneaking or hiding. The monster will also follow even if the player escapes and re-sneaks."],
  58: ["Crits", "Increases or decreases the targets critical hit chance by the value specified in the parameter."],
  59: ["ClassOk", "Used in items to make the item equippable by classes they aren't intended to be used with."],
  60: ["Fear", "Makes the target flee in fear. Used in duration spells."],
  61: ["AffectExit", "Unknown"],
  62: ["EvilChance", "Unknown"],
  63: ["Experience", "Unknown (although one would assume this can be used to add/remove experience)"],
  64: ["AddCP", "Unknown (although one would assume this can be used to add/remove CP)"],
  65: ["ResistStone", "Modifies resistance to stone by the value specified by the parameter."],
  66: ["Rlit", "Modifies resistance to lightning by the value specified by the parameter."],
  68: ["Slowness", 'Modifies the room movement time while not affecting stealth as a modified encumbrance would. The Mage spell "Slow" uses a parameter of +200 for this attribute.'],
  69: ["MaxMana", "Increases or decreases the targets maximum mana by the value specified in the parameter."],
  70: ["S.C.", "Incresaes or decreases the targets SC by the value specified in the parameter."],
  71: ["Confusion", 'Confuses the target, making them fumble in the % specified by the parameter. A parameter of +50 would make them fumble 50% of the time-- most "stun" spells use this with a value of +100, which makes the target fumble 100% of the time.'],
  72: ["DamageShield", "Used to setup a shockshield for the player. Shockshield uses this."],
  74: ["HoldPerson", "Holds the target in the room. Used with duration spells. Entangle and Hold Person use this attribute."],
  75: ["Paralyze", "Unknown"],
  76: ["Mute", "Strikes the target mute, making them unable to talk in rooms, etc."],
  77: ["Percep", "Increases or decreases the targets perception by the value specified in the parameter."],
  78: ["Animal", 'Used in monster attributes to flag a monster as an animal. See "Affects Animals".'],
  79: ["MageBind", "did use to make spellcasting impossible but not sure it's still working."],
  80: ["AffectsAnimals", "Used in spells to make it ONLY affect animals."],
  81: ["Freedom", "Frees the target from holding spells and spells that alter the targets speed."],
  82: ["Cursed", "Used on items, once worn, a Cursed item can't be removed."],
  84: ["Rcrs", 'Remove curse. Seen in spells. The Priest spell "Remove Curse" uses this flag. Removes all cursed items.'],
  85: ["Shatter", 'Shatters the targets weapon. The parameter (in theory) specifies which "Quality" of weapons the spell affects and can shatter. The Druid sell "Shatter" specifies a parameter of 1, and thus will not work on items with a Quality of or greater than 1.'],
  86: ["Quality", 'Specifies an items "Quality". The only effect of this is to counter "Shatter". See "Shatter".'],
  87: ["Speed", 'Increases or decreases the targets swing speed with their weapon or attack by the value specified in the parameter. The Mage spell "Speed" uses +85 for a parameter, the Mage spell "Slow" uses +125.'],
  88: ["Alter HP", "increases or decreases the targets max HP"],
  89: ["PunchACY", "believed to alter punch accuracy value"],
  92: ["PunchDmg", "Increases punch damage for the Mystic punch attack."],
  93: ["KickDmg", "Increases kick damage for the Mystic kick attack."],
  94: ["JumpKDmg", "Increases jumpkick damage for the Mystic jumpkick attack."],
  95: ["Slay", 'Unknown. Used by the "laen longsword" item.'],
  96: ["Encum", "Increases or decreases the targets encumberance % by the value specified in the parameter."],
  97: ["Good", "In spells and items, makes the spell/item useable only by Good aligned characters."],
  98: ["Evil", "In spells and items, makes the spell/item useable only by Evil aligned characters."],
  100: ["LoyalItem", "Makes the item stay with the player even into death. Seen in items."],
  101: ["ConfuseMsg", 'Sets the confusion message for a previous "Confusion" attribute. The parameter specifies the message number to use for when the player is confused and fumbles. For the Mage spell "confusion", the message is along the lines of "You fumble in confusion!".'],
  102: ["RaceStealth", "Used in Race definitions to grant racial stealth."],
  103: ["ClassStealth", "Used in Class definitions to grant class stealth."],
  104: ["DefenseModifier", "Unknown"],
  105: ["Accuracy(2)", "Increases or decreases accuracy by the value specified in the parameter."],
  106: ["Accuracy (3)", "Increases or decreases accuracy by the value specified in the parameter."],
  107: ["BlindUser", "Blinds the target."],
  108: ["AffectsLiving", 'Used in spells to limit their abilities to only living players or monsters. See "NonLiving"'],
  109: ["NonLiving", 'Used in monster attributes to flag the monster as being Non Living which makes it impossible to cast spells flagged as AffectsLiving on them. See "AffectsLiving"'],
  110: ["NotGood", "Used in items/spells to flag the object as being useable by any alignment EXCEPT Good. (Neutral, Seedy, Outlaw, Criminal, Villain and FIEND can use it, but Good, Saint, Lawful cannot.)"],
  111: ["NotEvil", "Used in items/spells to flag the object as being useable by any alignment EXCEPT Evil. (Lawful, Saint, Good, Neutral and Seedy can use it, but Outlaw, Criminal, Villain and FIEND cannot.)"],
  112: ["Neutral", "In spells and items, makes the spell/item useable only by Neutral aligned characters."],
  113: ["NotNeutral", "Used in items/spells to flag the object as being useable by any alignment EXCEPT Neutral. (Lawful, Saint, Good, Outlaw, Criminal, Villain and FIEND can use it, but Neutral and Seedy cannot.)"],
  114: ["%Spell", 'Used in items to make a "spell effect" occur on a percent chance on each hit or crit. The spell cast is specified by the NEXT "CastsSp" attribute. See "CastsSp".'],
  115: ["DescMsg", "Used in spells. Specifies the message in WCCMSG.DAT to use when the spell wears off, when you are looked at, and when you type 'STAT' to see the effect listed."],
  116: ["BSAccu", "Used in items (although I can see no reason why it couldn't be used in a spell) to specify the backstab accuracy for the weapon. A positive value increases the chance to hit, a negative value decreases the chance to hit."],
  117: ["BsMinDmg", "Used in items (although I can see no reason why it couldn't be used in a spell) to specify a higher (or lower) minimum backstab damage for the weapon when armed."],
  118: ["BsMaxDmg", "Used in items (although I can see no reason why it couldn't be used in a spell) to specify a higher (or lower) maximum backstab damage for the weapon when armed."],
  119: ["Del@Maint", "Used in items to delete the item, hidden or not, at cleanup."],
  120: ["StartMsg", "Used in spells to specify the 'start message' for the spell.Basically, this sets the opening line for the spell before the Cast Message is sent."],
  121: ["Recharge", 'Used in items to specify the amount of "uses" to recharge at cleanup.'],
  122: ["RemovesSpell", "Used in spells to remove other spells (cancel them out)."],
  123: ["HPRegen", 'Used in any object to modify the targets HP regen rate. The Druid spell "Rapid Healing" and the module-1 quest rings use this attribute.'],
  124: ["NegateAbility", "Removes the ability number?"],
  125: ["IceSorcQuest", "This is the progress the player has made with the Ice Sorceress quest."],
  126: ["GoodQuest", "This is the progress the player has made with the GOOD quest."],
  127: ["NeutralQuest", "This is the progress the player has made with the NEUTRAL quest."],
  128: ["EvilQuest", "This is the progress the player has made with the EVIL quest."],
  129: ["DarkDruidQuest", "This is the progress the player has made with the Dark Druid quest."],
  130: ["BloodChampQuest", "This is the progress the player has made with the Blood Champion quest."],
  131: ["SheDragonQuest", "This is the progress the player has made with the SheDragon quest."],
  132: ["WereratQuest", "This is the progress the player has made with the Wererat quest."],
  133: ["PhoenixQuest", "This is the progress the player has made with the Phoenix feather quest."],
  134: ["DaoLordQuest", "This is the progress the player has made with the DaoLord quest."],
  135: ["MinLevel", "Typically seen in items (useless for spells since they have their own internal Minimum level field) to restrict them to players ONLY of the level specified or higher. The value in the parameter is the minimum level."],
  136: ["MaxLevel", "Typically seen in items to restrict them to players ONLY of the level specified or LOWER. The value in the parameter is the maximum level."],
  137: ["Shock", 'Sets the Shockshield message for the spell/item. See "DmgShield".'],
  138: ["RoomVisible", "Makes the item visible to the entire room."],
  139: ["SpellImmu", "Used in objects to make the target immune to spells below or equal to the level specified in the parameter."],
  140: ["TeleportRoom", 'Teleports the target to the specified room in the parameter. Usually seen with a "Teleport map". See "Teleport map".'],
  141: ["TeleportMap", 'Teleports the target to the specified map in the parameter. Usually seen with a "Teleport room". See "Teleport room".'],
  142: ["HitMagic", "Seen in weapons (and possibly could be used in spells) and used to make them hit magical creatures of the value specified in the parameter or LOWER. HitMagic +5 for example would let the target hit monsters with a magical attribute of 5 or lower."],
  143: ["ClearItem", "Poofs or unequips item number?"],
  144: ["NonMagicalSpell", "Flags the spell as non-magical, thus making it immune to MR."],
  145: ["ManaRgn", "Used in objects to increase or decrease the targets mana regen by the percent specified in the parameter. A value of +25 would increase the targets mana regen by 25%. A value of +100 would increase the targets mana regen by 100% (double)."],
  146: ["MonsGuards", "Used in monster definitions to define a monster that protects this one. The parameter points to an entry in the monster database that will protect this one. Multiple MonsGuards can be used to make the monster be protected by different monsters."],
  147: ["ResistWater", "Increases or decreases the targets resistance to Water by the value specified in the parameter."],
  148: ["TextBlock", "Executes the text block specified by the parameter."],
  149: ["Remove@Maint", "Used in items-- removes the item at cleanup, poofing it."],
  150: ["HealMana", "Regenerates mana back by the value specified in the parameter."],
  151: ["EndCast", "Used in spell definitions to chain to another spell at the end of the current ones cast. EndCast is only called if the first spell doesn't fail."],
  152: ["Rune", "Flags the object as a Rune. Seen in one item."],
  153: ["KillSpell", "Kills a spell without calling it's EndCast effect (if one exists)."],
  154: ["Visible@Maint", "Used in items to keep them visible at cleanup."],
  155: ["DeathText", "Unknown (executes the specified text block when the player dies?)"],
  156: ["QuestItem", "Makes the item a quest item"],
  157: ["ScatterItems", "Scatters the items in the room to the specified room(s). You can use a range to specify multiple rooms, but, these rooms must all be a part of the current map."],
  158: ["ReqToHit", "Monsters with this flag require that weapons (and spells?) with this IDENTICAL flag (including parameter) be used to attack it. NOTHING else will work on these monsters."],
  159: ["KaiBind", "Prevents the target from using Kai powers. Seen in 1.1u's race change to Half-Ogre's."],
  160: ["GiveTempSpell", "Unknown (gives the spell specified in Value/Parameter temporarily, as in, it appears in the spells/powers list? Maybe used in future items to give out spells only if a player has a specific item or has a specific item armed/equipped?)"],
  161: ["OpenDoor", "Unknown (opens a door, I assume, but how it determines direction/etc is unknown)"],
  162: ["Lore", "Unknown"],
  163: ["SpellComponent", "Make the item a spell component."],
  164: ["CastOnEnd%", "cast a spell specificed in the next EndCast ability when this spell ends only Value % chance of the time"],
  165: ["AlterSpDmg", "Unknown (alter spells damage (eg: used in an item or another spell to temporarily increase all spells damage)?)"],
  166: ["AlterSpLength", "Unknown (alter the length of duration spells (eg: used in an item or another spell to temporarily increase all future spells durations)?)"],
  167: ["UnEquipItem", "Unknown (remove either the specific item, or, perhaps the value is an item location? (head, arms, etc)?)"],
  168: ["EquipItem", "Unknown (equip either the specific item, or, perhaps the value is an item location? (head, arms, etc)? and the first elgible item found in the players inventory is used?)"],
  169: ["CannotWearLocation", "Unknown (perhaps used to make an item that can be worn on multiple locations, but, this flag is used to define WHICH locations it cannot be used on?)"],
  170: ["Sleep", "Unknown"],
  171: ["Invisibility", "Unknown (some new form of stealth perhaps that doesn't rely on Stealth?)"],
  172: ["SeeInvisible", "Unknown (most likely a way for monsters/players to see others with this new Invisible ability)"],
  173: ["Scry", "Unknown (perhaps show the room specificed in the value? No idea really)"],
  174: ["StealMana", "Unknown (most likely steals Mana from another Player or monster for the value specified (or min/max in a spell))"],
  175: ["StealHPtoMP", "Unknown (convert stolen HP to Mana instead? Works like the other 'Steal' abilities?)"],
  176: ["StealMPtoHP", "Unknown (convert stolen Mana to HP instead? Works like the other 'Steal' abilities?)"],
  177: ["SpellColours", "Unknown (perhaps a way to specific the ANSI colour coding for spells, overriding the default colours for the spell?)"],
  178: ["Shadowform", "This functions much like shadowform except the value states which text block to display when the player is looked at."],
  181: ["GHouseDeed", "Gang House Deed value"],
  182: ["GHouseTax", "Gang House Tax value"],
  183: ["GHouseItem", "Gang House Item"],
  184: ["GShopItem", "Gang Shop Item (controller)"],
  185: ["BadAttk", "Do not Attack if Item Nmbr"],
  186: ["PerStealth", "Perfect Stealth"],
  187: ["Meditate", "Meditate skill granted"],
  99: ["AlterDRpercent", "changes DR by the % in the value (is cast by some Mod 9 monsters with negative values.)"],
  83: ["CURSED", "has the same affects as 82 (curse) so you can't remove an item but it also makes it not drop at death (unlike 82).. although the item is still unequipped at death so it kinda doesn't work right."],
  73: ["DispellMagic", 'cancels spells that affect the ability in the value (ie dispellmagic +107 cancels spells giving the "blind" ability), but probably only certain ones'],
  67: ["Quickness", "does in fact make you walk faster (but only to a point so that it only matters if you're heavy.)"],
  179: ["FindTrapsValue", "alters user's Traps Value"],
  180: ["PickLocksValue", "alters user's PickLocks Value"],
  91: ["JumpKACY", "believed to alter jump accuracy value"],
  90: ["KickACY", "believed to alter kick accuracy value"]
};

// src/ansi.ts
var ESC_SHOWN = "\u241B";
var toShown = (s) => s.replaceAll("\x1B", ESC_SHOWN);
var toWire = (s) => s.replaceAll(ESC_SHOWN, "\x1B");
var COLOURS = [
  "#000000",
  "#aa0000",
  "#00aa00",
  "#aa5500",
  "#0000aa",
  "#aa00aa",
  "#00aaaa",
  "#aaaaaa",
  "#555555",
  "#ff5555",
  "#55ff55",
  "#ffff55",
  "#5555ff",
  "#ff55ff",
  "#55ffff",
  "#ffffff"
];
function renderAnsi(text) {
  const out = document.createDocumentFragment();
  const pen = { fg: 7, bg: 0, bold: false };
  const s = toWire(text);
  let run = "";
  const flush = () => {
    if (!run) return;
    const span = document.createElement("span");
    span.textContent = run;
    span.style.color = COLOURS[pen.fg + (pen.bold ? 8 : 0)] ?? COLOURS[7];
    if (pen.bg) span.style.backgroundColor = COLOURS[pen.bg] ?? "";
    out.append(span);
    run = "";
  };
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c !== "\x1B") {
      if (c !== "\r") run += c;
      continue;
    }
    const m = /^\[([0-9;]*)([A-Za-z])/.exec(s.slice(i + 1));
    if (!m) continue;
    i += m[0].length;
    if (m[2] !== "m") continue;
    flush();
    const codes = (m[1] || "0").split(";").map((x) => Number(x || 0));
    for (const code of codes) {
      if (code === 0) Object.assign(pen, { fg: 7, bg: 0, bold: false });
      else if (code === 1) pen.bold = true;
      else if (code === 22) pen.bold = false;
      else if (code >= 30 && code <= 37) pen.fg = code - 30;
      else if (code >= 40 && code <= 47) pen.bg = code - 40;
    }
  }
  flush();
  return out;
}
function titleOf(text) {
  const line = toWire(text).split(/[\0\r\n]/).find((l) => l.trim()) ?? "";
  const clean = line.replace(/\x1b\[[0-9;]*[A-Za-z]?/g, "").trim();
  return clean.length > 60 ? `${clean.slice(0, 60)}...` : clean;
}

// src/api.ts
var ApiError = class extends Error {
  constructor(status2, message) {
    super(message);
    this.status = status2;
  }
  status;
};
async function call(method, url, body) {
  const res = await fetch(url, {
    method,
    headers: body === void 0 ? {} : { "content-type": "application/json" },
    body: body === void 0 ? void 0 : JSON.stringify(body)
  });
  if (res.status === 401) {
    location.href = "/login";
  }
  const text = await res.text();
  let json = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
  }
  if (!res.ok) {
    const msg = json?.error ?? (text || res.statusText);
    throw new ApiError(res.status, msg);
  }
  return json;
}
var t = (table) => `/api/tables/${encodeURIComponent(table)}`;
var k = (key) => `?key=${encodeURIComponent(JSON.stringify(key))}`;
var api = {
  realm: () => call("GET", "/api/realm"),
  tables: () => call("GET", "/api/tables").then((r) => r.tables),
  rows: (table) => call("GET", `${t(table)}/rows`),
  /** Each row's key, title and the values of `columns`. */
  rowsWith: (table, columns) => call("GET", `${t(table)}/rows?columns=${encodeURIComponent(columns.join(","))}`),
  row: (table, key) => call("GET", `${t(table)}/row${k(key)}`),
  save: (table, key, values) => call("PUT", `${t(table)}/row${k(key)}`, { values }),
  insert: (table, values) => call("POST", `${t(table)}/row`, { values }),
  remove: (table, key) => call("DELETE", `${t(table)}/row${k(key)}`),
  /** One of NMR's fixes (`daydream_nmr::presets::Preset`), written straight away. */
  preset: (body) => call("POST", "/api/tools/preset", { ...body, dry_run: false })
};

// src/layouts.ts
var ITEM_TYPE = {
  0: "Armour",
  1: "Weapon",
  2: "Projectile",
  3: "Sign",
  4: "Food",
  5: "Drink",
  6: "Light",
  7: "Key",
  8: "Container",
  9: "Scroll",
  10: "Special"
};
var WORN = {
  0: "Nowhere",
  1: "Everywhere",
  2: "Head",
  3: "Hands",
  4: "Finger",
  5: "Feet",
  6: "Arms",
  7: "Back",
  8: "Neck",
  9: "Legs",
  10: "Waist",
  11: "Torso",
  12: "Off-Hand",
  13: "Finger",
  14: "Wrist",
  15: "Ears",
  16: "Worn",
  17: "Wrist",
  18: "Eyes",
  19: "Face"
};
var WEAPON = { 0: "1H Blunt", 1: "2H Blunt", 2: "1H Sharp", 3: "2H Sharp" };
var CLASS_WEAPON = {
  ...WEAPON,
  4: "Any 1H",
  5: "Any 2H",
  6: "Any Sharp",
  7: "Any Blunt",
  8: "Any Weapon",
  9: "Staff"
};
var ARMOUR = {
  0: "Natural",
  1: "Silk",
  2: "Ninja",
  3: "Leather",
  4: "Leather",
  5: "Leather",
  6: "Leather",
  7: "Chainmail",
  8: "Scalemail",
  9: "Platemail"
};
var COST_TYPE = { 0: "Copper", 1: "Silver", 2: "Gold", 3: "Platinum", 4: "Runic" };
var MAGERY = { 0: "None", 1: "Mage", 2: "Priest", 3: "Druid", 4: "Bard", 5: "Kai" };
var list = (...labels) => Object.fromEntries(labels.map((l, i) => [i, l]));
var CLASS_ARMOUR = list(
  "Natural",
  "Robes",
  "Padded",
  "Soft Leather",
  "Soft Studded Leather",
  "Rigid Leather",
  "Studded Rigid Leather",
  "Chainmail",
  "Scalemail",
  "Platemail"
);
var CLASS_WEAPONS = list(
  "1H Blunt",
  "2H Blunt",
  "1H Sharp",
  "2H Sharp",
  "Any 1H",
  "Any 2H",
  "Any Sharp",
  "Any Blunt",
  "All Weapons",
  "Limited (staff)"
);
var MONSTER_GROUP = list(
  "Lair",
  "Wanderer",
  "NPC",
  "Living",
  "Random",
  "Guard",
  ...Array.from({ length: 30 }, (_, i) => `Group ${i + 1}`),
  "Arena",
  "Angel",
  "Quest",
  "Other"
);
var MONSTER_TYPE = list("Solo", "Leader", "Follower", "Stationary");
var ALIGNMENT = list("Good", "Evil", "Chaotic Evil", "Neutral", "Lawful Good", "Neutral Evil", "Lawful Evil");
var GENDER = list("It", "He", "She");
var ATTACK_TYPE = list("None", "Normal", "Spell", "Rob");
var SPELL_TYPE = list("Auto-Combat", "Poison", "Change me", "General");
var TARGET = list(
  "Single",
  "Self",
  "Self or Single",
  "Divided Area Not You",
  "Monster",
  "Divided Area Incl. You",
  "Any",
  "Item",
  "Monster or Single",
  "Divided Attack Area",
  "Divided Party Area",
  "Full Area",
  "Full Attack Area",
  "Full Party Area"
);
var RESIST_ABILITY = list("Magic Resistance", "Strength", "Agility", "Health", "Intellect", "Wisdom", "Charm");
var ELEMENT = list("Cold", "Hot", "Stone", "Lightning", "Normal", "Water", "Poison");
var RESISTS = list("Never", "Anti-Magic Only", "Everyone");
var YES_NO = list("Yes", "No");
var ROOM_TYPE = list("Normal", "Shop", "Arena", "Lair", "Hotel", "Colliseum", "Jail", "Library");
var EXIT_TYPE = list(
  "Normal",
  "Spell",
  "Key",
  "Item",
  "Toll",
  "Action",
  "Hidden",
  "Door",
  "map Change",
  "Trap",
  "Text",
  "Gate",
  "Remote Action",
  "Class",
  "Race",
  "Level",
  "Timed",
  "Ticket",
  "User Count",
  "Block Guard",
  "Alignment",
  "Delay",
  "Cast",
  "Ability",
  "Spell Trap"
);
var EXITS = ["N", "S", "E", "W", "NE", "NW", "SE", "SW", "U", "D"];
var SHOP_TYPE = list(
  "General",
  "Weapons",
  "Armour",
  "Items",
  "Spells",
  "Hospital",
  "Tavern",
  "Bank",
  "Training",
  "Inn",
  "Specific",
  "Gang Shop",
  "Deed Shop"
);
var f = (col, label, pick2) => ({ col, label, pick: pick2 });
var wide = (col, label) => ({ col, label, wide: true });
var abilities = (from, to) => ({
  columns: [
    { template: "Ability #", label: "Ability", pick: { abilities: true } },
    { template: "Ability Value #", label: "Value" }
  ],
  from,
  to
});
var LAYOUTS = {
  // frmItem
  Items: [
    {
      title: "General",
      fields: [
        f("Number"),
        f("Name"),
        f("Type", "Type", { options: ITEM_TYPE }),
        f("Worn On", "Worn", { options: WORN }),
        f("Armour", "Armour", { options: ARMOUR }),
        f("Weapon", "Weapon", { options: WEAPON }),
        f("Cost"),
        f("Cost Type", "Cost Type", { options: COST_TYPE }),
        f("Game Limit"),
        f("Uses", "Max Uses"),
        f("Weight"),
        f("Min Hit"),
        f("Max Hit"),
        f("Accuracy"),
        f("Speed"),
        f("AC"),
        f("DR"),
        f("Req Str", "Req STR"),
        f("Gettable", "Getable"),
        f("Robable"),
        f("Not Droppable"),
        f("Destroy On Death", "Destroys On Death"),
        f("Retain After Uses", "Retain After Uses Expire")
      ]
    },
    {
      title: "Desc/Msg/Cash",
      fields: [
        wide("Desc1", "Description"),
        wide("Desc2", ""),
        wide("Desc3", ""),
        wide("Desc4", ""),
        wide("Desc5", ""),
        wide("Desc6", ""),
        f("Hit Msg", "Hit (msg)", { ref: "Messages" }),
        f("Miss Msg", "Miss (msg)", { ref: "Messages" }),
        f("Read Msg", "Read (tb)"),
        f("Distruct Msg", "Destrct (msg)", { ref: "Messages" }),
        f("OpenRunic", "Runic"),
        f("OpenPlatinum", "Platinum"),
        f("OpenGold", "Gold"),
        f("OpenSilver", "Silver"),
        f("OpenCopper", "Copper")
      ]
    },
    {
      title: "Race/Class/Negate",
      grid: {
        columns: [
          { template: "Class #", label: "Class Restrictions", pick: { ref: "Classes" } },
          { template: "Race #", label: "Race Restrictions", pick: { ref: "Races" } },
          { template: "Negate #", label: "Negate Spells", pick: { ref: "Spells" } }
        ],
        from: 0,
        to: 9
      }
    },
    { title: "Abilities p1", grid: abilities(0, 9) },
    { title: "Abilities p2", grid: abilities(10, 19) }
  ],
  // frmMonster
  Monsters: [
    {
      title: "General",
      fields: [
        f("Number"),
        f("Name"),
        f("Group", "Group", { options: MONSTER_GROUP }),
        f("Index"),
        f("Type", "Type", { options: MONSTER_TYPE }),
        f("Alignment", "Alignment", { options: ALIGNMENT }),
        f("Gender", "Gender", { options: GENDER }),
        f("Active"),
        f("Undead"),
        f("Experience"),
        f("Exp Multiplier", "Multiplier"),
        f("Hit Points", "Hitpoints"),
        f("HP Regen"),
        f("Energy"),
        f("AC"),
        f("DR"),
        f("MR", "M.R."),
        f("Follow", "Follow %"),
        f("Charm LvL", "Charm Lvl"),
        f("Charm RES", "Charm Resist"),
        f("BS Defense"),
        f("Game Limit"),
        f("Regen Time"),
        f("Date Killed", "Last Killed (date)"),
        f("Time Killed", "Last Killed (time)")
      ]
    },
    {
      title: "Drop",
      fields: [f("Runic"), f("Platinum"), f("Gold"), f("Silver"), f("Copper")],
      grid: {
        columns: [
          { template: "Item Number #", label: "Item", pick: { ref: "Items" } },
          { template: "Item Uses #", label: "Uses" },
          { template: "Item Drop % #", label: "%" }
        ],
        from: 0,
        to: 9
      }
    },
    {
      title: "Spells/Attacks",
      grid: {
        columns: [
          { template: "Attack Type #", label: "Type", pick: { options: ATTACK_TYPE } },
          { template: "Attack Accu/Spell #", label: "Accu/Spell" },
          { template: "Attack % #", label: "Attack%" },
          { template: "Attack Min Hit/Cast % #", label: "MinH/Cast%" },
          { template: "Attack Max Hit/Cast LVL #", label: "MaxH/CastLvL" },
          { template: "Attack Energy #", label: "Energy" },
          { template: "Attack Hit Msg #", label: "Hit Msg", pick: { ref: "Messages" } },
          { template: "Attack Dodge Msg #", label: "Dodge Msg", pick: { ref: "Messages" } },
          { template: "Attack Miss Msg #", label: "Miss Msg", pick: { ref: "Messages" } },
          { template: "Attack Hit Spell #", label: "Hit Spell", pick: { ref: "Spells" } }
        ],
        from: 0,
        to: 4,
        across: "Attack"
      }
    },
    {
      title: "Between-Round Spells",
      grid: {
        columns: [
          { template: "Spell Number #", label: "Spell", pick: { ref: "Spells" } },
          { template: "Spell Cast % #", label: "%" },
          { template: "Spell Cast LVL #", label: "LvL" }
        ],
        from: 0,
        to: 4
      }
    },
    {
      title: "Weapon/Txt/Msg",
      fields: [
        f("Weapon Number", "Weapon", { ref: "Items" }),
        f("Death Spell", "Death Spell", { ref: "Spells" }),
        f("Create Spell", "Create Spell", { ref: "Spells" }),
        f("Move Msg", "Move Msg", { ref: "Messages" }),
        f("Death Msg", "Death Msg", { ref: "Messages" }),
        f("Greet Txt"),
        f("Desc Txt"),
        f("Talk Txt"),
        wide("Desc 1", "Description"),
        wide("Desc 2", ""),
        wide("Desc 3", ""),
        wide("Desc 4", "")
      ]
    },
    { title: "Abilities", grid: abilities(0, 9) }
  ],
  // frmSpell
  Spells: [
    {
      title: "General",
      fields: [
        f("Number"),
        f("Name"),
        f("Short Name"),
        f("Level", "Req. Level"),
        f("Level Cap"),
        f("Magery A", "Magery", { options: MAGERY }),
        f("Magery B", "Magery Level"),
        f("Spell Type", "Spell Type", { options: SPELL_TYPE }),
        f("Target", "Target", { options: TARGET }),
        f("Attack Type", "Type of Attack", { options: ELEMENT }),
        f("Type of Resists", "Type of Resists", { options: RESISTS }),
        f("Resist Ability", "Resist Ability", { options: RESIST_ABILITY }),
        f("Difficulty"),
        f("Energy"),
        f("Mana"),
        f("Min", "Base Min"),
        f("Max", "Base Max"),
        f("Duration", "Base Duration"),
        f("Min Increase", "Min increase"),
        f("LVLS Min Increase", "LVLs/Min increase"),
        f("Max Increase", "Max increase"),
        f("LVLS Max Increase", "LVLs/Max increase"),
        f("Dur Increase", "Dur. increase"),
        f("LVLS Dur Increase", "LVLs/Dur. increase"),
        f("MSG Style", "Norm Msg Style", { options: YES_NO }),
        f("Cast MSG A", "Cast Msg A", { ref: "Messages" }),
        f("Cast MSG B", "Cast Msg B", { ref: "Messages" }),
        wide("Desc 1", "Description Line 1"),
        wide("Desc 2", "Description Line 2"),
        f("UNDEFINED01", "unknown 1"),
        f("UNDEFINED02", "unknown 2")
      ]
    },
    { title: "Abilities", grid: abilities(0, 9) }
  ],
  // frmClass
  Classes: [
    {
      title: "General",
      fields: [
        f("Number"),
        f("Name"),
        f("EXP %", "Exp %"),
        f("Min HP", "HP (min)"),
        f("Max HP", "HP (max)"),
        f("Armour", "Armour", { options: CLASS_ARMOUR }),
        f("Weapon", "Weapons", { options: CLASS_WEAPONS }),
        f("Combat"),
        f("Magic Type", "Magery", { options: MAGERY }),
        f("Magic LVL", "Magery Level"),
        f("Title Text")
      ]
    },
    { title: "Abilities", grid: abilities(0, 9) }
  ],
  // frmRace
  Races: [
    {
      title: "General",
      fields: [f("Number"), f("Name"), f("EXP %", "Exp %"), f("CP", "Starting Cp"), f("HP Bonus", "Hp Bonus")],
      matrix: {
        headers: ["Min", "Max"],
        rows: [
          { label: "Strength", cols: ["Min STR", "Max STR"] },
          { label: "Agility", cols: ["Min AGL", "Max AGL"] },
          { label: "Intellect", cols: ["Min INT", "Max INT"] },
          { label: "Health", cols: ["Min HEA", "Max HEA"] },
          { label: "Willpower", cols: ["Min WIL", "Max WIL"] },
          { label: "Charm", cols: ["Min CHM", "Max CHM"] }
        ]
      }
    },
    { title: "Abilities", grid: abilities(0, 9) }
  ],
  // frmShop
  Shops: [
    {
      title: "General",
      fields: [
        f("Number"),
        f("Name"),
        f("Type", "Type", { options: SHOP_TYPE }),
        f("Min Lvl", "Min LVL"),
        f("Max Lvl", "Max LVL"),
        f("MarkUp", "Markup %"),
        f("Class Limit", "Class", { ref: "Classes" }),
        wide("Desc A", "Description"),
        wide("Desc B", ""),
        wide("Desc C", "")
      ]
    },
    {
      title: "Items Sold",
      grid: {
        columns: [
          { template: "Item #", label: "Item", pick: { ref: "Items" } },
          { template: "Max #", label: "Max" },
          { template: "Normal #", label: "Normal" },
          { template: "Regen Time #", label: "Time" },
          { template: "Regen Number#", label: "Rgn#" },
          { template: "Regen %#", label: "Rgn%" }
        ],
        from: 0,
        to: 19
      }
    }
  ],
  // frmRoom
  Rooms: [
    {
      title: "General",
      fields: [
        f("Map Number"),
        f("Room Number"),
        wide("Name"),
        wide("Desc 0", "Description"),
        wide("Desc 1", ""),
        wide("Desc 2", ""),
        wide("Desc 3", ""),
        wide("Desc 4", ""),
        wide("Desc 5", ""),
        wide("Desc 6", ""),
        f("Type", "Type", { options: ROOM_TYPE }),
        f("Shop Number", "Shop", { ref: "Shops" }),
        f("Gang House Number", "Gang House"),
        f("Light"),
        f("AnsiMap", "ANSI Map"),
        f("Attributes")
      ]
    },
    {
      title: "Exits",
      grid: {
        columns: [
          { template: "Exit #", label: "To Room" },
          { template: "Type #", label: "Type", pick: { options: EXIT_TYPE } },
          { template: "Para1 #", label: "Para 1" },
          { template: "Para2 #", label: "Para 2" },
          { template: "Para3 #", label: "Para 3" },
          { template: "Para4 #", label: "Para 4" }
        ],
        from: 0,
        to: 9,
        names: EXITS
      }
    },
    {
      title: "NPC / Items / Coins",
      fields: [
        f("Perm NPC", "Perm NPC", { ref: "Monsters" }),
        f("Mon Type", "Monster Type", { options: MONSTER_GROUP }),
        f("Min Index"),
        f("Max Index"),
        f("Max Regen"),
        f("Max Area"),
        f("Runic"),
        f("Platinum"),
        f("Gold"),
        f("Silver"),
        f("Copper"),
        f("InvisRunic", "Hidden Runic"),
        f("InvisPlatinum", "Hidden Platinum"),
        f("InvisGold", "Hidden Gold"),
        f("InvisSilver", "Hidden Silver"),
        f("InvisCopper", "Hidden Copper")
      ],
      grid: { columns: [{ template: "Placed Item #", label: "Placed Item", pick: { ref: "Items" } }], from: 0, to: 9 }
    },
    {
      title: "Visible Items",
      grid: {
        columns: [
          { template: "Room Item #", label: "Item", pick: { ref: "Items" } },
          { template: "Room Item # USES", label: "Uses" },
          { template: "Room Item # QTY", label: "Qty" }
        ],
        from: 0,
        to: 16
      }
    },
    {
      title: "Hidden Items",
      grid: {
        columns: [
          { template: "Hidden Item #", label: "Item", pick: { ref: "Items" } },
          { template: "Hidden Item # USES", label: "Uses" },
          { template: "Hidden Item # QTY", label: "Qty" }
        ],
        from: 0,
        to: 14
      }
    },
    {
      title: "Other",
      fields: [
        f("Control Room"),
        f("Death Room"),
        f("Exit Room"),
        f("Command Text"),
        f("Delay"),
        f("Spell", "Spell", { ref: "Spells" })
      ]
    }
  ],
  // frmMessage
  Messages: [
    {
      title: "General",
      fields: [f("Number"), wide("Line 1"), wide("Line 2"), wide("Line 3")],
      preview: ["Line 1", "Line 2", "Line 3"]
    }
  ],
  // frmTextblock
  Textblocks: [
    {
      title: "General",
      fields: [f("Number"), f("Part #", "Part"), f("Link To", "Link To"), wide("Data", "Text")],
      preview: ["Data"]
    }
  ],
  // frmAction
  Actions: [
    {
      title: "General",
      fields: [
        f("Action"),
        wide("Single to User"),
        wide("Single to Room"),
        wide("User to User"),
        wide("User to Other User"),
        wide("User to Room"),
        wide("Monster to User"),
        wide("Monster to Room"),
        wide("Inventory to User"),
        wide("Inventory to Room"),
        wide("Floor Item to User"),
        wide("Floor Item to Room")
      ]
    }
  ],
  // frmUser: NMR's tabs, the Misc tab's spells cast on a tab of their own
  Users: [
    {
      title: "Stats",
      fields: [
        f("BBS Name"),
        f("First Name"),
        f("Last Name"),
        f("Title"),
        f("Race", "Race", { ref: "Races" }),
        f("Class", "Class", { ref: "Classes" }),
        f("Level"),
        f("Exp Billions", "Exp (billions)"),
        f("Exp"),
        f("Lives"),
        f("CP"),
        f("Total CP"),
        f("Max HP"),
        f("HP", "Current HP"),
        f("Max Mana"),
        f("Mana", "Current Mana"),
        f("HP Bonus"),
        f("Mana Bonus"),
        f("Max MR", "Max M.R."),
        f("MR", "M.R."),
        f("Max Energy"),
        f("Energy"),
        f("Max Weight", "Max Enc."),
        f("Weight", "Current Enc.")
      ],
      matrix: {
        headers: ["Base", "Current"],
        rows: ["INT", "WIL", "STR", "HEA", "AGL", "CHM"].map((s) => ({ label: s, cols: [`Base ${s}`, s] }))
      }
    },
    {
      title: "Inven.",
      fields: [f("Weapon", "Weapon (readied)", { ref: "Items" })],
      grid: {
        columns: [
          { template: "Item #", label: "Item", pick: { ref: "Items" } },
          { template: "Item # Uses", label: "Uses" }
        ],
        from: 0,
        to: 99
      }
    },
    {
      title: "Keys",
      grid: {
        columns: [
          { template: "Key #", label: "Key", pick: { ref: "Items" } },
          { template: "Key # Uses", label: "Uses" }
        ],
        from: 0,
        to: 49
      }
    },
    {
      title: "Spellbk.",
      grid: { columns: [{ template: "Spellbook #", label: "Spell", pick: { ref: "Spells" } }], from: 0, to: 99 }
    },
    {
      title: "Abilities",
      grid: {
        columns: [
          { template: "Ability #", label: "Ability", pick: { abilities: true } },
          { template: "Ability # Value", label: "Value" }
        ],
        from: 0,
        to: 29
      }
    },
    {
      title: "Rooms",
      fields: [f("Map Number", "Current Map"), f("Room Number", "Current Room")],
      grid: {
        columns: [
          { template: "Trail # Map", label: "Map" },
          { template: "Trail # Room", label: "Room" }
        ],
        from: 0,
        to: 19
      }
    },
    {
      title: "Worn",
      grid: {
        columns: [
          { template: "Worn #", label: "Item", pick: { ref: "Items" } },
          { template: "Worn # Uses", label: "Uses" }
        ],
        from: 0,
        to: 19
      }
    },
    {
      title: "Misc",
      fields: [
        f("Runic"),
        f("Platinum"),
        f("Gold"),
        f("Silver"),
        f("Copper"),
        f("Perception"),
        f("Stealth"),
        f("Martial Arts"),
        f("Thievery"),
        f("Spellcasting"),
        f("Traps"),
        f("Disarm"),
        f("Picklocks"),
        f("Tracking"),
        f("Evil Points"),
        f("Broadcast Channel", "BC Channel"),
        f("Gang"),
        f("Suicide Password")
      ]
    },
    {
      title: "Spells Cast",
      fields: [f("Poison", "Poison flag")],
      grid: {
        columns: [
          { template: "Spell Cast #", label: "Spell", pick: { ref: "Spells" } },
          { template: "Spell Cast # Value", label: "Value" },
          { template: "Spell Cast # Rounds", label: "Rounds Left" }
        ],
        from: 0,
        to: 9
      }
    }
  ],
  // frmBank
  BankBooks: [{ title: "General", fields: [f("BBS Name"), f("Shop #", "Bank (shop)", { ref: "Shops" }), f("Amount"), f("Loan")] }],
  // frmGang
  Gangs: [
    {
      title: "General",
      fields: [
        f("KeyName"),
        f("Display Name"),
        f("Leader"),
        f("Base Exp"),
        f("Roll Over Exp"),
        f("Times Rolled"),
        f("Members"),
        f("Date Created")
      ]
    }
  ]
};
function families(t2) {
  const single = [];
  const byTemplate = /* @__PURE__ */ new Map();
  for (const c of t2.columns) {
    const m = /^(.*?)(\d+)(\D*)$/.exec(c.name);
    if (!m || t2.key.includes(c.name)) {
      single.push(c.name);
      continue;
    }
    const template = `${m[1]}#${m[3]}`;
    const list2 = byTemplate.get(template) ?? [];
    list2.push(Number(m[2]));
    byTemplate.set(template, list2);
  }
  const grids = /* @__PURE__ */ new Map();
  for (const [template, numbers] of byTemplate) {
    if (numbers.length < 3) {
      for (const n of numbers) single.push(template.replace("#", String(n)));
      continue;
    }
    const from = Math.min(...numbers);
    const to = Math.max(...numbers);
    const id = `${from}-${to}`;
    const g = grids.get(id) ?? { columns: [], from, to };
    g.columns.push({
      template,
      label: template.replace("#", "").replace(/\s+/g, " ").trim(),
      pick: /^Ability #$/.test(template) ? { abilities: true } : void 0
    });
    grids.set(id, g);
  }
  const order = new Map(t2.columns.map((c, i) => [c.name, i]));
  single.sort((a, b) => (order.get(a) ?? 0) - (order.get(b) ?? 0));
  return { single, grids: [...grids.values()] };
}
function defaultLayout(t2) {
  const { single, grids } = families(t2);
  const tabs = [
    {
      title: "General",
      fields: single.map((col) => {
        const info = t2.columns.find((c) => c.name === col);
        return { col, wide: info?.type === "memo" || (info?.size ?? 0) > 40 };
      })
    }
  ];
  for (const g of grids) {
    const first = g.columns[0]?.label ?? "List";
    tabs.push({ title: g.columns.length > 2 ? `${first} ...` : g.columns.map((c) => c.label).join(" / "), grid: g });
  }
  return tabs;
}
function layoutFor(t2) {
  return LAYOUTS[t2.name] ?? defaultLayout(t2);
}
function columnsShown(layout) {
  const shown = /* @__PURE__ */ new Set();
  for (const tab of layout) {
    for (const field2 of tab.fields ?? []) shown.add(field2.col);
    for (const row of tab.matrix?.rows ?? []) for (const c of row.cols) shown.add(c);
    if (tab.grid) {
      for (let n = tab.grid.from; n <= tab.grid.to; n++) {
        for (const c of tab.grid.columns) shown.add(c.template.replace("#", String(n)));
      }
    }
  }
  return shown;
}

// src/quests.ts
var field = (line, n) => line.split("|")[n - 1] ?? "";
var val = (s) => {
  const m = /^\s*[-+]?\d+/.exec(s);
  return m ? Number(m[0]) : 0;
};
function parseQuests(text) {
  const root = { kind: "section", text: "Quests", children: [] };
  let section = root;
  let sub = root;
  let working = root;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw;
    if (line === "" || line.startsWith("#")) continue;
    if (line.startsWith("---")) {
      working = { kind: "section", text: line.slice(3), children: [] };
      sub.children.push(working);
    } else if (line.startsWith("--")) {
      sub = working = { kind: "section", text: line.slice(2), children: [] };
      section.children.push(sub);
    } else if (line.startsWith("-")) {
      section = sub = working = { kind: "section", text: line.slice(1), children: [] };
      root.children.push(section);
    } else if (line.startsWith("'")) {
      working.children.push({ kind: "comment", text: field(line, 1).slice(1), children: [] });
    } else {
      working.children.push({
        kind: "line",
        text: `${field(line, 1)} - p${field(line, 2)} --> ${field(line, 3)}`,
        children: [],
        block: val(field(line, 1)),
        part: val(field(line, 2)),
        link: val(field(line, 3)),
        description: field(line, 4)
      });
    }
  }
  return root;
}
function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
var editLink = (block, part, label) => h("a", { href: `#/Textblocks/${encodeURIComponent(JSON.stringify([block, part]))}` }, label);
async function textblock(block, part) {
  const key = encodeURIComponent(JSON.stringify([block, part]));
  const res = await fetch(`/api/tables/Textblocks/row?key=${key}`);
  if (!res.ok) return null;
  const row = await res.json();
  const data = row?.values?.Data;
  return typeof data === "string" ? data.split("\0")[0] : null;
}
function questOrganizer() {
  const which = h("select", { "aria-label": "Quest file" }, h("option", { value: "default" }, "Default"), h("option", { value: "custom" }, "Custom"));
  const load = h("button", { type: "button", class: "primary" }, "Load");
  const pick2 = h("input", { type: "file", accept: ".txt,text/plain", "aria-label": "Open a quest file" });
  const tree = h("div", { class: "quest-tree" });
  const detail = h("div", { class: "quest-detail" });
  const out = h("div", { class: "tool-result" });
  const show2 = (text, from) => {
    const root = parseQuests(text);
    tree.replaceChildren(h("p", { class: "note" }, from), branch(root, true));
    detail.replaceChildren(h("p", { class: "note" }, "Pick a line to see its text block."));
    out.replaceChildren(h("div", { class: "quest-body" }, tree, detail));
  };
  function branch(node, open = false) {
    if (node.kind === "section") {
      const d = h("details", { class: "quest-section" }, h("summary", {}, node.text));
      if (open) d.open = true;
      d.append(...node.children.map((c) => branch(c)));
      return d;
    }
    const b = h("button", { type: "button", class: `quest-${node.kind}` }, node.kind === "comment" ? `${node.text.slice(0, 40)}...` : node.text);
    b.addEventListener("click", () => {
      tree.querySelectorAll(".picked").forEach((x) => x.classList.remove("picked"));
      b.classList.add("picked");
      void pickNode(node);
    });
    return b;
  }
  async function pickNode(node) {
    if (node.kind === "comment") {
      detail.replaceChildren(h("p", {}, node.text));
      return;
    }
    const { block = 0, part = 0, link: link2 = 0 } = node;
    const facts = h(
      "dl",
      {},
      h("dt", {}, "Text block"),
      h("dd", {}, editLink(block, part, `${block}, part ${part}`)),
      h("dt", {}, "Links to"),
      h("dd", {}, link2 ? editLink(link2, 0, String(link2)) : "0")
    );
    const box3 = h("pre", { class: "terminal", "aria-label": "Text block" }, "Loading\u2026");
    detail.replaceChildren(h("p", {}, node.description ?? ""), facts, box3);
    const data = await textblock(block, part);
    box3.replaceChildren(data === null ? `Text block ${block} part ${part} is not in the realm.` : renderAnsi(data));
  }
  load.addEventListener("click", async () => {
    out.replaceChildren(h("p", {}, "Loading\u2026"));
    try {
      const res = await fetch(`/api/quests?file=${which.value}`);
      const body = await res.json().catch(() => null);
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      show2(body.text, `${body.name}, from the realm's folder`);
    } catch (e) {
      out.replaceChildren(h("p", { class: "error" }, e instanceof Error ? e.message : String(e)));
    }
  });
  pick2.addEventListener("change", async () => {
    const f2 = pick2.files?.[0];
    if (!f2) return;
    const text = new TextDecoder("windows-1252").decode(await f2.arrayBuffer());
    show2(text, `${f2.name}, from this computer`);
  });
  return h(
    "section",
    { class: "tool" },
    h("h2", {}, "Quest Organizer"),
    h(
      "p",
      { class: "hint" },
      "NMR's quest notes: which text blocks make up each quest. Load NMR-Quests.txt or NMR-QuestsCustom.txt from the realm's folder, or open a quest file from this computer."
    ),
    h("div", { class: "toolbar" }, h("label", {}, "File ", which), load, h("label", {}, "or open ", pick2)),
    out
  );
}

// src/lists.ts
function h2(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
async function get(url) {
  const res = await fetch(url);
  const text = await res.text();
  let parsed = null;
  try {
    parsed = JSON.parse(text);
  } catch {
  }
  if (!res.ok) throw new Error(parsed?.error ?? (text || res.statusText));
  return parsed;
}
var link = (table, key, label) => h2("a", { href: `#/${table}/${encodeURIComponent(JSON.stringify(key))}` }, label);
var roomLink = (map, room, label = `${map}/${room}`) => link("Rooms", [map, room], label);
function placeLink(p) {
  if (p.kind === "room" || p.kind === "hidden") {
    const [map, room] = p.key.split("/").map(Number);
    return roomLink(map, room, `${p.kind === "hidden" ? "Room (hidden)" : "Room"} ${p.key}`);
  }
  if (p.kind === "shop") return link("Shops", [Number(p.key)], `Shop ${p.key}`);
  return document.createTextNode(`User ${p.name}`);
}
function grid(head, rows, rowClass) {
  const body = h2("tbody");
  rows.forEach((r, i) => body.append(h2("tr", { class: rowClass?.(i) }, ...r.map((c) => h2("td", {}, c)))));
  return h2("table", { class: "grid list" }, h2("thead", {}, h2("tr", {}, ...head.map((c) => h2("th", {}, c)))), body);
}
function joined(nodes) {
  return nodes.flatMap((n, i) => i ? [document.createTextNode(", "), n] : [n]);
}
function toCsv(table) {
  const quote = (t2) => /[",\n\r]/.test(t2) ? `"${t2.replace(/"/g, '""')}"` : t2;
  return [...table.rows].map((row) => [...row.cells].map((c) => quote((c.textContent ?? "").trim())).join(",")).join("\r\n") + "\r\n";
}
function csvButton(title, table) {
  const b = h2("button", { type: "button", class: "csv" }, "Download CSV");
  b.addEventListener("click", () => {
    const url = URL.createObjectURL(new Blob([toCsv(table)], { type: "text/csv" }));
    const a = h2("a", { href: url, download: `${title.replace(/[^A-Za-z0-9]+/g, "-").replace(/^-|-$/g, "")}.csv` });
    document.body.append(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  });
  return h2("div", { class: "toolbar" }, b);
}
function card(title, hint, controls, build) {
  const out = h2("div", { class: "tool-result" });
  const go = h2("button", { type: "button", class: "primary" }, "Build List");
  go.addEventListener("click", async () => {
    out.replaceChildren(h2("p", {}, "Building\u2026"));
    try {
      const built = await build();
      out.replaceChildren(built);
      const table = built.querySelector("table") ?? (built.tagName === "TABLE" ? built : null);
      if (table) out.prepend(csvButton(title, table));
    } catch (e) {
      out.replaceChildren(h2("p", { class: "error" }, e instanceof Error ? e.message : String(e)));
    }
  });
  return h2("section", { class: "tool" }, h2("h2", {}, title), h2("p", { class: "hint" }, hint), h2("div", { class: "toolbar" }, ...controls, go), out);
}
function scanBoxes() {
  const boxes = ["Users", "Shops", "Rooms"].map((name) => {
    const i = h2("input", { type: "checkbox", checked: "" });
    return [h2("label", {}, i, ` Scan ${name}`), i, name.toLowerCase()];
  });
  return [boxes.map(([l]) => l), () => boxes.map(([, i, n]) => `${n}=${i.checked}`).join("&")];
}
function limitedItems() {
  const [boxes, query] = scanBoxes();
  return card(
    "Limited Items List",
    "Every limited item, and where its copies are. Items with more copies than their limit are in red.",
    boxes,
    async () => {
      const res = await get(`/api/reports/limited-items?${query()}`);
      const over = res.items.filter((i) => i.over).length;
      return h2(
        "div",
        {},
        h2("p", {}, `${res.items.length} limited items; ${over} over their limit.`),
        grid(
          ["Number", "Name (limit)", "In use", "Where"],
          res.items.map((i) => [
            link("Items", [i.number], String(i.number)),
            i.limit > 1 ? `${i.name} (${i.limit})` : i.name,
            String(i.in_use),
            h2("span", {}, ...joined(i.places.map((p) => h2("span", {}, placeLink(p), ` (${p.count})`))))
          ]),
          (n) => res.items[n].over ? "over" : void 0
        )
      );
    }
  );
}
function findItem() {
  const [boxes, query] = scanBoxes();
  const item = h2("input", { type: "text", inputmode: "numeric", class: "number" });
  return card("Find an Item", "Every room, shop and user that has the item.", [h2("label", {}, "Item to Find: ", item), ...boxes], async () => {
    const n = Number(item.value.trim());
    if (!Number.isInteger(n) || n <= 0) throw new Error("Give the item number.");
    const res = await get(`/api/reports/find-item?item=${n}&${query()}`);
    if (!res.places.length) return h2("p", {}, `Item ${n} is nowhere.`);
    return grid(
      ["Where", "Name", "Count"],
      res.places.map((p) => [placeLink(p), p.name, String(p.count)])
    );
  });
}
function controlRooms() {
  return card("Control Room List", "Each control room, and the rooms on its map that name it.", [], async () => {
    const res = await get("/api/reports/control-rooms");
    return grid(
      ["Control Room", "Name", "References"],
      res.rooms.map((c) => [
        roomLink(c.map, c.room),
        c.name || "(no such room)",
        h2("span", {}, `${c.rooms.length}: `, ...joined(c.rooms.map((r) => roomLink(c.map, r, String(r)))))
      ])
    );
  });
}
function monsterGroups() {
  return card("Monster Group / Index List", "The monsters of each group and index (at most 15 each, as NMR lists).", [], async () => {
    const res = await get("/api/reports/monster-groups");
    return grid(
      ["Group/Index", "Monsters"],
      res.groups.map((g) => [
        `${MONSTER_GROUP[g.group] ?? g.group}/${g.index}`,
        h2("span", {}, ...joined(g.monsters.map(([n, name]) => link("Monsters", [n], `${name} (${n})`))))
      ])
    );
  });
}
function npcRooms() {
  return card("NPC / Room List", "Each room with a permanent NPC, by the monster's name.", [], async () => {
    const res = await get("/api/reports/npc-rooms");
    return grid(
      ["Room", "Room Name", "Monster"],
      res.rooms.map((r) => [roomLink(r.map, r.room), r.room_name, link("Monsters", [r.monster], `${r.monster_name} (${r.monster})`)])
    );
  });
}
function listsPage() {
  return h2("div", { class: "tools" }, limitedItems(), findItem(), controlRooms(), monsterGroups(), npcRooms(), questOrganizer());
}

// src/map.ts
var SIZE = 41;
var CELL = 22;
var STEP = [
  [-1, 0],
  [1, 0],
  [0, 1],
  [0, -1],
  [-1, 1],
  [-1, -1],
  [1, 1],
  [1, -1]
];
function lineColour(kind) {
  switch (kind) {
    case 2:
    case 3:
    case 4:
      return "#4ee04e";
    case 5:
    case 12:
      return "#4ee0e0";
    case 6:
      return "#a020a0";
    case 7:
    case 11:
      return "#6a8cff";
    case 8:
      return "#ff5cff";
    case 9:
    case 24:
      return "#ff5050";
    case 10:
      return "#e8d020";
    case 13:
    case 14:
    case 15:
    case 20:
    case 23:
      return "#2040b0";
    case 16:
      return "#20a020";
    default:
      return "#909090";
  }
}
var LEGEND = [
  ["Normal", 0],
  ["Key, item, toll", 2],
  ["Action, remote action", 5],
  ["Hidden", 6],
  ["Door, gate", 7],
  ["Map change", 8],
  ["Trap, spell trap", 9],
  ["Text", 10],
  ["Class, race, level, alignment, ability", 13],
  ["Timed", 16]
];
function h3(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
function svg(tag, attrs) {
  const el = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [k2, v] of Object.entries(attrs)) el.setAttribute(k2, String(v));
  return el;
}
function follows(kind, hidden) {
  if (kind === 8 || kind === 12) return false;
  if (kind === 6) return hidden;
  return true;
}
function layOut(rooms, start, hidden) {
  const cells = /* @__PURE__ */ new Map();
  const at = /* @__PURE__ */ new Map();
  const missing = /* @__PURE__ */ new Set();
  const centre = Math.floor(SIZE / 2) * SIZE + Math.floor(SIZE / 2);
  cells.set(centre, start);
  at.set(start, centre);
  const queue = [start];
  while (queue.length) {
    const n = queue.shift();
    const room = rooms.get(n);
    if (!room) continue;
    const cell = at.get(n);
    const [r, c] = [Math.floor(cell / SIZE), cell % SIZE];
    for (const e of room.exits) {
      if (e.dir > 7 || !follows(e.kind, hidden) || at.has(e.to)) continue;
      const [dr, dc] = STEP[e.dir];
      const [nr, nc] = [r + dr, c + dc];
      if (nr < 0 || nc < 0 || nr >= SIZE || nc >= SIZE) continue;
      const next = nr * SIZE + nc;
      if (cells.has(next)) continue;
      cells.set(next, e.to);
      at.set(e.to, next);
      if (rooms.has(e.to)) queue.push(e.to);
      else missing.add(e.to);
    }
  }
  return { cells, at, missing };
}
async function fetchMap(map) {
  const res = await fetch(`/api/map?map=${map}`);
  const body = await res.json().catch(() => null);
  if (!res.ok) throw new Error(body?.error ?? res.statusText);
  return body;
}
function mapPage() {
  const el = h3("div", { class: "map-page" });
  const mapSel = h3("select", { "aria-label": "Map" });
  const roomBox = h3("input", { type: "number", min: "1", "aria-label": "Room", class: "num" });
  const hidden = h3("input", { type: "checkbox" });
  hidden.checked = true;
  const draw = h3("button", { type: "button", class: "primary" }, "Map");
  const note = h3("span", { class: "note" });
  const flash = h3("p", { class: "note map-flash" });
  const bar = h3(
    "div",
    { class: "toolbar" },
    h3("label", {}, "Map ", mapSel),
    h3("label", {}, "Start room ", roomBox),
    h3("label", {}, hidden, " Follow hidden exits"),
    draw,
    note
  );
  const board = h3("div", { class: "map-board" });
  const side = h3("aside", { class: "map-side" });
  const legend = h3(
    "ul",
    { class: "map-legend" },
    ...LEGEND.map(([label, kind]) => {
      const sw = h3("span", { class: "swatch" });
      sw.style.background = lineColour(kind);
      return h3("li", {}, sw, label);
    }),
    h3("li", {}, h3("span", { class: "swatch start" }), "Start room"),
    h3("li", {}, h3("span", { class: "swatch missing" }), "Exit to a room not on the map"),
    h3("li", {}, h3("span", { class: "mark cmd" }), "Command text"),
    h3("li", {}, h3("span", { class: "mark shop" }), "Shop"),
    h3("li", {}, h3("span", { class: "mark npc" }), "Permanent NPC"),
    h3("li", {}, "\u25B2\u25BC up and down exits")
  );
  el.append(
    h3("h2", {}, "Map"),
    bar,
    flash,
    h3("div", { class: "map-body" }, board, h3("div", { class: "map-aside" }, side, legend))
  );
  let data = null;
  let lastLay = null;
  const go = (map, room) => {
    location.hash = `#/map/${map}/${room}`;
  };
  draw.addEventListener("click", () => go(Number(mapSel.value), Number(roomBox.value) || 1));
  mapSel.addEventListener("change", () => go(Number(mapSel.value), 1));
  hidden.addEventListener("change", () => void show2());
  function info(map, r, n) {
    side.replaceChildren();
    if (!r) {
      side.append(h3("p", {}, `Room ${map}/${n} is not on the map.`));
      return;
    }
    side.append(
      h3("h3", {}, `Room ${map}/${r.room}`),
      h3("p", {}, r.name || "(no name)"),
      h3("p", {}, h3("a", { href: `#/Rooms/${encodeURIComponent(JSON.stringify([map, r.room]))}` }, "Open in the room editor"))
    );
    const list2 = h3("ul", { class: "map-exits" });
    for (const e of r.exits) {
      const target = e.kind === 8 ? `map ${e.para1} room ${e.to}` : `room ${e.to}`;
      const a = h3("a", { href: e.kind === 8 ? `#/map/${e.para1}/${e.to}` : `#/map/${map}/${e.to}` }, target);
      const sw = h3("span", { class: "swatch" });
      sw.style.background = lineColour(e.kind);
      list2.append(h3("li", {}, sw, `${EXITS[e.dir]}: `, a, ` (${EXIT_TYPE[e.kind] ?? `type ${e.kind}`})`));
    }
    side.append(r.exits.length ? list2 : h3("p", {}, "No exits."));
    const extras = [];
    if (r.shop) extras.push(`Shop ${r.shop}`);
    if (r.npc) extras.push(`Permanent NPC ${r.npc}`);
    if (r.spell) extras.push(`Spell ${r.spell}`);
    if (r.command) extras.push(`Command text ${r.command}`);
    if (r.control) extras.push(`Control room ${r.control}`);
    if (r.death) extras.push(`Death room ${r.death}`);
    if (r.exit_room) extras.push(`Exit room ${r.exit_room}`);
    if (extras.length) side.append(h3("p", { class: "note" }, extras.join(" \xB7 ")));
    side.append(builder(map, r));
  }
  function builder(map, r) {
    const panel = h3("details", { class: "map-build" }, h3("summary", {}, "Build (Map Editor)"));
    const form2 = h3("div", { class: "map-build-form" });
    const pad = h3("div", { class: "map-pad" });
    const order = [5, 0, 4, 3, -1, 2, 7, 1, 6, 8, 9];
    for (const d of order) {
      if (d < 0) {
        pad.append(h3("span", {}, String(r.room)));
        continue;
      }
      const e = r.exits.find((x) => x.dir === d);
      const b = h3("button", { type: "button", title: e ? `Go ${EXITS[d]} to ${e.to}` : `No exit ${EXITS[d]}: make or join a room` }, EXITS[d]);
      if (e) b.classList.add("has-exit");
      b.addEventListener("click", () => {
        if (e) {
          go(e.kind === 8 ? e.para1 : map, e.to);
          return;
        }
        ask(d);
      });
      pad.append(b);
    }
    panel.append(pad, form2);
    function ask(d) {
      const rooms = data?.rooms ?? [];
      const next = rooms.reduce((m, x) => Math.max(m, x.room), 0) + 1;
      let near = "";
      if (lastLay && d < 8) {
        const cell = lastLay.at.get(r.room);
        if (cell !== void 0) {
          const [dr, dc] = STEP[d];
          const there = lastLay.cells.get(cell + dr * SIZE + dc);
          if (there !== void 0) near = String(there);
        }
      }
      const existing = h3("input", { type: "checkbox" });
      const toMap = h3("input", { type: "text", inputmode: "numeric", class: "num", value: String(map), "aria-label": "Map" });
      const toRoom = h3("input", { type: "text", inputmode: "numeric", class: "num", value: String(next), "aria-label": "Room" });
      existing.addEventListener("change", () => {
        toRoom.value = existing.checked ? near : String(next);
      });
      const replace = h3("input", { type: "checkbox" });
      const make = h3("button", { type: "button", class: "primary" }, "Create");
      const result = h3("div");
      make.addEventListener("click", async () => {
        const body = {
          map,
          room: r.room,
          dir: d,
          to_map: Number(toMap.value),
          to_room: Number(toRoom.value),
          existing: existing.checked,
          replace_back: replace.checked
        };
        result.replaceChildren(h3("p", {}, "Writing\u2026"));
        flash.textContent = "";
        try {
          const res = await fetch("/api/map/dig", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
          const out = await res.json().catch(() => null);
          if (!res.ok) throw new Error(out?.error ?? res.statusText);
          if (out.notes?.length) alertNote(out.notes.join(" "));
          await show2(true);
          if (body.to_map === map) go(map, body.to_room);
        } catch (e) {
          result.replaceChildren(h3("p", { class: "error" }, e.message));
        }
      });
      form2.replaceChildren(
        h3("p", {}, `No exit ${EXITS[d]} from room ${r.room}. Make a new room there, or join an existing one; an exit is made each way.`),
        h3("div", { class: "fix-inputs" }, h3("label", {}, existing, " Existing room"), h3("label", {}, "Map ", toMap), h3("label", {}, "Room ", toRoom)),
        h3("div", {}, h3("label", {}, replace, " Replace its exit back if in use")),
        h3("div", { class: "toolbar" }, make),
        result
      );
    }
    return panel;
  }
  function alertNote(text) {
    flash.textContent = text;
  }
  function render(map, start) {
    if (!data) return;
    const rooms = new Map(data.rooms.map((r) => [r.room, r]));
    if (!rooms.has(start) && data.rooms.length) start = data.rooms[0].room;
    roomBox.value = String(start);
    const lay = layOut(rooms, start, hidden.checked);
    lastLay = lay;
    const px = SIZE * CELL;
    const s = svg("svg", { viewBox: `0 0 ${px} ${px}`, width: px, height: px, class: "map-svg" });
    const centreOf = (cell) => [
      cell % SIZE * CELL + CELL / 2,
      Math.floor(cell / SIZE) * CELL + CELL / 2
    ];
    for (const [cell, n] of lay.cells) {
      const r = rooms.get(n);
      if (!r) continue;
      const [x, y] = centreOf(cell);
      for (const e of r.exits) {
        if (e.dir > 7) continue;
        const [dr, dc] = STEP[e.dir];
        const there = lay.at.get(e.to);
        const adjacent = e.kind !== 8 && there === cell + dr * SIZE + dc;
        const reach = adjacent ? 0.5 : 0.42;
        const line = svg("line", {
          x1: x,
          y1: y,
          x2: x + dc * CELL * reach,
          y2: y + dr * CELL * reach,
          stroke: lineColour(e.kind),
          "stroke-width": 2
        });
        if (!adjacent) line.setAttribute("stroke-dasharray", "2 2");
        s.append(line);
      }
    }
    const box3 = CELL * 0.55;
    for (const [cell, n] of lay.cells) {
      const r = rooms.get(n);
      const [x, y] = centreOf(cell);
      const g = svg("g", { class: "map-room", "data-room": n });
      const rect = svg("rect", {
        x: x - box3 / 2,
        y: y - box3 / 2,
        width: box3,
        height: box3,
        class: !r ? "missing" : n === start ? "start" : r.command ? "cmd" : ""
      });
      const title = svg("title", {});
      title.textContent = r ? `${n}: ${r.name}` : `${n}: not on map ${map}`;
      g.append(title, rect);
      if (r?.shop) g.append(svg("circle", { cx: x + box3 / 2, cy: y - box3 / 2, r: 2.5, class: "shop" }));
      if (r?.npc) g.append(svg("circle", { cx: x - box3 / 2, cy: y - box3 / 2, r: 2.5, class: "npc" }));
      if (r?.exits.some((e) => e.dir === 8)) {
        g.append(svg("path", { d: `M${x - 3} ${y - 1}l3 -4l3 4z`, class: "updown" }));
      }
      if (r?.exits.some((e) => e.dir === 9)) {
        g.append(svg("path", { d: `M${x - 3} ${y + 1}l3 4l3 -4z`, class: "updown" }));
      }
      g.addEventListener("click", () => {
        info(map, r, n);
        if (r && n !== start) go(map, n);
      });
      s.append(g);
    }
    board.replaceChildren(s);
    board.scrollLeft = (px - board.clientWidth) / 2;
    board.scrollTop = (px - board.clientHeight) / 2;
    info(map, rooms.get(start), start);
    const placed = [...lay.cells.values()].filter((n) => rooms.has(n)).length;
    note.textContent = `${placed} of ${data.rooms.length} rooms on map ${map} drawn from room ${start}` + (lay.missing.size ? `; ${lay.missing.size} exits lead to rooms not on the map` : "") + ".";
  }
  async function show2(fresh = false) {
    const [, , m, r] = location.hash.split("/");
    const map = Number(m) || 1;
    const start = Number(r) || 1;
    try {
      if (fresh || !data || data.map !== map) {
        note.textContent = "Loading\u2026";
        data = await fetchMap(map);
        mapSel.replaceChildren(
          ...data.maps.map(([n, count2]) => h3("option", { value: String(n) }, `${n} (${count2} rooms)`))
        );
        if (!data.maps.some(([n]) => n === map)) mapSel.append(h3("option", { value: String(map) }, `${map} (none)`));
      }
      mapSel.value = String(map);
      render(map, start);
    } catch (e) {
      note.textContent = `Could not load the map: ${e.message}`;
    }
  }
  return { el, show: show2 };
}

// src/massroom.ts
function h4(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
var count = (n) => Array.from({ length: n }, (_, i) => i);
var GROUPS = [
  ["Name", [["Name", "Name"]]],
  ["Description", count(7).map((i) => [`Desc ${i}`, `Line ${i + 1}`])],
  ["Ansi Map", [["AnsiMap", "Ansi Map"]]],
  ["Room Type", [["Type", "Room Type", ROOM_TYPE]]],
  ["Shop #", [["Shop Number", "Shop #"]]],
  ["Gang House #", [["Gang House Number", "Gang House #"]]],
  ["Light", [["Light", "Light"]]],
  ["Attributes", [["Attributes", "Attributes"]]],
  ["Permanent NPC", [["Perm NPC", "Permanent NPC"]]],
  ["Monster Type", [["Mon Type", "Monster Type", MONSTER_GROUP]]],
  ["Min Index", [["Min Index", "Min Index"]]],
  ["Max Index", [["Max Index", "Max Index"]]],
  ["Max Regen", [["Max Regen", "Max Regen"]]],
  ["Max Area", [["Max Area", "Max Area"]]],
  ["Control Room", [["Control Room", "Control Room"]]],
  ["Death Room", [["Death Room", "Death Room"]]],
  ["Exit Room", [["Exit Room", "Exit Room"]]],
  ["CMD Text", [["Command Text", "CMD Text"]]],
  ["Delay", [["Delay", "Delay"]]],
  ["Room Spell", [["Spell", "Room Spell"]]],
  ["Visible Coins", ["Runic", "Platinum", "Gold", "Silver", "Copper"].map((c) => [c, c])],
  ["Hidden Coins", ["Runic", "Platinum", "Gold", "Silver", "Copper"].map((c) => [`Invis${c}`, c])],
  ...EXITS.map((d, i) => [
    `Exit ${d}`,
    [[`Exit ${i}`, "To Room"], [`Type ${i}`, "Exit Type", EXIT_TYPE], [`Para1 ${i}`, "Para1"], [`Para2 ${i}`, "Para2"], [`Para3 ${i}`, "Para3"], [`Para4 ${i}`, "Para4"]]
  ]),
  ["Items In Room", count(17).flatMap((i) => [[`Room Item ${i}`, `Item ${i}`], [`Room Item ${i} USES`, "Uses"], [`Room Item ${i} QTY`, "Qty - 1"]])],
  ["Hidden Items In Room", count(15).flatMap((i) => [[`Hidden Item ${i}`, `Item ${i}`], [`Hidden Item ${i} USES`, "Uses"], [`Hidden Item ${i} QTY`, "Qty - 1"]])],
  ["Placed Items", count(10).map((i) => [`Placed Item ${i}`, `Placed ${i}`])],
  ["Current Monsters In Room", count(15).map((i) => [`CurrentRoomMon ${i}`, `Monster ${i}`])]
];
function massRoomEditor(rooms, post2, show2, changed2) {
  const textual = new Set(rooms.columns.filter((c) => c.type === "text" || c.type === "memo").map((c) => c.name));
  const num3 = (value = "") => h4("input", { type: "text", inputmode: "numeric", class: "number", value });
  const card2 = h4("section", { class: "tool" }, h4("h2", {}, "Mass Room Editor"));
  card2.append(
    h4("p", { class: "hint" }, "Set the checked fields on every room of a map from one room to another. Take the values from a room, or type them. Item quantities are the count less one (0 for 1).")
  );
  const map = num3("1");
  const from = num3("1");
  const to = num3("1");
  const srcMap = num3("1");
  const srcRoom = num3("1");
  const take = h4("button", { type: "button" }, "Take Values");
  card2.append(
    h4("div", { class: "toolbar" }, h4("label", {}, "Map ", map), h4("label", {}, "From: ", from), h4("label", {}, "To: ", to)),
    h4("div", { class: "toolbar" }, h4("span", {}, "Values from room "), h4("label", {}, "Map ", srcMap), h4("label", {}, "Room # ", srcRoom), take)
  );
  const inputs = [];
  const groups = [];
  const list2 = h4("div", { class: "mass-groups" });
  for (const [label, fields] of GROUPS) {
    const box3 = h4("input", { type: "checkbox" });
    const body = h4("div", { class: "mass-fields", hidden: "" });
    const mine = [];
    for (const [column2, name, opts] of fields) {
      const text = textual.has(column2);
      const el = opts ? h4("select", {}, ...Object.entries(opts).map(([v, l]) => h4("option", { value: v }, l))) : h4("input", { type: "text", class: text ? "text" : "number", value: text ? "" : "0" });
      const i = { column: column2, el, text };
      mine.push(i);
      inputs.push(i);
      body.append(h4("label", {}, `${name} `, el));
    }
    box3.addEventListener("change", () => body.hidden = !box3.checked);
    groups.push({ box: box3, fields: mine });
    list2.append(h4("div", { class: "mass-group" }, h4("label", { class: "check" }, box3, ` ${label}`), body));
  }
  const all = h4("button", { type: "button" }, "Select All");
  const none = h4("button", { type: "button" }, "Select None");
  const setAll = (on) => groups.forEach((g) => {
    g.box.checked = on;
    g.box.dispatchEvent(new Event("change"));
  });
  all.addEventListener("click", () => setAll(true));
  none.addEventListener("click", () => setAll(false));
  card2.append(h4("div", { class: "toolbar" }, all, none), list2);
  const out = h4("div");
  take.addEventListener("click", async () => {
    try {
      const row = await api.row("Rooms", [Number(srcMap.value), Number(srcRoom.value)]);
      for (const i of inputs) {
        const v = row.values[i.column] ?? (i.text ? "" : 0);
        i.el.value = i.text ? toShown(String(v ?? "")) : String(v ?? 0);
      }
      out.replaceChildren(h4("p", {}, `Values taken from room ${srcMap.value}/${srcRoom.value}.`));
    } catch (e) {
      out.replaceChildren(h4("p", { class: "error" }, e instanceof Error ? e.message : String(e)));
    }
  });
  const preview = h4("button", { type: "button" }, "Preview");
  const go = h4("button", { type: "button", class: "primary" }, "Go");
  card2.append(h4("div", { class: "toolbar" }, preview, go), out);
  const run = async (dryRun) => {
    const values = [];
    for (const g of groups) {
      if (!g.box.checked) continue;
      for (const i of g.fields) {
        if (i.text) values.push([i.column, toWire(i.el.value)]);
        else {
          const n = Number(i.el.value.trim() || 0);
          if (!Number.isFinite(n)) {
            out.replaceChildren(h4("p", { class: "error" }, `${i.column}: ${i.el.value} is not a number.`));
            return;
          }
          values.push([i.column, n]);
        }
      }
    }
    if (!values.length) {
      out.replaceChildren(h4("p", { class: "error" }, "Check the fields to set."));
      return;
    }
    const range2 = { table: "Rooms", map: Number(map.value), from: Number(from.value), to: Number(to.value) };
    if (range2.from > range2.to) {
      out.replaceChildren(h4("p", { class: "error" }, "Illegal Range Entered!"));
      return;
    }
    if (!dryRun && !confirm(`Set ${values.length} fields on rooms ${range2.from} to ${range2.to} of map ${range2.map}?`)) return;
    out.replaceChildren(h4("p", {}, dryRun ? "Previewing\u2026" : "Changing\u2026"));
    try {
      const res = await post2("/api/tools/modify", JSON.stringify({ table: "Rooms", action: { kind: "set", values }, range: range2, only_if: [], dry_run: dryRun }));
      out.replaceChildren(show2(res));
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(h4("p", { class: "error" }, e instanceof Error ? e.message : String(e)));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}

// src/mme.ts
function h5(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
function toIni(o) {
  const b = (v) => v ? "1" : "0";
  const rooms = o.excluded.map((r) => `${r.map}/${r.from}/${r.to},`).join("");
  const lines = [
    "[Settings]",
    `CustomName=${o.custom}`,
    `Legit=${b(o.legit)}`,
    "Gmud=0",
    `DisableKai=${b(o.disable_kai)}`,
    `ExcludeDefault=${b(o.exclude_default)}`,
    `DB_Ver=${o.dat_version}`,
    `UpdateURL=${o.update_url}`,
    `EnableExcludeRooms=${b(o.exclude_rooms)}`,
    `HideExcludedRooms=${b(o.hide_excluded)}`,
    `NoRooms=${b(o.no_rooms)}`,
    `ExcludedRooms=${rooms || "0"}`
  ];
  return lines.join("\r\n") + "\r\n";
}
function fromIni(text, base) {
  const settings = /* @__PURE__ */ new Map();
  let section = "";
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    const head = /^\[(.*)\]$/.exec(line);
    if (head) {
      section = head[1].toLowerCase();
      continue;
    }
    const eq = line.indexOf("=");
    if (section === "settings" && eq > 0) settings.set(line.slice(0, eq).trim().toLowerCase(), line.slice(eq + 1));
  }
  const o = { ...base, excluded: [...base.excluded] };
  const s = (k2) => settings.get(k2.toLowerCase());
  const flag = (k2, d) => {
    const v = s(k2);
    return v === void 0 ? d : v.trim() === "1";
  };
  const text_ = (k2, d) => {
    const v = s(k2);
    return v === void 0 || v === "0" ? d : v;
  };
  o.custom = text_("CustomName", o.custom);
  o.dat_version = text_("DB_Ver", o.dat_version);
  o.update_url = text_("UpdateURL", o.update_url);
  o.legit = flag("Legit", o.legit);
  o.disable_kai = flag("DisableKai", o.disable_kai);
  o.exclude_default = flag("ExcludeDefault", o.exclude_default);
  o.exclude_rooms = flag("EnableExcludeRooms", o.exclude_rooms);
  o.hide_excluded = flag("HideExcludedRooms", o.hide_excluded);
  o.no_rooms = flag("NoRooms", o.no_rooms);
  const rooms = s("ExcludedRooms");
  if (rooms !== void 0) {
    o.excluded = [];
    for (const entry of rooms.split(",").slice(0, -1)) {
      const [map, from, to] = entry.split("/", 3).map((n) => Number(n.trim()));
      if ([map, from, to].every(Number.isFinite)) o.excluded.push({ map, from, to });
    }
  }
  return o;
}
function fileName(o, ext) {
  const name = o.custom.replace(/ /g, "") || o.dat_version.replace(/ /g, "") || "mme";
  return `data-${name}.${ext}`;
}
function save(blob, name) {
  const a = h5("a", { href: URL.createObjectURL(blob), download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 1e4);
}
async function exportAs(o, format) {
  const res = await fetch("/api/mme/export", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ options: o, format })
  });
  if (!res.ok) {
    const text = await res.text();
    let msg = text || res.statusText;
    try {
      msg = JSON.parse(text).error ?? msg;
    } catch {
    }
    throw new Error(msg);
  }
  return res;
}
function mmeExport(failure2) {
  const card2 = h5("section", { class: "tool mme" }, h5("h2", {}, "Export to MMUD Explorer"));
  card2.append(
    h5(
      "p",
      { class: "hint" },
      "Writes the .mdb MMUD Explorer reads, as NMR's export does: the same tables and values, so it shows players what NMR's export shows and nothing more. It only reads the realm."
    )
  );
  const body = h5("div", {}, "Loading\u2026");
  card2.append(body);
  fetch("/api/mme").then((r) => r.json()).then((info) => body.replaceChildren(...form(info, failure2))).catch((e) => body.replaceChildren(failure2(e)));
  return card2;
}
function form(info, failure2) {
  const checkbox = (label, checked = false) => {
    const i = h5("input", { type: "checkbox", checked: checked ? "" : void 0 });
    return [h5("label", {}, i, " ", label), i];
  };
  const text = (value = "", cls = "text") => h5("input", { type: "text", value, class: cls });
  const num3 = () => h5("input", { type: "text", inputmode: "numeric", class: "number" });
  const loadFile = h5("input", { type: "file", accept: ".ini,.txt" });
  const saveCfg = h5("button", { type: "button" }, "Save Config ...");
  const config = h5("div", { class: "toolbar" }, h5("label", { class: "file" }, "Load Config ... ", loadFile), saveCfg);
  const custom = text("");
  const datVersion = text(info.dat_version);
  const updateUrl = text("", "text url");
  const names2 = h5(
    "div",
    { class: "mme-names" },
    h5("label", {}, "BBS/Database Name ", custom),
    h5("label", {}, "Dat Version/Realm ", datVersion),
    h5("label", {}, "Update Webpage (Help > Check for Updates in MME) ", updateUrl)
  );
  const [legitL, legit] = checkbox('"Legit" Export', true);
  const [kaiL, kai] = checkbox("Disable Kai Auto-Learning");
  const [defL, def] = checkbox('Exclude Default "Not in game" Rooms', true);
  const listBtn = h5("button", { type: "button" }, "List");
  const defaults = h5("table", { class: "grid list", hidden: "" });
  defaults.append(
    h5("thead", {}, h5("tr", {}, ...["Map", "Start", "End", "Room"].map((c) => h5("th", {}, c)))),
    h5(
      "tbody",
      {},
      ...info.defaults.map(
        (d) => h5("tr", {}, ...[d.map, d.from, d.to].map((v) => h5("td", { class: "n" }, String(v))), h5("td", {}, d.name))
      )
    )
  );
  listBtn.addEventListener("click", () => defaults.hidden = !defaults.hidden);
  const options2 = h5(
    "fieldset",
    {},
    h5("legend", {}, "Export Options"),
    h5("div", {}, legitL),
    h5("div", {}, kaiL),
    h5("div", {}, defL, listBtn),
    defaults
  );
  const [enableL, enable] = checkbox("Enable");
  const [hideL, hide] = checkbox("Hide Only");
  const [allL, all] = checkbox("Exclude All");
  const map = num3();
  const from = num3();
  const to = num3();
  const add = h5("button", { type: "button" }, "Add -->");
  const remove2 = h5("button", { type: "button" }, "Remove");
  const clear = h5("button", { type: "button" }, "Clear");
  const list2 = h5("select", { size: "6", multiple: "", class: "mme-rooms" });
  const exclude = h5(
    "fieldset",
    {},
    h5("legend", {}, "Exclude Rooms"),
    h5("div", {}, enableL, hideL, allL),
    h5(
      "div",
      { class: "toolbar" },
      h5("label", {}, "Map: ", map),
      h5("label", {}, "Start ", from),
      h5("label", {}, "End ", to),
      add
    ),
    list2,
    h5("div", { class: "toolbar" }, remove2, clear)
  );
  let excluded = [];
  const showExcluded = () => list2.replaceChildren(
    ...excluded.map((r, i) => h5("option", { value: String(i) }, `Map ${r.map}: ${r.from} - ${r.to}`))
  );
  add.addEventListener("click", () => {
    const n = (i) => Number(i.value.trim());
    const r = { map: n(map), from: n(from), to: to.value.trim() ? n(to) : n(from) };
    if (![r.map, r.from, r.to].every((v) => Number.isInteger(v) && v > 0) || r.to < r.from) {
      status2.replaceChildren(failure2("Give a map, and a start room no later than the end room."));
      return;
    }
    status2.replaceChildren();
    excluded.push(r);
    showExcluded();
    from.value = "";
    to.value = "";
    from.focus();
  });
  remove2.addEventListener("click", () => {
    const picked = new Set([...list2.selectedOptions].map((o) => Number(o.value)));
    excluded = excluded.filter((_, i) => !picked.has(i));
    showExcluded();
  });
  clear.addEventListener("click", () => {
    excluded = [];
    showExcluded();
  });
  const sync = () => {
    const l = legit.checked;
    custom.disabled = l;
    updateUrl.disabled = l;
    if (l) enable.checked = false;
    enable.disabled = l;
    const on = enable.checked && !l;
    for (const el of [hide, all, map, from, to, add, remove2, clear, list2]) el.disabled = !on;
  };
  legit.addEventListener("change", sync);
  enable.addEventListener("change", sync);
  const options_ = () => ({
    legit: legit.checked,
    disable_kai: kai.checked,
    exclude_default: def.checked,
    exclude_rooms: enable.checked,
    excluded: [...excluded],
    hide_excluded: hide.checked,
    no_rooms: all.checked,
    custom: custom.value,
    update_url: updateUrl.value,
    dat_version: datVersion.value
  });
  const fill = (o) => {
    legit.checked = o.legit;
    kai.checked = o.disable_kai;
    def.checked = o.exclude_default;
    enable.checked = o.exclude_rooms;
    hide.checked = o.hide_excluded;
    all.checked = o.no_rooms;
    custom.value = o.custom;
    updateUrl.value = o.update_url;
    datVersion.value = o.dat_version;
    excluded = [...o.excluded];
    showExcluded();
    sync();
  };
  sync();
  loadFile.addEventListener("change", async () => {
    const f2 = loadFile.files?.[0];
    if (!f2) return;
    try {
      fill(fromIni(await f2.text(), options_()));
      status2.replaceChildren(h5("p", {}, `Loaded ${f2.name}.`));
    } catch (e) {
      status2.replaceChildren(failure2(e));
    }
    loadFile.value = "";
  });
  saveCfg.addEventListener("click", () => save(new Blob([toIni(options_())], { type: "text/plain" }), "mme-export.ini"));
  const preview = h5("button", { type: "button" }, "Preview");
  const go = h5("button", { type: "button", class: "primary" }, info.jet ? "Export to MME" : "Export plan");
  const script = h5("a", { href: "/api/mme/write-mdb.ps1", download: "write-mdb.ps1" }, "write-mdb.ps1");
  const status2 = h5("div", {});
  const how = info.jet ? h5("p", { class: "hint" }, "The .mdb is written through Windows' Jet engine on this machine and downloaded.") : h5(
    "p",
    { class: "hint" },
    "This machine has no Windows Jet engine, which writing a .mdb needs. Export plan downloads the export as a plan; on any Windows machine, put it beside ",
    script,
    " and run ",
    h5("code", {}, "powershell -ExecutionPolicy Bypass -File write-mdb.ps1 data-NAME.plan data-NAME.mdb"),
    " to write the .mdb MME reads."
  );
  const buttons = [preview, go];
  const run = async (format) => {
    const o = options_();
    for (const b of buttons) b.disabled = true;
    status2.replaceChildren(h5("p", {}, format === "summary" ? "Building the export\u2026" : "Exporting\u2026 this walks the whole world and takes a little while."));
    try {
      const res = await exportAs(o, format);
      if (format === "summary") {
        const { tables } = await res.json();
        const t2 = h5("table", { class: "grid" });
        t2.append(
          h5("thead", {}, h5("tr", {}, h5("th", {}, "Table"), h5("th", {}, "Rows"))),
          h5("tbody", {}, ...tables.map((r) => h5("tr", {}, h5("td", {}, r.name), h5("td", { class: "n" }, String(r.rows)))))
        );
        status2.replaceChildren(h5("p", {}, "Preview: what the export would hold. Nothing was written."), t2);
      } else {
        const name = fileName(o, format);
        save(await res.blob(), name);
        status2.replaceChildren(h5("p", {}, `Done: ${name}.`));
      }
    } catch (e) {
      status2.replaceChildren(failure2(e));
    } finally {
      for (const b of buttons) b.disabled = false;
    }
  };
  preview.addEventListener("click", () => void run("summary"));
  go.addEventListener("click", () => void run(info.jet ? "mdb" : "plan"));
  return [
    config,
    names2,
    h5("div", { class: "mme-frames" }, options2, exclude),
    how,
    h5("div", { class: "toolbar" }, preview, go),
    status2
  ];
}

// src/indexchanger.ts
function h6(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
var groupName = (g) => MONSTER_GROUP[g] ?? `group ${g}`;
var groupSelect = () => h6("select", {}, ...Object.entries(MONSTER_GROUP).map(([v, label]) => h6("option", { value: v }, label)));
var box = (value = "") => h6("input", { type: "text", inputmode: "numeric", class: "number", value });
var int = (i) => /^\s*\d+\s*$/.test(i.value) ? Number(i.value) : NaN;
function indexChanger(post2, result, failure2, changed2) {
  const fromGroup = groupSelect();
  const toGroup = groupSelect();
  const fromStart = box();
  const fromEnd = box();
  const toStart = box();
  const toEnd = h6("input", { type: "text", class: "number", readonly: "", "aria-label": "To end" });
  const calc = () => {
    const [a, b, c] = [int(fromStart), int(fromEnd), int(toStart)];
    toEnd.value = [a, b, c].some(Number.isNaN) || b < a ? "INVALID" : String(c + (b - a));
  };
  for (const i of [fromStart, fromEnd, toStart]) i.addEventListener("input", calc);
  const add = h6("button", { type: "button" }, "Add \u2192");
  const remove2 = h6("button", { type: "button" }, "Remove Line");
  const clear = h6("button", { type: "button" }, "Clear");
  const list2 = h6("select", { size: "12", class: "change-list", "aria-label": "Change list" });
  const changes = [];
  const note = h6("p", { class: "hint" });
  const runs = h6("select", { size: "12", class: "change-list", "aria-label": "Indexes from the realm" });
  const byRooms = h6("button", { type: "button" }, "List Indexes by Rooms");
  const byMonsters = h6("button", { type: "button" }, "List Indexes by Monsters");
  const preview = h6("button", { type: "button" }, "Preview");
  const go = h6("button", { type: "button", class: "primary" }, "Start");
  const out = h6("div");
  const redraw = () => {
    list2.replaceChildren(
      ...changes.map(
        (c, i) => h6("option", { value: String(i) }, `${groupName(c.from_group)}/${c.from_index} -> ${groupName(c.to_group)}/${c.to_index}`)
      )
    );
  };
  add.addEventListener("click", () => {
    calc();
    const [a, b, c] = [int(fromStart), int(fromEnd), int(toStart)];
    if (toEnd.value === "INVALID") {
      note.textContent = "Give a start and end to change (end not below start) and a start to change to.";
      return;
    }
    const g = Number(fromGroup.value);
    const clash = changes.find((x) => x.from_group === g && x.from_index >= a && x.from_index <= b);
    if (clash) {
      note.textContent = "Adding this range would include 'change from' indexes you've already added.";
      return;
    }
    for (let x = a; x <= b; x++) {
      changes.push({ from_group: g, from_index: x, to_group: Number(toGroup.value), to_index: c + (x - a) });
    }
    note.textContent = "";
    redraw();
    toStart.value = String(c + (b - a) + 1);
    if (runs.selectedIndex >= 0 && runs.selectedIndex + 1 < runs.options.length) {
      runs.selectedIndex += 1;
      takeRun();
    } else calc();
  });
  remove2.addEventListener("click", () => {
    const i = list2.selectedIndex;
    if (i < 0) return;
    changes.splice(i, 1);
    redraw();
    if (changes.length) list2.selectedIndex = Math.max(0, i - 1);
  });
  clear.addEventListener("click", () => {
    changes.length = 0;
    redraw();
  });
  function takeRun() {
    const o = runs.selectedOptions[0];
    if (!o) return;
    fromGroup.value = o.dataset.group;
    fromStart.value = o.dataset.from;
    fromEnd.value = o.dataset.to;
    calc();
  }
  runs.addEventListener("change", takeRun);
  const fill = async (url, lines) => {
    runs.replaceChildren(h6("option", {}, "Building\u2026"));
    try {
      const res = await fetch(url);
      const body = await res.json();
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      runs.replaceChildren(...lines(body));
    } catch (e) {
      runs.replaceChildren(h6("option", {}, `Could not list: ${e.message}`));
    }
  };
  const option = (group, from, to, what) => {
    const range2 = from === to ? `${from}` : `${from}-${to}`;
    const o = h6("option", {}, `${groupName(group)}/${range2}: ${what}`);
    Object.assign(o.dataset, { group: String(group), from: String(from), to: String(to) });
    return o;
  };
  byRooms.addEventListener(
    "click",
    () => void fill(
      "/api/reports/room-indexes",
      (b) => b.runs.map((r) => option(r.group, r.from, r.to, r.rooms.map(([m, n]) => `${m}/${n}`).join(", ")))
    )
  );
  byMonsters.addEventListener(
    "click",
    () => void fill(
      "/api/reports/monster-groups",
      (b) => b.groups.map((g) => option(g.group, g.index, g.index, g.monsters.map(([n, name]) => `${name}(${n})`).join(", ")))
    )
  );
  const run = async (dryRun) => {
    if (!changes.length) {
      out.replaceChildren(h6("p", { class: "error" }, "Add a change first."));
      return;
    }
    if (!dryRun && !confirm("Change the monster groups and indexes, and the rooms that use them? Preview first to see them.")) return;
    out.replaceChildren(h6("p", {}, dryRun ? "Previewing\u2026" : "Changing\u2026"));
    try {
      const parts = [];
      for (const [table, title] of [
        ["monsters", "Monsters"],
        ["rooms", "Rooms"]
      ]) {
        const res = await post2("/api/tools/preset", JSON.stringify({ preset: "change_monster_indexes", table, changes, dry_run: dryRun }));
        parts.push(h6("h3", {}, title), result(res));
      }
      out.replaceChildren(...parts);
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(failure2(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  calc();
  return h6(
    "section",
    { class: "tool" },
    h6("h2", {}, "Monster Group/Index Changer"),
    h6(
      "p",
      { class: "hint" },
      "Moves monsters to new groups and indexes, and the rooms whose Monster Type and index range hold them. A room's range moves as one: change the indexes a room's range covers together (List Indexes by Rooms, then click a line to fill it in)."
    ),
    h6(
      "div",
      { class: "index-changer" },
      h6(
        "div",
        {},
        h6("h3", {}, "Change this \u2026"),
        h6("div", { class: "fix-inputs" }, h6("label", {}, "Group ", fromGroup), h6("label", {}, "Start ", fromStart), h6("label", {}, "End ", fromEnd)),
        h6("h3", {}, "To this \u2026"),
        h6("div", { class: "fix-inputs" }, h6("label", {}, "Group ", toGroup), h6("label", {}, "Start ", toStart), h6("label", {}, "End ", toEnd)),
        h6("div", { class: "toolbar" }, add),
        note,
        h6("div", { class: "toolbar" }, byRooms, byMonsters),
        runs
      ),
      h6("div", {}, h6("h3", {}, "Change list"), list2, h6("div", { class: "toolbar" }, remove2, clear, preview, go))
    ),
    out
  );
}

// src/renumber.ts
function h7(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
var KINDS = [
  ["classes", "Classes"],
  ["items", "Items"],
  ["messages", "Messages"],
  ["monsters", "Monsters"],
  ["shops", "Shops"],
  ["spells", "Spells"],
  ["races", "Races"],
  ["rooms", "Rooms"],
  ["textblocks", "Textblocks"],
  ["map", "Change Map Number"]
];
var box2 = (label) => h7("input", { type: "text", inputmode: "numeric", class: "number", "aria-label": label });
var int2 = (i) => /^\s*\d+\s*$/.test(i.value) ? Number(i.value) : NaN;
function recordNumberChanger(post2, result, failure2, changed2) {
  const kind = h7("select", { "aria-label": "Records" }, ...KINDS.map(([v, l]) => h7("option", { value: v }, l)));
  kind.value = "items";
  const map = box2("Map");
  const fromStart = box2("Change from start");
  const fromEnd = box2("Change from end");
  const toStart = box2("Change to start");
  const toEnd = h7("input", { type: "text", class: "number", readonly: "", "aria-label": "Change to end" });
  const calc = () => {
    const [a, b, c] = [int2(fromStart), int2(fromEnd), int2(toStart)];
    toEnd.value = [a, b, c].some(Number.isNaN) || b < a ? "INVALID" : String(c + (b - a));
  };
  for (const i of [fromStart, fromEnd, toStart]) i.addEventListener("input", calc);
  const note = h7("p", { class: "hint" });
  const list2 = h7("select", { size: "12", class: "change-list", "aria-label": "Change list" });
  const moves = [];
  const records = h7("select", { size: "12", class: "change-list", "aria-label": "Records in the realm" });
  const out = h7("div");
  const mapFrom = box2("Change map");
  const mapTo = box2("To map");
  const boxes = [
    ["exits", "Change 'Map Change' exit types on rooms to new map."],
    ["normal", "If the new map on a map change exit now matches the rooms' map, change the exit type to normal."],
    ["textblocks", "Change Teleport commands in textblocks to new map."],
    ["spells", "Change Teleport abilities in spells to new map."]
  ].map(([k2, label]) => {
    const c = h7("input", { type: "checkbox" });
    c.checked = true;
    return [k2, c, h7("label", {}, c, " ", label)];
  });
  const mapForm = h7(
    "div",
    { class: "renumber-map" },
    h7("div", { class: "fix-inputs" }, h7("label", {}, "Change Map: ", mapFrom), h7("label", {}, "To Map: ", mapTo)),
    ...boxes.map(([, , l]) => h7("div", {}, l))
  );
  const redraw = () => {
    list2.replaceChildren(...moves.map((m, i) => h7("option", { value: String(i) }, `${m.from} -> ${m.to}`)));
    const locked = moves.length > 0;
    kind.disabled = locked;
    map.readOnly = locked;
  };
  const add = (a, b, c) => {
    for (const m of moves) {
      const clash = m.from >= a && m.from <= b || m.to >= a && m.to <= b || m.from >= c && m.from <= c + (b - a) || m.to >= c && m.to <= c + (b - a);
      if (clash) {
        note.textContent = `Adding this range would include record numbers you've already added (record ${m.from} is set to be changed to ${m.to}).`;
        return false;
      }
    }
    for (let x = a; x <= b; x++) moves.push({ from: x, to: c + (x - a) });
    note.textContent = "";
    redraw();
    toStart.value = String(c + (b - a) + 1);
    calc();
    return true;
  };
  const addButton = h7("button", { type: "button" }, "Add \u2192");
  addButton.addEventListener("click", () => {
    calc();
    if (kind.value === "rooms" && !(int2(map) > 0)) {
      note.textContent = "Please enter a map number first.";
      return;
    }
    if (toEnd.value === "INVALID") {
      note.textContent = "Give a start and end to change (end not below start) and a start to change to.";
      return;
    }
    add(int2(fromStart), int2(fromEnd), int2(toStart));
  });
  const removeButton = h7("button", { type: "button" }, "Remove");
  removeButton.addEventListener("click", () => {
    const i = list2.selectedIndex;
    if (i < 0) return;
    moves.splice(i, 1);
    redraw();
  });
  const clearButton = h7("button", { type: "button" }, "Clear");
  clearButton.addEventListener("click", () => {
    moves.length = 0;
    redraw();
  });
  const listButton = h7("button", { type: "button" }, "List Records");
  const addAll = h7("button", { type: "button" }, "Add All");
  const tableName = () => KINDS.find(([v]) => v === kind.value)[1];
  listButton.addEventListener("click", async () => {
    records.replaceChildren(h7("option", {}, "Building\u2026"));
    try {
      const res = await fetch(`/api/tables/${tableName()}/rows`);
      const body = await res.json();
      if (!res.ok) throw new Error(body?.error ?? res.statusText);
      const want = int2(map);
      const nums = [
        ...new Set(
          body.rows.filter(([key]) => kind.value !== "rooms" || key[0] === want).map(([key]) => kind.value === "rooms" ? key[1] : key[0])
        )
      ].sort((x, y) => x - y);
      const runs = [];
      for (const x of nums) {
        const last = runs[runs.length - 1];
        if (last && last[1] + 1 === x) last[1] = x;
        else runs.push([x, x]);
      }
      records.replaceChildren(
        ...runs.map(([a, b]) => {
          const o = h7("option", {}, a === b ? `${a}` : `${a}-${b}`);
          Object.assign(o.dataset, { from: String(a), to: String(b) });
          return o;
        })
      );
      if (!runs.length) records.replaceChildren(h7("option", {}, kind.value === "rooms" ? `No rooms on map ${want || "?"}.` : "No records."));
    } catch (e) {
      records.replaceChildren(h7("option", {}, `Could not list: ${e.message}`));
    }
  });
  records.addEventListener("change", () => {
    const o = records.selectedOptions[0];
    if (!o?.dataset.from) return;
    fromStart.value = o.dataset.from;
    fromEnd.value = o.dataset.to;
    calc();
  });
  addAll.addEventListener("click", () => {
    if (!(int2(toStart) > 0)) {
      note.textContent = "Give the number to start changing to first.";
      return;
    }
    for (const o of records.options) {
      if (!o.dataset.from) continue;
      if (!add(Number(o.dataset.from), Number(o.dataset.to), int2(toStart))) break;
    }
  });
  const recordsForm = h7(
    "div",
    { class: "index-changer" },
    h7(
      "div",
      {},
      h7("div", { class: "fix-inputs" }, h7("label", {}, "Map: ", map)),
      h7("h3", {}, "Change This \u2026"),
      h7("div", { class: "fix-inputs" }, h7("label", {}, "Start ", fromStart), h7("label", {}, "End ", fromEnd)),
      h7("h3", {}, "To This \u2026"),
      h7("div", { class: "fix-inputs" }, h7("label", {}, "Start ", toStart), h7("label", {}, "End ", toEnd)),
      h7("div", { class: "toolbar" }, addButton),
      note,
      h7("div", { class: "toolbar" }, listButton, addAll),
      records
    ),
    h7("div", {}, h7("h3", {}, "Change list"), list2, h7("div", { class: "toolbar" }, removeButton, clearButton))
  );
  const showForm = () => {
    const isMap = kind.value === "map";
    recordsForm.hidden = isMap;
    mapForm.hidden = !isMap;
    map.parentElement.hidden = kind.value !== "rooms";
    records.replaceChildren();
    out.replaceChildren();
  };
  kind.addEventListener("change", showForm);
  const users = h7("input", { type: "checkbox" });
  const usersBox = h7(
    "label",
    { title: "Items and keys they carry, wear or wield; spells in their spellbook or on them; their class and race; where they stand and their trail" },
    users,
    " Users follow too (NMR's changer leaves users alone)"
  );
  const request = () => {
    if (kind.value === "map") {
      const body = { kind: "map", from: int2(mapFrom), to: int2(mapTo), users: users.checked };
      for (const [k2, c] of boxes) body[k2] = c.checked;
      return body;
    }
    return { kind: "records", table: kind.value, map: kind.value === "rooms" ? int2(map) : void 0, moves, users: users.checked };
  };
  const run = async (dryRun) => {
    if (!dryRun && !confirm("Change the record numbers, and everything that names them? Preview first to see the changes.")) return;
    out.replaceChildren(h7("p", {}, dryRun ? "Previewing\u2026" : "Changing\u2026"));
    try {
      const res = await post2("/api/tools/renumber", JSON.stringify({ ...request(), dry_run: dryRun }));
      const parts = res.tables.map((t2) => h7("div", {}, h7("h3", {}, t2.table), result(t2)));
      out.replaceChildren(...parts.length ? parts : [h7("p", {}, "Nothing would change.")]);
      if (!dryRun) {
        changed2();
        moves.length = 0;
        redraw();
      }
    } catch (e) {
      out.replaceChildren(failure2(e));
    }
  };
  const preview = h7("button", { type: "button" }, "Preview");
  const go = h7("button", { type: "button", class: "primary" }, "Start");
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  calc();
  const card2 = h7(
    "section",
    { class: "tool" },
    h7("h2", {}, "Record Number Changer"),
    h7(
      "p",
      { class: "hint" },
      "Moves records to new numbers, and changes every place that names them: other records, abilities and text block commands, as NMR does. New numbers must be free. Rooms move within one map; Change Map Number moves a whole map."
    ),
    h7("div", { class: "toolbar" }, h7("label", {}, "Records: ", kind)),
    recordsForm,
    mapForm,
    h7("div", {}, usersBox),
    h7("div", { class: "toolbar" }, preview, go),
    out
  );
  showForm();
  return card2;
}

// src/tools.ts
var TABLES = ["Items", "Monsters", "Spells", "Shops", "Races", "Classes", "Rooms", "Actions", "Messages", "Textblocks"];
function h8(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  el.append(...children);
  return el;
}
var input = (props) => h8("input", props);
async function post(url, body, json = false) {
  const res = await fetch(url, {
    method: "POST",
    body,
    headers: json ? { "content-type": "application/json" } : {}
  });
  const text = await res.text();
  let parsed = null;
  try {
    parsed = JSON.parse(text);
  } catch {
  }
  if (!res.ok) throw new Error(parsed?.error ?? (text || res.statusText));
  return parsed;
}
function reportTable(reports, dryRun) {
  const box3 = h8("div", { class: "tool-result" });
  box3.append(h8("p", {}, dryRun ? "Preview: nothing was written." : "Done."));
  const table = h8("table", { class: "grid" });
  table.append(
    h8("thead", {}, h8("tr", {}, ...["Table", "Inserted", "Updated", "Unchanged", "Skipped", "Deleted", "Errors"].map((c) => h8("th", {}, c))))
  );
  const body = h8("tbody");
  for (const r of reports) {
    body.append(
      h8("tr", {}, ...[r.table, r.inserted, r.updated, r.unchanged, r.skipped ?? 0, r.deleted, r.errors.length].map((v) => h8("td", {}, String(v))))
    );
  }
  table.append(body);
  box3.append(table);
  const errors = reports.flatMap((r) => r.errors);
  if (errors.length) box3.append(h8("pre", { class: "errors" }, errors.join("\n")));
  return box3;
}
function failure(e) {
  return h8("p", { class: "error" }, e instanceof Error ? e.message : String(e));
}
function rangeOf(r) {
  if (r.all.checked) return null;
  const n = (i) => Number(i.value.trim() || 0);
  return { table: r.table, map: r.map ? n(r.map) : void 0, from: n(r.from), to: n(r.to) };
}
function rangeRow(table, withUse) {
  const use = input({ type: "checkbox", checked: "" });
  const all = input({ type: "checkbox", checked: "" });
  const num3 = () => input({ type: "text", inputmode: "numeric", class: "number", disabled: "" });
  const map = table === "Rooms" ? num3() : null;
  const from = num3();
  const to = num3();
  if (table === "Actions") all.disabled = true;
  all.addEventListener("change", () => {
    for (const i of [map, from, to]) if (i) i.disabled = all.checked;
  });
  const tr = h8(
    "tr",
    {},
    h8("td", {}, withUse ? h8("label", {}, use, " ", table) : table),
    h8("td", {}, h8("label", {}, all, " All")),
    h8("td", {}, map ?? ""),
    h8("td", {}, from),
    h8("td", {}, to)
  );
  return [tr, { table, use, all, map, from, to }];
}
function rangeHead(first) {
  return h8("thead", {}, h8("tr", {}, ...[first, "Range", "Map", "From", "To"].map((c) => h8("th", {}, c))));
}
function importer(changed2) {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Database Importer"));
  card2.append(
    h8("p", { class: "hint" }, "Import an NMR export (.mdb) into this realm. Stop the realm first. Preview First shows what would change without writing.")
  );
  const file = input({ type: "file", accept: ".mdb,.json" });
  card2.append(h8("label", { class: "file" }, "Import file: ", file));
  const rows = [];
  const table = h8("table", { class: "grid" });
  table.append(rangeHead("Databases to Import"));
  const body = h8("tbody");
  for (const t2 of TABLES) {
    const [tr, r] = rangeRow(t2, true);
    rows.push(r);
    body.append(tr);
  }
  table.append(body);
  const allBtn = h8("button", { type: "button" }, "All");
  const noneBtn = h8("button", { type: "button" }, "None");
  allBtn.addEventListener("click", () => rows.forEach((r) => r.use.checked = true));
  noneBtn.addEventListener("click", () => rows.forEach((r) => r.use.checked = false));
  card2.append(table, h8("div", { class: "toolbar" }, allBtn, noneBtn));
  const radio = (name, value, label, checked = false) => h8("label", {}, input({ type: "radio", name, value, checked: checked ? "" : void 0 }), " ", label);
  card2.append(
    h8("fieldset", {}, h8("legend", {}, "Existing Records"), radio("existing", "update", "Update", true), radio("existing", "skip", "Skip")),
    h8(
      "fieldset",
      {},
      h8("legend", {}, "Rooms"),
      radio("rooms", "all", "Everything", true),
      radio("rooms", "only_items", "Only Items"),
      radio("rooms", "not_items", "Not Items")
    )
  );
  const extra = input({ type: "checkbox", checked: "" });
  card2.append(h8("label", { class: "check" }, extra, " Delete text block parts past the last part the file has (NMR always does)"));
  const preview = h8("button", { type: "button" }, "Preview First");
  const go = h8("button", { type: "button", class: "primary" }, "Import");
  const out = h8("div");
  card2.append(h8("div", { class: "toolbar" }, preview, go), out);
  const run = async (dryRun) => {
    const f2 = file.files?.[0];
    if (!f2) {
      out.replaceChildren(failure("Pick the file to import."));
      return;
    }
    const picked = (name) => card2.querySelector(`input[name=${name}]:checked`).value;
    const options2 = {
      tables: rows.filter((r) => r.use.checked).map((r) => r.table),
      ranges: rows.filter((r) => r.use.checked).map(rangeOf).filter((r) => r !== null),
      skip_existing: picked("existing") === "skip",
      room_parts: picked("rooms"),
      extra_parts: extra.checked,
      dry_run: dryRun
    };
    if (!dryRun && !confirm(`Import ${f2.name} into this realm?`)) return;
    out.replaceChildren(h8("p", {}, `${dryRun ? "Previewing" : "Importing"} ${f2.name}\u2026`));
    try {
      const res = await post(`/api/tools/import?options=${encodeURIComponent(JSON.stringify(options2))}`, f2);
      out.replaceChildren(reportTable(res.reports, res.dry_run));
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
function deleter(changed2) {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Database Deleter"));
  card2.append(h8("p", { class: "hint" }, "Delete every record of a table, or a range of them. This cannot be undone; Preview shows how many would go."));
  const select2 = h8("select", {}, ...TABLES.map((t2) => h8("option", { value: t2 }, t2)));
  const holder = h8("tbody");
  let row = rangeRow(TABLES[0], false);
  holder.append(row[0]);
  select2.addEventListener("change", () => {
    row = rangeRow(select2.value, false);
    holder.replaceChildren(row[0]);
  });
  const table = h8("table", { class: "grid" }, rangeHead("Table"), holder);
  const preview = h8("button", { type: "button" }, "Preview");
  const go = h8("button", { type: "button", class: "danger" }, "Delete Selected");
  const out = h8("div");
  card2.append(h8("label", {}, "Delete from: ", select2), table, h8("div", { class: "toolbar" }, preview, go), out);
  const run = async (dryRun) => {
    const r = row[1];
    const range2 = rangeOf(r);
    const what = range2 ? `${r.table} ${range2.map !== void 0 ? `map ${range2.map}, ` : ""}${range2.from} to ${range2.to}` : `every ${r.table} record`;
    if (!dryRun && !confirm(`Delete ${what} from the realm? This cannot be undone.`)) return;
    try {
      const res = await post("/api/tools/delete", JSON.stringify({ table: r.table, range: range2, all: range2 === null, dry_run: dryRun }), true);
      out.replaceChildren(h8("p", {}, `${res.report.deleted} ${r.table} record${res.report.deleted === 1 ? "" : "s"} ${dryRun ? "would be " : ""}deleted.`));
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
function mergeUsers() {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Merge Users"));
  card2.append(h8("p", { class: "hint" }, "Copy the users of another user file (WCCUSER2.DAT, or a Daydream .db) into this realm, byte for byte. Users the realm already has are skipped."));
  const file = input({ type: "file", accept: ".dat,.db" });
  const preview = h8("button", { type: "button" }, "Preview");
  const go = h8("button", { type: "button", class: "primary" }, "Merge");
  const out = h8("div");
  card2.append(h8("label", { class: "file" }, "Users to merge in: ", file), h8("div", { class: "toolbar" }, preview, go), out);
  const run = async (dryRun) => {
    const f2 = file.files?.[0];
    if (!f2) {
      out.replaceChildren(failure("Pick the user file."));
      return;
    }
    if (!dryRun && !confirm(`Merge the users of ${f2.name} into this realm?`)) return;
    try {
      const res = await post(`/api/tools/merge-users?dry_run=${dryRun}`, f2);
      const lines = [`${res.copied} users ${dryRun ? "would be " : ""}copied, ${res.skipped.length} skipped (already in the realm).`];
      out.replaceChildren(h8("p", {}, lines[0]));
      if (res.skipped.length) out.append(h8("pre", { class: "errors" }, res.skipped.join("\n")));
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
var WCC_KEY = "daydream.wcc";
var remembered = () => {
  try {
    return localStorage.getItem(WCC_KEY) ?? "";
  } catch {
    return "";
  }
};
var remember = (dir) => {
  try {
    localStorage.setItem(WCC_KEY, dir);
  } catch {
  }
};
function writeDat() {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Write to WCC files"));
  card2.append(
    h8(
      "p",
      { class: "hint" },
      "Write this realm back into the WCC*.DAT files it was converted from, for Worldgroup and NMR. Files nobody changed are left alone, edits that keep every key are written in place, and files with records added, deleted or renumbered are rebuilt. Stop the game first."
    )
  );
  const wcc = input({ type: "text", placeholder: "C:\\WGSERV", value: remembered() });
  const out = input({ type: "text", placeholder: "leave empty to write over the WCC folder" });
  const backup = input({ type: "checkbox", checked: "" });
  const force = input({ type: "checkbox" });
  const preview = h8("button", { type: "button" }, "Preview");
  const go = h8("button", { type: "button", class: "primary" }, "Write");
  const result = h8("div");
  card2.append(
    h8("label", { class: "path" }, "WCC folder (on the machine Daydream runs on) ", wcc),
    h8("label", { class: "path" }, "Write to another folder instead ", out),
    h8("label", { class: "check" }, backup, " Back up each file before changing it"),
    h8("label", { class: "check" }, force, " Write even over files the game changed since the realm was converted"),
    h8("div", { class: "toolbar" }, preview, go),
    result
  );
  const run = async (dryRun) => {
    const dir = wcc.value.trim();
    if (!dir) {
      result.replaceChildren(failure("Give the WCC folder."));
      return;
    }
    if (!dryRun && !confirm(`Write the realm into the WCC files${out.value.trim() ? ` in ${out.value.trim()}` : ` of ${dir}`}? Stop the game first.`)) return;
    try {
      const res = await post(
        "/api/tools/write-dat",
        JSON.stringify({ wcc: dir, out: out.value.trim() || null, backup: backup.checked, force: force.checked, dry_run: dryRun }),
        true
      );
      remember(dir);
      const files = res.files;
      const changed2 = files.filter((f2) => f2.action !== "unchanged");
      const would = dryRun ? "would be " : "";
      const line = (f2) => `${f2.dest}: ${f2.action === "updated" ? `${f2.records} records ${would}updated in place` : `${would}rebuilt with ${f2.records} records`}`;
      result.replaceChildren(
        h8("p", {}, changed2.length ? `${changed2.length} of ${files.length} files ${dryRun ? "would change" : "written"}.` : `All ${files.length} files are as the realm has them: nothing to write.`)
      );
      if (changed2.length) result.append(h8("pre", {}, changed2.map(line).join("\n")));
      if (res.backup) result.append(h8("p", { class: "hint" }, `The files as they were are in ${res.backup}.`));
    } catch (e) {
      result.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
var OPS = [["add", "+"], ["sub", "-"], ["mul", "*"], ["div", "/"], ["percent", "%"], ["set", "="]];
var CMPS = [["eq", "="], ["le", "<="], ["ge", ">="], ["ne", "<>"]];
var options = (pairs) => pairs.map(([v, label]) => h8("option", { value: v }, label));
var numberInput = (value = "") => input({ type: "text", inputmode: "decimal", class: "number", value });
function numericColumns(t2) {
  const keys = new Set(t2.key);
  const names2 = t2.columns.filter((c) => (c.type === "long" || c.type === "double") && !keys.has(c.name)).map((c) => c.name);
  const families2 = /* @__PURE__ */ new Map();
  for (const n of names2) {
    const m = /^(.*\D) ?\d+$/.exec(n);
    if (m) families2.set(m[1], (families2.get(m[1]) ?? 0) + 1);
  }
  const out = [];
  for (const [f2, count2] of families2) if (count2 > 1) out.push([`${f2}*`, `${f2.trimEnd()} (every one)`]);
  return [...names2.map((n) => [n, n]), ...out];
}
function columnsOf(t2, choice) {
  if (!choice.endsWith("*")) return [choice];
  const stem = choice.slice(0, -1);
  return t2.columns.map((c) => c.name).filter((n) => n.startsWith(stem) && /^\s?\d+$/.test(n.slice(stem.length)));
}
var abilityOptions = () => options(
  Object.entries(ABILITIES).filter(([n]) => n !== "0").map(([n, [name]]) => [n, `${n}: ${name}`])
);
function universalModifier(tables, changed2) {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Universal Modifier"));
  card2.append(
    h8(
      "p",
      { class: "hint" },
      "Change a field (or an ability) in many records at once. Preview lists every value that would change; Start writes them."
    )
  );
  const editable = tables.filter((t2) => TABLES.includes(t2.name) && t2.name !== "Actions");
  const editor = h8("select", {}, ...editable.map((t2) => h8("option", { value: t2.name }, t2.name)));
  const top = h8("div", { class: "toolbar" }, h8("label", {}, "Editor: ", editor));
  const rangeBody = h8("tbody");
  const actionBox = h8("div");
  const onlyIfBox = h8("div");
  const out = h8("div");
  const preview = h8("button", { type: "button" }, "Preview");
  const go = h8("button", { type: "button", class: "primary" }, "Start");
  card2.append(
    top,
    h8("table", { class: "grid" }, rangeHead("Table"), rangeBody),
    h8("fieldset", {}, h8("legend", {}, "Modifier"), actionBox),
    h8("fieldset", {}, h8("legend", {}, "Only Perform Action If:"), onlyIfBox),
    h8("div", { class: "toolbar" }, preview, go),
    out
  );
  let build = () => ({});
  const setUp = () => {
    const t2 = editable.find((x) => x.name === editor.value);
    const hasAbilities = t2.columns.some((c) => c.name === "Ability 0");
    const [tr, range2] = rangeRow(t2.name, false);
    rangeBody.replaceChildren(tr);
    const kinds = [["math", "Field"]];
    if (hasAbilities) kinds.push(["change_ability", "Ability value"], ["give_ability", "Give ability"], ["take_ability", "Take ability"]);
    const kind = h8("select", {}, ...options(kinds));
    const field2 = h8("select", {}, ...options(numericColumns(t2)));
    const ability = h8("select", {}, ...abilityOptions());
    const op = h8("select", {}, ...options(OPS));
    const value = numberInput("0");
    const limitCmp = h8("select", {}, h8("option", { value: "" }, "(no limit)"), ...options(CMPS.filter(([c]) => c === "le" || c === "ge")));
    const limit = numberInput();
    const clear = input({ type: "checkbox" });
    const fieldLabel = h8("label", {}, "Field ", field2);
    const abilityLabel = h8("label", {}, "Ability ", ability);
    const opLabel = h8("label", {}, "Modifier ", op);
    const valueLabel = h8("label", {}, "Value ", value);
    const limitLabel = h8("label", {}, "Limit to ", limitCmp, limit);
    const clearLabel = h8("label", {}, clear, " Clear its value too");
    const show2 = () => {
      const k2 = kind.value;
      fieldLabel.hidden = k2 !== "math";
      abilityLabel.hidden = k2 === "math";
      opLabel.hidden = limitLabel.hidden = k2 === "give_ability" || k2 === "take_ability";
      valueLabel.hidden = k2 === "take_ability";
      clearLabel.hidden = k2 !== "take_ability";
    };
    kind.addEventListener("change", show2);
    actionBox.replaceChildren(h8("label", {}, kind), fieldLabel, abilityLabel, opLabel, valueLabel, limitLabel, clearLabel);
    show2();
    const conditionKinds = [["", "(none)"], ["column", "Field"]];
    if (hasAbilities) conditionKinds.push(["has_ability", "Has ability"], ["lacks_ability", "Lacks ability"]);
    const plainColumns = numericColumns(t2).filter(([v]) => !v.endsWith("*"));
    const conditions = [0, 1, 2].map(() => {
      const k2 = h8("select", {}, ...options(conditionKinds));
      const col = h8("select", {}, ...options(plainColumns));
      const ab = h8("select", {}, ...abilityOptions());
      const cmp = h8("select", {}, ...options(CMPS));
      const v = numberInput("0");
      const mustBe = h8("span", {}, " must be ");
      const sync = () => {
        col.hidden = k2.value !== "column";
        ab.hidden = !k2.value.endsWith("ability");
        cmp.hidden = v.hidden = mustBe.hidden = k2.value === "";
      };
      k2.addEventListener("change", sync);
      sync();
      return { row: h8("div", { class: "toolbar" }, k2, col, ab, mustBe, cmp, v), k: k2, col, ab, cmp, v };
    });
    onlyIfBox.replaceChildren(...conditions.map((c) => c.row));
    build = () => {
      const n = (i) => Number(i.value.trim() || 0);
      const lim = limitCmp.value ? [limitCmp.value, n(limit)] : null;
      const k2 = kind.value;
      const action = k2 === "math" ? { kind: k2, columns: columnsOf(t2, field2.value), op: op.value, value: n(value), limit: lim } : k2 === "change_ability" ? { kind: k2, ability: Number(ability.value), op: op.value, value: n(value), limit: lim } : k2 === "give_ability" ? { kind: k2, ability: Number(ability.value), value: n(value) } : { kind: k2, ability: Number(ability.value), clear_value: clear.checked };
      const only_if = conditions.filter((c) => c.k.value).map(
        (c) => c.k.value === "column" ? { kind: "column", column: c.col.value, cmp: c.cmp.value, value: n(c.v) } : { kind: c.k.value, ability: Number(c.ab.value), cmp: c.cmp.value, value: n(c.v) }
      );
      return { table: t2.name, action, range: rangeOf(range2), only_if };
    };
  };
  editor.addEventListener("change", () => {
    setUp();
    out.replaceChildren();
  });
  setUp();
  const run = async (dryRun) => {
    const body = build();
    if (!dryRun && !confirm(`Change ${editor.value} records as set? Preview first to see every change.`)) return;
    out.replaceChildren(h8("p", {}, dryRun ? "Previewing\u2026" : "Changing\u2026"));
    try {
      const res = await post("/api/tools/modify", JSON.stringify({ ...body, dry_run: dryRun }), true);
      out.replaceChildren(modifyResult(res));
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
var shownCell = (v) => typeof v === "string" ? toShown(v.split("\0")[0]) : String(v);
function modifyResult(res) {
  const box3 = h8("div", { class: "tool-result" });
  const would = res.dry_run ? "would change" : "changed";
  box3.append(
    h8(
      "p",
      {},
      `${res.records} record${res.records === 1 ? "" : "s"} ${would} (${res.changed} value${res.changed === 1 ? "" : "s"}); ${res.unchanged} already as set; ${res.skipped} left out by its conditions.` + (res.dry_run ? " Preview: nothing was written." : "")
    )
  );
  if (res.notes?.length) box3.append(h8("p", {}, "Worth a look (not changed):"), h8("pre", { class: "errors" }, res.notes.join("\n")));
  if (res.report?.errors?.length) box3.append(h8("pre", { class: "errors" }, res.report.errors.join("\n")));
  if (res.changes.length) {
    const body = h8("tbody");
    for (const c of res.changes) {
      body.append(h8("tr", {}, ...[c.key.join(" / "), c.column, shownCell(c.before), shownCell(c.after)].map((v) => h8("td", {}, v))));
    }
    box3.append(h8("table", { class: "grid changes" }, h8("thead", {}, h8("tr", {}, ...["Record", "Field", "Before", "After"].map((c) => h8("th", {}, c)))), body));
    if (res.changes.length < res.changed) box3.append(h8("p", { class: "hint" }, `The first ${res.changes.length} changes are listed.`));
  }
  return box3;
}
var pick = (values, selected) => {
  const s = h8("select", {}, ...options(values));
  if (selected) s.value = selected;
  return s;
};
var check = (label, on = true) => {
  const i = input({ type: "checkbox", checked: on ? "" : void 0 });
  return [i, h8("label", {}, i, " ", label)];
};
var num = (i) => Number(i.value.trim() || 0);
function fixes() {
  const mulBy = numberInput("2");
  const mulCap = numberInput("0");
  const divBy = numberInput("2");
  const bossBy = numberInput("2");
  const date = input({ type: "date", value: "1990-01-01" });
  const strip = input({ type: "text", class: "strip" });
  const padMap = numberInput("1");
  const unpadMap = numberInput("1");
  const mapOrAll = (i) => i.value.trim() === "" ? null : num(i);
  const dropMode = pick([["set", "Set to"], ["add", "Add (+/-)"]]);
  const dropValue = numberInput("20");
  const dropCmp = pick(CMPS, "le");
  const dropLimit = numberInput("20");
  const dropLimited = pick([["include", "Include limited items"], ["exclude", "Exclude limited items"], ["only", "Only limited items"]]);
  const [delMaint, delMaintLabel] = check("Exclude weapons and armour deleted at maintenance (ability 119)", true);
  const [regenOn, regenLabel] = check("AND only if monster regen >", true);
  const regen = numberInput("1");
  const types = ["Weapons", "Armour", "Keys", "Containers", "Scrolls"].map((t2) => check(t2));
  return [
    {
      title: "No Limited Items",
      hint: "Every item's Game Limit to 0, except the quest portals (839, 1078, 1326, 1531, 1643, 1686, 1749) and gang items.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "no_limited_items" }]
    },
    {
      title: "No Level Restrictions",
      hint: "Takes the level restriction abilities (135 and 136) off every item, values too.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "no_level_restrictions" }]
    },
    {
      title: "Fix Number of Uses (items)",
      hint: "Items with 0 uses get -1. Items kept after their uses get a recharge value equal to their uses; those without a recharge ability are listed.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "fix_item_uses" }]
    },
    {
      title: "Fix Number of Uses on Item Drops",
      hint: "Each monster drop's uses set to the item's own uses. Run the item fix first.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "fix_drop_uses" }]
    },
    {
      title: "Multiply All Exp",
      hint: "Monster experience times 2 to 20, split into Experience \xD7 Exp Multiplier as NMR splits it. A cap of 0 is the most 1.11p holds.",
      inputs: [h8("label", {}, "Times ", mulBy), h8("label", {}, "Cap ", mulCap)],
      request: () => ["/api/tools/preset", { preset: "multiply_exp", by: num(mulBy), cap: num(mulCap) || null }]
    },
    {
      title: "Divide All Exp",
      hint: "Monster experience divided by 2 to 20 (at least 1).",
      inputs: [h8("label", {}, "Divide by ", divBy)],
      request: () => ["/api/tools/preset", { preset: "divide_exp", by: num(divBy) }]
    },
    {
      title: "Multiply Boss Exp",
      hint: "Exp Multiplier times this, for monsters with Game Limit = 1 (which are probably bosses). The Universal Modifier does the same with other settings.",
      inputs: [h8("label", {}, "Times ", bossBy)],
      request: () => [
        "/api/tools/modify",
        {
          table: "Monsters",
          action: { kind: "math", columns: ["Exp Multiplier"], op: "mul", value: num(bossBy), limit: null },
          only_if: [{ kind: "column", column: "Game Limit", cmp: "eq", value: 1 }]
        }
      ]
    },
    {
      title: "Combine Exp and Exp Multi Fields",
      hint: "Experience \xD7 Exp Multiplier into Experience, multiplier 1 (fixes the experience of monsters killed by damage over time). A multiplier of 0 or less counts as 1.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "combine_exp" }]
    },
    {
      title: "Set Last Killed Times to Date",
      hint: "Every monster last killed on this date, at 01:01:02.",
      inputs: [h8("label", {}, "Date ", date)],
      request: () => {
        const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date.value);
        if (!m) throw new Error("Pick a date.");
        return ["/api/tools/preset", { preset: "set_kill_times", year: Number(m[1]), month: Number(m[2]), day: Number(m[3]) }];
      }
    },
    {
      title: "Reset Last Killed Times to Zero",
      hint: "Every monster's last killed date and time to 0.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "reset_kill_times" }]
    },
    {
      title: "Restock All Shops",
      hint: "Every shop item's stock to its max, where its regen number, regen % and regen time are all set. Gang shops are left alone.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "restock_shops" }]
    },
    {
      title: "Combine Like Items on Floor",
      hint: "In every room, items with the same number and uses (on the floor, or hidden) are put in one slot.",
      inputs: [],
      request: () => ["/api/tools/preset", { preset: "combine_room_items" }]
    },
    {
      title: "Insert Buffer Rooms",
      hint: `Adds a "Buffer Room" for every room number missing below a map's last room, so rooms keep loading (and sweeping) past gaps of 50. Leave the map empty for maps 1 to 18.`,
      inputs: [h8("label", {}, "Map ", padMap)],
      request: () => ["/api/tools/preset", { preset: "insert_buffer_rooms", map: mapOrAll(padMap) }]
    },
    {
      title: "Delete Buffer Rooms",
      hint: 'Deletes every room whose name begins with "Buffer Room". Leave the map empty for every map.',
      inputs: [h8("label", {}, "Map ", unpadMap)],
      request: () => ["/api/tools/preset", { preset: "delete_buffer_rooms", map: mapOrAll(unpadMap) }]
    },
    {
      title: "Strip Characters off Text Blocks",
      hint: "Takes this text off the end of every text block part that ends with it. Use it for strange characters at the end of every text block. ESC is \u241B.",
      inputs: [h8("label", {}, "Text ", strip)],
      request: () => {
        if (!strip.value) throw new Error("Give the text to take off the end.");
        return ["/api/tools/preset", { preset: "strip_textblocks", text: toWire(strip.value) }];
      }
    },
    {
      title: "Modify Item Drop Percentages",
      hint: "Change the drop % of monsters' item drops, by the kind of item. A drop already at the value is left alone, as NMR leaves it.",
      inputs: [
        h8("div", { class: "toolbar" }, dropMode, dropValue, h8("span", {}, " only if the drop % is "), dropCmp, dropLimit),
        h8("div", { class: "toolbar" }, ...types.map(([, l]) => l)),
        h8("div", { class: "toolbar" }, dropLimited, delMaintLabel),
        h8("div", { class: "toolbar" }, regenLabel, regen)
      ],
      request: () => {
        const [w, a, k2, c, sc] = types.map(([i]) => i.checked);
        return [
          "/api/tools/preset",
          {
            preset: "item_drop_pct",
            set: dropMode.value === "set",
            value: num(dropValue),
            only_if: [dropCmp.value, num(dropLimit)],
            limited: dropLimited.value,
            exclude_del_at_maint: delMaint.checked,
            min_regen: regenOn.checked ? num(regen) : null,
            weapons: w,
            armour: a,
            keys: k2,
            containers: c,
            scrolls: sc
          }
        ];
      }
    }
  ];
}
function fixesCard(changed2) {
  const card2 = h8("section", { class: "tool" }, h8("h2", {}, "Fixes"));
  card2.append(h8("p", { class: "hint" }, "NMR's one-click fixes from its Items and Monsters menus. Each previews first; Run writes."));
  const choose = h8("select", {});
  const list2 = fixes();
  list2.forEach((f2, i) => choose.append(h8("option", { value: String(i) }, f2.title)));
  const body = h8("div");
  const out = h8("div");
  const preview = h8("button", { type: "button" }, "Preview");
  const go = h8("button", { type: "button", class: "primary" }, "Run");
  const show2 = () => {
    const f2 = list2[Number(choose.value)];
    body.replaceChildren(h8("p", { class: "hint" }, f2.hint), ...f2.inputs.length ? [h8("div", { class: "fix-inputs" }, ...f2.inputs)] : []);
    out.replaceChildren();
  };
  choose.addEventListener("change", show2);
  show2();
  card2.append(h8("div", { class: "toolbar" }, h8("label", {}, "Fix: ", choose)), body, h8("div", { class: "toolbar" }, preview, go), out);
  const run = async (dryRun) => {
    const f2 = list2[Number(choose.value)];
    try {
      const [url, req] = f2.request();
      if (!dryRun && !confirm(`${f2.title}: write the changes? Preview first to see them.`)) return;
      out.replaceChildren(h8("p", {}, dryRun ? "Previewing\u2026" : "Changing\u2026"));
      const res = await post(url, JSON.stringify({ ...req, dry_run: dryRun }), true);
      out.replaceChildren(modifyResult(res));
      if (!dryRun) changed2();
    } catch (e) {
      out.replaceChildren(failure(e));
    }
  };
  preview.addEventListener("click", () => void run(true));
  go.addEventListener("click", () => void run(false));
  return card2;
}
function toolsPage(changed2) {
  const page = h8("div", { class: "tools" }, importer(changed2), deleter(changed2), mergeUsers(), mmeExport(failure), writeDat());
  api.tables().then((tables) => {
    page.append(universalModifier(tables, changed2));
    const rooms = tables.find((t2) => t2.name === "Rooms");
    if (rooms) page.append(massRoomEditor(rooms, (url, body) => post(url, body, true), modifyResult, changed2));
    page.append(fixesCard(changed2));
    page.append(indexChanger((url, body) => post(url, body, true), modifyResult, failure, changed2));
    page.append(recordNumberChanger((url, body) => post(url, body, true), modifyResult, failure, changed2));
  }).catch((e) => page.append(failure(e)));
  return page;
}

// src/users.ts
function nextNumber(text, at) {
  let start = at;
  let y = at;
  for (; y < text.length; y++) {
    const c = text[y];
    if (c === "*") start = y + 1;
    else if (!/[0-9 ]/.test(c)) break;
  }
  if (y <= start) return "";
  const digits = text.slice(start, y).replace(/ /g, "");
  return digits === "" ? "0" : String(Number(digits));
}
function nameAfter(text, label, next) {
  const i = text.indexOf(label);
  if (i < 0) return "";
  const x = i + label.length;
  let y = text.indexOf(next, x);
  if (y > x + 15 || y >= 0 && text.slice(x, y).trim().includes("\n")) y = -1;
  if (y < 0) y = text.indexOf("\r", x);
  if (y < 0) y = text.indexOf("\n", x);
  return y > x ? text.slice(x, y).trim() : "";
}
function parseStats(text) {
  const values = {};
  const take = (label, columns, min = 1) => {
    const i = text.indexOf(label);
    if (i < 0) return;
    const n = nextNumber(text, i + label.length);
    if (n === "" || Number(n) < min) return;
    for (const c of columns) values[c] = n;
  };
  for (const [label, stat] of [
    ["Intellect: ", "INT"],
    ["Willpower: ", "WIL"],
    ["Strength: ", "STR"],
    ["Health: ", "HEA"],
    ["Agility: ", "AGL"],
    ["Charm: ", "CHM"]
  ]) {
    take(label, [`Base ${stat}`, stat]);
  }
  const exp = text.indexOf("Exp: ");
  if (exp >= 0) {
    const n = nextNumber(text, exp + 5);
    if (n !== "") {
      const e = Number(n);
      values["Exp Billions"] = String(Math.floor(e / 1e9));
      values["Exp"] = String(e % 1e9);
    }
  }
  take("Level: ", ["Level"]);
  take("Hits: ", ["HP"]);
  take("Mana: ", ["Mana"]);
  take("Kai: ", ["Mana"]);
  take("Lives/CP:", ["Lives"]);
  const cp = text.indexOf("CP:");
  if (cp >= 0) {
    const slash = text.indexOf("/", cp + 3);
    if (slash >= 0 && slash + 1 < text.length) {
      const n = nextNumber(text, slash + 1);
      if (n !== "") values["CP"] = n;
    }
  }
  return { values, race: nameAfter(text, "Race: ", "Exp:"), cls: nameAfter(text, "Class: ", "Level:") };
}
var STATS = ["INT", "WIL", "STR", "HEA", "AGL", "CHM"];
var STAT_COLUMNS = [...STATS.map((s) => `Base ${s}`), ...STATS];
var range = (n) => Array.from({ length: n }, (_, i) => i);
var SINGLES = [
  ["BBSName", "BBS Name"],
  ["FirstName", "First Name", "name"],
  ["LastName", "Last Name", "name"],
  ["Race", "Race"],
  ["Class", "Class"],
  ["Experience", ""],
  ["Level", "Level"],
  ["Title", "Title"],
  ["EvilPoints", "Evil Points"],
  ["GangName", "Gang", "gang"],
  ["SuicidePassword", "Suicide Password", "deets"],
  ["BroadcastChan", "Broadcast Channel", "deets"],
  ["bEDITED", "Edited"],
  ["Runic", "Runic", "deets"],
  ["Platinum", "Platinum", "deets"],
  ["Gold", "Gold", "deets"],
  ["Silver", "Silver", "deets"],
  ["Copper", "Copper", "deets"],
  ["MaxHP", "Max HP"],
  ["CurrentHP", "HP"],
  ["HPRolls", "HP Rolls"],
  ["MaxMana", "Max Mana"],
  ["CurrentMana", "Mana"],
  ["SpellCasting", "Spellcasting"],
  ["LivesRemaining", "Lives", "deets"],
  ["CPRemaining", "CP"],
  ["Perception", "Perception"],
  ["Thievery", "Thievery"],
  ["Traps", "Traps"],
  ["Picklocks", "Picklocks"],
  ["Tracking", "Tracking"],
  ["MartialArts", "Martial Arts"],
  ["MagicRes", "Max MR"],
  ["Stealth", "Stealth"],
  ["CurrentENC", "Weight"],
  ["MaxENC", "Max Weight"],
  ["WeaponHand", "Weapon"],
  ["RoomNum", "Room Number", "rooms"],
  ["MapNumber", "Map Number", "rooms"]
];
var LISTS = [
  ["STATS", 12, (i) => [STAT_COLUMNS[i]]],
  ["ABILS", 30, (i) => [`Ability ${i}`, `Ability ${i} Value`]],
  ["WORN", 20, (i) => [`Worn ${i}`]],
  ["ITEMS", 100, (i) => [`Item ${i}`, `Item ${i} Uses`]],
  ["KEYS", 50, (i) => [`Key ${i}`, `Key ${i} Uses`]],
  ["SPELLS", 100, (i) => [`Spellbook ${i}`]],
  ["AURAS", 10, (i) => [`Spell Cast ${i}`, `Spell Cast ${i} Value`, `Spell Cast ${i} Rounds`]],
  ["ROOMS", 20, (i) => [`Trail ${i} Map`, `Trail ${i} Room`], "rooms"]
];
function exportUser(v) {
  const s = (c) => String(v[c] ?? 0);
  const exp = BigInt(Math.trunc(Number(v["Exp Billions"] ?? 0))) * 1000000000n + BigInt(Math.trunc(Number(v["Exp"] ?? 0)));
  const lines = [];
  const list2 = (key) => {
    const [, n, cols] = LISTS.find(([k2]) => k2 === key);
    lines.push(`${key}:${range(n).map((i) => cols(i).map(s).join("|")).join(",")}`);
  };
  for (const [key, col] of SINGLES) {
    if (key === "CurrentENC") list2("STATS"), list2("ABILS");
    if (key === "RoomNum") list2("WORN"), list2("ITEMS"), list2("KEYS"), list2("SPELLS"), list2("AURAS");
    lines.push(`${key}:${key === "Experience" ? exp.toString() : s(col).split("\0")[0]}`);
  }
  list2("ROOMS");
  return lines.join("\r\n");
}
var IMPORT_DEFAULTS = {
  name: false,
  gang: false,
  deets: false,
  rooms: false,
  clearItems: true,
  clearKeys: false,
  clearSpells: true,
  clearAbils: false
};
function importUser(text, o) {
  const out = {};
  const zero = (key) => {
    const [, n, cols] = LISTS.find(([k2]) => k2 === key);
    for (const i of range(n)) for (const c of cols(i)) out[c] = "0";
  };
  if (o.clearItems) zero("ITEMS"), zero("WORN");
  if (o.clearKeys) zero("KEYS");
  if (o.clearSpells) zero("SPELLS"), zero("AURAS");
  if (o.clearAbils) zero("ABILS");
  const allowed = (need) => !need || o[need];
  for (const line of text.trim().split(/\r?\n/)) {
    const m = /^([^\r\n:]+):(.*)$/.exec(line);
    if (!m) continue;
    const [, key, raw] = m;
    const value = raw.trim();
    const single = SINGLES.find(([k2]) => k2 === key);
    if (single && key !== "BBSName") {
      const [, col, need2] = single;
      if (!allowed(need2)) continue;
      if (key === "Experience") {
        const e = /^\d+$/.test(value) ? BigInt(value) : 0n;
        out["Exp Billions"] = (e / 1000000000n).toString();
        out["Exp"] = (e % 1000000000n).toString();
      } else out[col] = value;
      continue;
    }
    const list2 = LISTS.find(([k2]) => k2 === key);
    if (!list2) continue;
    const [, n, cols, need] = list2;
    if (!allowed(need)) continue;
    value.split(",").slice(0, n).forEach((entry, i) => {
      const parts = entry.split("|");
      const want = cols(i);
      if (want.length > 1 && parts.length !== want.length) return;
      want.forEach((c, j) => out[c] = parts[j].trim());
    });
  }
  return out;
}
var PASTE_CHAR = /[a-z()\-_,: '".`0-9]/i;
var EQUIP = {
  HEAD: 0,
  EARS: 1,
  NECK: 2,
  BACK: 3,
  TORSO: 4,
  ARMS: 5,
  WRIST: 6,
  WAIST: 7,
  HANDS: 8,
  FINGER: 9,
  LEGS: 11,
  FEET: 12,
  WORN: 13,
  "OFF-HAND": 14,
  WEAPONHAND: 15,
  TWOHANDED: 15
};
var COINS = [
  ["platinumpieces", "Platinum"],
  ["goldcrowns", "Gold"],
  ["silvernobles", "Silver"],
  ["copperfarthings", "Copper"]
];
function parseInventory(text) {
  const out = { items: [], equipped: Array(16).fill(""), coins: {} };
  let t2 = "";
  let amount = "";
  let paren = -1;
  const store = () => {
    const rest = t2.slice(amount.length);
    const coin = COINS.find(([c]) => c === rest);
    if (coin) out.coins[coin[1]] = amount;
    else if (rest.startsWith("runiccoin")) out.coins["Runic"] = amount;
    else if (amount !== "" && t2 !== "") out.items.push({ name: rest, amount });
    else out.items.push({ name: t2, amount: "" });
    clear();
  };
  const clear = () => {
    t2 = "";
    amount = "";
    paren = -1;
  };
  for (let p = 0; p < text.length; p++) {
    const c = text[p];
    if (!PASTE_CHAR.test(c)) continue;
    t2 = (t2 + c).replace(/ /g, "");
    const marker = ["isequippedwith:", "arecarrying", "followingkeys:", "wealth:"].find((m) => t2.toLowerCase().includes(m));
    if (marker) {
      t2 = t2.slice(0, t2.length - marker.length);
      store();
      continue;
    }
    if (/[0-9]/.test(c) && t2.length === amount.length + 1) amount += c;
    if (c === "," || c === ".") {
      t2 = t2.slice(0, -1);
      store();
      continue;
    }
    if (c === "(") paren = t2.length;
    else if (c === ")") {
      if (paren >= 0) {
        const place = EQUIP[t2.slice(paren, t2.length - 1).toUpperCase()];
        const name = t2.slice(0, paren - 1);
        if (place === 9 && out.equipped[9] !== "") out.equipped[10] = name;
        else if (place !== void 0) out.equipped[place] = name;
      }
      clear();
      continue;
    }
    if (p === text.length - 1) store();
  }
  return out;
}
var num2 = (v, c) => Number(v[c] ?? 0) || 0;
function placeInventory(p, items, current, clear) {
  const item = range(100).map((i) => num2(current, `Item ${i}`));
  const itemUses = range(100).map((i) => num2(current, `Item ${i} Uses`));
  const key = range(50).map((i) => num2(current, `Key ${i}`));
  const keyUses = range(50).map((i) => num2(current, `Key ${i} Uses`));
  let worn = range(20).map((i) => num2(current, `Worn ${i}`));
  let weapon = num2(current, "Weapon");
  const equipped = [...p.equipped];
  if (clear) {
    item.fill(0), itemUses.fill(-1), key.fill(0), keyUses.fill(-2);
    worn = worn.map(() => 0);
    weapon = 0;
  } else if (equipped.some((e) => e !== "")) {
    worn = worn.map(() => 0);
    weapon = 0;
  }
  const names2 = p.items.map((i) => i.name);
  const found = names2.map(() => ["", []]);
  let curItem = 0;
  let curKey = 0;
  let curWorn = 0;
  const addItem = (it) => {
    while (curItem < 100 && item[curItem] !== 0) curItem++;
    if (curItem >= 100) return;
    item[curItem] = it.number;
    if ((it.uses ?? 0) > 0) itemUses[curItem] = it.uses;
    curItem++;
  };
  for (const it of items) {
    const name = it.name.split("\0")[0].replace(/ /g, "");
    if (name === "") continue;
    equipped.forEach((e, x) => {
      if (e !== name) return;
      if (x === 15) weapon = it.number;
      else if (curWorn < 20) worn[curWorn++] = it.number;
      addItem(it);
      equipped[x] = "";
    });
    names2.forEach((n, x) => {
      if (n !== name) return;
      const times = Math.max(1, Math.trunc(Number(p.items[x].amount) || 0));
      for (let y = 0; y < times; y++) {
        if (it.type === 7) {
          while (curKey < 50 && key[curKey] !== 0) curKey++;
          if (curKey < 50) {
            key[curKey] = it.number;
            keyUses[curKey] = it.uses ?? 0;
            curKey++;
          }
        } else addItem(it);
      }
      found[x] = [name, [it.number]];
      names2[x] = "";
    });
    const dup = found.find(([n, nums]) => n === name && nums[0] !== it.number);
    if (dup) dup[1].push(it.number);
  }
  const values = {};
  range(100).forEach((i) => (values[`Item ${i}`] = String(item[i]), values[`Item ${i} Uses`] = String(itemUses[i])));
  range(50).forEach((i) => (values[`Key ${i}`] = String(key[i]), values[`Key ${i} Uses`] = String(keyUses[i])));
  range(20).forEach((i) => values[`Worn ${i}`] = String(worn[i]));
  values["Weapon"] = String(weapon);
  const has = (n) => item.includes(n);
  range(20).forEach((i) => worn[i] > 0 && !has(worn[i]) && (values[`Worn ${i}`] = "0"));
  if (weapon > 0 && !has(weapon)) values["Weapon"] = "0";
  Object.assign(values, p.coins);
  const ambiguous = found.filter(([, nums]) => nums.length > 1).map(([name, numbers]) => ({ name, numbers }));
  return { values, ambiguous };
}
function chooseItem(values, from, to) {
  for (const i of range(100)) if (values[`Item ${i}`] === String(from)) values[`Item ${i}`] = String(to.number), values[`Item ${i} Uses`] = String(to.uses ?? 0);
  for (const i of range(50)) if (values[`Key ${i}`] === String(from)) values[`Key ${i}`] = String(to.number), values[`Key ${i} Uses`] = String(to.uses ?? 0);
  for (const i of range(20)) if (values[`Worn ${i}`] === String(from)) values[`Worn ${i}`] = String(to.number);
  if (values["Weapon"] === String(from)) values["Weapon"] = String(to.number);
}
function parseSpellbook(text) {
  const out = [];
  for (let line of text.split(/\r|\n/)) {
    for (const h10 of ["you have the following spells:", "level mana short spell name"]) {
      const at = line.toLowerCase().lastIndexOf(h10);
      if (at >= 0) line = line.slice(at + h10.length);
    }
    if (line.length > 17) out.push(line.slice(17));
  }
  return out;
}
function placeSpells(names2, spells, current, clear) {
  const book = range(100).map((i) => clear ? 0 : num2(current, `Spellbook ${i}`));
  const want = [...names2];
  const found = want.map(() => ["", []]);
  let cur = 0;
  for (const s of spells) {
    const name = s.name.split("\0")[0];
    if (name === "") continue;
    want.forEach((n, x) => {
      if (n !== name) return;
      while (cur < 100 && book[cur] !== 0) cur++;
      if (cur >= 100) return;
      book[cur++] = s.number;
      found[x] = [name, [s.number]];
      want[x] = "";
    });
    const dup = found.find(([n, nums]) => n === name && nums[0] !== s.number);
    if (dup) dup[1].push(s.number);
  }
  const values = {};
  range(100).forEach((i) => values[`Spellbook ${i}`] = String(book[i]));
  const ambiguous = found.filter(([, nums]) => nums.length > 1).map(([name, numbers]) => ({ name, numbers }));
  return { values, ambiguous };
}

// src/main.ts
var ORDER = ["Items", "Monsters", "Spells", "Classes", "Races", "Shops", "Rooms", "Messages", "Textblocks", "Actions", "Users", "BankBooks", "Gangs"];
var LIST_LIMIT = 500;
var state = {
  tables: [],
  table: null,
  layout: [],
  rows: [],
  filter: "",
  row: null,
  edits: /* @__PURE__ */ new Map(),
  tab: 0,
  names: /* @__PURE__ */ new Map()
};
var $ = (sel) => document.querySelector(sel);
function h9(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [k2, v] of Object.entries(props)) {
    if (v === void 0) continue;
    if (k2 === "class") el.className = v;
    else el.setAttribute(k2, v);
  }
  for (const c of children) if (c !== null) el.append(c);
  return el;
}
var show = (v) => v === null ? "" : toShown(String(v));
var keyText = (key) => key.map(show).join("/");
var sameKey = (a, b) => JSON.stringify(a) === JSON.stringify(b);
function status(text, kind = "info") {
  const el = $("#status");
  el.textContent = text;
  el.className = kind;
}
function fail(e) {
  status(e instanceof Error ? e.message : String(e), "error");
}
function column(name) {
  return state.table?.columns.find((c) => c.name === name);
}
function changed() {
  const out = {};
  if (!state.row) return out;
  for (const [col, typed] of state.edits) {
    if (typed !== show(state.row.values[col] ?? null)) out[col] = toWire(typed);
  }
  return out;
}
var dirty = () => Object.keys(changed()).length > 0;
function confirmLeave() {
  return !dirty() || confirm("Discard the changes to this record?");
}
function refreshDirty() {
  const d = changed();
  for (const el of document.querySelectorAll("[data-col]")) {
    el.classList.toggle("changed", el.dataset.col in d);
  }
  for (const b of document.querySelectorAll(".needs-dirty")) b.disabled = !dirty();
  const n = Object.keys(d).length;
  $("#dirty").textContent = n ? `${n} unsaved change${n === 1 ? "" : "s"}` : "";
}
async function names(table) {
  let m = state.names.get(table);
  if (m) return m;
  m = /* @__PURE__ */ new Map();
  state.names.set(table, m);
  try {
    const list2 = await api.rows(table);
    for (const [key, title] of list2.rows) {
      if (typeof key[0] === "number") m.set(key[0], show(title));
    }
  } catch {
  }
  return m;
}
function pickName(pick2, value) {
  if (!pick2 || value.trim() === "") return "";
  const n = Number(value);
  if ("abilities" in pick2) return n === 0 ? "" : ABILITIES[n]?.[0] ?? "unknown ability";
  if ("ref" in pick2) return n === 0 ? "" : state.names.get(pick2.ref)?.get(n) ?? "none such";
  return "";
}
function renderNav() {
  const nav = $("#tables");
  nav.replaceChildren();
  const ordered = [...state.tables].sort(
    (a, b) => (ORDER.indexOf(a.name) + 1 || 99) - (ORDER.indexOf(b.name) + 1 || 99)
  );
  for (const t2 of ordered) {
    const a = h9("a", { href: `#/${encodeURIComponent(t2.name)}` }, t2.name);
    if (t2 === state.table && !onPage()) a.classList.add("active");
    nav.append(a);
  }
  const map = h9("a", { href: "#/map", class: "tools-link" }, "Map");
  const lists = h9("a", { href: "#/lists" }, "Lists");
  const tools = h9("a", { href: "#/tools" }, "Tools");
  if (onPage() === "map") map.classList.add("active");
  if (onPage() === "lists") lists.classList.add("active");
  if (onPage() === "tools") tools.classList.add("active");
  nav.append(map, lists, tools);
}
var PAGES = ["tools", "lists", "map"];
var onPage = () => PAGES.find((p) => location.hash.startsWith(`#/${p}`)) ?? null;
var theMap = null;
var lastPage = null;
function showPage(which) {
  $("main").hidden = which !== null;
  for (const id of PAGES) {
    const page = $(`#${id}`);
    page.hidden = which !== id;
    if (id === "map" && which === "map") {
      if (!theMap) {
        theMap = mapPage();
        page.append(theMap.el);
      }
      void theMap.show(lastPage !== "map");
      continue;
    }
    if (which === id && !page.childElementCount) {
      page.append(
        id === "lists" ? listsPage() : toolsPage(() => {
          state.table = null;
          state.row = null;
          state.names.clear();
        })
      );
    }
  }
  lastPage = which;
}
function renderList() {
  const list2 = $("#rows");
  list2.replaceChildren();
  const needle = state.filter.trim().toLowerCase();
  const matches = state.rows.filter(
    ([key, title]) => !needle || `${keyText(key)} ${show(title)}`.toLowerCase().includes(needle)
  );
  for (const [key, title] of matches.slice(0, LIST_LIMIT)) {
    const li = h9("li", {}, h9("span", { class: "key" }, keyText(key)), " ", show(title).trimEnd());
    if (state.row && sameKey(state.row.key, key)) li.classList.add("selected");
    li.addEventListener("click", () => select(key));
    list2.append(li);
  }
  $("#count").textContent = matches.length > LIST_LIMIT ? `${LIST_LIMIT} of ${matches.length}; filter to narrow` : `${matches.length} of ${state.rows.length}`;
}
function input2(field2, info) {
  const col = field2.col;
  const value = state.edits.get(col) ?? "";
  const isKey = state.table?.key.includes(col) ?? false;
  const pick2 = field2.pick;
  let control;
  if (pick2 && "options" in pick2) {
    const select2 = h9("select", { "data-col": col });
    const options2 = { ...pick2.options };
    if (value !== "" && !(Number(value) in options2)) options2[Number(value)] = `${value} (unknown)`;
    for (const [n, label] of Object.entries(options2)) {
      select2.append(h9("option", { value: n }, `${n}: ${label}`));
    }
    select2.value = value;
    control = select2;
  } else if (info?.type === "memo") {
    control = h9("textarea", { "data-col": col, rows: "8", spellcheck: "false" });
    control.value = value;
  } else {
    control = h9("input", {
      "data-col": col,
      type: "text",
      spellcheck: "false",
      inputmode: info?.type === "text" ? void 0 : "numeric",
      maxlength: info?.size ? String(info.size) : void 0
    });
    control.value = value;
    if (info?.type !== "text") control.classList.add("number");
  }
  if (isKey) {
    control.setAttribute("disabled", "");
    control.title = "The record\u2019s key: insert a new record to use another";
  }
  const name = h9("span", { class: "pick-name" }, pickName(pick2, value));
  if (pick2 && "abilities" in pick2) name.title = ABILITIES[Number(value)]?.[1] ?? "";
  control.addEventListener("input", () => {
    state.edits.set(col, control.value);
    name.textContent = pickName(pick2, control.value);
    refreshDirty();
  });
  const wrap = h9("span", { class: "control" }, control);
  if (pick2 && !("options" in pick2)) wrap.append(name);
  return wrap;
}
function renderField(field2) {
  const info = column(field2.col);
  if (!info) return h9("span");
  const label = field2.label ?? field2.col;
  return h9(
    "label",
    { class: field2.wide ? "field wide" : "field", title: field2.col },
    h9("span", { class: "label" }, label),
    input2(field2, info)
  );
}
function renderTab(tabIndex) {
  const tab = state.layout[tabIndex];
  const body = h9("div", { class: "tab-body" });
  if (!tab) return body;
  if (tab.fields) {
    const grid2 = h9("div", { class: "fields" });
    for (const field2 of tab.fields) grid2.append(renderField(field2));
    body.append(grid2);
  }
  if (tab.preview) {
    const box3 = h9("pre", { class: "terminal", "aria-label": "Preview" });
    const draw = () => {
      box3.replaceChildren();
      tab.preview.forEach((col, i) => {
        if (i) box3.append("\n");
        box3.append(renderAnsi(state.edits.get(col) ?? ""));
      });
    };
    draw();
    body.addEventListener("input", draw);
    body.append(h9("div", { class: "preview-label" }, "As players see it"), box3);
  }
  if (tab.matrix) {
    const m = tab.matrix;
    const table = h9("table", { class: "grid matrix" });
    const head = h9("tr", {}, h9("th"));
    for (const label of m.headers) head.append(h9("th", {}, label));
    table.append(h9("thead", {}, head));
    const tbody = h9("tbody");
    for (const row of m.rows) {
      const tr = h9("tr", {}, h9("td", { class: "n" }, row.label));
      for (const col of row.cols) {
        const info = column(col);
        tr.append(h9("td", { title: col }, info ? input2({ col }, info) : ""));
      }
      tbody.append(tr);
    }
    table.append(tbody);
    body.append(table);
  }
  if (tab.grid) {
    const g = tab.grid;
    if (g.across) {
      const table2 = h9("table", { class: "grid across" });
      const head2 = h9("tr", {}, h9("th"));
      for (let n = g.from; n <= g.to; n++) head2.append(h9("th", {}, `${g.across} ${n - g.from + 1}`));
      table2.append(h9("thead", {}, head2));
      const tbody2 = h9("tbody");
      for (const c of g.columns) {
        const tr = h9("tr", {}, h9("td", { class: "n" }, c.label ?? c.template));
        for (let n = g.from; n <= g.to; n++) {
          const col = c.template.replace("#", String(n));
          const info = column(col);
          tr.append(h9("td", { title: col }, info ? input2({ col, pick: c.pick }, info) : ""));
        }
        tbody2.append(tr);
      }
      table2.append(tbody2);
      body.append(table2);
      return body;
    }
    const table = h9("table", { class: "grid" });
    const head = h9("tr", {}, h9("th", {}, "#"));
    for (const c of g.columns) head.append(h9("th", {}, c.label ?? c.template));
    table.append(h9("thead", {}, head));
    const tbody = h9("tbody");
    for (let n = g.from; n <= g.to; n++) {
      const tr = h9("tr", {}, h9("td", { class: "n" }, g.names?.[n - g.from] ?? String(n)));
      for (const c of g.columns) {
        const col = c.template.replace("#", String(n));
        const info = column(col);
        tr.append(h9("td", {}, info ? input2({ col, pick: c.pick }, info) : ""));
      }
      tbody.append(tr);
    }
    table.append(tbody);
    body.append(table);
  }
  return body;
}
function renderEditor() {
  const ed = $("#editor");
  ed.replaceChildren();
  const t2 = state.table;
  if (!t2) return;
  if (!state.row) {
    ed.append(h9("p", { class: "empty" }, "Pick a record on the left, or insert a new one."));
    ed.append(toolbar(t2));
    return;
  }
  const title = t2.title ? titleOf(show(state.row.values[t2.title] ?? null)) : "";
  ed.append(h9("h2", {}, `${t2.name} ${keyText(state.row.key)}`, title ? ` \u2014 ${title}` : ""));
  ed.append(toolbar(t2));
  const tabs = h9("div", { class: "tabs", role: "tablist" });
  state.layout.forEach((tab, i) => {
    const b = h9("button", { role: "tab", type: "button" }, tab.title);
    if (i === state.tab) b.classList.add("active");
    b.addEventListener("click", () => {
      state.tab = i;
      renderEditor();
    });
    tabs.append(b);
  });
  ed.append(tabs, renderTab(state.tab));
  refreshDirty();
}
function toolbar(t2) {
  const bar = h9("div", { class: "toolbar" });
  const button = (label, title, run, cls = "") => {
    const b = h9("button", { type: "button", title, class: cls }, label);
    b.addEventListener("click", run);
    bar.append(b);
    return b;
  };
  if (state.row) {
    button("Save", "Write the changes to the realm (Ctrl+S)", () => void save2(), "needs-dirty primary");
    button("Discard", "Undo the unsaved changes", discard, "needs-dirty");
  }
  button("Insert", state.row ? `Copy this record to a new ${t2.name} record` : `Add a new ${t2.name} record`, () => void insert());
  if (state.row) button("Delete", "Remove this record from the realm", () => void remove(), "danger");
  if (state.row && t2.name === "Rooms") {
    const [map, room] = state.row.key;
    bar.append(h9("a", { href: `#/map/${map}/${room}`, class: "button-link" }, "Map"));
  }
  if (state.row && t2.name === "Users") {
    button("Revive", "NMR's Revive Character: HP and Mana to full, then lives, spells cast and where to move them", () => void revive());
    button("Paste Stats", "NMR's Paste Character: read a capture of the game's stat output into the fields", () => void pasteStats());
    button("Paste Items", "NMR's Paste Items/Keys/Worn/$$: read a capture of the game's inventory into the lists", () => void pasteItems());
    button("Paste Spells", "NMR's Paste Spellbook: read a capture of the game's spellbook into the spellbook", () => void pasteSpells());
    button("Copy User", "NMR's Copy User: this user onto another BBS name, new or existing", () => void copyUser());
    button("Export", "NMR's character export: copy this user as text", () => void exportCharacter());
    button("Import", "NMR's character import: fill this user from exported text", () => void importCharacter());
  }
  bar.append(h9("span", { id: "dirty" }));
  return bar;
}
function loadEdits(row) {
  state.edits = new Map(Object.entries(row.values).map(([k2, v]) => [k2, show(v)]));
}
async function openTable(name, key) {
  const t2 = state.tables.find((x) => x.name === name);
  if (!t2) return;
  if (t2 !== state.table) {
    state.table = t2;
    const layout = layoutFor(t2);
    const rest = t2.columns.map((c) => c.name).filter((c) => !columnsShown(layout).has(c));
    state.layout = rest.length ? [...layout, { title: "More", fields: rest.map((col) => ({ col })) }] : layout;
    state.tab = 0;
    state.row = null;
    state.filter = "";
    $("#filter").value = "";
    renderNav();
    status(`Loading ${t2.name}\u2026`);
    const list2 = await api.rows(t2.name);
    state.rows = list2.rows;
    status(`${t2.name}: ${list2.rows.length} records`);
    const refs = /* @__PURE__ */ new Set();
    for (const tab of state.layout) {
      for (const f2 of tab.fields ?? []) if (f2.pick && "ref" in f2.pick) refs.add(f2.pick.ref);
      for (const c of tab.grid?.columns ?? []) if (c.pick && "ref" in c.pick) refs.add(c.pick.ref);
    }
    await Promise.all([...refs].map(names));
  }
  if (key) {
    try {
      state.row = await api.row(t2.name, key);
      loadEdits(state.row);
    } catch (e) {
      state.row = null;
      fail(e);
    }
  }
  renderList();
  renderEditor();
}
async function revive() {
  const row = state.row;
  if (!row || !state.table) return;
  if (Object.keys(changed()).length) {
    status("Save or discard your changes first.", "error");
    return;
  }
  const v = (c) => Number(row.values[c] ?? 0);
  const lives = v("Lives") < 9 && confirm("Reset Lives to 9?");
  const cast = v("Poison") !== 0 || Array.from({ length: 10 }, (_, i) => v(`Spell Cast ${i}`)).some((x) => x > 0);
  const spells = cast && confirm("Reset Auras (clear the spells cast on them)?");
  let to = null;
  if (v("Map Number") !== 1 || v("Room Number") !== 2189) {
    const answer = prompt(
      "Move user?\n- Enter 1-19: move them that many rooms back\n- Enter 99: to the Halls of the Dead\n- Enter Map/Room, as 1/2189\n\nEnter 0 or cancel to leave them alone.",
      "0"
    );
    const mr = /^\s*(\d+)\s*\/\s*(\d+)\s*$/.exec(answer ?? "");
    const n = parseInt(answer ?? "", 10) || 0;
    if (mr) to = { room: { map: Number(mr[1]), room: Number(mr[2]) } };
    else if (n === 99) to = "halls";
    else if (n >= 1 && n <= 19) to = { back: n };
  }
  try {
    await api.preset({ preset: "revive_character", user: String(row.key[0]), lives, spells, to });
    const fresh = await api.row(state.table.name, row.key);
    state.row = fresh;
    loadEdits(fresh);
    renderEditor();
    status(`Revived ${row.key[0]}.`, "ok");
  } catch (e) {
    fail(e);
  }
}
async function textDialog(intro, text, buttons, extra) {
  const form2 = h9(
    "form",
    { method: "dialog" },
    h9("p", {}, intro),
    text,
    ...extra ? [extra] : [],
    h9("div", { class: "toolbar" }, ...buttons.map((b, i) => h9("button", { type: "submit", value: b, class: i === 0 ? "primary" : "" }, b)))
  );
  const dialog = h9("dialog", { class: "paste-dialog" }, form2);
  document.body.append(dialog);
  dialog.showModal();
  const answer = await new Promise((done) => dialog.addEventListener("close", () => done(dialog.returnValue), { once: true }));
  dialog.remove();
  return answer;
}
var textBox = (label, value = "") => {
  const t2 = h9("textarea", { rows: "14", cols: "80", spellcheck: "false", "aria-label": label });
  t2.value = value;
  return t2;
};
function fillFields(values, what) {
  for (const [col, v] of Object.entries(values)) if (column(col)) state.edits.set(col, v);
  renderEditor();
  const n = Object.keys(values).length;
  status(n ? `${what}: ${n} fields filled in; save to keep them.` : `${what}: nothing in the text was recognised.`, n ? "ok" : "error");
}
async function pasteStats() {
  const text = textBox("Stat capture");
  const intro = "Paste a capture of a character's stat output (or MMUD Explorer's Copy Only Stats). Class, race, level, exp, lives, CP, HP, mana and the six stats are read.";
  if (await textDialog(intro, text, ["Paste", "Cancel"]) !== "Paste" || text.value.trim().length < 10) return;
  const { values, race, cls } = parseStats(text.value);
  for (const [table, name, col] of [
    ["Races", race, "Race"],
    ["Classes", cls, "Class"]
  ]) {
    if (!name) continue;
    for (const [n, title] of await names(table)) if (title === name) values[col] = String(n);
  }
  fillFields(values, "Paste Stats");
}
async function named(table) {
  const cols = table === "Items" ? ["Number", "Name", "Type", "Uses"] : ["Number", "Name"];
  const { rows } = await api.rowsWith(table, cols);
  return rows.map(([, , [n, name, type, uses]]) => ({ number: Number(n), name: String(name ?? ""), type: Number(type ?? 0), uses: Number(uses ?? 0) })).sort((a, b) => a.number - b.number);
}
async function chooseAmong(ambiguous, records, what) {
  const chosen = /* @__PURE__ */ new Map();
  if (!ambiguous.length) return chosen;
  const byNumber = new Map(records.map((r) => [r.number, r]));
  const selects = ambiguous.map((a) => {
    const sel = h9("select", {}, ...a.numbers.map((n) => h9("option", { value: String(n) }, `${n}  ${byNumber.get(n)?.name ?? ""}`)));
    return [a, sel];
  });
  const form2 = h9(
    "form",
    { method: "dialog" },
    h9("p", {}, `More than one ${what} has these names. Which is it?`),
    ...selects.map(([a, sel]) => h9("label", { class: "choose" }, `${a.name} `, sel)),
    h9("div", { class: "toolbar" }, h9("button", { type: "submit", value: "OK", class: "primary" }, "OK"), h9("button", { type: "submit", value: "Cancel" }, "Cancel"))
  );
  const dialog = h9("dialog", { class: "paste-dialog" }, form2);
  document.body.append(dialog);
  dialog.showModal();
  const answer = await new Promise((done) => dialog.addEventListener("close", () => done(dialog.returnValue), { once: true }));
  dialog.remove();
  if (answer !== "OK") return null;
  for (const [a, sel] of selects) chosen.set(a.numbers[0], byNumber.get(Number(sel.value)));
  return chosen;
}
function onlyChanged(values) {
  return Object.fromEntries(Object.entries(values).filter(([c, v]) => state.edits.get(c) !== v));
}
function currentValues() {
  return Object.fromEntries(state.edits);
}
async function pasteItems() {
  const text = textBox("Inventory capture");
  try {
    text.value = await navigator.clipboard.readText();
  } catch {
  }
  const intro = "Paste a capture of the game's inventory (i). Items, keys, worn items and coins are read. Clear First empties the items, keys and worn lists; Add puts what is found after what is there (the worn list is replaced either way when the capture names places).";
  const answer = await textDialog(intro, text, ["Clear First", "Add", "Cancel"]);
  if (answer !== "Clear First" && answer !== "Add" || text.value.length < 10) return;
  const items = await named("Items");
  const { values, ambiguous } = placeInventory(parseInventory(text.value), items, currentValues(), answer === "Clear First");
  const chosen = await chooseAmong(ambiguous, items, "item");
  if (!chosen) return;
  for (const [from, to] of chosen) if (from !== to.number) chooseItem(values, from, to);
  fillFields(onlyChanged(values), "Paste Items");
}
async function pasteSpells() {
  const text = textBox("Spellbook capture");
  try {
    text.value = await navigator.clipboard.readText();
  } catch {
  }
  const intro = "Paste a capture of the game's spellbook (spells). Clear First empties the spellbook; Add puts what is found after what is there.";
  const answer = await textDialog(intro, text, ["Clear First", "Add", "Cancel"]);
  if (answer !== "Clear First" && answer !== "Add" || text.value.length < 10) return;
  const spells = await named("Spells");
  const { values, ambiguous } = placeSpells(parseSpellbook(text.value), spells, currentValues(), answer === "Clear First");
  const chosen = await chooseAmong(ambiguous, spells, "spell");
  if (!chosen) return;
  for (const [from, to] of chosen) {
    if (from === to.number) continue;
    for (const [c, v] of Object.entries(values)) if (v === String(from)) values[c] = String(to.number);
  }
  fillFields(onlyChanged(values), "Paste Spells");
}
async function exportCharacter() {
  if (!state.row) return;
  const text = textBox("Exported character", exportUser(state.row.values));
  text.readOnly = true;
  let copied = false;
  try {
    await navigator.clipboard.writeText(text.value);
    copied = true;
  } catch {
  }
  const intro = `${copied ? "Copied to the clipboard. " : ""}NMR's character export of the saved record: Import it into another user, here or in NMR.`;
  await textDialog(intro, text, ["Close"]);
}
async function importCharacter() {
  const text = textBox("Character to import");
  try {
    text.value = await navigator.clipboard.readText();
  } catch {
  }
  const labels = [
    ["name", "Import Character Name"],
    ["gang", "Import Gang Name"],
    ["deets", "Import Lives, Suicide, Channel, Cash"],
    ["rooms", "Import Rooms"],
    ["clearItems", "Clear Items First"],
    ["clearKeys", "Clear Keys First"],
    ["clearSpells", "Clear Spells First"],
    ["clearAbils", "Clear Abilities First"]
  ];
  const boxes = labels.map(([k2, label]) => {
    const c = h9("input", { type: "checkbox" });
    c.checked = IMPORT_DEFAULTS[k2];
    return [k2, c, h9("label", {}, c, " ", label)];
  });
  const extra = h9("div", { class: "import-boxes" }, ...boxes.map(([, , l]) => l));
  const intro = "Paste a character exported from NMR's or Daydream's user editor.";
  if (await textDialog(intro, text, ["Import", "Cancel"], extra) !== "Import") return;
  const o = { ...IMPORT_DEFAULTS };
  for (const [k2, c] of boxes) o[k2] = c.checked;
  fillFields(importUser(text.value, o), "Import");
}
function select(key) {
  if (!state.table || state.row && sameKey(state.row.key, key)) return;
  if (!confirmLeave()) return;
  location.hash = `#/${encodeURIComponent(state.table.name)}/${encodeURIComponent(JSON.stringify(key))}`;
}
async function save2() {
  const t2 = state.table;
  const row = state.row;
  if (!t2 || !row) return;
  const values = changed();
  if (!Object.keys(values).length) return;
  try {
    const report = await api.save(t2.name, row.key, values);
    state.row = await api.row(t2.name, row.key);
    loadEdits(state.row);
    const i = state.rows.findIndex(([k2]) => sameKey(k2, row.key));
    if (i >= 0 && t2.title) {
      const v = state.row.values[t2.title] ?? null;
      state.rows[i] = [row.key, typeof v === "string" ? titleOf(v) : v];
    }
    state.names.delete(t2.name);
    renderList();
    renderEditor();
    status(report.updated ? `Saved ${t2.name} ${keyText(row.key)}.` : "Nothing changed in the record.", "ok");
  } catch (e) {
    fail(e);
  }
}
function discard() {
  if (!state.row) return;
  loadEdits(state.row);
  renderEditor();
  status("Changes discarded.");
}
function newKey(t2) {
  const numeric = t2.key.length === 1 && t2.columns.find((c) => c.name === t2.key[0])?.type === "long";
  if (numeric) {
    const max = state.rows.reduce((m, [k2]) => Math.max(m, Number(k2[0]) || 0), 0);
    const typed2 = prompt(`Number of the new ${t2.name} record:`, String(max + 1));
    if (typed2 === null) return null;
    const n = Number(typed2.trim());
    if (typed2.trim() === "" || !Number.isInteger(n) || n <= 0) {
      status(`${typed2} is not a record number`, "error");
      return null;
    }
    return [n];
  }
  const typed = prompt(`${t2.key.join(", ")} of the new ${t2.name} record (separate with /):`);
  if (typed === null) return null;
  const parts = typed.split("/").map((s) => s.trim());
  if (parts.length !== t2.key.length) {
    status(`A ${t2.name} key is ${t2.key.length} value(s): ${t2.key.join(", ")}`, "error");
    return null;
  }
  return parts.map((p, i) => t2.columns.find((c) => c.name === t2.key[i])?.type === "text" ? p : Number(p));
}
async function insert() {
  const t2 = state.table;
  if (!t2) return;
  if (dirty()) await save2();
  if (dirty()) return;
  const key = newKey(t2);
  if (!key) return;
  const values = {};
  if (state.row) for (const [col, typed] of state.edits) values[col] = toWire(typed);
  t2.key.forEach((c, i) => values[c] = key[i] ?? null);
  try {
    await api.insert(t2.name, values);
    const list2 = await api.rows(t2.name);
    state.rows = list2.rows;
    state.row = null;
    state.names.delete(t2.name);
    status(`Inserted ${t2.name} ${keyText(key)}.`, "ok");
    location.hash = `#/${encodeURIComponent(t2.name)}/${encodeURIComponent(JSON.stringify(key))}`;
  } catch (e) {
    fail(e);
  }
}
async function copyUser() {
  const t2 = state.table;
  const row = state.row;
  if (!t2 || !row) return;
  if (!confirm(
    "Warning, this will have VERY unpredictable results with limited items, gangs, etc and should really only be used to transfer characters between BBS accounts.\n\nAre you sure you want to continue?"
  ))
    return;
  const bbs = (prompt("Enter the BBS Name to copy to (new or existing).\n\nNOTE: enter it EXACTLY, case sensitive!", "") ?? "").trim();
  if (!bbs) return;
  if (dirty()) await save2();
  if (dirty()) return;
  const values = {};
  for (const [col, typed] of state.edits) values[col] = toWire(typed);
  const key = [bbs];
  try {
    if (state.rows.some(([k2]) => sameKey(k2, key))) {
      const target = await api.row(t2.name, key);
      delete values["BBS Name"];
      values["First Name"] = String(target.values["First Name"] ?? "").replace(/ /g, "");
      await api.save(t2.name, key, values);
    } else {
      const first = (prompt("Enter the new First Name (must be unique)", "") ?? "").replace(/ /g, "");
      if (!first) return;
      values["BBS Name"] = bbs;
      values["First Name"] = first;
      await api.insert(t2.name, values);
      state.rows = (await api.rows(t2.name)).rows;
    }
    state.names.delete(t2.name);
    status(`Copied ${keyText(row.key)} to ${bbs}.`, "ok");
    location.hash = `#/${encodeURIComponent(t2.name)}/${encodeURIComponent(JSON.stringify(key))}`;
  } catch (e) {
    fail(e);
  }
}
async function remove() {
  const t2 = state.table;
  const row = state.row;
  if (!t2 || !row) return;
  if (!confirm(`Delete ${t2.name} ${keyText(row.key)} from the realm? This cannot be undone.`)) return;
  try {
    await api.remove(t2.name, row.key);
    state.rows = state.rows.filter(([k2]) => !sameKey(k2, row.key));
    state.row = null;
    state.names.delete(t2.name);
    status(`Deleted ${t2.name} ${keyText(row.key)}.`, "ok");
    location.hash = `#/${encodeURIComponent(t2.name)}`;
    renderList();
    renderEditor();
  } catch (e) {
    fail(e);
  }
}
async function route() {
  showPage(onPage());
  if (onPage()) {
    renderNav();
    return;
  }
  const [, table, key] = location.hash.split("/");
  const name = decodeURIComponent(table ?? "") || state.tables.find((t2) => t2.name === ORDER[0])?.name;
  if (!name) return;
  let parsed;
  try {
    parsed = key ? JSON.parse(decodeURIComponent(key)) : void 0;
  } catch {
    parsed = void 0;
  }
  if (!parsed && state.table?.name === name) {
    state.row = null;
    renderList();
    renderEditor();
    return;
  }
  await openTable(name, parsed);
}
async function main() {
  $("#filter").addEventListener("input", (e) => {
    state.filter = e.target.value;
    renderList();
  });
  document.addEventListener("keydown", (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void save2();
    }
  });
  window.addEventListener("beforeunload", (e) => {
    if (dirty()) e.preventDefault();
  });
  window.addEventListener("hashchange", () => void route().catch(fail));
  try {
    const realm = await api.realm();
    $("#realm").textContent = realm.dir;
    state.tables = await api.tables();
    renderNav();
    await route();
  } catch (e) {
    fail(e);
  }
}
void main();
