//! The checklist section (#301): the questions and checkbox items an agent
//! put in front of the user with `show-checklist`, worked through in place
//! and sent back as one message.
//!
//! What the user ticks and types is written to the agent's checklist in the
//! store as it happens, so `get-checklist` reads it without the user sending
//! anything. Send delivers the whole list the way the markdown section's
//! Approve does: a prompt for a panel agent, typed at the prompt for a
//! terminal one.

use std::collections::BTreeMap;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::base::v_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::div;
use knot_agents::Checklist;
use knot_agents::ChecklistItemKind;
use uuid::Uuid;

use super::section::Section;
use super::section::SectionChrome;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// One agent's answer fields, by item id, each with the subscription that
/// writes its edits back to the store. Dropped with the checklist.
#[derive(Default)]
pub(in crate::workspace_window) struct ChecklistInputs {
    pub(super) fields: BTreeMap<String, (Entity<InputState>, Subscription)>,
}

/// An element id unique to one item of one agent's checklist.
fn item_id(prefix: &'static str, agent: Uuid, index: usize) -> gpui_kit::ElementId {
    gpui_kit::ElementId::Name(format!("{prefix}-{agent}-{index}").into())
}

impl WorkspaceWindow {
    /// The checklist section for `id`.
    pub(in crate::workspace_window) fn render_checklist_pane(&mut self, id: Uuid,
                                                             checklist: &Checklist,
                                                             chrome: SectionChrome,
                                                             window: &mut Window,
                                                             cx: &mut Context<Self>)
                                                             -> gpui_kit::AnyElement {
        let title = checklist.title
                             .clone()
                             .unwrap_or_else(|| knot_core::l10n::t("artifact_panel.checklist"));
        let actions = vec![self.checklist_send_button(id, checklist.submitted, cx)];
        let header =
            self.artifact_section_header(id, Section::Checklist, &title, chrome, actions, cx);
        self.prune_checklist_inputs(id, checklist);
        let rows =
            checklist.items
                     .iter()
                     .enumerate()
                     .map(|(index, item)| match item.kind {
                         ChecklistItemKind::Check => {
                             let item_key = item.id.clone();
                             Checkbox::new(item_id("checklist-check", id, index))
                                        .label(item.text.clone())
                                        .checked(item.checked)
                                        .on_click(cx.listener(move |view, checked: &bool, _, cx| {
                                            view.store
                                                .lock()
                                                .edit_checklist(id, |list| {
                                                    list.set_checked(&item_key, *checked)
                                                });
                                            cx.notify();
                                        }))
                                        .into_any_element()
                         }
                         ChecklistItemKind::Question => {
                             let input =
                                 self.checklist_input(id, &item.id, &item.answer, window, cx);
                             v_flex().gap_1()
                                     .child(div().text_sm().child(item.text.clone()))
                                     .child(Input::new(&input).small())
                                     .into_any_element()
                         }
                     })
                     .collect::<Vec<_>>();
        v_flex().size_full()
                .child(header)
                .child(div().id(("checklist-pane", id.as_u128() as u64))
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .min_w_0()
                            .overflow_scrollbar()
                            // Per agent, as the mermaid pane's is: the
                            // wrapper otherwise keys its scroll position by
                            // call site, which every agent shares.
                            .id(("checklist-pane-scroll", id.as_u128() as u64))
                            .p_4()
                            .child(v_flex().gap_3().children(rows)))
                .into_any_element()
    }

    /// Send, or a disabled Sent once the current answers have gone.
    fn checklist_send_button(&self, id: Uuid, submitted: bool, cx: &mut Context<Self>)
                             -> gpui_kit::AnyElement {
        let (label, tooltip) = if submitted {
            ("artifact_panel.checklist_sent", "artifact_panel.checklist_sent_tooltip")
        }
        else {
            ("artifact_panel.checklist_send", "artifact_panel.checklist_send_tooltip")
        };
        Button::new(("checklist-send", id.as_u128() as u64))
            .icon(IconName::Send)
            .label(knot_core::l10n::t(label))
            .success()
            .small()
            .disabled(submitted)
            .tooltip(knot_core::l10n::t(tooltip))
            .on_click(cx.listener(move |view, _, _window, cx| view.send_checklist(id, cx)))
            .into_any_element()
    }

    /// Delivers `id`'s checklist to the agent and marks it sent.
    ///
    /// Nothing happens without a checklist: the control is drawn only beside
    /// one, but the store may have changed under a click in flight.
    pub(in crate::workspace_window) fn send_checklist(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let panel = self.is_panel_agent(id);
        // One line for a terminal: whether a newline typed at the prompt
        // submits early depends on the CLI.
        let Some(summary) = self.store.lock().edit_checklist(id, |list| {
                                                 list.submitted = true;
                                                 if panel {
                                                     list.summary()
                                                 }
                                                 else {
                                                     list.summary_line()
                                                 }
                                             })
        else {
            return;
        };
        if panel {
            self.deliver_panel_prompt(id, summary, PromptOrigin::User);
        }
        else {
            self.send_terminal_command(id, &summary, cx);
        }
        cx.notify();
    }

    /// The answer field for item `item` of `id`'s checklist, created on the
    /// first frame that draws it and seeded with `answer`.
    fn checklist_input(&mut self, id: Uuid, item: &str, answer: &str, window: &mut Window,
                       cx: &mut Context<Self>)
                       -> Entity<InputState> {
        let inputs = self.checklist_inputs.entry(id).or_default();
        if let Some((input, _)) = inputs.fields.get(item) {
            return input.clone();
        }
        let answer = answer.to_owned();
        let input = cx.new(|cx| {
                          InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("artifact_panel.checklist_answer_placeholder"))
                .default_value(answer)
                      });
        let key = item.to_owned();
        let subscription =
            cx.subscribe_in(&input,
                            window,
                            move |view: &mut Self, input, event, _, cx| {
                                if matches!(event, InputEvent::Change) {
                                    let value = input.read(cx).value().to_string();
                                    view.store
                                        .lock()
                                        .edit_checklist(id, |list| list.set_answer(&key, &value));
                                    cx.notify();
                                }
                            });
        self.checklist_inputs
            .entry(id)
            .or_default()
            .fields
            .insert(item.to_owned(), (input.clone(), subscription));
        input
    }

    /// Drops the fields of items `checklist` no longer has, so a republish
    /// that removes a question does not leave its field behind.
    fn prune_checklist_inputs(&mut self, id: Uuid, checklist: &Checklist) {
        if let Some(inputs) = self.checklist_inputs.get_mut(&id) {
            inputs.fields.retain(|item, _| {
                             checklist.item(item)
                                      .is_some_and(|item| item.kind == ChecklistItemKind::Question)
                         });
        }
    }
}

#[cfg(test)]
mod tests;
