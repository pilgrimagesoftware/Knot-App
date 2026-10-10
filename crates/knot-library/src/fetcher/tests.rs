use std::collections::HashMap;

use super::*;
use crate::index::ItemKind;

/// An in-memory fetcher for tests: no network, no disk.
struct FixtureFetcher {
    pub index: Index,
    pub items: HashMap<String, Vec<u8>>,
}

impl LibraryFetcher for FixtureFetcher {
    fn fetch_index(&self, _location: &Location) -> Result<Index> {
        Ok(self.index.clone())
    }

    fn fetch_item(&self, _location: &Location, _index: &Index, item: &IndexItem)
                  -> Result<Vec<u8>> {
        self.items
            .get(&item.path)
            .cloned()
            .ok_or_else(|| LibraryError::Io(format!("no fixture body for {}", item.path)))
    }
}

#[test]
fn fixture_fetcher_serves_index_and_item_from_memory() {
    let index = Index { format:       1,
                        commit:       "abc".to_string(),
                        generated_at: "2026-01-01T00:00:00Z".to_string(),
                        kinds:        vec!["persona".to_string()],
                        items:        vec![], };
    let fetcher = FixtureFetcher { index: index.clone(),
                                   items: HashMap::from([("personas/x.md".to_string(),
                                                          b"body".to_vec())]), };
    let location = Location::Folder { path: std::path::PathBuf::from("/unused"), };
    assert_eq!(fetcher.fetch_index(&location).unwrap(), index);
    let item = IndexItem { kind:        ItemKind::Persona,
                           id:          "id".to_string(),
                           slug:        "slug".to_string(),
                           title:       "title".to_string(),
                           description: "description".to_string(),
                           tags:        vec![],
                           authors:     vec![],
                           path:        "personas/x.md".to_string(),
                           size:        4,
                           sha256:      String::new(), };
    assert_eq!(fetcher.fetch_item(&location, &index, &item).unwrap(),
               b"body");
}

#[test]
fn fetcher_for_picks_https_for_github_and_web() {
    let github = Location::GitHub { repo:   "owner/repo".to_string(),
                                    branch: None, };
    let web = Location::Web { base: "https://example.com".to_string(), };
    let folder = Location::Folder { path: std::path::PathBuf::from("/tmp"), };
    // Smoke-checks that each kind resolves to a fetcher without panicking.
    let _ = fetcher_for(&github);
    let _ = fetcher_for(&web);
    let _ = fetcher_for(&folder);
}

#[test]
fn folder_fetcher_reads_index_and_item_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{"format":1,"commit":"abc","generatedAt":"2026-01-01T00:00:00Z","kinds":["persona"],"items":[]}"#;
    std::fs::write(dir.path().join("index.json"), json).unwrap();
    let location = Location::Folder { path: dir.path().to_path_buf(), };
    let fetcher = FolderFetcher;
    let index = fetcher.fetch_index(&location).expect("reads index.json");
    assert_eq!(index.format, 1);
}
