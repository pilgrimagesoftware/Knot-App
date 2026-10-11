//! The saved-locations collection: its document and the helpers that edit
//! it.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/
//! settings-persistence/spec.md`.
//!
//! Saved locations are objects the user creates and names - durable data,
//! not a preference - so they get a document of their own,
//! `library-locations.json`, written through the same atomic collection
//! writer as personas and prompts. A store without the document loads with
//! no saved locations, same as a store that predates the prompt library.

use knot_library::Location;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Settings, documents};
use crate::error::{Error, Result};

/// A location the user saved, beside the built-in Knot-Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryLocation {
    pub id:       Uuid,
    pub name:     String,
    pub location: Location,
}

impl Settings {
    /// Write the saved-locations document.
    ///
    /// Skipped while there are no saved locations and no document exists,
    /// matching the prompt library's rule: an installation that never saved
    /// a location grows no file for it.
    pub(crate) fn persist_library_locations(&self) -> Result<()> {
        let path = self.resolved_paths()?.library_locations();
        if self.library_locations.is_empty() && !path.exists() {
            return Ok(());
        }
        documents::write_collection(&path, &self.library_locations)
    }

    /// Save a new location, named by the user. Refused, leaving the saved
    /// locations unchanged, when the name is blank or the location's own
    /// form check fails (`owner/repo`, `https://`, an existing folder).
    pub fn add_library_location(&mut self, name: impl Into<String>, location: Location)
                                -> Result<Uuid> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(blank_location_name());
        }
        location.validate()
                .map_err(|err| Error::Config(err.to_string()))?;
        let id = Uuid::new_v4();
        self.library_locations
            .push(LibraryLocation { id, name, location });
        self.persist_library_locations()?;
        Ok(id)
    }

    /// Rewrite a saved location's name, keeping its id and location. Refused
    /// when the name is blank; an unknown id changes nothing.
    pub fn rename_library_location(&mut self, id: Uuid, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(blank_location_name());
        }
        let Some(saved) = self.library_locations.iter_mut().find(|l| l.id == id)
        else {
            return Ok(());
        };
        saved.name = name;
        self.persist_library_locations()
    }

    /// Remove a saved location. Anything already imported from it is kept -
    /// removing a location is about where to look next, not about what is
    /// already in Knot. An unknown id changes nothing and writes nothing.
    pub fn remove_library_location(&mut self, id: Uuid) -> Result<()> {
        let before = self.library_locations.len();
        self.library_locations.retain(|l| l.id != id);
        if self.library_locations.len() == before {
            return Ok(());
        }
        self.persist_library_locations()
    }
}

fn blank_location_name() -> Error {
    Error::Config("a saved location needs a name".to_string())
}

#[cfg(test)]
#[path = "library_locations/tests.rs"]
mod tests;
