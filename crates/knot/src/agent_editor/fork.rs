//! What Fork Agent pre-fills the editor with: the source's whole setup and
//! its conversation (#38).

use super::AgentPrefill;

impl AgentPrefill {
    /// A fork of `source`: set up the way `source` is and continuing its
    /// session, under a name that says it is a fork.
    ///
    /// `Agent` is destructured without `..`, as `AgentStore::duplicate`
    /// does, so a field added to it fails to compile here until someone
    /// decides what a fork does with it. That is how this came to drop the
    /// description, capabilities, cost tier, activation mode and session
    /// setup: they were added after Fork, and nothing made anyone look.
    pub(crate) fn fork_of(source: &knot_agents::Agent) -> Self {
        let knot_agents::Agent { // The fork's own identity.
                                 id: _,
                                 name,
                                 // Set up the same way.
                                 avatar,
                                 folder,
                                 agent_type,
                                 shell_command,
                                 persona_id,
                                 activation_mode,
                                 description,
                                 capabilities,
                                 cost_tier,
                                 session_config,
                                 startup_prompt,
                                 // Follows from `agent_type` in `create`.
                                 view_mode: _,
                                 // Not copied: only an owner can be forked, and
                                 // the fork belongs to no one.
                                 created_by: _,
                                 is_companion: _,
                                 // The conversation the fork continues.
                                 session_id,
                                 // Runtime state, and the source's own resume
                                 // bookkeeping: `fork_session` sets the fork's
                                 // from `session_id`.
                                 activated: _,
                                 state: _,
                                 idle_since: _,
                                 status_text: _,
                                 is_registered: _,
                                 is_pending_start: _,
                                 terminal_title: _,
                                 restart_token: _,
                                 resume_session_id: _,
                                 fork_session: _,
                                 acp_session_id: _,
                                 metadata: _,
                                 markdown_file: _,
                                 markdown_maximized: _,
                                 markdown_history: _,
                                 mermaid_source: _,
                                 mermaid_title: _,
                                 checklist: _, } = source;
        Self { name:            Some(format!("{name} (fork)")),
               avatar:          Some(avatar.clone()),
               folder:          Some(folder.clone()),
               agent_type:      Some(agent_type.clone()),
               persona_id:      *persona_id,
               startup_prompt:  startup_prompt.clone(),
               created_by:      None,
               is_companion:    false,
               session_id:      session_id.clone(),
               shell_command:   shell_command.clone(),
               description:     Some(description.clone()),
               capabilities:    Some(capabilities.clone()),
               cost_tier:       Some(*cost_tier),
               activation_mode: Some(*activation_mode),
               session_config:  session_config.clone(), }
    }
}

#[cfg(test)]
mod tests;
