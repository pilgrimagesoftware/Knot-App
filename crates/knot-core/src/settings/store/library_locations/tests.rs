use tempfile::tempdir;

use super::*;
use crate::settings::store::Settings;

fn github(repo: &str) -> Location {
    Location::GitHub { repo: repo.to_string(), branch: None }
}

#[test]
fn a_store_without_a_library_locations_document_loads_empty_and_writes_none() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9101;

    s.persist().unwrap();

    assert!(Settings::load_from_root(dir.path()).unwrap().library_locations.is_empty());
    assert!(!dir.path().join("library-locations.json").exists());
}

#[test]
fn add_rename_and_remove_round_trip() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let id = s.add_library_location("Team Library", github("acme/team-library")).unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(back.library_locations.len(), 1);
    assert_eq!(back.library_locations[0].name, "Team Library");

    let mut s = back;
    s.rename_library_location(id, "Renamed").unwrap();
    assert_eq!(s.library_locations[0].name, "Renamed");

    s.remove_library_location(id).unwrap();
    assert!(s.library_locations.is_empty());
}

#[test]
fn add_rejects_a_blank_name() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let err = s.add_library_location("   ", github("acme/team-library")).unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert!(s.library_locations.is_empty());
}

#[test]
fn add_rejects_a_malformed_location() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    let err = s.add_library_location("Bad", github("not-owner-slash-repo")).unwrap_err();
    assert!(matches!(err, Error::Config(_)));
    assert!(s.library_locations.is_empty());
}

#[test]
fn remove_is_a_no_op_for_an_unknown_id() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.add_library_location("Team Library", github("acme/team-library")).unwrap();
    s.remove_library_location(Uuid::new_v4()).unwrap();
    assert_eq!(s.library_locations.len(), 1);
}
