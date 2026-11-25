//! Shift alternates in an open agent row context menu (#475;
//! `agent-list-ui`: "Shift alternates in the agent row context menu", "A
//! Shift marker shows which lines have an alternate").
//!
//! Driven through a real right-click on a real row, because what is under
//! test is gpui's behaviour around the popup: that an open menu redraws on
//! a Shift edge only because the window root refreshes it, and that a click
//! inside the popup has to read Shift from the window rather than from its
//! `ClickEvent`.

use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::MouseDownEvent;
use gpui_kit::MouseUpEvent;
use gpui_kit::TestAppContext;
use knot_core::LayoutMode;

use super::layout_test_support::Fixture;
use super::layout_test_support::Spec;
use super::layout_test_support::open;
use super::layout_test_support::open_as;
use super::menus::line_selector;
use crate::agent_menu_entries::AgentMenuEntry;

const AGENTS: &Spec = &Spec { agents:     &["A", "B"],
                              companions: &[],
                              mode:       LayoutMode::Single,
                              panes:      &["B"],
                              focus:      0, };

fn label(entry: AgentMenuEntry) -> String {
    entry.label().expect("a labelled entry")
}

impl Fixture {
    /// Makes `name` a running agent with ACP session `a1`, and turns
    /// restored conversations on or off.
    fn prepare(&mut self, name: &str, restores_conversations: bool) {
        let id = self.id(name);
        {
            let mut store = self.store.lock();
            store.set_activated(id, true);
            store.set_acp_session_id(id, "a1".to_string());
        }
        self.window.update(|_, cx| {
                       crate::settings_global::write(cx, |settings| {
                           settings.restore_conversation_on_launch = restores_conversations;
                       });
                   });
        self.window.run_until_parked();
    }

    fn right_click_row(&mut self, name: &str) {
        let selector: &'static str =
            Box::leak(format!("workspace-agent-row-{}", self.row(name)).into_boxed_str());
        let at = self.window
                     .debug_bounds(selector)
                     .expect("the row is painted")
                     .center();
        self.right_click_at(at);
    }

    /// Right-clicks the sidebar's empty space below `last`'s row.
    fn right_click_sidebar_below(&mut self, last: &str) {
        let selector: &'static str =
            Box::leak(format!("workspace-agent-row-{}", self.row(last)).into_boxed_str());
        let row = self.window
                      .debug_bounds(selector)
                      .expect("the row is painted");
        let at = gpui_kit::point(row.center().x, row.bottom() + gpui_kit::px(40.));
        self.right_click_at(at);
    }

    fn right_click_at(&mut self, at: gpui_kit::Point<gpui_kit::Pixels>) {
        self.window
            .simulate_event(MouseDownEvent { button:      MouseButton::Right,
                                             position:    at,
                                             modifiers:   Modifiers::none(),
                                             click_count: 1,
                                             first_mouse: false, });
        self.window
            .simulate_event(MouseUpEvent { button:      MouseButton::Right,
                                           position:    at,
                                           modifiers:   Modifiers::none(),
                                           click_count: 1, });
        self.window.run_until_parked();
    }

    /// Whether an alternate line currently reading `title` is drawn.
    fn shows_line(&mut self, title: &str) -> bool {
        let selector: &'static str = Box::leak(line_selector(title).into_boxed_str());
        self.window.debug_bounds(selector).is_some()
    }

    fn line_center(&mut self, title: &str) -> gpui_kit::Point<gpui_kit::Pixels> {
        let selector: &'static str = Box::leak(line_selector(title).into_boxed_str());
        self.window
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("no line reads {title}"))
            .center()
    }

    fn shift(&mut self, down: bool) {
        let modifiers = if down {
            Modifiers::shift()
        }
        else {
            Modifiers::none()
        };
        self.window.simulate_modifiers_change(modifiers);
        self.window.run_until_parked();
    }

    /// Accepts the confirmation the last choice opened.
    fn confirm(&mut self) {
        self.press(gpui_kit::component::dialog::Confirm { secondary: false });
    }

    fn acp_session(&self, name: &str) -> Option<String> {
        self.store
            .lock()
            .agent(self.id(name))
            .and_then(|agent| agent.acp_session_id.clone())
    }
}

#[gpui_kit::test]
fn holding_shift_swaps_the_titles_of_an_open_menu(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    assert!(fixture.shows_line(&label(AgentMenuEntry::RestartAgent)));
    assert!(fixture.shows_line(&label(AgentMenuEntry::Deactivate)));

    fixture.shift(true);
    assert!(fixture.shows_line(&label(AgentMenuEntry::RestartWithNewConversation)));
    assert!(fixture.shows_line(&label(AgentMenuEntry::DeactivateWithNewSession)));
    assert!(!fixture.shows_line(&label(AgentMenuEntry::RestartAgent)));

    fixture.shift(false);
    assert!(fixture.shows_line(&label(AgentMenuEntry::RestartAgent)));
    assert!(fixture.shows_line(&label(AgentMenuEntry::Deactivate)));
}

