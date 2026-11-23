//! Every integration test of daydream-wcc is a module of this one test binary, so the
//! crate's dependency graph is linked once rather than once per file. A new test file
//! goes in the list below.

mod support {
    pub mod stock;
}
use support::stock;

mod btrieve_fixtures;
mod decode_offsets;
mod roundtrip;
mod stock_vir;
