//! D9 through the catalog (`mcp-auth` task 5.1): a shell command or a
//! distant worktree waits for the user; confirmed it runs, denied or timed
//! out it is a tool error and nothing is created; a plain shell and a
//! sibling worktree go through without asking.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use knot_agents::{AgentStore, CreateOptions};
use knot_git::Runner;
use knot_mcp::{ToolCallResult, ToolCatalog};
use knot_messaging::NoopNotifier;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tokio::sync::watch;
use uuid::Uuid;

use super::*;
use crate::McpToolCatalog;

struct Fixture {
    catalog: Arc<McpToolCatalog>,
    store:   Arc<Mutex<AgentStore>>,
    queue:   ConfirmationQueue,
    caller:  Uuid,
    dir:     tempfile::TempDir,
}

fn fixture(timeout: Duration) -> Fixture {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller",
                              CreateOptions { name: Some("Builder".into()),
                                              ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let (_tx, repos) = watch::channel(Vec::new());
    let queue = ConfirmationQueue::default();
    let catalog = McpToolCatalog::new(Arc::clone(&store), repos, Arc::new(NoopNotifier))
        .with_confirmation_queue(Arc::clone(&queue))
        .with_confirmation_timeout(timeout);
    let dir = tempfile::tempdir().expect("a temporary directory");
    init_repo(&dir.path().join("repo"));
    Fixture { catalog: Arc::new(catalog),
              store,
              queue,
              caller,
              dir }
}

fn init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let run = |args: &[&str]| {
        Runner::new(dir).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("seed.txt"), "seed\n").unwrap();
    run(&["add", "-A"]);
    run(&["commit", "-qm", "init"]);
}

impl Fixture {
    fn repo(&self) -> String {
        self.dir.path().join("repo").display().to_string()
    }

    fn shell_agent(&self, command: Option<&str>) -> Value {
        let mut arguments = json!({ "agentId": self.caller.to_string(), "name": "Runner",
                                    "agentType": "shell", "repoPath": self.repo() });
        if let Some(command) = command {
            arguments["command"] = json!(command);
        }
        arguments
    }

