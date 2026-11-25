//! Catalog coverage for the strings that would otherwise ship as their own
//! keys - a missing entry renders the key, which is visible but easy to
//! miss in review.
//!
//! Split by area of the catalogue; each module checks the keys one surface
//! draws.

mod chrome;
mod menus_dialogs;
mod panel;
mod processes_changes;
mod queued_prompts;
mod settings;
