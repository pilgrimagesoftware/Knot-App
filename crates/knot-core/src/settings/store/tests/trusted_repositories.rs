//! Trusted repositories are a preference: they persist with the preferences
//! document, and a document written before the setting existed trusts none.

use std::fs;

use tempfile::tempdir;

use crate::consts::PREFERENCES_FILE;
use crate::{Settings, TrustedRepository};

#[test]
fn trusted_repositories_survive_a_restart() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    let trusted = TrustedRepository { path:     "/src/repo".to_owned(),
                                      settings:
                                          vec!["filter.lfs.clean=git-lfs clean -- %f".to_owned()], };
    settings.trusted_repositories.push(trusted.clone());
    settings.persist_preferences().expect("preferences persist");

    let loaded = Settings::load_from_root(dir.path()).expect("the store loads");

    assert_eq!(loaded.trusted_repositories, vec![trusted]);
}

#[test]
fn a_store_from_before_the_setting_trusts_nothing() {
    let dir = tempdir().expect("a temporary store root");
    let mut settings = Settings::with_store_root(dir.path());
    settings.trusted_repositories
            .push(TrustedRepository { path:     "/src/repo".to_owned(),
                                      settings: Vec::new(), });
    settings.persist_preferences().expect("preferences persist");
    let path = dir.path().join(PREFERENCES_FILE);
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("the document reads"))
            .expect("the document is JSON");
    let removed = document.as_object_mut()
                          .and_then(|object| object.remove("trustedRepositories"));
    assert!(removed.is_some(),
            "the setting is written under the key this test strips");
    fs::write(&path, document.to_string()).expect("the document writes");

    let loaded = Settings::load_from_root(dir.path()).expect("the store loads");

    assert!(loaded.trusted_repositories.is_empty());
}
