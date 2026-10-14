//! Reading back through a conversation moves it by exactly what the user
//! scrolled (#246).
//!
//! The list is virtualized: a row is measured when it first comes within
//! `LIST_OVERDRAW` of the viewport, and that height is what the next wheel
//! event is converted against. A row whose height changes *after* it was
//! measured - a long reply whose Markdown finishes parsing on a background
//! task a frame later - moves everything below it once it becomes the row
//! the viewport is anchored on, and the reader sees the conversation jump.
//! These drive the real panel rows through real wheel events, so the jump
//! shows up as a reference row landing somewhere other than where the
//! scroll put it.

use gpui_kit::ListState;
use gpui_kit::ScrollDelta;
use gpui_kit::ScrollWheelEvent;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::point;
use gpui_kit::px;
use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;

use super::panel_scroll::VIEWPORT_HEIGHT;
use super::panel_scroll::panel_probe_with;
use super::panel_scroll::redraw;
use crate::app_support::CHECK_INBOX_PROMPT;
use crate::panel_state::PanelState;
use crate::panel_view;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// How many question-and-reply turns the conversation holds: enough that
/// most of it starts outside the viewport and its overdraw.
const TURNS: usize = 12;

/// One wheel notch, upwards.
const NOTCH: f32 = 60.;

/// How far a reference row may land from where the scroll put it: layout
/// rounds to the pixel.
const TOLERANCE: f32 = 1.;

/// A reply long enough that `TextView` parses it on a background task
/// rather than during its first layout - its synchronous limit is 4 KiB.
fn long_reply(turn: usize) -> String {
    (0..24).map(|paragraph| {
               format!("Reply {turn}, paragraph {paragraph}: {}\n\n",
                       "a sentence the agent wrote while explaining its change. ".repeat(4))
           })
           .collect()
}

/// A finished conversation of [`TURNS`] long replies.
fn conversation() -> PanelState {
    let mut state = PanelState::new();
    for turn in 0..TURNS {
        state.push_user_message(format!("Question {turn}"));
        state.apply(SessionEvent::Update(SessionUpdate::TextDelta { text: long_reply(turn), }));
    }
    assert!(long_reply(0).len() > 4 * 1024,
            "the reply has to be past TextView's synchronous parse limit to exercise the defect");
    state.turn_active = false;
    state
}

/// The first row whose top edge is inside the viewport, and that top edge:
/// a row the reader can see, whose position after a scroll says where the
/// content went.
fn reference_row(list: &ListState) -> Option<(usize, f32)> {
    let viewport = list.viewport_bounds();
    let first = list.logical_scroll_top().item_ix;
    (first..list.item_count()).find_map(|ix| {
                                  let bounds = list.bounds_for_item(ix)?;
                                  (bounds.top() >= viewport.top()).then(|| {
                                                                      (ix, f32::from(bounds.top()))
                                                                  })
                              })
}

/// Turns the wheel one notch upwards over the list and lets every
/// background parse it started land and repaint.
fn scroll_up_one_notch(probe_cx: &mut VisualTestContext, list: &ListState) {
    let centre = list.viewport_bounds().center();
    probe_cx.simulate_event(ScrollWheelEvent { position: centre,
                                               delta: ScrollDelta::Pixels(point(px(0.),
                                                                                px(NOTCH))),
                                               ..Default::default() });
    probe_cx.run_until_parked();
}

/// The reported defect: scrolling back through long replies, one notch at a
/// time, every notch moves the content by one notch - never by a notch plus
/// whatever a reply above grew by after it was measured.
#[gpui_kit::test]
fn scrolling_back_through_long_replies_moves_by_exactly_the_scroll(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, _state, list) = panel_probe_with(cx, conversation());
    redraw(&mut probe_cx, &probe);
    assert!(list.viewport_bounds().size.height <= px(VIEWPORT_HEIGHT));

    let mut notches = 0;
    while list.logical_scroll_top().item_ix > 0 && notches < 2_000 {
        let (ix, before) = reference_row(&list).expect("some row is always on screen");
        scroll_up_one_notch(&mut probe_cx, &list);
        redraw(&mut probe_cx, &probe);
        notches += 1;
        // The row can scroll out of the bottom of the viewport, in which
        // case it is no longer measured against anything.
        let Some(after) = list.bounds_for_item(ix)
        else {
            continue;
        };
        let moved = f32::from(after.top()) - before;
        assert!((moved - NOTCH).abs() <= TOLERANCE,
                "notch {notches}: row {ix} moved {moved}px for a {NOTCH}px scroll - the \
                 conversation jumped while the user was reading back");
    }
    assert!(notches > 10,
            "the conversation should have taken many notches to read back");
}

