# Daydream's patches to datacraft-jet 0.2.0

The source of [datacraft-jet](https://github.com/RealRauch/datacraft) 0.2.0 (MIT OR
Apache-2.0), as published on crates.io, without its tests and examples, with two fixes to
Jet 4 text compression (`src/jet/text.rs`, `src/jet/encode.rs`):

- Reading: a value without the `FF FE` marker is plain UCS-2, and in a marked value every
  `0x00` byte toggles between single-byte and UCS-2 characters (0.2.0 read unmarked values
  as compressed and toggled once, garbling texts Jet stored with NULs or mixed characters).
- Writing: a text holding a NUL is not compressed, since the NUL would read as a toggle.

Drop this copy for an upstream release with the same fixes.
