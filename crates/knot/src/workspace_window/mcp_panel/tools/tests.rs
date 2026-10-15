//! What a row's tools may be listed from, how a listing lands in its
//! section, and the window path for the rows that need no connection.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::TestAppContext;
use knot_mcp_client::{Endpoint, StdioServer, Tool};
use knot_mcp_probe::{Inventory, ServerRow, ServerState, Target};
use parking_lot::Mutex;
use uuid::Uuid;

use super::{ToolSource, ToolsClaim, ToolsKey, ToolsState, Unlisted, knot_tools, source_of};
use crate::workspace_window::mcp_panel::rows::{KnotRow, SectionRow};
use crate::workspace_window::mcp_panel::state::McpSection;
use crate::workspace_window::shortcuts_tests::window_with_agents;

fn folder() -> PathBuf {
    PathBuf::from("/work/agent")
}

fn http(state: ServerState) -> SectionRow {
    SectionRow::Agent(ServerRow::new("search",
                                     Target::Http { url: "http://127.0.0.1:9/mcp".to_owned(), },
                                     state))
}

fn stdio(launch: Option<StdioServer>) -> SectionRow {
    let row = ServerRow::new("files",
                             Target::Stdio { command: "node files.js".to_owned(), },
                             ServerState::Connected);
    SectionRow::Agent(match launch {
                          Some(launch) => row.with_launch(launch),
                          None => row,
                      })
}

fn launch(cwd: Option<&str>) -> StdioServer {
    StdioServer { program: "node".to_owned(),
                  args:    vec!["files.js".to_owned()],
                  env:     Vec::new(),
                  cwd:     cwd.map(PathBuf::from), }
}

fn knot_row() -> SectionRow {
    knot_row_in(knot_mcp::ServerState::Running { addr: "127.0.0.1:8767".parse()
                                                                       .expect("an address"), })
}

fn knot_row_in(state: knot_mcp::ServerState) -> SectionRow {
    SectionRow::Knot(KnotRow { state,
                               url: "http://127.0.0.1:8767/mcp".to_owned(),
                               also_configured: false })
}

#[test]
fn knots_own_row_lists_from_its_catalog() {
    assert_eq!(source_of(&knot_row(), &folder()), ToolSource::Knot);
    let tools = knot_tools();
    assert!(tools.iter().any(|tool| tool.name == "send-message"),
            "{tools:?}");
    assert!(tools.iter().all(|tool| tool.description.is_some()));
}

/// Knot's tools are only callable while its server runs.
#[test]
fn knots_own_row_is_unlisted_while_its_server_is_off() {
    assert_eq!(source_of(&knot_row_in(knot_mcp::ServerState::Disabled), &folder()),
               ToolSource::Unlisted(Unlisted::NotConnected));
}

#[test]
fn a_connected_or_unknown_http_server_is_asked() {
    for state in [ServerState::Connected, ServerState::Unknown] {
        assert_eq!(source_of(&http(state), &folder()),
                   ToolSource::Remote(Endpoint::Http { url: "http://127.0.0.1:9/mcp".to_owned(), }),
                   "{state:?}");
    }
}

/// A server the agent is not connected to is not contacted: disabled and
/// pending-approval servers are deliberately off, and one needing sign-in
/// would need a credential Knot does not hold.
#[test]
fn a_server_the_agent_is_not_connected_to_is_not_asked() {
    for state in [ServerState::NeedsAuthentication,
                  ServerState::PendingApproval,
                  ServerState::Disabled,
                  ServerState::Failed]
    {
        assert_eq!(source_of(&http(state), &folder()),
                   ToolSource::Unlisted(Unlisted::NotConnected),
                   "{state:?}");
    }
}

/// A command line given only as text is never split into a guess.
#[test]
fn a_stdio_server_without_an_exact_launch_is_not_started() {
    assert_eq!(source_of(&stdio(None), &folder()),
               ToolSource::Unlisted(Unlisted::NoLaunch));
}

#[test]
fn a_stdio_server_starts_in_the_agent_folder_unless_it_names_its_own() {
    let ToolSource::Remote(Endpoint::Stdio(default)) =
        source_of(&stdio(Some(launch(None))), &folder())
    else {
        panic!("a stdio launch is listed");
    };
    assert_eq!(default.cwd.as_deref(), Some(folder().as_path()));

    let ToolSource::Remote(Endpoint::Stdio(own)) =
        source_of(&stdio(Some(launch(Some("/srv/files")))), &folder())
    else {
        panic!("a stdio launch is listed");
    };
    assert_eq!(own.cwd.as_deref(), Some(Path::new("/srv/files")));
}

