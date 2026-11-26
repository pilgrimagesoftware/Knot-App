//! Unit tests for [`super`].
//!
//! Migration from the legacy single document has its own suite in
//! `legacy/tests.rs`; everything here is about a store already in the
//! per-document arrangement.

mod binding;
mod dismissed_mcp_servers;
mod personas;
mod preferences;
mod prompts;
mod pull_requests;
mod session_scheduling;
mod trusted_repositories;

use std::fs;

use tempfile::TempDir;
use tempfile::tempdir;

use super::*;
use crate::CostTier;
use crate::consts::{
    AGENTS_FILE, BENCH_FILE, DOCUMENT_TEMP_EXTENSION, PERSONAS_FILE, PREFERENCES_FILE,
    PROMPTS_FILE, RECENT_REPOS_FILE, WORKSPACES_FILE,
};
use crate::settings::{PersonaState, PersonaType};

fn agent_id() -> Uuid {
    Uuid::new_v4()
}

fn persona(name: &str, persona_type: PersonaType, state: PersonaState) -> Persona {
    Persona { id: agent_id(),
              name: name.to_string(),
              instructions: String::new(),
              persona_type,
              state }
}

/// Writes `document` as the store's preferences and loads it back, so each
/// decode case reads as the document it is about.
fn load_document(document: &str) -> (TempDir, Settings) {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(PREFERENCES_FILE), document).unwrap();
    let settings = Settings::load_from_root(dir.path()).unwrap();
    (dir, settings)
}

#[test]
fn default_scalars() {
    let s = Settings::default();
    assert_eq!(s.mcp_server_port, 8767);
    assert_eq!(s.terminal_font_name, "JetBrains Mono");
    assert!(s.restore_layout_on_launch);
    assert!(!s.restore_conversation_on_launch);
    assert!(s.reopen_workspaces_on_launch);
    assert!(s.mcp_server_enabled);
}

#[test]
fn a_store_with_no_documents_yields_defaults() {
    let dir = tempdir().unwrap();
    let s = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(s, Settings::with_store_root(dir.path()));
}

#[test]
fn corrupt_preferences_yield_defaults() {
    let (_dir, s) = load_document("{ not json");
    assert_eq!(s.mcp_server_port, 8767);
    assert!(s.saved_agents.is_empty());
}

#[test]
fn scalar_persists_across_reload() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9000;
    s.persist().unwrap();
    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.mcp_server_port, 9000);
}

/// The collections are their own documents now, so the preferences document
/// must not carry them: a key left behind would be written on every scalar
/// edit and read back as a second, stale copy of the collection.
#[test]
fn the_preferences_document_holds_no_collection_keys() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.recent_repos = vec!["alpha".to_string()];
    s.personas = vec![persona("Rookie", PersonaType::User, PersonaState::Enabled)];

    s.persist().unwrap();

    let written = fs::read_to_string(dir.path().join(PREFERENCES_FILE)).unwrap();
    for key in ["savedAgents",
                "savedWorkspaces",
                "personas",
                "benchAgents",
                "recentRepos"]
    {
        assert!(!written.contains(key),
                "preferences document must not carry `{key}`:\n{written}");
    }
    assert!(written.contains("mcpServerPort"));
}

#[test]
fn a_whole_surface_persist_writes_every_document() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9100;
    s.saved_agents = Vec::new();
    s.recent_repos = vec!["alpha".to_string(), "beta".to_string()];
    s.personas = vec![persona("Rookie", PersonaType::User, PersonaState::Enabled)];
    s.bench_agents = vec![BenchAgent::new(agent_id(), "bench", None, "/repo")];
    s.prompts = vec![crate::Prompt::new("gate", "make").unwrap()];

    s.persist().unwrap();

    for file in [PREFERENCES_FILE,
                 AGENTS_FILE,
                 WORKSPACES_FILE,
                 PERSONAS_FILE,
                 BENCH_FILE,
                 PROMPTS_FILE,
                 RECENT_REPOS_FILE]
    {
        assert!(dir.path().join(file).exists(), "{file} was not written");
    }
    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.mcp_server_port, 9100);
    assert_eq!(reloaded.recent_repos, vec!["alpha", "beta"]);
    assert_eq!(reloaded.personas.len(), 1);
    assert_eq!(reloaded.bench_agents.len(), 1);
    assert_eq!(reloaded.prompts.len(), 1);
}

