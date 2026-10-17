//! The checklist section against a real workspace window (#301): that it
//! opens the panel on its own, is drawn with a field per question, keeps the
//! user's answers through a close and reopen, and is marked sent by Send.

use gpui_kit::TestAppContext;
use knot_agents::{ChecklistItem, ChecklistItemKind};

use crate::keymap::SelectAgent1;
use crate::workspace_window::artifact_panel::closed::CloseTarget;
use crate::workspace_window::shortcuts_tests::{Fixture, window_with_agents};

fn show_checklist(fixture: &mut Fixture) -> uuid::Uuid {
    let id = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store
                        .lock()
                        .set_checklist(id,
                                       Some("Release".to_owned()),
                                       vec![ChecklistItem::new("tests",
                                                               "Tests pass",
                                                               ChecklistItemKind::Check),
                                            ChecklistItem::new("name",
                                                               "What should it be called?",
                                                               ChecklistItemKind::Question)])
                        .expect("the agent exists");
                    cx.notify();
                });
    id
}

#[gpui_kit::test]
fn a_checklist_alone_opens_the_panel_and_draws_a_field_per_question(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(SelectAgent1);
    let id = show_checklist(&mut fixture);
    fixture.window.run_until_parked();

    fixture.view.read_with(&fixture.window, |view, _| {
                    assert!(view.artifact_panel_open(id), "a checklist is an artifact");
                    assert!(view.artifact_snapshot(id).checklist.is_some());
                    let fields = view.checklist_inputs
                                     .get(&id)
                                     .map(|inputs| inputs.fields.len());
                    assert_eq!(fields,
                               Some(1),
                               "one answer field, for the one question, drawn with the panel");
                });
}

#[gpui_kit::test]
fn closing_and_reopening_keeps_the_users_answers(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_checklist(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.store.lock().edit_checklist(id, |list| {
                                         list.set_checked("tests", true);
                                         list.set_answer("name", "Knot");
                                     });
                    view.close_artifact_sections(id, CloseTarget::All);
                    assert!(!view.artifact_panel_open(id));

                    view.reopen_artifacts(id);

                    let list = view.artifact_snapshot(id)
                                   .checklist
                                   .expect("the checklist came back");
                    assert!(list.item("tests").unwrap().checked);
                    assert_eq!(list.item("name").unwrap().answer, "Knot");
                });
}

/// Closing only the checklist leaves the other artifacts where they are.
#[gpui_kit::test]
fn closing_the_checklist_alone_leaves_a_diagram_open(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_checklist(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.store
                        .lock()
                        .set_mermaid_panel(id, "graph TD; A-->B;".to_owned(), None)
                        .expect("the agent exists");
                    view.close_artifact_sections(id, CloseTarget::Checklist);

                    let snapshot = view.artifact_snapshot(id);
                    assert!(snapshot.checklist.is_none());
                    assert!(snapshot.mermaid.is_some());
                    assert!(view.reopenable_artifacts(id).checklist.is_some());
                });
}

/// Send with nothing to deliver to - here a Panel-mode agent with no panel
/// session - must not claim the list was sent (#307): the header would read
/// Sent and get-checklist would tell the agent it has answers it never got.
#[gpui_kit::test]
fn send_with_no_session_does_not_mark_the_checklist_sent(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_checklist(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.send_checklist(id, cx);

                    let list = view.artifact_snapshot(id).checklist.unwrap();
                    assert!(!list.submitted, "nothing was delivered, so nothing is sent");
                });
}

/// Send to an agent with a session marks the list sent, and the agent gets
/// the summary - queued here, since the session is still connecting.
#[gpui_kit::test]
fn send_delivers_the_summary_and_marks_the_checklist_sent(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_checklist(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, cx| {
                    let (slot, _progress) = crate::panel_session::PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(id, std::sync::Arc::new(parking_lot::Mutex::new(slot)));

                    view.send_checklist(id, cx);

                    let list = view.artifact_snapshot(id).checklist.unwrap();
                    assert!(list.submitted);
                    let queued = view.panel_prompt_queues
                                     .get(&id)
                                     .and_then(|queue| queue.first())
                                     .map(|prompt| prompt.text.clone())
                                     .expect("the summary was queued for the agent");
                    assert!(queued.contains("What should it be called?"), "{queued}");
                });
}

/// #307's blocking case: the user answers, closes only the checklist (a
/// diagram keeps the panel open), and the agent shows the same items again.
/// The store's answer is reset, and the field must show that - not the text
/// typed into the list that was closed.
#[gpui_kit::test]
fn an_answer_field_never_shows_text_the_checklist_does_not_hold(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(SelectAgent1);
    let id = show_checklist(&mut fixture);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store
                        .lock()
                        .set_mermaid_panel(id, "graph TD; A-->B;".to_owned(), None)
                        .expect("the agent exists");
                    cx.notify();
                });
    fixture.window.run_until_parked();

    // The user types an answer: the field's value, and the store write its
    // `Change` makes.
    let field = fixture.view.read_with(&fixture.window, |view, _| {
                                view.checklist_inputs[&id].fields["name"].0.clone()
                            });
    fixture.window.update(|window, cx| {
                      field.update(cx, |state, cx| state.set_value("Knot", window, cx));
                  });
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store
                        .lock()
                        .edit_checklist(id, |list| list.set_answer("name", "Knot"));
                    view.close_artifact_sections(id, CloseTarget::Checklist);
                    cx.notify();
                });
    fixture.window.run_until_parked();

    show_checklist(&mut fixture);
    fixture.window.run_until_parked();

    fixture.view.read_with(&fixture.window, |view, cx| {
                    let stored =
                        view.artifact_snapshot(id)
                            .checklist
                            .and_then(|list| list.item("name").map(|item| item.answer.clone()))
                            .expect("the checklist is shown again");
                    let shown = view.checklist_inputs[&id].fields["name"].0
                                                                         .read(cx)
                                                                         .value()
                                                                         .to_string();
                    assert_eq!(stored, "", "a list shown after a close starts unanswered");
                    assert_eq!(shown, stored,
                               "the field shows what Send and get-checklist would read");
                });
}
