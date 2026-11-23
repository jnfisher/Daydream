# Code copied from other projects

Daydream reuses code from two MIT-licensed projects. It is copied rather than shared as a
dependency, so neither project has to change for Daydream, and fixes are carried across by
hand. When you fix a bug in copied code, check whether the upstream copy has it too.

## silvermere 0.1.18 (https://github.com/jnfisher/silvermere, same author, MIT)

| Daydream | Upstream | Changes |
|---|---|---|
| `crates/daydream-types` | `crates/silvermere-types` | Renamed. Otherwise unchanged. |
| `crates/daydream-wcc/src/btrieve/` | `crates/silvermere-import/src/btrieve/` | Added `BtrieveFile::default_acs` (wbtrv32 records the file's ACS for every key) and `KeyDataType::code()`. |
| `crates/daydream-wcc/src/decode/` | `crates/silvermere-import/src/decode/` | `split_room`, `split_monster` and `split_shop` became `to_room`, `to_monster` and `to_shop`: they no longer build silvermere-db's separate state rows. |
| `crates/daydream-wcc/src/{dos_time,json,zip}.rs` | same names in `silvermere-import/src/` | None. |
| `crates/daydream-wcc/src/stock.rs` | `silvermere-import/src/stock.rs` | The URL override is `DAYDREAM_STOCK_URL`; messages name `daydream`. |
| `crates/daydream-wcc/tests/{btrieve_fixtures,decode_offsets,stock_vir}.rs`, `tests/fixtures/btrieve/` | `silvermere-import/tests/` | Adjusted for the renames and the `to_*` functions. |

Not copied: the importer, the `WCCUPDAT.DAT` patch merge, asset unpacking and the checks,
which write silvermere's normalised database. Daydream keeps the raw records instead (see
`docs/approach.md`). `T-nnn` references in the copied comments are silvermere's task cards.

New in Daydream: `daydream-wcc/src/tables.rs` (file stem to record type) and
`daydream-wcc/src/stock_files.rs` (the stock files unpacked into a cache).

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
