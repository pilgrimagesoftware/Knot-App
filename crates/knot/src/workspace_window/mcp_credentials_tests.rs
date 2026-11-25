//! An agent's MCP credentials follow its adapter (`mcp-auth` D3, D8): every
//! path that retires a panel session revokes the pair it ran on, and the
//! environment an adapter is given carries the hook credential alone.
//!
//! The fixture's agent type launches nothing, so the slot is planted
//! `Connecting` and the pair issued as `ensure_panel_session` issues it.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use knot_mcp::Principal;
use parking_lot::Mutex;

use super::shortcuts_tests::window_with_agents;
use crate::panel_session::PanelSessionSlot;

/// Plants a `Connecting` slot for `agent` and issues its pair, as a launch
/// does.
fn launched(view: &mut super::WorkspaceWindow, agent: uuid::Uuid) -> knot_mcp::AgentCredentials {
    view.store.lock().set_activated(agent, true);
    let (connecting, _progress) = PanelSessionSlot::connecting();
    view.panel_sessions
        .insert(agent, Arc::new(Mutex::new(connecting)));
    view.mcp_credentials.issue(agent)
}

#[gpui_kit::test]
fn a_restart_refuses_the_old_pair_and_the_relaunch_gets_a_new_one(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let old = launched(view, agent);
                    view.restart_agents(&[agent], false, cx);
                    let registry = &view.mcp_credentials.registry;
                    assert_eq!(registry.authenticate(old.tool.expose()), None);
                    assert_eq!(registry.authenticate(old.hooks.expose()), None);

                    let new = launched(view, agent);
                    assert_eq!(view.mcp_credentials
                                   .registry
                                   .authenticate(new.tool.expose()),
                               Some(Principal::Agent(agent)));
                    assert_eq!(view.mcp_credentials
                                   .registry
                                   .authenticate(old.tool.expose()),
                               None,
                               "the relaunch does not bring the old pair back");
                });
}

#[gpui_kit::test]
fn deactivating_or_removing_an_agent_revokes_its_pair(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (deactivated, removed) = (fixture.agents[0], fixture.agents[1]);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let first = launched(view, deactivated);
                    let second = launched(view, removed);
                    view.deactivate_agent(deactivated);
                    view.remove_agent(removed, cx);

                    let registry = &view.mcp_credentials.registry;
                    assert_eq!(registry.authenticate(first.tool.expose()), None);
                    assert_eq!(registry.authenticate(second.tool.expose()), None);
                });
}

#[gpui_kit::test]
fn a_window_without_the_agents_slot_leaves_its_pair_alone(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, _| {
                    // Issued by some other window's launch: this one holds no
                    // slot for the agent, so retiring here must not revoke it.
                    let pair = view.mcp_credentials.issue(agent);
                    view.retire_panel_session(agent);
                    assert_eq!(view.mcp_credentials
                                   .registry
                                   .authenticate(pair.tool.expose()),
                               Some(Principal::Agent(agent)));
                });
}

#[test]
fn the_adapter_environment_carries_only_the_hook_credential() {
    let agent = uuid::Uuid::new_v4();
    let pair = knot_mcp::CredentialRegistry::new().issue_agent_pair(agent);
    let env = super::panel::hook_environment(agent, &pair, 8767);

    let value = |name: &str| {
        env.iter()
           .find(|(key, _)| key == name)
           .map(|(_, value)| value.as_str())
    };
    assert_eq!(value(crate::consts::MCP_HOOK_TOKEN_ENV),
               Some(pair.hooks.expose()));
    assert_eq!(value(crate::consts::MCP_HOOK_URL_ENV),
               Some("http://127.0.0.1:8767"));
    assert_eq!(value(crate::consts::MCP_HOOK_AGENT_ENV),
               Some(agent.to_string().as_str()));
    assert!(env.iter()
               .all(|(_, value)| !value.contains(pair.tool.expose())),
            "the tool credential never goes in an environment");
}
