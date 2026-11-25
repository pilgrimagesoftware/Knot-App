//! Unit tests for the crate root: the tool list, the hook handler
//! (registration, status, subagent events) and catalogs sharing a store.

// The catalogue and dispatch moved to `catalog.rs`; these exercise them
// through the trait, so it has to be in scope here now.
use knot_mcp::ToolCatalog;
use knot_messaging::NoopNotifier;
use tempfile::tempdir;

use super::*;

fn catalog() -> McpToolCatalog {
    let (_tx, rx) = watch::channel(Vec::new());
    McpToolCatalog::new(Arc::new(Mutex::new(AgentStore::new())),
                        rx,
                        Arc::new(NoopNotifier))
}

#[test]
fn lists_exactly_the_catalogued_tools_with_object_schemas() {
    let defs = catalog().list();
    assert_eq!(defs.len(), 21);
    for def in &defs {
        assert_eq!(def.input_schema.schema_type, "object");
    }

    let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    for expected in ["register-agent",
                     "list-agents",
                     "describe-agents",
                     "send-message",
                     "check-messages",
                     "broadcast-message",
                     "list-repos",
                     "list-worktrees",
                     "create-agent",
                     "close-agent",
                     "create-worktree",
                     "set-status",
                     "set-working-directory",
                     "display-markdown",
                     "view-mermaid",
                     "show-checklist",
                     "get-checklist",
                     "plan-tasks",
                     "dispatch-task",
                     "complete-task",
                     "task-status"]
    {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }
}

#[tokio::test]
async fn unknown_tool_name_is_a_tool_error() {
    let result = catalog().call("does-not-exist", serde_json::json!({}))
                          .await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("does-not-exist"));
}

#[tokio::test]
async fn dispatches_register_agent_by_name() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());

    let result = cat.call(consts::REGISTER_AGENT,
                          serde_json::json!({"agentId": id.to_string()}))
                    .await;

    assert_eq!(result.is_error, None);
    assert!(cat.agents.lock().agent(id).unwrap().is_registered);
}

#[tokio::test]
async fn successful_agent_mutation_persists_durable_state() {
    let dir = tempdir().unwrap();
    let cat = catalog().with_settings(knot_core::SharedSettings::new(
        knot_core::Settings::with_store_root(dir.path()),
    ));
    let id = cat.agents
                .lock()
                .create("/tmp/persisted", knot_agents::CreateOptions::default());

    let result = cat.call(consts::REGISTER_AGENT,
                          serde_json::json!({"agentId": id.to_string()}))
                    .await;

    assert!(result.is_error.is_none());
    let settings = knot_core::Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(settings.saved_agents.len(), 1);
    assert_eq!(settings.saved_agents[0].id, id);
    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_workspaces[0].agent_ids, vec![id]);
}

/// A subagent event is keyed on `hook`, carries no `status`, and must not
/// move the agent's activity state as a side effect of being reported.
#[test]
fn a_subagent_dispatch_is_recorded_without_touching_the_agents_state() {
    let subagents: SubagentRegistryHandle = Arc::default();
    let cat = catalog().with_subagents(Arc::clone(&subagents));
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    cat.agents.lock().set_state(id, AgentState::Running);

    cat.status(&subagent_hook(id,
                              "SubagentStart",
                              serde_json::json!({
                                  "subagent_id": "sub-1",
                                  "subagent_type": "discovery",
                                  "task": "Map the callers"
                              })))
       .expect("a complete dispatch is accepted");

    assert_eq!(cat.agents.lock().agent(id).unwrap().state,
               AgentState::Running,
               "reporting a subagent moved the agent's own status");
    let held = subagents.lock();
    let records = held.ordered(id, std::time::Instant::now());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task, "Map the callers");
}

#[test]
fn a_subagent_completion_moves_its_record() {
    let subagents: SubagentRegistryHandle = Arc::default();
    let cat = catalog().with_subagents(Arc::clone(&subagents));
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());

    cat.status(&subagent_hook(id,
                              "SubagentStart",
                              serde_json::json!({ "subagent_id": "sub-1",
                                                  "task": "Map the callers" })))
       .unwrap();
    cat.status(&subagent_hook(id,
                              "SubagentStop",
                              serde_json::json!({ "subagent_id": "sub-1",
                                                  "outcome": "failed",
                                                  "reason": "no such persona" })))
       .unwrap();

    let held = subagents.lock();
    let records = held.ordered(id, std::time::Instant::now());
    assert_eq!(records[0].state.failure_reason(), Some("no such persona"));
}

