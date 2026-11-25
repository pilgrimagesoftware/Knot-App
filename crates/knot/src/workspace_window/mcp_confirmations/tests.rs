//! The confirmation dialog resolves the request it shows (`mcp-auth` task
//! 5.2): Allow answers yes, Deny or dismissing answers no, and the window
//! leaves another workspace's requests queued.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::TestAppContext;
use gpui_kit::component::WindowExt;
use knot_mcp_tools::{ConfirmationQueue, ConfirmationRequest, PendingConfirmation};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::workspace_window::shortcuts_tests::{Fixture, window_with_agents};

fn request(requester: Uuid) -> ConfirmationRequest {
    ConfirmationRequest { requester,
                          requester_name: "Builder".to_owned(),
                          command: Some("make test".to_owned()),
                          folder: PathBuf::from("/tmp/project"),
                          distant_worktree: false }
}

/// A window over one agent with `queue` installed, and a request from that
/// agent queued on it.
fn queued(cx: &mut TestAppContext) -> (Fixture, ConfirmationQueue, oneshot::Receiver<bool>) {
    let fixture = window_with_agents(1, cx);
    let queue = ConfirmationQueue::default();
    cx.update(|cx| cx.set_global(crate::app_support::Confirmations(Arc::clone(&queue))));
    let (pending, answered) = PendingConfirmation::new(request(fixture.agents[0]));
    queue.lock().push(pending);
    (fixture, queue, answered)
}

fn shown(fixture: &mut Fixture) -> bool {
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.show_mcp_confirmations(cx)
                });
    fixture.window.run_until_parked();
    fixture.window
           .update(|window, cx| window.has_active_dialog(cx))
}

#[gpui_kit::test]
fn allowing_answers_yes(cx: &mut TestAppContext) {
    let (mut fixture, queue, mut answered) = queued(cx);
    assert!(shown(&mut fixture), "the dialog opens");
    assert!(queue.lock().is_empty(),
            "the window claimed its agent's request");

    fixture.window.simulate_keystrokes("enter");
    fixture.window.run_until_parked();

    assert_eq!(answered.try_recv(), Ok(true));
}

#[gpui_kit::test]
fn dismissing_answers_no(cx: &mut TestAppContext) {
    let (mut fixture, _queue, mut answered) = queued(cx);
    assert!(shown(&mut fixture));

    fixture.window.simulate_keystrokes("escape");
    fixture.window.run_until_parked();

    assert!(!fixture.window
                    .update(|window, cx| window.has_active_dialog(cx)));
    assert_ne!(answered.try_recv(), Ok(true), "a dismissal is never a yes");
}

#[gpui_kit::test]
fn another_workspaces_request_stays_queued(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let queue = ConfirmationQueue::default();
    cx.update(|cx| cx.set_global(crate::app_support::Confirmations(Arc::clone(&queue))));
    let elsewhere = fixture.view.update(&mut fixture.window, |view, _| {
                                    let mut store = view.store.lock();
                                    let space = crate::tests::workspace("Elsewhere");
                                    let workspace_id = space.id;
                                    store.add_workspace(space);
                                    store.create("/tmp/elsewhere",
                                            knot_agents::CreateOptions { workspace_id:
                                                                             Some(workspace_id),
                                                                         ..Default::default() })
                                });
    let (pending, _answered) = PendingConfirmation::new(request(elsewhere));
    queue.lock().push(pending);

    assert!(!shown(&mut fixture));
    assert_eq!(queue.lock().len(),
               1,
               "left for the window that owns the agent");
}

#[test]
fn the_body_names_the_agent_the_command_and_the_folder() {
    let mut request = request(Uuid::new_v4());
    let body = super::body(&request);
    for part in ["Builder", "make test", "/tmp/project"] {
        assert!(body.contains(part), "{part} missing: {body}");
    }
    assert!(!body.contains("mcp_confirm."), "the keys resolve: {body}");

    request.command = None;
    request.distant_worktree = true;
    let body = super::body(&request);
    assert!(body.contains("Builder") && body.contains("/tmp/project"),
            "{body}");
    assert!(!body.contains("mcp_confirm."), "{body}");
}
