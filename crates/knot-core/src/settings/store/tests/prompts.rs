//! The prompt-library document and the startup prompt on agents and bench
//! entries: round trips, the blank rules, and that editing the library
//! touches no other document.

use std::fs;

use tempfile::tempdir;

use super::super::*;
use super::agent_id;
use crate::consts::{AGENTS_FILE, PROMPTS_FILE};
use crate::error::Error;
use crate::{Prompt, StartupPrompt};

#[test]
fn a_store_without_a_prompts_document_loads_an_empty_library_and_writes_none() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9101;

    s.persist().unwrap();

    assert!(Settings::load_from_root(dir.path()).unwrap()
                                                .prompts
                                                .is_empty());
    assert!(!dir.path().join(PROMPTS_FILE).exists());
}

#[test]
fn library_operations_persist_in_insertion_order() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let first = s.add_prompt("first", "one").unwrap();
    let second = s.add_prompt("second", "two\nlines").unwrap();
    s.update_prompt(first, "first", "uno").unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    let names: Vec<&str> = back.prompts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["first", "second"]);
    assert_eq!(back.prompt(first).unwrap().text, "uno");
    assert_eq!(back.prompt(second).unwrap().text, "two\nlines");

    s.remove_prompt(first).unwrap();
    s.remove_prompt(second).unwrap();
    assert!(Settings::load_from_root(dir.path()).unwrap()
                                                .prompts
                                                .is_empty(),
            "removing the last prompt must survive a reload");
}

#[test]
fn a_blank_prompt_is_refused_and_leaves_the_library_unchanged() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_prompt("keep", "text").unwrap();

    assert!(s.add_prompt("   ", "text").is_err());
    assert!(s.update_prompt(id, "keep", " \n").is_err());

    assert_eq!(s.prompts.len(), 1);
    assert_eq!(s.prompt(id).unwrap().text, "text");
}

#[test]
fn removing_a_referenced_prompt_leaves_the_agents_document_untouched() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_prompt("gate", "make").unwrap();
    let mut agent = SavedAgent::new(agent_id(), "a", None, "/repo");
    agent.startup_prompt = Some(StartupPrompt::Library(id));
    s.saved_agents = vec![agent];
    s.persist_roster().unwrap();
    let before = fs::read(dir.path().join(AGENTS_FILE)).unwrap();

    s.remove_prompt(id).unwrap();

    assert_eq!(fs::read(dir.path().join(AGENTS_FILE)).unwrap(), before);
    assert_eq!(s.saved_agents[0].startup_prompt,
               Some(StartupPrompt::Library(id)));
}

#[test]
fn references_are_counted_across_agents_and_bench() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = Prompt::new("gate", "make").unwrap().id;
    let mut agent = SavedAgent::new(agent_id(), "a", None, "/a");
    agent.startup_prompt = Some(StartupPrompt::Library(id));
    let mut bench = BenchAgent::new(agent_id(), "b", None, "/b");
    bench.startup_prompt = Some(StartupPrompt::Library(id));
    let mut other = BenchAgent::new(agent_id(), "c", None, "/c");
    other.startup_prompt = StartupPrompt::custom("mine");
    s.saved_agents = vec![agent];
    s.bench_agents = vec![bench, other];

    assert_eq!(s.prompt_references(id),
               PromptReferences { agents: 1,
                                  bench:  1, });
    assert!(s.prompt_references(agent_id()).is_empty());
}

#[test]
fn a_custom_startup_prompt_round_trips_through_the_agents_document() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let mut agent = SavedAgent::new(agent_id(), "a", None, "/repo");
    agent.startup_prompt = StartupPrompt::custom("pick up\nthe next issue");
    s.saved_agents = vec![agent.clone()];

    s.persist_roster().unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(back.saved_agents[0].startup_prompt, agent.startup_prompt);
}

#[test]
fn records_without_a_startup_prompt_or_with_blank_custom_text_load_as_none() {
    let agent: SavedAgent =
        serde_json::from_value(serde_json::json!({
                                   "id": agent_id(), "name": "a", "folder": "/repo"
                               })).unwrap();
    let bench: BenchAgent =
        serde_json::from_value(serde_json::json!({
                                   "id": agent_id(), "name": "b", "folder": "/repo",
                                   "startupPrompt": { "kind": "custom", "value": "  " }
                               })).unwrap();
    assert_eq!(agent.startup_prompt, None);
    assert_eq!(bench.startup_prompt, None);
}

#[test]
fn a_library_reference_survives_the_bench_as_a_reference() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_prompt("gate", "make").unwrap();
    let mut entry = BenchAgent::new(agent_id(), "b", None, "/repo");
    entry.startup_prompt = Some(StartupPrompt::Library(id));
    s.add_bench_agent(entry).unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(back.bench_agents[0].startup_prompt,
               Some(StartupPrompt::Library(id)));
}

