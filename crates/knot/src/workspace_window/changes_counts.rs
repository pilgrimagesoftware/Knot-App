//! What the sidebar's Changes row counts (#210): the workspace's pull
//! requests, open issues and un-archived OpenSpec changes, in one line.
//!
//! Contract: "The Changes row counts what the view holds" in
//! `openspec/specs/pull-request-tracking/spec.md`.
//!
//! The rule is about width as much as wording. With nothing counted the row
//! is its title alone. One or two kinds are named in full ("3 pull
//! requests, 2 issues"); all three switch to short nouns ("3 PRs, 2 issues,
//! 4 changes") so the line still fits a sidebar.
//!
//! Issues and OpenSpec changes are counted from what the Changes view has
//! fetched, which happens only while its tab is shown. A kind Knot has not
//! looked at yet is absent rather than counted as zero, the same honesty
//! `counts_label` keeps for pull request states it has not asked about.

use knot_core::l10n;

use super::WorkspaceWindow;
use super::work_items::IssueLookup;
use crate::consts::{
    CHANGES_COUNT_ISSUES, CHANGES_COUNT_OPENSPEC_CHANGES, CHANGES_COUNT_PULL_REQUESTS,
};

/// One kind's count, and whether it is a floor rather than the total - an
/// issue list stops at a fixed number per repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Count {
    pub(crate) count:    u64,
    pub(crate) at_least: bool,
}

impl Count {
    pub(crate) fn exact(count: u64) -> Self {
        Self { count,
               at_least: false }
    }
}

/// The three kinds the row counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChangesCounts {
    pub(crate) pull_requests:    Count,
    pub(crate) issues:           Count,
    pub(crate) openspec_changes: Count,
}

impl ChangesCounts {
    /// The row's second line, or `None` when there is nothing to count and
    /// the row shows its title alone.
    pub(crate) fn summary(&self) -> Option<String> {
        let counted =
            [(self.pull_requests, &CHANGES_COUNT_PULL_REQUESTS),
             (self.issues, &CHANGES_COUNT_ISSUES),
             (self.openspec_changes, &CHANGES_COUNT_OPENSPEC_CHANGES)].into_iter()
                                                                      .filter(|(count, _)| {
                                                                          count.count > 0
                                                                      })
                                                                      .collect::<Vec<_>>();
        if counted.is_empty() {
            return None;
        }
        // Short nouns only once every kind is in the line.
        let short = counted.len() == 3;
        let parts = counted.into_iter()
                           .map(|(count, nouns)| {
                               let (singular, plural) =
                                   if short { nouns.short } else { nouns.long };
                               counted_text(count, singular, plural)
                           })
                           .collect::<Vec<_>>();
        Some(parts.join(&l10n::t("changes_view.count.separator")))
    }
}

/// "3 pull requests", or "100+ issues" for a capped count. The noun is
/// plural for a capped count whatever the number, since there are more.
fn counted_text(count: Count, singular: &str, plural: &str) -> String {
    let number = count.count.to_string();
    let (number, noun) = if count.at_least {
        (l10n::t_with("changes_view.count.at_least", &[("count", &number)]), l10n::t(plural))
    }
    else {
        (number, l10n::plural_noun(count.count, singular, plural))
    };
    l10n::t_with("changes_view.count.item",
                 &[("count", &number), ("noun", &noun)])
}

impl WorkspaceWindow {
    /// The counts the Changes row shows, from the pull request records and
    /// whatever the Issues and OpenSpec tabs have fetched.
    ///
    /// Read on every sidebar render, so the caches are folded under their
    /// locks rather than snapshotted: a snapshot would clone every cached
    /// issue page per keystroke to read one length from each.
    pub(super) fn changes_counts(&self) -> ChangesCounts {
        let issues = self.work_items
                         .issues
                         .fold(Count::default(), |mut issues, lookup| {
                             if let IssueLookup::Page(page) = lookup {
                                 issues.count += page.issues.len() as u64;
                                 issues.at_least |= page.truncated;
                             }
                             issues
                         });
        let changes = self.work_items
                          .changes
                          .fold(0, |total, changes| total + changes.len() as u64);
        ChangesCounts { pull_requests: Count::exact(self.pull_request_counts().total() as u64),
                        issues,
                        openspec_changes: Count::exact(changes) }
    }
}

#[cfg(test)]
mod tests;
