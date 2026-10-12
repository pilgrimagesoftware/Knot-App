//! The composer's Insert Prompt picker in a real workspace window
//! (`prompt-library`: "The composer's Insert Prompt list" and "Choosing a
//! prompt from the Insert Prompt list"; `keybindings`: "Inserting a prompt by
//! shortcut").
//!
//! The agent uses an agent type with no terminal process and no ACP
//! adapter, so nothing launches, and its folder is a fresh repository on
//! `main` so `{{branch}}` has something to read. Settings are rooted at a
//! temporary directory.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use knot_core::Prompt;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::picker_rows;
use crate::keymap::InsertPrompt;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::input::PickerRows;
use crate::workspace_window::panel::prompt::PanelInputState;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "prompt-picker-test-inert";

struct Fixture {
    window:         VisualTestContext,
    view:           Entity<WorkspaceWindow>,
    agent:          Uuid,
    input:          Entity<PanelInputState>,
    /// How many times the Prompts window was asked for.
    opened_prompts: Rc<Cell<usize>>,
    _settings:      TempDir,
    _folder:        TempDir,
    /// Drives the session behind the composer, which is drawn - and so
    /// receives the shortcut - only while that session is `Ready`.
    _runtime:       tokio::runtime::Runtime,
}

fn repository_on_main() -> TempDir {
    let dir = TempDir::new().expect("a temporary agent folder");
    let run = |args: &[&str]| {
        knot_git::Runner::new(dir.path()).run(args)
                                         .expect("git runs in the test folder");
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    run(&["commit", "-qm", "root", "--allow-empty"]);
    dir
}

/// A workspace window over one agent with `library` as the prompt library
/// and an empty composer.
fn window_with_library(cx: &mut TestAppContext, library: Vec<Prompt>) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = crate::workspace_window::panel::input::tests::ready_slot(&runtime);
    let folder = repository_on_main();
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agent = store.create(folder.path().to_string_lossy().into_owned(),
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let settings_dir = TempDir::new().expect("a temporary settings root");
    let mut settings = knot_core::Settings::with_store_root(settings_dir.path());
    settings.prompts = library;
    let opened_prompts = Rc::new(Cell::new(0));
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     crate::settings_global::install(settings, cx);
                     let opened = Rc::clone(&opened_prompts);
                     cx.on_action(move |_: &crate::app_bootstrap::OpenPrompts, _| {
                           opened.set(opened.get() + 1)
                       });
                     WorkspaceWindow::open(store, messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let mut window = VisualTestContext::from_window(handle, cx);
    let input = view.update_in(&mut window, |view, window, cx| {
                        // Activated with a session already in place, so the
                        // window draws the composer rather than the stopped
                        // pane, and connects nothing of its own.
                        view.panel_sessions.insert(agent, slot);
                        view.store.lock().set_activated(agent, true);
                        view.selected_agent = Some(agent);
                        cx.notify();
                        view.panel_prompt_input(agent, window, cx)
                    });
    window.run_until_parked();
    Fixture { window,
              view,
              agent,
              input,
              opened_prompts,
              _settings: settings_dir,
              _folder: folder,
              _runtime: runtime }
}

fn prompt(name: &str, text: &str) -> Prompt {
    Prompt::new(name, text).expect("a valid prompt")
}

impl Fixture {
    /// Puts `value` in the composer with `selection` selected.
    fn compose(&mut self, value: &str, selection: std::ops::Range<usize>) {
        let value = value.to_string();
        self.input.update_in(&mut self.window, |state, window, cx| {
                      state.set_value(value, window, cx);
                      state.set_selected_range(selection, cx);
                  });
    }

    fn open(&mut self) {
        let agent = self.agent;
        self.view.update_in(&mut self.window, |view, window, cx| {
                     view.open_prompt_picker(agent, window, cx)
                 });
        self.window.run_until_parked();
    }

    fn search(&mut self, query: &str) {
        let query = query.to_string();
        let search = self.view
                         .read_with(&self.window, |view, _| {
                             view.prompt_picker
                                 .as_ref()
                                 .map(|picker| picker.search.clone())
                         })
                         .expect("the picker is open");
        search.update_in(&mut self.window, |state, window, cx| {
                  state.set_value(query, window, cx)
              });
        self.window.run_until_parked();
    }

    fn choose(&mut self, index: usize) {
        self.view.update_in(&mut self.window, |view, window, cx| {
                     view.choose_prompt_picker(Some(index), window, cx)
                 });
        self.window.run_until_parked();
    }

    fn is_open(&mut self) -> bool {
        let agent = self.agent;
        self.view
            .read_with(&self.window, |view, _| view.prompt_picker_open(agent))
    }

    fn rows(&mut self) -> Option<PickerRows> {
        self.view
            .update(&mut self.window, |view, cx| view.prompt_picker_rows(cx))
    }

    fn row_names(&mut self) -> Vec<String> {
        match self.rows() {
            Some(PickerRows::Prompts(prompts)) => {
                prompts.into_iter().map(|prompt| prompt.name).collect()
            }
            other => panic!("expected prompt rows, got {other:?}"),
        }
    }

    fn composer(&mut self) -> (String, std::ops::Range<usize>) {
        let input = self.input.clone();
        self.window.update(|_, cx| {
                       let state = input.read(cx);
                       (state.value().to_string(), state.selected_range())
                   })
    }

    fn composer_focused(&mut self) -> bool {
        let input = self.input.clone();
        self.window
            .update(|window, cx| input.read(cx).focus_handle(cx).is_focused(window))
    }

    fn nothing_sent(&mut self) -> bool {
        let agent = self.agent;
        self.view.read_with(&self.window, |view, _| {
                     view.panel_prompt_queues
                         .get(&agent)
                         .is_none_or(|queue| queue.is_empty())
                     && !view.panel_prompts_in_flight.contains_key(&agent)
                 })
    }
}

#[test]
fn rows_filter_by_name_or_text_in_library_order() {
    let library = [prompt("Review", "look over the diff"),
                   prompt("Run the gate", "make, then commit"),
                   prompt("Ship", "push and open the GATE PR")];

    let names = |query| match picker_rows(&library, query) {
        PickerRows::Prompts(prompts) => prompts.into_iter().map(|p| p.name).collect::<Vec<_>>(),
        PickerRows::EmptyLibrary => panic!("the library is not empty"),
    };

    assert_eq!(names(""), ["Review", "Run the gate", "Ship"]);
    assert_eq!(names("gate"), ["Run the gate", "Ship"]);
    assert_eq!(names("DIFF"), ["Review"]);
    assert!(names("nothing like it").is_empty());
    assert_eq!(picker_rows(&[], "gate"), PickerRows::EmptyLibrary);
}

/// "Insert at the caret".
#[gpui_kit::test]
fn a_prompt_is_inserted_at_the_caret(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, vec![prompt("Review", "review {{branch}}")]);
    fixture.compose("Please ", 7..7);

    fixture.open();
    assert!(fixture.is_open());
    fixture.choose(0);

    let (value, selection) = fixture.composer();
    assert_eq!(value, "Please review main");
    assert_eq!(selection,
               value.len()..value.len(),
               "the caret is after the insertion");
    assert!(!fixture.is_open());
    assert!(fixture.composer_focused());
    assert!(fixture.nothing_sent());
}

/// "Replace a selection", leaving the rest of the buffer alone.
#[gpui_kit::test]
fn a_prompt_replaces_the_selection(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, vec![prompt("Tests", "run the tests")]);
    fixture.compose("first TODO last", 6..10);

    fixture.open();
    fixture.choose(0);

    assert_eq!(fixture.composer().0, "first run the tests last");
    assert!(fixture.nothing_sent());
}

