use uuid::Uuid;

use super::{AgentStore, EditRequest};
use crate::agent::view_mode_for;
use crate::error::{AgentError, Result};

impl AgentStore {
    pub fn edit(&mut self, id: Uuid, req: EditRequest) -> Result<()> {
        let old_folder = self.agent(id)
                             .ok_or(AgentError::NotFound(id))?
                             .folder
                             .clone();
        let mut needs_restart = false;
        let schedule_changed;
        {
            let agent = self.agent_mut(id).expect("agent checked above");
            agent.name = req.name;
            agent.avatar = req.avatar;
            // Deliberately does not set `needs_restart`: changing when an
            // agent would start says nothing about the session it already has.
            agent.activation_mode = req.activation_mode;
            // Registry metadata, for the same reason: a label is not a
            // launch parameter.
            agent.description = req.description;
            agent.capabilities = req.capabilities;
            agent.cost_tier = req.cost_tier;
            // Not a launch parameter either: it waits for the next fresh
            // session rather than interrupting this one.
            agent.startup_prompt = req.startup_prompt;
            // Neither interrupts the session: a schedule acts at its next
            // run, an idle policy at the next idle check (#404).
            schedule_changed = agent.schedule != req.schedule && req.schedule.is_some();
            agent.schedule = req.schedule;
            agent.idle_policy = req.idle_policy;
            if let Some(agent_type) = &req.agent_type
               && *agent_type != agent.agent_type
            {
                agent.agent_type = agent_type.clone();
                agent.view_mode = view_mode_for(agent_type);
                needs_restart = true;
            }
            if req.persona_changed {
                agent.persona_id = req.persona_id;
                needs_restart = true;
            }
            if let Some(folder) = &req.folder
               && *folder != old_folder
            {
                agent.folder = folder.clone();
                needs_restart = true;
            }
        }
        if let Some(new_folder) = req.folder.as_ref().filter(|folder| **folder != old_folder)
           && req.relocate_companions
        {
            let stale_companions = self.companions(id)
                                       .into_iter()
                                       .filter(|companion| {
                                           self.agent(*companion)
                                               .is_some_and(|agent| agent.folder == old_folder)
                                       })
                                       .collect::<Vec<_>>();
            for companion_id in stale_companions {
                if let Some(agent) = self.agent_mut(companion_id) {
                    agent.folder = new_folder.clone();
                }
                self.restart(companion_id)?;
            }
        }
        // A new or changed schedule runs only at occurrences after now (#404).
        if schedule_changed {
            self.seed_schedule_handled(id, jiff::Timestamp::now().as_second());
        }
        if needs_restart {
            self.restart(id)?;
        }
        Ok(())
    }
}
