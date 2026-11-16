//! Every label the scheduling and idle-shutdown controls show resolves from
//! the catalog (#404). Its own module rather than more lines in
//! `l10n_catalog`, which is at the file-size limit; the editor's error keys
//! and the row and countdown lines are checked beside the code that uses them
//! (`agent_editor::scheduling`, `schedule_text`).

#[test]
fn every_scheduling_label_resolves() {
    for key in ["settings.general.idle",
                "settings.general.idle_action",
                "settings.general.idle_action_off",
                "settings.general.idle_action_deactivate",
                "settings.general.idle_action_reset",
                "settings.general.idle_after",
                "settings.general.idle_hint",
                "agent_editor.schedule",
                "agent_editor.schedule_hint",
                "agent_editor.schedule_kind.none",
                "agent_editor.schedule_kind.daily",
                "agent_editor.schedule_kind.weekdays",
                "agent_editor.schedule_kind.days",
                "agent_editor.schedule_kind.every_hours",
                "agent_editor.schedule_kind.cron",
                "agent_editor.idle",
                "agent_editor.idle_hint",
                "agent_editor.idle_minutes",
                "agent_editor.idle_kind.inherit",
                "agent_editor.idle_kind.off",
                "agent_editor.idle_kind.deactivate",
                "agent_editor.idle_kind.reset"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The idle default's duration keeps its number.
#[test]
fn the_idle_duration_label_keeps_its_minutes() {
    let label = knot_core::l10n::t_with("settings.general.idle_minutes", &[("minutes", "45")]);
    assert!(label.contains("45") && !label.contains("%{"), "got {label}");
}
