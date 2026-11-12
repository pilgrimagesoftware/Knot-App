//! #405 review: a task the worker finished with `complete-task` alone must
//! not hold the next `resetContext` restart. `complete-task` writes no
//! message back, so a goal counted as owed a reply would stay owed forever,
//! and the restart - and the nudge held behind it - would never come.

use serde_json::json;

use super::{body, fixture};

#[test]
fn a_task_completed_without_a_reply_holds_no_later_reset() {
    let mut fx = fixture();
    let worker = fx.worker.to_string();
    body(&fx.plan(json!([{"id": "a", "goal": "Port it", "assignee": worker},
                         {"id": "b", "goal": "Then this, fresh", "assignee": worker,
                          "dependsOn": ["a"], "resetContext": true}])));
    body(&fx.dispatch("a"));
    // The worker reads its goal, does the work, and reports it - no message.
    fx.messages.mark_read(fx.worker);
    body(&fx.complete("a", "done"));

    body(&fx.dispatch("b"));

    assert!(fx.agents.agent(fx.worker).unwrap().fresh_context_requested,
            "the fresh-context restart is requested");
    assert!(!fx.messages.has_unanswered(fx.worker, |_| true),
            "and nothing the worker read holds it: a dispatched goal is never owed");
}
