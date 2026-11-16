//! Scheduled starts and idle shutdown persist (#404): an agent's schedule and
//! idle policy with the saved agents, the idle default with the preferences,
//! and the last scheduled run handled with the workspace's UI state. Records
//! written before any of these existed load as no schedule, the default idle
//! policy, no idle default, and nothing handled.

use std::fs;

use tempfile::tempdir;
use uuid::Uuid;

use crate::Settings;
use crate::consts::{AGENTS_FILE, PREFERENCES_FILE};
use crate::schedule::{IdleAction, IdlePolicy, IdleRule, Schedule};
use crate::settings::{SavedAgent, WorkspaceUiState};

fn strip_key(path: &std::path::Path, key: &str) -> bool {
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).expect("the document reads"))
            .expect("the document is JSON");
    let removed = match &mut document {
        serde_json::Value::Object(object) => object.remove(key).is_some(),
        serde_json::Value::Array(items) => {
            // Every item, not `any`: `any` stops at the first removal and
            // leaves the key on the rest.
            let mut removed = false;
            for item in items.iter_mut()
                             .filter_map(serde_json::Value::as_object_mut)
            {
                removed |= item.remove(key).is_some();
            }
            removed
        }
        _ => false,
    };
    fs::write(path, document.to_string()).expect("the document writes");
    removed
}

#[test]
fn a_schedule_and_idle_policy_survive_a_restart() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    let mut agent = SavedAgent::new(Uuid::new_v4(), "nightly", None, "/tmp/nightly");
    agent.schedule = Some("weekdays 09:00".parse().expect("a preset"));
    agent.idle_policy = IdlePolicy::Custom(IdleRule::new(30, IdleAction::Reset).expect("non-zero"));
    settings.saved_agents = vec![agent.clone()];
    settings.persist().expect("persists");

    let loaded = Settings::load_from_root(dir.path()).expect("loads");

    assert_eq!(loaded.saved_agents, vec![agent]);
}

#[test]
fn an_agent_from_before_scheduling_loads_unscheduled_and_inheriting() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    let mut agent = SavedAgent::new(Uuid::new_v4(), "old", None, "/tmp/old");
    agent.schedule = Some(Schedule::cron("0 9 * * *").expect("valid"));
    settings.saved_agents = vec![agent];
    settings.persist().expect("persists");
    let path = dir.path().join(AGENTS_FILE);
    assert!(strip_key(&path, "schedule"),
            "the schedule is written under the key stripped here");
    strip_key(&path, "idlePolicy");

    let loaded = Settings::load_from_root(dir.path()).expect("loads");

    assert_eq!(loaded.saved_agents[0].schedule, None);
    assert_eq!(loaded.saved_agents[0].idle_policy, IdlePolicy::Inherit);
}

#[test]
fn the_idle_default_is_a_preference_and_off_until_set() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    assert_eq!(settings.idle_default, None);
    settings.idle_default = IdleRule::new(45, IdleAction::Deactivate);
    settings.persist_preferences().expect("persists");

    let loaded = Settings::load_from_root(dir.path()).expect("loads");

    assert_eq!(loaded.idle_default,
               IdleRule::new(45, IdleAction::Deactivate));
}

#[test]
fn an_unreadable_idle_default_loads_as_off_without_failing_the_document() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    settings.mcp_server_port = 9301;
    settings.persist_preferences().expect("persists");
    let path = dir.path().join(PREFERENCES_FILE);
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("reads")).expect("JSON");
    document["idleDefault"] = serde_json::json!("nap for a while");
    fs::write(&path, document.to_string()).expect("writes");

    let loaded = Settings::load_from_root(dir.path()).expect("loads");

    assert_eq!(loaded.idle_default, None);
    assert_eq!(loaded.mcp_server_port, 9301,
               "the rest of the preferences still load");
}

#[test]
fn ui_state_from_before_scheduling_has_handled_nothing() {
    let state: WorkspaceUiState =
        serde_json::from_str(r#"{"layoutMode":"single"}"#).expect("loads");
    assert!(state.schedule_runs_handled.is_empty());
}
