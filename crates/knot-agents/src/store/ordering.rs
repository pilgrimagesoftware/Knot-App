use uuid::Uuid;

use super::AgentStore;

impl AgentStore {
    /// The number of ids directly after `owner_index` in `agent_ids` that are
    /// `owner`'s companions - the contiguous group a reorder carries as one
    /// unit, per `agent-lifecycle`'s placement rule.
    fn companion_span(&self, agent_ids: &[Uuid], owner_index: usize) -> usize {
        let owner = agent_ids[owner_index];
        agent_ids[owner_index + 1..].iter()
                                    .take_while(|id| {
                                        self.agent(**id).is_some_and(|agent| {
                                                            agent.is_companion
                                                            && agent.created_by == Some(owner)
                                                        })
                                    })
                                    .count()
    }

    /// Moves `id` and its companions into `gap`, counted in the current
    /// order of the workspace's `agent_ids`: gap `n` is just above the `n`th
    /// id, and gap `len` is after the last. A gap rather than a target index,
    /// so the end of the list is a place a group can go.
    ///
    /// The moved unit is `id` plus the ids directly after it that are its
    /// companions (`is_companion` and `created_by == Some(id)`). Returns
    /// `false` and changes nothing when `id` is itself a companion, is not in
    /// the workspace, `gap` falls inside the group (directly before one of
    /// its companions), or `gap` is within the group's own span - which
    /// would leave the order unchanged.
    pub fn move_agent_to_gap(&mut self, workspace_id: Uuid, id: Uuid, gap: usize) -> bool {
        if self.agent(id).is_some_and(|agent| agent.is_companion) {
            return false;
        }
        let Some(workspace) = self.workspaces
                                  .iter()
                                  .find(|workspace| workspace.id == workspace_id)
        else {
            return false;
        };
        let Some(source_index) = workspace.agent_ids
                                          .iter()
                                          .position(|agent_id| *agent_id == id)
        else {
            return false;
        };
        let span = self.companion_span(&workspace.agent_ids, source_index);
        let group_end = source_index + 1 + span;
        if gap > workspace.agent_ids.len() {
            return false;
        }
        // A gap directly before any companion would split an owner from its
        // group - whether that group is the one being moved or another one
        // entirely.
        if workspace.agent_ids
                    .get(gap)
                    .is_some_and(|id| self.agent(*id).is_some_and(|agent| agent.is_companion))
        {
            return false;
        }
        // Within the moved group's own span, every gap leaves the order
        // unchanged.
        if gap >= source_index && gap <= group_end {
            return false;
        }

        let Some(workspace) = self.workspaces
                                  .iter_mut()
                                  .find(|workspace| workspace.id == workspace_id)
        else {
            return false;
        };
        let group: Vec<Uuid> = workspace.agent_ids.drain(source_index..group_end).collect();
        // Everything from the group moved up `group.len()` when it was
        // removed, so a gap below it lands `group.len()` earlier.
        let insertion_index = if gap > source_index {
            gap - group.len()
        }
        else {
            gap
        };
        workspace.agent_ids
                 .splice(insertion_index..insertion_index, group);
        true
    }
}
