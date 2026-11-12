//! Slash tokens and `@` mentions.
//!
//! Both are "a trigger character, then everything up to whitespace", and
//! both are styled from the trigger onwards before the rest of the token
//! exists - the composer never waits to see whether a token resolves
//! (`panel-rich-input`, "A token being typed is styled as it grows").
//!
//! What separates a token from prose is only where the trigger sits:
//!
//! - `/` has to be the first non-whitespace character of its line, which is the
//!   rule [`crate::panel_commands::active_token`] already applies, so
//!   `crates/knot/src` is a path and not three tokens.
//! - `@` has to follow whitespace or begin the buffer, so the `@` in
//!   `paul@example.com` is part of an address.
//! - A skill trigger of its own - Codex's `$` (#272) - follows the `@` rule,
//!   since Codex reads `$name` anywhere in a prompt, and must open a name: the
//!   trigger alone while it is being typed, or one followed by a letter, so
//!   `costs $5` stays prose.
//!
//! The characters are the agent's, from `knot_core::agent_type::Invocation`;
//! nothing here assumes `/`.

use std::ops::Range;

use knot_core::agent_type::Invocation;

use crate::composer_scan::scan::Construct;
use crate::composer_scan::scan::Span;

/// Every token whose *line* lies within `range`.
///
/// Scanning by line rather than by byte is what makes the incremental path
/// safe: a token cannot span a newline, so a line's tokens depend on that
/// line alone.
pub(super) fn scan(text: &str, range: Range<usize>, invocation: Invocation) -> Vec<Span> {
    let mut spans = Vec::new();
    for (start, line) in super::markdown::lines_in(text, range) {
        scan_command(line, start, invocation.command, &mut spans);
        if invocation.separate_skills() {
            scan_skills(line, start, invocation.skill, &mut spans);
        }
        scan_mentions(line, start, &mut spans);
    }
    spans
}

/// The leading command - `/command` - if this line has one.
fn scan_command(line: &str, start: usize, trigger: char, spans: &mut Vec<Span>) {
    let indent = line.len() - line.trim_start().len();
    if !line[indent..].starts_with(trigger) {
        return;
    }
    let slash = start + indent;
    let end = token_end(line, indent, start);
    spans.push(Span::new(slash..end, Construct::SlashToken));
}

/// Every skill token - `$skill` - on this line, for an agent whose skills
/// have a trigger of their own.
fn scan_skills(line: &str, start: usize, trigger: char, spans: &mut Vec<Span>) {
    for (offset, _) in line.match_indices(trigger) {
        if !at_word_boundary(line, offset) {
            continue;
        }
        let opens_a_name = line[offset + trigger.len_utf8()..].chars()
                                                              .next()
                                                              .is_none_or(|next| {
                                                                  next.is_alphabetic()
                                                                  || next.is_whitespace()
                                                              });
        if !opens_a_name {
            continue;
        }
        let end = token_end(line, offset, start);
        spans.push(Span::new(start + offset..end, Construct::SkillToken));
    }
}

/// Every `@mention` on this line.
fn scan_mentions(line: &str, start: usize, spans: &mut Vec<Span>) {
    for (offset, _) in line.match_indices('@') {
        if !at_word_boundary(line, offset) {
            continue;
        }
        let end = token_end(line, offset, start);
        // A bare `@` with nothing after it is still a token: the trigger
        // character carries the treatment before the rest is typed.
        spans.push(Span::new(start + offset..end, Construct::Mention));
    }
}

/// Whether the trigger at `offset` begins a word - nothing before it, or
/// whitespace.
fn at_word_boundary(line: &str, offset: usize) -> bool {
    line[..offset].chars()
                  .next_back()
                  .is_none_or(char::is_whitespace)
}

/// The buffer offset one past the token that starts at `offset` in
/// `line`. Every trigger is one ASCII byte - `/`, `$`, `@` - which is what
/// lets the token's text start one byte after it.
///
/// A token ends at the first **unescaped** whitespace. A path containing a
/// space is a real path, and a mention of one has to stay one token or the
/// composer styles half of it and the lookup completes over a fragment -
/// so `\\ ` is a space *in* the token rather than the end of it. See
/// [`escape_token`] for the other half of the rule.
fn token_end(line: &str, offset: usize, start: usize) -> usize {
    let after_trigger = offset + 1;
    let rest = &line[after_trigger..];

    let mut escaped = false;
    for (index, character) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character.is_whitespace() {
            return start + after_trigger + index;
        }
    }
    start + after_trigger + rest.len()
}

/// `text` with the characters that would end a token escaped, so
/// inserting it produces one token.
///
/// The inverse of [`token_end`]'s rule, and the reason they live beside
/// each other: an escape the scanner does not understand splits the token
/// it was meant to hold together.
pub(crate) fn escape_token(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '\\' || character.is_whitespace() {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
