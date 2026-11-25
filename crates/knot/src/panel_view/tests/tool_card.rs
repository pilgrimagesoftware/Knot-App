//! Tests for the tool-call card: icons, status labels, outlines, diffs, element
//! ids, permission modes and the title shimmer.

use gpui_kit::assets::IconName;

use super::fixture::*;
use crate::panel_view::style::*;
use crate::panel_view::tool_call::*;
use crate::panel_view::*;

#[test]
fn every_known_kind_maps_to_a_distinct_icon_and_unknown_kinds_fall_back() {
    let known = ["read", "edit", "delete", "move", "search", "execute", "think", "fetch"];
    for kind in known {
        assert_ne!(tool_call_icon(kind),
                   IconName::Wrench,
                   "expected a specific icon for known kind {kind:?}");
    }
    assert_eq!(tool_call_icon("some-future-kind"), IconName::Wrench);
    assert_eq!(tool_call_icon(""), IconName::Wrench);
}

/// The bug this guards: keying the in-progress placeholder on "no
/// output yet" left a completed call showing "Running…" forever,
/// because a call can finish without ever reporting content.
#[test]
fn only_a_running_status_reads_as_running() {
    assert_eq!(status_label("in_progress"), "Running…");
    assert_eq!(status_label("pending"), "Pending");
    assert_eq!(status_label("completed"), "Done");
    assert_eq!(status_label("failed"), "Failed");
}

#[test]
fn an_unrecognized_status_passes_through_rather_than_vanishing() {
    assert_eq!(status_label("some_future_status"), "some_future_status");
}

#[test]
fn known_statuses_map_to_icons_and_unknown_statuses_stay_text() {
    assert_eq!(status_icon("pending"), Some(IconName::Loader));
    assert_eq!(status_icon("in_progress"), Some(IconName::Loader));
    assert_eq!(status_icon("completed"), Some(IconName::CircleCheck));
    assert_eq!(status_icon("failed"), Some(IconName::CircleX));
    assert_eq!(status_icon("some_future_status"), None);
}

#[test]
fn status_icon_color_tracks_card_outline() {
    assert_eq!(card_outline("failed"), CardOutline::Danger);
    assert_eq!(card_outline("pending"), CardOutline::Info);
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
}

#[test]
fn a_failed_call_is_the_only_status_outlined_in_danger() {
    assert_eq!(card_outline("failed"), CardOutline::Danger);
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
    assert_eq!(card_outline("pending"), CardOutline::Info);
    assert_eq!(card_outline("in_progress"), CardOutline::Info);
}

/// Success is the common case: outlining every finished call in a
/// colour of its own would leave the two states worth noticing - a
/// call still running and one that failed - looking like the rest.
#[test]
fn a_completed_call_recedes_to_the_neutral_border() {
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
}

/// `status` is a plain wire string, so a value this build has never
/// heard of must render as an ordinary card rather than a failure -
/// or panic.
#[test]
fn an_unrecognized_status_takes_the_neutral_border() {
    assert_eq!(card_outline("some_future_status"), CardOutline::Neutral);
    assert_eq!(card_outline(""), CardOutline::Neutral);
}

#[test]
fn a_replacement_diff_reads_as_removals_then_additions() {
    let lines = diff_lines(Some("a\nb"), "a\nc");

    assert_eq!(lines,
               vec![(ERROR_COLOR, "- a".to_string()),
                    (ERROR_COLOR, "- b".to_string()),
                    (SAFE_COLOR, "+ a".to_string()),
                    (SAFE_COLOR, "+ c".to_string())]);
}

#[test]
fn a_new_file_diff_is_all_additions_with_no_prefixes_to_strip() {
    let lines = diff_lines(None, "fn main() {}");

    assert_eq!(lines, vec![(MUTED, "fn main() {}".to_string())]);
}

#[test]
fn a_unified_diff_without_old_text_colors_by_prefix() {
    let lines = diff_lines(None, "-old\n+new\n context");

    assert_eq!(lines,
               vec![(ERROR_COLOR, "- old".to_string()),
                    (SAFE_COLOR, "+ new".to_string()),
                    (MUTED, " context".to_string())]);
}

#[test]
fn the_chevron_states_which_way_the_card_goes() {
    assert_eq!(disclosure_icon(true), IconName::ChevronRight);
    assert_eq!(disclosure_icon(false), IconName::ChevronDown);
}

/// Two cards in one conversation must not share a header element id,
/// or a click on one would carry the other's state.
#[test]
fn each_tool_call_id_gets_its_own_element_id() {
    assert_eq!(element_id("tc1"), element_id("tc1"));
    assert_ne!(element_id("tc1"), element_id("tc2"));
}

#[test]
fn permission_modes_are_classified_case_insensitively() {
    assert_eq!(permission_risk_level("bypassPermissions", "Restricted"),
               RiskLevel::Danger);
    assert_eq!(permission_risk_level("unknown", "PLAN"), RiskLevel::Safe);
    assert_eq!(permission_risk_level("default", "Normal"),
               RiskLevel::Neutral);
}

/// Codex's three modes, as `codex-acp` 1.13.1 advertises them: full access
/// is its counterpart to `bypassPermissions`, so it must not stay neutral.
#[test]
fn codex_permission_modes_are_classified() {
    assert_eq!(permission_risk_level("agent-full-access", "Mode"),
               RiskLevel::Danger);
    assert_eq!(permission_risk_level("unknown", "Full access"),
               RiskLevel::Danger);
    assert_eq!(permission_risk_level("read-only", "Mode"), RiskLevel::Safe);
    assert_eq!(permission_risk_level("agent", "Mode"), RiskLevel::Neutral);
}

/// A mode is matched whole, never by a word inside it:
/// a name or id that only contains "read", "plan" or "full access" is
/// neutral.
#[test]
fn a_mode_that_only_contains_a_known_word_is_neutral() {
    for (value, name) in [("custom", "Read and write all files"),
                          ("already", "Already approved"),
                          ("thread", "Thread"),
                          ("explanation", "Explanation"),
                          ("planner", "Planner"),
                          ("not-full-access", "Mostly full access")]
    {
        assert_eq!(permission_risk_level(value, name),
                   RiskLevel::Neutral,
                   "{value} / {name}");
    }
}

#[test]
fn known_modes_match_whatever_their_case_and_separators() {
    assert_eq!(permission_risk_level("Read-Only", "Mode"), RiskLevel::Safe);
    assert_eq!(permission_risk_level("read_only", "Mode"), RiskLevel::Safe);
    assert_eq!(permission_risk_level("yolo", "Mode"), RiskLevel::Danger);
    assert_eq!(permission_risk_level("danger-full-access", "Mode"),
               RiskLevel::Danger);
}

/// A call's title shimmers while it has not finished - pending, running, or
/// a status Knot does not know - and stops once it is done or failed (#499).
#[test]
fn a_tool_call_title_shimmers_until_it_finishes() {
    for status in ["pending", "in_progress", "some-future-status"] {
        assert!(title_shimmers(&tool_call(status)),
                "{status} should shimmer");
    }
    for status in ["completed", "failed"] {
        assert!(!title_shimmers(&tool_call(status)),
                "{status} should be still");
    }
}
