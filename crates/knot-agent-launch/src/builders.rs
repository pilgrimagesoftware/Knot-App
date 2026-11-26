use knot_core::Settings;

use crate::adapter::AdapterConfig;
use crate::consts::AGENT_FOLDER_ENV;

#[derive(Debug, Clone, Default)]
pub struct LaunchRequest<'a> {
    pub agent_type:    &'a str,
    pub shell_command: Option<&'a str>,
}

/// The knot MCP HTTP server's own URL, for wiring into an ACP session's
/// `mcpServers` params.
pub fn mcp_url(settings: &Settings) -> String {
    mcp_url_for_port(settings.mcp_server_port)
}

/// `agent_id`'s own MCP URL: [`mcp_url`] naming the agent it is given to,
/// so the server can tell which agent each connection belongs to.
pub fn agent_mcp_url(settings: &Settings, agent_id: uuid::Uuid) -> String {
    format!("{}?{}={agent_id}",
            mcp_url(settings),
            knot_core::consts::MCP_AGENT_QUERY)
}

/// The same URL from a bare port, for callers that have no `Settings` -
/// notably the settings window, which shows the URL and the `mcp add`
/// command a user copies to register Knot with an agent by hand.
///
/// One function, because the path is the part that gets forgotten: the
/// server routes MCP at `/mcp` and answers a POST to `/` with 405, so a
/// URL built without it produces an entry that can never connect.
pub fn mcp_url_for_port(port: u16) -> String {
    format!("{}/mcp", server_url_for_port(port))
}

/// The MCP server's base URL on `port`, with no path: what hook scripts
/// append the hook routes to (`KNOT_MCP_URL`).
pub fn server_url_for_port(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

/// Builds the shell agent's terminal command: its custom command if set,
/// otherwise empty (a plain interactive shell). Non-shell agent types no
/// longer launch through this builder - see `plan_launch`.
pub fn build_agent_command(request: &LaunchRequest<'_>) -> String {
    request.shell_command.unwrap_or_default().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    Terminal(String),
    Adapter(AdapterConfig),
}

/// Decides how to launch an agent: a shell agent always gets the Terminal
/// path; every other type launches through its registered ACP adapter (see
/// `acp_adapter`) - there is no Terminal fallback for non-shell agents.
pub fn plan_launch(request: &LaunchRequest<'_>) -> LaunchPlan {
    if !knot_core::agent_type::is_shell(request.agent_type)
       && let Some(config) = crate::adapter::acp_adapter(request.agent_type)
    {
        return LaunchPlan::Adapter(config);
    }
    LaunchPlan::Terminal(build_agent_command(request))
}

/// Build the terminal initialization command: `cd` into the agent's folder,
/// clear the screen, then (for a non-empty `agent_command`) run it. The
/// leading space suppresses shell history under `ignorespace`. Shell agents
/// only - no `KNOT_AGENT_ID` is set (shell agents do not register with MCP).
///
/// The folder is read from [`AGENT_FOLDER_ENV`], which the terminal sets on
/// the PTY, and never written into the line. The line is typed into the
/// terminal, and the line editor acts on control characters (`Ctrl-U`, a
/// carriage return) before the shell parses anything, so no quoting of a
/// name typed there can keep it one word.
///
/// The PTY already starts in the folder; the `cd` stays because the user's
/// own shell reads their startup files first, and one that changes
/// directory would otherwise leave the agent somewhere else.
pub fn build_initialization_command(agent_command: &str) -> String {
    let cd = format!(" cd -- \"${AGENT_FOLDER_ENV}\" && clear");
    if agent_command.is_empty() {
        return cd;
    }
    format!("{cd} && {agent_command}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_agent_custom_command() {
        let req = LaunchRequest { agent_type:    "shell",
                                  shell_command: Some("htop"), };
        assert_eq!(build_agent_command(&req), "htop");
    }

    #[test]
    fn shell_agent_no_custom_command_is_empty() {
        let req = LaunchRequest { agent_type: "shell",
                                  ..Default::default() };
        assert_eq!(build_agent_command(&req), "");
    }

    #[test]
    fn shell_agent_wrapper_with_no_custom_command() {
        let cmd = build_initialization_command("");
        assert_eq!(cmd, " cd -- \"$KNOT_AGENT_FOLDER\" && clear");
    }

    #[test]
    fn shell_agent_wrapper_with_a_custom_command() {
        let cmd = build_initialization_command("htop");
        assert_eq!(cmd, " cd -- \"$KNOT_AGENT_FOLDER\" && clear && htop");
    }

    /// The command run by a real POSIX shell, with the folder in the
    /// environment and a name holding a quote, a substitution and a leading
    /// dash: the shell ends up in that folder and treats the name as text.
    #[cfg(unix)]
    #[test]
    fn a_folder_name_from_the_environment_is_only_a_name() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let folder = root.path().join("-it's $(touch marker) `touch marker2`");
        std::fs::create_dir(&folder).expect("the folder");
        let command = build_initialization_command("pwd");
        // `clear` needs a terminal; stand it in with a no-op.
        let output =
            std::process::Command::new("/bin/sh").arg("-c")
                                                 .arg(format!("clear() {{ :; }};{command}"))
                                                 .env(AGENT_FOLDER_ENV, &folder)
                                                 .current_dir(root.path())
                                                 .output()
                                                 .expect("sh runs");

        assert!(output.status.success(), "{output:?}");
        let pwd = String::from_utf8(output.stdout).expect("utf-8");
        assert_eq!(std::path::Path::new(pwd.trim()).canonicalize()
                                                   .expect("pwd exists"),
                   folder.canonicalize().expect("folder exists"));
        assert!(!root.path().join("marker").exists() && !folder.join("marker").exists());
        assert!(!root.path().join("marker2").exists() && !folder.join("marker2").exists());
    }

    #[test]
    fn shell_agent_always_uses_the_terminal_path() {
        let req = LaunchRequest { agent_type: "shell",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&req)));
    }

    #[test]
    fn non_shell_agent_with_a_registered_adapter_launches_via_acp() {
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        let LaunchPlan::Adapter(config) = plan
        else {
            panic!("expected an adapter launch plan, not a terminal command");
        };
        assert_eq!(config, crate::adapter::acp_adapter("claude").unwrap());
    }

    #[test]
    fn non_shell_agent_without_a_registered_adapter_has_no_launch_plan_worth_running() {
        let req = LaunchRequest { agent_type: "unknown-type",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&req)));
    }

    #[test]
    fn mcp_url_uses_the_configured_port() {
        let mut s = Settings::default();
        s.mcp_server_port = 8767;
        assert_eq!(mcp_url(&s), "http://127.0.0.1:8767/mcp");
    }

    #[test]
    fn an_agents_mcp_url_names_the_agent() {
        let mut s = Settings::default();
        s.mcp_server_port = 8767;
        let id = uuid::Uuid::new_v4();
        assert_eq!(agent_mcp_url(&s, id),
                   format!("http://127.0.0.1:8767/mcp?agent={id}"));
    }
}