/// Every document beside the one being written, as bytes.
fn other_documents(dir: &TempDir, written: &str) -> Vec<(String, Vec<u8>)> {
    [PREFERENCES_FILE,
     AGENTS_FILE,
     WORKSPACES_FILE,
     PERSONAS_FILE,
     BENCH_FILE,
     RECENT_REPOS_FILE].into_iter()
                       .filter(|file| *file != written)
                       .map(|file| (file.to_string(), fs::read(dir.path().join(file)).unwrap()))
                       .collect()
}

#[test]
fn saving_a_persona_leaves_every_other_document_untouched() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.recent_repos = vec!["alpha".to_string()];
    s.persist().unwrap();
    let before = other_documents(&dir, PERSONAS_FILE);

    s.add_persona("Rookie", "be helpful").unwrap();

    assert_eq!(other_documents(&dir, PERSONAS_FILE), before);
    assert_eq!(Settings::load_from_root(dir.path()).unwrap().personas.len(),
               1);
}

#[test]
fn setting_a_scalar_leaves_every_collection_untouched() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.recent_repos = vec!["alpha".to_string()];
    s.personas = vec![persona("Rookie", PersonaType::User, PersonaState::Enabled)];
    s.persist().unwrap();
    let before = other_documents(&dir, PREFERENCES_FILE);

    s.mcp_server_port = 9200;
    s.persist_preferences().unwrap();

    assert_eq!(other_documents(&dir, PREFERENCES_FILE), before);
    assert_eq!(Settings::load_from_root(dir.path()).unwrap()
                                                   .mcp_server_port,
               9200);
}

#[test]
fn one_corrupt_collection_costs_only_itself() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9300;
    s.recent_repos = vec!["alpha".to_string(), "beta".to_string()];
    s.personas = vec![persona("Rookie", PersonaType::User, PersonaState::Enabled)];
    s.persist().unwrap();
    fs::write(dir.path().join(PERSONAS_FILE), "{ not json").unwrap();

    let reloaded = Settings::load_from_root(dir.path()).unwrap();

    assert!(reloaded.personas.is_empty());
    assert_eq!(reloaded.mcp_server_port, 9300);
    assert_eq!(reloaded.recent_repos, vec!["alpha", "beta"]);
}

#[test]
fn corrupt_preferences_cost_only_the_scalars() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9400;
    s.recent_repos = vec!["alpha".to_string()];
    s.persist().unwrap();
    fs::write(dir.path().join(PREFERENCES_FILE), "{ not json").unwrap();

    let reloaded = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(reloaded.mcp_server_port, MCP_PORT_DEFAULT);
    assert_eq!(reloaded.recent_repos, vec!["alpha"]);
}

#[test]
fn detect_picks_first_existing() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("src");
    let present = dir.path().join("source");
    fs::create_dir(&present).unwrap();
    let candidates: Vec<&Path> = vec![missing.as_path(), present.as_path()];
    assert_eq!(detect_source_base_folder(&candidates), Some(present));
}

#[test]
fn init_source_folder_runs_once() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.init_source_folder().unwrap();
    assert!(s.source_folder_detected);
    s.source_base_folder = "/explicit".to_string();
    s.init_source_folder().unwrap();
    assert_eq!(s.source_base_folder, "/explicit");
}

#[test]
fn recent_repos_moves_to_front_and_caps() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    for name in ["c", "b", "a"] {
        s.add_recent_repo(name).unwrap();
    }
    assert_eq!(s.recent_repos, vec!["a", "b", "c"]);
    s.add_recent_repo("b").unwrap();
    assert_eq!(s.recent_repos, vec!["b", "a", "c"]);
    for name in ["d", "e", "f"] {
        s.add_recent_repo(name).unwrap();
    }
    assert_eq!(s.recent_repos.len(), RECENT_REPOS_MAX);
    assert_eq!(s.recent_repos[0], "f");
}

