//! What a restart into a new conversation carries over, and when Knot's
//! own reset waits (#405).
//!
//! The fixture's agents launch nothing, so the handoff note is read where
//! the next connect would take it, `panel_handoffs`, not off a live turn.

use gpui_kit::TestAppContext;
use knot_messaging::Message;
use uuid::Uuid;

use super::shortcuts_tests::{Fixture, window_with_agents};

/// `sender` briefs `agent`, and `agent` reads it.
fn read_brief(fixture: &mut Fixture, sender: Uuid, agent: Uuid) {
    fixture.view.update(&mut fixture.window, |view, _| {
                    let mut messages = view.messages.lock();
                    messages.add(Message::new(sender, agent, "the brief"));
                    messages.mark_read(agent);
                });
}

#[gpui_kit::test]
fn a_new_conversation_gets_the_brief_back_and_a_note(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (sender, agent) = (fixture.agents[0], fixture.agents[1]);
    read_brief(&mut fixture, sender, agent);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store
                        .lock()
                        .set_status_text(agent, "Fixing #405".into());
                    view.restart_agents(&[agent], false, cx);

                    assert_eq!(view.messages.lock().unread_count(agent),
                               1,
                               "the brief is back in the inbox, whole");
                    let note = view.panel_handoffs
                                   .get(&agent)
                                   .expect("the next connect has a note to send");
                    assert!(note.contains("Fixing #405"));
                    assert!(note.contains("check-messages"));
                });
}

#[gpui_kit::test]
fn keeping_the_conversation_hands_nothing_off(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (sender, agent) = (fixture.agents[0], fixture.agents[1]);
    read_brief(&mut fixture, sender, agent);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.restart_agents(&[agent], true, cx);

                    assert!(!view.panel_handoffs.contains_key(&agent));
                    assert_eq!(view.messages.lock().unread_count(agent),
                               0,
                               "the conversation that read it is still there");
                });
}

#[gpui_kit::test]
fn an_answered_brief_stays_read(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (sender, agent) = (fixture.agents[0], fixture.agents[1]);
    read_brief(&mut fixture, sender, agent);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.messages
                        .lock()
                        .add(Message::new(agent, sender, "done"));
                    view.restart_agents(&[agent], false, cx);

                    assert_eq!(view.messages.lock().unread_count(agent), 0);
                    assert!(!view.panel_handoffs.contains_key(&agent),
                            "no status and nothing owed: nothing to hand off");
                });
}

/// Item 1: Knot's own reset - a plan step's fresh context - waits while the
/// agent owes a reply, and goes ahead once it has answered.
#[gpui_kit::test]
fn a_fresh_context_waits_for_the_reply(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (sender, agent) = (fixture.agents[0], fixture.agents[1]);
    read_brief(&mut fixture, sender, agent);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store.lock().request_fresh_context(agent);
                    assert!(!view.restart_for_fresh_context(cx),
                            "a brief read and not answered holds the reset");

                    view.messages
                        .lock()
                        .add(Message::new(agent, sender, "done"));
                    assert!(view.restart_for_fresh_context(cx),
                            "answered, so the reset goes ahead");
                });
}

/// #405 review: a dispatched task's goal, finished with `complete-task`
/// and no message back, must not hold a later fresh-context restart.
#[gpui_kit::test]
fn a_completed_tasks_goal_does_not_hold_a_fresh_context(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (lead, agent) = (fixture.agents[0], fixture.agents[1]);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    {
                        let mut messages = view.messages.lock();
                        let goal = Message::new(lead, agent, "the task's goal");
                        let goal_id = goal.id;
                        messages.add(goal);
                        messages.mark_task(goal_id);
                        messages.mark_read(agent);
                    }
                    view.store.lock().request_fresh_context(agent);
                    assert!(view.restart_for_fresh_context(cx));
                });
}

/// A brief from an agent that has since been removed can never be answered,
/// so it holds nothing either.
#[gpui_kit::test]
fn a_brief_from_a_removed_agent_does_not_hold_a_fresh_context(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let agent = fixture.agents[1];
    let gone = Uuid::new_v4();
    read_brief(&mut fixture, gone, agent);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store.lock().request_fresh_context(agent);
                    assert!(view.restart_for_fresh_context(cx));
                });
}
