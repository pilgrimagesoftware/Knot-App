//! Registers a stdio server with the real `claude` and `gemini` CLIs in a
//! throwaway home, probes it, and checks the launch recovered from their
//! configuration is exactly what was registered (#280).
//!
//! Ignored by default and never run in CI, for the reasons
//! `against_a_real_cli` gives. The user's own configuration is never
//! touched: every command runs with `HOME` set to a temporary directory.
//!
//! ```text
//! cargo test -p knot-mcp-probe --test recovers_from_a_real_cli -- --ignored --nocapture
//! ```

use std::path::Path;
use std::process::Command;

use knot_mcp_probe::{CommandRunner, ConfigRoots, ListFormat, ProbeCommand, ProbePlan, probe};

const ARGS: [&str; 3] = ["/opt/files/index.js", "--root", "/tmp/a b"];

fn run(home: &Path, folder: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program).args(args)
                                      .current_dir(folder)
                                      .env("HOME", home)
                                      .status()
                                      .expect("the CLI runs");
    assert!(status.success(), "{program} {args:?} failed");
}

fn recovered(home: &Path, folder: &Path, program: &str, format: ListFormat)
             -> knot_mcp_client::StdioServer {
    let command = ProbeCommand::new(program,
                                    vec!["mcp".to_owned(), "list".to_owned()],
                                    folder).with_env(vec![("HOME".to_owned(),
                                                           home.display().to_string()),
                                                          ("PATH".to_owned(),
                                                           knot_core::exec_path::search_path())]);
    let plan = ProbePlan::Run { command,
                                format,
                                roots: Some(ConfigRoots { home:   home.to_path_buf(),
                                                          folder: folder.to_path_buf(), }) };
    let inventory = probe(&CommandRunner::new(), &plan).expect("a listing");
    let row = inventory.rows()
                       .iter()
                       .find(|row| row.name == "files")
                       .expect("the registered server is listed");
    println!("{program}: listed {:?} -> {:?}", row.target, row.launch);
    row.launch.clone().expect("the launch is recovered")
}

#[test]
#[ignore = "needs the real claude CLI on PATH"]
fn claude_code_user_server() {
    let home = tempfile::tempdir().expect("home");
    let folder = home.path().join("project");
    std::fs::create_dir_all(&folder).expect("folder");
    let mut args = vec!["mcp",
                        "add",
                        "-s",
                        "user",
                        "files",
                        "-e",
                        "GREETING=hi",
                        "--",
                        "node"];
    args.extend(ARGS);
    run(home.path(), &folder, "claude", &args);

    let launch = recovered(home.path(), &folder, "claude", ListFormat::ClaudeCode);

    assert_eq!(launch.program, "node");
    assert_eq!(launch.args, ARGS);
    assert_eq!(launch.env, [("GREETING".to_owned(), "hi".to_owned())]);
}

#[test]
#[ignore = "needs the real gemini CLI on PATH"]
fn gemini_user_server() {
    let home = tempfile::tempdir().expect("home");
    let folder = home.path().join("project");
    std::fs::create_dir_all(&folder).expect("folder");
    let mut args = vec!["mcp",
                        "add",
                        "-s",
                        "user",
                        "files",
                        "-e",
                        "GREETING=hi",
                        "node"];
    args.extend(ARGS);
    run(home.path(), &folder, "gemini", &args);

    let launch = recovered(home.path(), &folder, "gemini", ListFormat::Gemini);

    assert_eq!(launch.args, ARGS);
}
