//! Pulling a queued prompt back into the composer to edit it, with the
//! confirmation when that would replace text the user already typed.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
use uuid::Uuid;

use super::composer::PanelInputState;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;

impl WorkspaceWindow {
    /// Returns a queued message to the composer so the user can change it.
    ///
    /// The entry leaves the queue and does not hold its place: edited text
    /// is sent as a new prompt, behind whatever is still waiting. Text the
    /// user already typed is the only thing this can destroy, so replacing
    /// it asks first.
    pub(in crate::workspace_window) fn edit_queued_prompt(&mut self, id: Uuid, prompt_id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        if !prompt_queue::needs_replace_confirmation(&input.read(cx).value()) {
            self.take_queued_prompt_into(&input, id, prompt_id, window, cx);
            return;
        }
        let view = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let view = view.clone();
                  let input = input.clone();
                  alert.title(knot_core::l10n::t("panel.replace_composer_title"))
                       .description(knot_core::l10n::t("panel.replace_composer_body"))
                       .confirm()
                       .on_ok(move |_, window, app| {
                           view.update(app, |view, cx| {
                                   view.take_queued_prompt_into(&input, id, prompt_id, window, cx);
                               });
                           true
                       })
              });
    }

    /// Moves `prompt_id` out of `id`'s queue and into `input`, focused and
    /// ready to edit.
    ///
    /// A message already in flight is left where it is, so a delivery that
    /// starts between the click and the confirmation cannot be pulled back
    /// out from under the agent.
    pub(in crate::workspace_window) fn take_queued_prompt_into(&mut self,
                                                               input: &Entity<PanelInputState>,
                                                               id: Uuid, prompt_id: Uuid,
                                                               window: &mut Window,
                                                               cx: &mut Context<Self>) {
        let Some(queue) = self.panel_prompt_queues.get_mut(&id)
        else {
            return;
        };
        let Some(text) = prompt_queue::take(queue, prompt_id)
        else {
            return;
        };
        cx.update_entity(input, |state, cx| {
              state.set_value(text, window, cx);
              state.focus(window, cx);
          });
        cx.notify();
    }
}
