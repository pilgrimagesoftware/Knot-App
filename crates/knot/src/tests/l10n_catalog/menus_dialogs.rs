//! Catalog coverage for dialog copy, the agent and sidebar menu entries and the
//! menu confirmations.

use crate::agent_menu_entries::AgentMenuEntry;
use crate::app_state::AgentListBackgroundEntry;
use crate::app_state::SidebarMenuFacts;
use crate::app_state::sidebar_background_menu_entries;

/// The agent editor, the workspace manager and the broadcast sheet had no
/// catalog references at all before #223. Harvested from the source, so a
/// label added without an entry fails rather than shipping its key.
#[test]
fn dialog_labels_resolve() {
    for key in ["agent_editor.activation",
                "agent_editor.activation_hint",
                "agent_editor.avatar",
                "agent_editor.cancel",
                "agent_editor.capabilities",
                "agent_editor.capabilities_hint",
                "agent_editor.capabilities_placeholder",
                "agent_editor.coding_agent",
                "agent_editor.command",
                "agent_editor.cost_tier",
                "agent_editor.cost_tier_high",
                "agent_editor.cost_tier_hint",
                "agent_editor.cost_tier_low",
                "agent_editor.cost_tier_medium",
                "agent_editor.description",
                "agent_editor.description_placeholder",
                "agent_editor.error_choose_folder",
                "agent_editor.no_folder",
                "agent_editor.persona_none",
                "agent_editor.error_enter_name",
                "agent_editor.folder",
                "agent_editor.name",
                "agent_editor.persona",
                "agent_editor.shell_command",
                "agent_editor.title_edit",
                "agent_editor.title_new",
                "broadcast.cancel",
                "broadcast.placeholder",
                "broadcast.send",
                "workspace_manager.cancel",
                "workspace_manager.delete",
                "workspace_manager.delete_title",
                "workspace_manager.error_last_workspace",
                "workspace_manager.error_missing",
                "workspace_manager.error_name_empty",
                "workspace_manager.new",
                "workspace_manager.open",
                "workspace_manager.rename",
                "workspace_manager.title"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
}

/// Driven by `ALL` rather than a written-out key list, so a variant added
/// without a catalog entry fails here instead of drawing its own key in the
/// menu. Separators have no label and are skipped.
#[test]
fn every_menu_entry_label_resolves() {
    for entry in AgentMenuEntry::ALL {
        let Some(label) = entry.label()
        else {
            assert_eq!(entry, AgentMenuEntry::Separator);
            continue;
        };
        assert!(!label.starts_with("menu.agent."),
                "{entry:?} is missing from the catalog: {label}");
    }
}

/// The sidebar's background menu, the same way. It has no `ALL`, so the
/// entries come from a full menu - which this one always is, since it
/// disables rather than omits.
#[test]
fn every_sidebar_menu_entry_label_resolves() {
    for item in sidebar_background_menu_entries(SidebarMenuFacts::default()) {
        let Some(label) = item.entry.label()
        else {
            assert_eq!(item.entry, AgentListBackgroundEntry::Separator);
            continue;
        };
        assert!(!label.starts_with("menu.sidebar."),
                "{:?} is missing from the catalog: {label}",
                item.entry);
    }
}

/// The four confirmations, and the values they embed. A body that lost its
/// placeholder would ask "Restart ?" and still pass a resolution check, so
/// these assert the substitution as well as the key.
#[test]
fn menu_confirmation_copy_resolves_and_keeps_its_values() {
    for key in ["menu.agent.confirm.restart_title",
                "menu.agent.confirm.remove_title",
                "menu.sidebar.confirm.restart_all_title",
                "menu.sidebar.confirm.close_all_title"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }

    for key in ["menu.agent.confirm.restart_body",
                "menu.agent.confirm.remove_body"]
    {
        let body = knot_core::l10n::t_with(key, &[("name", "Alpha")]);
        assert!(body.contains("Alpha"), "{key} must name the agent: {body}");
        assert!(!body.contains("%{name}"),
                "{key} left its placeholder unfilled: {body}");
    }

    for key in ["menu.sidebar.confirm.restart_all_body",
                "menu.sidebar.confirm.close_all_body"]
    {
        let agents = knot_core::l10n::pluralize(3, "count.agent", "count.agents");
        let body = knot_core::l10n::t_with(key, &[("agents", &agents)]);
        assert!(body.contains("3 agents"), "{key} must say how many: {body}");
        assert!(!body.contains("%{agents}"),
                "{key} left its placeholder unfilled: {body}");
    }
}
