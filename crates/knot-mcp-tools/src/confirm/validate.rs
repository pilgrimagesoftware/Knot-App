//! Refusing a call that would fail anyway, before the user is asked about
//! it (review of #522): a dialog for a call that cannot succeed only
//! trains the user to click through dialogs.
//!
//! The same checks the tools make, for the arguments that decide whether a
//! confirmation is needed. Whatever the tool would still refuse after a
//! confirmation (a bench template that went away, say) it refuses then.

use std::path::Path;

use knot_git::worktree::is_working_tree;
use knot_mcp::ToolCallResult;

use crate::args::{optional_bool, optional_str};
use crate::consts;

/// The error `tool` would answer `arguments` with, checked without side
/// effects; `None` when the call is well formed.
pub(crate) fn invalid(tool: &str, arguments: &serde_json::Value) -> Option<ToolCallResult> {
    match tool {
        consts::CREATE_AGENT => invalid_create_agent(arguments),
        consts::CREATE_WORKTREE => invalid_worktree(optional_str(arguments, "repoPath"),
                                                    optional_str(arguments, "branchName"),
                                                    optional_str(arguments, "path")),
        _ => None,
    }
}

fn invalid_create_agent(arguments: &serde_json::Value) -> Option<ToolCallResult> {
    let worktree = optional_bool(arguments, "createWorktree").unwrap_or(false);
    let requested = optional_str(arguments, "worktreePath").filter(|path| !path.is_empty());
    if requested.is_some() && !worktree {
        return Some(ToolCallResult::error("worktreePath requires createWorktree: true"));
    }
    if !worktree {
        return None;
    }
    // A bench template supplies the folder when the call names none; its
    // folder is checked when the tool runs.
    match optional_str(arguments, "repoPath") {
        Some(repo) => {
            invalid_worktree(Some(repo), optional_str(arguments, "branchName"), requested)
        }
        None => optional_str(arguments, "branchName").filter(|branch| !branch.is_empty())
                                                     .is_none()
                                                     .then(|| missing("branchName")),
    }
}

fn invalid_worktree(repo: Option<&str>, branch: Option<&str>, requested: Option<&str>)
                    -> Option<ToolCallResult> {
    let Some(repo) = repo.filter(|repo| !repo.is_empty())
    else {
        return Some(missing("repoPath"));
    };
    if branch.is_none_or(str::is_empty) {
        return Some(missing("branchName"));
    }
    if !is_working_tree(Path::new(repo)) {
        return Some(ToolCallResult::error(format!("Failed to create worktree: {repo} is not a git working tree")));
    }
    let requested = requested.filter(|path| !path.is_empty())?;
    crate::repos::requested_destination(requested, std::env::home_dir().as_deref())
        .err()
        .map(|error| ToolCallResult::error(format!("Failed to create worktree: {error}")))
}

fn missing(parameter: &str) -> ToolCallResult {
    ToolCallResult::error(format!("Missing required parameter: {parameter}"))
}
