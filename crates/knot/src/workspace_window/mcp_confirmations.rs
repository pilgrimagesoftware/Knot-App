//! The dialog an MCP call waits on before it runs a command the user did not
//! write, or puts a worktree outside its repository's parent
//! (`openspec/changes/mcp-auth/design.md` D9).
//!
//! The tool layer queues the request from the MCP server's thread; the
//! window that owns the requesting agent claims it on its repaint poll and
//! shows one dialog per request. Allow answers yes; Deny, Escape, or the
//! dialog going away unanswered answers no, and the call fails with nothing
//! created. The call's own timeout answers no if nobody does.

use std::sync::Arc;

use gpui_kit::Context;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use knot_mcp_tools::{ConfirmationRequest, PendingConfirmation};
use parking_lot::Mutex;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Opens a dialog for each queued request from an agent in this
    /// workspace. Requests from another workspace's agents stay queued for
    /// that window; one from an agent no window knows is claimed by the
    /// first window to look, so it is never left to time out unseen.
    pub(in crate::workspace_window) fn show_mcp_confirmations(&mut self, cx: &mut Context<Self>) {
        let Some(queue) = cx.try_global::<crate::app_support::Confirmations>()
                            .map(|global| Arc::clone(&global.0))
        else {
            return;
        };
        let mine = self.workspace_agent_ids();
        // Taken out and the rest put back, so the queue's lock and the
        // store's are never held together.
        let waiting = std::mem::take(&mut *queue.lock());
        if waiting.is_empty() {
            return;
        }
        let (claimed, kept): (Vec<PendingConfirmation>, Vec<PendingConfirmation>) = {
            let store = self.store.lock();
            waiting.into_iter().partition(|pending| {
                                   let requester = pending.request.requester;
                                   mine.contains(&requester) || store.agent(requester).is_none()
                               })
        };
        queue.lock().extend(kept);
        let window = self.window_handle;
        for pending in claimed {
            // Deferred: this runs inside the view's update, and opening a
            // dialog updates the window.
            cx.defer(move |app| {
                  let _ = window.update(app, |_, window, cx| open(pending, window, cx));
              });
        }
    }
}

/// The dialog's text for `request`.
///
/// Every value from the call is shown through [`visible`], so a command or
/// path cannot hide part of itself from the person approving it.
pub(in crate::workspace_window) fn body(request: &ConfirmationRequest) -> String {
    let folder = visible(&request.folder.display().to_string());
    let agent = visible(&request.requester_name);
    match &request.command {
        Some(command) => {
            let command = visible(command);
            let mut body = knot_core::l10n::t_with("mcp_confirm.command_body",
                                                   &[("agent", &agent),
                                                     ("command", &command),
                                                     ("folder", &folder)]).to_string();
            if request.distant_worktree {
                body.push_str("\n\n");
                body.push_str(&knot_core::l10n::t("mcp_confirm.distant_note"));
            }
            body
        }
        None => knot_core::l10n::t_with("mcp_confirm.worktree_body",
                                        &[("agent", &agent), ("folder", &folder)]).to_string(),
    }
}

/// `text` with every character that could hide or rearrange what is shown
/// written out as `\u{..}`: C0 controls and DEL (a carriage return or a run
/// of backspaces can overwrite the visible command), C1 controls, and the
/// Unicode format characters that reorder or hide text - bidirectional
/// overrides and isolates, zero-width characters, and the byte-order mark
/// (security review of #516).
pub(in crate::workspace_window) fn visible(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if hides_text(character) {
            out.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        }
        else {
            out.push(character);
        }
    }
    out
}

fn hides_text(character: char) -> bool {
    character.is_control()
    || matches!(character,
                '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{2069}'
                | '\u{feff}')
}

/// Shows `pending`'s dialog in `window`. The answer is taken once: the
/// dialog's builder runs on every frame, so the request sits behind a shared
/// slot that the first of Allow or Deny empties.
fn open(pending: PendingConfirmation, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
    let text = body(&pending.request);
    let slot = Arc::new(Mutex::new(Some(pending)));
    window.open_alert_dialog(cx, move |alert, _, _| {
              let (allow, deny) = (Arc::clone(&slot), Arc::clone(&slot));
              alert.title(knot_core::l10n::t("mcp_confirm.title"))
                   .description(text.clone())
                   .button_props(DialogButtonProps::default()
                       .ok_text(knot_core::l10n::t("mcp_confirm.allow"))
                       .ok_variant(ButtonVariant::Danger)
                       .cancel_text(knot_core::l10n::t("mcp_confirm.deny"))
                       .show_cancel(true))
                   .on_ok(move |_, _, _| {
                       answer(&allow, true);
                       true
                   })
                   .on_cancel(move |_, _, _| {
                       answer(&deny, false);
                       true
                   })
          });
}

/// Answers the request in `slot`, if it is still unanswered.
fn answer(slot: &Mutex<Option<PendingConfirmation>>, confirmed: bool) {
    if let Some(pending) = slot.lock().take() {
        pending.answer(confirmed);
    }
}

#[cfg(test)]
mod tests;
