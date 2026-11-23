# Daydream

A modern, cross-platform, web-based editor for MajorMUD 1.11p game data, built to replace
Nightmare Redux (NMR).

**Status: milestone M1 (foundation) in progress.** Daydream converts a realm's Btrieve files
into its store and serves the records, decoded, to the browser. The editors arrive in M2.
The design is in [docs/approach.md](docs/approach.md).

## Goals

- Edit realm data in SQLite on Windows, macOS and Linux, through a browser.
- Stay backward compatible with WCC v1.11p, with a guaranteed path back to the WCC Btrieve
  databases a realm runs on.
- Feel familiar to NMR users: same editors, field labels and tools.
- Keep the MMUD Explorer export limited to in-game records, so sysops' creative work stays
  protected.

## Try it

```sh
# the stock 1.11p world (downloaded once from MBBSEmu and checked), or a realm's own files
cargo run -p daydream -- convert --stock --realm ./realm
cargo run -p daydream -- convert /path/to/realm/files --realm ./realm

cargo run -p daydream -- serve ./realm          # then open http://127.0.0.1:7340/
cargo run -p daydream -- dump ./realm WCCITEMS  # every item as a JSON line
```

`convert` only reads the source files. It writes one `.db` per Btrieve file, in the format of
wbtrv32 (the `WBTRV32.DLL` replacement MBBSEmu uses), so MBBSEmu and legacy NMR can open the
same files.

## Layout

| Path | What it is |
|---|---|
| `bin/daydream` | The command line: `convert`, `serve`, `dump` |
| `crates/daydream-server` | The HTTP API the web UI uses |
| `crates/daydream-store` | The realm store: wbtrv32 `.db` files, read and written with wbtrv32's exact key columns |
| `crates/daydream-wcc` | The Btrieve reader and the 1.11p record decoders and encoders |
| `crates/daydream-types` | Record types, enums, ids and byte layouts |
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
stock package and the wbtrv32 build cached.

## License

MIT. See [LICENSE](LICENSE).
