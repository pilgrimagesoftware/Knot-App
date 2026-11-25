//! The persona roster: the shipped defaults, and the helpers that list, add,
//! edit, remove and restore personas.
//!
//! Every mutating helper writes `personas.json` through
//! [`Settings::persist_personas`]; none touches another document.

use uuid::Uuid;

use super::Settings;
use crate::consts::DEFAULT_PERSONAS;
use crate::error::{Error, Result};
use crate::settings::{Persona, PersonaState, PersonaType};

impl Settings {
    /// Personas excluding soft-deleted ones, sorted case-insensitively by name.
    pub fn active_personas(&self) -> Vec<&Persona> {
        let mut personas: Vec<&Persona> = self.personas
                                              .iter()
                                              .filter(|p| p.state != PersonaState::Deleted)
                                              .collect();
        personas.sort_by_key(|p| p.name.to_lowercase());
        personas
    }

    /// Install any shipped default persona whose id is not already present
    /// (deleted entries count as present, so a removed default stays removed).
    pub fn install_default_personas(&mut self) -> Result<()> {
        let mut changed = false;
        for persona in default_personas() {
            if self.personas.iter().any(|p| p.id == persona.id) {
                continue;
            }
            self.personas.push(persona);
            changed = true;
        }
        if changed {
            self.persist_personas()?;
        }
        Ok(())
    }

    /// Add a new user persona, enabled by default.
    pub fn add_persona(&mut self, name: impl Into<String>, instructions: impl Into<String>)
                       -> Result<&Persona> {
        let persona = Persona { id:           Uuid::new_v4(),
                                name:         name.into(),
                                instructions: instructions.into(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, };
        self.personas.push(persona);
        self.persist_personas()?;
        Ok(self.personas.last().expect("just pushed"))
    }

    /// Rewrite name/instructions for an existing persona of any type. A no-op
    /// if `id` is not present.
    pub fn update_persona(&mut self, id: Uuid, name: impl Into<String>,
                          instructions: impl Into<String>)
                          -> Result<()> {
        let Some(persona) = self.personas.iter_mut().find(|p| p.id == id)
        else {
            return Ok(());
        };
        persona.name = name.into();
        persona.instructions = instructions.into();
        self.persist_personas()
    }

    /// Look up a persona by id, restricted to the active (non-deleted) list.
    pub fn persona(&self, id: Uuid) -> Option<&Persona> {
        self.active_personas().into_iter().find(|p| p.id == id)
    }

    /// The persona an agent is assigned, from its `persona_id`. Takes the
    /// field rather than an id so a launch path cannot hand it the agent's
    /// own id, which is what dropped every persona from every launch (#519).
    pub fn persona_for(&self, persona_id: Option<Uuid>) -> Option<&Persona> {
        persona_id.and_then(|id| self.persona(id))
    }

    /// Remove a persona: soft delete (state becomes `deleted`, record kept)
    /// for a system persona, hard delete (record removed) for a user persona.
    /// A no-op if `id` is not present.
    pub fn remove_persona(&mut self, id: Uuid) -> Result<()> {
        let Some(index) = self.personas.iter().position(|p| p.id == id)
        else {
            return Ok(());
        };
        if self.personas[index].persona_type == PersonaType::System {
            self.personas[index].state = PersonaState::Deleted;
        }
        else {
            self.personas.remove(index);
        }
        self.persist_personas()
    }

    /// Reset every shipped default persona already present (matched by id,
    /// including soft-deleted ones) to its shipped name, instructions, type,
    /// and state; append any shipped default that is entirely missing. User
    /// personas are untouched.
    pub fn restore_default_personas(&mut self) -> Result<()> {
        for default in default_personas() {
            match self.personas.iter_mut().find(|p| p.id == default.id) {
                Some(existing) => *existing = default,
                None => self.personas.push(default),
            }
        }
        self.persist_personas()
    }

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

/// The six personas shipped with the app, keyed by fixed ids.
fn default_personas() -> Vec<Persona> {
    DEFAULT_PERSONAS.iter()
                    .map(|(id, name, instructions)| {
                        Persona {
            id: Uuid::parse_str(id).expect("default persona id is a valid uuid"),
            name: (*name).to_string(),
            instructions: (*instructions).to_string(),
            persona_type: PersonaType::System,
            state: PersonaState::Enabled,
        }
                    })
                    .collect()
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
