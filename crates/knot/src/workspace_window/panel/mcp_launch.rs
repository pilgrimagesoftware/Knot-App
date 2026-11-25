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
