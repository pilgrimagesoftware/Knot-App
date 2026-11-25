//! Tests for the permission prompt: painted key hints and the choices it
//! offers.

use knot_acp::PermissionRequest;

use super::fixture::*;
use crate::panel_state::PanelState;
use crate::panel_view::style::*;

// ------------------------------------------------- the prompt's key hints

/// The `debug_bounds` key `Kbd` paints itself under, which is
/// `Keystroke::unparse` and so spells the platform modifier three ways.
/// `debug_bounds` wants a `&'static str`, so these are literals rather
/// than something built from the binding at runtime.
#[cfg(target_os = "macos")]
const ALLOW_HINT: &str = "kbd:cmd-shift-a";
#[cfg(target_os = "windows")]
const ALLOW_HINT: &str = "kbd:win-shift-a";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const ALLOW_HINT: &str = "kbd:super-shift-a";

#[cfg(target_os = "macos")]
const DENY_HINT: &str = "kbd:cmd-shift-d";
#[cfg(target_os = "windows")]
const DENY_HINT: &str = "kbd:win-shift-d";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DENY_HINT: &str = "kbd:super-shift-d";

/// Draws the permission prompt into a real window and returns whether each
/// hint made it into the painted frame.
///
/// The prompt is rendered from inside a view rather than handed to
/// `VisualTestContext::draw`: the decision controls are interactive
/// elements, and gpui resolves those against the currently rendering view,
/// so drawing one as a bare element panics before it can paint.
///
/// `install_actions_and_keys` is what puts the bindings in the keymap the
/// prompt reads, over a temporary settings root so the persist it performs
/// cannot reach the developer's own workspaces and agents.
fn painted_hints(cx: &mut gpui_kit::TestAppContext) -> (bool, bool) {
    let dir = tempfile::tempdir().expect("failed to make a temp settings directory");
    let settings = knot_core::Settings::with_store_root(dir.path());
    std::mem::forget(dir);
    let store = std::sync::Arc::new(parking_lot::Mutex::new(knot_agents::AgentStore::new()));
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::app_bootstrap::install_actions_and_keys(&settings, store, cx);
      });

    let mut state = PanelState::new();
    state.pending_input = Some(crate::panel_state::PendingInput::Permission(permission_request()));
    let request = state.pending_permission().cloned().expect("just set above");

    let window = cx.add_window(|_, _| PromptView { state, request });
    let mut cx = gpui_kit::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    (cx.debug_bounds(ALLOW_HINT).is_some(), cx.debug_bounds(DENY_HINT).is_some())
}

/// A view whose whole body is the permission prompt.
struct PromptView {
    state:   PanelState,
    request: PermissionRequest,
}

impl gpui_kit::Render for PromptView {
    fn render(&mut self, window: &mut gpui_kit::Window, _: &mut gpui_kit::Context<Self>)
              -> impl gpui_kit::IntoElement {
        crate::panel_view::render::render_permission_prompt(&self.state,
                                                            &self.request,
                                                            RiskLevel::Neutral,
                                                            std::rc::Rc::new(|_| {}),
                                                            window)
    }
}

/// The hint has to survive all the way into the painted frame, not merely
/// resolve from the keymap: `Kbd` renders nothing for an action it cannot
/// find, and `children(None)` is silently empty, so every failure between
/// the binding and the pixel looks like a button that simply has no hint.
#[gpui_kit::test]
fn the_prompt_paints_a_key_hint_on_each_decision_button(cx: &mut gpui_kit::TestAppContext) {
    let (allow, deny) = painted_hints(cx);
    assert!(allow, "the Allow button painted no {ALLOW_HINT} hint");
    assert!(deny, "the Deny button painted no {DENY_HINT} hint");
}

/// With kinds, the prompt offers every option by the agent's own name -
/// Always Allow included (#530) - and each decision key's hint sits on the
/// option it answers with, not on whichever is listed first.
#[test]
fn every_offered_option_is_a_choice_with_its_key() {
    use knot_acp::PermissionDecision;

    use crate::panel_view::render::{ChoiceKey, ChoiceLook, prompt_choices};

    let choices = prompt_choices(&claude_request());

    let summary: Vec<_> = choices.iter()
                                 .map(|choice| (choice.label.as_str(), choice.look, choice.key))
                                 .collect();
    assert_eq!(summary,
               [("Always Allow", ChoiceLook::Outline, Some(ChoiceKey::AllowAlways)),
                ("Allow", ChoiceLook::Primary, Some(ChoiceKey::Allow)),
                ("Reject", ChoiceLook::Quiet, Some(ChoiceKey::Deny))]);
    assert_eq!(choices[0].decision, PermissionDecision::Choose(0));
}

/// Without kinds the prompt keeps its Allow and Deny pair, which answer as
/// they always did.
#[test]
fn an_adapter_without_kinds_keeps_allow_and_deny() {
    use knot_acp::PermissionDecision;

    use crate::panel_view::render::{ChoiceKey, prompt_choices};

    let choices = prompt_choices(&permission_request());

    let summary: Vec<_> = choices.iter()
                                 .map(|choice| (choice.decision, choice.key))
                                 .collect();
    assert_eq!(summary,
               [(PermissionDecision::Allow, Some(ChoiceKey::Allow)),
                (PermissionDecision::Deny, Some(ChoiceKey::Deny))]);
    assert!(choices.iter()
                   .all(|choice| !choice.label.starts_with("panel.")),
            "a label rendered its key");
}
