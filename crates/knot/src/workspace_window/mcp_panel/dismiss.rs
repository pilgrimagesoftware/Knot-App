//! Dismissing an MCP server the user does not use (#402).
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "The collapsed
//! section names what needs attention".
//!
//! An agent's account can offer connectors the user never set up - claude.ai
//! lists every connector it has, connected or not - and each reads as needing
//! authentication. Knot cannot tell one the user never meant to use from one
//! whose sign-in lapsed, so it does not guess: the user says so, once, and the
//! choice is a preference ([`knot_core::Settings::dismissed_mcp_servers`]),
//! kept by name across agents and launches. Which rows it then affects is
//! [`super::rows::is_dismissed`]'s to decide.

use gpui_kit::Context;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Dismiss the server called `name`, or bring it back, and save the
    /// choice. The section reads the preference as it draws, so the next
    /// frame shows it; a save that fails leaves the choice unmade and says so.
    pub(in crate::workspace_window) fn set_mcp_dismissed(&mut self, name: &str, dismissed: bool,
                                                         cx: &mut Context<Self>) {
        let saved = crate::settings_global::write_persisting(cx, |settings| {
            let changed = if dismissed {
                settings.dismissed_mcp_servers.insert(name.to_owned())
            }
            else {
                settings.dismissed_mcp_servers.remove(name)
            };
            if changed {
                settings.persist_preferences()?;
            }
            Ok(())
        });
        if let Err(error) = saved {
            self.error = Some(knot_core::l10n::t_with("mcp.dismiss_failed",
                                                      &[("error", &error.to_string())]));
        }
        cx.notify();
    }
}
