//! The workspace dashboard's card fields: given to panel agents, pruned
//! with the roster, cleared only once a prompt is taken.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::dashboard::DashboardAgent;
use crate::panel_session::PanelSessionSlot;
use crate::workspace_window::shortcuts_tests::window_with_agents;

fn card(id: Uuid, has_session: bool) -> DashboardAgent {
    DashboardAgent { id,
                     avatar: String::new(),
                     name: String::new(),
                     folder_name: String::new(),
                     state: knot_agents::AgentState::Idle,
                     is_shell: false,
                     header_title: String::new(),
                     input_reason: None,
                     session_limit: None,
                     git_stats: None,
                     has_session,
                     quick_prompt: None }
}

#[gpui_kit::test]
fn fields_follow_the_roster_and_clear_once_taken(cx: &mut TestAppContext) {
    let fixture = window_with_agents(2, cx);
    let (first, second) = (fixture.agents[0], fixture.agents[1]);
    let mut window = fixture.window;
    let view = fixture.view;

    let input = window.update(|window, cx| {
                          view.update(cx, |view, cx| {
                                  let mut cards = vec![card(first, false), card(second, false)];
                                  view.attach_quick_prompts(&mut cards, window, cx);
                                  assert!(cards.iter().all(|card| card.quick_prompt.is_some()));

                                  let mut cards = vec![card(first, false)];
                                  view.attach_quick_prompts(&mut cards, window, cx);
                                  assert!(view.dashboard_quick_prompts.get(second).is_none(),
                                          "a card that left takes its field with it");
                                  cards[0].quick_prompt.clone().expect("a field")
                              })
                      });
    window.update(|window, cx| input.update(cx, |field, cx| field.set_value("hi", window, cx)));

    // No session: nothing takes it, and the text stays.
    window.update(|window, cx| {
              view.update(cx, |view, cx| {
                      view.send_dashboard_prompt(first, false, window, cx)
                  })
          });
    assert_eq!(input.read_with(&window, |field, _| field.value().to_string()),
               "hi");

    view.update(&mut window, |view, _| {
            let (connecting, _progress) = PanelSessionSlot::connecting();
            view.panel_sessions
                .insert(first, Arc::new(Mutex::new(connecting)));
        });
    window.update(|window, cx| {
              view.update(cx, |view, cx| {
                      view.send_dashboard_prompt(first, false, window, cx)
                  })
          });
    assert_eq!(input.read_with(&window, |field, _| field.value().to_string()),
               "",
               "taken, so the field is cleared");
}
