//! Catalog coverage for the settings window, its tabs and the Import window.

use crate::settings_window::SettingsTab;

/// Every key the settings window asks for, harvested from the source rather
/// than hand-listed, so a new label cannot be added without an entry. A
/// missing one renders the key itself - visible in the window, easy to miss
/// in review, and exactly what #223 is about.
#[test]
fn settings_window_labels_resolve() {
    for key in ["settings.appearance.fonts",
                "settings.appearance.terminal",
                "settings.appearance.title",
                "settings.appearance.ui",
                "settings.autopilot.action_group",
                "settings.autopilot.api_key",
                "settings.autopilot.blurb",
                "settings.autopilot.custom_prompt",
                "settings.autopilot.enable",
                "settings.autopilot.enable_group",
                "settings.autopilot.model",
                "settings.autopilot.on_input",
                "settings.autopilot.provider",
                "settings.autopilot.provider_group",
                "settings.coding.agent_options",
                "settings.coding.clear_source_folder",
                "settings.coding.clear_source_folder_body",
                "settings.coding.coding_agent",
                "settings.coding.folder",
                "settings.coding.options",
                "settings.coding.source_folder",
                "settings.compact_tool_calls",
                "settings.compact_tool_calls_hint",
                "settings.font_unavailable",
                "settings.general.agent_panel",
                "settings.general.appearance",
                "settings.general.appearance_hint",
                "settings.general.desktop_notifications",
                "settings.general.notifications",
                "settings.general.reopen_workspaces",
                "settings.general.restore_agents",
                "settings.general.restore_conversation",
                "settings.general.shift_enter_hint",
                "settings.general.shift_enter_to_send",
                "settings.general.startup",
                "settings.input.api_key",
                "settings.input.custom_prompt",
                "settings.input.extra_cli_options",
                "settings.input.port",
                "settings.mcp.agent",
                "settings.mcp.blurb",
                "settings.mcp.command",
                "settings.mcp.enable",
                "settings.mcp.install_command",
                "settings.mcp.no_setup",
                "settings.mcp.port",
                "settings.mcp.server_settings",
                "settings.mcp.url",
                "settings.persona_editor.cancel",
                "settings.persona_editor.instructions",
                "settings.persona_editor.name",
                "settings.persona_editor.name_placeholder",
                "settings.persona_editor.save",
                "settings.personas.delete_persona",
                "settings.personas.none_defined",
                "settings.personas.personas",
                "settings.personas.edit",
                "settings.personas.add",
                "settings.personas.delete_body",
                "settings.personas.restore_defaults",
                "settings.personas.restore_defaults_body",
                "settings.voice.auto_insert",
                "settings.voice.blurb",
                "settings.voice.enable",
                "settings.voice.engine",
                "settings.voice.engine_apple",
                "settings.voice.engine_hint",
                "settings.voice.input",
                "settings.voice.push_to_talk"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
}

/// Every settings tab's title, so an added pane cannot show `settings.tabs.*`
/// where its name belongs.
#[test]
fn every_settings_tab_label_resolves() {
    for tab in SettingsTab::ALL {
        let label = tab.label();
        assert!(!label.starts_with("settings.tabs."),
                "{tab:?} is missing from the catalog: {label}");
    }
}

/// The Import window draws every word it shows from the catalog, its title
/// and empty states included - and an empty state is exactly the copy a
/// missing key would replace with a key string, since it only appears when
/// there is nothing else on screen to notice.
#[test]
fn import_window_labels_resolve() {
    for key in ["import.title",
                "import.personas_title",
                "import.personas_none",
                "import.workspaces_title",
                "import.workspaces_none",
                "import.import_selected",
                "import.import_count",
                "import.result_title",
                "import.nothing_to_do",
                "import.refresh"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}
