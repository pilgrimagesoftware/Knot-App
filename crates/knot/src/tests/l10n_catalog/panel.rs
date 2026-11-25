//! Catalog coverage for the agent panel's copy: slash lookup, shell
//! passthrough, transcript errors, chrome, the artifact panel's actions and
//! harness notices.

/// The slash lookup's own copy: the popup's key hint, and the description
/// beside every built-in command. A missing key ships the key itself as the
/// line explaining what a command does.
#[test]
fn slash_lookup_labels_resolve() {
    for key in ["panel.lookup_hint",
                "panel.command.broadcast",
                "panel.command.check",
                "panel.command.create_agent",
                "panel.command.list_agents",
                "panel.command.list_repos",
                "panel.command.list_worktrees",
                "panel.command.send",
                "panel.command.show_markdown",
                "panel.command.worktree"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The `!` shell passthrough's chrome. Every one of these appears on an entry
/// the user made happen deliberately, so a key string in place of copy is
/// exactly where it would be noticed and exactly where it should not be.
#[test]
fn shell_passthrough_labels_resolve() {
    for key in ["panel.shell.marker",
                "panel.shell.running",
                "panel.shell.cancel",
                "panel.shell.cancelled",
                "panel.shell.truncated",
                "panel.shell.exit_ok",
                "panel.shell.signalled",
                "panel.shell.pending",
                "panel.shell.discard",
                "panel.shell.shared"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The three that embed a value. A body that lost its placeholder still
/// resolves, so resolving is not enough: assert the value survives
/// substitution.
#[test]
fn shell_passthrough_substitutions_survive() {
    let folder = knot_core::l10n::t_with("panel.shell.ran_in", &[("folder", "/tmp/worktree")]);
    assert!(folder.contains("/tmp/worktree"), "{folder}");

    let code = knot_core::l10n::t_with("panel.shell.exit_failed", &[("code", "3")]);
    assert!(code.contains('3'), "{code}");

    let error =
        knot_core::l10n::t_with("panel.shell.failed_to_start", &[("error", "no such file")]);
    assert!(error.contains("no such file"), "{error}");

    let limit = knot_core::l10n::t_with("panel.shell.timed_out", &[("limit", "2 minutes")]);
    assert!(limit.contains("2 minutes"), "{limit}");
}

/// The three failures a panel writes into the conversation itself. Each
/// embeds the underlying error, so a body that lost its placeholder would
/// report a failure without saying what failed.
#[test]
fn panel_transcript_errors_resolve_and_keep_their_cause() {
    for key in ["panel.error_first_turn",
                "panel.error_registration",
                "panel.error_answer"]
    {
        let message = knot_core::l10n::t_with(key, &[("error", "connection refused")]);
        assert_ne!(message, key, "{key} is missing from the catalog");
        assert!(message.contains("connection refused"),
                "{key} must carry the cause: {message}");
        assert!(!message.contains("%{error}"),
                "{key} left its placeholder unfilled");
    }
}

/// The chrome a panel draws around a conversation - two decision buttons,
/// four icon-only controls whose tooltip is their only name, and the two
/// lines a panel shows when it has no conversation yet.
#[test]
fn panel_chrome_labels_resolve() {
    for key in ["panel.allow",
                "panel.deny",
                "panel.close",
                "panel.stop",
                "panel.running",
                "panel.connecting",
                "panel.select_agent_to_start",
                "panel.attach_context",
                "panel.scroll_to_latest",
                "panel.scroll_to_your_message",
                "panel.scroll_to_top"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The artifact panel's section actions and its reopen control (#535).
#[test]
fn artifact_panel_actions_resolve() {
    for key in ["artifact_panel.reopen",
                "menu.view.artifacts",
                "keymap.shortcut.toggle_artifacts",
                "artifact_panel.loading",
                "artifact_panel.approve",
                "artifact_panel.approve_tooltip",
                "artifact_panel.review",
                "artifact_panel.review_tooltip",
                "artifact_panel.submit_review",
                "artifact_panel.submit_review_tooltip",
                "artifact_panel.font_smaller",
                "artifact_panel.font_larger",
                "artifact_panel.copy_markdown",
                "artifact_panel.reveal_in_finder",
                "artifact_panel.copy_diagram"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
    let note = knot_core::l10n::t_with("artifact_panel.read_failed",
                                       &[("path", "/tmp/plan.md"), ("error", "not found")]);
    assert!(note.contains("/tmp/plan.md") && note.contains("not found"),
            "{note}");
}

/// What a replayed conversation shows in place of a harness-injected message
/// (#551). The summary is the row's whole point, so it has to survive.
#[test]
fn harness_notices_resolve_and_keep_their_summary() {
    for key in ["panel.harness.task_finished_unnamed",
                "panel.harness.automated"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
    let finished = knot_core::l10n::t_with("panel.harness.task_finished",
                                           &[("summary", "Tests passed")]);
    assert_ne!(finished, "panel.harness.task_finished");
    assert!(finished.contains("Tests passed"), "{finished}");
}
