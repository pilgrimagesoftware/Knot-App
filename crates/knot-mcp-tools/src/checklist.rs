//! `show-checklist` and `get-checklist` (#301): a list of questions and
//! checkbox items an agent puts in front of the user in the artifact panel,
//! and the read-back of what the user has done with it.
//!
//! The answers also reach the agent without it asking: the panel's Send
//! control delivers [`knot_agents::Checklist::summary`] as a prompt. The
//! read tool is for an agent that wants the state mid-way, or again later.

use knot_agents::{AgentStore, ChecklistItem, ChecklistItemKind};
use knot_mcp::ToolCallResult;
use serde::Serialize;
use serde_json::Value;

use crate::args::{optional_str, require_str};
use crate::lookup::{agent_not_found, find_by_name_or_id};
use crate::responses::success;

/// The `items` argument as checklist items, or why it is not one.
///
/// Each element is `{"id", "text", "kind"}`, `kind` being `check` (the
/// default) or `question`. Ids must be present, non-empty and unique: they
/// are what carries an answer across a republish.
fn parse_items(arguments: &Value) -> Result<Vec<ChecklistItem>, String> {
    let Some(raw) = arguments.get("items").and_then(Value::as_array)
    else {
        return Err("Missing required argument: items (an array)".to_owned());
    };
    if raw.is_empty() {
        return Err("items must hold at least one item".to_owned());
    }
    let mut items: Vec<ChecklistItem> = Vec::with_capacity(raw.len());
    for (index, entry) in raw.iter().enumerate() {
        let field = |name: &str| entry.get(name).and_then(Value::as_str).map(str::trim);
        let id = field("id").filter(|id| !id.is_empty())
                            .ok_or_else(|| format!("items[{index}] needs a non-empty \"id\""))?;
        let text =
            field("text").filter(|text| !text.is_empty())
                         .ok_or_else(|| format!("items[{index}] needs a non-empty \"text\""))?;
        let kind = match field("kind") {
            None => ChecklistItemKind::Check,
            Some(kind) => kind.parse::<ChecklistItemKind>()
                              .map_err(|error| format!("items[{index}]: {error}"))?,
        };
        if items.iter().any(|item| item.id == id) {
            return Err(format!("items[{index}]: the id \"{id}\" is used twice"));
        }
        items.push(ChecklistItem::new(id, text, kind));
    }
    Ok(items)
}

#[derive(Serialize)]
struct ShowChecklistResponse {
    success: bool,
    message: String,
}

pub fn show_checklist(store: &mut AgentStore, arguments: &Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let items = match parse_items(arguments) {
        Ok(items) => items,
        Err(message) => return ToolCallResult::error(message),
    };
    let Some(agent) = find_by_name_or_id(store, agent_id_str)
    else {
        return agent_not_found(store, agent_id_str);
    };
    let id = agent.id;
    let count = items.len();
    let title = optional_str(arguments, "title").map(str::to_owned);
    store.set_checklist(id, title, items).ok();

    success(&ShowChecklistResponse { success: true,
                                     message: format!("Checklist of {count} items shown in the artifact panel. The user's answers arrive as a message when they press Send; call get-checklist to read them at any time."), })
}

#[derive(Serialize)]
struct ChecklistItemState {
    id:      String,
    text:    String,
    kind:    String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    answer:  Option<String>,
}

#[derive(Serialize)]
struct GetChecklistResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    title:     Option<String>,
    /// Whether the user has sent their answers since the list last changed.
    submitted: bool,
    items:     Vec<ChecklistItemState>,
}

pub fn get_checklist(store: &AgentStore, arguments: &Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(v) => v,
        Err(err) => return err,
    };
    let Some(agent) = find_by_name_or_id(store, agent_id_str)
    else {
        return agent_not_found(store, agent_id_str);
    };
    let Some(checklist) = agent.checklist.as_ref()
    else {
        return ToolCallResult::error(format!("{} has no checklist open; show one with show-checklist.",
                                             agent.name));
    };
    let items = checklist.items
                         .iter()
                         .map(|item| {
                             let (checked, answer) = match item.kind {
                                 ChecklistItemKind::Check => (Some(item.checked), None),
                                 ChecklistItemKind::Question => (None, Some(item.answer.clone())),
                             };
                             ChecklistItemState { id: item.id.clone(),
                                                  text: item.text.clone(),
                                                  kind: item.kind.to_string(),
                                                  checked,
                                                  answer }
                         })
                         .collect();
    success(&GetChecklistResponse { title: checklist.title.clone(),
                                    submitted: checklist.submitted,
                                    items })
}

#[cfg(test)]
mod tests;
