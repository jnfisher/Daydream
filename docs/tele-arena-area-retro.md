# Tele-Arena area in Silvermere: what we learned

Written 2026-11-24 after adding a Tele-Arena (TA) clone to the Silvermere realm through the
daydream MCP server. It covers what was built, what the work taught us about the data and
the engine, the defects still open, and what would make the MCP server and the agent's
process better next time.

## What was built

| | Count | Numbers |
|---|---|---|
| Rooms (map 1) | 198 | 3464-3479 trail and town, 3501-3682 dungeon |
| Monsters | 47 | 1119-1165 (group 33 arena, group 34 dungeon) |
| Items | 35 | 2048-2082 |
| Shops | 4 | 183-186 |
| Existing rows changed | 1 | Room 36 gained a NW exit |

Twelve applied batches, 320 record writes, no import errors. Source: `~/repos/tele-arena`
(TSGARN 5.6d). Layout, room text, lair placement and the key chain are TA's; monster and
item stats are scaled to Silvermere.

## Open defects (not fixed yet)

Seen in the first play test of the arena:

1. **Death lines name the wrong monster.** A Messages row is a three-line bundle, and the
   third line is often a death line with the original monster's name written into it.
   Message 8572 reads "The %s slashes you for %d damage!" / "The %s slashes %s for %s
   damage!" / "The troll spits up some black blood, and dies!". Every new monster borrowed
   its Hit, Move and Death message numbers from an existing monster, so most of the 47 will
   announce the death of a troll, orc warrior, ogre and so on. Each needs its own message
   rows.
2. **Hit verbs do not match the creature.** For the same reason a mountain ape "slashes"
   and "swipes with its claws", a cave lion uses troll text, a wild boar uses wolf text.
3. **Arrival line reads "A the west lumbers in from mountain ape."** Message 8574 line 1
   is "A %s lumbers in from %s." The two arguments arrive in the opposite order. The stock
   troll uses the same message, so this may be an engine argument-order bug rather than a
   data problem. Check the stock troll before changing data.
4. **Attack rate looks high.** The ape attacked at least three times in one round. Its one
   attack costs 500 of 1000 energy, which should give two. The stock troll uses 1000 per
   attack. Check how the engine spends attack energy before tuning.

Known gaps from the build, unchanged:

- The white rune is a plain room item. Whether it restocks after pickup is unknown.
- Yarrow, rowan and hyssop potions, heartstone, manastone and the three containers were
  left out for lack of a Silvermere equivalent.
- No monster has a spell or breath attack; ogre mage and warlock only melee.
- Keys drop from every minotaur, ogre and cyclops at 50%, not only the lair ones.
- The two level 2 pits can only be left with the rope item.
- The wands and potions cast existing spells that were chosen by name, not tested.

## What we learned about the Tele-Arena source

- The dungeon under town 1 is rooms 1-182 of `TSGARNDD.MSG`. The three levels are the
  terrain ranges in `DD2-*`: rooms 1-51, 52-107, 108-182.
- **Lair monster ids are 0-based; `MNAMn` keys are 1-based.** Reading them as the same
  index put the Stygian dragon in the wrong place (room 580, an ice giantess lair) and
  cost a wrong first answer to the user. The dragon is lair id 84 at room 111.
- Exit rows hold ten targets in the order N, NE, E, SE, S, SW, W, NW, U, D, then a
  description index. The short description is `ROOMn`; the long one is `ROOM(n+1)`.
- "TRIG" rows are special locations (1 badge and treasure, 2 trap, 3 teleport, 4 typed
  command). "DOOR" rows are item-gated passages; the first number is the item id.
- Room 182's badge trigger is the white rune. The fourth lair field is the item the last
  monster in a lair carries, which is how the key chain works.
- Monster descriptions contain `%s %s` where TA prints a random weapon. Copying them
  unchanged would have put literal placeholders in the game.
- Negative exit values link the dungeon to town rooms; room 1's up exit is the arena.

## What we learned about the realm and the engine

Data conventions (from existing rows and `crates/daydream-types/src/records/`):

- An insert with only key columns leaves every other column 0 or empty, and that is a
  valid row.
- Monster `Group` is the room's `Mon Type`. Values 6-35 are "Group 1" to "Group 30".
  Groups 33, 34 and 35 were unused; 33 and 34 are now taken.
- A room spawns monsters whose `Index` falls between `Min Index` and `Max Index`.
  `Max Regen` is the most it generates, `Delay` the minutes between generations.
  Lair rooms are Type 3 with Min equal to Max. A boss can also be pinned with `Perm NPC`.
- **In an arena room (Type 2) `Max Index` is also the highest player level allowed in.**
  The check is in `silvermere-engine/src/movement/move_user.rs`. Setting 4-10 locked a
  level 70 tester out of the arena, which is the only way into the dungeon. The fix was
  Max Index 32767; the spawn pool did not change because the group only holds indices
  4-10.
- Exit types: 2 key (Para1 item, Para2 2 means locked), 3 item required (Para1 item),
  9 trap (Para1 damage), 15 level range. Direction slots are N, S, E, W, NE, NW, SE, SW,
  U, D.
