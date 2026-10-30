//! "Send prompt to" delivers to the chosen agent alone, queues behind a
//! session that cannot take it yet, activates a chosen agent that is not
//! running (#208), and leaves the Changes view where it is
//! (`work-item-prompts`, #504).
//!
//! The chosen agent's panel session is still connecting, which is the case
//! `deliver_panel_prompt` queues - the same path a busy agent's prompt
//! takes - so the assertion reads the queue rather than a live adapter.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;

use super::WorkspaceViewMode;
use super::changes_tab::ChangesTab;
use super::render::send_prompt_menu::WorkItemRef;
use super::shortcuts_tests::window_with_agents;
use crate::panel_session::PanelSessionSlot;
use crate::workspace_window::prompt_queue::PromptOrigin;

#[gpui_kit::test]
fn the_chosen_agent_alone_queues_the_prompt_and_the_view_stays(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (chosen, other) = (fixture.agents[0], fixture.agents[1]);
    let url = "https://github.com/acme/widget/issues/42";

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.view_mode = WorkspaceViewMode::Changes;
                    view.changes_view.tab = ChangesTab::Issues;
                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(chosen, Arc::new(Mutex::new(connecting)));

                    assert!(view.can_receive_prompt(chosen));
                    assert!(view.send_work_item(chosen, &WorkItemRef::Issue(url.to_owned()), cx));

                    let queue = &view.panel_prompt_queues[&chosen];
                    assert_eq!(queue.len(), 1);
                    assert_eq!(queue[0].text,
                               knot_core::l10n::t_with("changes_view.prompt.issue",
                                                       &[("url", url)]));
                    assert_eq!(queue[0].origin, PromptOrigin::User);
                    assert!(view.panel_prompt_queues
                                .get(&other)
                                .is_none_or(Vec::is_empty),
                            "no other agent receives anything");
                    assert_eq!((view.view_mode, view.changes_view.tab),
                               (WorkspaceViewMode::Changes, ChangesTab::Issues),
                               "sending leaves the view where it was");
                });
}

/// An agent that has not started has no session to take a prompt; the
/// menu still lists it, and choosing it activates it (#208). The fixture's
/// agent type launches nothing, so activation is read off the store rather
/// than off a session - which also keeps a live process out of the test.
#[gpui_kit::test]
fn choosing_an_inactive_agent_activates_it(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (chosen, other) = (fixture.agents[0], fixture.agents[1]);
    let activated = |view: &super::WorkspaceWindow, id| {
        view.store
            .lock()
            .agent(id)
            .is_some_and(|agent| agent.activated)
    };

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.view_mode = WorkspaceViewMode::Changes;
                    assert!(!view.can_receive_prompt(chosen),
                            "the fixture's agent has started a session, so this proves nothing");
                    assert!(!activated(view, chosen));
                    let targets = view.prompt_targets();
                    assert_eq!(targets.len(), 2);
                    assert!(targets.iter().all(|target| !target.running));

                    view.send_work_item(chosen, &WorkItemRef::Change("add-login".to_owned()), cx);

                    assert!(activated(view, chosen), "the chosen agent is activated");
                    assert!(!activated(view, other), "no other agent is");
                    assert_eq!(view.view_mode,
                               WorkspaceViewMode::Changes,
                               "activating does not leave the Changes view");
                });
}

/// #310: the menu reads each agent's lock from the store and marks it, and
/// choosing a locked agent still sends - the user prompting an agent is not
/// agent messaging, which is all a lock shuts out.
#[gpui_kit::test]
fn a_locked_agent_is_marked_and_still_receives_the_prompt(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (locked, open) = (fixture.agents[0], fixture.agents[1]);
    let url = "https://github.com/acme/widget/issues/7";

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.store.lock().set_locked(locked, true);
                    let targets = view.prompt_targets();
                    let marked = |id| {
                        targets.iter()
                               .find(|target| target.id == id)
                               .map(|target| target.locked)
                    };
                    assert_eq!(marked(locked), Some(true));
                    assert_eq!(marked(open), Some(false));

                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(locked, Arc::new(Mutex::new(connecting)));
                    assert!(view.send_work_item(locked, &WorkItemRef::Issue(url.to_owned()), cx),
                            "a locked agent still takes the user's prompt");
                    assert_eq!(view.panel_prompt_queues[&locked].len(), 1);
                });
}
