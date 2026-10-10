//! State for the Library section: which location is chosen, what its index
//! last said, and the off-thread work that reads and imports from it.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/import-ui/spec.
//! md`, over `.../data-import/spec.md`.
//!
//! Fetching never runs from `render` - it is triggered by [`super::window`]
//! on open, on choosing another location, and on refresh, and always goes
//! through `background_executor`. The result reaches the screen through an
//! explicit `this.update_in` + `cx.notify()`, the same shape `bug_report`'s
//! submit uses, rather than a poll loop: this window does one fetch at a
//! time in response to a user action, not a continuously-running state.

use std::collections::BTreeSet;

use gpui_kit::Context;
use gpui_kit::Window;
use knot_core::Settings;
use knot_core::consts::{KNOT_LIBRARY_BRANCH, KNOT_LIBRARY_REPO};
use knot_library::{Index, IndexItem, LibraryError, Location};
use uuid::Uuid;

use super::outcome::summarise;
use super::window::ImportWindow;

/// Which saved location the picker points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocationRef {
    /// The constant, always-available Knot-Library.
    BuiltIn,
    Saved(Uuid),
}

/// What the chosen location's index last said.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum LibraryIndex {
    Loading,
    Loaded(Index),
    /// The location could not be reached at all.
    Unreachable,
    /// The location answered with an index format this Knot does not
    /// understand.
    Unsupported,
}

pub(super) struct LibraryState {
    pub(super) selected:          LocationRef,
    pub(super) index:             LibraryIndex,
    pub(super) persona_selection: BTreeSet<String>,
    pub(super) prompt_selection:  BTreeSet<String>,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self { selected:          LocationRef::BuiltIn,
               index:             LibraryIndex::Loading,
               persona_selection: BTreeSet::new(),
               prompt_selection:  BTreeSet::new(), }
    }
}

/// Knot-Library's own name, shown in the picker and in error messages. Not
/// through the catalog - it is this location's proper name, the same for
/// every user, like a repository name would be.
pub(super) const BUILTIN_LIBRARY_NAME: &str = "Knot-Library";

/// Resolves `selected` to its name and [`Location`], or `None` when a saved
/// location was picked and then removed out from under the window.
pub(super) fn resolve_location(settings: &Settings, selected: LocationRef)
                               -> Option<(String, Location)> {
    match selected {
        LocationRef::BuiltIn => Some((BUILTIN_LIBRARY_NAME.to_string(),
                                      Location::GitHub { repo:   KNOT_LIBRARY_REPO.to_string(),
                                                         branch:
                                                             Some(KNOT_LIBRARY_BRANCH.to_string()), })),
        LocationRef::Saved(id) => {
            settings.library_locations
                    .iter()
                    .find(|saved| saved.id == id)
                    .map(|saved| (saved.name.clone(), saved.location.clone()))
        }
    }
}

impl ImportWindow {
    /// Reads the chosen location's index off-thread. Called when the window
    /// opens, when another location is picked, and from the refresh action -
    /// never from `render`.
    pub(super) fn fetch_library_index(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let settings = crate::settings_global::read(cx);
        let Some((_, location)) = resolve_location(&settings, self.library.selected)
        else {
            self.library.index = LibraryIndex::Unreachable;
            cx.notify();
            return;
        };
        self.library.index = LibraryIndex::Loading;
        self.library.persona_selection.clear();
        self.library.prompt_selection.clear();
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
              let outcome = cx.background_executor()
                              .spawn(async move {
                                  let fetcher = knot_library::fetcher_for(&location);
                                  fetcher.fetch_index(&location)
                              })
                              .await;
              let _ =
                  this.update_in(cx, |view, _window, cx| {
                          view.library.index = match outcome {
                              Ok(index) => LibraryIndex::Loaded(index),
                              Err(LibraryError::UnsupportedFormat(_)) => LibraryIndex::Unsupported,
                              Err(_) => LibraryIndex::Unreachable,
                          };
                          cx.notify();
                      });
          })
          .detach();
    }

    /// Fetches, verifies and imports the selected items, off-thread, then
    /// writes them through the shared settings surface and shows the
    /// outcome - an item that fails either step is reported as unreadable by
    /// title, and the rest still import.
    pub(super) fn import_selected_library_items(&mut self, window: &mut Window,
                                                cx: &mut Context<Self>) {
        let LibraryIndex::Loaded(index) = &self.library.index
        else {
            return;
        };
        let settings = crate::settings_global::read(cx);
        let Some((_, location)) = resolve_location(&settings, self.library.selected)
        else {
            return;
        };
        let selected_ids: BTreeSet<String> = self.library
                                                 .persona_selection
                                                 .iter()
                                                 .chain(self.library.prompt_selection.iter())
                                                 .cloned()
                                                 .collect();
        let items: Vec<IndexItem> = index.items
                                         .iter()
                                         .filter(|item| selected_ids.contains(&item.id))
                                         .cloned()
                                         .collect();
        let index_snapshot = index.clone();
        self.library.persona_selection.clear();
        self.library.prompt_selection.clear();

        cx.spawn_in(window, async move |this, cx| {
              let fetched: Vec<(IndexItem, Result<String, String>)> =
                  cx.background_executor()
                    .spawn(async move {
                        let fetcher = knot_library::fetcher_for(&location);
                        items.into_iter()
                             .map(|item| {
                                 let body = fetcher.fetch_item(&location, &index_snapshot, &item)
                                                   .and_then(|bytes| {
                                                       knot_library::verify(&bytes, &item)?;
                                                       knot_library::split_item(&bytes)
                                                   });
                                 (item, body.map_err(|error| error.to_string()))
                             })
                             .collect()
                    })
                    .await;
              let _ = this.update_in(cx, |view, _window, cx| {
                              let result =
                              knot_core::import::import_items(&crate::settings_global::handle(cx),
                                                              &fetched);
                              view.outcome = Some(summarise(result, &[]));
                              view.redraw_every_window(cx);
                          });
          })
          .detach();
    }
}
