//! Turning Knot-Library index items into personas and prompts.
//!
//! Contract: `openspec/changes/import-from-knot-library/specs/data-import/spec.md`.
//!
//! This module knows nothing about fetching or verifying bytes -
//! `knot-library` does that, off-thread, for only the items the user
//! selected. It turns an already-verified body into a record and applies
//! the identity rule: a record counts as already held when Knot has a
//! persona or prompt with the item's id, in any state.

use knot_library::{IndexItem, ItemKind};
use uuid::Uuid;

use super::result::{ImportResult, Unreadable, UnreadableReason};
use crate::settings::{Persona, PersonaState, PersonaType, Prompt, Settings, SharedSettings};

#[cfg(test)]
mod tests;

/// Where an index item stands against what Knot already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    /// Nothing in Knot has this id; the item can be imported.
    Importable,
    /// A live record already has this id.
    AlreadyHeld,
    /// The only record with this id is a soft-deleted built-in persona.
    /// Reviving it would overwrite a tombstone, which `data-import` forbids;
    /// the user wants the Personas window's Restore Defaults instead.
    DeletedBuiltin,
}

/// Classifies `item` against `settings`, or `None` for a kind this Knot
/// doesn't import (`data-import` - "skip kinds it doesn't recognise").
#[must_use]
pub fn classify(settings: &Settings, item: &IndexItem) -> Option<ItemStatus> {
    let id = Uuid::parse_str(&item.id).ok()?;
    match item.kind {
        ItemKind::Persona => Some(match settings.personas.iter().find(|p| p.id == id) {
            Some(persona) if persona.state == PersonaState::Deleted => ItemStatus::DeletedBuiltin,
            Some(_) => ItemStatus::AlreadyHeld,
            None => ItemStatus::Importable,
        }),
        ItemKind::Prompt => Some(if settings.prompts.iter().any(|p| p.id == id) {
            ItemStatus::AlreadyHeld
        }
        else {
            ItemStatus::Importable
        }),
        ItemKind::Other(_) => None,
    }
}

/// Imports the selected items, each already fetched and verified by the
/// caller (`Ok(body)`) or already known to have failed (`Err`, reported as
/// unreadable by title - `data-import`'s partial-failure rule).
///
/// An item the caller still passes that is not [`ItemStatus::Importable`] -
/// another selection landed first, say - is skipped rather than imported,
/// since `data-import` never overwrites an existing record.
///
/// Only the documents of the kinds actually imported are written: importing
/// personas alone leaves the prompt library untouched, and the reverse.
pub fn import_items(settings: &SharedSettings, items: &[(IndexItem, Result<String, String>)])
                    -> crate::Result<ImportResult> {
    settings.write_persisting(|settings| {
        let mut result = ImportResult::default();
        for (item, body) in items {
            match classify(settings, item) {
                Some(ItemStatus::Importable) => {}
                _ => {
                    result.skipped.push(item.title.clone());
                    continue;
                }
            }
            let Ok(body) = body
            else {
                result.unreadable
                      .push(Unreadable::new(item.title.clone(), UnreadableReason::Unreadable));
                continue;
            };
            let Ok(id) = Uuid::parse_str(&item.id)
            else {
                result.unreadable
                      .push(Unreadable::new(item.title.clone(), UnreadableReason::Unreadable));
                continue;
            };
            let inserted = match item.kind {
                ItemKind::Persona => settings.insert_persona(Persona { id,
                                                                       name: item.title.clone(),
                                                                       instructions:
                                                                           body.clone(),
                                                                       persona_type:
                                                                           PersonaType::User,
                                                                       state:
                                                                           PersonaState::Enabled }),
                ItemKind::Prompt => settings.insert_prompt(Prompt { id,
                                                                    name: item.title.clone(),
                                                                    text: body.clone() }),
                ItemKind::Other(_) => continue,
            };
            match inserted {
                Ok(()) => result.added.push(item.title.clone()),
                Err(_) => result.unreadable
                                .push(Unreadable::new(item.title.clone(),
                                                       UnreadableReason::Unreadable)),
            }
        }
        Ok(result)
    })
}
