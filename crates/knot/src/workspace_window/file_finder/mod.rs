//! The file finder (#5, `openspec/specs/file-finder` once archived;
//! `openspec/changes/file-finder` until then): Edit > Find File… (⌘⇧F) over
//! the selected agent's folder, handing the chosen path to that agent.
//!
//! `state` owns opening, searching, moving and choosing; `render` draws the
//! overlay. The listing is the agent's `@` mention listing, and the ranking
//! is `crate::file_search`, so neither the walk nor the scorer runs twice
//! and neither runs in render.

mod render;
mod state;

pub(in crate::workspace_window) use state::FileFinder;

#[cfg(test)]
mod tests;
