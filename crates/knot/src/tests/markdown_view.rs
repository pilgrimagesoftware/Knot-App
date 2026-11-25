//! Unit tests for [`crate::markdown_view`].
//!
//! The heading table is pinned level by level because the size scales from
//! the configured body size rather than `gpui-base`'s fixed 14px: nothing but
//! these assertions would notice the two drifting apart after a gpui-kit bump.
//!
//! What a heading looks like on screen, and that it can be selected, needs a
//! window; `tests/markdown_selection.rs` covers that. What is tested here is
//! the pure part - the level-to-size-and-weight mapping and the refinement
//! built from it.
//!
//! The same boundary decides what the code block copy control leaves
//! untested. Only [`code_block_copy_text`] is a pure function; the button
//! around it needs a `Window` and an `App`, and where `gpui-base` draws it is
//! `gpui-base`'s own placement. So its corner, its tooltip and its click
//! handler are covered by the spec's scenarios and by the manual check in the
//! change's task list, not by an assertion here.

use gpui_kit::base::text::CodeBlock;
use gpui_kit::{FontWeight, Pixels, px};

use crate::markdown_view::{code_block_copy_text, heading_refinement, heading_size_and_weight};

/// The body size the factors multiply. Deliberately not 14, the default of
/// `TextViewStyle::heading_base_font_size`, so these assertions would fail if
/// the base silently reverted to that.
const BODY: f32 = 16.;

fn size_of(level: u8) -> Pixels {
    heading_size_and_weight(level, px(BODY)).0
}

fn weight_of(level: u8) -> FontWeight {
    heading_size_and_weight(level, px(BODY)).1
}

#[test]
fn every_heading_level_keeps_its_own_size() {
    assert_eq!(size_of(1), px(32.));
    assert_eq!(size_of(2), px(24.));
    assert_eq!(size_of(3), px(20.));
    assert_eq!(size_of(4), px(18.));
    assert_eq!(size_of(5), px(16.));
    assert_eq!(size_of(6), px(16.));
}

#[test]
fn every_heading_level_keeps_its_own_weight() {
    assert_eq!(weight_of(1), FontWeight::BOLD);
    assert_eq!(weight_of(2), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(3), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(4), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(5), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(6), FontWeight::MEDIUM);
}

/// A level outside 1-6 cannot come from Markdown, but it can come from a node
/// carrying no level payload. Body size at normal weight, as `gpui-base` falls
/// through to - not a panic on the index, and not a clamp to level 6's medium
/// weight.
#[test]
fn an_out_of_range_level_falls_back_to_body_size_at_normal_weight() {
    for level in [0, 7, u8::MAX] {
        assert_eq!(size_of(level), px(BODY), "level {level} should not scale");
        assert_eq!(weight_of(level), FontWeight::NORMAL, "level {level}");
    }
}

#[test]
fn the_heading_size_scales_with_the_body_size() {
    assert_eq!(heading_size_and_weight(1, px(10.)).0, px(20.));
    assert_eq!(heading_size_and_weight(1, px(20.)).0, px(40.));
}

/// The refinement carries the face, size and weight the heading branch then
/// applies - the face is the reason the refinement exists at all.
#[test]
fn a_heading_refinement_carries_the_title_face_and_its_levels_size() {
    let refinement = heading_refinement(2, "Manrope".into(), px(BODY));
    assert_eq!(refinement.text.font_family, Some("Manrope".into()));
    assert_eq!(refinement.text.font_size, Some(size_of(2).into()));
    assert_eq!(refinement.text.font_weight, Some(weight_of(2)));
}

/// The language tag is metadata the parser keeps beside the code, never part
/// of it, so the spec's "no fence markers, no language tag" needs no trimming
/// - only that this reads the code and not the source.
#[test]
fn a_tagged_block_copies_its_lines_without_the_tag() {
    let block = CodeBlock::from_code("cargo test\ncargo build\n", Some("bash"));
    assert_eq!(code_block_copy_text(&block), "cargo test\ncargo build\n");
}

/// Backticks inside the code are content, not delimiters. Re-scanning the
/// Markdown source for a closing fence is where this would go wrong; reading
/// the parser's own text is where it cannot.
#[test]
fn backticks_inside_the_code_are_copied_verbatim() {
    let block = CodeBlock::from_code("echo `date` and ``nested``", Some("sh"));
    assert_eq!(code_block_copy_text(&block), "echo `date` and ``nested``");
}

/// An empty block still draws a button, per design.md - suppressing it would
/// make the block chrome inconsistent - so the empty copy has to be the honest
/// answer rather than a panic or a placeholder.
#[test]
fn an_empty_block_copies_an_empty_string() {
    let block = CodeBlock::from_code("", None::<&str>);
    assert_eq!(code_block_copy_text(&block), "");
}
