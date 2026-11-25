//! Decoding the preferences document: defaults for keys an older document
//! lacks, the `"SF Mono"` upgrade, the font-role swap gated on
//! `settingsVersion`, and the sidebar width clamp.

use serde_json::Value;
use tempfile::tempdir;

use super::super::*;
use super::load_document;
use crate::consts::{SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN};

/// A preferences document written before the setting existed must read as
/// on, so an upgrade starts reopening workspaces rather than silently not.
#[test]
fn legacy_settings_blob_defaults_reopen_workspaces_on() {
    let (_dir, s) = load_document(r#"{"restoreLayoutOnLaunch":true}"#);
    assert!(s.reopen_workspaces_on_launch);
}

#[test]
fn legacy_settings_blob_defaults_restore_conversation_off() {
    let (_dir, s) = load_document(r#"{"restoreLayoutOnLaunch":true}"#);
    assert!(s.restore_layout_on_launch);
    assert!(!s.restore_conversation_on_launch);
}

#[test]
fn persisted_sf_mono_upgrades_to_the_new_terminal_font_default() {
    let (_dir, s) = load_document(r#"{"terminalFontName":"SF Mono"}"#);
    assert_eq!(s.terminal_font_name, "JetBrains Mono");
}

#[test]
fn persisted_custom_terminal_font_is_not_overridden() {
    let (_dir, s) = load_document(r#"{"terminalFontName":"Fira Code"}"#);
    assert_eq!(s.terminal_font_name, "Fira Code");
}

#[test]
fn a_fresh_store_is_already_at_the_current_settings_version() {
    assert_eq!(Settings::default().settings_version,
               SETTINGS_VERSION_CURRENT);
}

#[test]
fn a_document_without_the_marker_reads_as_pre_migration() {
    // Not the container-level default, which is `SETTINGS_VERSION_CURRENT` -
    // the field's own serde default has to win on the deserialize path, or an
    // unmigrated document would declare itself migrated.
    assert_eq!(preferences::de_legacy_settings_version(), 0);

    let mut document: Value = serde_json::from_str("{}").unwrap();
    let object = document.as_object_mut().unwrap();
    assert!(!object.contains_key("settingsVersion"));

    let settings: Settings = serde_json::from_value(document).unwrap();
    assert_eq!(settings.settings_version, 0);
}

#[test]
fn the_font_defaults_name_what_they_draw() {
    let s = Settings::default();
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.ui_font_size, 16.0);
    assert_eq!(s.title_font_name, "Manrope");
    assert_eq!(s.title_font_size, 14.0);
}

#[test]
fn both_customized_fonts_are_exchanged() {
    let (_dir, s) = load_document(r#"{"uiFontName":"Helvetica Neue","uiFontSize":13,
                                      "titleFontName":"Palatino","titleFontSize":18}"#);
    assert_eq!(s.ui_font_name, "Palatino");
    assert_eq!(s.ui_font_size, 18.0);
    assert_eq!(s.title_font_name, "Helvetica Neue");
    assert_eq!(s.title_font_size, 13.0);
    assert_eq!(s.settings_version, SETTINGS_VERSION_CURRENT);
}

#[test]
fn one_customized_font_moves_and_the_other_takes_the_new_default() {
    let (_dir, s) = load_document(r#"{"titleFontName":"Palatino"}"#);
    assert_eq!(s.ui_font_name, "Palatino");
    // Not "Palatino" - an absent key stays absent through the exchange rather
    // than inheriting the other's value.
    assert_eq!(s.title_font_name, "Manrope");
}

#[test]
fn a_document_that_customized_neither_font_keeps_the_new_defaults() {
    let (_dir, s) = load_document(r#"{"appearanceMode":"dark"}"#);
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.ui_font_size, 16.0);
    assert_eq!(s.title_font_name, "Manrope");
    assert_eq!(s.title_font_size, 14.0);
}

#[test]
fn the_migration_does_not_run_twice() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,
                                      "uiFontName":"Adamina","titleFontName":"Manrope"}"#);
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.title_font_name, "Manrope");
}

#[test]
fn the_migration_leaves_the_terminal_font_alone() {
    let (_dir, s) = load_document(r#"{"terminalFontName":"Fira Code","terminalFontSize":11,
                                      "uiFontName":"Helvetica Neue","titleFontName":"Palatino"}"#);
    assert_eq!(s.terminal_font_name, "Fira Code");
    assert_eq!(s.terminal_font_size, 11.0);
}

#[test]
fn a_fresh_store_opens_the_sidebar_at_the_default_width() {
    assert_eq!(Settings::default().sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn a_document_without_the_sidebar_width_takes_the_default() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"mcpServerPort":9000}"#);
    assert_eq!(s.mcp_server_port, 9000);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn an_in_range_sidebar_width_is_honored() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":320}"#);
    assert_eq!(s.sidebar_width, 320.0);
}

#[test]
fn a_sidebar_width_below_the_minimum_clamps_up() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":40}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_MIN);
}

#[test]
fn a_sidebar_width_above_the_maximum_clamps_down() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":5000}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_MAX);
}

#[test]
fn a_non_numeric_sidebar_width_leaves_the_default() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":"wide"}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn legacy_settings_blob_defaults_compact_tool_calls_off() {
    let (_dir, s) = load_document(r#"{"restoreLayoutOnLaunch":true}"#);
    assert!(!s.agent_panel_compact_tool_calls);
}

#[test]
fn compact_tool_calls_round_trips_through_the_store() {
    let dir = tempdir().unwrap();
    let mut s = Settings::load_from_root(dir.path()).unwrap();
    assert!(!s.agent_panel_compact_tool_calls);
    s.agent_panel_compact_tool_calls = true;
    s.persist().unwrap();

    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert!(reloaded.agent_panel_compact_tool_calls);
}
