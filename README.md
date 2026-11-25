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

`daydream serve` with no realm opens the game you had open last, or none, and the game
button at the top of the page (the game picker) opens another. Daydream assumes it runs on
the machine that hosts the game, and looks for games where they are usually installed: the
drives' top folders on Windows (`C:\WGSERV`, `C:\MBBSEmu`, `C:\Silvermere`), your home
folder, the folder it was started in, `/opt` and `/srv`, a few levels down, plus any folder
given with `--search DIR`. On macOS it leaves out your Desktop, Documents, Downloads,
Pictures, Movies and Music, so macOS does not ask whether Daydream may read them; give
`--search ~/Documents/mud` (say) for a game kept there. It finds:

| Game | Found by | Edited in | Written back by |
|---|---|---|---|
| Worldgroup, The Major BBS (`WCC*.DAT`) | its world files | a working copy, made when first opened | Tools, **Write to WCC files** (no folder to type) |
| MBBSEmu, or wbtrv32 (`WCC*.db`) | its world files | the game's own files | nothing to do: every save is in the game |
| silvermere (SQLite or PostgreSQL, such as Neon) | `silvermere.toml`, `silvermere.db`, `SILVERMERE_DATABASE_URL`, `/etc/silvermere/env` | a working copy, read from its database | Tools, **silvermere**, Export |

A folder or database Daydream did not find can be typed in the picker. Working copies are
kept in `DAYDREAM_HOME` (else `%LOCALAPPDATA%\daydream`, or `~/.local/share/daydream`),
one per game; **Read again** replaces one with the game as it is now, keeping the old copy
beside it as `.previous`. `daydream games` lists what the picker would show.

**Stop the game to write into it, not to edit.** Edits go into the working copy while the
game runs; only writing back needs it stopped, and Daydream refuses while it sees it
running: a Worldgroup or MBBSEmu process whose program or folder is the game's (on
Windows, also a WCC file open in another program), or silvermere's writer lock. A game
edited in its own `.db` files (MBBSEmu) refuses every save while it runs. The game changes
some of its files as it runs (rooms' floors, shop stock, players), and `write-dat` refuses
to write over a file that changed since its working copy was read: read the game again
just before editing, or write with "even over files the game changed".

**With an access token** (Daydream reached from elsewhere, as below), the picker opens only
the games Daydream found itself, the ones opened before and the one it was started with,
and the write-back cards write only to the open game. Nobody holding the token can make it
read or write another folder or database. `serve --allow-any-source` lifts that; adding
`--search DIR` is the narrower way to offer another game.

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

Back to the WCC files themselves, with no `WBTRV32.DLL` or NMR: `write-dat` writes the realm
into the `WCC*.DAT` files it was converted from. Files nobody changed are left alone, edits
that keep every key are written in place, and files with records added, deleted or renumbered
are rebuilt whole (checked in CI against Pervasive's own Btrieve 6.15 engine, which reads them
back, deletes and inserts in them). Each changed file is copied to
`WCC/daydream-backup-<time>` first, and a WCC file the game changed since the conversion is
refused rather than overwritten. Stop the game first.

```sh
cargo run -p daydream -- write-dat ./realm /path/to/WGSERV --dry-run
cargo run -p daydream -- write-dat ./realm /path/to/WGSERV           # or --out DIR for a copy
```

In the browser it is the Tools page's **Write to WCC files** card.

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
with changes. Previews leave out the columns an insert leaves blank, and a big preview comes
back as a summary. `list_rows` takes `where` conditions on any column (`"Room Item *"` is any
of a room's items), and `get_map` takes a range of rooms and extra columns.

`describe_table` explains each column: its choice list, the table it refers to, and notes on
what the game does with it (an arena's Max Index is also its level cap), with the source in
silvermere's engine; for Rooms it gives what each exit type's parameters mean.
`find_references` lists every row that names a record. These come from
`crates/daydream-nmr/src/fields.json`, which the web editors' choice lists use too.

```sh
claude mcp add daydream -- daydream mcp /path/to/realm      # Claude Code, over stdio
```

For Claude Desktop, add `{"command": "daydream", "args": ["mcp", "/path/to/realm"]}` under
`mcpServers` in its config. `daydream serve` also answers MCP over HTTP at
`http://127.0.0.1:7340/mcp`. Over HTTP the agent also gets `list_games` and `open_game`, which
choose the game as the web UI's picker does, so it can connect before a game is open.

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
| `bin/daydream` | The command line: `convert`, `serve`, `dump`, `nmr-export`, `nmr-import` (with NMR's importer options), `mme-export`, `delete`, `merge-users`, `write-dat`, `silvermere-export`, `silvermere-import`, `silvermere-info`, `mcp`, `new-token` |
| `crates/daydream-mcp` | The MCP server for AI agents: its tools, the undo journal, stdio |
| `crates/daydream-nmr` | NMR's `.mdb` format: export a realm to it, import it, read and write the file; one row at a time for the editors (`rows`) |
| `crates/daydream-silvermere` | silvermere's database: its schema and repositories (copied), the writer lock, and moving a realm's world in and out |
| `crates/daydream-server` | The HTTP server: the editors' API, and the web UI embedded |
| `web` | The web UI's TypeScript sources; `npm run build` bundles them into the server |
| `crates/daydream-store` | The realm store: wbtrv32 `.db` files, read and written with wbtrv32's exact key columns |
| `crates/daydream-wcc` | The Btrieve reader and the 1.11p record decoders and encoders |
| `crates/daydream-types` | Record types, enums, ids and byte layouts |
| `tools/nmr` | Generates the `.mdb` column table from NMR's source; `write-mdb.ps1` writes the file through Jet |
| `tools/wbtrv32` | Builds wbtrv32's own converter, the oracle for the store's tests |

Some of this code comes from silvermere and wbtrv32; [UPSTREAM.md](UPSTREAM.md) says what and
from which version.

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

The web UI's bundle (`crates/daydream-server/src/app.js`) is committed, so building the
server needs no node. After changing `web/src`, rebuild it:

```sh
cd web && npm ci && npm run check && npm run build
```

## License

MIT. See [LICENSE](LICENSE).
