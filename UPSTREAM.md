# Code copied from other projects

Daydream reuses code from two MIT-licensed projects, and word lists from a third project. It is copied rather than shared as a
dependency, so neither project has to change for Daydream, and fixes are carried across by
hand. When you fix a bug in copied code, check whether the upstream copy has it too.

## silvermere 0.1.18 (https://github.com/jnfisher/silvermere, same author, MIT)

| Daydream | Upstream | Changes |
|---|---|---|
| `crates/daydream-types` | `crates/silvermere-types` | Renamed. Otherwise unchanged. |
| `crates/daydream-wcc/src/btrieve/` | `crates/silvermere-import/src/btrieve/` | Added `BtrieveFile::default_acs` (wbtrv32 records the file's ACS for every key) and `KeyDataType::code()`. |
| `crates/daydream-wcc/src/decode/` | `crates/silvermere-import/src/decode/` | `split_room`, `split_monster` and `split_shop` became `to_room`, `to_monster` and `to_shop`: they no longer build silvermere-db's separate state rows. `Text` keeps the member's stored bytes (old bytes after the NUL) so records round-trip byte for byte. |
| `crates/daydream-wcc/src/{dos_time,json,zip}.rs` | same names in `silvermere-import/src/` | None. |
| `crates/daydream-wcc/src/stock.rs` | `silvermere-import/src/stock.rs` | The URL override is `DAYDREAM_STOCK_URL`; messages name `daydream`. |
| `crates/daydream-wcc/tests/{btrieve_fixtures,decode_offsets,stock_vir}.rs`, `tests/fixtures/btrieve/` | `silvermere-import/tests/` | Adjusted for the renames and the `to_*` functions. |
| `crates/daydream-silvermere/migrations/{sqlite,postgres}/` | `crates/silvermere-db/migrations/` | None. `0001` and `0002` are 0.1.18's; `0003_moderation.sql` (bans, mutes, IP bans, a closed realm: no world tables) is silvermere 1.0.1's, taken from the 1.0.1 release binary, which embeds each migration after its SHA-384 checksum, and checked against that checksum. The adapter refuses a database whose migrations are not these (or a prefix of them). |
| `crates/daydream-silvermere/src/db/{dialect,guard,world_import}.rs`, `db/repo/` (rooms, items, monsters, spells, classes, races, shops, messages, text_blocks) | same names in `crates/silvermere-db/src/` | `silvermere_types` became `daydream_types` and `crate::` became `crate::db::`. Test modules left out. A new `DbError` stands in for silvermere-db's error type. |
| `crates/daydream-silvermere/src/db/pool.rs` | `crates/silvermere-db/src/pool.rs` | Trimmed to `Db`: no `TestDb`, no accessors for the player tables. |
| `web/src/library/script.ts` | `crates/silvermere-engine/src/scripts/{mod,actions,special}.rs` | Ported to TypeScript for the text block card: the clause parser (`skip_word`, `leading_int32`, `Clause::parse`, the keyword, teleport and skill tables) and how each runner cuts a block into lines. What a clause does is said in words, not run; `script.test.ts` carries the upstream parser tests. |
| `crates/daydream-nmr/src/script.rs` | the same, through `web/src/library/script.ts` | The parser and the line cutting again, in Rust, for the checks across the world (`wiring.rs`). Keep it in step with `script.ts`; its tests are `script.test.ts`'s. |
| `crates/daydream-nmr/src/messages.rs` | `crates/silvermere-engine/src/movement/exits.rs` (`fmt_msg`), and how `combat/{messages,kill_monster,mod}.rs`, `monsters/generate.rs`, `items/inventory.rs`, `cycles/medium.rs`, `spells/messages.rs` and `scripts/actions.rs` read a message | `fill` is `fmt_msg` ported. `reads` is a table, not code: for each column that names a message, which lines the game reads, when, to whom, and what fills their blanks in order (an arrival line takes the name first only for a monster with a name list). Keep it in step when the engine changes how it reads one. |
| `.github/scripts/zigbuild-prep.sh` | `.github/scripts/zigbuild-prep.sh` (1.0.1) | Its own cache folder (`daydream-tools`); the basement-runner notes left out. The zig and cargo-zigbuild pins are silvermere's. |

Not copied: the importer, the `WCCUPDAT.DAT` patch merge, asset unpacking and the checks,
which write silvermere's normalised database. Daydream keeps the raw records instead (see
`docs/approach.md`). `T-nnn` references in the copied comments are silvermere's task cards.

