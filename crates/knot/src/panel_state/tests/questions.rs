//! Question cards in the panel state (#430): what a card holds and sends,
//! where its keys move, and how a question shares the pending-input queue
//! with permission requests.

use knot_acp::{
    AnswerValue, Choice, FieldKind, FormAnswer, FormField, PermissionRequest, QuestionRequest,
    SessionEvent, SessionUpdate, StopReason,
};
use serde_json::json;

use crate::panel_state::{CardKey, FieldValue, PanelState, PendingInput, QuestionState, Stop};

fn choices(values: &[&str]) -> Vec<Choice> {
    values.iter()
          .map(|value| Choice { value:       (*value).to_owned(),
                                title:       (*value).to_owned(),
                                description: None, })
          .collect()
}

fn field(key: &str, description: &str, kind: FieldKind) -> FormField {
    FormField { key: key.to_owned(),
                title: None,
                description: Some(description.to_owned()),
                required: false,
                kind }
}

fn text() -> FieldKind {
    FieldKind::Text { default:    None,
                      min_length: None,
                      max_length: None, }
}

/// The AskUserQuestion shape `claude-agent-acp` sends: a single and a
/// multiple choice question, each followed by its "Other" text field.
fn color_and_fruits(rpc_id: i64) -> QuestionRequest {
    QuestionRequest { rpc_id:       json!(rpc_id),
                      session_id:   "sess-1".to_owned(),
                      tool_call_id: Some("toolu_1".to_owned()),
                      message:      "Please answer the following questions.".to_owned(),
                      fields:       vec![field("question_0",
                                               "Which color?",
                                               FieldKind::Single { choices: choices(&["Red",
                                                                                      "Green",
                                                                                      "Blue"]),
                                                                   default: None, }),
                                         field("question_0_custom", "Other", text()),
                                         field("question_1",
                                               "Which fruits?",
                                               FieldKind::Multi { choices:   choices(&["Apple",
                                                                                       "Pear",
                                                                                       "Plum"]),
                                                                  default:   Vec::new(),
                                                                  min_items: None,
                                                                  max_items: None, }),
                                         field("question_1_custom", "Other", text())], }
}

fn permission(rpc_id: i64) -> PermissionRequest {
    PermissionRequest { rpc_id:          json!(rpc_id),
                        tool_call_id:    "tc1".to_owned(),
                        tool_call_title: None,
                        options:         Vec::new(), }
}

fn submitted(question: &QuestionState) -> Vec<(String, AnswerValue)> {
    match question.submitted() {
        FormAnswer::Submitted(fields) => fields,
        other => panic!("expected a submission, got {other:?}"),
    }
}

/// `acp-panel-ui`'s "Answering from the keyboard": Down, Return, Space and
/// the answers that Submit then sends.
#[test]
fn down_return_space_answers_both_questions() {
    let mut question = QuestionState::new(color_and_fruits(1));
    assert_eq!(question.focus(),
               Some(Stop { field:  0,
                           choice: Some(0), }),
               "focus starts on the first choice of the first field");

    question.key(CardKey::Down);
    question.key(CardKey::Return);
    assert_eq!(question.focus(),
               Some(Stop { field:  2,
                           choice: Some(0), }),
               "Return moves past the first question's Other field");
    question.key(CardKey::Space);

    assert_eq!(submitted(&question),
               vec![("question_0".to_owned(), AnswerValue::Text("Green".to_owned())),
                    ("question_1".to_owned(), AnswerValue::Choices(vec!["Apple".to_owned()]))]);
}

#[test]
fn other_is_paired_with_its_question() {
    let question = QuestionState::new(color_and_fruits(1));
    assert_eq!(question.other_of(0), Some(1));
    assert_eq!(question.other_of(2), Some(3));
    assert!(question.is_other(1) && question.is_other(3));
    assert!(!question.is_other(0) && !question.is_other(2));
}

/// "Answering with Other": no pick, a typed answer, sent as that question's
/// free text.
#[test]
fn a_typed_other_answer_is_sent_without_a_pick() {
    let mut question = QuestionState::new(color_and_fruits(1));
    question.set_text(1, "Teal".to_owned());
    assert_eq!(submitted(&question),
               vec![("question_0_custom".to_owned(), AnswerValue::Text("Teal".to_owned()))]);
}

/// #465: one AskUserQuestion question, three options, nothing picked. The
/// adapter marks no field required, so the schema alone let Submit send an
/// empty `accept`, which the model reads as "The user did not answer the
/// questions". Submit waits for a pick or "Other" text; Skip still declines.
#[test]
fn submit_waits_for_each_question_to_be_answered() {
    let mut request = color_and_fruits(1);
    request.fields.truncate(2);
    let mut question = QuestionState::new(request);
    assert!(!question.can_submit(), "nothing picked and no Other text");

    question.set_text(1, "   ".to_owned());
    assert!(!question.can_submit(), "blank Other text is no answer");

    question.set_text(1, "Teal".to_owned());
    assert!(question.can_submit(), "Other text answers the question");

    question.set_text(1, String::new());
    question.pick(0, 0);
    assert!(question.can_submit(), "a pick answers the question");
    assert_eq!(submitted(&question),
               vec![("question_0".to_owned(), AnswerValue::Text("Red".to_owned()))]);
}

/// With two questions, answering one is not enough: the other would reach
/// the model as unanswered.
#[test]
fn submit_waits_for_every_question() {
    let mut question = QuestionState::new(color_and_fruits(1));
    question.pick(0, 1);
    assert!(!question.can_submit(), "the fruits question is unanswered");
    question.toggle(2, 0);
    assert!(question.can_submit());
}

