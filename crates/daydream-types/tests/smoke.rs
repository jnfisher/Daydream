//! Placeholder so `cargo test -p daydream-types` has a target. Later cards add their
//! tests next to this file.

use daydream_types as _;

#[test]
fn crate_builds_and_links() {
    assert_eq!(env!("CARGO_PKG_NAME"), "daydream-types");
}
