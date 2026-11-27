# daydream

A modern, cross-platform, web-based editor for MajorMUD 1.11p game data, built to replace
Nightmare Redux (NMR).

**Status: milestone M2 (the editors) in progress.** Daydream converts a realm's Btrieve files
into its store, moves a realm to and from NMR's `.mdb` export format (real NMR imports what
Daydream exports, byte for byte, in CI), and edits every table in the browser. The item
editor is laid out as NMR's; the other tables use a generic layout until theirs land.
The design is in [docs/approach.md](docs/approach.md).

## Goals

- Edit realm data in SQLite on Windows, macOS and Linux, through a browser.
- Stay backward compatible with WCC v1.11p, with a guaranteed path back to the WCC Btrieve
  databases a realm runs on.
- Feel familiar to NMR users: same editors, field labels and tools.
- Keep the MMUD Explorer export limited to in-game records, so sysops' creative work stays
  protected.

## Try it

Release builds for Linux (x86_64 and aarch64), macOS (universal) and Windows are on the
[releases page](https://github.com/jnfisher/daydream/releases), each checked by its
`SHA256SUMS`; unpack one and run `daydream` in place of `cargo run -p daydream --` below. A
tag `vX.Y.Z` that matches the version in `Cargo.toml` builds them
(`.github/workflows/release.yml`).

```sh
# the stock 1.11p world (downloaded once from MBBSEmu and checked), or a realm's own files
cargo run -p daydream -- convert --stock --realm ./realm
cargo run -p daydream -- convert /path/to/realm/files --realm ./realm

cargo run -p daydream -- serve ./realm          # then open http://127.0.0.1:7340/
cargo run -p daydream -- dump ./realm WCCITEMS  # every item as a JSON line
```

### Choosing the game in the browser

`daydream serve` with no realm opens the game you had open last, or none. The **Worlds**
page (**Open another world…** under the world button at the top of the page) shows every
game Daydream found, which one is open and whether each runs, where it looked, and opens
another. Daydream assumes it runs on the machine that hosts the game, and looks for games where they are usually installed: the
drives' top folders on Windows (`C:\WGSERV`, `C:\MBBSEmu`, `C:\Silvermere`), your home
folder, the folder it was started in, `/opt` and `/srv`, a few levels down, plus any folder
given with `--search DIR`. On macOS it leaves out your Desktop, Documents, Downloads,
Pictures, Movies and Music, so macOS does not ask whether Daydream may read them; give
`--search ~/Documents/mud` (say) for a game kept there. It finds:

| Game | Found by | Edited in | Written back by |
|---|---|---|---|
| Worldgroup, The Major BBS (`WCC*.DAT`) | its world files | a working copy, made when first opened | **Publish**, Write to WCC files (no folder to type) |
| MBBSEmu, or wbtrv32 (`WCC*.db`) | its world files | a working copy, taken from its `.db` files while it runs | **Publish** |
| silvermere (SQLite or PostgreSQL, such as Neon) | `silvermere.toml`, `silvermere.db`, `SILVERMERE_DATABASE_URL`, `/etc/silvermere/env` | a working copy, read from its database | **Publish**, silvermere, Export |

A folder or database Daydream did not find can be typed on the Worlds page. Working copies are
kept in `DAYDREAM_HOME` (else `%LOCALAPPDATA%\daydream`, or `~/.local/share/daydream`),
one per game; **Read again** replaces one with the game as it is now, keeping the old copy
beside it as `.previous`. `daydream games` lists what the Worlds page would show.

**Stop the game to write into it, not to edit.** Edits go into the working copy while the
game runs; only writing back into its world files needs it stopped (publishing through the
update file, below, needs no stop at all), and Daydream refuses while it sees it
running: a Worldgroup (`WGSERVER`, or its service `WGSSRV` and console `WGSAPPGO`) or
MBBSEmu process whose program or folder is the game's, a WCC file another program holds
(open without sharing on Windows; open or locked on Linux), or silvermere's writer lock. An
MBBSEmu that Daydream cannot tie to the game's folder (one in a container, say) is a warning
before an MBBSEmu game publishes, since it may be running that game. The game changes some of its files as it runs (rooms' floors,
shop stock, players), so writing back writes only what the working copy changed, onto the
files as the game has them then: nothing the game did meanwhile is undone (see `write-dat`
below). An MBBSEmu game opened in its own `.db` files by an earlier Daydream moves to a
working copy at the next start.

**With an access token** (Daydream reached from elsewhere, as below), the Worlds page opens only
the games Daydream found itself, the ones opened before and the one it was started with,
and the write-back cards write only to the open game. Nobody holding the token can make it
read or write another folder or database. `serve --allow-any-source` lifts that; adding
`--search DIR` is the narrower way to offer another game.

### Publishing

**Publish** (the rainbow button) shows what goes out (the changes waiting), the checks, and
how it gets into the game, step by step, the ways the game allows:

| Game | Way | Steps |
|---|---|---|
| Worldgroup, The Major BBS | **At the next start**, with the game running | check, back up the update file, write it; MajorMUD merges it at its next start or nightly cleanup |
| | **Now, with the game stopped** | check, wait while you stop the game, back up each file about to change, write only the changes (and the update file), and you start it again |
| silvermere | **Now** | check, warn the players (a countdown you choose: now, 1, 5 or 15 minutes), shut down (silvermere saves every character), back up the database (SQLite), write the world, start it again and wait until it answers. Without an admin token: wait while you stop it, and you start it again |
| MBBSEmu (its own `.db` files) | **Now, with the game stopped** | check, wait while you stop MBBSEmu, back up each `.db` file about to change, write only the changes into them (each onto the game's record as play left it, at its own place, and the update file whole), and you start it again |

The update file carries edits and new records of the ten kinds it holds. It cannot delete or
renumber a record, carry players, gangs or bank books, or change what MajorMUD keeps as play
changes it (what lies on the floors, shop stock, kill timers, a gang's shop, an action marked
to keep): the page names each change it cannot carry, and those publish now instead.

Hold the button down until it fills, or press Enter. Publishing goes on in the background
wherever you go in Daydream, the top bar's button follows it, **Cancel** stops it while it
waits for the game to stop, and **Call it off** stops silvermere's countdown (silvermere tells
the players). Backups go in a `daydream-backup-<date>-<time>` folder beside the files. The
server answers the same at `GET /api/publish` (the plan), `POST /api/publish`
(`{"way": "update"}` or `"now"`, for silvermere with `"warn": {"in_secs": 300, "message":
"..."}`), `GET /api/publish/job` and `POST /api/publish/cancel`.

**silvermere's admin API** lets Daydream warn the players, stop silvermere and start it again.
Make a token where silvermere runs with `silvermere admin token new daydream --scopes
read,write,admin`, add the `[[admin.tokens]]` block it prints to `silvermere.toml`, start
silvermere again once (it reads its tokens when it starts), and paste the token on the
Publish page's **Server** card. Daydream keeps it in its workspace (`control/<game>.json`,
mode 600), never shows it again, and sends it only to the admin API `silvermere.toml` names
(`[server] admin_bind`). To start silvermere again it uses what started it: its systemd unit
(a system unit only when Daydream runs as root), its launchd agent, or the same command line
in the same folder when it was started by hand (its output goes to `control/<game>.log`).
Daydream runs only what it found running silvermere, never a command it is sent, and the
**Start it again** switch turns that off. The card also says something to everyone in the
game. Routes: `GET` and `PUT /api/control` (`{"token", "autostart", "admin"}`) and
`POST /api/control/say` (`{"text"}`).

