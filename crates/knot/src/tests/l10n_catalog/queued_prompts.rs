//! Catalog coverage for the queued-prompt row: control labels and where a
//! queued prompt says it came from.

use crate::workspace_window;

/// The queued-row controls carry no visible text of their own, so their
/// tooltips and accessibility labels are the only thing naming them - a
/// missing key would ship the key string itself as the button's name.
#[test]
fn queued_message_control_labels_resolve() {
    for key in ["panel.queued",
                "panel.queued_inbox_nudge",
                "panel.failed",
                "panel.retry",
                "panel.retry_queued",
                "panel.delete_queued",
                "panel.edit_queued",
                "panel.replace_composer_title",
                "panel.replace_composer_body",
                "panel.retry_connect",
                "panel.lookup_hint",
                "panel.mentions_gathering",
                "panel.mentions_capped"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn the_queued_status_label_follows_the_failed_mark() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::User),
               knot_core::l10n::t("panel.queued"));
    assert_eq!(queued_status_label(true, PromptOrigin::User),
               knot_core::l10n::t("panel.failed"));
}

/// A queued nudge is labelled as one, so a prompt the user never typed does
/// not read as one they did - except when it failed, where the state the
/// user must act on wins.
#[test]
fn a_queued_inbox_nudge_says_where_it_came_from() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.queued_inbox_nudge"));
    assert_ne!(queued_status_label(false, PromptOrigin::InboxNudge),
               queued_status_label(false, PromptOrigin::User));
    assert_eq!(queued_status_label(true, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.failed"));
}

/// `queued-message-management` - "A queued startup prompt is labelled as
/// one": distinct from both a user's prompt and an inbox check.
#[test]
fn a_queued_startup_prompt_says_where_it_came_from() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    let label = queued_status_label(false, PromptOrigin::Startup);
    assert_eq!(label, knot_core::l10n::t("panel.queued_startup_prompt"));
    assert_ne!(label, "panel.queued_startup_prompt", "the key must resolve");
    assert_ne!(label, queued_status_label(false, PromptOrigin::User));
    assert_ne!(label, queued_status_label(false, PromptOrigin::InboxNudge));
    assert_eq!(queued_status_label(true, PromptOrigin::Startup),
               knot_core::l10n::t("panel.failed"));
}
