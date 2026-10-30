//! Unit tests for [`super`]: what a query matches, and where.

use super::FindMatch;
use super::FindMode;
use super::FindQuery;
use super::conversation_matches;
use crate::panel_state::PanelMessage;

fn query(text: &str, mode: FindMode) -> FindQuery {
    FindQuery::new(text, mode).expect("the query compiles")
                              .expect("the query is not empty")
}

#[test]
fn a_substring_matches_ignoring_case() {
    let found = query("gate", FindMode::Substring).find_all("Run the Gate, then the GATE");
    assert_eq!(found, [8..12, 23..27]);
}

/// The substring is taken literally: its regex syntax means nothing.
#[test]
fn a_substring_is_not_a_pattern() {
    let substring = query("a.c", FindMode::Substring);
    assert!(substring.find_all("abc").is_empty());
    assert_eq!(substring.find_all("a.c").first(), Some(&(0..3)));
}

#[test]
fn a_regex_matches_its_pattern() {
    let found = query(r"v\d+\.\d+", FindMode::Regex).find_all("from v1.2 to V10.4");
    assert_eq!(found, [5..9, 13..18]);
}

/// Case is ignored unless the pattern says otherwise.
#[test]
fn a_regex_can_turn_case_back_on() {
    assert!(query("(?-i)Gate", FindMode::Regex).find_all("gate")
                                               .is_empty());
}

#[test]
fn an_empty_query_finds_nothing() {
    assert!(FindQuery::new("", FindMode::Regex).expect("empty is valid")
                                               .is_none());
}

#[test]
fn an_unparsable_regex_is_an_error() {
    assert!(FindQuery::new("(unclosed", FindMode::Regex).is_err());
    assert!(FindQuery::new("(unclosed", FindMode::Substring).is_ok(),
            "the same text as a substring is fine");
}

/// A pattern that can match nothing - `x*` - finds only real text.
#[test]
fn empty_matches_are_not_matches() {
    let found = query("x*", FindMode::Regex).find_all("axxb");
    assert_eq!((found.len(), found.first()), (1, Some(&(1..3))));
}

/// Ranges index the original text, even where ignoring case changes a
/// character's length - so they are always safe to slice and to highlight.
#[test]
fn ranges_land_on_character_boundaries() {
    let text = "Straße İstanbul straße";
    for range in query("straße", FindMode::Substring).find_all(text) {
        assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
        assert_eq!(text[range].to_lowercase(), "straße");
    }
}

/// Prompts and replies are searched, in order; notices and errors are not.
#[test]
fn the_conversation_is_searched_in_order() {
    let messages = vec![PanelMessage::User("fix the gate".into()),
                        PanelMessage::Notice("gate notice".into()),
                        PanelMessage::Assistant("the gate, and the gate again".into()),
                        PanelMessage::Error("gate error".into())];

    let found = conversation_matches(&messages, &query("gate", FindMode::Substring));

    assert_eq!(found,
               [FindMatch { message: 0,
                            nth:     0, },
                FindMatch { message: 2,
                            nth:     0, },
                FindMatch { message: 2,
                            nth:     1, }]);
}

/// The current match is painted in its own colour, and only in its own
/// message.
#[test]
fn only_the_current_match_wears_the_current_colour() {
    use std::rc::Rc;

    use super::{FindColors, FindHighlight};

    let colors = FindColors { matched: gpui_kit::red(),
                              current: gpui_kit::blue(), };
    let find = FindHighlight { query: Rc::new(query("gate", FindMode::Substring)),
                               generation: 1,
                               current: Some(FindMatch { message: 2,
                                                         nth:     1, }),
                               colors };

    let here = find.ranges(2, "gate and gate");
    assert_eq!(here, [(0..4, colors.matched), (9..13, colors.current)]);
    let elsewhere = find.ranges(5, "gate and gate");
    assert!(elsewhere.iter().all(|(_, color)| *color == colors.matched));
}