    /// Calls `tool`, answering the dialog with `answer` if one appears
    /// (`None` leaves it unanswered). Returns the result and the request
    /// the dialog showed, if any.
    async fn call(&self, tool: &'static str, arguments: Value, answer: Option<bool>)
                  -> (ToolCallResult, Option<ConfirmationRequest>) {
        let catalog = Arc::clone(&self.catalog);
        let call = tokio::spawn(async move { catalog.call(tool, arguments).await });
        let mut shown = None;
        for _ in 0..200 {
            if call.is_finished() {
                break;
            }
            if let Some(pending) = self.queue.lock().pop() {
                shown = Some(pending.request.clone());
                match answer {
                    Some(confirmed) => pending.answer(confirmed),
                    // Held, unanswered, until the call times out.
                    None => std::mem::forget(pending),
                }
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        (call.await.expect("the call finished"), shown)
    }

    fn agents(&self) -> usize {
        self.store.lock().agents().len()
    }
}

#[tokio::test]
async fn a_confirmed_shell_command_creates_its_agent() {
    let fixture = fixture(Duration::from_secs(30));
    let (result, shown) = fixture.call("create-agent",
                                       fixture.shell_agent(Some("make test")),
                                       Some(true))
                                 .await;

    assert_eq!(result.is_error, None, "{:?}", result.content);
    let shown = shown.expect("the user was asked");
    assert_eq!(shown.requester, fixture.caller);
    assert_eq!(shown.requester_name, "Builder");
    assert_eq!(shown.command.as_deref(), Some("make test"));
    assert_eq!(shown.folder, Path::new(&fixture.repo()));
    assert_eq!(fixture.agents(), 2);
}

#[tokio::test]
async fn a_denied_shell_command_creates_nothing() {
    let fixture = fixture(Duration::from_secs(30));
    let (result, shown) = fixture.call("create-agent",
                                       fixture.shell_agent(Some("make test")),
                                       Some(false))
                                 .await;

    assert!(shown.is_some());
    assert_eq!(result.is_error, Some(true));
    assert_eq!(fixture.agents(), 1, "no agent was created");
}

#[tokio::test]
async fn an_unanswered_confirmation_times_out_and_creates_nothing() {
    let fixture = fixture(Duration::from_millis(200));
    let (result, shown) =
        fixture.call("create-agent", fixture.shell_agent(Some("make test")), None)
               .await;

    assert!(shown.is_some());
    assert_eq!(result.is_error, Some(true));
    assert_eq!(fixture.agents(), 1);
}

#[tokio::test]
async fn a_plain_shell_needs_no_confirmation() {
    let fixture = fixture(Duration::from_secs(30));
    for command in [None, Some(""), Some("   ")] {
        let (result, shown) = fixture.call("create-agent", fixture.shell_agent(command), None)
                                     .await;
        assert_eq!(result.is_error, None, "{command:?}: {:?}", result.content);
        assert!(shown.is_none(), "{command:?} asked");
    }
}

#[tokio::test]
async fn a_sibling_worktree_needs_no_confirmation() {
    let fixture = fixture(Duration::from_secs(30));
    let (result, shown) =
        fixture.call("create-worktree",
                     json!({ "agentId": fixture.caller.to_string(), "repoPath": fixture.repo(),
                             "branchName": "feature" }),
                     None)
               .await;

    assert_eq!(result.is_error, None, "{:?}", result.content);
    assert!(shown.is_none());
    assert!(fixture.dir.path().join("repo-feature").is_dir());
}

#[tokio::test]
async fn a_distant_worktree_waits_and_a_denial_creates_no_folder() {
    let fixture = fixture(Duration::from_secs(30));
    let elsewhere = tempfile::tempdir().expect("another directory");
    let destination = elsewhere.path().join("far/away");
    let (result, shown) =
        fixture.call("create-worktree",
                     json!({ "agentId": fixture.caller.to_string(), "repoPath": fixture.repo(),
                             "branchName": "feature", "path": destination }),
                     Some(false))
               .await;

    let shown = shown.expect("the user was asked");
    assert!(shown.distant_worktree);
    assert_eq!(shown.command, None);
    assert_eq!(result.is_error, Some(true));
    assert!(!destination.exists(), "no worktree or folder was created");
}

#[tokio::test]
async fn a_distant_worktree_for_a_new_agent_is_confirmed_too() {
    let fixture = fixture(Duration::from_secs(30));
    let elsewhere = tempfile::tempdir().expect("another directory");
    let destination = elsewhere.path().join("wt");
    let mut arguments = fixture.shell_agent(None);
    arguments["agentType"] = json!("claude");
    arguments["createWorktree"] = json!(true);
    arguments["branchName"] = json!("feature");
    arguments["worktreePath"] = json!(destination);

    let (result, shown) = fixture.call("create-agent", arguments, Some(true)).await;

    assert!(shown.expect("asked").distant_worktree);
    assert_eq!(result.is_error, None, "{:?}", result.content);
    assert!(destination.is_dir());
}

#[tokio::test]
async fn with_no_ui_to_ask_the_call_is_refused() {
    let fixture = fixture(Duration::from_secs(30));
    let (_tx, repos) = watch::channel(Vec::new());
    let unattended = McpToolCatalog::new(Arc::clone(&fixture.store), repos, Arc::new(NoopNotifier));

    let result = unattended.call("create-agent", fixture.shell_agent(Some("make test")))
                           .await;

    assert_eq!(result.is_error, Some(true));
    assert_eq!(fixture.agents(), 1);
}

#[test]
fn a_path_climbing_out_of_the_parent_is_distant() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let repo = repo.display().to_string();

    assert!(!rule::is_distant(&repo, &dir.path().join("repo-feature")));
    assert!(!rule::is_distant(&repo, &dir.path().join("nested/deeper")));
    assert!(rule::is_distant(&repo, &dir.path().join("../escape")));
    assert!(rule::is_distant(&repo, Path::new("/")));
}

/// Review of #522: a call that would fail anyway is refused before the
/// user is asked.
#[tokio::test]
async fn an_invalid_call_is_refused_without_a_dialog() {
    let fixture = fixture(Duration::from_secs(30));
    let elsewhere = tempfile::tempdir().expect("another directory");
    let not_a_repo = elsewhere.path().join("plain");
    std::fs::create_dir(&not_a_repo).unwrap();

    for arguments in [json!({ "agentId": fixture.caller.to_string(), "repoPath": not_a_repo,
                              "branchName": "feature", "path": elsewhere.path().join("wt") }),
                      json!({ "agentId": fixture.caller.to_string(), "repoPath": fixture.repo(),
                              "branchName": "", "path": elsewhere.path().join("wt") })]
    {
        let (result, shown) = fixture.call("create-worktree", arguments.clone(), Some(true))
                                     .await;
        assert_eq!(result.is_error, Some(true), "{arguments}");
        assert!(shown.is_none(),
                "asked about a call that cannot succeed: {arguments}");
    }
}

/// Review of #522: one dialog per agent at a time. A second call while the
/// first waits is refused; once the first is answered, the agent may ask
/// again.
#[tokio::test]
async fn an_agent_has_at_most_one_call_waiting() {
    let fixture = fixture(Duration::from_secs(30));
    let catalog = Arc::clone(&fixture.catalog);
    let first_arguments = fixture.shell_agent(Some("make test"));
    let first = tokio::spawn(async move { catalog.call("create-agent", first_arguments).await });
    // Wait for the first to be queued.
    let pending = loop {
        if let Some(pending) = fixture.queue.lock().pop() {
            break pending;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    let second = fixture.catalog
                        .call("create-agent", fixture.shell_agent(Some("make lint")))
                        .await;
    assert_eq!(second.is_error, Some(true));
    assert!(second.content[0].text.contains("already waiting"),
            "{:?}",
            second.content);
    assert!(fixture.queue.lock().is_empty(),
            "no second dialog was queued");

    pending.answer(false);
    assert_eq!(first.await.unwrap().is_error, Some(true));
    let (result, shown) = fixture.call("create-agent",
                                       fixture.shell_agent(Some("make lint")),
                                       Some(true))
                                 .await;
    assert!(shown.is_some(), "asking again after an answer is allowed");
    assert_eq!(result.is_error, None, "{:?}", result.content);
}
