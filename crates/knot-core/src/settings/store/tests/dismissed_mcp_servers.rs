//! The MCP servers a user has dismissed (#402) are a preference: they persist
//! with the preferences document, and a document written before the setting
//! existed reads as nothing dismissed.

use std::fs;

use tempfile::tempdir;

use crate::Settings;
use crate::consts::PREFERENCES_FILE;

#[test]
fn dismissed_servers_survive_a_restart() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    settings.dismissed_mcp_servers
            .insert("claude.ai Notion".to_owned());
    settings.persist_preferences().expect("preferences persist");

    let loaded = Settings::load_from_root(dir.path()).expect("the store loads");

    assert!(loaded.dismissed_mcp_servers.contains("claude.ai Notion"));
    assert_eq!(loaded.dismissed_mcp_servers.len(), 1);
}

#[test]
fn a_store_from_before_the_setting_dismisses_nothing() {
    let dir = tempdir().expect("a temporary store root");
    Settings::with_store_root(dir.path()).persist_preferences()
                                         .expect("preferences persist");
    // Strip the key, as a document written before #402 never had it.
    let path = dir.path().join(PREFERENCES_FILE);
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("the document reads"))
            .expect("the document is JSON");
    let removed = document.as_object_mut()
                          .and_then(|object| object.remove("dismissedMcpServers"));
    assert!(removed.is_some(),
            "the setting is written under the key this test strips");
    fs::write(&path, document.to_string()).expect("the document writes");

    let loaded = Settings::load_from_root(dir.path()).expect("the store loads");

    assert!(loaded.dismissed_mcp_servers.is_empty());
}