#[test]
fn bench_entries_are_removed_and_edited_by_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let keep = BenchAgent::new(agent_id(), "keep", None, "/keep");
    let drop = BenchAgent::new(agent_id(), "drop", None, "/drop");
    let (keep_id, drop_id) = (keep.id, drop.id);
    s.add_bench_agent(keep).unwrap();
    s.add_bench_agent(drop).unwrap();

    s.remove_bench_agent(drop_id).unwrap();
    s.update_bench_agent(keep_id, "renamed", StartupPrompt::custom("go"))
     .unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(back.bench_agents.len(), 1);
    assert_eq!(back.bench_agents[0].name, "renamed");
    assert_eq!(back.bench_agents[0].folder, "/keep");
    assert_eq!(back.bench_agents[0].startup_prompt,
               StartupPrompt::custom("go"));
}

#[test]
fn insert_prompt_keeps_the_caller_supplied_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = agent_id();
    s.insert_prompt(Prompt { id,
                             name: "From Library".to_string(),
                             text: "do the thing".to_string(),
                             description: None })
     .unwrap();
    assert_eq!(s.prompts.len(), 1);
    assert_eq!(s.prompt(id).unwrap().name, "From Library");
}

#[test]
fn insert_prompt_rejects_a_duplicate_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = agent_id();
    s.insert_prompt(Prompt { id,
                             name: "First".to_string(),
                             text: "one".to_string(),
                             description: None })
     .unwrap();

    let err = s.insert_prompt(Prompt { id,
                                       name: "Second".to_string(),
                                       text: "two".to_string(),
                                       description: None })
               .unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert_eq!(s.prompts.len(), 1);
}

#[test]
fn insert_prompt_rejects_blank_fields() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let err = s.insert_prompt(Prompt { id:          agent_id(),
                                       name:        String::new(),
                                       text:        "one".to_string(),
                                       description: None, })
               .unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert!(s.prompts.is_empty());
}

#[test]
fn a_prompt_description_is_set_trimmed_and_cleared_when_blank() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_prompt("Review", "Review it.").unwrap();

    s.set_prompt_description(id, "  Reviews the branch. ")
     .unwrap();
    assert_eq!(s.prompt(id).unwrap().description.as_deref(),
               Some("Reviews the branch."));
    let reread = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reread.prompt(id).unwrap().description.as_deref(),
               Some("Reviews the branch."),
               "the description is persisted");

    s.set_prompt_description(id, " ").unwrap();
    assert_eq!(s.prompt(id).unwrap().description, None);
}

#[test]
fn a_prompt_saved_before_descriptions_reads_without_one() {
    let prompt: Prompt =
        serde_json::from_str(r#"{"id":"399d4053-a1e4-4266-a847-de5afadb4caa","name":"n","text":"t"}"#)
            .unwrap();
    assert_eq!(prompt.description, None);
    assert!(!serde_json::to_string(&prompt).unwrap()
                                           .contains("description"));
}

#[test]
fn duplicating_inserts_a_renamed_copy_after_the_original_and_persists_it() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let first = s.add_prompt("Review", "review {{branch}}\nthoroughly")
                 .unwrap();
    s.set_prompt_description(first, "Reviews the branch")
     .unwrap();
    let last = s.add_prompt("Gate", "make").unwrap();

    let copy = s.duplicate_prompt(first)
                .unwrap()
                .expect("a copy of a known prompt");

    let back = Settings::load_from_root(dir.path()).unwrap();
    let ids: Vec<_> = back.prompts.iter().map(|p| p.id).collect();
    assert_eq!(ids,
               [first, copy, last],
               "the copy goes right after the original");
    let (original, duplicate) = (back.prompt(first).unwrap(), back.prompt(copy).unwrap());
    assert_ne!(copy, first);
    assert_ne!(duplicate.name, original.name);
    assert!(duplicate.name.contains("Review"));
    assert_eq!(duplicate.text, original.text);
    assert_eq!(duplicate.description, original.description);
}

#[test]
fn duplicating_twice_gives_each_copy_its_own_name() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_prompt("Review", "text").unwrap();

    let a = s.duplicate_prompt(id).unwrap().unwrap();
    let b = s.duplicate_prompt(id).unwrap().unwrap();

    let names: Vec<&str> = s.prompts.iter().map(|p| p.name.as_str()).collect();
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 3, "no two rows share a name: {names:?}");
    // The newest copy sits directly after the original, the older one after it.
    assert_eq!(s.prompts.iter().map(|p| p.id).collect::<Vec<_>>(),
               [id, b, a]);
}

#[test]
fn duplicating_an_unknown_prompt_changes_and_writes_nothing() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());

    assert_eq!(s.duplicate_prompt(Uuid::new_v4()).unwrap(), None);

    assert!(s.prompts.is_empty());
    assert!(!dir.path().join(PROMPTS_FILE).exists());
}
