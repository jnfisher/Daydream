# Overnight summary, 2026-11-22 to 11-23

Work done autonomously overnight, until 8 AM Eastern on 2026-11-23. Where a decision came
up, the recommended option was taken; each is listed with its reason, so any can be
revisited. Pull requests merged once CI was green.

## Where things stand

- **M1 is done.** The gate (PR #5): real Nightmare Redux, driven on the Windows runner,
  imports about 8,600 cases Daydream wrote to a `.mdb`, and the realm it writes is byte for
  byte the realm Daydream's own import (in NMR mode) writes. The Windows job takes about
  4 minutes.
- **M2: every editor is in.** Items, monsters, spells, classes, races, shops, rooms,
  messages, text blocks and actions each follow their NMR form's tabs, labels and choice
  lists, and save through the same import an NMR `.mdb` goes through. Messages and text
  blocks show a preview in a terminal's ANSI colours.

## Pull requests

| PR | What | State |
|---|---|---|
| #5 | M1 gate: real NMR in CI | merged (before the night began) |
| #6 | M2: editor API and web UI, item editor as NMR's | merged (squash df539c4) |
| #7 | M2: every other editor laid out as NMR's, ANSI preview | merged (squash 163fa0c) |
| #8 | NMR's import options, Database Deleter and Merge Users | merged (squash 4c25fd8) |
| #9 | The importer, deleter and Merge Users in the browser (Tools) | merged (squash 3435600) |
| #10 | M3: the Universal Modifier, with a preview | merged (squash f4eb920) |
| #11 | M3: NMR's one-click fixes (limited items, exp, uses, kill times, drop %) | merged (squash c2536d0) |
| #12 | M3: NMR's lists, the Mass Room Editor, and the rest of its fixes | merged (squash 0f7b6dc) |
| #13 | M3: the Map and the Quest Organizer | open |

- **NMR's import options and tools are in** (library and command line): which tables, a
  range per table (rooms by map), Skip or Update, a room's items only or all but them, the
  deletion of text block parts the file no longer has, NMR's Database Deleter (all or a
  range) and its Merge Users. In the browser they are on a Tools page laid out as NMR's
  Database Importer (per table: import it or not, All or a From/To range, Map for rooms),
  with Preview First on each tool.

