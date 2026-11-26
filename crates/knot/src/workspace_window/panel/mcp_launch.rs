//! The MCP side of a panel launch (`openspec/changes/mcp-auth`, D3): what an
//! adapter's environment carries for its hook scripts, and the notice for a
//! configuration that puts the agent's credential on a command line.

use uuid::Uuid;

/// The environment an adapter's hook scripts read (D3, D6): the server's
/// base URL, the agent's id and its hook credential - the only agent
/// credential placed in an environment, and one the server refuses on
/// `/mcp`.
pub(in crate::workspace_window) fn hook_environment(agent: Uuid,
                                                    pair: &knot_mcp::AgentCredentials, port: u16)
                                                    -> Vec<(String, String)> {
    vec![(crate::consts::MCP_HOOK_URL_ENV.to_owned(), knot_agent_launch::server_url_for_port(port)),
         (crate::consts::MCP_HOOK_TOKEN_ENV.to_owned(), pair.hooks.expose().to_owned()),
         (crate::consts::MCP_HOOK_AGENT_ENV.to_owned(), agent.to_string())]
}

/// Whether a launch of `agent_type` is given Knot's MCP server - and with it
/// an issued credential and a registration turn. Only with the server on,
/// and only for an adapter checked to send the server entry's headers
/// (`knot_core::agent_type::sends_mcp_headers`, D3): any other would be
/// refused on every request, so it gets neither the entry nor a turn asking
/// it to register through it.
pub(super) fn wires_mcp(mcp_enabled: bool, agent_type: &str) -> bool {
    mcp_enabled && knot_core::agent_type::sends_mcp_headers(agent_type)
}

/// The first turn a fresh session opens on: `lead` (standing instructions
/// that could not go through the adapter's system channel), the request to
/// register with the knot when Knot's server is `wired`, and the handoff
/// note. `None` when there is nothing to say.
pub(super) fn first_turn(wired: bool, lead: Option<&str>, handoff: Option<&str>) -> Option<String> {
    let opening = if wired {
        knot_agent_launch::acp_registration_prompt(false, lead)
    }
    else {
        lead.map(str::to_owned)
    };
    match (opening, handoff) {
        (Some(opening), Some(note)) => Some(format!("{opening} {note}")),
        (Some(opening), None) => Some(opening),
        (None, Some(note)) => Some(note.to_owned()),
        (None, None) => None,
    }
}

/// The conversation notice for a launch of `agent_type` with `options` that
/// makes its adapter pass the MCP servers - and the agent's tool credential
/// with them - on its CLI's command line, or `None` when it does not. Also
/// logged, naming what selected that path but never the credential. The
/// user's options are left as they are (`agent-launch-command`
/// "Adapter-side credential exposure is surfaced").
pub(super) fn command_line_notice(agent: Uuid, agent_type: &str,
                                  options: &knot_agent_launch::AdapterOptions)
                                  -> Option<String> {
    let delivery = knot_agent_launch::command_line_delivery(agent_type, options, &|name| {
        std::env::var_os(name).is_some()
    })?;
    let trigger = match delivery {
        knot_agent_launch::CommandLineDelivery::Executable => "CLAUDE_CODE_EXECUTABLE".to_owned(),
        knot_agent_launch::CommandLineDelivery::Option(name) => format!("--{name}"),
    };
    eprintln!("agent {agent}: its MCP credential travels on the Claude CLI's command line ({trigger})");
    Some(knot_core::l10n::t_with("panel.mcp_command_line_notice",
                                 &[("trigger", &trigger)]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B3's review of #521: removing the header gate must fail a test.
    #[test]
    fn only_checked_adapters_are_wired_and_only_with_the_server_on() {
        for agent_type in ["claude", "codex", "antigravity", "copilot"] {
            assert!(wires_mcp(true, agent_type), "{agent_type}");
            assert!(!wires_mcp(false, agent_type),
                    "{agent_type} with the server off");
        }
        for agent_type in ["gemini", "opencode", "shell", "custom1"] {
            assert!(!wires_mcp(true, agent_type), "{agent_type}");
        }
    }

    #[test]
    fn an_unwired_agent_is_not_asked_to_register() {
        let register = knot_agent_launch::registration_user_prompt();
        let wired = first_turn(true, None, None).expect("a registration turn");
        assert!(wired.contains(register));

        assert_eq!(first_turn(false, None, None), None);
        let kept = first_turn(false, Some("instructions"), Some("handoff")).expect("kept");
        assert_eq!(kept, "instructions handoff");
        assert!(!kept.contains(register));
        assert_eq!(first_turn(true, Some("lead"), Some("note")),
                   Some(format!("lead {register} note")));
    }

    #[test]
    fn a_settings_option_gets_the_notice_and_the_options_are_unchanged() {
        let options = knot_agent_launch::adapter_options("claude", "--settings ./s.json");
        let before = options.clone();

        let notice = command_line_notice(Uuid::new_v4(), "claude", &options).expect("a notice");

        assert!(notice.contains("--settings"),
                "the trigger is named: {notice}");
        assert_ne!(notice, "panel.mcp_command_line_notice", "the key resolves");
        assert_eq!(options, before);
    }

    #[test]
    fn ordinary_options_get_no_notice() {
        // Unless the test runner itself sets the variable.
        if std::env::var_os("CLAUDE_CODE_EXECUTABLE").is_some() {
            return;
        }
        let options = knot_agent_launch::adapter_options("claude", "--model opus");
        assert_eq!(command_line_notice(Uuid::new_v4(), "claude", &options),
                   None);
    }
}
