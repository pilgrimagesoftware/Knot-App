//! Showing text from an MCP call so that none of it can hide.
//!
//! A confirmation dialog is only as good as what it shows. A carriage
//! return or a run of backspaces can overwrite the visible command, an
//! escape sequence can restyle it, and a bidirectional override or a
//! zero-width character can reorder or hide part of it - so the person
//! approving sees something other than what would run (security review of
//! #516 and #522).

/// `text` with every character in [`hides_text`]'s classes written out as
/// `\u{..}` instead of obeyed.
pub(in crate::workspace_window) fn visible(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        // A literal backslash is doubled, so text that spells out
        // `\u{..}` cannot pass for one of the escapes below.
        if character == '\\' {
            out.push_str("\\\\");
        }
        else if hides_text(character) {
            out.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        }
        else {
            out.push(character);
        }
    }
    out
}

/// Whether `character` can hide or rearrange text: a C0 or C1 control or
/// DEL (`char::is_control`), a line or paragraph separator, or a Unicode
/// format character (general category Cf), which includes every
/// bidirectional override, embedding and isolate, the zero-width characters,
/// the byte-order mark and the tag characters.
fn hides_text(character: char) -> bool {
    character.is_control()
    || matches!(character, '\u{2028}' | '\u{2029}')
    || is_format(character)
    || is_invisible_letter(character)
}

/// Characters that draw as nothing but are not format characters: the
/// Hangul fillers, the braille blank, and the variation selectors, which
/// a font may render as nothing at all (review of #522).
fn is_invisible_letter(character: char) -> bool {
    matches!(character,
             '\u{115f}'
             | '\u{1160}'
             | '\u{3164}'
             | '\u{ffa0}'
             | '\u{2800}'
             | '\u{180b}'..='\u{180f}'
             | '\u{fe00}'..='\u{fe0f}'
             | '\u{e0100}'..='\u{e01ef}')
}

/// Unicode general category Cf, as of Unicode 16. Listed rather than taken
/// from a crate: the set is small, changes rarely, and a stale entry only
/// leaves a character unescaped that was never shown by any font.
fn is_format(character: char) -> bool {
    matches!(character,
             '\u{00ad}'
             | '\u{0600}'..='\u{0605}'
             | '\u{061c}'
             | '\u{06dd}'
             | '\u{070f}'
             | '\u{0890}'..='\u{0891}'
             | '\u{08e2}'
             | '\u{180e}'
             | '\u{200b}'..='\u{200f}'
             | '\u{202a}'..='\u{202e}'
             | '\u{2060}'..='\u{2064}'
             | '\u{2066}'..='\u{206f}'
             | '\u{feff}'
             | '\u{fff9}'..='\u{fffb}'
             | '\u{110bd}'
             | '\u{110cd}'
             | '\u{13430}'..='\u{1343f}'
             | '\u{1bca0}'..='\u{1bca3}'
             | '\u{1d173}'..='\u{1d17a}'
             | '\u{e0001}'
             | '\u{e0020}'..='\u{e007f}')
}

#[cfg(test)]
mod tests {
    use super::visible;

    /// One case per class Security and B3 named, each shown as `\u{..}` and
    /// never passed through.
    #[test]
    fn every_hiding_class_is_written_out() {
        let classes = [("C0 carriage return", '\r'),
                       ("C0 escape", '\u{1b}'),
                       ("C0 backspace", '\u{8}'),
                       ("C0 newline", '\n'),
                       ("C0 NUL", '\0'),
                       ("DEL", '\u{7f}'),
                       ("C1 CSI", '\u{9b}'),
                       ("bidi override RLO", '\u{202e}'),
                       ("bidi embedding LRE", '\u{202a}'),
                       ("bidi isolate RLI", '\u{2067}'),
                       ("bidi isolate end PDI", '\u{2069}'),
                       ("bidi mark RLM", '\u{200f}'),
                       ("zero-width space", '\u{200b}'),
                       ("zero-width joiner", '\u{200d}'),
                       ("word joiner", '\u{2060}'),
                       ("byte-order mark", '\u{feff}'),
                       ("soft hyphen (Cf)", '\u{ad}'),
                       ("tag character (Cf)", '\u{e0041}'),
                       ("line separator", '\u{2028}'),
                       ("Hangul choseong filler", '\u{115f}'),
                       ("Hangul jungseong filler", '\u{1160}'),
                       ("Hangul filler", '\u{3164}'),
                       ("halfwidth Hangul filler", '\u{ffa0}'),
                       ("braille blank", '\u{2800}'),
                       ("Mongolian free variation selector", '\u{180b}'),
                       ("variation selector 16", '\u{fe0f}'),
                       ("variation selector supplement", '\u{e0100}')];
        for (class, character) in classes {
            let shown = visible(&format!("a{character}b"));
            assert!(!shown.contains(character),
                    "{class} passed through: {shown:?}");
            assert_eq!(shown,
                       format!("a\\u{{{:x}}}b", u32::from(character)),
                       "{class}");
        }
    }

    /// Review of #522: text spelling out an escape is not mistaken for one.
    #[test]
    fn a_literal_backslash_is_doubled() {
        assert_eq!(visible("echo \\u{202e}"), "echo \\\\u{202e}");
        assert_ne!(visible("a\\u{d}b"), visible("a\rb"));
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        for text in ["make test",
                     "/Users/me/Code/project-feature",
                     "Builder 1",
                     "café ✓ 日本"]
        {
            assert_eq!(visible(text), text);
        }
    }
}
