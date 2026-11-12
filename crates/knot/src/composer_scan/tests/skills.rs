//! Skill tokens for an agent whose skills have a character of their own -
//! Codex's `$skill` (#272) - and what stays prose.

use knot_core::agent_type::Invocation;

use crate::composer_scan::Construct;
use crate::composer_scan::scan;
use crate::composer_scan::tests::slices;

/// Codex's characters: commands behind `/`, skills behind `$`.
const CODEX: Invocation = Invocation { command: '/',
                                       skill:   '$', };

fn found(text: &str, invocation: Invocation, construct: Construct) -> Vec<&str> {
    slices(text, &scan(text, &[], invocation), construct)
}

#[test]
fn a_skill_token_is_found_anywhere_a_word_starts() {
    assert_eq!(found("$prd", CODEX, Construct::SkillToken), vec!["$prd"]);
    assert_eq!(found("use $prd for the plan", CODEX, Construct::SkillToken),
               vec!["$prd"],
               "Codex reads `$name` anywhere in a prompt, not only at a line's head");
}

/// `panel-rich-input`: a token being typed is styled as it grows.
#[test]
fn a_bare_skill_trigger_is_already_a_token() {
    assert_eq!(found("$", CODEX, Construct::SkillToken), vec!["$"]);
    assert_eq!(found("use $", CODEX, Construct::SkillToken),
               vec!["$"],
               "a trigger typed at the end of a line");
}

/// A `$` that opens no name is prose, including one followed by a space: only
/// a `$` at the end of its line is still being typed.
#[test]
fn a_price_or_a_variable_inside_a_word_stays_prose() {
    for text in ["it costs $5", "echo a$b", "100$", "pay $ 5", "$ alone"] {
        assert!(found(text, CODEX, Construct::SkillToken).is_empty(),
                "{text}");
    }
}

/// The command character keeps its own rule for an agent that splits them.
#[test]
fn commands_stay_behind_the_slash_for_codex() {
    assert_eq!(found("/compact now", CODEX, Construct::SlashToken),
               vec!["/compact"]);
    assert!(found("/compact now", CODEX, Construct::SkillToken).is_empty());
}

/// An agent whose skills share `/` has no `$` trigger to recognise.
#[test]
fn a_dollar_is_prose_for_an_agent_that_shares_the_slash() {
    assert!(found("$prd", Invocation::SLASH, Construct::SkillToken).is_empty());
}
