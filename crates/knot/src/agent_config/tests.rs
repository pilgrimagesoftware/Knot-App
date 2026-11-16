//! Unit tests for [`super`].

use super::*;

fn roots() -> ConfigRoots {
    ConfigRoots { home:       PathBuf::from("/Users/me"),
                  codex_home: PathBuf::from("/Users/me/.codex"), }
}

fn labels(agent_type: &str) -> Vec<String> {
    config_files(agent_type, Path::new("/work/repo"), &roots()).into_iter()
                                                               .map(|file| file.label)
                                                               .collect()
}

/// Every coding agent in the roster has somewhere its config lives; a shell
/// and the custom slots have nothing Knot can name. A roster entry added
/// without a row here fails this.
#[test]
fn every_coding_agent_in_the_roster_has_config_files() {
    for info in knot_core::agent_type::ALL {
        let files = config_files(info.id, Path::new("/work/repo"), &roots());
        let expect_none = info.id == "shell" || info.id.starts_with("custom");
        assert_eq!(files.is_empty(), expect_none, "{}", info.id);
    }
}

#[test]
fn claude_lists_user_files_then_the_projects() {
    let files = config_files("claude", Path::new("/work/repo"), &roots());
    assert_eq!(files[0].label, "~/.claude.json");
    assert_eq!(files[1].candidates,
               vec![PathBuf::from("/Users/me/.claude/settings.json")]);
    assert_eq!(files[2].candidates,
               vec![PathBuf::from("/work/repo/.claude/settings.json")]);
    assert!(files[2].label.contains(".claude/settings.json"),
            "the path survives substitution into the project label");
}

/// Codex honours `$CODEX_HOME`, and the label still reads from `~` when it
/// is under the home directory.
#[test]
fn codex_follows_its_home() {
    let moved = ConfigRoots { codex_home: PathBuf::from("/Users/me/.config/codex"),
                              ..roots() };
    let files = config_files("codex", Path::new("/work/repo"), &moved);
    assert_eq!(files[0].label, "~/.config/codex/config.toml");
    assert_eq!(files[0].candidates,
               vec![PathBuf::from("/Users/me/.config/codex/config.toml")]);
}

/// opencode accepts either extension, so one entry carries both.
#[test]
fn opencode_tries_both_extensions() {
    let files = config_files("opencode", Path::new("/work/repo"), &roots());
    assert_eq!(files[0].candidates,
               vec![PathBuf::from("/Users/me/.config/opencode/opencode.jsonc"),
                    PathBuf::from("/Users/me/.config/opencode/opencode.json")]);
    assert_eq!(labels("opencode").len(), 2);
}

#[test]
fn the_project_label_key_resolves() {
    assert_ne!(knot_core::l10n::t("menu.agent.config_project_file"),
               "menu.agent.config_project_file");
}

/// A click on an entry none of whose files exist does nothing, rather than
/// opening an editor on a file that isn't there.
#[test]
fn opening_a_missing_file_does_nothing() {
    open_config_file(&[PathBuf::from("/nonexistent/knot-test/config.toml")]);
}

#[test]
fn whether_a_type_has_config_files_needs_no_paths() {
    assert!(has_config_files("claude"));
    assert!(!has_config_files("shell"));
    assert!(!has_config_files("custom1"));
}