- Item `Type` in the data: 0 armour, 1 weapon, 4 food, 5 drink, 6 light, 7 key,
  10 general. Armour AC is in tenths. `Cost Type` 2 is gold. A potion is a drink with
  ability 43 set to a spell number.
- Shops that sell a fixed list are Type 10. Temple (shop 4) and bank (shop 8) can be
  reused by pointing another room at them.
- Unused columns in stock rows hold junk (8224, -771747072). Existing rows are not a
  clean template.
- Some stock monsters carry placeholder message numbers (the cyclops has 1, 1, 1).

Stat bands used, taken from stock monsters:

| Level | Hit points | Experience | Damage per hit |
|---|---|---|---|
| 4 | 58 | 44 | 3-12 |
| 10 | 130 | 390 | 6-20 |
| 15 | 190 | 1,091 | 8-26 |
| 22 | 274 | 2,991 | 12-34 |
| Boss | 1,500 | 18,000 | 20-70 |

## Suggestions for the MCP server

Ordered by how much time each would have saved.

1. **Take edits from a file.** The 285 edits were generated by a script, then had to be
   read back and retyped by the model as tool arguments, about 135KB in and 135KB out.
   A `preview_edits` that accepts a path to a JSON file would remove nearly all of that
   cost and the risk of a retyping error.
2. **Make preview output proportional to the change.** A preview of an insert lists every
   column, about 10KB per room. Batches of 35 rooms returned 400KB and overflowed into a
   file, so the token had to be dug out with `jq`. Return only the columns that differ
   from the default, put the token first, and offer a summary mode (counts per table plus
   warnings).
3. **Validate references in preview.** Report exits to rooms that do not exist, one-way
   exits, item, monster, shop, spell and message numbers that do not exist, text longer
   than the column, and `%` placeholders in description text. All of these were checked
   by hand or not at all.
4. **Explain the columns.** `describe_table` gives names and types only. The meanings
   (enum values, units, "arena: Max Index is the player level cap", which message line is
   used for what) had to be found by reading two repositories. Put the enum tables and
   one line of meaning per column in the tool output.
5. **Treat messages as first-class.** Add a way to see a message bundle with its
   placeholders, to list which monsters and items use it, and to flag bundles with a
   name written into them. A "clone this monster's messages under a new name" helper
   would have prevented the main open defect.
6. **Add a clone operation.** "Insert a copy of row X with these overrides" is the
   natural way to make a new monster, item or shop, and it keeps related fields
   consistent.
7. **Filter on the server.** `list_rows` can only match the title. Finding lair rooms,
   rooms that hold an item, or monsters in a group meant downloading whole tables to
   files. Add column filters, and let `get_map` take a room range and include type and
   spawn fields.
8. **Report whether the game is running.** `silvermere_status` returned `running: null`.
   The server's own rule is that the game must be stopped, so it should know, and refuse
   to apply when it is running.
9. **Group undo.** Twelve tokens make up one feature. A label or parent id that undoes
   the set in order would make rollback safe.
10. **Small lookups.** Free group numbers, exact-name search across a table, and "what
    refers to this row" each replaced a script here.

## Suggestions for how the agent works

- **Check an index convention against a known name before building on it.** One lookup of
  the lair at the dragon's expected room would have caught the off-by-one.
- **Do not present a first reading as settled.** The first message told the user their
  memory of the dungeon was wrong; it was the analysis that was wrong.
- **Build one of each and test it in the game before bulk work.** One monster, one item,
  one arena room and a five-minute play test would have exposed the message bundles, the
  arena level cap and the attack rate before 47 monsters were written.
- **Read the engine source for any field whose meaning is inferred.** The arena cap was
  flagged as a risk in the plan but its effect, blocking the only entrance, was not
  thought through.
- **Inspect a template row in full before copying from it.** The cyclops placeholders
  were caught by luck while retyping; the death lines were not caught at all.
- **Keep generated inputs somewhere durable.** The generator script and batch files are
  in a session scratch directory and will be lost.
- **Report progress in the user's terms and less often.** The session was long; shorter
  status lines tied to visible milestones would have served better than tool narration.

## Reference

- Generator and batches (not durable):
  `/private/tmp/claude-502/-Users-john-fisher-repos-daydream/d00e69a5-7161-465a-9012-902eb4705a2e/scratchpad/`
  (`ta.py`, `gen.py`, `batch_00.json` to `batch_10.json`).
- Undo tokens, in apply order: `e1795550631-1`, `e1795551649-2`, `e1795551710-3`,
  `e1795551743-4`, `e1795551812-5`, `e1795551881-6`, `e1795551970-7`, `e1795552081-8`,
  `e1795552190-9`, `e1795552307-10`, then `e1795553013-0` (descriptions and arena cap).
  Undo in reverse order.
- Test rooms: 36 (walk NW), 3470 south plaza, 3468 arena, 3501 dungeon entrance,
  3552 level 2, 3608 level 3, 3611 dragon, 3682 rune chamber.
