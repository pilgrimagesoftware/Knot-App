//! Unit tests for [`super`]: the text worked out from Markdown is the text
//! `gpui-base` renders, sample by sample.

use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use gpui_kit::component::text::TextViewState;

use super::PlainTexts;
use super::plain_text;

/// Constructs a reply is likely to hold, each on its own and together.
const SAMPLES: &[&str] = &["plain words",
                           "hello **world** and _more_ ~~gone~~",
                           "see [docs](https://example.com/docs) and docs",
                           "# A heading\n\nA paragraph after it.",
                           "- one\n- two\n  - nested\n\n1. first\n2. second",
                           "> quoted *text*\n> on two lines",
                           "run `make test` now",
                           "```rust\nfn main() {}\n```\n\nafter the code",
                           "| a | b |\n|---|---|\n| one | two |\n| three | four |",
                           "costs $5 and $10 today",
                           "line one  \nline two",
                           "a soft\nbreak in a paragraph",
                           "> - a list\n> - inside a quote",
                           "1. item with `code` and [a link](u)\n\n   a second paragraph\n2. next",
                           "**bold [link](https://x/y) inside** text",
                           "# Title\n\ntext\n\n---\n\nafter a rule",
                           "an ![image](x.png) between words",
                           "escaped \\*stars\\* stay"];

/// The text a real view renders `source` as.
fn rendered(source: &str, cx: &mut TestAppContext) -> String {
    cx.update(|cx| {
          gpui_kit::init(cx);
          let state = cx.new(|cx| TextViewState::markdown(source, cx));
          state.read(cx).rendered_text().as_str().to_owned()
      })
}

#[gpui_kit::test]
fn the_worked_out_text_is_the_rendered_text(cx: &mut TestAppContext) {
    let wrong: Vec<String> = SAMPLES.iter()
                                    .filter_map(|source| {
                                        let (ours, theirs) = (plain_text(source), rendered(source, cx));
                                        (ours != theirs).then(|| {
                                                            format!("{source:?}\n  worked out: {ours:?}\n  rendered:   {theirs:?}")
                                                        })
                                    })
                                    .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The case that made counting in the source wrong: a match inside a link's
/// address is not in the text, so it is not counted.
#[test]
fn a_links_address_is_not_text() {
    let text = plain_text("see [docs](https://example.com/docs) and docs");
    assert_eq!(text.matches("docs").count(), 2);
}

#[test]
fn a_message_is_worked_out_again_only_when_it_changes() {
    let mut texts = PlainTexts::default();
    assert_eq!(texts.get(0, "**a**"), "a\n");
    assert_eq!(texts.get(0, "**a** b"), "a b\n");
    assert_eq!(texts.get(1, "c"), "c\n");

    texts.truncate(1);
    assert_eq!(texts.0.len(), 1, "messages past the end are forgotten");
}
