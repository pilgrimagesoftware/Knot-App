//! The persona roster: listing, adding, editing, removing and restoring
//! personas, and the shipped defaults themselves.

use tempfile::tempdir;

use super::super::*;
use super::{agent_id, persona};
use crate::consts::DEFAULT_PERSONAS;
use crate::settings::{PersonaState, PersonaType};

#[test]
fn active_personas_excludes_deleted_and_sorts_ci() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.personas = vec![persona("beta", PersonaType::User, PersonaState::Enabled),
                      persona("Alpha", PersonaType::User, PersonaState::Enabled),
                      persona("gone", PersonaType::System, PersonaState::Deleted),];
    let names: Vec<&str> = s.active_personas()
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect();
    assert_eq!(names, vec!["Alpha", "beta"]);
}

#[test]
fn default_personas_install_once() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), DEFAULT_PERSONAS.len());
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), DEFAULT_PERSONAS.len());
}

#[test]
fn add_update_and_lookup_persona() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_persona("Rookie", "be helpful").unwrap().id;
    assert_eq!(s.personas.len(), 1);
    assert_eq!(s.persona(id).unwrap().name, "Rookie");

    s.update_persona(id, "Veteran", "be terse").unwrap();
    let persona = s.persona(id).unwrap();
    assert_eq!(persona.name, "Veteran");
    assert_eq!(persona.instructions, "be terse");

    s.update_persona(agent_id(), "Nobody", "").unwrap();
    assert_eq!(s.personas.len(), 1);
}

#[test]
fn persona_lookup_excludes_deleted() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.personas = vec![persona("gone", PersonaType::System, PersonaState::Deleted)];
    assert!(s.persona(s.personas[0].id).is_none());
}

/// Both launch paths once passed the agent's own id here, so every
/// assigned persona resolved to nothing (#519).
#[test]
fn persona_for_resolves_an_agents_assignment_not_its_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let persona_id = s.add_persona("Rust Pro", "borrow first").unwrap().id;
    let agent = agent_id();

    assert_eq!(s.persona_for(Some(persona_id)).unwrap().name, "Rust Pro");
    assert!(s.persona_for(None).is_none());
    assert!(s.persona_for(Some(agent)).is_none());
}

#[test]
fn remove_persona_soft_deletes_system_and_hard_deletes_user() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.personas = vec![persona("System", PersonaType::System, PersonaState::Enabled),
                      persona("User", PersonaType::User, PersonaState::Enabled),];
    let system_id = s.personas[0].id;
    let user_id = s.personas[1].id;

    s.remove_persona(system_id).unwrap();
    assert_eq!(s.personas.len(), 2);
    assert_eq!(s.personas.iter().find(|p| p.id == system_id).unwrap().state,
               PersonaState::Deleted);

    s.remove_persona(user_id).unwrap();
    assert_eq!(s.personas.len(), 1);
    assert!(s.personas.iter().all(|p| p.id != user_id));

    s.remove_persona(agent_id()).unwrap();
    assert_eq!(s.personas.len(), 1);
}

#[test]
fn restore_default_personas_reverts_edits_and_adds_missing() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let (id, name, instructions) = DEFAULT_PERSONAS[0];
    let id = Uuid::parse_str(id).unwrap();
    s.personas = vec![Persona { id,
                                name: "Renamed".to_string(),
                                instructions: "different".to_string(),
                                persona_type: PersonaType::System,
                                state: PersonaState::Disabled },
                      persona("Mine", PersonaType::User, PersonaState::Enabled),];

    s.restore_default_personas().unwrap();

    assert_eq!(s.personas.len(),
               DEFAULT_PERSONAS.len() + 1,
               "the defaults plus the user's own");
    let restored = s.personas.iter().find(|p| p.id == id).unwrap();
    assert_eq!(restored.name, name);
    assert_eq!(restored.instructions, instructions);
    assert_eq!(restored.state, PersonaState::Enabled);
    assert!(s.personas.iter().any(|p| p.name == "Mine"));
}

#[test]
fn deleted_default_persona_not_reinstalled() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let (id, _, _) = DEFAULT_PERSONAS[0];
    s.personas = vec![Persona { id:           Uuid::parse_str(id).unwrap(),
                                name:         "custom".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::System,
                                state:        PersonaState::Deleted, }];
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), DEFAULT_PERSONAS.len());
    assert_eq!(s.personas
                .iter()
                .filter(|p| p.state == PersonaState::Deleted)
                .count(),
               1);
}

/// The Orchestrator persona describes a *method*, never a team.
///
/// `agent-registry` - "A roster is obtained by query, never stored as
/// text". A teammate named here is a copy of state that goes stale the
/// first time an agent is added or removed, and nothing would catch it,
/// which is the whole reason the registry exists. Checked rather than left
/// to review, because prose drifts.
#[test]
fn the_orchestrator_persona_names_a_method_and_no_teammates() {
    let (_, name, instructions) = DEFAULT_PERSONAS.iter()
                                                  .find(|(_, name, _)| *name == "Orchestrator")
                                                  .expect("the Orchestrator persona ships");
    assert_eq!(*name, "Orchestrator");

    // It has to point at the tools, or it describes nothing actionable.
    for tool in ["describe-agents",
                 "plan-tasks",
                 "dispatch-task",
                 "complete-task",
                 "task-status",
                 "send-message"]
    {
        assert!(instructions.contains(tool),
                "the persona should name {tool}");
    }

    // And it must not name an agent, an agent type, or how many there are -
    // every one of those goes stale without warning.
    let lowered = instructions.to_lowercase();
    for stale in ["claude", "codex", "opencode", "gemini", "copilot", "shell"] {
        assert!(!lowered.contains(stale),
                "the persona must not name the agent type {stale}");
    }
    for (_, other, _) in DEFAULT_PERSONAS.iter()
                                         .filter(|(_, other, _)| *other != "Orchestrator")
    {
        assert!(!instructions.contains(other),
                "the persona must not name {other}");
    }
}

/// A shipped default is matched by id across installs, so two entries
/// sharing one would overwrite each other on startup.
#[test]
fn every_shipped_persona_has_a_distinct_id() {
    let mut ids: Vec<&str> = DEFAULT_PERSONAS.iter().map(|(id, _, _)| *id).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "two shipped personas share an id");
}