New in Daydream: `daydream-wcc/src/tables.rs` (file stem to record type),
`daydream-wcc/src/stock_files.rs` (the stock files unpacked into a cache), and the
silvermere adapter's own code (`daydream-silvermere/src/{lib,records,sync,raw}.rs`), which
moves a realm's world through the copied repositories.

When you move to a newer silvermere, copy its new migrations too, and check the copied
repositories and `records.rs` against any column it added; CI serves the adapter's export
with the release pinned by `SILVERMERE_VERSION` in `.github/workflows/ci.yml`.

## wbtrv32 (https://github.com/mbbsemu/wbtrv32, by Paladine, MIT)

Commit `4a439579952c51c59a61f0fa7d39f0c26681508f` (2026-10-23).

`crates/daydream-store/src/key.rs` and `db.rs` port wbtrv32's SQLite storage (schema version
4): `btrieve/Key.cc`, `KeyDefinition.h`, `AttributeMask.h`, `KeyDataType.h`, `SqliteUtil.cc`
and the schema, insert, update and conversion parts of `SqliteDatabase.cc` and
`BtrieveDriver.cc`. Only whole records are handled, so the truncated search-key paths are
left out.

`tools/wbtrv32/build-oracle.sh` builds wbtrv32's own converter at that commit, and the store's
differential tests (`crates/daydream-store/tests/oracle.rs`) check that Daydream writes the
same `.db` it does, byte for byte in every table. When you move to a newer wbtrv32, bump the
commit there and here, and port any change to the key columns.

## SCOWL (http://wordlist.aspell.net/), as the npm package wordlist-english 1.2.1 ships it

`crates/daydream-nmr/src/words/words.deflate`, the spelling check's dictionary, is SCOWL's
word lists of sizes 10 to 60 in every dialect (79,785 words, lower case), raw-deflated;
`tools/words/build.mjs` makes it, and copies SCOWL's notice to `words/COPYRIGHT.txt`. The
notice ships in the server's `/notices.txt` (`web/build.mjs` appends it). To change the
lists' size, change `LARGEST` there and run it again.

## Nightmare Redux 1.8.3 (https://github.com/jnfisher/Nightmare-Redux)

Not copied, but read by a generator: `crates/daydream-nmr/src/columns.rs` is generated by
`tools/nmr/generate.sh` from NMR's `CreateAccessTables`, the Access branch of its `Export*`
routines and its vO field maps, so the `.mdb` tables match NMR's column for column. Hand-
written from NMR's code: the "zero user interaction" export option, the `Info` row, the
textblock shift (`DecryptTextblock`, `EncryptTextblock`), `SLong2ULong`/`ULong2SLong`, and the
import's matching rules (`crates/daydream-nmr/src/import.rs` lists where Daydream is stricter).
`tools/nmr/write-mdb.ps1` creates the file through ADOX as `CreateAccessTables` does.

## datacraft-jet 0.2.0 (https://github.com/RealRauch/datacraft, MIT OR Apache-2.0)

A dependency, pinned exactly (it is young and expects breaking changes). Daydream uses only
its Jet 4 reader, to read `.mdb` files on any OS; it does not write index pages, so `.mdb`
files are written through Windows' Jet instead.

Its source is copied into `vendor/datacraft-jet` and patched in through `[patch.crates-io]`:
0.2.0 misreads Jet's compressed text (values with NULs, or that switch between one- and
two-byte characters), which NMR's files hold. `vendor/datacraft-jet/PATCHES.md` lists the
changes; drop the copy once an upstream release has the same fixes.
