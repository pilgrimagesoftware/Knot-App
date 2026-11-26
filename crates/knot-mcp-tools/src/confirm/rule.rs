//! Which `create-agent` and `create-worktree` calls need the user's
//! confirmation, and what the dialog shows for them.

use std::path::{Component, Path, PathBuf};

use knot_core::BenchAgent;
use uuid::Uuid;

use crate::args::{optional_bool, optional_str};

/// What the user is asked to confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationRequest {
    /// The agent whose call this is.
    pub requester:        Uuid,
    /// Its name as the knot shows it, or its id when it has none.
    pub requester_name:   String,
    /// The shell command the new agent would run, when that is why.
    pub command:          Option<String>,
    /// Where it would run: the new agent's folder, or the worktree's.
    pub folder:           PathBuf,
    /// Whether that folder is a worktree outside the repository's parent
    /// directory - the other reason to ask.
    pub distant_worktree: bool,
}

/// The confirmation a `create-agent` call needs, or `None` when it needs
/// none: a shell agent with a non-empty command, or a worktree whose
/// destination is outside the repository's parent. `bench` is the template
/// the call deploys, whose command counts as the call's own.
pub(crate) fn create_agent_request(arguments: &serde_json::Value, bench: Option<&BenchAgent>,
                                   requester: (Uuid, String))
                                   -> Option<ConfirmationRequest> {
    let agent_type =
        optional_str(arguments, "agentType").map(str::to_owned)
                                            .or_else(|| bench.map(|b| b.agent_type.clone()))?;
    let command =
        optional_str(arguments, "command").map(str::to_owned)
                                          .or_else(|| bench.and_then(|b| b.shell_command.clone()))
                                          .filter(|command| !command.trim().is_empty())
                                          .filter(|_| knot_core::agent_type::is_shell(&agent_type));
    let repo_path =
        optional_str(arguments, "repoPath").map(str::to_owned)
                                           .or_else(|| bench.map(|b| b.folder.clone()))?;
    let worktree = optional_bool(arguments, "createWorktree").unwrap_or(false);
    let (folder, distant_worktree) = match (worktree, optional_str(arguments, "branchName")) {
        (true, Some(branch)) => {
            let destination =
                destination(&repo_path, branch, optional_str(arguments, "worktreePath"));
            let distant = is_distant(&repo_path, &destination);
            (destination, distant)
        }
        _ => (PathBuf::from(&repo_path), false),
    };
    (command.is_some() || distant_worktree).then_some(ConfirmationRequest { requester:
                                                                                requester.0,
                                                                            requester_name:
                                                                                requester.1,
                                                                            command,
                                                                            folder,
                                                                            distant_worktree })
}

/// The confirmation a `create-worktree` call needs: only when its `path` is
/// outside the repository's parent directory.
pub(crate) fn create_worktree_request(arguments: &serde_json::Value, requester: (Uuid, String))
                                      -> Option<ConfirmationRequest> {
    let repo_path = optional_str(arguments, "repoPath")?;
    let branch = optional_str(arguments, "branchName")?;
    let destination = destination(repo_path, branch, optional_str(arguments, "path"));
    is_distant(repo_path, &destination).then_some(ConfirmationRequest { requester:
                                                                            requester.0,
                                                                        requester_name:
                                                                            requester.1,
                                                                        command:          None,
                                                                        folder:
                                                                            destination,
                                                                        distant_worktree: true, })
}

/// Where a worktree of `repo_path` on `branch` would go: `requested`, as
/// the tools read it, or the suggested sibling.
fn destination(repo_path: &str, branch: &str, requested: Option<&str>) -> PathBuf {
    match requested.filter(|path| !path.is_empty()) {
        Some(path) => {
            crate::repos::requested_destination(path, std::env::home_dir().as_deref())
                .unwrap_or_else(|_| PathBuf::from(path))
        }
        None => knot_git::suggest_worktree_path(Path::new(repo_path), branch),
    }
}

/// Whether `destination` lies outside `repo_path`'s parent directory once
/// both are canonical. A destination that does not exist yet is resolved
/// through its nearest existing ancestor. One that cannot be resolved at
/// all, or that climbs with `..` past what exists, counts as distant: when
/// in doubt, ask.
pub(in crate::confirm) fn is_distant(repo_path: &str, destination: &Path) -> bool {
    let Some(parent) = Path::new(repo_path).canonicalize()
                                           .ok()
                                           .and_then(|repo| repo.parent().map(Path::to_path_buf))
    else {
        return true;
    };
    canonical_destination(destination).is_none_or(|resolved| !resolved.starts_with(&parent))
}

/// `path` with its existing part canonical and the rest appended, or `None`
/// when the rest is not plain names.
fn canonical_destination(path: &Path) -> Option<PathBuf> {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            let mut resolved = canonical;
            for name in rest.iter().rev() {
                resolved.push(name);
            }
            return Some(resolved);
        }
        let name = existing.file_name()?;
        if !matches!(Path::new(name).components().next(),
                     Some(Component::Normal(_)))
        {
            return None;
        }
        rest.push(name.to_owned());
        existing = existing.parent()?;
    }
}
