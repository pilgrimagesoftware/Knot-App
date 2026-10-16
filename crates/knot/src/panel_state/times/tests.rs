//! Restored conversation times (#220), through `PanelState`'s own fold: a
//! live session's entries are logged, and a restored session's replay of
//! them takes those times back instead of the moment it was replayed.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use knot_acp::{SessionEvent, SessionUpdate};

use super::{TimedEntry, prompt_hash};
use crate::panel_state::{PanelMessage, PanelState};

fn user(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::UserMessageChunk { text: text.to_string(),
                                                           meta: None, })
}

fn agent(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_string(), })
}

fn tool_call(id: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         "read".to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    None,
                                                        meta:         None, })
}

fn turn_end() -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: "end_turn".to_string(), })
}

/// A time as the log keeps it: to the millisecond.
fn to_ms(at: SystemTime) -> SystemTime {
    let ms = at.duration_since(UNIX_EPOCH)
               .expect("after the epoch")
               .as_millis();
    UNIX_EPOCH + Duration::from_millis(u64::try_from(ms).expect("fits"))
}

/// Moves the clock on far enough that consecutive entries differ.
fn tick() {
    std::thread::sleep(Duration::from_millis(3));
}

/// A live session: two turns, the second with a tool call, plus the local
/// entries a replay never carries.
fn live_session() -> PanelState {
    let mut state = PanelState::new();
    state.push_prompt("register".into(), true);
    tick();
    state.apply(agent("registered"));
    state.apply(turn_end());
    tick();
    state.push_notice("a local notice".into());
    state.push_prompt("fix the build".into(), true);
    tick();
    state.apply(tool_call("call-1"));
    tick();
    state.apply(agent("fixed"));
    state.apply(turn_end());
    state
}

/// What a `session/load` replays of [`live_session`]: its prompts, replies
/// and tool call, without the notice.
fn replay(state: &mut PanelState) {
    for event in [user("register"),
                  agent("registered"),
                  user("fix the build"),
                  tool_call("call-1"),
                  agent("fixed")]
    {
        state.apply(event);
    }
}

/// The time of each prompt and reply, in order.
fn displayed_times(state: &PanelState) -> Vec<Option<SystemTime>> {
    state.messages
         .iter()
         .zip(&state.sent_at)
         .filter(|(message, _)| {
             matches!(message, PanelMessage::User(_) | PanelMessage::Assistant(_))
         })
         .map(|(_, at)| *at)
         .collect()
}

#[test]
fn a_live_entry_is_stamped_now_and_logged() {
    let before = SystemTime::now();
    let mut state = PanelState::new();
    state.push_prompt("hello".into(), true);
    let at = state.sent_at[0].expect("a live prompt has a time");
    assert!(at >= before && at <= SystemTime::now());
    assert!(matches!(state.times.log(), [TimedEntry::Prompt { hash, .. }] if *hash == prompt_hash("hello")));
}

#[test]
fn a_restored_session_shows_when_its_entries_happened() {
    let live = live_session();
    let original: Vec<_> = displayed_times(&live).into_iter()
                                                 .map(|at| at.map(to_ms))
                                                 .collect();

    let mut restored = PanelState::restoring(live.times.log().to_vec());
    replay(&mut restored);
    assert!(displayed_times(&restored).iter().all(Option::is_none),
            "replayed entries are not stamped with the moment of replay");

    // The first live prompt ends the replay.
    restored.push_prompt("next".into(), true);
    let mut times = displayed_times(&restored);
    let next = times.pop().expect("the live prompt");
    assert!(next.is_some_and(|at| at > original.last().copied().flatten().expect("a time")));
    assert_eq!(times, original);
}

/// A prompt the log never saw - injected by the agent's harness, say - gets
/// no time, and the entries around it keep theirs.
#[test]
fn an_unknown_replayed_prompt_has_no_time_and_does_not_shift_the_rest() {
    let live = live_session();
    let original: Vec<_> = displayed_times(&live).into_iter()
                                                 .map(|at| at.map(to_ms))
                                                 .collect();

    let mut restored = PanelState::restoring(live.times.log().to_vec());
    for event in [user("injected by the harness"),
                  agent("ok"),
                  user("register"),
                  agent("registered"),
                  user("fix the build"),
                  tool_call("call-1"),
                  agent("fixed")]
    {
        restored.apply(event);
    }
    restored.push_prompt("next".into(), true);

    let times = displayed_times(&restored);
    assert_eq!(times[..2], [None, None]);
    assert_eq!(times[2..6], original[..]);
}

/// A session that could not be loaded is a fresh one: its entries happen now.
#[test]
fn a_discarded_restore_stamps_entries_live() {
    let mut state = PanelState::restoring(live_session().times.log().to_vec());
    state.times.discard_restore();
    state.apply(agent("hello"));
    assert!(state.sent_at[0].is_some());
}

/// Stored, so its encoding is a contract: camelCase, tagged by kind.
#[test]
fn a_logged_entry_keeps_its_stored_shape() {
    let entry = TimedEntry::ToolCall { id:    "call-1".into(),
                                       at_ms: 42, };
    let json = serde_json::to_value(&entry).expect("serializes");
    assert_eq!(json,
               serde_json::json!({ "kind": "toolCall", "id": "call-1", "atMs": 42 }));
    assert_eq!(serde_json::from_value::<TimedEntry>(json).expect("deserializes"),
               entry);
}

/// FNV-1a's published value for "a", so a stored hash means the same thing in
/// every build. Trimmed first.
#[test]
fn the_prompt_hash_is_stable() {
    assert_eq!(prompt_hash("a"), 0xAF63_DC4C_8601_EC8C);
    assert_eq!(prompt_hash("  a\n"), prompt_hash("a"));
}

/// What a restored session saves before its first live prompt is the
/// restored log, not the empty live one.
#[test]
fn a_replaying_session_snapshots_its_restored_log() {
    let restored = live_session().times.log().to_vec();
    let mut state = PanelState::restoring(restored.clone());
    replay(&mut state);
    assert!(state.times.log().is_empty());
    assert_eq!(state.times.snapshot(), restored);
}
