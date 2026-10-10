//! The library window entity and how it opens: one instance per [`Library`],
//! single-instance like the Command Center and Workspaces windows -
//! requesting one that is already open raises it instead of opening another.
//!
//! What it draws lives in `super::render` and the `panes` modules.

use std::sync::Arc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::component::Root;
use parking_lot::Mutex;

use super::library::Library;
use crate::app_support::observe_system_appearance;
use crate::window_options::library_window_options;
use crate::window_registry::activate_or_open;

pub(crate) struct LibraryWindow {
    /// The live agent store, for questions this window's own `Settings`
    /// snapshot can't answer truthfully - whether a persona is still
    /// assigned to an agent, which changes while this window is open.
    pub(super) store:   Arc<Mutex<knot_agents::AgentStore>>,
    pub(super) library: Library,
}

/// Opens `library`'s window, or raises it if one is already open.
pub(crate) fn open_library_window(library: Library, store: Arc<Mutex<knot_agents::AgentStore>>,
                                  cx: &mut App) {
    activate_or_open(library.window_key(), cx, move |cx| {
        let options = library_window_options(library, cx);
        match cx.open_window(options, move |window, cx| {
                    // Every window tracks the OS appearance, so a light/dark
                    // flip re-resolves the system palette and repaints.
                    observe_system_appearance(window);
                    let view = cx.new(|_| LibraryWindow { store, library });
                    cx.new(|cx| Root::new(view, window, cx))
                }) {
            Ok(window) => Some(AnyWindowHandle::from(window)),
            Err(error) => {
                eprintln!("failed to open {} window: {error}", library.title());
                None
            }
        }
    });
}
