//! Rebuilds this crate when a translation changes.
//!
//! `rust_i18n::i18n!("locales")` in `lib.rs` reads `locales/*.yml` while the
//! macro expands and compiles the catalog into this crate, but it tells
//! cargo nothing about those files. Without this, an edit to `en.yml` alone
//! leaves an incremental build's catalog stale, and a new key renders as the
//! key itself until something else makes this crate recompile. Clean builds,
//! CI's included, were never affected.
//!
//! rust-i18n ships a `build.rs` of its own that emits `rerun-if-changed`
//! for `**/locales/**`, but it runs in rust-i18n's own build with
//! rust-i18n's own manifest directory, so it never sees these files - and
//! re-running a dependency's build script would not recompile this crate
//! anyway. The rerun has to be declared here.
//!
//! A directory, not a file list: cargo scans every file under it, so a new
//! locale file is tracked without editing this.

fn main() {
    println!("cargo:rerun-if-changed=locales");
}
