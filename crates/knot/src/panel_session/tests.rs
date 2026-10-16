//! Connecting a panel session: when the slot goes live, and which first
//! turns a connection sends.

use std::collections::BTreeMap;

use knot_agent_launch::AdapterConfig;

use super::*;

/// A fake adapter that completes the handshake but never answers
/// `session/prompt` - standing in for an agent whose first turn is
/// blocked (Gemini stalls its registration turn on a
/// `session/request_permission` for the first Knot MCP tool call,
/// which only a `Ready` slot can show and answer).
fn stalling_prompt_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
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

/// The registration turn must not gate the slot: an agent that asks
/// permission mid-registration can only be answered through a `Ready`
/// slot, so publishing the handle after the turn finishes deadlocks
/// the connection.
#[tokio::test]
async fn the_slot_goes_ready_before_the_registration_turn_finishes() {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config:                     &stalling_prompt_adapter(),
                                   cwd:                        "/tmp/project",
                                   prior_session_id:           None,
                                   mcp_url:                    None,
                                   registration_prompt:        Some("register".to_string()),
                                   resume_registration_prompt: None,
                                   session_config:             BTreeMap::new(),
                                   default_mode:               None,
                                   option_mode:                None,
                                   session_meta:               None,
                                   env:                        Vec::new(),
                                   args:                       Vec::new(),
                                   subagents:                  None,
                                   startup_prompt:             None,
                                   times:                      None, };

    let watched = Arc::clone(&slot);
    let became_ready = async move {
        for _ in 0..200 {
            let phase = watched.lock().phase();
            if phase == PanelPhase::Ready {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    };

    tokio::select! {
        ready = became_ready => assert!(
            ready,
            "the slot must reach Ready while the registration turn is still in flight"
        ),
        () = connect_into(&slot, request, &progress, |_| {}) => {
            panic!("the stalled registration turn should never finish")
        }
    }
}

/// A fake adapter that advertises `loadSession`, logs every method it is
/// sent to `$KNOT_TEST_LOG`, and loads or refuses a prior session as
/// `$KNOT_TEST_LOAD` says. Its load result is empty, the shape `codex-acp`
/// 2.0.0 answers with.
fn logging_loading_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  echo "$method" >> "$KNOT_TEST_LOG"
  case "$method" in
    initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true}}}" ;;
    session/load)
      if [ "$KNOT_TEST_LOAD" = refuse ]; then
        echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"error\":{\"code\":-32602,\"message\":\"no such session\"}}"
      else
        echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}"
      fi
      ;;
    session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-new\"}}" ;;
    session/prompt) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"stopReason\":\"end_turn\"}}" ;;
  esac
done"#,
    ],
                    supports_resume:           true,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// Connects to [`logging_loading_adapter`] naming prior session `thread-7`,
/// with a registration prompt ready, and returns the methods it was sent
/// and the session id the caller was handed to persist.
async fn connect_naming_a_prior_session(load: &str, resume_prompt: Option<&str>)
                                        -> (Vec<String>, String) {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("methods.log");
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config:                     &logging_loading_adapter(),
                                   cwd:                        "/tmp/project",
                                   prior_session_id:           Some("thread-7"),
                                   mcp_url:                    None,
                                   registration_prompt:        Some("register".to_string()),
                                   resume_registration_prompt: resume_prompt.map(str::to_owned),
                                   session_config:             BTreeMap::new(),
                                   default_mode:               None,
                                   option_mode:                None,
                                   session_meta:               None,
                                   env:                        vec![("KNOT_TEST_LOG".to_string(),
                                                                     log.display().to_string()),
                                                                    ("KNOT_TEST_LOAD".to_string(),
                                                                     load.to_string())],
                                   args:                       Vec::new(),
                                   subagents:                  None,
                                   startup_prompt:             None,
                                   times:                      None, };
    let mut persisted = String::new();
    connect_into(&slot, request, &progress, |id| persisted = id.to_string()).await;
    if let PanelSessionSlot::Ready(handle) = &*slot.lock() {
        let session = handle.session();
        tokio::spawn(async move { session.stop().await });
    }
    let methods = std::fs::read_to_string(&log).unwrap_or_default()
                                               .lines()
                                               .map(str::to_string)
                                               .collect();
    (methods, persisted)
}

