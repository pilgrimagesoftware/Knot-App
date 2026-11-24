//! Catalog coverage for the processes section's subagent copy and the Changes
//! view's tabs and notices.

/// Every string the subagents half of the processes section introduces.
///
/// Asserted by key, never by copy: a copy edit must not fail a test. What
/// this catches is a key that is never added to the catalogue, which renders
/// as the key itself - readable enough to pass a glance in review and wrong
/// on screen.
#[test]
fn subagent_section_labels_resolve() {
    for key in ["processes.group_subagents",
                "processes.group_processes",
                "processes.subagent_running",
                "processes.subagent_finished",
                "processes.subagent_failed",
                "processes.subagent_kind_unstated",
                "processes.subagent_count_one",
                "processes.subagent_count_many",
                "processes.copy_task",
                "processes.copied_task",
                "processes.empty_no_subagents"]
    {
        let text = knot_core::l10n::t(key);

        assert_ne!(text, key, "{key} is missing from the catalogue");
        assert!(!text.is_empty(), "{key} resolved to nothing");
    }
}

/// The one key here that substitutes a value. A body that lost its
/// placeholder still resolves and still reads as a sentence, so resolution
/// alone would not catch it - the value has to survive the substitution.
#[test]
fn a_subagent_failure_reason_survives_substitution() {
    let text = knot_core::l10n::t_with("processes.subagent_failed_reason",
                                       &[("reason", "ran out of context")]);

    assert!(text.contains("ran out of context"), "{text}");
    assert!(!text.contains("%{reason}"),
            "the placeholder was left unsubstituted: {text}");
}

/// The two groups must not read as the same thing, and neither may collide
/// with the section's own title.
#[test]
fn the_two_group_labels_are_distinguishable() {
    let subagents = knot_core::l10n::t("processes.group_subagents");
    let processes = knot_core::l10n::t("processes.group_processes");

    assert_ne!(subagents, processes);
    assert_ne!(subagents, knot_core::l10n::t("processes.title"));
}

/// Every label and message of the Changes view's tabs (#504) - a missing key
/// would ship the key string itself as a tab, a menu item or a notice.
#[test]
fn changes_view_labels_resolve() {
    for key in ["changes_view.clear_filters",
                "changes_view.list_actions",
                "changes_view.no_pull_requests",
                "changes_view.prompt.change",
                "changes_view.prompt.issue",
                "changes_view.refresh_now",
                "changes_view.repo_all",
                "changes_view.row_actions",
                "changes_view.send_prompt_to",
                "changes_view.tab.issues",
                "changes_view.tab.openspec",
                "changes_view.tab.pull_requests",
                "changes_view.title",
                "issues.checking",
                "issues.copy_url",
                "issues.copy_urls",
                "issues.empty",
                "issues.fetch_failed",
                "issues.forge_failed",
                "issues.forge_missing",
                "issues.forge_unauthenticated",
                "issues.no_github_repos",
                "issues.no_match",
                "issues.open_in_browser",
                "issues.search_placeholder",
                "issues.sort.newest",
                "issues.sort.number",
                "issues.sort.oldest",
                "issues.sort.recently_updated",
                "issues.truncated_one",
                "issues.truncated_other",
                "issues.updated",
                "openspec_changes.checking",
                "openspec_changes.copy_name",
                "openspec_changes.empty",
                "openspec_changes.no_match",
                "openspec_changes.reveal",
                "openspec_changes.search_placeholder"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}
