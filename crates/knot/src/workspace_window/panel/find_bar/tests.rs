//! The conversation's find bar in a real workspace window (#251;
//! `acp-panel-ui`, "Finding text in the conversation").
//!
//! The agent uses an agent type with no terminal process and no ACP adapter,
//! so nothing launches; its conversation is written straight into the
//! connected fake session's state.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::PanelSessionSlot;
use crate::panel_state::PanelMessage;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

const INERT_AGENT_TYPE: &str = "find-bar-test-inert";

struct Fixture {
    window:    VisualTestContext,
    view:      Entity<WorkspaceWindow>,
    agent:     Uuid,
    composer:  Entity<PanelInputState>,
    _settings: TempDir,
    _folder:   TempDir,
    _runtime:  tokio::runtime::Runtime,
}

/// A workspace window over one agent whose conversation is `messages`, with
/// the composer focused.
fn window_with(cx: &mut TestAppContext, conversation: Vec<PanelMessage>) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = crate::workspace_window::panel::input::tests::ready_slot(&runtime);
    if let PanelSessionSlot::Ready(handle) = &*slot.lock() {
        handle.state().lock().messages = conversation;
    }
    let folder = TempDir::new().expect("a temporary agent folder");
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
    let settings = knot_core::Settings::with_store_root(settings_dir.path());
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     // The app's fixed keys, ⌘F among them. On macOS the
                     // composer's own ⌘F (the input's `Search`) gets there
                     // first and is handed up; elsewhere the input's find key
                     // is Ctrl-F, and ⌘F is this binding alone.
                     cx.bind_keys(crate::keymap::fixed_bindings().into_iter()
                                                                 .map(|fixed| fixed.binding));
                     WindowRegistry::install(cx);
                     crate::settings_global::install(settings, cx);
                     WorkspaceWindow::open(store, messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let mut window = VisualTestContext::from_window(handle, cx);
    let composer = view.update_in(&mut window, |view, window, cx| {
                           view.panel_sessions.insert(agent, slot);
                           view.store.lock().set_activated(agent, true);
                           view.selected_agent = Some(agent);
                           cx.notify();
                           view.panel_prompt_input(agent, window, cx)
                       });
    window.run_until_parked();
    composer.update_in(&mut window, |state, window, cx| state.focus(window, cx));
    window.run_until_parked();
    Fixture { window,
              view,
              agent,
              composer,
              _settings: settings_dir,
              _folder: folder,
              _runtime: runtime }
}

impl Fixture {
    fn press(&mut self, keys: &str) {
        self.window.simulate_keystrokes(keys);
        self.window.run_until_parked();
    }

    fn search(&mut self, query: &str) {
        let agent = self.agent;
        let input = self.view
                        .read_with(&self.window, |view, _| view.panel_find_input(agent))
                        .expect("the find bar is open");
        let query = query.to_owned();
        input.update_in(&mut self.window, |state, window, cx| {
                 state.set_value(query, window, cx)
             });
        self.window.run_until_parked();
    }

    /// `(current match from 1, total)`, or `None` with the bar closed.
    fn position(&mut self) -> Option<(Option<usize>, usize)> {
        let agent = self.agent;
        self.view
            .read_with(&self.window, |view, _| view.panel_find_position(agent))
            .map(|(current, total)| (current.map(|at| at + 1), total))
    }

    fn bar_shown(&mut self) -> bool {
        self.window.debug_bounds("panel-find-bar").is_some()
    }

    fn composer_focused(&mut self) -> bool {
        let composer = self.composer.clone();
        self.window
            .update(|window, cx| composer.read(cx).focus_handle(cx).is_focused(window))
    }

    fn top_message(&mut self) -> usize {
        let agent = self.agent;
        self.view.read_with(&self.window, |view, _| {
                     view.panel_lists[&agent].list.logical_scroll_top().item_ix
                 })
    }

    fn toggle_regex(&mut self) {
        let agent = self.agent;
        self.view.update(&mut self.window, |view, cx| {
                     view.toggle_panel_find_mode(agent, cx)
                 });
        self.window.run_until_parked();
    }
}

fn conversation() -> Vec<PanelMessage> {
    vec![PanelMessage::User("Run the gate".into()),
         PanelMessage::Assistant("The **gate** passed. Ran the gate twice.".into()),
         PanelMessage::Notice("gate notice, not searched".into()),
         PanelMessage::User("ship v1.2".into())]
}

