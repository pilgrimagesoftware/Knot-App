//! Every formatting case of the Changes row's counts (#210), asserted
//! through the catalog rather than against English copy, per
//! `.claude/rules/rust-structure.md`.

use knot_core::l10n;

use super::{ChangesCounts, Count};

fn counts(pull_requests: u64, issues: u64, openspec_changes: u64) -> ChangesCounts {
    ChangesCounts { pull_requests:    Count::exact(pull_requests),
                    issues:           Count::exact(issues),
                    openspec_changes: Count::exact(openspec_changes), }
}

/// `count` and `noun`, as the catalog puts them together.
fn item(count: &str, noun_key: &str) -> String {
    l10n::t_with("changes_view.count.item",
                 &[("count", count), ("noun", &l10n::t(noun_key))])
}

fn joined(parts: &[String]) -> String {
    parts.join(&l10n::t("changes_view.count.separator"))
}

#[test]
fn nothing_counted_shows_the_title_alone() {
    assert_eq!(counts(0, 0, 0).summary(), None);
}

#[test]
fn one_kind_is_named_in_full() {
    assert_eq!(counts(3, 0, 0).summary(),
               Some(item("3", "changes_view.count.pull_requests")));
    assert_eq!(counts(0, 2, 0).summary(),
               Some(item("2", "changes_view.count.issues")));
    assert_eq!(counts(0, 0, 4).summary(),
               Some(item("4", "changes_view.count.openspec_changes")));
}

#[test]
fn two_kinds_are_named_in_full_in_order() {
    assert_eq!(counts(3, 2, 0).summary(),
               Some(joined(&[item("3", "changes_view.count.pull_requests"),
                             item("2", "changes_view.count.issues")])));
    assert_eq!(counts(1, 0, 3).summary(),
               Some(joined(&[item("1", "changes_view.count.pull_request"),
                             item("3", "changes_view.count.openspec_changes")])));
    assert_eq!(counts(0, 1, 1).summary(),
               Some(joined(&[item("1", "changes_view.count.issue"),
                             item("1", "changes_view.count.openspec_change")])));
}

#[test]
fn all_three_kinds_use_the_short_nouns() {
    assert_eq!(counts(3, 2, 4).summary(),
               Some(joined(&[item("3", "changes_view.count.prs_short"),
                             item("2", "changes_view.count.issues_short"),
                             item("4", "changes_view.count.changes_short")])));
}

#[test]
fn one_of_each_is_singular_in_both_forms() {
    assert_eq!(counts(1, 0, 0).summary(),
               Some(item("1", "changes_view.count.pull_request")));
    assert_eq!(counts(1, 1, 1).summary(),
               Some(joined(&[item("1", "changes_view.count.pr_short"),
                             item("1", "changes_view.count.issue_short"),
                             item("1", "changes_view.count.change_short")])));
}

/// An issue list stops at a fixed number per repository; a capped count
/// says so, and takes the plural noun even at one.
#[test]
fn a_capped_issue_count_is_marked_and_plural() {
    let capped = ChangesCounts { issues: Count { count:    100,
                                                 at_least: true, },
                                 ..counts(0, 0, 0) };
    let at_least = l10n::t_with("changes_view.count.at_least", &[("count", "100")]);

    assert_eq!(capped.summary(),
               Some(item(&at_least, "changes_view.count.issues")));
    assert!(at_least.contains("100"), "{at_least}");
}

#[test]
fn every_count_key_resolves_and_keeps_its_values() {
    for key in ["changes_view.count.pull_request",
                "changes_view.count.pull_requests",
                "changes_view.count.issue",
                "changes_view.count.issues",
                "changes_view.count.openspec_change",
                "changes_view.count.openspec_changes",
                "changes_view.count.pr_short",
                "changes_view.count.prs_short",
                "changes_view.count.issue_short",
                "changes_view.count.issues_short",
                "changes_view.count.change_short",
                "changes_view.count.changes_short",
                "changes_view.count.separator"]
    {
        assert_ne!(l10n::t(key), key, "{key} does not resolve");
    }
    let item = l10n::t_with("changes_view.count.item",
                            &[("count", "7"), ("noun", "things")]);
    assert!(item.contains('7') && item.contains("things") && !item.contains("%{"),
            "{item}");
    let states = l10n::t_with("changes_view.pull_request_states", &[("states", "2 open")]);
    assert!(states.contains("2 open") && !states.contains("%{"),
            "{states}");
}

/// The window reads the counts from the caches the Changes view fills: every
/// issue page's issues, a capped page marking the total as a floor, and
/// every repository's changes.
#[gpui_kit::test]
fn the_window_counts_what_the_view_has_fetched(cx: &mut gpui_kit::TestAppContext) {
    use std::path::PathBuf;
    use std::time::Duration;

    use knot_forge::{Issue, IssuePage, RepoSlug};

    use crate::openspec_changes::ChangeEntry;
    use crate::workspace_window::shortcuts_tests::window_with_agents;
    use crate::workspace_window::work_items::IssueLookup;

    fn issue(number: u64) -> Issue {
        Issue { number,
                title: format!("issue {number}"),
                url: format!("https://github.com/acme/widget/issues/{number}"),
                labels: Vec::new(),
                author: None,
                created_at: time::OffsetDateTime::UNIX_EPOCH,
                updated_at: time::OffsetDateTime::UNIX_EPOCH }
    }
    fn change(name: &str) -> ChangeEntry {
        ChangeEntry { name:      name.to_owned(),
                      dir:       PathBuf::from(name),
                      subfolder: None,
                      why:       None, }
    }
    fn slug(repo: &str) -> RepoSlug {
        RepoSlug { owner: "acme".to_owned(),
                   repo:  repo.to_owned(), }
    }

    let mut fixture = window_with_agents(1, cx);
    fixture.view.update(&mut fixture.window, |view, _| {
                    assert_eq!(view.changes_counts(),
                               counts(0, 0, 0),
                               "nothing fetched yet");

                    let caches = &mut view.work_items;
                    let age = Duration::from_secs(60);
                    caches.issues
                          .claim_refresh(slug("widget"), age)
                          .expect("a claim")
                          .record(IssueLookup::Page(IssuePage { issues:    vec![issue(1), issue(2)],
                                                                truncated: false, }));
                    caches.issues
                          .claim_refresh(slug("gadget"), age)
                          .expect("a claim")
                          .record(IssueLookup::Page(IssuePage { issues:    vec![issue(3)],
                                                                truncated: true, }));
                    caches.issues
                          .claim_refresh(slug("broken"), age)
                          .expect("a claim")
                          .record(IssueLookup::Failed("gh failed".to_owned()));
                    caches.changes
                          .claim_refresh(PathBuf::from("/repo/a"), age)
                          .expect("a claim")
                          .record(vec![change("add-login"), change("add-export")]);

                    let counted = view.changes_counts();
                    assert_eq!(counted.issues,
                               Count { count:    3,
                                       at_least: true, },
                               "a failed repository adds nothing; a capped one marks a floor");
                    assert_eq!(counted.openspec_changes, Count::exact(2));
                    assert_eq!(counted.pull_requests, Count::exact(0));
                });
}
