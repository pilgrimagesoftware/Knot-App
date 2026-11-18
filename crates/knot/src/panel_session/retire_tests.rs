//! Retiring a slot stops its adapter, whichever phase the connect is in
//! (#382).
//!
//! Each test runs a real `sh` stand-in for the adapter that writes its own
//! pid to `$KNOT_TEST_PID` the moment it starts, so the assertion is on the
//! process itself rather than on what the slot says about it: a retired
//! connect that leaves its adapter running is exactly the defect, and the
//! slot cannot see that.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use knot_agent_launch::AdapterConfig;

use super::*;

/// Records its pid, then completes the handshake and never answers a
/// prompt - or, with `$KNOT_TEST_HANG` set, never answers `initialize`
/// either, which holds the connect mid-start for as long as a test needs.
fn pid_recording_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"echo $$ > "$KNOT_TEST_PID"
while IFS= read -r line; do
  [ -n "$KNOT_TEST_HANG" ] && continue
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
    initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
    session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
  esac
done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// How long a test waits for something that should happen promptly before
/// calling it stuck. Not a budget the machine has to beat.
const HANG_GUARD: Duration = Duration::from_secs(60);

fn request<'a>(config: &'a AdapterConfig, env: Vec<(String, String)>) -> ConnectRequest<'a> {
    ConnectRequest { agent_id: uuid::Uuid::nil(),
                     config,
                     cwd: "/tmp",
                     prior_session_id: None,
                     mcp_url: None,
                     registration_prompt: Some("register".to_string()),
                     resume_registration_prompt: None,
                     session_config: BTreeMap::new(),
                     default_mode: None,
                     option_mode: None,
                     session_meta: None,
                     env,
                     args: Vec::new(),
                     subagents: None,
                     startup_prompt: None,
                     times: None }
}

/// The adapter's pid, once it has written it.
fn recorded_pid(file: &Path) -> Option<u32> {
    std::fs::read_to_string(file).ok()?.trim().parse().ok()
}

/// Whether `pid` is a running process. A zombie counts as gone: it has
/// been killed and is only waiting to be reaped.
fn running(pid: u32) -> bool {
    let output = std::process::Command::new("ps").args(["-o", "stat=", "-p", &pid.to_string()])
                                                 .output()
                                                 .expect("ps runs");
    let stat = String::from_utf8_lossy(&output.stdout);
    let stat = stat.trim();
    !stat.is_empty() && !stat.starts_with('Z')
}

async fn wait_until(mut condition: impl FnMut() -> bool, what: &str) {
    let waited = tokio::time::timeout(HANG_GUARD, async {
                     while !condition() {
                         tokio::time::sleep(Duration::from_millis(10)).await;
                     }
                 }).await;
    assert!(waited.is_ok(), "{what} within {HANG_GUARD:?}");
}

/// Retires `slot` the way the window does: a `Ready` handle it hands back
/// is stopped.
async fn retire(slot: &Mutex<PanelSessionSlot>) {
    let handle = slot.lock().retire();
    if let Some(handle) = handle {
        handle.stop().await;
    }
}

/// A slot torn down while its adapter is still starting: the connect gives
/// up, the adapter it spawned dies, nothing is published and no session id
/// is reported.
#[tokio::test]
async fn retiring_a_connecting_slot_kills_the_adapter_it_was_starting() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let pid_file = dir.path().join("pid");
    let config = pid_recording_adapter();
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let reported = Arc::new(Mutex::new(None::<String>));

    let task = {
        let (slot, reported) = (Arc::clone(&slot), Arc::clone(&reported));
        let env = vec![("KNOT_TEST_PID".to_string(), pid_file.display().to_string()),
                       ("KNOT_TEST_HANG".to_string(), "1".to_string())];
        tokio::spawn(async move {
            let config = config;
            connect_into(&slot, request(&config, env), &progress, |id| {
                *reported.lock() = Some(id.to_string());
            }).await;
        })
    };
    wait_until(|| recorded_pid(&pid_file).is_some(), "the adapter started").await;
    let pid = recorded_pid(&pid_file).expect("the pid was just read");
    assert!(running(pid), "the adapter is up and waiting on initialize");

    retire(&slot).await;

    tokio::time::timeout(HANG_GUARD, task).await
                                          .expect("the connect gives up once its slot is retired")
                                          .expect("the connect task does not panic");
    wait_until(|| !running(pid), "the adapter was killed").await;
    assert_eq!(slot.lock().phase(),
               PanelPhase::Retired,
               "nothing was published over the retire");
    assert_eq!(*reported.lock(),
               None,
               "a retired attempt reports no session id");
}

/// The concurrent-trigger race: a teardown on one thread against a connect
/// finishing on another, swept across the whole connect so the retire
/// lands before the spawn, mid-handshake, between the handshake and the
/// publish, and after it. Whichever order wins, the slot ends retired and
/// no adapter is left running - the two halves of #382's "at most one".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retire_racing_the_connect_never_leaves_an_adapter_running() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    for step in 0..24_u64 {
        let pid_file = dir.path().join(format!("pid-{step}"));
        let config = pid_recording_adapter();
        let (connecting, progress) = PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        let task = {
            let slot = Arc::clone(&slot);
            let env = vec![("KNOT_TEST_PID".to_string(), pid_file.display().to_string())];
            tokio::spawn(async move {
                let config = config;
                connect_into(&slot, request(&config, env), &progress, |_| {}).await;
            })
        };

        tokio::time::sleep(Duration::from_millis(step * 4)).await;
        retire(&slot).await;

        tokio::time::timeout(HANG_GUARD, task)
            .await
            .unwrap_or_else(|_| panic!("step {step}: the connect did not end after the retire"))
            .expect("the connect task does not panic");
        assert_eq!(slot.lock().phase(),
                   PanelPhase::Retired,
                   "step {step}: the retire stands, whatever the connect did after it");
        // Retired before the spawn: no adapter, so nothing to check.
        if let Some(pid) = recorded_pid(&pid_file) {
            wait_until(|| !running(pid),
                       &format!("step {step}: adapter {pid} was stopped")).await;
        }
    }
}

/// A connect handed a slot that is already retired starts nothing.
#[tokio::test]
async fn a_connect_into_a_retired_slot_spawns_no_adapter() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let pid_file = dir.path().join("pid");
    let config = pid_recording_adapter();
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    retire(&slot).await;

    let env = vec![("KNOT_TEST_PID".to_string(), pid_file.display().to_string())];
    connect_into(&slot, request(&config, env), &progress, |_| {}).await;

    assert_eq!(recorded_pid(&pid_file), None, "no adapter was spawned");
    assert_eq!(slot.lock().phase(), PanelPhase::Retired);
}