/// "Filtering".
#[gpui_kit::test]
fn typing_filters_the_list(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx,
                                          vec![prompt("Review", "look it over"),
                                               prompt("Run the gate", "make")]);
    fixture.open();
    assert_eq!(fixture.row_names(), ["Review", "Run the gate"]);

    fixture.search("gate");

    assert_eq!(fixture.row_names(), ["Run the gate"]);
}

/// "Empty library": the one row offers the Prompts window, and choosing it
/// opens that rather than inserting anything.
#[gpui_kit::test]
fn an_empty_library_offers_the_prompts_window(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, Vec::new());
    fixture.compose("keep me", 7..7);

    fixture.open();
    assert_eq!(fixture.rows(), Some(PickerRows::EmptyLibrary));
    let key = "panel.prompt_picker.empty_library";
    assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    fixture.choose(0);

    assert_eq!(fixture.opened_prompts.get(), 1);
    assert!(!fixture.is_open());
    assert_eq!(fixture.composer().0, "keep me");
}

/// "Dismissing changes nothing": Esc in the search field closes the list
/// and hands focus back with the text and selection as they were.
#[gpui_kit::test]
fn escape_changes_nothing(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, vec![prompt("Review", "review it")]);
    fixture.compose("half a thought", 5..9);

    fixture.open();
    assert!(!fixture.composer_focused(), "the search field takes focus");
    fixture.window
           .dispatch_action(gpui_kit::component::input::Escape);
    fixture.window.run_until_parked();

    assert!(!fixture.is_open());
    assert_eq!(fixture.composer(), ("half a thought".to_string(), 5..9));
    assert!(fixture.composer_focused());
}

/// `keybindings`' "Default opens the picker": the shortcut, dispatched with
/// the composer focused, opens that composer's list.
#[gpui_kit::test]
fn the_shortcut_opens_the_focused_composers_picker(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, vec![prompt("Review", "review it")]);
    let input = fixture.input.clone();
    fixture.window
           .update(|window, cx| input.update(cx, |state, cx| state.focus(window, cx)));
    fixture.window.run_until_parked();

    fixture.window.dispatch_action(InsertPrompt);
    fixture.window.run_until_parked();

    assert!(fixture.is_open());
}

/// "No composer focused": with focus elsewhere in the window, the shortcut
/// does nothing.
#[gpui_kit::test]
fn the_shortcut_does_nothing_without_a_focused_composer(cx: &mut TestAppContext) {
    let mut fixture = window_with_library(cx, vec![prompt("Review", "review it")]);
    let root = fixture.view
                      .read_with(&fixture.window, |view, _| view.root_focus.clone());
    fixture.window.update(|window, cx| window.focus(&root, cx));
    fixture.window.run_until_parked();

    fixture.window.dispatch_action(InsertPrompt);
    fixture.window.run_until_parked();

    assert!(!fixture.is_open());
}
