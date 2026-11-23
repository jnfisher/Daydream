# Overnight summary, 2026-11-22 to 11-23

Work done autonomously overnight, until 8 AM Eastern on 2026-11-23. Where a decision came
up, the recommended option was taken; each is listed with its reason, so any can be
revisited. Pull requests merged once CI was green.

## Where things stand

- **M1 is done.** The gate (PR #5): real Nightmare Redux, driven on the Windows runner,
  imports about 8,600 cases Daydream wrote to a `.mdb`, and the realm it writes is byte for
  byte the realm Daydream's own import (in NMR mode) writes. The Windows job takes about
  4 minutes.
- **M2 has started.** The browser editors work on every table, saving through the same
  import an NMR `.mdb` goes through.

## Pull requests

| PR | What | State |
|---|---|---|
| #5 | M1 gate: real NMR in CI | merged (before the night began) |
| M2 A | Editor API and web UI, item editor as NMR's | this PR |

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

## Known gaps

- The read-only per-file API (`/api/files/...`) decodes records in the DOS layout only, so on
  a Worldgroup realm it errors (`1072 bytes, expected 1061`). The editors do not use it; it
  will learn the realm's layout.

## Needs John

- Nothing blocking. Worth a look when convenient: the item editor's layout against NMR's.
