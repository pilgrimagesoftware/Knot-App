//! The Markdown `TextView` both Markdown surfaces are built from: the ACP
//! panel's assistant messages and the pane `display-markdown` opens.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md` - rendered Markdown draws
//! body text in the UI font and headers in the title font.
//!
//! One constructor, [`markdown_view`], so the two surfaces cannot drift: the
//! spec requires them to render alike, and a shared constructor makes that
//! true by construction rather than by review.
//!
//! # How headers get their own face
//!
//! Through `gpui-base`'s per-level heading refinement
//! (`TextViewStyle::with_heading`), which its own heading branch applies over
//! its defaults. The heading stays `gpui-base`'s node, so it is selectable and
//! copyable like a paragraph, and keeps its inline marks.
//!
//! An earlier version claimed the heading node through the block parser and
//! renderer hooks and drew it as a plain `div` of text. That gave headers the
//! title face, but `gpui-base` leaves a claimed node out of every selection,
//! so header text could not be selected or copied (#491).
//!
//! # The upgrade checks this implies
//!
//! Passing a style replaces the one `gpui-component` installs for every text
//! view rather than refining it, and that one cannot be read back. So
//! [`MarkdownStyle::new`] rebuilds it from the theme - as of gpui-component
//! 0.7.0, its `text::base_text_view_style`. A gpui-kit upgrade that changes
//! that mapping leaves these disagreeing, silently; check it when bumping.
//!
//! [`HEADING_SIZE_FACTORS`] and [`HEADING_WEIGHTS`] are `gpui-base`'s own
//! values, re-stated because the size scales from the configured body size
//! rather than a fixed 14px. Check them against `BlockNode::render_block`
//! too.

use std::fmt;

use gpui_kit::assets::IconName;
use gpui_kit::base::text::{CodeBlock, TextView, TextViewState, TextViewStyle};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme, Sizable, Theme, WindowExt};
use gpui_kit::{
    App, ClickEvent, ClipboardItem, ElementId, Entity, FontWeight, HighlightStyle, Pixels,
    SharedString, StyleRefinement, Styled, Window, rems,
};

/// Size factor per heading level, indexed by `level - 1`, multiplied by the
/// body text size. `gpui-base`'s own values - see the module doc's upgrade
/// check.
const HEADING_SIZE_FACTORS: [f32; 6] = [2.0, 1.5, 1.25, 1.125, 1.0, 1.0];

/// Font weight per heading level, indexed by `level - 1`. `gpui-base`'s own
/// values - see the module doc's upgrade check.
const HEADING_WEIGHTS: [FontWeight; 6] = [FontWeight::BOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::MEDIUM];

/// What a level outside 1-6 renders at. `gpui-base` falls through to body size
/// at normal weight rather than clamping to level 6.
const HEADING_FALLBACK: (f32, FontWeight) = (1.0, FontWeight::NORMAL);

/// The pixel size and weight a heading level draws at, against `body_size` as
/// the rem base.
///
/// `body_size` rather than `gpui-base`'s fixed 14px: scaling from the
/// configured Markdown body size is what makes a user who enlarges that size
/// get proportionally larger headers.
pub(crate) fn heading_size_and_weight(level: u8, body_size: Pixels) -> (Pixels, FontWeight) {
    let (factor, weight) = match level {
        1..=6 => {
            let index = usize::from(level - 1);
            (HEADING_SIZE_FACTORS[index], HEADING_WEIGHTS[index])
        }
        _ => HEADING_FALLBACK,
    };
    (rems(factor).to_pixels(body_size), weight)
}

/// The refinement a heading at `level` draws with: the title family, and the
/// level's size and weight against `body_size`.
pub(crate) fn heading_refinement(level: u8, title_font_family: SharedString, body_size: Pixels)
                                 -> StyleRefinement {
    let (text_size, font_weight) = heading_size_and_weight(level, body_size);
    StyleRefinement::default().font_family(title_font_family)
                              .text_size(text_size)
                              .font_weight(font_weight)
}

/// How a Markdown surface draws: the theme's colors, and headers in the title
/// family at their level's size and weight.
///
/// Built per render from the live theme, so code blocks, tables and the
/// selection color track the system appearance the way `gpui-component`'s
/// installed style does. A `TextViewStyle::default()` would pin them to the
/// light palette.
#[derive(Clone)]
pub(crate) struct MarkdownStyle(TextViewStyle);

impl fmt::Debug for MarkdownStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarkdownStyle").finish_non_exhaustive()
    }
}