/// The payload is the poster's contract, so a malformed one is an error it
/// can act on rather than a silent no-op.
#[test]
fn a_subagent_event_missing_a_required_field_is_rejected() {
    let cat = catalog().with_subagents(Arc::default());
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());

    let no_id = subagent_hook(id, "SubagentStart", serde_json::json!({ "task": "go" }));
    let no_task = subagent_hook(id,
                                "SubagentStart",
                                serde_json::json!({ "subagent_id": "s" }));
    let no_outcome = subagent_hook(id,
                                   "SubagentStop",
                                   serde_json::json!({ "subagent_id": "s" }));

    assert!(cat.status(&no_id).is_err());
    assert!(cat.status(&no_task).is_err());
    assert!(cat.status(&no_outcome).is_err());
}

/// Knot may have started after the dispatch. Answering an error there would
/// make the agent's hook report a failure for something it did correctly.
#[test]
fn a_completion_for_an_unknown_subagent_succeeds_and_records_nothing() {
    let subagents: SubagentRegistryHandle = Arc::default();
    let cat = catalog().with_subagents(Arc::clone(&subagents));
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());

    let request = subagent_hook(id,
                                "SubagentStop",
                                serde_json::json!({ "subagent_id": "never-seen",
                                                    "outcome": "succeeded" }));

    assert!(cat.status(&request).is_ok());
    assert!(subagents.lock().is_empty_for(id));
}

/// The subagent branch returns early, so a mistake there would silently
/// stop every ordinary activity update.
#[test]
fn an_ordinary_status_hook_still_moves_the_agent() {
    let cat = catalog().with_subagents(Arc::default());
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());

    let request: HookRequest = serde_json::from_value(serde_json::json!({
                                                          "agent_id": id,
                                                          "hook": "Stop",
                                                          "status": "idle",
                                                          "payload": {}
                                                      })).unwrap();

    cat.status(&request)
       .expect("an ordinary status post is accepted");
    assert_eq!(cat.agents.lock().agent(id).unwrap().state, AgentState::Idle);
}

fn subagent_hook(agent: Uuid, hook: &str, payload: serde_json::Value) -> HookRequest {
    serde_json::from_value(serde_json::json!({
                               "agent_id": agent,
                               "hook": hook,
                               "payload": payload
                           })).unwrap()
}

#[test]
fn hook_registration_updates_session_and_metadata() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest =
        serde_json::from_value(serde_json::json!({
                                   "agent_id": id,
                                   "session_id": "session-1",
                                   "payload": {"cwd": "/tmp/a", "model": "claude-sonnet"}
                               })).unwrap();

    cat.register(&request).unwrap();
    let agent = cat.agents.lock().agent(id).unwrap().clone();
    assert!(agent.is_registered);
    assert_eq!(agent.session_id.as_deref(), Some("session-1"));
    assert_eq!(agent.metadata.get("model"),
               Some(&"claude-sonnet".to_string()));
}

#[tokio::test]
async fn hook_status_updates_state_through_tracker_and_codex_session() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest = serde_json::from_value(serde_json::json!({
        "agent_id": id,
        "agent": "codex",
        "payload": {"type": "agent-turn-complete", "thread-id": "thread-1"}
    }))
    .unwrap();

    cat.status(&request).unwrap();
    tokio::task::yield_now().await;
    let agent = cat.agents.lock().agent(id).unwrap().clone();
    assert_eq!(agent.state, AgentState::Idle);
    assert_eq!(agent.session_id.as_deref(), Some("thread-1"));
}

#[tokio::test]
async fn claude_hook_status_updates_state_through_tracker() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest = serde_json::from_value(serde_json::json!({
                                                          "agent_id": id,
                                                          "agent": "claude",
                                                          "status": "input"
                                                      })).unwrap();

    cat.status(&request).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(cat.agents.lock().agent(id).unwrap().state,
               AgentState::Input);
}

#[test]
fn catalogs_sharing_a_store_observe_each_others_mutations() {
    let (_tx, rx_a) = watch::channel(Vec::new());
    let (_tx, rx_b) = watch::channel(Vec::new());
    let shared = Arc::new(Mutex::new(AgentStore::new()));
    let a = McpToolCatalog::new(Arc::clone(&shared), rx_a, Arc::new(NoopNotifier));
    let b = McpToolCatalog::new(Arc::clone(&shared), rx_b, Arc::new(NoopNotifier));

    let id = shared.lock()
                   .create("/tmp/one", knot_agents::CreateOptions::default());

    assert_eq!(a.agents_snapshot().len(), 1);
    assert_eq!(b.agents_snapshot().len(), 1);

    shared.lock().set_status_text(id, "planning".to_string());
    assert_eq!(a.agents_snapshot()[0].status_text, "planning");
    assert_eq!(b.agents_snapshot()[0].status_text, "planning");
}

#[test]
fn hook_input_message_extracts_optional_payload_message() {
    assert_eq!(hook_input_message(&serde_json::json!({"message": "Question?"})),
               Some("Question?".to_string()));
    assert_eq!(hook_input_message(&serde_json::json!({"message": 42})),
               None);
    assert_eq!(hook_input_message(&serde_json::json!({})), None);
}
