//! The MCP tab's access section (`openspec/changes/mcp-auth`, settings-ui
//! "MCP tab manages the client token"): the client token, masked, with copy
//! and regenerate; the installation command for each client, carrying the
//! token in that client's syntax; and the requests refused in the last
//! hour.

use gpui_kit::App;
use gpui_kit::ClipboardItem;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::div;
use knot_mcp::RefusalSource;

use crate::mcp_credentials::{
    ClientToken, ClientTokenStore, McpCredentials, PlatformTokenStore, regenerate,
};
use crate::settings_window::SettingsWindow;

/// The environment variable every installation command reads the client
/// token from, so the token is never displayed in a command, never copied
/// with one, and never written to shell history. The shell expands it when
/// the command runs; Codex reads it itself each time it connects
/// (`codex mcp add --bearer-token-env-var`, codex-cli 0.172.0), so for
/// Codex it has to stay set.
pub(crate) const CLIENT_TOKEN_ENV: &str = "KNOT_MCP_CLIENT_TOKEN";

/// The note under a command that reads [`CLIENT_TOKEN_ENV`]: on macOS, the
/// line that sets it from the clipboard - so the token is pasted, never
/// typed into the shell's history; elsewhere, to set it by hand. Both say
/// what an unset variable does: the shell substitutes nothing, and the
/// client registers an empty token that Knot refuses.
pub(crate) fn token_env_hint() -> String {
    if cfg!(target_os = "macos") {
        let command = format!("export {CLIENT_TOKEN_ENV}=\"$(pbpaste)\"");
        knot_core::l10n::t_with("settings.mcp.token_env_paste", &[("command", &command)])
    }
    else {
        knot_core::l10n::t_with("settings.mcp.token_env", &[("variable", CLIENT_TOKEN_ENV)])
    }
}

impl SettingsWindow {
    /// The command to copy for registering `agent_type` against Knot's MCP
    /// server at `url`, sending the client token from [`CLIENT_TOKEN_ENV`].
    /// Ported from the Swift reference's `MCPCommandView.mcpCommandCopy`;
    /// each header flag was checked against the CLI's own `mcp add --help`
    /// (Claude Code 2.1.326, codex-cli 0.172.0, Gemini CLI 0.71.0, agy
    /// 1.5.17). Empty for a CLI with no verified command - Copilot, whose
    /// CLI has not been checked.
    pub(crate) fn mcp_install_command(agent_type: &str, url: &str) -> String {
        // Double quotes, so the shell substitutes the token when the command
        // runs; only the variable's name is in the text and the history.
        let header = format!("\"Authorization: Bearer ${CLIENT_TOKEN_ENV}\"");
        match agent_type {
            "claude" => {
                format!("claude mcp add --transport http --scope user knot {url} --header {header}")
            }
            "codex" => {
                format!("codex mcp add knot --url {url} --bearer-token-env-var {CLIENT_TOKEN_ENV}")
            }
            // Interactive: it asks for the URL and headers itself.
            "opencode" => "opencode mcp add".to_string(),
            "gemini" => {
                format!("gemini mcp add --transport http knot {url} --scope user --header {header}")
            }
            // Go's flag parsing stops at the first argument, so the flags
            // go before the name; http is inferred from the URL.
            "antigravity" => format!("agy mcp add --header {header} knot {url}"),
            _ => String::new(),
        }
    }

    /// `token` as the tab shows it: its kind prefix and last four
    /// characters, the rest hidden.
    pub(crate) fn masked_token(token: &str) -> String {
        let prefix = knot_mcp::CredentialKind::Client.prefix();
        let tail: String = token.chars()
                                .rev()
                                .take(4)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .collect();
        format!("{prefix}••••••••{tail}")
    }

    /// The refused-request lines: the hour's total, then each Knot-launched
    /// agent with refusals, by the name the roster gives it.
    pub(crate) fn mcp_refusal_lines(refusals: &[(RefusalSource, u64)],
                                    name_of: impl Fn(uuid::Uuid) -> Option<String>)
                                    -> Vec<String> {
        let total: u64 = refusals.iter().map(|(_, count)| count).sum();
        if total == 0 {
            return vec![knot_core::l10n::t("settings.mcp.refused_none").to_string()];
        }
        let mut lines = vec![knot_core::l10n::t_count("settings.mcp.refused_total", total, &[])];
        lines.extend(refusals.iter().filter_map(|(source, count)| match source {
                                       RefusalSource::Agent(id) => {
                                           let agent = name_of(*id).unwrap_or_else(|| id.to_string());
                                           Some(knot_core::l10n::t_count("settings.mcp.refused_agent",
                                                                         *count,
                                                                         &[("agent", &agent)]))
                                       }
                                       RefusalSource::UserAgent(_)
                                       | RefusalSource::Unknown
                                       | RefusalSource::Other => None,
                                   }));
        lines
    }

