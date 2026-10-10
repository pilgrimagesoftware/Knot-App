//! The persona roster helper that takes a caller-supplied id.
//!
//! Split from [`super`] once it grew past the size limit; the other persona
//! helpers (`add_persona`, `update_persona`, `remove_persona`, and so on,
//! which always mint a fresh id) stayed there.

use super::Settings;
use crate::error::{Error, Result};
use crate::settings::Persona;

impl Settings {
    /// Insert a persona carrying its own id - the library import's path,
    /// where identity comes from the source rather than being minted here.
    /// Refused, leaving the roster unchanged, when a persona with that id is
    /// already present (`Uuid` compares parsed values, so case doesn't
    /// matter) or its name or instructions are blank.
    pub fn insert_persona(&mut self, persona: Persona) -> Result<()> {
        if self.personas.iter().any(|p| p.id == persona.id) {
            return Err(duplicate_persona_id(persona.id));
        }
        if persona.name.trim().is_empty() || persona.instructions.trim().is_empty() {
            return Err(blank_persona());
        }
        self.personas.push(persona);
        self.persist_personas()
    }
}

fn blank_persona() -> Error {
    Error::Config("a persona needs a name and instructions".to_string())
}

fn duplicate_persona_id(id: uuid::Uuid) -> Error {
    Error::Config(format!("a persona with id {id} already exists"))
}

#[cfg(test)]
#[path = "personas/tests.rs"]
mod tests;
