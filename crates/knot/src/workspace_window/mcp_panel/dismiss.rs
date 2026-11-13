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
use knot_mcp_probe::Inventory;

use super::rows::clear_connected;
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
        self.report_dismissal_save(saved);
        cx.notify();
    }

    /// Clear the dismissal of every server one of `inventories` reports
    /// connected (#417), saving only when a name was cleared. The probe's
    /// drain calls this, and its repaint draws the result.
    pub(in crate::workspace_window) fn clear_connected_dismissals(&mut self,
                                                                  inventories: &[Inventory],
                                                                  cx: &gpui_kit::App) {
        // Checked on a copy first, so a tick that clears nothing writes
        // nothing; then applied to the settings as the write finds them.
        let clear_all = |dismissed: &mut std::collections::BTreeSet<String>| {
            // Every inventory, always: `any` would stop at the first that
            // cleared something and leave the others' connected servers
            // dismissed. `|=` on a bool does not short-circuit.
            let mut changed = false;
            for inventory in inventories {
                changed |= clear_connected(dismissed, inventory);
            }
            changed
        };
        let mut probe = crate::settings_global::read(cx).dismissed_mcp_servers
                                                        .clone();
        if !clear_all(&mut probe) {
            return;
        }
        let saved = crate::settings_global::write_persisting(cx, |settings| {
            if clear_all(&mut settings.dismissed_mcp_servers) {
                settings.persist_preferences()?;
            }
            Ok(())
        });
        self.report_dismissal_save(saved);
    }

    /// A failed save leaves the choice unmade; say so where the window shows
    /// errors.
    fn report_dismissal_save(&mut self, saved: knot_core::Result<()>) {
        if let Err(error) = saved {
            self.error = Some(knot_core::l10n::t_with("mcp.dismiss_failed",
                                                      &[("error", &error.to_string())]));
        }
    }
}
