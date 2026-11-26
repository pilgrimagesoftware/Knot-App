//! Whether a launch makes the Claude adapter put the MCP servers - and with
//! them the agent's tool credential - on the Claude CLI's command line
//! (`openspec/changes/mcp-auth/design.md` D3, "Claude argv path").
//!
//! `claude-agent-acp` normally hands MCP servers to the CLI in its stdin
//! initialize message. It switches to a `--mcp-config` argument when
//! `CLAUDE_CODE_EXECUTABLE` is set in its environment, or when the session's
//! extra arguments name `settings` or `sdk-url`. Knot does not change the
//! user's options to avoid that; it says so instead.

use serde_json::Value;

use crate::agent_options::AdapterOptions;
use crate::consts::{CLAUDE_ARGV_DELIVERY_ENV, CLAUDE_ARGV_DELIVERY_OPTIONS};

/// Why this launch puts the MCP servers on the Claude CLI's command line,
/// or `None` when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLineDelivery {
    /// `CLAUDE_CODE_EXECUTABLE` is in the adapter's environment.
    Executable,
    /// A user option named `settings` or `sdk-url`.
    Option(&'static str),
}

/// What makes `agent_type`'s adapter, launched with `options` and an
/// environment in which `env_has` says which variables are set, deliver MCP
/// servers on its CLI's command line. Only Claude's adapter does this.
pub fn command_line_delivery(agent_type: &str, options: &AdapterOptions,
                             env_has: &dyn Fn(&str) -> bool)
                             -> Option<CommandLineDelivery> {
    if agent_type != "claude" {
        return None;
    }
    if env_has(CLAUDE_ARGV_DELIVERY_ENV) {
        return Some(CommandLineDelivery::Executable);
    }
    let extra_args = options.session_meta
                            .as_ref()
                            .and_then(|meta| meta.pointer("/claudeCode/options/extraArgs"))
                            .and_then(Value::as_object)?;
    CLAUDE_ARGV_DELIVERY_OPTIONS.iter()
                                .find(|name| extra_args.contains_key(**name))
                                .map(|name| CommandLineDelivery::Option(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter_options;

    fn none_set(_: &str) -> bool {
        false
    }

    #[test]
    fn the_executable_variable_selects_command_line_delivery() {
        let options = adapter_options("claude", "");
        assert_eq!(command_line_delivery("claude", &options, &|name| {
                       name == "CLAUDE_CODE_EXECUTABLE"
                   }),
                   Some(CommandLineDelivery::Executable));
    }

    #[test]
    fn a_settings_or_sdk_url_option_selects_it() {
        for (text, name) in [("--settings ./s.json", "settings"),
                             ("--sdk-url=http://localhost:1", "sdk-url")]
        {
            let options = adapter_options("claude", text);
            assert_eq!(command_line_delivery("claude", &options, &none_set),
                       Some(CommandLineDelivery::Option(name)),
                       "{text}");
        }
    }

    #[test]
    fn other_options_and_other_agents_do_not() {
        let options = adapter_options("claude", "--verbose --model opus");
        assert_eq!(command_line_delivery("claude", &options, &none_set), None);
        let codex = adapter_options("codex", "--settings x");
        assert_eq!(command_line_delivery("codex", &codex, &|_| true), None);
    }
}