/// ⌘F from the composer opens the bar - the composer hands its own find
/// key up - and focus moves to the find field.
#[gpui_kit::test]
fn command_f_in_the_composer_opens_the_find_bar(cx: &mut TestAppContext) {
    let mut fixture = window_with(cx, conversation());
    assert!(!fixture.bar_shown());

    fixture.press("cmd-f");

    assert!(fixture.bar_shown());
    assert!(!fixture.composer_focused(), "focus moves to the find field");
}

/// A text field's own find key - the input's `Search`, ⌘F on macOS and
/// Ctrl-F elsewhere - opens the bar too: the composer is not searchable, so
/// it hands the action up to the pane.
#[gpui_kit::test]
fn the_composers_own_find_key_opens_the_find_bar(cx: &mut TestAppContext) {
    let mut fixture = window_with(cx, conversation());

    fixture.window
           .dispatch_action(gpui_kit::component::input::Search);
    fixture.window.run_until_parked();

    assert!(fixture.bar_shown());
}

/// "Matches are counted and stepped through": prompts and replies are
/// searched, notices are not; Enter and Shift-Enter step, wrapping.
#[gpui_kit::test]
fn matches_are_counted_and_stepped_through(cx: &mut TestAppContext) {
    let mut fixture = window_with(cx, conversation());
    fixture.press("cmd-f");
    fixture.search("GATE");
    assert_eq!(fixture.position(), Some((Some(1), 3)));

    fixture.press("enter");
    assert_eq!(fixture.position(), Some((Some(2), 3)));
    fixture.press("enter");
    fixture.press("enter");
    assert_eq!(fixture.position(),
               Some((Some(1), 3)),
               "stepping past the last wraps");
    fixture.press("shift-enter");
    assert_eq!(fixture.position(),
               Some((Some(3), 3)),
               "and back past the first");

    fixture.search("nowhere");
    assert_eq!(fixture.position(), Some((None, 0)));
}

#[gpui_kit::test]
fn a_regular_expression_finds_its_pattern(cx: &mut TestAppContext) {
    let mut fixture = window_with(cx, conversation());
    fixture.press("cmd-f");
    fixture.search(r"v\d+\.\d+");
    assert_eq!(fixture.position(),
               Some((None, 0)),
               "as text, it is not there");

    fixture.toggle_regex();
    assert_eq!(fixture.position(), Some((Some(1), 1)));

    fixture.search("(unclosed");
    assert_eq!(fixture.position(),
               Some((None, 0)),
               "an invalid pattern finds nothing");
}

/// Stepping to a match far from the view takes the conversation there.
#[gpui_kit::test]
fn stepping_takes_the_conversation_to_the_match(cx: &mut TestAppContext) {
    let mut messages: Vec<PanelMessage> =
        (0..80).map(|n| PanelMessage::User(format!("filler {n}")))
               .collect();
    messages[3] = PanelMessage::User("needle early".into());
    messages[70] = PanelMessage::User("needle late".into());
    let mut fixture = window_with(cx, messages);
    fixture.press("cmd-f");
    fixture.search("needle");

    let first = fixture.position();
    let target = if first == Some((Some(1), 2)) { 70 } else { 3 };
    fixture.press("enter");

    assert_eq!(fixture.top_message(), target);
}

/// Esc closes the bar and gives the composer its focus back.
#[gpui_kit::test]
fn escape_closes_the_bar_and_returns_to_the_composer(cx: &mut TestAppContext) {
    let mut fixture = window_with(cx, conversation());
    fixture.press("cmd-f");
    fixture.search("gate");

    fixture.press("escape");

    assert!(!fixture.bar_shown());
    assert_eq!(fixture.position(), None);
    assert!(fixture.composer_focused());
}

#[test]
fn the_find_bar_copy_resolves() {
    for key in ["panel.find.placeholder",
                "panel.find.regex",
                "panel.find.no_matches",
                "panel.find.invalid",
                "panel.find.previous",
                "panel.find.next",
                "panel.find.close",
                "keymap.fixed.find"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
    }
    let count = knot_core::l10n::t_with("panel.find.count", &[("current", "2"), ("total", "7")]);
    assert!(count.contains('2') && count.contains('7'), "{count}");
}