#[test]
fn space_toggles_a_multiple_choice_off_again() {
    let mut question = QuestionState::new(color_and_fruits(1));
    question.focus_stop(Stop { field:  2,
                               choice: Some(1), });
    question.key(CardKey::Space);
    question.key(CardKey::Space);
    assert_eq!(question.value(2),
               Some(&FieldValue::Multi(vec![false, false, false])));
}

/// In a text field there are no choices, so Up and Down move between
/// fields, like Shift-Tab and Tab.
#[test]
fn up_and_down_in_a_text_field_move_between_fields() {
    let mut question = QuestionState::new(color_and_fruits(1));
    question.focus_stop(Stop { field:  1,
                               choice: None, });
    question.key(CardKey::Down);
    assert_eq!(question.focus(),
               Some(Stop { field:  2,
                           choice: Some(0), }));
    question.focus_stop(Stop { field:  1,
                               choice: None, });
    question.key(CardKey::Up);
    assert_eq!(question.focus(),
               Some(Stop { field:  0,
                           choice: Some(0), }));
}

#[test]
fn tab_and_shift_tab_move_by_field() {
    let mut question = QuestionState::new(color_and_fruits(1));
    question.key(CardKey::Down);
    question.key(CardKey::Tab);
    assert_eq!(question.focus(),
               Some(Stop { field:  1,
                           choice: None, }));
    question.key(CardKey::BackTab);
    assert_eq!(question.focus(),
               Some(Stop { field:  0,
                           choice: Some(0), }));
}

/// "A required field blocks Submit", and the bounds the Submit gate reads
/// from the schema.
#[test]
fn submit_waits_for_required_fields_and_valid_numbers() {
    let mut name = field("name", "Name", text());
    name.required = true;
    let port = field("port",
                     "Port",
                     FieldKind::Number { integer: true,
                                         default: None,
                                         minimum: Some(1.0),
                                         maximum: Some(65535.0), });
    let mut question = QuestionState::new(QuestionRequest { rpc_id:       json!(1),
                                                            session_id:   "s".to_owned(),
                                                            tool_call_id: None,
                                                            message:      "Configure".to_owned(),
                                                            fields:       vec![name, port], });
    assert!(!question.can_submit(), "the required name is empty");
    question.set_text(0, "knot".to_owned());
    assert!(question.can_submit());
    question.set_text(1, "eighty".to_owned());
    assert!(!question.can_submit(), "not a number");
    question.set_text(1, "70000".to_owned());
    assert!(!question.can_submit(), "above the maximum");
    question.set_text(1, "8080".to_owned());
    assert!(question.can_submit());
    assert_eq!(submitted(&question),
               vec![("name".to_owned(), AnswerValue::Text("knot".to_owned())),
                    ("port".to_owned(), AnswerValue::Number(8080.into()))]);
}

#[test]
fn schema_defaults_are_filled_in() {
    let request = QuestionRequest { rpc_id:       json!(1),
                                    session_id:   "s".to_owned(),
                                    tool_call_id: None,
                                    message:      String::new(),
                                    fields:
                                        vec![field("level",
                                                   "Level",
                                                   FieldKind::Single { choices: choices(&["lo",
                                                                                          "hi"]),
                                                                       default:
                                                                           Some("hi".to_owned()), }),
                                             field("verbose",
                                                   "Verbose",
                                                   FieldKind::Boolean { default: Some(true), })], };
    let mut question = QuestionState::new(request);
    assert_eq!(question.value(0), Some(&FieldValue::Single(Some(1))));
    question.focus_stop(Stop { field:  1,
                               choice: None, });
    question.key(CardKey::Space);
    assert_eq!(question.value(1), Some(&FieldValue::Boolean(Some(false))));
}

/// "A permission request arriving behind a question waits", and the other
/// way round: one queue, arrival order, either kind.
#[test]
fn questions_and_permissions_share_one_queue_in_arrival_order() {
    let mut state = PanelState::new();
    state.apply(SessionEvent::Question(color_and_fruits(200)));
    state.apply(SessionEvent::PermissionRequest(permission(100)));
    assert!(state.pending_question().is_some());
    assert_eq!(state.pending_permission(),
               None,
               "the permission waits behind the question");
    assert!(state.has_pending_input());

    state.resolve_input();
    assert_eq!(state.pending_permission(), Some(&permission(100)));
    state.resolve_input();
    assert!(!state.has_pending_input());
}

/// Withdrawn by the agent, or cancelled by Stop: the client reports it
/// resolved and the card goes, whichever position it held.
#[test]
fn a_resolved_question_leaves_the_queue_by_id() {
    let mut state = PanelState::new();
    state.apply(SessionEvent::PermissionRequest(permission(100)));
    state.apply(SessionEvent::Question(color_and_fruits(200)));
    state.apply(SessionEvent::InputResolved { rpc_id: json!(200) });
    assert_eq!(state.queued_inputs, Vec::<PendingInput>::new());
    assert_eq!(state.pending_permission(), Some(&permission(100)));

    state.apply(SessionEvent::InputResolved { rpc_id: json!(100) });
    state.apply(SessionEvent::Question(color_and_fruits(201)));
    state.apply(SessionEvent::InputResolved { rpc_id: json!(201) });
    assert!(!state.has_pending_input());
}

#[test]
fn a_session_ending_clears_a_pending_question() {
    let mut state = PanelState::new();
    state.apply(SessionEvent::Question(color_and_fruits(200)));
    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: StopReason::EndTurn, }));
    state.apply(SessionEvent::Ended(knot_acp::SessionEndCause::ProcessExited { code: Some(0) }));
    assert!(!state.has_pending_input());
}
