//! Catalog coverage for the checklist section's copy (#301), split out of
//! `l10n_catalog`, which is at the size limit.

#[test]
fn checklist_labels_resolve() {
    for key in ["artifact_panel.checklist",
                "artifact_panel.checklist_send",
                "artifact_panel.checklist_send_tooltip",
                "artifact_panel.checklist_sent",
                "artifact_panel.checklist_sent_tooltip",
                "artifact_panel.checklist_answer_placeholder"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}
