//! Recovering launches from configuration laid out the way `claude mcp add`
//! and `gemini mcp add` write it - shapes captured from both CLIs in a
//! throwaway home, then reduced to what each case needs.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{ConfigRoots, recover_launches};
use crate::parse::ListFormat;
use crate::server::{ServerRow, Target};
use crate::state::ServerState;

struct Home {
    home:   TempDir,
    folder: std::path::PathBuf,
}

impl Home {
    fn new() -> Self {
        let home = TempDir::new().expect("home");
        let folder = home.path().join("project");
        fs::create_dir_all(&folder).expect("project folder");
        Self { home, folder }
    }

    fn write(&self, relative: &str, json: &str) {
        let path = self.home.path().join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
        fs::write(path, json).expect("write");
    }

    fn write_in_project(&self, relative: &str, json: &str) {
        let path = self.folder.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
        fs::write(path, json).expect("write");
    }

    fn roots(&self) -> ConfigRoots {
        ConfigRoots { home:   self.home.path().to_path_buf(),
                      folder: self.folder.clone(), }
    }
}

fn stdio(name: &str, listed: &str) -> ServerRow {
    ServerRow::new(name,
                   Target::Stdio { command: listed.to_owned(), },
                   ServerState::Connected)
}

fn recovered(format: ListFormat, home: &Home, row: ServerRow)
             -> Option<knot_mcp_client::StdioServer> {
    let mut rows = [row];
    recover_launches(format, &mut rows, &home.roots());
    rows[0].launch.clone()
}

const USER_FILES: &str = r#"{ "mcpServers": { "files": { "type": "stdio", "command": "node",
    "args": ["/opt/files/index.js", "--root", "/tmp/a b"], "env": { "GREETING": "hi" } } } }"#;

/// The case #280 exists for: an argument with a space, which the listing
/// flattens and only the configuration keeps whole.
#[test]
fn a_claude_user_server_is_recovered_with_its_arguments_whole() {
    let home = Home::new();
    home.write(".claude.json", USER_FILES);

    let launch =
        recovered(ListFormat::ClaudeCode,
                  &home,
                  stdio("files", "node /opt/files/index.js --root /tmp/a b")).expect("recovered");

    assert_eq!(launch.program, "node");
    assert_eq!(launch.args, ["/opt/files/index.js", "--root", "/tmp/a b"]);
    assert_eq!(launch.env, [("GREETING".to_owned(), "hi".to_owned())]);
}

#[test]
fn a_claude_project_server_is_recovered_from_mcp_json() {
    let home = Home::new();
    home.write_in_project(".mcp.json",
                          r#"{ "mcpServers": { "proj": { "type": "stdio", "command": "npx", "args": ["-y", "@scope/server"], "env": {} } } }"#);

    let launch = recovered(ListFormat::ClaudeCode,
                           &home,
                           stdio("proj", "npx -y @scope/server")).expect("recovered");

    assert_eq!(launch.args, ["-y", "@scope/server"]);
}

/// Claude Code writes a local server under a real (canonical) path -
/// `/private/tmp/x` for `/tmp/x` on macOS.
#[test]
fn a_claude_local_server_is_found_under_the_canonical_folder() {
    let home = Home::new();
    let canonical = fs::canonicalize(&home.folder).expect("canonical");
    home.write(".claude.json",
               &format!(r#"{{ "projects": {{ "{}": {{ "mcpServers": {{ "loc": {{ "type": "stdio", "command": "uvx", "args": ["thing", "--flag"], "env": {{}} }} }} }} }} }}"#,
                        canonical.display()));

    let launch = recovered(ListFormat::ClaudeCode,
                           &home,
                           stdio("loc", "uvx thing --flag")).expect("recovered");

    assert_eq!(launch.program, "uvx");
}

/// Two scopes with an identical entry are one answer, so it is used.
#[test]
fn candidates_that_agree_are_used() {
    let home = Home::new();
    home.write(".claude.json", USER_FILES);
    home.write_in_project(".mcp.json", USER_FILES);

    assert!(recovered(ListFormat::ClaudeCode,
                      &home,
                      stdio("files", "node /opt/files/index.js --root /tmp/a b")).is_some());
}

/// Claude Code's precedence does not hold in practice - a user entry was
/// seen to beat an unapproved project one - so which scope it resolved
/// cannot be known. Two same-line candidates that differ in anything leave
/// the row without a launch rather than guessing.
#[test]
fn candidates_that_disagree_recover_nothing() {
    let home = Home::new();
    let canonical = fs::canonicalize(&home.folder).expect("canonical");
    home.write(".claude.json",
               &format!(r#"{{ "mcpServers": {{ "dup": {{ "command": "node", "args": ["x.js"], "env": {{ "SCOPE": "user" }} }} }},
                           "projects": {{ "{}": {{ "mcpServers": {{ "dup": {{ "command": "node", "args": ["x.js"], "env": {{ "SCOPE": "local" }} }} }} }} }} }}"#,
                        canonical.display()));

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("dup", "node x.js")),
               None);
}