### Finding your way

The rail down the left (along the bottom on a phone) has the six places: **Home** (the
world at a glance), **World** (the map), **Library** (every record, table by table),
**Story** (quests and the text they are made of), **Players** (users, gangs and bank
books) and **Workshop** (NMR's tools and lists). The top bar has the open world, one search
box for everything, **Changes** (every save waiting to publish, each one openable and
undoable) and **Publish**.

- **Every record has a link** that stays the same: `#/items/868`, `#/rooms/1/224`,
  `#/users/Sysop`, `#/world/1/224` for a room on the map. Links from before still work, and
  NMR's map with its room digger is still at `#/map-classic/1/224`.
- **Undo works for every save**, wherever it was made (an editor, a tool, an agent, a
  command): Ctrl+Z, or the toast after a save. Changes also takes back one record at a time.
- **A record opens beside the page** from search, a change or a reference, without leaving
  what you were doing; its **Open full page** goes to its editor.

| Keys | Do |
|---|---|
| Ctrl+K (⌘K), or `/` | Search records, places and commands: a name, a number (`868`, `1/224`), or a kind first (`item axe`, `r 12`) |
| Enter, Shift+Enter | In search: open beside the page, or the full page |
| `g` then `h` `w` `l` `s` `p` `k` | Go to Home, World, Library, Story, Players, Workshop; `g i` Items, `g m` Monsters |
| Ctrl+Z, Ctrl+Shift+Z or Ctrl+Y | Undo and redo the last save |
| Ctrl+S | Save the record |
| `[` and `]` | The previous and next record, as NMR's buttons step |
| Esc | Close the record beside the page |

**The World** draws a whole map on one canvas, laid out from room 1 by its exits (a room
whose cell is taken goes to the nearest free one), and the layout holds still as you look
around. Drag to pan, scroll or pinch to zoom, double-click to close in. A click on a room
opens its card beside the map, and the keys walk its exits from there. Rooms are coloured by
place (guessed from their names), area, monsters, light (in the game's words, from daylight
to pitch black) or your own edits; a click on a colour in the legend picks its rooms out. An exit that takes something to pass (a door, a
key, a toll, a trap) carries its symbol, a one-way exit an arrow, and an exit to a room drawn
elsewhere (a click goes there) or to a room the map does not have ends in a chip. A map change
ends in a portal with its map's number, which a click follows; the browser's Back comes back.
**Show** turns names, symbols and the grid on and off and says what the marks mean. It also
picks the map's style: the theme's own colours (Daydream by day, Starlight by night), either
of those whatever the theme, **Terminal** (the 16 ANSI colours on black), **Blueprint** (white
lines on blue paper, on a grid) or **Classic NMR** (Nightmare Redux's grey rooms on black, each
exit's line in its type's colour, and rooms with a way up green, down yellow, both cyan).

Dotted arcs draw the links no exit does, for the selected room (or every room picked): where
its command lines and spells teleport players (its room spell, a Cast exit's or a Spell
Trap's, followed through the text blocks and spells they run), the exits its remote actions
work (in a script, or a Remote Action exit's words), and its death, hang-up and control rooms;
the links into the room show too. Pointing at one says what sets it off and what does it ("Typing
'turn statue' in 9/1412 works the south exit of 9/1343"), and a click goes to the room at its
other end. A link onto another map ends in a chip with that map's number. A teleport rolled
from a range is drawn to each room of it, lighter, and a room scattering players over many
rooms has 60 of each kind drawn. **Show** can draw every room's links instead, fainter, leaving
out those that scatter players over many rooms, or none.

**Build** (or `B`) digs with the same keys, as NMR's map editor does. From the selected room,
a way with an exit is walked, a way into a room drawn next door joins the two (an exit each
way), and a way into a free cell digs a new room there: a copy of the room it is dug from,
without its exits, items, coins and permanent NPC, numbered after the map's highest room.
Digits first dig a line (`5` then an arrow: five rooms, as far as the cells stay free), `.`
does the last move again, and Enter names the room in place. Free cells and joins are marked
around the room for the mouse too: a click on one does what its key would, and a drag out of
it digs a corridor as long as the drag, walking the exits already that way and joining a room
it runs into (one-way when that room's way back is taken). Let go over a room off the line,
the drag joins that room instead, and Shift and a click digs a straight line from the room to
the cell clicked, along any of the eight ways. The hint bar says what it will make before it
is let go, and Esc lets go of it. Each edit shows at once and is written in turn, and its
toast (or Ctrl+Z) undoes it.

In Build mode a room can also be dragged to a free cell, or moved a cell at a time with
Shift and an arrow. The first move on a map keeps every room where it is drawn, so the map
holds still from then on, and rooms dug later join it by their exits. So does the first join
dragged across the map, whose exit is not the way its rooms are drawn from each other. MajorMUD keeps no
places for its rooms, so the layout is kept beside the working copy (`.daydream/map.json`),
never in the game's files: it is not published, and Ctrl+Z takes a move back like an edit.
**Show** exports every map's layout to a file another builder can import, and lays a map
out by its exits again.

**Show** can also draw a map in levels: Up and Down then lead a level up and down, to a room
drawn in the same place, so a tower's floors stack and each level is laid out around its
stairs instead of as islands below the map. One level shows at a time, with the level below
faint under it. Selecting a room on another level (walking a stair, a search, a link) shows
its level; the lift at the left goes a level up or down or picks one from the list, and `<`
and `>` do the same. An exit or a link to a room on another level ends in a chip with an
arrow. Each way of drawing a map keeps its own layout, so rooms moved by hand in levels stay
on their level, and drawing the map flat again brings back its flat layout.

**Notes** are the builder's own, kept beside the working copy with the layout and never
published. `N` writes a note on the room selected: words to remember, or a thing to do that is
ticked off when done. Shift+`N` writes a label, drawn over the room on the map, and a region (a
name drawn around rooms) goes around the rooms picked with Select. Each is written beside its
room, in one of six colours, and a note's mark sits on its room's corner: pointing at it reads
it, and a click opens it to change. The **Notes** menu lists the map's notes, the things still
to do first, ticks them off, and goes to a note's room. Ctrl+Z takes a change back like a move,
and Show's export and import carry the notes with the layouts (the server answers the same at
`POST /api/map/notes`).

**Stamps** (or `S`) put a shape of rooms down at once: a corridor, an L-bend, a T, a
crossroads, an open hall, a ring, a plaza, a spiral, a maze, a stairwell, a gatehouse, a lair
spur, a shop row, a forest patch, a cave or a one-way drop. `1` to `9` pick the first nine,
and the one held follows the pointer see-through, its way in under the pointer: `R` turns it,
`X` and `Y` mirror it, `[` `]` and `{` `}` size it, Space rolls another maze, forest or cave,
and the arrows and Enter aim and put it down from the keys (on a phone, a tap aims it and a
second tap puts it down). Where a cell lands on a room, that room is joined, keeping all it
has; an exit is only made where both rooms have that way free, so a link with an exit in its
way is left (marked `!`) rather than made one way. **Join the rooms it touches** (`J`) also
joins the rooms beside it, **Leave rooms under it alone** (Shift) keeps it off the rooms
already there, and **Replace the exits in its way** (Alt) replaces those exits. The drawer
says what the stamp would do where it is before it goes down. New rooms take the numbers after
the map's highest and copy the first room the stamp lands on or joins (else the room picked),
as a dig does, and stay where the stamp drew them. A stamp is one step: its toast or one
Ctrl+Z takes back its rooms, its exits and their places together (the server answers the
same at `POST /api/map/stamp`).

**Select** (or `V`) picks rooms to change together. **Box** (`1`) picks the rooms a drag
draws around, **Lasso** (`2`) the rooms a drawn line goes around, **Brush** (`3`) the rooms
painted over (`[` and `]` size it), and **Wand** (`4`) the rooms joined to the one clicked
through rooms of its colour: colour the map by area, monsters or light to pick a control
room's area, a monster region or a dark patch in one click. A click picks one room; Shift
adds to the rooms picked, Alt takes away, Shift or Ctrl and a click flips one, and **New**,
**Add** and **Take away** do the same for a finger. Space and a drag (or two fingers) moves
the map meanwhile, Ctrl+A picks every room of the map, and Esc lets go of them. The panel
shows what the rooms picked share, or **Mixed**, of their name, light, monsters (Mon Type,
Min Index, Max Index and Max Regen), control room and room type, and sets any of them on them
all as one step that one Ctrl+Z takes back: NMR's Mass Room Editor on just those rooms (the
server answers the same at `POST /api/map/set`, and refuses a number its field cannot hold).

**Paint** (`5`, in the Select panel) is the region brush: it puts light, monsters (Mon Type,
Min Index, Max Index and Max Regen) or a control room straight onto the rooms a drag goes
over, a step for each stroke that one Ctrl+Z takes back. The map is coloured by what is
painted (light, monsters or area), and the stroke shows the colours its rooms take before it
is let go. `[` and `]` size it, Alt and a click (or **Take from a room**, for a finger) takes a
room's value, and while rooms are picked, only they take paint, so a wand's area or a box
keeps the paint inside it. Rooms that hold the value already are left as they are.

**Save as a stamp** (in the Select panel, under **Their shape**) turns the rooms picked into a
stamp of one's own: their cells as the map draws them and the exits between them, named after
most of them. Saved stamps follow the drawer's own shapes under **Saved** (**Put it down**
opens them holding the new one), and the one held can be deleted. They are kept beside the
working copy in `.daydream/stamps.json`, as the map's places are, never in the game's data, so
saving or deleting one is taken back with Ctrl+Z like a room moved by hand. Its way in (the
cell under the pointer) is the first room with an exit out of the rooms picked. Put down on the
map it came from, its new rooms copy one of the rooms picked with the name most of them have
(the way in, when it has it) unless it lands on a room, and its exits are plain (the server
answers the same at `/api/map/stamps`).

**Walk** (or `P`) playtests from the selected room. The arrow keys (or the ways under the
panel's text, on a phone) walk a pretend player through the exits as the game's `move_user`
does, and the panel shows what the game sends back in its colours: each room's name,
description and obvious exits, and the game's own refusals for a shut or locked door, an item
not carried, a toll not covered, a hidden way not yet found, a class, race, level or
alignment kept out, a closed timed way, an arena the player is past, or an add-on's map the
world lacks. A note says what the game does not (coins paid, the damage a trap rolled), and a
refusal's button changes the player (opens the door, adds the coins, sets the level) and tries
again. Typing works as in the game: a direction or LOOK first, then the room's text exits and
commands. The map follows the player, onto other maps too; a click on a room puts the player
there, `H` puts them in the start room, and the player's level, class, race, coins and items
are under their name. Esc (or `P`) goes back to editing at the room reached, its card open.
Nothing is written: a walk changes only the pretend player.

**All maps** (`W`, the top of the maps list, or zooming out past a map) shows the whole world:
every map as a disc, sized by its rooms and named by the name most of its rooms share
(MajorMUD's maps have only numbers), and the map change exits between maps as lines, thicker
for more exits, with an arrow where they go one way only. Pointing at a disc says which maps
it joins, and at a line how many exits go each way and where the first is. A click on a disc
opens its map, and a click on a line opens the room its first exit is in, on the map nearer
the click; scrolling in over a disc opens it too. The arrow keys go from disc to disc and
Enter opens one; `W` or Esc go back into the map. The maps list names each map the same way,
and finds one by its number or its name.

| Keys on the map | Do |
|---|---|
| Arrows, or the number pad | Walk the room's exits; Home, PgUp, End and PgDn walk NW, NE, SW and SE |
| `U` `D` | Walk up or down |
| `<` `>` | In levels, show the next level up or down |
| `H` | The start room |
| `P` | Walk: playtest from the room, the arrows and `U` `D` walking its exits; a click puts the player in a room, Esc goes back to editing at the room reached |
| `W` | All maps: every map as a disc and the map changes between them; the arrows go from map to map, Enter opens one, `W` or Esc go back |
| `+` `-` | Zoom in or out (out past the whole map: all maps) |
| Shift+1, Shift+2 | Every room in view; close up on the room |
| `F` | Full screen (Esc comes back) |
| `B` | Build mode: the arrows and `U` `D` dig, join or walk; digits first for a line, `.` again, Enter to name the room, Esc when done |
| Shift+arrows | In Build mode, move the room a cell (or drag it) |
| Shift+click | In Build mode, dig a line from the room to the cell clicked |
| `S` | Stamps: `1` to `9` pick one; `R` turn, `X` `Y` mirror, `[` `]` `{` `}` size, Space roll, `J` join what it touches, arrows and Enter aim and put it down, Shift and Alt held leave rooms alone or replace exits, Esc puts it away |
| `V` | Select: `1` to `4` box, lasso, brush, wand; Shift adds, Alt takes away, `[` `]` size the brush, Space and a drag moves the map, Ctrl+A picks every room, Esc lets go of them, then puts it away |
| `5` | In Select, Paint: a drag paints the field onto rooms, `[` `]` size it, Alt and a click takes a room's value, Esc puts the pipette back, then lets go of the rooms picked |
| `N` | A note on the room (perhaps a thing to do): Ctrl+Enter keeps it, Esc puts it away unchanged |
| Shift+N | A label on the room, drawn on the map: Enter keeps it |

**The Library** has the world's records a table at a time: the list (found by name or
number, narrowed by chips such as Weapons or Map 3, or **Changed** for what changed since
publishing) and the open record's card. A card groups the fields by what they are for, with
NMR's labels: an item's fight, what players see (its text as the game shows it), its
abilities by name, who may use it, and where it is in the world (the shops that sell it, the
monsters that drop or wield it, the rooms it lies in, the scripts that name it). A number
that names a record shows it as a chip, which opens it beside the page, with a search to
pick another by name. A number is drawn among its peers' (every 1H Sharp weapon) to read in context, and a weapon's Max Hit
against its Speed among theirs, its own dot moving as you type and each of theirs opening it. A field changed since
publishing shows what it was, struck through, and the header counts those changes and takes
them back. **All fields** lays out every column as NMR's forms do, and **Copy** makes a new
record from the open one, as NMR's Insert does. **Grid** shows the whole table as a
spreadsheet, with a link of its own (`#/items?grid`): the rows the search and chips leave,
the columns you pick under the card's headings (remembered in this browser), sorted by a
header's click; **CSV** writes what it shows, numbers as the records hold them. On a phone
the card is the page and the list a sheet at its foot: its arrows step to the record before
or after, and a tap or a drag up raises it to find another. A monster's card plays its attacks as a fight
does: how often each is picked (the Attack % add up), how many blows its Energy buys, the
lines each outcome sends to the player hit and to the room, and **Roll another round** for a
round drawn as the game draws one. It also shows where the monster spawns (the rooms that
take its Group and Index, and the group's monsters by Index) and how it is wired: its weapon,
the lines a room reads when it arrives and dies, its names, and what it says. A text block's
card says what it does as the game reads it, following the text as it is typed: a script's
lines as steps in plain words ("Take 20 platinum, or say message 1458 and stop"), each record
it names a chip; a room's commands by the words that run them; a random block's lines by
their chances; a talk block's topics with the text each shows and the script that follows; and
a clause the game skips or a line it never reaches, with why. A room's card says what players
can type there, in the order the game tries it: the words of an exit that take them through it
or work a door, a hidden way or a trap (here or in another room), then the room's commands,
with a warning where an exit's words keep a command or another exit from ever answering; and
every way out, its "Obvious exits:" line as the game prints it and what each exit asks of a
player who walks it (a key or a pick of the lock, a toll, a level, a search), in plain words
from the game's own rules. A message's card shows how the game reads it for each record that
uses it, gathered by when it is read (a monster's hit lines, its death line, a script's words):
the lines filled in as the game fills them, each monster's own name where a blank takes it, so
an edit shows what every one of them will say, and while a line is changed, that saving changes
them all. A message field on any other card counts the other records that read the same
message; its badge lists them and gives the record a copy of its own, with the record's own
name written in where the lines name another monster, in one save that Undo takes back (the
server answers the same at `GET /api/messages/users` and `POST /api/messages/own`). A card also
says what is wrong with its record, each with a one-click fix that fills in the fields (so
Save keeps it and Undo takes it back): a value its field can't hold, checked as you type
(Save waits until it is fixed), a number that names a record the world does not have, an
empty slot still holding an old entry's values, and for a monster, an attack no blow picks
or a blank placeholder message players would read. **Compare** puts the record beside the
three most like it (a weapon among its weapon type and a monster among its group, by their
numbers; a room beside the rooms next to it), field by field under the card's headings, the
values that differ in yellow; a click copies one across, and any record of the table can be
added. Players keep today's editor for now.

**Story** lists the monsters that talk and the rooms players type in (an exit's words, the
room's commands), found by name or number. One opens as a map of its story, with a link of its
own (`#/story/monsters/381`, `#/story/rooms/6/1664`): the monster or room on the left, then a
column for each step on, as the game runs it: the talk block and the topics players can ask
about, a room's exits' words with where they take players and what they work, the text each
answer shows and the script it then runs, and the messages, items and rooms the steps name.
Each arrow leaves by the row that takes it (a topic, a step) and comes in at what it reaches,
its colour saying what it does: leads on, says, the way out when a test fails, or gives,
takes, needs or moves. Hovering a node lights up what it connects to, and a click opens it
beside the map. **In Story** on a text block's card, or on the card of a monster that talks or
a room players type in, opens its story. NMR's Quest Organizer is below the lists, as it was.

**Play it** tries a story as a pretend player: greet the monster and ask it about its topics,
or type in the room (its exits' words, its commands), and read what the game would send back,
in its colours, run as silvermere runs it (the same topic matching, the same rolls, the same
order of steps; an exit's words before the room's commands). Notes say what the game does not:
the coins paid, the items given, a door unlocked or a hidden way opened (as it stays for the
turns after), the roll and what it picked, and why a test failed, with a button that changes
the player so it passes and tries again. The
player's level, class, race, alignment, stats, purse and what they carry can be set; the map
lights up the way each turn went.

**Checks that read the wiring**, on Home, read across the whole world and name the records
each finding rests on. **Prices** puts what each script line charges (its `price` clauses
added up, as the game takes them) against the amounts its own words and its conversation
quote, the lines beside it that do the same for another price, and the Cost of the item it
gives: the stock smith charges 200 gold to make a hide tunic his words price at 2,000.
**Spelling** finds the words of players' text that the dictionary (SCOWL's word lists)
lacks, used once or twice, a letter from a word the world uses ("exhange" for "exchange");
**Not a typo** leaves one alone from then on. **Placeholders** finds blank messages where
the game would tell a blow in its own words (a monster's attack, a weapon's verbs) and names
left as placeholders. **Messages** reads each message as the game fills it in for each
monster that uses it: a line written for another creature ("Aiken utters a sharp cry, and
dies." on a monster that is not Aiken), an arrival line the wrong way round for how the game
fills it ("A the north lumbers in from troll.", since without a name list the game gives
where a monster came from first), and "from the the north". **Spawns** finds rooms whose
spawn range holds no monster of their group, and monsters no room spawns, with what brings
them instead: a room that keeps one as its NPC, a spell that summons it and what casts the
spell, a script. Warnings come first, and what was checked and holds is folded away. A
record's card lists the findings about it among its problems, with a fix where there is a
sure one (the game's own words for a blank message, the likely word for a typo, a copy of its
own for a line written for another creature). The checks
run again after each save.

The paint palette in the top bar (or searching "theme") picks the theme: Daydream,
Starlight, or whichever matches the system's light or dark. It also turns on the little
sounds, which are off until you do. Licenses: `/notices.txt`.

To and from Nightmare Redux's `.mdb` export (the tables NMR's Database Export writes and its
Database Import reads):

```sh
cargo run -p daydream -- nmr-export ./realm realm.mdb       # on Windows (uses its Jet engine)
cargo run -p daydream -- nmr-export ./realm realm.plan --plan
#   elsewhere: then on Windows, powershell -ExecutionPolicy Bypass -File tools\nmr\write-mdb.ps1 realm.plan realm.mdb
cargo run -p daydream -- nmr-import ./realm export.mdb --dry-run   # any OS
```

Like NMR's export, `nmr-export` zeroes the running game's state (room cash and floor items,
shop stock, monster kills) unless given `--keep-runtime`. Writing the `.mdb` needs Windows
because NMR's import seeks through the file's Jet indexes, which only Jet itself writes; NMR
runs only on Windows, so the machine that imports the file can always write it.

For MMUD Explorer, `mme-export` writes the `.mdb` MME reads, as NMR's Export to MMUD Explorer
does: the same tables and values (checked against NMR's own export of the stock world), the
same rooms left out, and nothing players could not learn from NMR's. It writes NMR's "Legit"
export unless given `--not-legit`, which allows `--custom`, `--update-url`, `--exclude
MAP:FROM-TO` and `--no-rooms`.

```sh
cargo run -p daydream -- mme-export ./realm data.mdb      # on Windows; or --plan as above
```

In the browser the same export is the Tools page's **Export to MMUD Explorer** card, laid out
as NMR's form (its config `.ini` files load and save there too). On Windows it downloads the
`.mdb`; elsewhere it downloads the plan and `write-mdb.ps1` to run on a Windows machine.

Back to the WCC files themselves, with no `WBTRV32.DLL` or NMR: `write-dat` writes the realm's
edits into the `WCC*.DAT` files it was converted from. Only the edits go in, onto the files
as they are now: each member an edit changed (a name, one of a room's item slots) is written
into the game's record of the same number, and every other member keeps the game's value, so
a night's play is not rolled back; added records are added, deleted ones deleted, and a
renumbered record takes the place of the one it was. An edit the game has made impossible
meanwhile (its record is gone, or the number it takes is now the game's) is reported, left
out, and kept for the next write. Files with no edits are left alone, edits that keep every
key are written in place, and files with records added, deleted or renumbered are rebuilt
whole (checked in CI against Pervasive's own Btrieve 6.15 engine, which reads them back,
deletes and inserts in them). Each changed file is copied to `WCC/daydream-backup-<time>`
first. Stop the game first.

A working copy edited before this version has no record of its edits, so its files are
written whole, as they are in the working copy; a WCC file the game changed since the
conversion is then refused rather than overwritten, unless `--force`. After one write,
only edits are written.

The working copy's edits since it was read or last written back are what `write-dat` writes:
`changes` lists them (each record added, changed or deleted, with the members it changed),
and takes some or all of them back; the server answers the same at `GET /api/changes` and
`POST /api/changes/revert`.

```sh
cargo run -p daydream -- changes ./realm
cargo run -p daydream -- changes ./realm --revert WCCITEMS:12   # or --revert-all
```

**Undo.** Every save is a step of the working copy's history, whoever made it: a save in an
editor, a Tools page run, an agent's batch of edits, a command. `history` lists the steps,
`undo` takes the newest back as a whole (or `--step N`), and `redo` writes the step undone
last again; the server answers the same at `GET /api/history`, `POST /api/undo` and
`POST /api/redo`. A step whose records changed since is not undone (undo the later step
first), and undo and redo bring the update file along as a save does. The last 100 steps
are kept, in `.daydream/journal.db`; a step of more than 10,000 records (a whole import) is
listed but too big to keep for undo.

```sh
cargo run -p daydream -- history ./realm
cargo run -p daydream -- undo ./realm             # or --step N; redo the same way
```

```sh
cargo run -p daydream -- write-dat ./realm /path/to/WGSERV --dry-run
cargo run -p daydream -- write-dat ./realm /path/to/WGSERV           # or --out DIR for a copy
```

In the browser it is the Tools page's **Write to WCC files** card.

**The update file.** MajorMUD merges its update file (`WCCUPDAT`, or `WCCUPDA2` on Worldgroup
3) into the world at every start, and MBBSEmu at its nightly cleanup too. The stock file holds
a copy of every stock record, so an edit written only into the world files is changed back
at the next start, and a deleted record comes back. Daydream therefore keeps the realm's
update file in step with every save (the editors, the Tools page, the MCP server and the
commands that write): an edited record's copy becomes the record as saved, a new record gets
one, and a deleted or renumbered record's old copy goes. `write-dat` writes the update file
with the rest, so the next start applies the edits again rather than undoing them. Edits
made before this version, or with other tools, are not in it: write the whole file again
from the realm with `update-file --full` (Nightmare Redux's "Compile Full Update File"), or
a blank one with `--blank`. A text block too long for a DOS update record (2010 bytes) loses
its copy instead, so the game leaves it as written. What Daydream changed is kept beside the
realm in `.daydream/journal.db`, never in the game's own files.

```sh
cargo run -p daydream -- update-file ./realm          # bring it in step (saves do this already)
cargo run -p daydream -- update-file ./realm --full   # or --blank
```

To and from a [silvermere](https://github.com/jnfisher/silvermere) server's database (its
SQLite file or PostgreSQL), straight into its tables while it is stopped:
`silvermere-export` writes the realm's world (rooms, items, monsters, spells, classes, races,
shops, messages, text blocks, actions) as `silvermere import` would, and `silvermere-import`
reads it back. Player tables are not touched. Daydream takes silvermere's own writer lock, so
both refuse while `silvermere serve` runs. The export also keeps each record's bytes in a
table of its own (`daydream_records`, which silvermere ignores), so an unchanged world comes
back byte for byte and an edit made in silvermere changes only the bytes it stores. Live
state silvermere already has (floors, shop stock, kills) is kept unless given
`--reset-state`.

```sh
cargo run -p daydream -- silvermere-export ./realm /srv/silvermere/data/silvermere.db --dry-run
cargo run -p daydream -- silvermere-export ./realm postgres://user@host/silvermere
cargo run -p daydream -- silvermere-import ./realm postgres://user@host/silvermere
```

`serve` and `mcp` take `--silvermere DATABASE` to offer the same in the browser (the Tools
page's **silvermere** card) and to agents (`silvermere_status`, `silvermere_export`,
`silvermere_import`, which preview unless told otherwise).

For AI agents, Daydream is an MCP (Model Context Protocol) server. An agent reads the realm
in NMR's terms (tables, columns, rows, free record numbers, maps) and makes content with
`preview_edits`, then `apply_edits`. Every write goes through the same strict import as the
editors' saves, and `undo` puts back exactly the records an applied edit changed (the journal
is kept in `REALM/.daydream/undo`). Give each batch of one piece of work the same
`changeset` label and `undo` takes the whole piece back. `game_status` says where edits go
and whether the game is running; writes to a running game's own files are refused. The
prompts `new_area`, `new_monster` and `new_item` walk an agent through building content one
tested piece at a time. The player tables are left out unless given `--players`.

Big pieces of work stay cheap. An agent can write its edits to a JSON file in the realm's
`.daydream/batches` folder (or a folder given with `--batch-dir`) and hand `preview_edits`
the file's name instead of retyping every edit; that only works for an agent on the same
machine, so a hosted one still sends its edits inline. A `clone` edit copies an existing row
with changes; given `"messages": "named"`, the copy gets messages of its own, the template's
messages that name it copied to new rows with the name swapped. Previews leave out the
columns an insert leaves blank, and a big preview comes back as a summary. `list_rows` takes
`where` conditions on any column (`"Room Item *"` is any of a room's items), and `get_map`
takes a range of rooms and extra columns; it answers with the links no exit draws too (the
teleports, spell teleports and remote actions the rooms' scripts and spells set off).

`describe_table` explains each column: its choice list, the table it refers to, and notes on
what the game does with it (an arena's Max Index is also its level cap), with the source in
silvermere's engine; for Rooms it gives what each exit type's parameters mean.
`find_references` lists every row that names a record (a script's lines included), and
`get_message` gives a message's three lines with what the game fills each blank with, and
every monster, item, spell and script that uses it, each line as it reads for that one.
`check_world` runs Home's checks that read the wiring, for the whole world or one record.
`preview_edits` warns of what a batch would leave pointing at nothing: a value that names a
record the realm does not have (the batch's own inserts and deletes counted), or rows still
naming a record it deletes. These come from `crates/daydream-nmr/src/fields.json`, which the
web editors' choice lists use too.

```sh
claude mcp add daydream -- daydream mcp /path/to/realm      # Claude Code, over stdio
```

For Claude Desktop, add `{"command": "daydream", "args": ["mcp", "/path/to/realm"]}` under
`mcpServers` in its config. `daydream serve` also answers MCP over HTTP at
`http://127.0.0.1:7340/mcp`. Over HTTP the agent also gets `list_games` and `open_game`, which
choose the game as the web UI's Worlds page does, so it can connect before a game is open.

### Reaching Daydream from elsewhere

Out of the box `daydream serve` answers this machine only. It refuses requests that come
through a tunnel or proxy, and refuses to listen anywhere but `127.0.0.1` or `::1`. A raw
port forward (`ssh -R`, `socat`) adds nothing to tell it apart from this machine, so never
forward Daydream's port without a token. To use
the web UI from another machine, or to connect a hosted agent (Claude.ai, ChatGPT, the
Claude or OpenAI APIs), give it an access token and put HTTPS in front of it with a tunnel
or reverse proxy, which brings the certificate (the tunnels hand you an `https` URL; Caddy
gets one from Let's Encrypt for your domain). Daydream does not do TLS itself. Never serve
the token over plain HTTP: it is a password for the realm.

```sh
export DAYDREAM_TOKEN=$(daydream new-token)     # keep it: agents and the browser need it
daydream serve ./realm                          # still on 127.0.0.1:7340, behind one of:

cloudflared tunnel --url http://127.0.0.1:7340  # Cloudflare Tunnel: prints an https URL
ngrok http 7340                                 # ngrok
tailscale funnel 7340                           # Tailscale Funnel
caddy reverse-proxy --from daydream.example.com --to 127.0.0.1:7340   # your own domain
```

With a token:

- **The browser** opens the public URL and signs in with the token once (`/login`). The
  sign-in is a cookie, so other sites cannot use it.
- **Agents that can send a header** (Claude Code, the Claude and OpenAI APIs' MCP
  connectors, Claude.ai's connectors where request headers are offered) use
  `https://<host>/mcp` with `Authorization: Bearer <token>`.
- **Connectors that take only a URL** (ChatGPT's custom connectors, Claude.ai's with no
  sign-in) use `https://<host>/mcp/<token>` and "No authentication". The URL is the
  secret: treat it like a password.

```sh
claude mcp add --transport http daydream https://<host>/mcp --header "Authorization: Bearer $DAYDREAM_TOKEN"
```

A token changes when you start Daydream with another one: every connector and browser then
needs the new one.

`convert` only reads the source files. It writes one `.db` per Btrieve file, in the format of
wbtrv32 (the `WBTRV32.DLL` replacement MBBSEmu uses), so MBBSEmu and legacy NMR can open the
same files.

## Layout

| Path | What it is |
|---|---|
| `bin/daydream` | The command line: `convert`, `serve`, `dump`, `nmr-export`, `nmr-import` (with NMR's importer options), `mme-export`, `delete`, `merge-users`, `write-dat`, `changes`, `history`, `undo`, `redo`, `update-file`, `silvermere-export`, `silvermere-import`, `silvermere-info`, `mcp`, `new-token` |
| `crates/daydream-mcp` | The MCP server for AI agents: its tools, the undo journal, stdio |
| `crates/daydream-nmr` | NMR's `.mdb` format: export a realm to it, import it, read and write the file; one row at a time for the editors (`rows`) |
| `crates/daydream-silvermere` | silvermere's database: its schema and repositories (copied), the writer lock, and moving a realm's world in and out |
| `crates/daydream-server` | The HTTP server: the editors' API, and the web UI embedded |
| `web` | The web UI: the frame (Svelte 5 and Bits UI, `web/src/shell`) around today's pages (TypeScript); `npm run build` bundles it, its two fonts and the licenses into the server |
| `crates/daydream-store` | The realm store: wbtrv32 `.db` files, read and written with wbtrv32's exact key columns; the journal of what Daydream changed, the history of saves for undo, the update file kept in step with it, and writing only the edits back onto the game's files |
| `crates/daydream-wcc` | The Btrieve reader and the 1.11p record decoders and encoders |
| `crates/daydream-types` | Record types, enums, ids and byte layouts |
| `tools/nmr` | Generates the `.mdb` column table from NMR's source; `write-mdb.ps1` writes the file through Jet |
| `tools/wbtrv32` | Builds wbtrv32's own converter, the oracle for the store's tests |
| `tools/words` | Builds the spelling check's dictionary from SCOWL's word lists |

Some of this code comes from silvermere and wbtrv32, and the dictionary from SCOWL;
[UPSTREAM.md](UPSTREAM.md) says what and from which version.

## Tests

```sh
cargo test --workspace
```

The tests download the stock 1.11p package once (6.7 MB, checked against a pinned sha256) into
`~/.cache/daydream/stock`; no game data is committed. The store's differential tests compare
Daydream's `.db` files with wbtrv32's own converter, which they skip locally unless it is built:

```sh
export $(tools/wbtrv32/build-oracle.sh | xargs)   # needs git and a C++20 compiler
cargo test --workspace
```

CI runs everything on each pull request that touches code, with the dependency builds, the
stock package and the wbtrv32 build cached. A Windows job writes real `.mdb` files through
Jet, reads them back and imports them; it runs when the code it covers changes, and nightly.

The silvermere adapter's tests run on SQLite, and on PostgreSQL too when
`DAYDREAM_POSTGRES_URL` names a database they may create schemas in. With `SILVERMERE_BIN`
set to a silvermere binary, they also serve the export with it. CI does both, with
silvermere's release binary cached.

The web UI's bundle (`crates/daydream-server/src`: `app.js`, `app.css`, `fonts/` and
`notices.txt`) is committed, so building the server needs no node. After changing
`web/src`, rebuild it, and keep it within its size budget (175 KB of script and 25 KB of
styles, gzipped; CI checks both):

```sh
cd web && npm ci && npm run check && npm run build && npm run budget
```

## License

MIT. See [LICENSE](LICENSE).
