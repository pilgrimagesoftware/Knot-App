//! Reads a coding agent's past sessions for a given project folder:
//! per-agent providers (claude, codex, gemini, copilot), a uniform
//! `SessionSummary` shape, and a folder-scoped cache with explicit
//! refresh / invalidate and delete-then-backfill.
//!
//! Contract: `openspec/specs/conversation-history/spec.md`.
//!
//! Runtime-agnostic: no async runtime; wrap calls in `spawn_blocking` at an
//! async boundary. Reading degrades to an empty result rather than surfacing
//! an I/O error; deleting returns its error, so a delete that did not happen
//! is never reported as done.

pub mod cache;
pub mod consts;
pub mod error;
mod files;
pub mod paths;
pub mod provider;
pub mod providers;
pub mod title;

pub use cache::HistoryCache;
pub use error::{HistoryError, Result};
pub use provider::{HistoryProvider, SessionSummary, provider, supports_history};