/// A same-named entry whose line differs is not a candidate at all, so it
/// cannot veto the one that matches.
#[test]
fn a_same_named_entry_for_another_line_does_not_veto() {
    let home = Home::new();
    home.write(".claude.json", USER_FILES);
    home.write_in_project(".mcp.json",
                          r#"{ "mcpServers": { "files": { "command": "node", "args": ["other.js"] } } }"#);

    assert!(recovered(ListFormat::ClaudeCode,
                      &home,
                      stdio("files", "node /opt/files/index.js --root /tmp/a b")).is_some());
}

/// Claude Code keys a local server by the git root's real path, so an agent
/// working in a subfolder still finds it.
#[test]
fn a_claude_local_server_is_found_from_a_subfolder_of_the_git_root() {
    let home = Home::new();
    fs::create_dir_all(home.folder.join(".git")).expect("a .git");
    let root = fs::canonicalize(&home.folder).expect("canonical");
    home.write(".claude.json",
               &format!(r#"{{ "projects": {{ "{}": {{ "mcpServers": {{ "loc": {{ "command": "uvx", "args": ["thing"] }} }} }} }} }}"#,
                        root.display()));
    let sub = Home { home:   TempDir::new().expect("unused"),
                     folder: home.folder.join("crates").join("api"), };
    fs::create_dir_all(&sub.folder).expect("subfolder");
    let roots = ConfigRoots { home:   home.home.path().to_path_buf(),
                              folder: sub.folder.clone(), };

    let mut rows = [stdio("loc", "uvx thing")];
    recover_launches(ListFormat::ClaudeCode, &mut rows, &roots);

    assert_eq!(rows[0].launch
                      .as_ref()
                      .map(|launch| launch.program.as_str()),
               Some("uvx"));
}

/// `.mcp.json` is found by walking up from the agent's folder.
#[test]
fn a_claude_project_file_above_the_folder_is_found() {
    let home = Home::new();
    home.write_in_project(".mcp.json",
                          r#"{ "mcpServers": { "proj": { "command": "npx", "args": ["-y", "@scope/server"] } } }"#);
    let deeper = home.folder.join("packages").join("web");
    fs::create_dir_all(&deeper).expect("deeper");
    let roots = ConfigRoots { home:   home.home.path().to_path_buf(),
                              folder: deeper, };

    let mut rows = [stdio("proj", "npx -y @scope/server")];
    recover_launches(ListFormat::ClaudeCode, &mut rows, &roots);

    assert!(rows[0].launch.is_some());
}

/// The safety rule: an entry that does not join to exactly the listed line
/// is not what the CLI resolved, so nothing is recovered.
#[test]
fn an_entry_that_differs_from_the_listed_line_is_not_used() {
    let home = Home::new();
    home.write(".claude.json", USER_FILES);

    assert_eq!(recovered(ListFormat::ClaudeCode,
                         &home,
                         stdio("files", "node /opt/files/OTHER.js --root /tmp/a b")),
               None);
}

/// A placeholder would be expanded by the CLI in a way this crate can only
/// guess at, and the line match would not catch a wrong environment value.
#[test]
fn an_entry_with_a_placeholder_is_not_used() {
    let home = Home::new();
    home.write(".claude.json",
               r#"{ "mcpServers": { "keyed": { "command": "node", "args": ["k.js"], "env": { "TOKEN": "${API_TOKEN}" } } } }"#);

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("keyed", "node k.js")),
               None);
}

#[test]
fn a_gemini_user_server_is_recovered() {
    let home = Home::new();
    home.write(".gemini/settings.json",
               r#"{ "mcpServers": { "g-user": { "command": "node", "args": ["/opt/g/index.js", "--root", "/tmp/a b"], "env": { "GREETING": "hi" } } } }"#);

    let launch =
        recovered(ListFormat::Gemini,
                  &home,
                  stdio("g-user", "node /opt/g/index.js --root /tmp/a b")).expect("recovered");

    assert_eq!(launch.args, ["/opt/g/index.js", "--root", "/tmp/a b"]);
}

#[test]
fn a_gemini_project_server_keeps_its_cwd() {
    let home = Home::new();
    home.write_in_project(".gemini/settings.json",
                          r#"{ "mcpServers": { "g": { "command": "npx", "args": ["-y", "@scope/g"], "cwd": "/srv/g" } } }"#);

    let launch =
        recovered(ListFormat::Gemini, &home, stdio("g", "npx -y @scope/g")).expect("recovered");

    assert_eq!(launch.cwd.as_deref(), Some(Path::new("/srv/g")));
}

#[test]
fn gemini_scopes_that_disagree_recover_nothing() {
    let home = Home::new();
    home.write(".gemini/settings.json",
               r#"{ "mcpServers": { "g": { "command": "npx", "args": ["-y", "@scope/g"], "env": { "SCOPE": "user" } } } }"#);
    home.write_in_project(".gemini/settings.json",
                          r#"{ "mcpServers": { "g": { "command": "npx", "args": ["-y", "@scope/g"] } } }"#);

    assert_eq!(recovered(ListFormat::Gemini, &home, stdio("g", "npx -y @scope/g")),
               None);
}

#[test]
fn no_configuration_recovers_nothing() {
    let home = Home::new();

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node x.js")),
               None);
    assert_eq!(recovered(ListFormat::Gemini, &home, stdio("files", "node x.js")),
               None);
}

#[test]
fn an_unreadable_file_recovers_nothing() {
    let home = Home::new();
    home.write(".claude.json", "{ this is not json");

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node x.js")),
               None);
}

/// An HTTP row has no launch to recover, and a row that already has one -
/// Codex's - keeps it.
#[test]
fn only_stdio_rows_without_a_launch_are_touched() {
    let home = Home::new();
    home.write(".claude.json", USER_FILES);
    let http = ServerRow::new("files",
                              Target::Http { url: "http://x/mcp".to_owned(), },
                              ServerState::Connected);

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, http), None);
}

/// PR #303 review, the blocking case: the user scope's `files` uses a
/// placeholder Claude resolves, and a project `.mcp.json` defines the same
/// line with a literal environment. Skipping the placeholder entry would
/// launch the project file's environment under a name the CLI resolved from
/// the user scope, so the placeholder must veto instead.
#[test]
fn a_placeholder_candidate_vetoes_rather_than_being_skipped() {
    let home = Home::new();
    home.write(".claude.json",
               r#"{ "mcpServers": { "files": { "command": "node", "args": ["x.js"], "env": { "TOKEN": "${X}" } } } }"#);
    home.write_in_project(".mcp.json",
                          r#"{ "mcpServers": { "files": { "command": "node", "args": ["x.js"], "env": { "TOKEN": "attacker" } } } }"#);

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node x.js")),
               None);
}

/// A placeholder in an entry for a *different* line is not a candidate, so
/// it does not veto: only the line match narrows.
#[test]
fn a_placeholder_in_a_different_line_does_not_veto() {
    let home = Home::new();
    home.write(".claude.json",
           r#"{ "mcpServers": { "files": { "command": "node", "args": ["${DIR}/other.js"] } } }"#);
    home.write_in_project(".mcp.json",
                      r#"{ "mcpServers": { "files": { "command": "node", "args": ["x.js"] } } }"#);

    assert!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node x.js")).is_some());
}

/// A same-named entry that cannot be read as an entry - here `args` is not a
/// list of strings - has an unknown line, so it vetoes rather than vanishing.
#[test]
fn an_unparsable_same_named_entry_vetoes() {
    let home = Home::new();
    home.write(".claude.json",
               r#"{ "mcpServers": { "files": { "command": "node", "args": "x.js" } } }"#);
    home.write_in_project(".mcp.json",
                      r#"{ "mcpServers": { "files": { "command": "node", "args": ["x.js"] } } }"#);

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node x.js")),
               None);
}

/// Joining loses argument boundaries: `["a b"]` and `["a", "b"]` both list
/// as `node a b`. Both are candidates and they differ, so nothing is started.
#[test]
fn entries_that_join_to_the_same_line_but_split_differently_recover_nothing() {
    let home = Home::new();
    home.write(".claude.json",
               r#"{ "mcpServers": { "files": { "command": "node", "args": ["a b"] } } }"#);
    home.write_in_project(".mcp.json",
                          r#"{ "mcpServers": { "files": { "command": "node", "args": ["a", "b"] } } }"#);

    assert_eq!(recovered(ListFormat::ClaudeCode, &home, stdio("files", "node a b")),
               None);
}
