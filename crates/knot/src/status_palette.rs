//! The fixed status colours, in a light and a dark set (#12).
//!
//! Each colour means one thing across the app - green is an idle agent and
//! an added line, red an error and a removed line - which the theme's
//! semantic tokens do not cover one-for-one, so they stay fixed rather than
//! becoming theme tokens. But a single set tuned against a dark surface fell
//! well short of contrast on a light one (the green measured about 2:1 as
//! text), so each colour now has a shade per appearance: the same hue,
//! darker on light. Which set applies follows the active theme.
//!
//! Colours drawn as text must reach WCAG AA's 4.5:1 against the surfaces
//! they sit on, and those drawn only as shapes - a dot, a row's border and
//! tint - 3:1. The tests hold every colour to its bar against the theme's
//! real surfaces in both appearances.

use gpui_kit::App;
use gpui_kit::component::ActiveTheme;

use crate::consts::{DARK_STATUS_PALETTE, LIGHT_STATUS_PALETTE};

/// One appearance's status colours, as `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusPalette {
    /// An idle agent, and a diff's added lines.
    pub(crate) idle:                u32,
    /// A working agent, and an open pull request that needs work to land.
    pub(crate) running:             u32,
    /// An agent awaiting input, a diff's changed-file count, and a pull
    /// request whose checks are still running.
    pub(crate) input:               u32,
    /// An agent in error, a diff's removed lines, and a closed pull request.
    pub(crate) error:               u32,
    /// A stopped agent.
    pub(crate) stopped:             u32,
    /// Muted text on a dashboard card.
    pub(crate) card_muted:          u32,
    /// A pull request whose branch is behind its base. Drawn only as a
    /// shape (a row's border and tint).
    pub(crate) pull_request_behind: u32,
    /// A merged pull request. Drawn only as a shape.
    pub(crate) pull_request_merged: u32,
}

/// The palette for the theme `cx` is drawing with.
pub(crate) fn status_palette(cx: &App) -> &'static StatusPalette {
    if cx.theme().is_dark() {
        &DARK_STATUS_PALETTE
    }
    else {
        &LIGHT_STATUS_PALETTE
    }
}

#[cfg(test)]
mod tests;
