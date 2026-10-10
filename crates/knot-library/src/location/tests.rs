use super::*;
use crate::index::ItemKind;

fn item(path: &str) -> IndexItem {
    IndexItem {
        kind: ItemKind::Persona,
        id: "id".to_string(),
        slug: "slug".to_string(),
        title: "title".to_string(),
        description: "description".to_string(),
        tags: vec![],
        authors: vec![],
        path: path.to_string(),
        size: 0,
        sha256: String::new(),
    }
}

#[test]
fn github_addresses_pin_the_commit_and_default_to_head() {
    let location = Location::GitHub {
        repo: "pilgrimagesoftware/Knot-Library".to_string(),
        branch: None,
    };
    assert_eq!(
        location.index_address(),
        "https://raw.githubusercontent.com/pilgrimagesoftware/Knot-Library/HEAD/index.json"
    );
    assert_eq!(
        location
            .item_address("abc123", &item("personas/x.md"))
            .unwrap(),
        "https://raw.githubusercontent.com/pilgrimagesoftware/Knot-Library/abc123/personas/x.md"
    );
}

#[test]
fn github_addresses_use_an_explicit_branch() {
    let location = Location::GitHub {
        repo: "acme/team-library".to_string(),
        branch: Some("main".to_string()),
    };
    assert_eq!(
        location.index_address(),
        "https://raw.githubusercontent.com/acme/team-library/main/index.json"
    );
}

#[test]
fn web_addresses_join_the_base() {
    let location = Location::Web {
        base: "https://example.com/lib".to_string(),
    };
    assert_eq!(
        location.index_address(),
        "https://example.com/lib/index.json"
    );
    assert_eq!(
        location
            .item_address("unused", &item("personas/x.md"))
            .unwrap(),
        "https://example.com/lib/personas/x.md"
    );
}

#[test]
fn folder_addresses_join_the_directory() {
    let location = Location::Folder {
        path: PathBuf::from("/tmp/my-library"),
    };
    assert_eq!(location.index_address(), "/tmp/my-library/index.json");
}

#[test]
fn validate_rejects_a_malformed_github_repo() {
    let location = Location::GitHub {
        repo: "acme".to_string(),
        branch: None,
    };
    assert!(location.validate().is_err());
}

#[test]
fn validate_rejects_a_non_https_web_base() {
    let location = Location::Web {
        base: "http://example.com".to_string(),
    };
    assert!(location.validate().is_err());
}

#[test]
fn validate_rejects_a_missing_folder() {
    let location = Location::Folder {
        path: PathBuf::from("/no/such/directory/anywhere"),
    };
    assert!(location.validate().is_err());
}

#[test]
fn validate_accepts_an_existing_folder() {
    let dir = tempfile::tempdir().unwrap();
    let location = Location::Folder {
        path: dir.path().to_path_buf(),
    };
    assert!(location.validate().is_ok());
}

#[test]
fn item_address_rejects_an_escaping_path() {
    let location = Location::Folder {
        path: PathBuf::from("/tmp/my-library"),
    };
    assert!(
        location
            .item_address("unused", &item("../../.ssh/config"))
            .is_err()
    );
}

#[test]
fn location_serializes_as_a_tagged_enum() {
    let location = Location::GitHub {
        repo: "owner/repo".to_string(),
        branch: None,
    };
    let json = serde_json::to_value(&location).unwrap();
    assert_eq!(json["kind"], "github");
    assert_eq!(json["repo"], "owner/repo");
}
