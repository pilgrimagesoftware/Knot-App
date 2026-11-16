//! Where each agent type keeps its configuration, for the Config Files menu
//! (#27): the user-level files under the home directory and the project
//! files under the agent's folder.
//!
//! Pure: no file is checked here. The menus are rebuilt on the repaint poll,
//! which must not touch the disk, so an entry lists the paths it may be at
//! and [`open_config_file`] picks the first that exists when it is clicked.
//! `agent_type` is an open vocabulary (see `knot_core::agent_type`), so the
//! table keeps a test that every roster entry with config files has some.

use std::path::{Path, PathBuf};

/// The directories the user-level files hang off, read once per menu build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigRoots {
    /// The user's home directory.
    pub(crate) home:       PathBuf,
    /// Codex's home: `$CODEX_HOME`, else `~/.codex`.
    pub(crate) codex_home: PathBuf,
}

impl ConfigRoots {
    /// The roots for this process, or `None` without a home directory. Only
    /// environment reads, no file access.
    pub(crate) fn from_env() -> Option<Self> {
        let home = std::env::home_dir()?;
        let codex_home =
            std::env::var_os("CODEX_HOME").map_or_else(|| home.join(".codex"), PathBuf::from);
        Some(Self { home, codex_home })
    }
}

/// One Config Files entry: what it is called and where it may be, the
/// usual name first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigFile {
    pub(crate) label:      String,
    pub(crate) candidates: Vec<PathBuf>,
}

/// Which directory a config file hangs off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Root {
    Home,
    CodexHome,
    Project,
}

/// One entry of the table: its root and the names it may have there, the
/// usual one first.
struct Spec {
    root:  Root,
    names: &'static [&'static str],
}

/// The table, by agent type: user-level files first, then the project's.
/// Empty for a shell, a custom type, or a type Knot doesn't know.
fn specs(agent_type: &str) -> &'static [Spec] {
    use Root::{CodexHome, Home, Project};
    match agent_type {
        "claude" => &[Spec { root:  Home,
                             names: &[".claude.json"], },
                      Spec { root:  Home,
                             names: &[".claude/settings.json"], },
                      Spec { root:  Project,
                             names: &[".claude/settings.json"], },
                      Spec { root:  Project,
                             names: &[".claude/settings.local.json"], },
                      Spec { root:  Project,
                             names: &[".mcp.json"], }],
        "codex" => &[Spec { root:  CodexHome,
                            names: &["config.toml"], },
                     Spec { root:  Project,
                            names: &[".codex/config.toml"], }],
        "opencode" => &[Spec { root:  Home,
                               names: &[".config/opencode/opencode.jsonc",
                                        ".config/opencode/opencode.json"], },
                        Spec { root:  Project,
                               names: &["opencode.jsonc", "opencode.json"], }],
        "gemini" => &[Spec { root:  Home,
                             names: &[".gemini/settings.json"], },
                      Spec { root:  Project,
                             names: &[".gemini/settings.json"], }],
        "copilot" => &[Spec { root:  Home,
                              names: &[".copilot/config.json"], },
                       Spec { root:  Home,
                              names: &[".copilot/mcp-config.json"], }],
        "antigravity" => &[Spec { root:  Home,
                                  names: &[".gemini/antigravity-cli/settings.json"], }],
        _ => &[],
    }
}

/// Whether Knot knows where `agent_type` keeps any config, which is what
/// shows the Config Files entry. Depends on the type alone.
pub(crate) fn has_config_files(agent_type: &str) -> bool {
    !specs(agent_type).is_empty()
}

/// `agent_type`'s config files, resolved against `roots` and `folder`.
pub(crate) fn config_files(agent_type: &str, folder: &Path, roots: &ConfigRoots)
                           -> Vec<ConfigFile> {
    specs(agent_type).iter()
                     .map(|spec| {
                         let dir = match spec.root {
                             Root::Home => roots.home.as_path(),
                             Root::CodexHome => roots.codex_home.as_path(),
                             Root::Project => folder,
                         };
                         let candidates: Vec<PathBuf> =
                             spec.names.iter().map(|name| dir.join(name)).collect();
                         let label = match spec.root {
                             Root::Project => {
                                 knot_core::l10n::t_with("menu.agent.config_project_file",
                                                         &[("path", spec.names[0])]).to_string()
                             }
                             Root::Home | Root::CodexHome => {
                                 home_label(&roots.home, &candidates[0])
                             }
                         };
                         ConfigFile { label, candidates }
                     })
                     .collect()
}

/// `path` written from `~` when it is under `home`.
fn home_label(home: &Path, path: &Path) -> String {
    path.strip_prefix(home)
        .map_or_else(|_| path.display().to_string(),
                     |rest| format!("~/{}", rest.display()))
}

/// The config files of agent `id`'s type, resolved for its folder - what the
/// Config Files submenus list. Empty for an unknown agent or without a home
/// directory. Reads only the store and the environment.
pub(crate) fn for_agent(store: &knot_agents::AgentStore, id: uuid::Uuid) -> Vec<ConfigFile> {
    let (Some(agent), Some(roots)) = (store.agent(id), ConfigRoots::from_env())
    else {
        return Vec::new();
    };
    config_files(&agent.agent_type, Path::new(&agent.folder), &roots)
}

/// Opens the first of `candidates` that exists in the user's default text
/// editor, off the calling thread. Quietly does nothing when none exists or
/// the platform has no `open`, as Open In does for a missing application.
pub(crate) fn open_config_file(candidates: &[PathBuf]) {
    let Some(path) = candidates.iter().find(|path| path.is_file()).cloned()
    else {
        return;
    };
    std::thread::spawn(move || open_in_text_editor(&path));
}

/// `open -t`: the default text editor, not whatever owns the extension -
/// `.json` would otherwise go to Xcode on many Macs.
#[cfg(target_os = "macos")]
fn open_in_text_editor(path: &Path) {
    if let Err(error) = std::process::Command::new("/usr/bin/open").arg("-t")
                                                                   .arg(path)
                                                                   .status()
    {
        eprintln!("failed to open {} in an editor: {error}", path.display());
    }
}

/// Nothing to open elsewhere: Knot ships on macOS.
#[cfg(not(target_os = "macos"))]
fn open_in_text_editor(_path: &Path) {}

#[cfg(test)]
mod tests;
