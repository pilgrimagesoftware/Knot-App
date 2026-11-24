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

fn messages() -> Arc<Mutex<knot_messaging::MessageStore>> {
    Arc::new(Mutex::new(knot_messaging::MessageStore::new()))
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

/// A persona assigned to agents can't be deleted, and the agents using it
/// are named (#365) - they drive both the disabled delete button and the
/// "Used by" links.
#[test]
fn persona_users_names_only_the_agents_that_reference_each_persona() {
    let assigned = Uuid::new_v4();
    let unused = Uuid::new_v4();
    let mut agents = knot_agents::AgentStore::new();
    let ws = workspace("One");
    agents.add_workspace(ws.clone());
    agents.set_current_workspace(ws.id);
    let alpha = agents.create("~/alpha",
                              knot_agents::CreateOptions { persona_id: Some(assigned),
                                                           ..Default::default() });
    let beta = agents.create("~/beta",
                             knot_agents::CreateOptions { persona_id: Some(assigned),
                                                          ..Default::default() });
    agents.create("~/gamma", knot_agents::CreateOptions::default());

    let users = LibraryWindow::persona_users(agents.agents());

    let ids: Vec<Uuid> = users[&assigned].iter().map(|user| user.id).collect();
    assert_eq!(ids, [alpha, beta], "in roster order");
    assert_eq!(users[&assigned][0].name, agents.agent(alpha).unwrap().name);
    assert!(!users.contains_key(&unused));
}

/// The delete tooltip says why it is disabled by naming the agents, and is
/// the delete label otherwise.
#[test]
fn persona_delete_tooltip_names_the_agents_using_it() {
    use crate::library_window::PersonaUser;

    for key in ["settings.personas.delete",
                "settings.personas.in_use_by",
                "settings.personas.used_by",
                "settings.personas.show_agent"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }
    assert_eq!(LibraryWindow::persona_delete_tooltip(&[]),
               knot_core::l10n::t("settings.personas.delete"));
    let users = [PersonaUser { id:   Uuid::new_v4(),
                               name: "Alpha".into(), },
                 PersonaUser { id:   Uuid::new_v4(),
                               name: "Beta".into(), }];
    let tooltip = LibraryWindow::persona_delete_tooltip(&users);
    assert!(tooltip.contains("Alpha") && tooltip.contains("Beta"),
            "{tooltip}");
}

/// A "Used by" link takes the user to the agent: its workspace window opens
/// with it selected (#365).
#[gpui_kit::test]
fn a_used_by_link_opens_the_agent(cx: &mut TestAppContext) {
    let mut agents = knot_agents::AgentStore::new();
    let ws = workspace("One");
    let workspace_id = ws.id;
    agents.add_workspace(ws);
    let store = Arc::new(Mutex::new(agents));
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       WindowRegistry::install(cx);
                       install_settings(cx);
                       crate::settings_global::write(cx, |settings| {
                           settings.restore_default_personas()
                                   .expect("the built-in personas restore");
                       });
                       let persona = crate::settings_global::read(cx).active_personas()[0].id;
                       store.lock().create("~/alpha",
                                knot_agents::CreateOptions { persona_id: Some(persona),
                                                             workspace_id: Some(workspace_id),
                                                             agent_type:
                                                                 Some("library-test-inert".into()),
                                                             ..Default::default() });
                       open_library_window(Library::Personas, Arc::clone(&store), messages(), cx);
                       *cx.windows().first().expect("the Personas window")
                   });
    let agent = store.lock().agents()[0].id;
    let mut window = gpui_kit::VisualTestContext::from_window(window, cx);
    window.run_until_parked();

    let link = window.debug_bounds("persona-0-user-0")
                     .expect("the persona in use names its agent");
    window.simulate_click(link.center(), gpui_kit::Modifiers::none());
    window.run_until_parked();

    let selected = cx.update(|cx| {
                         WindowRegistry::workspace_view(crate::window_registry::WindowKey::Workspace(workspace_id), cx)
                             .map(|view| view.read(cx).selected_agent_id())
                     });
    assert_eq!(selected, Some(Some(agent)));
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

          open_library_window(Library::Personas, store(), messages(), cx);
          assert_eq!(cx.windows().len(), 1);

          for _ in 0..9 {
              open_library_window(Library::Personas, store(), messages(), cx);
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
              open_library_window(library, store(), messages(), cx);
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
                       open_library_window(Library::Personas, store(), messages(), cx);
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
                                        open_library_window(library, store(), messages(), cx);
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

/// The Personas toolbar's buttons sit together at the left of their group,
/// as the Prompts pane's do - not spread across its width (#300). Each one
/// starts within a small gap of where the one before it ends.
#[gpui_kit::test]
fn the_personas_toolbar_buttons_sit_together(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       WindowRegistry::install(cx);
                       install_settings(cx);
                       open_library_window(Library::Personas, store(), messages(), cx);
                       cx.windows()
                         .last()
                         .copied()
                         .expect("the Personas window opened")
                   });
    let mut probe = VisualTestContext::from_window(window, cx);
    probe.run_until_parked();
    probe.update(|window, cx| {
             _ = window.draw(cx);
         });

    let bounds = ["personas-add",
                  "personas-restore-defaults",
                  "personas-import-from-library"].map(|selector| {
                                                     probe.debug_bounds(selector)
                 .unwrap_or_else(|| panic!("{selector} was not drawn"))
                                                 });
    for pair in bounds.windows(2) {
        let gap = f32::from(pair[1].left() - pair[0].right());
        assert!((0.0..=12.0).contains(&gap),
                "a toolbar button sits {gap}px from the one before it - spread across the \
                 group rather than grouped at its left");
    }
}