/// A resumed agent is already registered, so its first turn is not the
/// registration request - and the id persisted for the next launch is the
/// loaded one, so the conversation stays resumable.
#[tokio::test]
async fn a_loaded_session_sends_no_registration_turn() {
    let (methods, persisted) = connect_naming_a_prior_session("load", None).await;

    assert!(methods.contains(&"session/load".to_string()), "{methods:?}");
    assert!(!methods.contains(&"session/new".to_string()), "{methods:?}");
    assert!(!methods.contains(&"session/prompt".to_string()),
            "a resumed session is not re-registered: {methods:?}");
    assert_eq!(persisted, "thread-7");
}

/// The case the prior-session id alone got wrong: the adapter could not
/// load it, so the session is fresh and must be registered like any other.
#[tokio::test]
async fn a_refused_load_registers_the_fresh_session_it_falls_back_to() {
    let (methods, persisted) = connect_naming_a_prior_session("refuse", None).await;

    assert!(methods.contains(&"session/new".to_string()), "{methods:?}");
    assert!(methods.contains(&"session/prompt".to_string()),
            "the fallback session is sent the registration turn: {methods:?}");
    assert_eq!(persisted, "sess-new");
}

/// #552: an adapter whose connection cannot register the agent is sent the
/// one-line registration request once the session has resumed - and only
/// then, so the conversation gains one turn, not a repeat of the first.
#[tokio::test]
async fn a_loaded_session_is_sent_the_resume_registration_where_needed() {
    let (methods, persisted) = connect_naming_a_prior_session("load", Some("re-register")).await;

    assert!(methods.contains(&"session/load".to_string()), "{methods:?}");
    assert_eq!(methods.iter().filter(|m| *m == "session/prompt").count(),
               1,
               "one registration turn: {methods:?}");
    assert_eq!(persisted, "thread-7");
}

/// A fake adapter whose `session/load` replays one exchange - a prompt and
/// its reply - as session updates before answering, the way an adapter
/// replays a loaded session's history, with no times attached. With
/// `$KNOT_TEST_TURN_END=1` the replay also carries a turn end.
fn replaying_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
    initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true}}}" ;;
    session/load)
      echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"thread-7\",\"update\":{\"sessionUpdate\":\"user_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"fix the build\"}}}}"
      echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"thread-7\",\"update\":{\"sessionUpdate\":\"agent_message_chunk\",\"content\":{\"type\":\"text\",\"text\":\"fixed\"}}}}"
      if [ "$KNOT_TEST_TURN_END" = 1 ]; then
        echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionId\":\"thread-7\",\"update\":{\"sessionUpdate\":\"turn_end\",\"stopReason\":\"end_turn\"}}}"
      fi
      echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}"
      ;;
    session/prompt) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"stopReason\":\"end_turn\"}}" ;;
  esac
done"#,
    ],
                    supports_resume:           true,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// #220, end to end: a restored session's replayed prompt and reply show the
/// times they were logged with - not the moment they were replayed - once
/// the first live prompt ends the replay, and the session's times are saved
/// again when that turn ends.
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_session_keeps_its_entries_times() {
    use crate::panel_state::{PanelMessage, TimedEntry, prompt_hash};

    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let store = TimesStore::new(dir.path().join("times"));
    let logged = vec![TimedEntry::Prompt { hash:  prompt_hash("fix the build"),
                                           at_ms: 1_700_000_000_000, },
                      TimedEntry::Reply { at_ms: 1_700_000_060_000, }];
    store.save("thread-7", &logged).expect("seeds the log");

    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config:                     &replaying_adapter(),
                                   cwd:                        "/tmp/project",
                                   prior_session_id:           Some("thread-7"),
                                   mcp_url:                    None,
                                   registration_prompt:        None,
                                   resume_registration_prompt: None,
                                   session_config:             BTreeMap::new(),
                                   default_mode:               None,
                                   option_mode:                None,
                                   session_meta:               None,
                                   env:                        Vec::new(),
                                   args:                       Vec::new(),
                                   subagents:                  None,
                                   startup_prompt:             None,
                                   times:                      Some(store.clone()), };
    connect_into(&slot, request, &progress, |_| {}).await;
    let (state, session) = match &*slot.lock() {
        PanelSessionSlot::Ready(handle) => (handle.state(), handle.session()),
        _ => panic!("the session did not connect"),
    };

    // The replay is folded by the session's own task; wait for it.
    for _ in 0..200 {
        if state.lock().messages.len() == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    {
        let state = state.lock();
        assert_eq!(state.messages,
                   vec![PanelMessage::User("fix the build".into()),
                        PanelMessage::Assistant("fixed".into())]);
        assert_eq!(state.sent_at,
                   vec![None, None],
                   "not stamped with the replay's moment");
    }

    // The first live prompt, as the composer sends one.
    state.lock().push_prompt("next".into(), true);
    session.prompt("next")
           .await
           .expect("the prompt is answered");
    let at = |ms: u64| Some(std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms));
    assert_eq!(state.lock().sent_at[..2],
               [at(1_700_000_000_000), at(1_700_000_060_000)]);

    // Saved when that turn ended: the restored entries, then the new prompt.
    let mut saved = Vec::new();
    for _ in 0..200 {
        saved = store.load("thread-7");
        if saved.len() == 3 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(saved[..2], logged[..]);
    assert!(matches!(&saved[2], TimedEntry::Prompt { hash, .. } if *hash == prompt_hash("next")));
    session.stop().await;
}

