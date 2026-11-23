# NMR's `.mdb` format

`crates/daydream-nmr/src/columns.rs` is generated from Nightmare Redux's own VB6 source, so
Daydream's `.mdb` tables are NMR's, column for column:

```sh
tools/nmr/generate.sh path/to/Nightmare-Redux
```

It reads three things from NMR, then joins them:

| Script | Reads | Gives |
|---|---|---|
| `fieldmap.py` | `modFieldmaps_vO.bas`: the record types and the `Add*FieldMap` lists (the build for 1.11o and 1.11p) | Each VB member's byte offset, length and VB type in the record |
| `exports.py` | `frmDatabaseExport.frm`: the Access branch of every `Export*` routine | Which record member fills each column |
| `gen.py` | `CreateAccessTables` (`modMMUDFunctions.bas`) and `daydream-types`' `WG_*` layouts | Each column's type, and the layout members its bytes are in |

`gen_rust.py` writes the result as Rust. NMR edits the Worldgroup 3 build's files, whose
records are 4-byte aligned, so its offsets are matched against the `WG_*` layouts; naming the
member (not the offset) is what lets the same column be read from a DOS realm. `gen.py`
stops if a column has no export source or its bytes fall outside the layout.

Hand-written, in `crates/daydream-nmr/src/export.rs`: NMR's "zero user interaction" export
option (on by default in NMR) and the `Info` row.

`write-mdb.ps1` creates the `.mdb` on Windows the way `CreateAccessTables` does (ADOX, the
same column types, keys and indexes) and inserts the rows through OLE DB. It reads the plan
`daydream nmr-export --plan` writes, so the export can be made on any OS.

`gen_players.py` writes `crates/daydream-nmr/src/players.rs`: Users, Bank Books and Gangs,
which NMR edits in its User, Bank and Gang forms but never exports. They take the same
`NmrTable` shape so the editors, imports and deletes work on them, and stay out of
`columns::TABLES`, so the `.mdb` export is unchanged.
