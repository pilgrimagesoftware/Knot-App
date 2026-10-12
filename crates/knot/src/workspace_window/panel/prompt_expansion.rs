//! Inserting a library prompt into a composer: its text, with its variables
//! expanded for the agent whose composer it is, written over a target range:
//! the slash token for the `/` lookup, the selection or bare caret for the
//! Insert Prompt picker.
//!
//! Contract: `openspec/specs/panel-slash-commands/spec.md`, "Inserting a
//! library prompt expands its text", and
//! `openspec/specs/prompt-library/spec.md`, "The clipboard is captured when a
//! prompt is chosen".
//!
//! `{{clipboard}}` is read here, on the UI thread where gpui allows it, at
//! the moment the prompt is chosen and before the expansion leaves for the
//! background executor - so a copy made while the branch resolves does not
//! change what lands.
//!
//! Expanding `{{branch}}` runs `git`, so the expansion runs on the
//! background executor and lands through `update_in`, which wakes the window
//! itself - no flag for `repaint_poll_tick` to poll. The buffer is captured
//! when the prompt is chosen and the text lands only if the buffer still
//! reads the same: an edit made while it resolved wins, and the expansion is
//! dropped.

use std::ops::Range;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

impl WorkspaceWindow {
    /// Expand library prompt `prompt` for agent `id` and write it over
    /// `target` in `input` once it resolves. `target` is empty to insert at
    /// a bare caret.
    pub(in crate::workspace_window) fn expand_library_prompt(&mut self, id: Uuid, prompt: Uuid,
                                                             target: Range<usize>,
                                                             input: &Entity<PanelInputState>,
                                                             window: &mut Window,
                                                             cx: &mut Context<Self>) {
        let Some(text) = crate::settings_global::read(cx).prompt(prompt)
                                                         .map(|prompt| prompt.text.clone())
        else {
            return;
        };
        let Some(agent) = self.store.lock().agent(id).cloned()
        else {
            return;
        };
        let source = knot_agent_launch::ContextSource { clipboard: clipboard_text(cx),
                                                        ..self.prompt_context_source(&agent) };
        let snapshot = input.read(cx).value().to_string();
        let input = input.clone();
        cx.spawn_in(window, async move |this, cx| {
              let expanded =
                  cx.background_executor()
                    .spawn(async move {
                        knot_agent_launch::expand(&text, &knot_agent_launch::read_context(source))
                    })
                    .await;
              let _ = this.update_in(cx, |_, window, cx| {
                              apply_expansion(&input, &snapshot, target, &expanded, window, cx);
                              cx.notify();
                          });
          })
          .detach();
    }
}

/// The clipboard's text for `{{clipboard}}`: every text entry joined, or
/// empty when it holds none - an image, a file, or nothing at all.
fn clipboard_text(cx: &App) -> String {
    cx.read_from_clipboard()
      .and_then(|item| item.text())
      .unwrap_or_default()
}

/// Write `expanded` over `target` if `input` still holds `snapshot`.
fn apply_expansion(input: &Entity<PanelInputState>, snapshot: &str, target: Range<usize>,
                   expanded: &str, window: &mut Window, cx: &mut App) {
    input.update(cx, |state, cx| {
             if state.value().as_str() != snapshot {
                 return;
             }
             state.set_selected_range(target, cx);
             state.replace(expanded, window, cx);
         });
}

#[cfg(test)]
mod tests;
