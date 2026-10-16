//! Recovers exactly how a listed stdio server is started, for the CLIs whose
//! listing prints its command only as one line (#280).
//!
//! `claude mcp list` and `gemini mcp list` print `node index.js --root /tmp/a
//! b`, which cannot be split back into arguments: `/tmp/a b` was one.
//! `claude mcp get` joins its `Args:` line the same way, and Gemini has no
//! `get` at all, so the agent's own configuration - which keeps arguments
//! separate - is the only exact source. It is read to recover a launch and
//! never to decide what servers exist; that stays the listing's job, for the
//! reasons the crate docs give.
//!
//! The safety of reading it rests on two rules:
//!
//! - **The listed line decides.** An entry is a candidate only when its program
//!   and arguments, joined with spaces, are exactly the line the CLI listed, so
//!   nothing can be started that the agent would not start.
//! - **Every candidate must agree, and any doubt vetoes.** Claude Code's
//!   documented scope precedence does not hold in practice - a same-named user
//!   entry was seen to win over an unapproved project `.mcp.json` entry - so
//!   this never guesses which scope the CLI resolved. Every same-named entry
//!   whose line matches is a candidate, and the launch is used only when all of
//!   them are identical, environment and directory included. A candidate
//!   carrying a `$` placeholder vetoes rather than being skipped: the CLI may
//!   well resolve that one, and skipping it would launch another scope's
//!   environment under its name. A same-named entry that cannot be read as an
//!   entry at all vetoes too, since its line is unknown.
//!
//! Credentials are not read: `headers` and OAuth settings are HTTP concerns,
//! and only stdio rows are touched. A missing or unreadable *file* contributes
//! nothing and is not an error - the row says its tools cannot be listed, as
//! before.
//!
//! What this cannot see: configuration Knot does not read - managed or
//! enterprise MCP policy, and servers contributed by Claude Code plugins or
//! Gemini extensions - takes no part in the agreement. A row whose name comes
//! only from there has no candidate here and recovers nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use knot_mcp_client::StdioServer;
use serde::Deserialize;
use serde_json::Value;

use crate::consts::{
    CLAUDE_PROJECT_CONFIG, CLAUDE_PROJECTS_KEY, CLAUDE_USER_CONFIG, GEMINI_CONFIG_DIR,
    GEMINI_SETTINGS_FILE, MCP_SERVERS_KEY,
};
use crate::parse::ListFormat;
use crate::server::{ServerRow, Target};

/// Where an agent's configuration lives: the user's home, and the folder the
/// agent runs in, which project and local scopes are found from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRoots {
    pub home:   PathBuf,
    pub folder: PathBuf,
}

/// One server entry, as both CLIs write it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct Entry {
    command: Option<String>,
    #[serde(default)]
    args:    Vec<String>,
    #[serde(default)]
    env:     BTreeMap<String, String>,
    cwd:     Option<PathBuf>,
}

impl Entry {
    /// The line a CLI lists this entry as.
    fn line(&self) -> Option<String> {
        let command = self.command.as_deref()?;
        Some(std::iter::once(command).chain(self.args.iter().map(String::as_str))
                                     .collect::<Vec<_>>()
                                     .join(" "))
    }

    fn has_placeholder(&self) -> bool {
        let cwd = self.cwd.as_ref().map(|cwd| cwd.to_string_lossy());
        self.command
            .iter()
            .map(String::as_str)
            .chain(self.args.iter().map(String::as_str))
            .chain(self.env.keys().map(String::as_str))
            .chain(self.env.values().map(String::as_str))
            .chain(cwd.as_deref())
            .any(|text| text.contains('$'))
    }

    fn into_launch(self) -> Option<StdioServer> {
        Some(StdioServer { program: self.command?,
                           args:    self.args,
                           env:     self.env.into_iter().collect(),
                           cwd:     self.cwd, })
    }
}

/// One same-named entry in a configuration file: read, or present but not in
/// a shape this crate understands - which still counts, as a veto.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Candidate {
    Read(Entry),
    Unreadable,
}

/// One configuration file's servers, by name.
type Scope = BTreeMap<String, Candidate>;

