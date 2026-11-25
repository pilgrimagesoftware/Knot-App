//! Catalog coverage for the window-level chrome: About, bug report, View and
//! Window menus, and the sidebar status dot.

/// Every word the About window shows comes from the catalog, so a missing
/// key would ship the key string itself where the version, copyright or a
/// credit line belongs.
#[test]
fn about_window_labels_resolve() {
    for key in ["about.title",
                "about.version_label",
                "about.build_label",
                "about.commit_unknown",
                "about.copy_details",
                "about.close",
                "about.copyright",
                "about.derived_from",
                "about.credits.author",
                "about.credits.license",
                "about.credits.built_with",
                "about.credits.toolkit",
                "about.credits.terminal",
                "about.credits.fonts"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// Every word the bug-report dialog and its menu item show, per
/// `bug-reporting`'s localization requirement. A missing key would ship the
/// key string itself as a label, a forge state or a status message.
#[test]
fn bug_report_labels_resolve() {
    for key in ["menu.help.report_issue",
                "bug_report.title",
                "bug_report.kind_label",
                "bug_report.subject_label",
                "bug_report.subject_placeholder",
                "bug_report.description_label",
                "bug_report.description_placeholder",
                "bug_report.diagnostics_label",
                "bug_report.diagnostics_hint",
                "bug_report.diagnostics_collecting",
                "bug_report.diagnostics.app",
                "bug_report.diagnostics.os",
                "bug_report.diagnostics.arch",
                "bug_report.delivery.checking",
                "bug_report.delivery.gh",
                "bug_report.delivery.missing",
                "bug_report.delivery.unauthenticated",
                "bug_report.delivery.failed",
                "bug_report.submitting",
                "bug_report.filed",
                "bug_report.failed",
                "bug_report.browser_ready",
                "bug_report.browser_failed",
                "bug_report.filed_screenshots",
                "bug_report.filed_screenshots_by_hand",
                "bug_report.browser_ready_screenshots",
                "bug_report.browser_ready_screenshots_by_hand",
                "bug_report.screenshots.label",
                "bug_report.screenshots.hint",
                "bug_report.screenshots.add",
                "bug_report.screenshots.remove",
                "bug_report.screenshots.skipped",
                "bug_report.screenshots.body_note",
                "bug_report.cancel",
                "bug_report.report"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The View menu's labels (`app-menu`).
#[test]
fn view_menu_labels_resolve() {
    for key in ["menu.view.dashboard",
                "menu.view.pull_requests",
                "menu.view.focus_agent_input",
                "menu.view.jump_to_bottom",
                "menu.view.collapse_tool_calls",
                "menu.view.select_agent"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
}

/// The window title and the one error the sidebar can show, neither of
/// which is reachable from a label lookup.
/// The Window menu's own items. A missing key here ships the key string as a
/// menu label, in the one menu a user goes to when they have lost a window.
#[test]
fn window_menu_labels_resolve() {
    for key in ["menu.window.command_center", "menu.window.workspaces"] {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn window_chrome_labels_resolve() {
    for key in ["broadcast.window_title",
                "workspace.name_placeholder",
                "workspace.missing",
                "workspace.choose_agent"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }

    let error = knot_core::l10n::t_with("sidebar.width_error", &[("error", "disk full")]);
    assert!(error.contains("disk full"),
            "the width error must say what failed: {error}");
    assert!(!error.contains("%{error}"));
}

/// The sidebar status dot's idle tooltip (#582). A missing key would ship
/// the key string itself as the tooltip.
#[test]
fn status_dot_tooltip_resolves() {
    assert_ne!(knot_core::l10n::t("sidebar.status_idle_since"),
               "sidebar.status_idle_since",
               "sidebar.status_idle_since is missing from the catalog");
}
