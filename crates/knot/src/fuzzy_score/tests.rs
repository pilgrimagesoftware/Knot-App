//! The port against the Swift scorer's own output.
//!
//! `snapshot.jsonl` was produced by compiling the reference's
//! `FuzzyScorer.swift` (from `d0a84963^`) and running it over `corpus.txt`;
//! `openspec/changes/file-finder/reference/README.md` has the generator and
//! how to regenerate. Each line: `query`, `path`, `score`, `indices`, the top
//! 50 per query, ties by path.

use super::*;

const CORPUS: &str = include_str!("corpus.txt");
const SNAPSHOT: &str = include_str!("snapshot.jsonl");

#[derive(serde::Deserialize)]
struct Row {
    query:   String,
    path:    String,
    score:   i32,
    indices: Vec<usize>,
}

/// Every query's top 50 - paths, scores and matched offsets - equals the
/// Swift scorer's.
#[test]
fn the_ranking_matches_the_swift_reference() {
    let rows: Vec<Row> = SNAPSHOT.lines()
                                 .map(|line| serde_json::from_str(line).expect("snapshot row"))
                                 .collect();
    // Listed rather than read from the rows: a query the Swift scorer
    // matched to nothing has no rows, and must match nothing here too.
    let queries = ["lib",
                   "kgs",
                   "mod",
                   "readme",
                   "WsWin",
                   "spec.md",
                   "agentrow",
                   "Cargo",
                   "fuzzy",
                   "panelstate",
                   "consts",
                   "tests.rs",
                   "ACP",
                   "knot-git/src"];
    let paths: Vec<&str> = CORPUS.lines().filter(|line| !line.is_empty()).collect();
    for query in queries {
        let expected: Vec<(&str, i32, &[usize])> =
            rows.iter()
                .filter(|row| row.query == query)
                .map(|row| (row.path.as_str(), row.score, row.indices.as_slice()))
                .collect();
        let got = rank(query, paths.iter().copied());
        let got: Vec<(&str, i32, &[usize])> =
            got.iter()
               .map(|(path, found)| (*path, found.score, found.indices.as_slice()))
               .collect();
        assert_eq!(got, expected, "query {query:?}");
    }
}

#[test]
fn an_empty_pattern_matches_no_file() {
    assert_eq!(score_file("", "src/lib.rs"), None);
    assert!(rank("", ["src/lib.rs"]).is_empty());
}

#[test]
fn a_pattern_longer_than_the_path_does_not_match() {
    assert_eq!(score("abcdef", "abc"), None);
}

/// `NSString`'s rules, checked against Swift: a leading-dot name has no
/// extension, nor does one ending in a dot; a trailing slash is dropped.
#[test]
fn file_names_and_stems_follow_nsstring() {
    assert_eq!(last_component("a/.gitignore"), ".gitignore");
    assert_eq!(last_component("dir/"), "dir");
    assert_eq!(stem(".gitignore"), ".gitignore");
    assert_eq!(stem("b."), "b.");
    assert_eq!(stem(".env.local"), ".env");
    assert_eq!(stem("foo.tar.gz"), "foo.tar");
}

/// Offsets are `char` offsets, so a multi-byte character before the match
/// moves them by one, not by its byte length.
#[test]
fn offsets_count_characters() {
    let found = score_file("lib", "crates/é/lib.rs").expect("matches");
    assert_eq!(found.indices, vec![9, 10, 11]);
}
