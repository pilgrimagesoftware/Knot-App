//! A queued inbox nudge leaves the queue once its agent has read its inbox
//! (#24), so the row above the composer does not keep saying "check your
//! inbox" for messages already picked up.

use gpui_kit::TestAppContext;

use super::prompt_queue::{PromptOrigin, QueuedPanelPrompt};
use super::shortcuts_tests::window_with_agents;

fn nudge_behind_a_prompt() -> Vec<QueuedPanelPrompt> {
    vec![QueuedPanelPrompt::new("mine".to_owned(), PromptOrigin::User),
         QueuedPanelPrompt::new("check".to_owned(), PromptOrigin::InboxNudge),]
}

#[gpui_kit::test]
fn a_read_inbox_drops_the_waiting_nudge_and_keeps_the_rest(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.panel_prompt_queues.insert(id, nudge_behind_a_prompt());

                    assert!(view.drop_read_nudges(), "a dropped row needs a repaint");
                    let left = &view.panel_prompt_queues[&id];
                    assert_eq!(left.len(), 1);
                    assert_eq!(left[0].origin, PromptOrigin::User);
                    assert!(!view.drop_read_nudges(), "nothing more to drop");
                });
}

#[gpui_kit::test]
fn an_unread_message_keeps_its_nudge(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (id, sender) = (fixture.agents[0], fixture.agents[1]);

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.messages
                        .lock()
                        .add(knot_messaging::Message::new(sender, id, "still unread"));
                    view.panel_prompt_queues.insert(id, nudge_behind_a_prompt());

                    assert!(!view.drop_read_nudges());
                    assert_eq!(view.panel_prompt_queues[&id].len(), 2);
                });
}
