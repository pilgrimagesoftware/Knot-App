//! Colours: the status palettes, pull request row tints, the workspace default
//! colour and the conversation find highlights.

use gpui_kit::Rgba;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------
//
// The status colours, one set per appearance - see `crate::status_palette`
// for what each means and the contrast each set is held to (#12). The hues
// are the Tailwind ramp the Swift reference used, so the two apps read
// alike: the 500 shades on dark, as before, and the 700 shades on light,
// where the 500s fell short of contrast.

/// The status colours on a dark surface.
pub(crate) const DARK_STATUS_PALETTE: crate::status_palette::StatusPalette =
    crate::status_palette::StatusPalette { idle:                0x22C55E,
                                           running:             0xF97316,
                                           // Blue and red 400, not the 500s
                                           // they were, and a lighter muted
                                           // grey: as text on a selected
                                           // row the old ones read 4.0-4.3:1.
                                           input:               0x60A5FA,
                                           error:               0xF87171,
                                           // Grey 400, not the 500 it was:
                                           // the stopped glyph is text, and
                                           // the 500 read 3.4:1 on dark.
                                           stopped:             0x9CA3AF,
                                           card_muted:          0x9A9A9A,
                                           pull_request_behind: 0xEAB308,
                                           pull_request_merged: 0xA855F7,
                                           // Red 500 and 600: saturated where
                                           // `error`'s red 400 is not (#369).
                                           session_limit:       0xEF4444,
                                           session_limit_fill:  0xDC2626, };

/// The status colours on a light surface: the same hues, darker.
pub(crate) const LIGHT_STATUS_PALETTE: crate::status_palette::StatusPalette =
    crate::status_palette::StatusPalette { idle:                0x15803D,
                                           running:             0xC2410C,
                                           input:               0x1D4ED8,
                                           error:               0xB91C1C,
                                           stopped:             0x4B5563,
                                           card_muted:          0x5F6368,
                                           pull_request_behind: 0xA16207,
                                           pull_request_merged: 0x7E22CE,
                                           session_limit:       0xDC2626,
                                           session_limit_fill:  0xDC2626, };

/// How much of a pull request row's state colour reaches its background.
///
/// Low enough that the row's text keeps the theme's contrast in both light
/// and dark - the colour is a tint identifying the state, not a fill
/// competing with the title on top of it.
pub(crate) const PULL_REQUEST_ROW_TINT: f32 = 0.10;

/// How much reaches its border, where there is no text to stay legible
/// against and the colour does the identifying.
pub(crate) const PULL_REQUEST_ROW_BORDER_TINT: f32 = 0.55;

/// The colour a workspace gets when it has none, or when the one it has
/// stored will not parse.
pub(crate) const COLOR_WORKSPACE_DEFAULT: u32 = 0x1B4FB2;

/// [`COLOR_WORKSPACE_DEFAULT`] in the `#rrggbb` form `Workspace::color_hex`
/// stores, so a new workspace and a workspace whose colour failed to parse
/// come out the same shade.
pub(crate) const COLOR_WORKSPACE_DEFAULT_HEX: &str = "#1B4FB2";

/// The workspace colour as a `Rgba`, for the parse-failure fallback.
pub(crate) fn workspace_default_color() -> Rgba {
    gpui_kit::rgb(COLOR_WORKSPACE_DEFAULT)
}

/// A find match's background, painted under the text. Translucent so it
/// reads on a light or a dark conversation alike.
pub(crate) const FIND_MATCH_COLOR: u32 = 0xFACC15;
pub(crate) const FIND_MATCH_ALPHA: f32 = 0.35;
/// The current find match's background, stronger than the rest.
pub(crate) const FIND_CURRENT_COLOR: u32 = 0xF97316;
pub(crate) const FIND_CURRENT_ALPHA: f32 = 0.6;
