//! The Personas, Prompts and Bench library windows - one single-instance
//! window per [`Library`], split out of the settings window so each can be
//! reached, and left open, on its own (#20).
//!
//! Contract: `openspec/specs/library-windows/spec.md`, "Personas window",
//! "Prompts window", "Bench window".
//!
//! `library` names the three kinds; `window` owns the entity and opens it,
//! raising an already-open instance through `window_registry` rather than
//! opening a second; `render` draws whichever pane the window is for, and
//! each pane's own content lives under `panes`. The three editor windows a
//! pane opens - persona, prompt, bench entry - live beside them, as does
//! `submit`, the dialog that sends a persona or prompt to Knot-Library.

mod bench_editor;
mod library;
mod panes;
mod persona_editor;
mod prompt_editor;
mod render;
mod submit;
mod window;

pub(crate) use library::Library;
#[cfg(test)]
pub(crate) use panes::personas::PersonaUser;
#[cfg(test)]
pub(crate) use prompt_editor::PromptEditor;
pub(crate) use prompt_editor::duplicate_and_edit;
pub(crate) use prompt_editor::open_prompt_editor;
pub(crate) use prompt_editor::open_prompt_editor_with_text;
#[cfg(test)]
pub(crate) use submit::LookupLocation;
pub(crate) use window::LibraryWindow;
pub(crate) use window::open_library_window;
