//! A path with the characters a fuzzy match found marked, for the `@`
//! lookup's rows and the file finder's (`panel-file-mentions`: "the matched
//! characters SHALL be marked"; `file-finder`).

use std::ops::Range;

use gpui_kit::AnyElement;
use gpui_kit::FontWeight;
use gpui_kit::HighlightStyle;
use gpui_kit::Hsla;
use gpui_kit::IntoElement;
use gpui_kit::StyledText;

/// The byte ranges to mark in `text`: one per matched character, each at
/// its byte offset in the matched string moved along by `shift` (the
/// length of anything drawn in front of it, such as `@`).
pub(in crate::workspace_window) fn marked_ranges(text: &str, matched: &[usize], shift: usize)
                                                 -> Vec<Range<usize>> {
    matched.iter()
           .filter_map(|offset| {
               let start = offset + shift;
               let width = text.get(start..)?.chars().next()?.len_utf8();
               Some(start..start + width)
           })
           .collect()
}

/// `text` with the characters at `matched` (shifted by `shift`) drawn in
/// `color`, bold. Plain text when nothing matched.
pub(in crate::workspace_window) fn marked_text(text: String, matched: &[usize], shift: usize,
                                               color: Hsla)
                                               -> AnyElement {
    let ranges = marked_ranges(&text, matched, shift);
    if ranges.is_empty() {
        return text.into_any_element();
    }
    let style = HighlightStyle { color: Some(color),
                                 font_weight: Some(FontWeight::BOLD),
                                 ..Default::default() };
    StyledText::new(text).with_highlights(ranges.into_iter().map(|range| (range, style)))
                         .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_matched_character_is_one_range() {
        assert_eq!(marked_ranges("@src/lib.rs", &[4, 5], 1), vec![5..6, 6..7]);
    }

    /// A multi-byte character is marked whole.
    #[test]
    fn a_multi_byte_character_is_marked_whole() {
        assert_eq!(marked_ranges("é/lib", &[0], 0), vec![0..2]);
    }

    #[test]
    fn an_offset_past_the_end_is_skipped() {
        assert!(marked_ranges("ab", &[5], 0).is_empty());
    }
}