/// Fills in the launch of every stdio row in `rows` that has none, from the
/// configuration `format`'s CLI reads, under the rules in the module docs.
pub fn recover_launches(format: ListFormat, rows: &mut [ServerRow], roots: &ConfigRoots) {
    let scopes = match format {
        ListFormat::ClaudeCode => claude_scopes(roots),
        ListFormat::Gemini => gemini_scopes(roots),
        // Codex lists its launches in separate words already.
        ListFormat::Codex => return,
    };
    for row in rows.iter_mut().filter(|row| row.launch.is_none()) {
        if let Target::Stdio { command } = &row.target {
            row.launch = agreed_launch(&scopes, &row.name, command.trim());
        }
    }
}

/// The launch every same-named candidate agrees on, or `None` when there is
/// none, any is unreadable or carries a placeholder, or any two differ.
///
/// Only the line match narrows the candidates. Nothing else is filtered out
/// before the agreement check, because a skipped entry is exactly how a
/// launch the CLI would not use gets chosen.
fn agreed_launch(scopes: &[Scope], name: &str, listed: &str) -> Option<StdioServer> {
    let mut agreed: Option<&Entry> = None;
    for candidate in scopes.iter().filter_map(|scope| scope.get(name)) {
        let entry = match candidate {
            Candidate::Unreadable => return None,
            Candidate::Read(entry) => entry,
        };
        if entry.line().as_deref() != Some(listed) {
            continue;
        }
        if entry.has_placeholder() || agreed.is_some_and(|first| first != entry) {
            return None;
        }
        agreed = Some(entry);
    }
    agreed.cloned().and_then(Entry::into_launch)
}

/// Every file Claude Code may resolve a server from: the local scope in
/// `~/.claude.json` (keyed by the git root's real path - the folder's own
/// real path is tried too), each `.mcp.json` from the folder up to the
/// filesystem root, and the user scope in `~/.claude.json`.
fn claude_scopes(roots: &ConfigRoots) -> Vec<Scope> {
    let mut scopes = Vec::new();
    let user_file = read_json(&roots.home.join(CLAUDE_USER_CONFIG));
    if let Some(projects) = user_file.as_ref()
                                     .and_then(|file| file.get(CLAUDE_PROJECTS_KEY))
    {
        let mut keys = vec![real_path(&roots.folder), roots.folder.clone()];
        if let Some(root) = git_root(&roots.folder) {
            keys.push(real_path(&root));
            keys.push(root);
        }
        keys.dedup();
        scopes.extend(keys.iter()
                          .filter_map(|key| projects.get(key.to_string_lossy().as_ref()))
                          .map(servers_in));
    }
    scopes.extend(ancestors_with(&roots.folder, Path::new(CLAUDE_PROJECT_CONFIG)));
    scopes.extend(user_file.as_ref().map(servers_in));
    scopes
}

/// Every file Gemini may resolve a server from: each `.gemini/settings.json`
/// from the folder up, then the user's.
fn gemini_scopes(roots: &ConfigRoots) -> Vec<Scope> {
    let settings = Path::new(GEMINI_CONFIG_DIR).join(GEMINI_SETTINGS_FILE);
    let mut scopes = ancestors_with(&roots.folder, &settings);
    scopes.extend(read_json(&roots.home.join(&settings)).as_ref()
                                                        .map(servers_in));
    scopes
}

/// `relative`'s servers in `folder` and each of its ancestors that has it.
fn ancestors_with(folder: &Path, relative: &Path) -> Vec<Scope> {
    folder.ancestors()
          .filter_map(|dir| read_json(&dir.join(relative)))
          .map(|file| servers_in(&file))
          .collect()
}

/// The nearest folder at or above `folder` holding `.git` - a directory in a
/// clone, a file in a linked worktree.
fn git_root(folder: &Path) -> Option<PathBuf> {
    folder.ancestors()
          .find(|dir| dir.join(".git").exists())
          .map(Path::to_path_buf)
}

fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The server entries of `value`, by name. An entry not in a recognizable
/// shape is kept as [`Candidate::Unreadable`] rather than dropped, so it can
/// veto a launch under its name.
fn servers_in(value: &Value) -> Scope {
    value.get(MCP_SERVERS_KEY)
         .and_then(Value::as_object)
         .map(|servers| {
             servers.iter()
                    .map(|(name, entry)| {
                        let candidate = serde_json::from_value::<Entry>(entry.clone())
                            .map_or(Candidate::Unreadable, Candidate::Read);
                        (name.clone(), candidate)
                    })
                    .collect()
         })
         .unwrap_or_default()
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

#[cfg(test)]
mod tests;
