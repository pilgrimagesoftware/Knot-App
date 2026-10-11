use super::*;
use crate::index::ItemKind;

fn item(size: u64, sha256: &str) -> IndexItem {
    IndexItem { kind: ItemKind::Persona,
                id: "id".to_string(),
                slug: "slug".to_string(),
                title: "title".to_string(),
                description: "description".to_string(),
                tags: vec![],
                authors: vec![],
                path: "personas/x.md".to_string(),
                size,
                sha256: sha256.to_string() }
}

#[test]
fn verify_accepts_a_match() {
    let bytes = b"hello";
    let hash = hex_sha256(bytes);
    assert!(verify(bytes, &item(bytes.len() as u64, &hash)).is_ok());
}

#[test]
fn verify_rejects_a_size_mismatch() {
    let bytes = b"hello";
    let hash = hex_sha256(bytes);
    let err = verify(bytes, &item(999, &hash)).expect_err("size mismatch is refused");
    assert!(matches!(err, LibraryError::SizeMismatch { .. }));
}

#[test]
fn verify_rejects_a_hash_mismatch() {
    let bytes = b"hello";
    let err = verify(bytes, &item(bytes.len() as u64, "0".repeat(64).as_str()))
        .expect_err("hash mismatch is refused");
    assert!(matches!(err, LibraryError::HashMismatch { .. }));
}

#[test]
fn split_item_drops_front_matter_and_trims() {
    let bytes = b"---\nid: x\n---\n\n  Body text.  \n";
    assert_eq!(split_item(bytes).unwrap(), "Body text.");
}

#[test]
fn split_item_rejects_missing_closing_delimiter() {
    let bytes = b"---\nid: x\nBody with no close";
    let err = split_item(bytes).expect_err("unterminated front matter is refused");
    assert!(matches!(err, LibraryError::UnterminatedFrontMatter));
}

#[test]
fn split_item_rejects_an_empty_body() {
    let bytes = b"---\nid: x\n---\n   \n";
    let err = split_item(bytes).expect_err("empty body is refused");
    assert!(matches!(err, LibraryError::EmptyBody));
}
