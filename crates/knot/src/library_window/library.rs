//! The three library kinds, each its own single-instance window: what it is
//! called, which [`WindowKey`](crate::window_registry::WindowKey) it opens
//! under, and how tall its window opens.

use gpui_kit::Pixels;
use gpui_kit::px;

use crate::window_registry::WindowKey;

/// A closed vocabulary rather than three near-identical window types: the
/// three only differ in title, target height and which pane they draw, all
/// of which this enum answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Library {
    Personas,
    Prompts,
    Bench,
}

impl Library {
    /// Every library, for the tests that prove each window has a title, a
    /// height and a catalog entry. Nothing else walks them: there is no tab
    /// strip to draw.
    #[cfg(test)]
    pub(crate) const ALL: [Library; 3] = [Library::Personas, Library::Prompts, Library::Bench];

    /// The window's title, shown in its OS titlebar.
    pub(crate) fn title(self) -> String {
        match self {
            Library::Personas => knot_core::l10n::t("library.personas.title"),
            Library::Prompts => knot_core::l10n::t("library.prompts.title"),
            Library::Bench => knot_core::l10n::t("library.bench.title"),
        }
    }

    /// The registry key a single instance of this window opens under.
    pub(crate) fn window_key(self) -> WindowKey {
        match self {
            Library::Personas => WindowKey::Personas,
            Library::Prompts => WindowKey::Prompts,
            Library::Bench => WindowKey::Bench,
        }
    }

    /// The window's opening height - unchanged from the cap the settings
    /// window gave this pane before the move, so moving it did not resize it.
    pub(crate) fn height(self) -> Pixels {
        match self {
            Library::Personas => px(600.),
            Library::Prompts => px(600.),
            Library::Bench => px(560.),
        }
    }
}
