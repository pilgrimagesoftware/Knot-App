//! The "Knot tools unavailable" notice follows the server's refusals for the
//! agent (`mcp-auth` task 6.2): a refused request puts it over the agent's
//! pane, a later authenticated one takes it away.

use gpui_kit::TestAppContext;
use knot_mcp::{RefusalKind, RefusalSource};

use crate::workspace_window::shortcuts_tests::{Fixture, window_with_agents};

fn banner_shown(fixture: &mut Fixture) -> bool {
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.selected_agent = Some(agent);
                    view.poll_mcp_unavailable();
                    cx.notify();
                });
    fixture.window.run_until_parked();
    fixture.window.debug_bounds("mcp-refused-banner").is_some()
}

#[gpui_kit::test]
fn a_refused_request_shows_the_notice_until_the_agent_authenticates(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    let refusals = fixture.view.update(&mut fixture.window, |view, _| {
                                   view.mcp_credentials.issue(agent);
                                   view.mcp_credentials.refusals.clone()
                               });
    assert!(!banner_shown(&mut fixture));

    refusals.record(RefusalSource::Agent(agent), RefusalKind::Stream);
    assert!(!banner_shown(&mut fixture),
            "a refused stream alone is not a lost tool");

    refusals.record(RefusalSource::Agent(agent), RefusalKind::Request);
    assert!(banner_shown(&mut fixture));
    let changed_again = fixture.view
                               .update(&mut fixture.window, |view, _| view.poll_mcp_unavailable());
    assert!(!changed_again, "an unchanged set asks for no repaint");

    refusals.authenticated(agent);
    assert!(!banner_shown(&mut fixture));
}

#[test]
fn the_notice_key_resolves() {
    assert_ne!(knot_core::l10n::t("panel.mcp_refused"), "panel.mcp_refused");
}
