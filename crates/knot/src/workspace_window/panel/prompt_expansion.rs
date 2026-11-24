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
//!
//! A prompt with user inputs (`{{user_input:...}}`) opens the inputs sheet
//! first. Everything above is captured at the choice and carried across the
//! sheet unchanged, so the same snapshot rule drops an Insert when the
//! composer was edited while the sheet was open.

use std::ops::Range;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;
use crate::workspace_window::panel::prompt_inputs_sheet::SheetOutcome;
use crate::workspace_window::panel::prompt_inputs_sheet::SheetRequest;
use crate::workspace_window::panel::prompt_inputs_sheet::open_prompt_inputs_sheet;

/// Everything captured when a prompt is chosen, carried unchanged across the
/// inputs sheet so the expansion that finally runs sees the clipboard and
/// buffer as they were at the choice.
struct PendingExpansion {
    text:     String,
    source:   knot_agent_launch::ContextSource,
    snapshot: String,
    target:   Range<usize>,
    input:    Entity<PanelInputState>,
}

impl WorkspaceWindow {
    /// Expand library prompt `prompt` for agent `id` and write it over
    /// `target` in `input` once it resolves. `target` is empty to insert at
    /// a bare caret. A prompt with user inputs asks for them first.
    pub(in crate::workspace_window) fn expand_library_prompt(&mut self, id: Uuid, prompt: Uuid,
                                                             target: Range<usize>,
                                                             input: &Entity<PanelInputState>,
                                                             window: &mut Window,
                                                             cx: &mut Context<Self>) {
        let Some((name, text)) =
            crate::settings_global::read(cx).prompt(prompt)
                                            .map(|prompt| {
                                                (prompt.name.clone(), prompt.text.clone())
                                            })
        else {
            return;
        };
        let Some(agent) = self.store.lock().agent(id).cloned()
        else {
            return;
        };
        let inputs = knot_agent_launch::user_inputs(&text);
        // One sheet per composer: a second choice with inputs fronts the
        // open one and is dropped, keeping what was typed there.
        if !inputs.is_empty() && self.front_prompt_inputs_sheet(id, cx) {
            return;
        }
        let pending =
            PendingExpansion { source: knot_agent_launch::ContextSource { clipboard:
                                                                              clipboard_text(cx),
                                                                          ..self.prompt_context_source(&agent) },
                               snapshot: input.read(cx).value().to_string(),
                               text,
                               target,
                               input: input.clone() };
        if inputs.is_empty() {
            land_expansion(pending, window, cx);
            return;
        }
        self.ask_for_inputs(id,
                            SheetRequest { title: name,
                                           inputs },
                            pending,
                            window,
                            cx);
    }

    /// Bring `id`'s open inputs sheet to the front; `false` when it has
    /// none, or the one recorded has already gone.
    fn front_prompt_inputs_sheet(&mut self, id: Uuid, cx: &mut App) -> bool {
        let Some(handle) = self.prompt_inputs_sheets.get(&id)
        else {
            return false;
        };
        if handle.update(cx, |_, window, _| window.activate_window())
                 .is_ok()
        {
            return true;
        }
        self.prompt_inputs_sheets.remove(&id);
        false
    }

    /// Open the inputs sheet for `pending`. Insert lands the expansion with
    /// the typed values; every way the sheet closes returns focus to the
    /// composer.
    fn ask_for_inputs(&mut self, id: Uuid, request: SheetRequest, mut pending: PendingExpansion,
                      window: &mut Window, cx: &mut Context<Self>) {
        let owner = cx.entity();
        let workspace = cx.weak_entity();
        let workspace_window = window.window_handle();
        let on_close = move |outcome: SheetOutcome, cx: &mut App| {
            let _ =
                workspace_window.update(cx, |_, window, cx| {
                                    window.activate_window();
                                    let _ =
                                        workspace.update(cx, |view, cx| {
                                                     view.prompt_inputs_sheets.remove(&id);
                                                     pending.input.update(cx, |state, cx| {
                                                                      state.focus(window, cx)
                                                                  });
                                                     if let SheetOutcome::Insert(values) = outcome {
                                                         pending.source.inputs = values;
                                                         land_expansion(pending, window, cx);
                                                     }
                                                 });
                                });
        };
        let Some(handle) = open_prompt_inputs_sheet(request, on_close, cx)
        else {
            return;
        };
        // The sheet closes, inserting nothing, when its entry leaves the map
        // (the agent was torn down) or the workspace window goes.
        let _ = handle.update(cx, |sheet, window, cx| {
                          sheet.keep(cx.observe_in(&owner, window, move |_, owner, window, cx| {
                                           if !owner.read(cx).prompt_inputs_sheets.contains_key(&id)
                                           {
                                               window.remove_window();
                                           }
                                       }));
                          sheet.keep(cx.observe_release_in(&owner, window, |_, _, window, _| {
                                           window.remove_window()
                                       }));
                      });
        self.prompt_inputs_sheets.insert(id, handle);
    }
}

/// Expand `pending` on the background executor - `{{branch}}` runs `git` -
/// and write it over its target once it resolves, if the buffer still reads
/// as it did when the prompt was chosen.
fn land_expansion(pending: PendingExpansion, window: &mut Window,
                  cx: &mut Context<WorkspaceWindow>) {
    let PendingExpansion { text,
                           source,
                           snapshot,
                           target,
                           input, } = pending;
    cx.spawn_in(window, async move |this, cx| {
          let expanded = cx.background_executor()
                           .spawn(async move {
                               knot_agent_launch::expand(&text,
                                                         &knot_agent_launch::read_context(source))
                           })
                           .await;
          let _ = this.update_in(cx, |_, window, cx| {
                          apply_expansion(&input, &snapshot, target, &expanded, window, cx);
                          cx.notify();
                      });
      })
      .detach();
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
