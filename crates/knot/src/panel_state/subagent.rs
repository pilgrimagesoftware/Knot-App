//! Where a subagent's updates go: inside the delegation card that launched
//! it, never into the main conversation (#476).
//!
//! claude-agent-acp forwards a subagent's `tool_call`, `tool_call_update` and
//! `agent_message_chunk` into the same session stream as the main agent's,
//! stamped `_meta.claudeCode.parentToolUseId` with the id of the `Agent` /
//! `Task` call that launched it (`stampParentToolUseId`, `sdk-to-acp.js` in
//! 0.111.0). A subagent running beside the main agent - a background one, or
//! one of several in parallel - reports while the main agent is still
//! writing, so folding those updates in as top-level entries cut the main
//! reply wherever one landed, mid-word or mid-table.
//!
//! An update whose parent card has not arrived yet is held, in order, and
//! moves into the card if it arrives later in the turn. Whatever is still
//! held when the turn or session ends is placed in the conversation rather
//! than dropped.
//!
//! The interim path until Knot takes subagents over ACP's own channel (#15).

use serde_json::Value;

use super::message::{PanelMessage, SubagentItem, ToolCallCard};
use super::state::PanelState;

/// The delegation `meta` attributes an update to, if any.
pub(super) fn parent_tool_use_id(meta: Option<&Value>) -> Option<String> {
    meta?.pointer("/claudeCode/parentToolUseId")?
         .as_str()
         .filter(|id| !id.is_empty())
         .map(str::to_owned)
}

/// `card`, or the card with `id` anywhere in the subagent activity under it.
fn card_in<'a>(card: &'a ToolCallCard, id: &str) -> Option<&'a ToolCallCard> {
    if card.id == id {
        return Some(card);
    }
    card.subagent.iter().rev().find_map(|item| match item {
                                  SubagentItem::ToolCall(child) => card_in(child, id),
                                  SubagentItem::Text(_) => None,
                              })
}

fn card_in_mut<'a>(card: &'a mut ToolCallCard, id: &str) -> Option<&'a mut ToolCallCard> {
    if card.id == id {
        return Some(card);
    }
    card.subagent.iter_mut().rev().find_map(|item| match item {
                                      SubagentItem::ToolCall(child) => card_in_mut(child, id),
                                      SubagentItem::Text(_) => None,
                                  })
}

impl PanelState {
    /// The card with `id`, at the top level or nested under a delegation.
    pub(super) fn tool_call(&self, id: &str) -> Option<&ToolCallCard> {
        let in_messages = self.messages
                              .iter()
                              .rev()
                              .find_map(|message| match message {
                                  PanelMessage::ToolCall(card) => card_in(card, id),
                                  _ => None,
                              });
        in_messages.or_else(|| {
                       self.subagent_orphans
                           .iter()
                           .find_map(|(_, item)| match item {
                               SubagentItem::ToolCall(card) => card_in(card, id),
                               SubagentItem::Text(_) => None,
                           })
                   })
    }

    /// [`Self::tool_call`], for updating. A card still held for a parent
    /// that has not arrived is found too, so its own updates are not lost.
    pub(super) fn tool_call_mut(&mut self, id: &str) -> Option<&mut ToolCallCard> {
        let Self { messages,
                   subagent_orphans,
                   .. } = self;
        let in_messages =
            messages.iter_mut().rev().find_map(|message| match message {
                                         PanelMessage::ToolCall(card) => card_in_mut(card, id),
                                         _ => None,
                                     });
        match in_messages {
            Some(card) => Some(card),
            None => subagent_orphans.iter_mut()
                                    .find_map(|(_, item)| match item {
                                        SubagentItem::ToolCall(card) => card_in_mut(card, id),
                                        SubagentItem::Text(_) => None,
                                    }),
        }
    }

    /// Puts a subagent's `item` under the card for `parent`: in it if the
    /// card exists, held for it while the turn runs, in the conversation
    /// otherwise.
    pub(super) fn deliver_to_subagent(&mut self, parent: String, item: SubagentItem) {
        let nested_id = match &item {
            SubagentItem::ToolCall(card) => Some(card.id.clone()),
            SubagentItem::Text(_) => None,
        };
        if let Some(card) = self.tool_call_mut(&parent) {
            card.push_subagent(item);
        }
        else if self.turn_active {
            self.subagent_orphans.push((parent, item));
        }
        else {
            self.place_in_conversation(item);
        }
        // A subagent's own delegation can have reported before it did.
        if let Some(id) = nested_id {
            self.adopt_subagent_items(&id);
        }
    }

    /// Moves everything held for `card_id` into that card, in arrival order.
    pub(super) fn adopt_subagent_items(&mut self, card_id: &str) {
        if !self.subagent_orphans
                .iter()
                .any(|(parent, _)| parent == card_id)
        {
            return;
        }
        let (adopted, held): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.subagent_orphans).into_iter()
                                                      .partition(|(parent, _)| parent == card_id);
        self.subagent_orphans = held;
        let Some(card) = self.tool_call_mut(card_id)
        else {
            // Not reachable through the public fold, which calls this only
            // after placing the card; kept rather than dropped regardless.
            self.subagent_orphans.extend(adopted);
            return;
        };
        for (_, item) in adopted {
            card.push_subagent(item);
        }
    }

    /// Places whatever is still held in the conversation, in order: the
    /// turn ended without the card it was waiting for ever arriving.
    pub(super) fn release_subagent_orphans(&mut self) {
        for (_, item) in std::mem::take(&mut self.subagent_orphans) {
            self.place_in_conversation(item);
        }
    }

    fn place_in_conversation(&mut self, item: SubagentItem) {
        let message = match item {
            SubagentItem::ToolCall(card) => PanelMessage::ToolCall(card),
            SubagentItem::Text(text) => PanelMessage::Assistant(text),
        };
        self.push_message(message);
    }
}
