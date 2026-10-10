//! The Import window's tabs: which source's section is showing.
//!
//! Library is first - ahead of the Claude subagent and Skwad sources - per
//! Paul's request on PR #230. Held per window and never persisted, so a
//! reopened window starts back on Library.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ImportTab {
    #[default]
    Library,
    Personas,
    Skwad,
}

impl ImportTab {
    pub(super) const ALL: [Self; 3] = [Self::Library, Self::Personas, Self::Skwad];

    pub(super) fn index(self) -> usize {
        Self::ALL.iter()
                 .position(|tab| *tab == self)
                 .expect("every tab is in ALL")
    }

    pub(super) fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::Library => "import.tab_library",
                               Self::Personas => "import.tab_personas",
                               Self::Skwad => "import.tab_skwad",
                           })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_is_first_and_the_default() {
        assert_eq!(ImportTab::ALL[0], ImportTab::Library);
        assert_eq!(ImportTab::default(), ImportTab::Library);
    }

    #[test]
    fn every_tab_label_resolves() {
        for tab in ImportTab::ALL {
            let label = tab.label();
            assert!(!label.starts_with("import.tab_"),
                    "{tab:?} is missing from the catalog: {label}");
        }
    }
}
