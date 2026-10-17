//! The checklist artifact's store writes (#301). The MCP tool replaces the
//! list; the panel ticks, answers, and marks it sent.

use uuid::Uuid;

use super::AgentStore;
use crate::checklist::{Checklist, ChecklistItem};
use crate::error::{AgentError, Result};

impl AgentStore {
    /// Shows `items` as `id`'s checklist, keeping the user's answers for
    /// items that are still the same item (see [`Checklist::republished`]).
    pub fn set_checklist(&mut self, id: Uuid, title: Option<String>, items: Vec<ChecklistItem>)
                         -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.checklist = Some(Checklist::republished(title, items, agent.checklist.as_ref()));
        Ok(())
    }

    /// Puts back a checklist exactly as it was, answers included - for
    /// reopening one the user closed.
    pub fn restore_checklist(&mut self, id: Uuid, checklist: Checklist) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.checklist = Some(checklist);
        Ok(())
    }

    /// Closes `id`'s checklist.
    pub fn clear_checklist(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.checklist = None;
        Ok(())
    }

    /// Applies `edit` to `id`'s checklist, if it has one, and returns what
    /// `edit` returned. `None` when the agent or its checklist is gone.
    pub fn edit_checklist<T>(&mut self, id: Uuid, edit: impl FnOnce(&mut Checklist) -> T)
                             -> Option<T> {
        self.agent_mut(id)?.checklist.as_mut().map(edit)
    }
}
