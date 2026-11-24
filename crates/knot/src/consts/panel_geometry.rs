//! Side-panel, artifact-section and diff-list geometry: default and clamp
//! widths, the artifact split, section heights, diff limits and the markdown
//! font-size range.

/// The git panel's width when it opens, and the bounds a drag may take it to.
///
/// Carried from the Swift panel, which clamped the same way. Not persisted:
/// a reopened panel starts at the default again.
pub(crate) const GIT_PANEL_DEFAULT_WIDTH: f32 = 500.;
pub(crate) const GIT_PANEL_MIN_WIDTH: f32 = 350.;
pub(crate) const GIT_PANEL_MAX_WIDTH: f32 = 800.;

/// The artifact panel's width when it opens, and the bounds a drag may take
/// it to.
///
/// The same three values the git panel uses, and deliberately so: the two are
/// siblings in the same resizable group, and a user who has learned how wide
/// one goes has learned the other. Carried from `ArtifactPanelView`'s own
/// `max(350, min(800, ...))`. Not persisted, for the reason the git panel's
/// are not.
pub(crate) const ARTIFACT_PANEL_DEFAULT_WIDTH: f32 = 500.;
pub(crate) const ARTIFACT_PANEL_MIN_WIDTH: f32 = 350.;
pub(crate) const ARTIFACT_PANEL_MAX_WIDTH: f32 = 800.;

/// How the artifact panel's height divides between its two sections, and how
/// far a drag on the divider may take it.
///
/// The clamp keeps either section from being dragged away entirely: at the
/// limit the smaller one still shows content rather than collapsing to its
/// header, which is what the collapse chevron is for and is a different
/// gesture. Carried from `ArtifactPanelView.sectionDivider`.
pub(crate) const ARTIFACT_PANEL_DEFAULT_SPLIT: f32 = 0.5;
pub(crate) const ARTIFACT_PANEL_MIN_SPLIT: f32 = 0.15;
pub(crate) const ARTIFACT_PANEL_MAX_SPLIT: f32 = 0.85;

/// The bounds the markdown section's font size controls step within, from
/// `MarkdownPanelView`'s `A` buttons.
pub(crate) const MARKDOWN_FONT_SIZE_MIN: i32 = 10;
pub(crate) const MARKDOWN_FONT_SIZE_MAX: i32 = 24;

/// A collapsed artifact section's height: its header and nothing else.
pub(crate) const ARTIFACT_SECTION_HEADER_HEIGHT: f32 = 34.;

/// The draggable divider between the two artifact sections.
///
/// Counted out of the height the sections share, so the two plus this equal
/// the section area exactly at every split.
pub(crate) const ARTIFACT_PANEL_DIVIDER_HEIGHT: f32 = 4.;

/// The least width the content pane keeps while side panels are open.
///
/// Both side panels have a 350pt minimum of their own, which on a narrow
/// window would otherwise leave the conversation nothing. This does not apply
/// while the artifact panel is expanded: that state takes the content pane's
/// width deliberately, and is the one case both focus requirements account
/// for.
pub(crate) const CONTENT_PANE_MIN_WIDTH: f32 = 360.;

/// The most diff lines the panel will hold and draw for one file.
///
/// Drawing is virtualized, so this is not what bounds the frame - it bounds
/// what `parse_diff` materializes into the cache. A generated file of several
/// hundred thousand lines should not be held in memory per selected row.
pub(crate) const GIT_DIFF_MAX_LINES: usize = 20_000;

/// How much space above and below the diff viewport the list measures, so
/// scrolling a diff does not pop lines in at the edges. The conversation
/// panel's own overdraw, for the same reason.
pub(crate) const GIT_DIFF_LIST_OVERDRAW: f32 = 400.;

/// The height hint every diff row starts with, before it has been measured.
///
/// A virtualized list summarises an unmeasured row as zero height and flags
/// the whole summary unknown, so anything derived from total content height -
/// a scrollbar thumb above all - is wrong until every row has been drawn.
/// Seeding a hint fixes that from the first frame, and real heights replace
/// it as rows render.
///
/// The value is the computed line box, not an estimate: `text_xs` is
/// `rems(0.75)` against the default 16px rem, so 12px; the default
/// `line_height` is `phi` *relative to the font size*, so 12 x 1.618034 =
/// 19.416, rounded to 19. A diff row carries horizontal padding only, and no
/// border, so the line box is the whole row.
///
/// One hint works here and would not for a conversation: diff rows are
/// uniform - one line each, one text size, no wrapping.
pub(crate) const GIT_DIFF_LINE_HEIGHT: f32 = 19.;