    /// The access group: the token row, what it is for, any problem keeping
    /// it, and the refused requests.
    pub(crate) fn render_mcp_access(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let token = cx.try_global::<ClientToken>()
                      .map(|global| global.0.clone());
        let masked = token.as_ref()
                          .map(|loaded| Self::masked_token(loaded.token.expose()))
                          .unwrap_or_default();
        let full = token.as_ref()
                        .map(|loaded| loaded.token.expose().to_owned())
                        .unwrap_or_default();
        let problem = token.and_then(|loaded| loaded.problem);
        let saved = crate::settings_global::read(cx).saved_agents.clone();
        let refusals = Self::mcp_refusal_lines(&self.last_mcp_refusals, |id| {
            saved.iter()
                 .find(|agent| agent.id == id)
                 .map(|agent| agent.name.clone())
        });
        let settings_window = cx.entity();

        let mut group = crate::controls::group(knot_core::l10n::t("settings.mcp.access"))
            .child(Self::text_row(
                knot_core::l10n::t("settings.mcp.token"),
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Self::mono_text(cx, masked).text_color(cx.theme().muted_foreground))
                    .child(
                        crate::controls::icon_button("mcp-copy-token",
                                                     "icons/copy.svg",
                                                     knot_core::l10n::t("settings.mcp.copy_token"),
                                                     false)
                            .on_click(move |_, _, app| {
                                if !full.is_empty() {
                                    app.write_to_clipboard(ClipboardItem::new_string(full.clone()));
                                }
                            }),
                    )
                    .child(
                        crate::controls::icon_button("mcp-regenerate-token",
                                                     "icons/refresh-cw.svg",
                                                     knot_core::l10n::t("settings.mcp.regenerate"),
                                                     true)
                            .on_click(move |_, window, app| {
                                confirm_regenerate(settings_window.clone(), window, app);
                            }),
                    ),
            ))
            .child(Self::hint(cx, knot_core::l10n::t("settings.mcp.token_hint")));
        if let Some(error) = problem {
            group = group.child(div().text_sm()
                                     .text_color(cx.theme().danger)
                                     .child(knot_core::l10n::t_with("settings.mcp.token_unsaved",
                                                                    &[("error", &error)])));
        }
        group.child(v_flex().gap_1()
                            .children(refusals.into_iter().map(|line| Self::hint(cx, line))))
    }
}

/// Asks before regenerating: every client registered with the current token
/// stops connecting. Confirming stores the new token, makes it the only one
/// the server accepts, and redraws the tab so the command shows it.
fn confirm_regenerate(settings_window: gpui_kit::Entity<SettingsWindow>, window: &mut Window,
                      cx: &mut App) {
    window.open_alert_dialog(cx, move |alert, _, _| {
              let settings_window = settings_window.clone();
              alert.title(knot_core::l10n::t("settings.mcp.regenerate_title"))
                   .description(knot_core::l10n::t("settings.mcp.regenerate_body"))
                   .button_props(DialogButtonProps::default()
                       .ok_text(knot_core::l10n::t("settings.mcp.regenerate_confirm"))
                       .ok_variant(ButtonVariant::Danger)
                       .show_cancel(true))
                   .on_ok(move |_, _, app| {
                       regenerate_client_token(&PlatformTokenStore, app);
                       settings_window.update(app, |_, cx| cx.notify());
                       true
                   })
          });
}

/// Regenerates the client token in `store`, makes it the registry's only
/// client token, and installs it as the [`ClientToken`] the tab shows.
pub(crate) fn regenerate_client_token(store: &dyn ClientTokenStore, cx: &mut App) {
    let credentials = McpCredentials::global(cx);
    let loaded = regenerate(store, &credentials.registry);
    cx.set_global(ClientToken(loaded));
}

#[cfg(test)]
mod tests;
