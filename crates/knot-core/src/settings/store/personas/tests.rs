use tempfile::tempdir;
use uuid::Uuid;

use super::super::Settings;
use crate::consts::DEFAULT_PERSONAS;
use crate::error::Error;
use crate::settings::{Persona, PersonaState, PersonaType};

fn agent_id() -> Uuid {
    Uuid::new_v4()
}

#[test]
fn insert_persona_keeps_the_caller_supplied_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = agent_id();
    s.insert_persona(Persona { id,
                               name: "From Library".to_string(),
                               instructions: "be helpful".to_string(),
                               persona_type: PersonaType::User,
                               state: PersonaState::Enabled })
     .unwrap();
    assert_eq!(s.personas.len(), 1);
    assert_eq!(s.persona(id).unwrap().name, "From Library");
}

/// A built-in's id is uppercase in `DEFAULT_PERSONAS`; the library publishes
/// the same ids lowercase. `Uuid` compares parsed values, so a duplicate
/// insert is refused regardless of case.
#[test]
fn insert_persona_rejects_a_duplicate_id_across_case() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.install_default_personas().unwrap();
    let (uppercase_id, _, _) = DEFAULT_PERSONAS[0];
    let lowercase_id = Uuid::parse_str(uppercase_id).unwrap()
                                                    .to_string()
                                                    .to_lowercase();

    let err = s.insert_persona(Persona { id:           Uuid::parse_str(&lowercase_id).unwrap(),
                                         name:         "Duplicate".to_string(),
                                         instructions: "be helpful".to_string(),
                                         persona_type: PersonaType::User,
                                         state:        PersonaState::Enabled, })
               .unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert_eq!(s.personas.len(), DEFAULT_PERSONAS.len());
}

#[test]
fn insert_persona_rejects_blank_fields() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let err = s.insert_persona(Persona { id:           agent_id(),
                                         name:         "   ".to_string(),
                                         instructions: "be helpful".to_string(),
                                         persona_type: PersonaType::User,
                                         state:        PersonaState::Enabled, })
               .unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert!(s.personas.is_empty());
}