#[gpui_kit::test]
fn a_shift_click_runs_the_alternate(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    fixture.shift(true);
    let at = fixture.line_center(&label(AgentMenuEntry::DeactivateWithNewSession));
    fixture.window.simulate_click(at, Modifiers::shift());
    fixture.window.run_until_parked();
    fixture.shift(false);
    // Plain Deactivate would have run at once and kept the session; the
    // alternate asks first, and forgets it.
    assert!(fixture.is_activated("A"),
            "nothing runs before the confirmation");
    fixture.confirm();
    assert!(!fixture.is_activated("A"));
    assert_eq!(fixture.acp_session("A"), None);
}

#[gpui_kit::test]
fn a_plain_click_runs_the_primary(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    let at = fixture.line_center(&label(AgentMenuEntry::Deactivate));
    fixture.window.simulate_click(at, Modifiers::none());
    fixture.window.run_until_parked();
    assert!(!fixture.is_activated("A"));
    assert_eq!(fixture.acp_session("A").as_deref(), Some("a1"));
}

#[gpui_kit::test]
fn shift_return_runs_the_alternate(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    // What `app_bootstrap` binds; the fixture does not run it.
    fixture.window
           .update(|_, cx| cx.bind_keys(super::menus::alternate_line_key_bindings()));
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    let at = fixture.line_center(&label(AgentMenuEntry::Deactivate));
    fixture.window
           .simulate_mouse_move(at, None, Modifiers::none());
    fixture.window.run_until_parked();
    fixture.shift(true);
    fixture.window.simulate_keystrokes("shift-enter");
    fixture.window.run_until_parked();
    fixture.shift(false);
    assert!(fixture.is_activated("A"),
            "nothing runs before the confirmation");
    fixture.confirm();
    assert!(!fixture.is_activated("A"));
    assert_eq!(fixture.acp_session("A"), None);
}

#[gpui_kit::test]
fn a_shell_has_no_alternate_lines(cx: &mut TestAppContext) {
    let mut fixture = open_as(AGENTS, knot_core::agent_type::SHELL, cx);
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    fixture.shift(true);
    for entry in [AgentMenuEntry::RestartAgent,
                  AgentMenuEntry::RestartWithNewConversation,
                  AgentMenuEntry::Deactivate,
                  AgentMenuEntry::DeactivateWithNewSession]
    {
        assert!(!fixture.shows_line(&label(entry)), "{entry:?}");
    }
}

#[gpui_kit::test]
fn no_alternate_lines_without_restored_conversations(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", false);
    fixture.right_click_row("A");
    fixture.shift(true);
    for entry in [AgentMenuEntry::RestartAgent,
                  AgentMenuEntry::RestartWithNewConversation,
                  AgentMenuEntry::Deactivate,
                  AgentMenuEntry::DeactivateWithNewSession]
    {
        assert!(!fixture.shows_line(&label(entry)), "{entry:?}");
    }
}

#[gpui_kit::test]
fn selecting_after_deactivate_with_new_session_starts_fresh(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", true);
    fixture.right_click_row("A");
    fixture.shift(true);
    let at = fixture.line_center(&label(AgentMenuEntry::DeactivateWithNewSession));
    fixture.window.simulate_click(at, Modifiers::shift());
    fixture.window.run_until_parked();
    fixture.shift(false);
    fixture.confirm();

    fixture.click_row("A", Modifiers::none());
    assert!(fixture.is_activated("A"));
    let to_load = fixture.store
                         .lock()
                         .agent(fixture.id("A"))
                         .and_then(|agent| agent.session_to_load().map(str::to_owned));
    assert_eq!(to_load, None, "the next start is a new session");
}

#[gpui_kit::test]
fn the_sidebar_offers_deactivate_all_with_new_sessions(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", true);
    fixture.prepare("B", true);
    fixture.right_click_sidebar_below("B");
    let restart_all = crate::app_state::AgentListBackgroundEntry::RestartAll;
    let deactivate_all = crate::app_state::AgentListBackgroundEntry::DeactivateAll;
    let restart_new = restart_all.shift_alternate(true).expect("an alternate");
    let deactivate_new = deactivate_all.shift_alternate(true).expect("an alternate");
    assert!(fixture.shows_line(&restart_all.label().expect("a label")));
    assert!(fixture.shows_line(&deactivate_all.label().expect("a label")));

    fixture.shift(true);
    assert!(fixture.shows_line(&restart_new.label().expect("a label")));
    let at = fixture.line_center(&deactivate_new.label().expect("a label"));
    fixture.window.simulate_click(at, Modifiers::shift());
    fixture.window.run_until_parked();
    fixture.shift(false);
    assert!(fixture.is_activated("A"), "nothing runs before the confirmation");
    fixture.confirm();
    for name in ["A", "B"] {
        assert!(!fixture.is_activated(name), "{name}");
        assert_eq!(fixture.acp_session(name), None, "{name}");
    }
}

#[gpui_kit::test]
fn the_sidebar_has_no_alternates_without_restored_conversations(cx: &mut TestAppContext) {
    let mut fixture = open(AGENTS, cx);
    fixture.prepare("A", false);
    fixture.right_click_sidebar_below("B");
    fixture.shift(true);
    let restart_all = crate::app_state::AgentListBackgroundEntry::RestartAll;
    assert!(!fixture.shows_line(&restart_all.label().expect("a label")));
    assert!(!fixture.shows_line(&knot_core::l10n::t("menu.sidebar.restart_all_new_conversations")));
}