/// Reading back while the agent is still answering: the reply growing at
/// the tail must not pull the reader's view anywhere.
#[gpui_kit::test]
fn a_streaming_reply_leaves_a_reader_further_up_where_they_are(cx: &mut TestAppContext) {
    let mut state = conversation();
    state.push_user_message("One more question".to_string());
    let (mut probe_cx, probe, state, list) = panel_probe_with(cx, state);
    redraw(&mut probe_cx, &probe);

    for _ in 0..30 {
        scroll_up_one_notch(&mut probe_cx, &list);
    }
    redraw(&mut probe_cx, &probe);
    let (ix, before) = reference_row(&list).expect("some row is on screen");

    for chunk in 0..20 {
        state.lock().apply(SessionEvent::Update(SessionUpdate::TextDelta {
            text: format!("Streamed chunk {chunk} of the answer still being written. "),
        }));
        redraw(&mut probe_cx, &probe);
    }

    let after = list.bounds_for_item(ix)
                    .expect("nothing scrolled, so the reference row is still on screen");
    assert!((f32::from(after.top()) - before).abs() <= TOLERANCE,
            "row {ix} moved from {before} to {} while the reader was further up",
            f32::from(after.top()));
}

/// Records a prompt the way `deliver_panel_prompt` does: whether its answer
/// is followed depends on where it came from and where the reader is.
fn deliver(state: &parking_lot::Mutex<PanelState>, list: &ListState, text: &str,
           origin: PromptOrigin) {
    let reader_at_tail = !panel_view::scrolled_away_from_tail(list, list.item_count());
    state.lock()
         .push_prompt(text.to_string(), origin.follows(reader_at_tail));
}

/// The inbox nudge another agent's message triggers is Knot's prompt, not
/// the reader's: it starts a turn, but leaves someone reading back through
/// the history exactly where they were (#246).
#[gpui_kit::test]
fn an_inbox_nudge_leaves_a_reader_further_up_where_they_are(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, state, list) = panel_probe_with(cx, conversation());
    redraw(&mut probe_cx, &probe);
    for _ in 0..30 {
        scroll_up_one_notch(&mut probe_cx, &list);
    }
    redraw(&mut probe_cx, &probe);
    let (ix, before) = reference_row(&list).expect("some row is on screen");

    deliver(&state, &list, CHECK_INBOX_PROMPT, PromptOrigin::InboxNudge);
    redraw(&mut probe_cx, &probe);
    state.lock().apply(SessionEvent::Update(SessionUpdate::TextDelta {
        text: "Checking my inbox now.".to_string(),
    }));
    redraw(&mut probe_cx, &probe);

    let after = list.bounds_for_item(ix)
                    .expect("the nudge should not have scrolled the reference row away");
    assert!((f32::from(after.top()) - before).abs() <= TOLERANCE,
            "row {ix} moved from {before} to {} when the inbox nudge arrived",
            f32::from(after.top()));
    assert!(panel_view::scrolled_away_from_tail(&list, list.item_count()),
            "the reader is still further up, so the jump to the latest stays offered");
}

/// The other half: a prompt the user typed takes them to its answer, even
/// from further up.
#[gpui_kit::test]
fn the_users_own_prompt_brings_them_to_its_answer(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, state, list) = panel_probe_with(cx, conversation());
    redraw(&mut probe_cx, &probe);
    for _ in 0..30 {
        scroll_up_one_notch(&mut probe_cx, &list);
    }
    redraw(&mut probe_cx, &probe);
    assert!(panel_view::scrolled_away_from_tail(&list, list.item_count()));

    deliver(&state,
            &list,
            "A question the user typed",
            PromptOrigin::User);
    redraw(&mut probe_cx, &probe);
    redraw(&mut probe_cx, &probe);

    assert!(!panel_view::scrolled_away_from_tail(&list, list.item_count()),
            "the user's own prompt should bring the conversation to its newest row");
}
