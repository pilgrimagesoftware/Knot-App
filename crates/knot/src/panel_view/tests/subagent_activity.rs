//! The delegation card's count line (#476): drawn once its subagent has run
//! something, and counting the subagent's calls the way a compact run does.

use super::fixture::tool_call;
use crate::panel_state::{SubagentItem, ToolRunSummary};
use crate::panel_view::message_tests::panel_style;
use crate::panel_view::subagent_activity::subagent_count_line;

fn command(id: &str) -> SubagentItem {
    let mut card = tool_call("completed");
    card.id = id.to_owned();
    card.kind = "execute".to_owned();
    SubagentItem::ToolCall(Box::new(card))
}

#[test]
fn a_delegation_with_no_subagent_calls_draws_no_count_line() {
    let mut delegation = tool_call("completed");
    delegation.subagent
              .push(SubagentItem::Text("thinking aloud".to_owned()));

    assert!(subagent_count_line(&delegation, &panel_style()).is_none());
}

#[test]
fn the_count_line_counts_the_subagents_calls_not_its_prose() {
    let mut delegation = tool_call("completed");
    delegation.subagent = vec![command("a"),
                               SubagentItem::Text("between".to_owned()),
                               command("b")];

    assert!(subagent_count_line(&delegation, &panel_style()).is_some());
    let summary = ToolRunSummary::of_cards(delegation.subagent_tool_calls());
    assert_eq!((summary.calls, summary.commands_run), (2, 2));
}