/// Each prompt and persona row offers to submit it to Knot-Library (#43):
/// the action asks for details in a dialog, and Review moves to the file
/// with the dialog still open - nothing is published without the user
/// confirming the reviewed file.
#[gpui_kit::test]
fn a_row_offers_to_submit_its_item_to_knot_library(cx: &mut TestAppContext) {
    for (library, selector) in [(Library::Prompts, "prompt-submit-0"),
                                (Library::Personas, "persona-submit-0")]
    {
        let window = cx.update(|cx| {
                           gpui_kit::init(cx);
                           WindowRegistry::install(cx);
                           install_settings(cx);
                           // An empty folder: the lookup fails offline and
                           // the item goes as new.
                           let folder = tempfile::tempdir().expect("a library folder");
                           cx.set_global(crate::library_window::LookupLocation(
                               knot_library::Location::Folder { path: folder.path().to_owned() },
                           ));
                           std::mem::forget(folder);
                           crate::settings_global::write(cx, |settings| {
                               settings.restore_default_personas()
                                       .expect("the built-in personas restore");
                               let id = settings.add_prompt("Review", "Review it.")
                                                .expect("the prompt is added");
                               settings.set_prompt_description(id, "Reviews the branch.")
                                       .expect("the description is kept");
                           });
                           open_library_window(library, store(), messages(), cx);
                           cx.windows().last().copied().expect("the library window")
                       });
        let mut window = VisualTestContext::from_window(window, cx);
        window.run_until_parked();

        let button = window.debug_bounds(selector)
                           .unwrap_or_else(|| panic!("{selector} was not drawn"));
        window.simulate_click(button.center(), gpui_kit::Modifiers::none());
        window.run_until_parked();
        let open = |window: &mut VisualTestContext| {
            window.update(|window, app| {
                      gpui_kit::component::WindowExt::has_active_dialog(window, app)
                  })
        };
        assert!(open(&mut window), "{library:?}: the submit dialog opened");
        assert!(window.debug_bounds("library-submit-review").is_none(),
                "{library:?}: the dialog starts on the details step");

        // The description field has focus. A prompt already has one; a
        // persona keeps none, so it is typed.
        if library == Library::Personas {
            window.simulate_input("Answers like a careful reviewer.");
            window.run_until_parked();
        }
        window.update(|window, app| {
                  window.dispatch_action(Box::new(gpui_kit::component::dialog::Confirm { secondary: false }),
                                         app);
              });
        window.run_until_parked();
        assert!(open(&mut window),
                "{library:?}: the dialog stays open on the review step, unpublished");
        assert!(window.debug_bounds("library-submit-review").is_some(),
                "{library:?}: Review shows the file");
    }
}

/// A Prompts window row's Duplicate action (#469) puts a copy of that
/// prompt right after it and opens the prompt editor on the copy.
#[gpui_kit::test]
fn a_prompt_row_duplicates_its_prompt(cx: &mut TestAppContext) {
    let (window, original) = cx.update(|cx| {
                                   gpui_kit::init(cx);
                                   WindowRegistry::install(cx);
                                   install_settings(cx);
                                   let id =
                                       crate::settings_global::write_persisting(cx, |settings| {
                                           settings.add_prompt("Review", "Review it.")
                                       }).expect("the prompt is added");
                                   open_library_window(Library::Prompts, store(), messages(), cx);
                                   (cx.windows().last().copied().expect("the library window"), id)
                               });
    let mut window = VisualTestContext::from_window(window, cx);
    window.run_until_parked();

    let button = window.debug_bounds("prompt-duplicate-0")
                       .expect("the row's Duplicate action is drawn");
    window.simulate_click(button.center(), gpui_kit::Modifiers::none());
    window.run_until_parked();

    let (library, editors) = window.update(|_, cx| {
                                       let library =
                                           crate::settings_global::read(cx).prompts.clone();
                                       let editors = cx.windows()
                        .into_iter()
                        .filter_map(|window| window.downcast::<gpui_kit::component::Root>())
                        .filter_map(|root| root.read(cx).ok()?.view().clone().downcast().ok())
                        .map(|editor: gpui_kit::Entity<crate::library_window::PromptEditor>| {
                            editor.read(cx).editing_id()
                        })
                        .collect::<Vec<_>>();
                                       (library, editors)
                                   });
    assert_eq!(library.len(), 2);
    assert_eq!(library[0].id, original);
    assert_eq!(library[1].text, "Review it.");
    assert_eq!(editors, [Some(library[1].id)]);
}
