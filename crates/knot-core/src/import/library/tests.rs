//! Tests for turning Knot-Library index items into personas and prompts.
//!
//! Every case runs against a settings surface pointed at a temp file, same
//! as the subagent import's tests - `import_items` writes immediately, so a
//! test that seeds existing records and never reloads would not catch a
//! write that silently failed.

use tempfile::TempDir;

use super::*;
use crate::consts::DEFAULT_PERSONAS;
use crate::settings::{PersonaState, Settings};

fn store(dir: &TempDir) -> Settings {
    Settings::with_store_root(dir.path())
}

fn shared(dir: &TempDir) -> SharedSettings {
    SharedSettings::new(store(dir))
}

fn item(kind: ItemKind, id: &str, title: &str) -> IndexItem {
    IndexItem { kind,
               id: id.to_string(),
               slug: title.to_lowercase(),
               title: title.to_string(),
               description: "a library item".to_string(),
               tags: vec![],
               authors: vec![],
               path: format!("{title}.md"),
               size: 0,
               sha256: String::new() }
}

#[test]
fn classify_reports_a_new_id_as_importable() {
    let settings = Settings::default();
    let item = item(ItemKind::Persona, &Uuid::new_v4().to_string(), "New");
    assert_eq!(classify(&settings, &item), Some(ItemStatus::Importable));
}

#[test]
fn classify_skips_a_kind_this_knot_does_not_know() {
    let settings = Settings::default();
    let item = item(ItemKind::Other("bench".to_string()), &Uuid::new_v4().to_string(), "Bench");
    assert_eq!(classify(&settings, &item), None);
}

#[test]
fn builtin_already_held_matches_case_insensitively() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = store(&dir);
    settings.install_default_personas().unwrap();
    let (uppercase_id, _, _) = DEFAULT_PERSONAS[0];
    let lowercase_id = uppercase_id.to_lowercase();

    let item = item(ItemKind::Persona, &lowercase_id, "Kent Beck");
    assert_eq!(classify(&settings, &item), Some(ItemStatus::AlreadyHeld));
}

#[test]
fn deleted_builtin_is_not_importable() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = store(&dir);
    settings.install_default_personas().unwrap();
    let (id, _, _) = DEFAULT_PERSONAS[0];
    settings.remove_persona(Uuid::parse_str(id).unwrap()).unwrap();

    let item = item(ItemKind::Persona, id, "Kent Beck");
    assert_eq!(classify(&settings, &item), Some(ItemStatus::DeletedBuiltin));
}

#[test]
fn a_renamed_copy_of_a_builtin_is_still_held_by_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = store(&dir);
    settings.install_default_personas().unwrap();
    let (id, _, _) = DEFAULT_PERSONAS[0];
    settings.update_persona(Uuid::parse_str(id).unwrap(), "My Own Name", "my own text")
            .unwrap();

    let item = item(ItemKind::Persona, id, "Kent Beck");
    assert_eq!(classify(&settings, &item), Some(ItemStatus::AlreadyHeld));
}

#[test]
fn import_items_adds_importable_persona_and_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    let persona_item = item(ItemKind::Persona, &Uuid::new_v4().to_string(), "Research Assistant");
    let prompt_item = item(ItemKind::Prompt, &Uuid::new_v4().to_string(), "Summarize");

    let result = import_items(&shared,
                              &[(persona_item.clone(), Ok("be helpful".to_string())),
                                (prompt_item.clone(), Ok("summarize this".to_string()))])
        .unwrap();

    assert_eq!(result.added, vec!["Research Assistant", "Summarize"]);
    let settings = shared.read();
    assert!(settings.personas
                     .iter()
                     .any(|p| p.id.to_string() == persona_item.id && p.name == "Research Assistant"));
    assert!(settings.prompts.iter().any(|p| p.id.to_string() == prompt_item.id));
}

#[test]
fn import_items_skips_an_already_held_item() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    shared.write(|settings| settings.install_default_personas().unwrap());
    let (id, _, name) = DEFAULT_PERSONAS[0];

    let result = import_items(&shared, &[(item(ItemKind::Persona, id, name), Ok("text".to_string()))])
        .unwrap();

    assert_eq!(result.skipped, vec![name]);
    assert!(result.added.is_empty());
}

#[test]
fn import_items_does_not_revive_a_deleted_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    shared.write(|settings| settings.install_default_personas().unwrap());
    let (id, _, name) = DEFAULT_PERSONAS[0];
    shared.write_persisting(|settings| settings.remove_persona(Uuid::parse_str(id).unwrap()))
          .unwrap();

    let result = import_items(&shared, &[(item(ItemKind::Persona, id, name), Ok("text".to_string()))])
        .unwrap();

    assert_eq!(result.skipped, vec![name]);
    let persona = shared.read().personas.iter().find(|p| p.id == Uuid::parse_str(id).unwrap())
                         .unwrap()
                         .clone();
    assert_eq!(persona.state, PersonaState::Deleted);
}

#[test]
fn reimport_after_delete_is_importable_again() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    let id = Uuid::new_v4().to_string();
    let persona_item = item(ItemKind::Persona, &id, "Research Assistant");

    import_items(&shared, &[(persona_item.clone(), Ok("be helpful".to_string()))]).unwrap();
    shared.write_persisting(|settings| settings.remove_persona(Uuid::parse_str(&id).unwrap()))
          .unwrap();

    let result = import_items(&shared, &[(persona_item, Ok("be helpful".to_string()))]).unwrap();
    assert_eq!(result.added, vec!["Research Assistant"]);
}

#[test]
fn import_items_reports_a_fetch_failure_as_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    let bad = item(ItemKind::Persona, &Uuid::new_v4().to_string(), "Broken");

    let result = import_items(&shared, &[(bad, Err("hash mismatch".to_string()))]).unwrap();

    assert!(result.added.is_empty());
    assert_eq!(result.unreadable.len(), 1);
    assert_eq!(result.unreadable[0].name, "Broken");
}

#[test]
fn one_bad_hash_among_good_items_still_imports_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let shared = shared(&dir);
    let good = item(ItemKind::Persona, &Uuid::new_v4().to_string(), "Good");
    let bad = item(ItemKind::Persona, &Uuid::new_v4().to_string(), "Bad");

    let result = import_items(&shared,
                              &[(good, Ok("be helpful".to_string())),
                                (bad, Err("hash mismatch".to_string()))])
        .unwrap();

    assert_eq!(result.added, vec!["Good"]);
    assert_eq!(result.unreadable.len(), 1);
    assert_eq!(result.unreadable[0].name, "Bad");
}
