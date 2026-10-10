//! Catalog coverage for the Library section's own copy, split out of
//! `l10n_catalog` once that file grew past the size limit.
//!
//! The Library section draws its location picker, its loading/error states,
//! and the add/rename/remove dialogs entirely from the catalog - a missing
//! key here would ship as the key string itself in the Import window.

/// Every key the Library section and its add/rename dialog ask for.
#[test]
fn import_library_labels_resolve() {
    for key in ["import.tab_library",
                "import.tab_personas",
                "import.tab_skwad",
                "import.library_title",
                "import.library_personas_title",
                "import.library_prompts_title",
                "import.library_none",
                "import.library_loading",
                "import.library_unreachable",
                "import.library_unsupported",
                "import.library_retry",
                "import.library_already_in_knot",
                "import.library_deleted_in_knot",
                "import.library_add",
                "import.library_rename",
                "import.library_remove",
                "import.library_dialog_add_title",
                "import.library_dialog_rename_title",
                "import.library_dialog_name_placeholder",
                "import.library_dialog_repo_placeholder",
                "import.library_dialog_branch_placeholder",
                "import.library_dialog_web_placeholder",
                "import.library_dialog_kind_github",
                "import.library_dialog_kind_web",
                "import.library_dialog_kind_folder",
                "import.library_dialog_choose_folder",
                "import.library_dialog_trust_note",
                "import.library_dialog_cancel",
                "import.library_dialog_add",
                "import.library_dialog_save",
                "settings.personas.import_from_library",
                "settings.prompts.import_from_library"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The remove-location confirmation, which names the location being
/// removed.
#[test]
fn library_remove_confirmation_resolves_and_keeps_its_name() {
    let title = knot_core::l10n::t_with("import.library_remove_title", &[("name", "Team Library")]);
    assert!(title.contains("Team Library"), "{title}");
    assert!(!title.contains("%{name}"),
            "left its placeholder unfilled: {title}");

    let body = knot_core::l10n::t_with("import.library_remove_body", &[("name", "Team Library")]);
    assert!(body.contains("Team Library"), "{body}");
    assert!(!body.contains("%{name}"),
            "left its placeholder unfilled: {body}");
}
