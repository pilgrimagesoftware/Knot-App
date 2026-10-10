//! Reads a Knot-Library-format location: a GitHub repository, a web
//! address, or a folder, each publishing an `index.json` and item files.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/data-import/spec.md`
//! (`Knot-Library`'s own `content-index` and `content-format` specs are the
//! upstream source of truth this crate implements as a client of).
//!
//! This crate knows nothing about Knot's `Settings` or records - it only
//! fetches, parses and verifies. `knot-core::import::library` maps verified
//! items onto `Persona` and `Prompt` records.

pub mod error;
pub mod fetcher;
pub mod index;
pub mod location;
pub mod resolve;
pub mod verify;

pub use error::{LibraryError, Result};
pub use fetcher::{FolderFetcher, HttpsFetcher, LibraryFetcher, fetcher_for};
pub use index::{Index, IndexItem, ItemKind};
pub use location::Location;
pub use resolve::resolve;
pub use verify::{split_item, verify};
