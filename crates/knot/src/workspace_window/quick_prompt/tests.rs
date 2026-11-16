//! Which agents have a session a prompt can reach, as both dashboards read
//! it (#10).

use std::sync::Arc;

use gpui_kit::{AppContext, TestAppContext};
use parking_lot::Mutex;

use crate::command_center::CommandCenterWindow;
use crate::panel_session::PanelSessionSlot;
use crate::workspace_window::shortcuts_tests::window_with_agents;

#[gpui_kit::test]
fn a_connecting_session_counts_and_a_failed_one_does_not(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    assert!(!view.has_session(agent), "no slot, no session");

                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(agent, Arc::new(Mutex::new(connecting)));
                    assert!(view.has_session(agent),
                            "a prompt queues into a connecting slot");

                    view.panel_sessions
                        .insert(agent,
                                Arc::new(Mutex::new(PanelSessionSlot::Failed("refused".into()))));
                    assert!(!view.has_session(agent),
                            "nothing drains a failed slot's queue");
                    assert!(!view.quick_prompt(agent, "hello"), "so nothing is taken");
                });
}

/// The Command Center reads sessions through the agent's open window. With
/// the window open it sees the slot; an agent of a workspace with no open
/// window has none, though `activated` is still set.
#[gpui_kit::test]
fn the_command_center_reads_sessions_from_the_open_window(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    let store = fixture.view
                       .read_with(&fixture.window, |view, _| Arc::clone(&view.store));
    let elsewhere = {
        let mut store = store.lock();
        let space = crate::tests::workspace("Closed");
        let workspace_id = space.id;
        store.add_workspace(space);
        let id = store.create("~/closed",
                              knot_agents::CreateOptions { workspace_id: Some(workspace_id),
                                                           ..Default::default() });
        store.set_activated(id, true);
        id
    };
    let center = cx.new(|_| CommandCenterWindow::for_tests(Arc::clone(&store)));

    let before = cx.update(|cx| center.update(cx, |view, cx| view.sessions(cx)));
    assert_eq!(before.get(&agent), Some(&false), "window open, no slot yet");
    assert_eq!(before.get(&elsewhere),
               Some(&false),
               "no open window, though activated");

    fixture.view.update(&mut fixture.window, |view, _| {
                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(agent, Arc::new(Mutex::new(connecting)));
                });
    let after = cx.update(|cx| center.update(cx, |view, cx| view.sessions(cx)));
    assert_eq!(after.get(&agent), Some(&true), "window open with a slot");
}

/// A prompt to an agent that cannot take it yet - here, still connecting -
/// waits in its queue as the user's, as a composer prompt would.
#[gpui_kit::test]
fn a_quick_prompt_queues_as_the_users(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(agent, Arc::new(Mutex::new(connecting)));

                    assert!(view.quick_prompt(agent, "check the tests"));
                    let queue = &view.panel_prompt_queues[&agent];
                    assert_eq!(queue.len(), 1);
                    assert_eq!(queue[0].text, "check the tests");
                    assert_eq!(queue[0].origin,
                               crate::workspace_window::prompt_queue::PromptOrigin::User);
                });
}

/// The Command Center routes a card's prompt to the window that owns the
/// agent's session; with no such session, nothing takes it.
#[gpui_kit::test]
fn the_command_center_routes_to_the_owning_window(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    let store = fixture.view
                       .read_with(&fixture.window, |view, _| Arc::clone(&view.store));
    let center = cx.new(|_| CommandCenterWindow::for_tests(Arc::clone(&store)));

    assert!(!cx.update(|cx| center.update(cx, |view, cx| view.route_quick_prompt(agent, "hi",
                                                                                 cx))),
            "no session yet: the text is not taken");

    fixture.view.update(&mut fixture.window, |view, _| {
                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(agent, Arc::new(Mutex::new(connecting)));
                });
    assert!(cx.update(|cx| center.update(cx, |view, cx| view.route_quick_prompt(agent, "hi", cx))));
    let queued = fixture.view.read_with(&fixture.window, |view, _| {
                                 view.panel_prompt_queues[&agent].iter()
                                                                 .map(|p| p.text.clone())
                                                                 .collect::<Vec<_>>()
                             });
    assert_eq!(queued, vec!["hi"], "it reached the owning window's queue");
}