- **M3 has started with the Universal Modifier** (#10): one change applied to many records,
  as NMR's: a field (or a numbered family such as every `Item Drop %`), or give, take or
  change an ability, over a range, with up to three "only if" boxes. Preview lists every
  value that would change, before and after; Start writes them. It is the fourth card on
  the Tools page.
- **NMR's one-click fixes are in** (#11), each with a preview: No Limited Items, No Level
  Restrictions, Fix Number of Uses (items, and on monster drops), Multiply / Divide All Exp
  (split into Experience × Exp Multiplier as NMR splits it), Multiply Boss Exp, Combine Exp
  and Exp Multi, Set / Reset Last Killed Times, and Modify Item Drop Percentages with NMR's
  form and defaults. Records NMR would log for a look (an item with uses but no recharge
  ability, a drop of a missing item) are listed under the preview.
- **NMR's lists, the Mass Room Editor and the rest of the fixes** (#12). A Lists page has the
  Limited Items List (copies on floors, hidden, in shops and carried by users; over the limit
  in red), Find an Item, the Control Room List, the Monster Group / Index List and the
  NPC / Room List, each record linking to its editor. The Tools page gains the Mass Room
  Editor (NMR's groups of fields over a map's room range, values taken from a room) and the
  fixes Restock All Shops, Combine Like Items on Floor, Strip Characters off Text Blocks and
  Insert / Delete Buffer Rooms.
- **The Map and the Quest Organizer** (#13). A Map page draws a map's rooms as NMR's Map form
  does: a 41 × 41 grid with the start room in the middle, rooms placed by a walk of their
  exits, each exit's line coloured by its type, and marks for shops, permanent NPCs, command
  text and up/down exits. Clicking a room centres on it; each room links to its editor, and
  the room editor has a Map button. The Lists page gains NMR's Quest Organizer, which reads
  NMR's quest file (its tree of quests and the text blocks in each) and shows each text block.

## Decisions taken

1. **The UI is TypeScript, bundled with esbuild into a file the server embeds; the bundle is
   committed.** A Rust-only build (`cargo build`) needs no node, and CI checks the committed
   bundle matches its sources. No framework: the UI is small DOM code, which keeps the bundle
   about 40 KB and the build under a second.
2. **The editors work in NMR's terms: its tables, columns and labels.** The API serves each
   table as the rows of NMR's `.mdb` (the same columns the M1 export and import use), keyed by
   NMR's index columns. Every save is a one-row strict import, so an edit in the browser lands
   exactly as the same edit made in Access and imported would, and the M1 tests cover it.
3. **Saves use the default (strict) import, not NMR mode.** NMR mode reproduces NMR's quirks
   (space padding, new records copied from the previous one) for the CI gate; an editor should
   change only what the user changed. A save that changes nothing leaves the file's bytes as
   they were (tested).
4. **Every table gets an editor at once.** Tables without a hand-made layout get a generic
   one: a General tab, and a grid tab per family of numbered columns (abilities, exits, attacks).
   Hand-made layouts, starting with items, follow NMR's tabs and labels and replace them one by
   one.
5. **A save cannot change a record's key; Insert copies, as NMR's does.** The key fields are
   read only. Insert saves the current record, asks for the new key (the next free number
   offered), and makes the new record a copy of the current one, which is what NMR's Insert
   does (`frmItem.cmdInsert_Click`).

6. **The editors' choice lists are NMR's own.** They were read out of NMR's form resources
   (`.frx`) and its `Get*` label functions, so a value reads the same as in NMR. Values NMR has
   no name for show as "(unknown)" and are kept.
7. **ESC in text shows as ␛.** Text blocks and messages carry ANSI colour escapes; in the
   editors ESC shows as ␛, which can be typed or pasted, and is turned back into ESC on save.
   Row lists show text without escapes.
8. **PRs B and C went up together** (#7): both were UI layouts on the same code, so one CI run
   (the Windows job is the slow one) covered them.

9. **Text block parts past the file's last are deleted on a command-line import, not on a
   save in the browser.** NMR always deletes them; the browser saves one part at a time, so
   there it would wipe the other parts. `--keep-extra-parts` turns it off on the command line.
   NMR mode keeps NMR's quirk of leaving the last text block's parts alone.
10. **The importer's logging options are not copied.** NMR writes a log at three levels;
    Daydream's import returns a report per table (inserted, updated, unchanged, skipped,
    deleted, errors), which the command line prints.

11. **The modifier always previews before it writes, and writes through the strict import.**
    NMR's "Only Log Changes" becomes Preview, which lists each changed value (the first
    2,000 are shown; the count is whole). Start writes only the changed rows, so the
    records it does not change keep their bytes. The arithmetic follows NMR's `DoMath`
    exactly, including VB's rounding of halves to even.

12. **Multiply All Exp takes an Exp Multiplier of 0 as 1.** NMR's code multiplies by the 0
    and so leaves such a monster with 1 experience; its Divide and Combine already take 0 as
    1, so Multiply does the same here. Everything else follows NMR's code exactly, including
    VB's rounding and its search for a multiplier.
13. **The fixes follow NMR's code where its prompt text differs.** Fix Uses on Item Drops
    copies each item's uses (NMR's prompt says items with 0 or fewer uses get -1; its code
    copies the uses, and the item fix, run first, is what sets -1). Modify Item Drop
    Percentages keeps NMR's rule that a drop already at the value is left alone, even when
    adding.
14. **Multiply Boss Exp runs as a fix.** NMR fills in the Universal Modifier's form
    (Exp Multiplier × 2 where Game Limit = 1); here it is a fix with its own "times" box, and
    the modifier can do the same with other settings.

15. **The Mass Room Editor sets only the exits that are checked.** NMR's loop sets all ten
    exits whenever any one is checked, which its form does not suggest. Its Delete Range is
    left to the Database Deleter, which deletes the whole range (NMR's stops one room short).
16. **Insert Buffer Rooms does what NMR's prompt says** (a room for every missing number
    below a map's last room). NMR's loop walks every map's rooms from the start of the file,
    so on maps after the first it can stop at the first room that already exists.

17. **NMR's quest file is not copied into Daydream.** It is NMR's content, with no licence to
    copy it; the Quest Organizer reads `NMR-Quests.txt` or `NMR-QuestsCustom.txt` from the
    realm's folder, or a file opened in the browser, so NMR users keep their own.
18. **The Map shows one map at a time, and follows NMR's walk.** Map-change exits are drawn
    as stubs that open the other map; hidden exits are followed unless unticked, as NMR's
    option. NMR's map editing (frmMapEditor) and favourites are not in yet.

## Known gaps

- The read-only per-file API (`/api/files/...`) decodes records in the DOS layout only, so on
  a Worldgroup realm it errors (`1072 bytes, expected 1061`). The editors do not use it; it
  will learn the realm's layout.
- Found and fixed on the way: `daydream nmr-export` and `nmr-import` used the DOS layouts
  even on a Worldgroup realm. They now use the realm's own (#8).
- The Tools page (#9) imports with Daydream's strict rules only; NMR mode stays a
  command-line and CI option.

## Needs John

- Nothing blocking. Worth a look when convenient:
  - The item editor's layout against NMR's.
  - **No Limited Items keeps item 1749 ("yellow bone portal"), as NMR does.** In the 1.11p
    data there are two yellow bone portals, 1749 and 1807, and only 1807 is placed in the
    world, so this fix makes the placed one unlimited. Kept as NMR has it; adding 1807 is a
    one-line change.
