//! The question card in a real workspace window (#430): it takes focus when
//! it appears, answers from the keyboard, leaves typing in its inputs alone,
//! and gives focus back to the composer when it goes.
//!
//! The session is the connected fake from `input::tests::ready_slot`; the
//! question is written straight into its state, as the ACP drain would.
//! What reaches the agent is `knot-acp`'s to test; here it is what the card
//! does to the state and to focus.

use std::sync::Arc;

use gpui_kit::{Entity, Focusable, TestAppContext, VisualTestContext};
use knot_acp::{Choice, FieldKind, FormField, QuestionRequest};
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::PanelSessionSlot;
use crate::panel_state::{FieldValue, QuestionState};
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

const INERT_AGENT_TYPE: &str = "question-card-test-inert";

struct Fixture {
    window:    VisualTestContext,
    view:      Entity<WorkspaceWindow>,
    agent:     Uuid,
    slot:      Arc<Mutex<PanelSessionSlot>>,
    composer:  Entity<PanelInputState>,
    _settings: TempDir,
    _folder:   TempDir,
    _runtime:  tokio::runtime::Runtime,
}

fn choices(values: &[&str]) -> Vec<Choice> {
    values.iter()
          .map(|value| Choice { value:       (*value).to_owned(),
                                title:       (*value).to_owned(),
                                description: None, })
          .collect()
}

fn text_field(key: &str) -> FormField {
    FormField { key:         key.to_owned(),
                title:       Some("Other".to_owned()),
                description: None,
                required:    false,
                kind:        FieldKind::Text { default:    None,
                                               min_length: None,
                                               max_length: None, }, }
}

fn color_and_fruits() -> QuestionRequest {
    QuestionRequest { rpc_id:       serde_json::json!(200),
                      session_id:   "sess-1".to_owned(),
                      tool_call_id: Some("toolu_1".to_owned()),
                      message:      "Please answer the following questions.".to_owned(),
                      fields:       vec![FormField { key:         "question_0".to_owned(),
                                                     title:       Some("Color".to_owned()),
                                                     description: Some("Which color?".to_owned()),
                                                     required:    false,
                                                     kind:        FieldKind::Single { choices:
                                                                                          choices(&["Red",
                                                                                                    "Green",
                                                                                                    "Blue"]),
                                                                                      default: None, }, },
                                         text_field("question_0_custom"),
                                         FormField { key:         "question_1".to_owned(),
                                                     title:       Some("Fruits".to_owned()),
                                                     description: Some("Which fruits?".to_owned()),
                                                     required:    false,
                                                     kind:        FieldKind::Multi { choices:
                                                                                         choices(&["Apple",
                                                                                                   "Pear",
                                                                                                   "Plum"]),
                                                                                     default:
                                                                                         Vec::new(),
                                                                                     min_items:
                                                                                         None,
                                                                                     max_items:
                                                                                         None, }, },
                                         text_field("question_1_custom")], }
}

fn window_with_question(cx: &mut TestAppContext) -> Fixture {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = crate::workspace_window::panel::input::tests::ready_slot(&runtime);
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
                     cx.bind_keys(crate::keymap::fixed_bindings().into_iter()
                                                                 .map(|fixed| fixed.binding));
                     cx.bind_keys(crate::panel_view::question_card_key_bindings());
                     WindowRegistry::install(cx);
                     crate::settings_global::install(settings, cx);
                     WorkspaceWindow::open(store, messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let mut window = VisualTestContext::from_window(handle, cx);
    let composer = view.update_in(&mut window, |view, window, cx| {
                           view.panel_sessions.insert(agent, Arc::clone(&slot));
                           view.store.lock().set_activated(agent, true);
                           view.selected_agent = Some(agent);
                           cx.notify();
                           view.panel_prompt_input(agent, window, cx)
                       });
    window.run_until_parked();
    composer.update_in(&mut window, |state, window, cx| state.focus(window, cx));
    window.run_until_parked();
    if let PanelSessionSlot::Ready(handle) = &*slot.lock() {
        handle.state().lock().pending_input = Some(color_and_fruits().into());
    }
    view.update_in(&mut window, |_, _, cx| cx.notify());
    window.run_until_parked();
    Fixture { window,
              view,
              agent,
              slot,
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

    fn question(&self) -> Option<QuestionState> {
        match &*self.slot.lock() {
            PanelSessionSlot::Ready(handle) => handle.state().lock().pending_question().cloned(),
            _ => None,
        }
    }

    fn card_focused(&mut self) -> bool {
        let agent = self.agent;
        self.view.update_in(&mut self.window, |view, window, _| {
                     view.panel_questions
                         .get(&agent)
                         .is_some_and(|ui| ui.focus.is_focused(window))
                 })
    }

    fn composer_focused(&mut self) -> bool {
        let composer = self.composer.clone();
        self.window
            .update(|window, cx| composer.focus_handle(cx).is_focused(window))
    }
}

/// "Question card takes focus", "Answering from the keyboard", and focus
/// going back to the composer once the card is answered.
#[gpui_kit::test]
fn the_card_takes_focus_and_answers_from_the_keyboard(cx: &mut TestAppContext) {
    let mut fixture = window_with_question(cx);
    assert!(fixture.card_focused(),
            "the card takes focus when it appears");

    fixture.press("down enter space");
    let question = fixture.question().expect("still pending");
    assert_eq!(question.value(0), Some(&FieldValue::Single(Some(1))));
    assert_eq!(question.value(2),
               Some(&FieldValue::Multi(vec![true, false, false])));

    fixture.press("cmd-enter");
    assert!(fixture.question().is_none(), "submitted");
    assert!(fixture.composer_focused(), "focus returns to the composer");
}

/// Space and letters typed into an Other field are text: the card's Space
/// (pick or toggle) is for the card, not for its inputs.
#[gpui_kit::test]
fn typing_in_an_other_field_stays_in_the_field(cx: &mut TestAppContext) {
    let mut fixture = window_with_question(cx);
    fixture.press("tab");
    assert!(!fixture.card_focused(),
            "Tab moved the keyboard into the Other input");

    fixture.press("t space b");
    let question = fixture.question().expect("still pending");
    assert_eq!(question.value(1), Some(&FieldValue::Text("t b".to_owned())));
    assert_eq!(question.value(0),
               Some(&FieldValue::Single(None)),
               "nothing was picked");
}

/// One Escape leaves a field with its text kept; a second one skips.
#[gpui_kit::test]
fn escape_leaves_a_field_first_and_skips_second(cx: &mut TestAppContext) {
    let mut fixture = window_with_question(cx);
    fixture.press("tab t");
    fixture.press("escape");
    assert!(fixture.card_focused(), "back on the card");
    assert_eq!(fixture.question().expect("still pending").value(1),
               Some(&FieldValue::Text("t".to_owned())));

    fixture.press("escape");
    assert!(fixture.question().is_none(), "skipped");
}

/// "Permission shortcuts do not answer a question": Allow and Deny find no
/// permission on screen and leave the card alone.
#[gpui_kit::test]
fn permission_shortcuts_leave_a_question_alone(cx: &mut TestAppContext) {
    let mut fixture = window_with_question(cx);
    fixture.press("cmd-shift-a");
    fixture.press("cmd-shift-d");
    assert!(fixture.question().is_some(), "the card stays");
}
