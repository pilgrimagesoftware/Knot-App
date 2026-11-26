//! The hook scripts Knot ships, run as an agent's hooks run them, against a
//! live server (`mcp-auth` task 4.3): with the agent's own id and hook
//! credential the event lands; naming another agent, it is refused.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use knot_mcp::HookError;
use parking_lot::Mutex;

use super::*;

/// Records the agent of every status hook that reached the handler.
#[derive(Default)]
struct Recorded(Mutex<Vec<String>>);

impl AgentHookHandler for Recorded {
    fn register(&self, _request: &HookRequest) -> Result<Value, HookError> {
        Ok(json!({}))
    }

    fn status(&self, request: &HookRequest) -> Result<Value, HookError> {
        self.0.lock().push(request.agent_id.clone());
        Ok(json!({}))
    }
}

fn activity_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../plugin/claude/scripts/activity.sh")
}

/// Runs `activity.sh` as `agent`, presenting `token`, and waits for its
/// fire-and-forget post to finish.
fn run_activity(base: &str, agent: Uuid, token: &str) {
    let status = Command::new("bash").arg(activity_script())
                                     .args(["PreToolUse", "running"])
                                     .env("KNOT_MCP_URL", base)
                                     .env("KNOT_MCP_TOKEN", token)
                                     .env("KNOT_AGENT_ID", agent.to_string())
                                     .stdin(std::process::Stdio::piped())
                                     .spawn()
                                     .and_then(|mut child| {
                                         // The hook payload Claude pipes in.
                                         use std::io::Write as _;
                                         child.stdin
                                              .take()
                                              .expect("piped stdin")
                                              .write_all(br#"{"hook_event_name":"PreToolUse"}"#)?;
                                         child.wait()
                                     })
                                     .expect("bash runs the script");
    assert!(status.success());
}

fn wait_for(recorded: &Recorded, count: usize) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let seen = recorded.0.lock().clone();
        if seen.len() >= count || Instant::now() > deadline {
            return seen;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_activity_hook_reports_for_its_own_agent_only() {
    let recorded = Arc::new(Recorded::default());
    let credentials = knot_mcp::CredentialRegistry::new();
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents())
        .with_hook_handler(Arc::clone(&recorded) as Arc<dyn AgentHookHandler>)
        .with_credentials(credentials.clone());
    server.start().await.expect("server starts");
    let base = format!("http://{}", server.bound_addr().expect("bound address"));
    let (agent, other) = (Uuid::new_v4(), Uuid::new_v4());
    let pair = credentials.issue_agent_pair(agent);

    let recorded_for_blocking = Arc::clone(&recorded);
    let hooks = pair.hooks.expose().to_owned();
    let seen = tokio::task::spawn_blocking(move || {
                   // Refused: another agent's id with this agent's credential.
                   run_activity(&base, other, &hooks);
                   // Refused: no credential at all.
                   run_activity(&base, agent, "");
                   run_activity(&base, agent, &hooks);
                   wait_for(&recorded_for_blocking, 1)
               }).await
                 .expect("the scripts ran");

    // Give a refused post that is still in flight time to land wrongly.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(seen, vec![agent.to_string()]);
    assert_eq!(*recorded.0.lock(), vec![agent.to_string()]);
    server.stop();
}
