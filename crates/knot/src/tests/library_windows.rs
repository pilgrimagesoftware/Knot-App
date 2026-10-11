//! The Personas, Prompts and Bench windows (#20): the label-lookup helpers
//! they moved out of the settings window with, and the single-instance
//! open/raise behavior each gets through `window_registry`.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::library_window::Library;
use crate::library_window::LibraryWindow;
use crate::library_window::open_library_window;
use crate::tests::workspace;
use crate::window_registry::WindowRegistry;

fn store() -> Arc<Mutex<knot_agents::AgentStore>> {
    Arc::new(Mutex::new(knot_agents::AgentStore::new()))
}

/// Installs the settings surface every pane reads from, rooted at a
/// temporary directory so a test opening a window never touches the
/// developer's own preferences.
///
/// The directory must outlive the test; leaking the handle keeps the path
/// valid and leaves the files for the OS to reap, as
/// `tests::menu_key_equivalents` does.
fn install_settings(cx: &mut gpui_kit::App) {
    let dir = tempfile::tempdir().expect("a temporary settings root");
    let settings = knot_core::Settings::with_store_root(dir.path());
    std::mem::forget(dir);
    crate::settings_global::install(settings, cx);
}

#[test]
fn persona_preview_returns_short_instructions_unchanged() {
    assert_eq!(LibraryWindow::persona_preview("be terse", 80), "be terse");
}

#[test]
fn persona_preview_truncates_long_instructions_with_ellipsis() {
    let instructions = "a".repeat(100);
    let preview = LibraryWindow::persona_preview(&instructions, 80);
    assert_eq!(preview.chars().count(), 81);
    assert!(preview.ends_with('…'));
    assert_eq!(&preview[..80], "a".repeat(80).as_str());
}

/// A persona assigned to an agent can't be deleted - the count drives both
/// the disabled delete button and its tooltip.
#[test]
fn personas_in_use_counts_only_the_agents_that_reference_each_persona() {
    let assigned = Uuid::new_v4();
    let unused = Uuid::new_v4();
    let mut agents = knot_agents::AgentStore::new();
    let ws = workspace("One");
    agents.add_workspace(ws.clone());
    agents.set_current_workspace(ws.id);
    agents.create("~/alpha",
                  knot_agents::CreateOptions { persona_id: Some(assigned),
                                               ..Default::default() });
    agents.create("~/beta",
                  knot_agents::CreateOptions { persona_id: Some(assigned),
                                               ..Default::default() });
    agents.create("~/gamma", knot_agents::CreateOptions::default());

    let in_use = LibraryWindow::personas_in_use(agents.agents());

    assert_eq!(in_use.get(&assigned).copied(), Some(2));
    assert_eq!(in_use.get(&unused).copied(), None);
}

#[test]
fn persona_delete_tooltip_names_the_reason_it_is_disabled() {
    assert_eq!(LibraryWindow::persona_delete_tooltip(0), "Delete persona");
    assert_eq!(LibraryWindow::persona_delete_tooltip(1),
               "In use by 1 agent");
    assert_eq!(LibraryWindow::persona_delete_tooltip(3),
               "In use by 3 agents");
}

/// Every library window's title and its Window menu item, so #20's three
/// windows can't ship either the `library.*.title` or `menu.window.*` key
/// in place of their name.
#[test]
fn every_library_window_label_resolves() {
    for library in Library::ALL {
        let title = library.title();
        assert!(!title.starts_with("library."),
                "{library:?} is missing a window title: {title}");
    }
    for key in ["menu.window.personas",
                "menu.window.prompts",
                "menu.window.bench"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[gpui_kit::test]
fn a_repeat_request_raises_the_open_library_window_rather_than_opening_another(cx: &mut TestAppContext)
{
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          install_settings(cx);

          open_library_window(Library::Personas, store(), cx);
          assert_eq!(cx.windows().len(), 1);

          for _ in 0..9 {
              open_library_window(Library::Personas, store(), cx);
          }
          assert_eq!(cx.windows().len(),
                     1,
                     "ten requests for the Personas window should leave one window");
      });
}

/// Each library opens its own window: Personas, Prompts and Bench do not
/// share a `WindowKey`.
#[gpui_kit::test]
fn each_library_gets_its_own_window(cx: &mut TestAppContext) {
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          install_settings(cx);

          for library in Library::ALL {
              open_library_window(library, store(), cx);
          }
          assert_eq!(cx.windows().len(), Library::ALL.len());
      });
}

/// The Personas and Prompts windows' "Import from Library…" button
/// dispatches the same `OpenImport` action the File menu does - so clicking
/// it opens the Import window exactly as the menu item would.
#[gpui_kit::test]
fn import_from_library_opens_the_import_window(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       WindowRegistry::install(cx);
                       install_settings(cx);
                       crate::import_window::register_import_action(store(), cx);
                       open_library_window(Library::Personas, store(), cx);
                       cx.windows()
                         .last()
                         .copied()
                         .expect("the Personas window opened")
                   });

    cx.update(|cx| {
          window.update(cx, |_, window, app| {
                    window.dispatch_action(Box::new(crate::app_bootstrap::OpenImport), app);
                })
                .expect("the Personas window is open");
      });

    assert_eq!(cx.update(|cx| cx.windows().len()),
               2,
               "the Import window should have opened alongside the Personas window");
}

/// Every library's pane draws a frame without panicking, exercising the
/// live-store reads `personas`, `prompts` and `bench` each make.
#[gpui_kit::test]
fn every_library_window_renders(cx: &mut TestAppContext) {
    let windows = cx.update(|cx| {
                        gpui_kit::init(cx);
                        WindowRegistry::install(cx);
                        install_settings(cx);
                        Library::ALL.map(|library| {
                                        open_library_window(library, store(), cx);
                                        cx.windows()
                                .last()
                                .copied()
                                .unwrap_or_else(|| panic!("{library:?}'s window did not open"))
                                    })
                    });
    for window in windows {
        let mut probe = VisualTestContext::from_window(window, cx);
        probe.run_until_parked();
        probe.update(|window, cx| {
                 _ = window.draw(cx);
             });
    }
}
