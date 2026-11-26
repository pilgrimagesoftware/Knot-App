//! The confirmation dialog resolves the request it shows (`mcp-auth` task
//! 5.2): only a click on Allow answers yes; Enter, Escape and dismissing
//! answer no; and the window leaves another workspace's requests queued.

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

/// Review of #522: the dialog can appear while the user is typing, so a
/// stray Enter must deny, never allow.
#[gpui_kit::test]
fn enter_denies(cx: &mut TestAppContext) {
    let (mut fixture, queue, mut answered) = queued(cx);
    assert!(shown(&mut fixture), "the dialog opens");
    assert!(queue.lock().is_empty(),
            "the window claimed its agent's request");

    fixture.window.simulate_keystrokes("enter");
    fixture.window.run_until_parked();

    assert_eq!(answered.try_recv(), Ok(false));
}

#[gpui_kit::test]
fn only_a_click_on_allow_answers_yes(cx: &mut TestAppContext) {
    let (mut fixture, _queue, mut answered) = queued(cx);
    assert!(shown(&mut fixture));
    let allow = fixture.window
                       .debug_bounds("mcp-confirm-allow")
                       .expect("the Allow button is drawn");

    fixture.window
           .simulate_click(allow.center(), gpui_kit::Modifiers::none());
    fixture.window.run_until_parked();

    assert_eq!(answered.try_recv(), Ok(true));
    assert!(!fixture.window
                    .update(|window, cx| window.has_active_dialog(cx)));
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

/// Security review of #516: a command cannot hide part of itself from the
/// person approving it - control and format characters are written out.
#[test]
fn control_and_format_characters_are_shown_not_obeyed() {
    let mut request = request(Uuid::new_v4());
    request.command = Some("echo safe\r\x1b[2Krm -rf ~\u{202e}txt.sh\u{7f}".to_owned());
    request.folder = PathBuf::from("/tmp/pro\u{200b}ject\u{0}");
    request.requester_name = "Build\u{2066}er".to_owned();

    let body = super::body(&request);

    for hidden in ['\r', '\u{1b}', '\u{202e}', '\u{7f}', '\u{200b}', '\u{0}', '\u{2066}'] {
        assert!(!body.contains(hidden),
                "{hidden:?} reached the dialog: {body:?}");
    }
    for shown in ["\\u{d}",
                  "\\u{1b}",
                  "\\u{202e}",
                  "\\u{7f}",
                  "\\u{200b}",
                  "\\u{0}",
                  "\\u{2066}"]
    {
        assert!(body.contains(shown), "{shown} missing: {body}");
    }
    assert!(body.contains("rm -rf ~"),
            "the visible text is kept: {body}");
    // The template's own line breaks are the dialog's, not the command's.
    assert_eq!(super::visible("make\ntest"), "make\\u{a}test");
}
