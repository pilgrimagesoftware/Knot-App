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
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::{InteractiveElement as _, ParentElement as _, Styled as _, div};
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
/// Every value from the call is shown through [`visible::visible`], so a
/// command or path cannot hide part of itself from the person approving it.
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

/// Shows `pending`'s dialog in `window`. The answer is taken once: the
/// dialog's builder runs on every frame, so the request sits behind a shared
/// slot that the first answer empties.
///
/// Only a click on **Allow** allows. The dialog's own Enter (its Confirm
/// action) and Escape (its Cancel action) both deny, and so does closing it
/// any other way: an agent can raise this dialog while the user is typing,
/// and a keystroke meant for something else must not approve a command
/// (review of #522).
fn open(pending: PendingConfirmation, window: &mut gpui_kit::Window, cx: &mut gpui_kit::App) {
    let text = body(&pending.request);
    let slot = Arc::new(Mutex::new(Some(pending)));
    window.open_alert_dialog(cx, move |alert, _, _| {
              let (on_enter, on_escape) = (Arc::clone(&slot), Arc::clone(&slot));
              alert.title(knot_core::l10n::t("mcp_confirm.title"))
                   .description(text.clone())
                   .footer(footer(Arc::clone(&slot)))
                   .on_ok(move |_, _, _| {
                       answer(&on_enter, false);
                       true
                   })
                   .on_cancel(move |_, _, _| {
                       answer(&on_escape, false);
                       true
                   })
          });
}

/// Deny and Allow, Deny first and the default; Allow answers only to a
/// click.
fn footer(slot: Arc<Mutex<Option<PendingConfirmation>>>) -> impl gpui_kit::IntoElement {
    let deny = Arc::clone(&slot);
    gpui_kit::base::h_flex().w_full()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("mcp-confirm-deny")
                                       .label(knot_core::l10n::t("mcp_confirm.deny"))
                                       .with_variant(ButtonVariant::Primary)
                                       .on_click(move |_, window, cx| {
                                           answer(&deny, false);
                                           window.close_dialog(cx);
                                       }))
                            .child(div().debug_selector(|| "mcp-confirm-allow".into())
                                        .child(Button::new("mcp-confirm-allow")
                                                   .label(knot_core::l10n::t("mcp_confirm.allow"))
                                                   .with_variant(ButtonVariant::Danger)
                                                   .on_click(move |_, window, cx| {
                                                       answer(&slot, true);
                                                       window.close_dialog(cx);
                                                   })))
}

/// Answers the request in `slot`, if it is still unanswered.
fn answer(slot: &Mutex<Option<PendingConfirmation>>, confirmed: bool) {
    if let Some(pending) = slot.lock().take() {
        pending.answer(confirmed);
    }
}

mod visible;

use visible::visible;

#[cfg(test)]
mod tests;