/// Two agents working in one repository are two bench entries (#500): the
/// second save must not overwrite the first.
#[test]
fn bench_keeps_different_agents_from_one_folder() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.add_bench_agent(BenchAgent::new(agent_id(), "Reviewer", None, "/repo"))
     .unwrap();
    s.add_bench_agent(BenchAgent::new(agent_id(), "Tester", None, "/repo"))
     .unwrap();
    let names: Vec<_> = s.bench_agents
                         .iter()
                         .map(|entry| entry.name.as_str())
                         .collect();
    assert_eq!(names, ["Reviewer", "Tester"]);
}

/// Saving the same agent again replaces its entry rather than piling up
/// copies.
#[test]
fn bench_replaces_the_same_agent_saved_again() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let mut first = BenchAgent::new(agent_id(), "Reviewer", None, "/repo");
    first.description = "before".to_owned();
    let mut again = BenchAgent::new(agent_id(), "Reviewer", None, "/repo");
    again.description = "after".to_owned();
    s.add_bench_agent(first).unwrap();
    s.add_bench_agent(again).unwrap();
    assert_eq!(s.bench_agents.len(), 1);
    assert_eq!(s.bench_agents[0].description, "after");
}

/// An agent of the same name in another folder is another agent.
#[test]
fn bench_keeps_same_name_in_another_folder() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.add_bench_agent(BenchAgent::new(agent_id(), "Reviewer", None, "/repo"))
     .unwrap();
    s.add_bench_agent(BenchAgent::new(agent_id(), "Reviewer", None, "/other"))
     .unwrap();
    assert_eq!(s.bench_agents.len(), 2);
}

#[test]
fn persist_leaves_no_temporary_file_behind() {
    let dir = tempdir().unwrap();
    let settings = Settings::with_store_root(dir.path());

    settings.persist().unwrap();

    let preferences = dir.path().join(PREFERENCES_FILE);
    assert!(preferences.exists());
    assert!(!preferences.with_extension(DOCUMENT_TEMP_EXTENSION).exists());
}

#[test]
fn persist_replaces_a_stale_temporary_file_rather_than_reusing_it() {
    let dir = tempdir().unwrap();
    let preferences = dir.path().join(PREFERENCES_FILE);
    let temporary = preferences.with_extension(DOCUMENT_TEMP_EXTENSION);
    // What an interrupted write would leave: a partial document that
    // must not become the next persisted one.
    fs::write(&temporary, "{ not json").unwrap();

    let mut settings = Settings::with_store_root(dir.path());
    settings.ui_font_size = 17.0;
    settings.persist().unwrap();

    assert!(!temporary.exists());
    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.ui_font_size, 17.0);
}

/// Task 10.3: records written before the agent registry existed carry no
/// `description`, `capabilities` or `costTier`. They have to load without
/// error and read back undescribed, untagged and mid-priced: the guarantee
/// `openspec/specs/agent-registry/spec.md` makes about records that predate
/// it.
#[test]
fn records_written_before_the_registry_load_with_the_registry_defaults() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(AGENTS_FILE),
              r#"[{
                   "id": "11111111-1111-4111-8111-111111111111",
                   "name": "Old Agent",
                   "folder": "/tmp/old"
                 }]"#).unwrap();
    fs::write(dir.path().join(BENCH_FILE),
              r#"[{
                   "id": "22222222-2222-4222-8222-222222222222",
                   "name": "Old Template",
                   "folder": "/tmp/bench"
                 }]"#).unwrap();

    let settings = Settings::load_from_root(dir.path()).unwrap();

    let agent = &settings.saved_agents[0];
    assert_eq!(agent.name, "Old Agent");
    assert_eq!(agent.description, "");
    assert!(agent.capabilities.is_empty());
    assert_eq!(agent.cost_tier, CostTier::Medium);

    let bench = &settings.bench_agents[0];
    assert_eq!(bench.name, "Old Template");
    assert_eq!(bench.description, "");
    assert!(bench.capabilities.is_empty());
    assert_eq!(bench.cost_tier, CostTier::Medium);
}
