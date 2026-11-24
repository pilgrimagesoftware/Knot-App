//! Window and dialog sizes, restored-window visibility rules, the link
//! tooltip's placement and the Keyboard tab's list height.

/// How wide the workspace name dialog is, in pixels.
///
/// Narrower than the dialog host's 448px default: the dialog holds one
/// single-line name field, and a box twice as wide as its contents reads as
/// an empty one.
pub(crate) const WORKSPACE_DIALOG_WIDTH: f32 = 360.;

/// The report window's width, also its minimum: the diagnostics block's
/// longest line - the app name, version, build date and commit - should fit
/// without wrapping, so it reads as the one line it is when pasted.
pub(crate) const BUG_REPORT_WINDOW_WIDTH: f32 = 560.;

/// The report window's opening height: the whole form, logs and screenshots
/// included, without scrolling.
pub(crate) const BUG_REPORT_WINDOW_HEIGHT: f32 = 760.;

/// How short the report window may be made; below its opening height the
/// form scrolls above the fixed Cancel / Report row.
pub(crate) const BUG_REPORT_WINDOW_MIN_HEIGHT: f32 = 420.;

/// How much of a restored window's top edge has to land on a display for the
/// user to be able to grab it: the workspace title bar's height, so the whole
/// bar is reachable rather than a sliver of it.
///
/// A window restored onto a display that is no longer attached was placed
/// unclamped, which is how one ended up where it could not be dragged back.
pub(crate) const WINDOW_GRAB_STRIP_HEIGHT: f32 = 64.;

/// How much of a restored window's width has to overlap a display, so it
/// cannot be pushed to a few pixels at the screen edge and counted as visible.
pub(crate) const WINDOW_MIN_VISIBLE_WIDTH: f32 = 160.;

/// The tallest the Keyboard tab's shortcut list grows before it scrolls.
///
/// A cap rather than a fill: the list sits inside a `GroupBox`, whose content
/// boxes never stretch, so a region filling its parent there has no height to
/// fill and draws nothing (#222). Sized so the blurb, the group and Restore
/// Defaults all fit the tab's 560 px window (`pane_target_height`).
pub(crate) const KEYBOARD_SHORTCUTS_LIST_MAX_HEIGHT: f32 = 280.;

/// The prompt editor's window, opened at a size that shows a prompt of a
/// few paragraphs without scrolling (#322). It used to share the persona
/// editor's 460 x 380, sized for a name and a short description; a prompt
/// is the opposite - one line of name over a long text.
pub(crate) const PROMPT_EDITOR_WINDOW_WIDTH: f32 = 720.;
pub(crate) const PROMPT_EDITOR_WINDOW_HEIGHT: f32 = 560.;

/// The smallest the prompt editor resizes to: the name row, several lines
/// of text, the variables control and the button row all still fit.
pub(crate) const PROMPT_EDITOR_WINDOW_MIN_WIDTH: f32 = 520.;
pub(crate) const PROMPT_EDITOR_WINDOW_MIN_HEIGHT: f32 = 400.;

/// How far below and right of the pointer a link's URL label sits (#326),
/// clear of the cursor.
pub(crate) const LINK_TOOLTIP_OFFSET: f32 = 14.;
/// The widest a link's URL label grows before the URL is cut short with an
/// ellipsis.
pub(crate) const LINK_TOOLTIP_MAX_WIDTH: f32 = 480.;
/// The least room a link's URL label keeps from the window's edges.
pub(crate) const LINK_TOOLTIP_WINDOW_MARGIN: f32 = 8.;