#[test]
fn two_servers_sharing_a_name_have_different_keys() {
    let other =
        SectionRow::Agent(ServerRow::new("search",
                                         Target::Http { url: "https://example.com/mcp".to_owned(), },
                                         ServerState::Connected));
    assert_ne!(ToolsKey::of(&http(ServerState::Connected)),
               ToolsKey::of(&other));
}

#[test]
fn a_new_inventory_clears_listings_and_open_rows() {
    let key = ToolsKey::of(&knot_row());
    let mut section = McpSection::default();
    assert!(section.toggle_tools(&key));
    section.set_tools(key.clone(), ToolsState::Listed(knot_tools()));

    section.publish(Ok(Inventory::Probed { rows: Vec::new(),
                                           at:   Instant::now(), }));

    assert!(!section.tools_open(&key));
    assert_eq!(section.tools(&key), None);
}

#[test]
fn toggling_twice_shuts_again() {
    let key = ToolsKey::of(&knot_row());
    let mut section = McpSection::default();
    assert!(section.toggle_tools(&key));
    assert!(!section.toggle_tools(&key));
    assert!(!section.tools_open(&key));
}

/// A claim dropped without a report still answers, so a row cannot sit on
/// "listing" with nothing running - and frees its slot either way.
#[test]
fn an_unreported_claim_reports_a_failure_and_frees_the_row() {
    let in_flight = Arc::new(Mutex::new(std::collections::BTreeSet::new()));
    let results = Arc::new(Mutex::new(Vec::new()));
    let (agent, key) = (Uuid::new_v4(), ToolsKey::of(&knot_row()));

    let claim = ToolsClaim::claim(&in_flight, &results, agent, key.clone()).expect("claimed");
    assert!(ToolsClaim::claim(&in_flight, &results, agent, key.clone()).is_none(),
            "a second listing of the same row waits for the first");
    drop(claim);

    assert!(in_flight.lock().is_empty());
    let reported = results.lock();
    assert_eq!(reported.len(), 1);
    assert!(reported[0].2.is_err());
}

/// The window path for the rows that need no connection: Knot's own tools
/// list at once, and an unconnected server says why without starting
/// anything.
#[gpui_kit::test]
fn opening_rows_lists_or_explains_without_a_connection(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.toggle_mcp_tools(agent, &knot_row());
                    view.toggle_mcp_tools(agent, &http(ServerState::Disabled));

                    let section = view.mcp_section(agent).expect("a section");
                    assert!(matches!(section.tools(&ToolsKey::of(&knot_row())),
                                     Some(ToolsState::Listed(tools)) if !tools.is_empty()));
                    assert_eq!(section.tools(&ToolsKey::of(&http(ServerState::Disabled))),
                               Some(&ToolsState::Unlisted(Unlisted::NotConnected)));
                    assert!(view.mcp_tools_in_flight.lock().is_empty(),
                            "nothing was started");
                });
}

/// A listing that lands is moved into its row and asks for a repaint; one
/// for an agent whose section has gone is dropped, not resurrected.
#[gpui_kit::test]
fn a_landed_listing_reaches_its_row_and_only_its_row(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];
    let key = ToolsKey::of(&http(ServerState::Connected));
    let tool = Tool { name:        "search".to_owned(),
                      description: None, };

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.mcp_sections.entry(agent).or_default();
                    view.mcp_tool_results
                        .lock()
                        .push((agent, key.clone(), Ok(vec![tool.clone()])));
                    view.mcp_tool_results
                        .lock()
                        .push((Uuid::new_v4(), key.clone(), Ok(Vec::new())));

                    assert!(view.drain_mcp_tools(), "a landing asks for a repaint");
                    assert_eq!(view.mcp_section(agent)
                                   .and_then(|section| section.tools(&key)),
                               Some(&ToolsState::Listed(vec![tool.clone()])));
                    assert_eq!(view.mcp_sections.len(),
                               1,
                               "no section was made for a stranger");
                    assert!(!view.drain_mcp_tools(), "nothing left to drain");
                });
}