/// The times a restored `thread-7` was logged with, seeded into a fresh store
/// as pretty-printed JSON - which no save writes, so a test can tell once the
/// session has written the document back.
fn seeded_store(dir: &std::path::Path)
                -> (TimesStore, Vec<crate::panel_state::TimedEntry>, Vec<u8>) {
    use crate::panel_state::{TimedEntry, prompt_hash};

    let times = dir.join("times");
    let logged = vec![TimedEntry::Prompt { hash:  prompt_hash("fix the build"),
                                           at_ms: 1_700_000_000_000, },
                      TimedEntry::Reply { at_ms: 1_700_000_060_000, }];
    std::fs::create_dir_all(&times).expect("creates the directory");
    let seed = serde_json::to_vec_pretty(&logged).expect("serializes");
    std::fs::write(times.join("thread-7.json"), &seed).expect("seeds the log");
    (TimesStore::new(times), logged, seed)
}

/// Restores `thread-7` from [`replaying_adapter`] with `env`, waits for the
/// replay to be folded, and hands back the session to end.
async fn restore_thread_7(store: &TimesStore, env: Vec<(String, String)>)
                          -> knot_terminal::AcpSession {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config: &replaying_adapter(),
                                   cwd: "/tmp/project",
                                   prior_session_id: Some("thread-7"),
                                   mcp_url: None,
                                   registration_prompt: None,
                                   resume_registration_prompt: None,
                                   session_config: BTreeMap::new(),
                                   default_mode: None,
                                   option_mode: None,
                                   session_meta: None,
                                   env,
                                   args: Vec::new(),
                                   subagents: None,
                                   startup_prompt: None,
                                   times: Some(store.clone()) };
    connect_into(&slot, request, &progress, |_| {}).await;
    let (state, session) = match &*slot.lock() {
        PanelSessionSlot::Ready(handle) => (handle.state(), handle.session()),
        _ => panic!("the session did not connect"),
    };
    for _ in 0..200 {
        if state.lock().messages.len() == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    session
}

/// The document's bytes once a save has replaced `seed`, or a panic.
async fn saved_over(store: &TimesStore, dir: &std::path::Path, seed: &[u8])
                    -> Vec<crate::panel_state::TimedEntry> {
    let path = dir.join("times").join("thread-7.json");
    for _ in 0..300 {
        if std::fs::read(&path).is_ok_and(|bytes| bytes != seed) {
            return store.load("thread-7");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("the session never saved its times");
}

/// Open a restored conversation, look at it, close it: the session ends
/// before any live prompt, and what it saves on ending must still be the
/// restored times - not the nothing it has logged live.
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_session_ending_before_a_prompt_keeps_its_times() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let (store, logged, seed) = seeded_store(dir.path());

    let session = restore_thread_7(&store, Vec::new()).await;
    session.stop().await;

    assert_eq!(saved_over(&store, dir.path(), &seed).await, logged);
}

/// A turn end inside the replay saves too, and must not wipe the times the
/// replay has not been reconciled with yet.
#[tokio::test(flavor = "multi_thread")]
async fn a_turn_end_during_the_replay_keeps_the_restored_times() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let (store, logged, seed) = seeded_store(dir.path());

    let session = restore_thread_7(&store,
                                   vec![("KNOT_TEST_TURN_END".to_string(), "1".to_string())]).await;

    assert_eq!(saved_over(&store, dir.path(), &seed).await, logged);
    session.stop().await;
}