impl MarkdownStyle {
    /// The style for `theme`, with headers in `title_font_family` scaled from
    /// `body_size`. The theme mapping is `gpui-component`'s - see the module
    /// doc's upgrade check.
    pub(crate) fn new(theme: &Theme, title_font_family: SharedString, body_size: Pixels) -> Self {
        // All four corners, as gpui-component sets them one by one.
        let radius = theme.semantic_tokens().radius.md;
        let table = StyleRefinement::default().rounded(radius);
        let code_block = StyleRefinement::default().rounded(radius);
        let table_head = StyleRefinement::default().bg(theme.table_head)
                                                   .text_color(theme.table_head_foreground);

        Self(TextViewStyle::default().with_foreground(theme.foreground)
                                     .with_muted_foreground(theme.muted_foreground)
                                     .with_link(theme.link)
                                     .with_selection(theme.selection)
                                     .with_code_background(theme.muted)
                                     .with_border(theme.border)
                                     .with_code_block(code_block)
                                     .with_table(table)
                                     .with_table_head(table_head)
                                     .with_inline_code(HighlightStyle { background_color:
                                                                            Some(theme.accent),
                                                                        ..Default::default() })
                                     .with_dark(theme.is_dark())
                                     .with_heading(move |level| {
                                         heading_refinement(level,
                                                            title_font_family.clone(),
                                                            body_size)
                                     }))
    }

    /// The style for the live theme and the user's title font and Markdown body
    /// size - what both surfaces draw with.
    pub(crate) fn from_settings(cx: &App) -> Self {
        let settings = crate::settings_global::read(cx);
        Self::new(cx.theme(),
                  settings.title_font_name.clone().into(),
                  gpui_kit::px(settings.markdown_font_size as f32))
    }

    /// The `gpui-base` style this wraps.
    pub(crate) fn text_view_style(&self) -> &TextViewStyle {
        &self.0
    }
}

/// The text a code block's copy control places on the clipboard.
///
/// `CodeBlock::code()` rather than a re-scan of the Markdown source for
/// fences: the parser already decided where the block starts and ends -
/// including for an indented block, and for a fence whose content itself
/// contains backticks - and re-deriving that here would be a second,
/// disagreeing parser. It is also what makes the spec's "no fence markers, no
/// language tag" true by construction rather than by trimming.
pub(crate) fn code_block_copy_text(block: &CodeBlock) -> SharedString {
    block.code()
}

/// A code block's copy control: puts [`code_block_copy_text`] on the clipboard
/// and confirms with the same notification the panel's own copy actions use -
/// see `panel_view::message::render_response_actions`.
///
/// A plain `"copy"` element id is safe: `gpui-base` scopes the ids under a
/// code block by that block's own id, so one surface's blocks cannot collide.
fn code_block_copy_button(block: &CodeBlock) -> Button {
    let code = code_block_copy_text(block);
    Button::new("copy").icon(IconName::Copy)
                       .tooltip(knot_core::l10n::t("panel.copy_code"))
                       .ghost()
                       .small()
                       .on_click(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                           cx.write_to_clipboard(ClipboardItem::new_string(code.to_string()));
                           window.push_notification(
                            Notification::info(knot_core::l10n::t("panel.copied_code")),
                            cx,
                        );
                       })
}

/// The configured Markdown view: body text in `ui_font_family`, headers as
/// `style` gives them.
///
/// The body family is set on the `TextView` itself, which sets the ambient
/// family for everything it draws, so every body construct - paragraphs,
/// lists, table cells, quotes, the text around inline code - inherits it.
/// Headings do not: their refinement sets the title family, and a child's own
/// family wins over the ambient one. Inline code and code blocks keep the
/// monospace family.
///
/// `gpui-base`'s `TextView` rather than `gpui-component`'s wrapper: the
/// wrapper's style type has no heading face, and with no style and no motion
/// set it adds nothing to the view it wraps.
///
/// The code block copy control is wired here, once, for the same no-drift
/// reason as everything else in this constructor. `code_block_actions` is
/// `gpui-base`'s own hook, and `gpui-base` draws what it returns absolutely at
/// the block's top-right, inside the block's background.
pub(crate) fn markdown_view(id: impl Into<ElementId>, source: impl Into<SharedString>,
                            ui_font_family: SharedString, style: &MarkdownStyle)
                            -> TextView {
    configured(TextView::markdown(id, source), ui_font_family, style)
}

/// [`markdown_view`] over a state the caller owns, for a view whose parsed
/// document has to outlive the element - a conversation row the list stops
/// laying out when it scrolls away (#246). The caller keeps the state's text
/// current; see `panel_view::markdown_rows`.
pub(crate) fn markdown_view_for(state: &Entity<TextViewState>, ui_font_family: SharedString,
                                style: &MarkdownStyle)
                                -> TextView {
    configured(TextView::new(state), ui_font_family, style)
}

/// Everything both constructors share, so the two cannot drift apart.
fn configured(view: TextView, ui_font_family: SharedString, style: &MarkdownStyle) -> TextView {
    view.font_family(ui_font_family)
        .style(style.text_view_style().clone())
        .code_block_actions(|block: &CodeBlock, _window: &mut Window, _cx: &mut App| {
            code_block_copy_button(block)
        })
}
