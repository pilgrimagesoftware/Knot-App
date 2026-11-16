//! The search's contract: rankings land through `drain`, only for the
//! filter asked, and a superseded one never shows.

use std::sync::Arc;
use std::time::Duration;

use super::*;

fn listing(paths: &[&str]) -> Arc<Vec<LookupEntry>> {
    Arc::new(paths.iter()
                  .map(|path| LookupEntry::new(*path, ""))
                  .collect())
}

/// Waits for a ranking to land, as the repaint poll would.
fn settle(search: &mut FileSearch) -> bool {
    for _ in 0..500 {
        if search.drain() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn tokens(matches: Option<&[LookupMatch]>) -> Vec<String> {
    matches.unwrap_or_default()
           .iter()
           .map(|found| found.entry.token.clone())
           .collect()
}

#[test]
fn a_ranking_lands_for_the_filter_asked() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let mut search = FileSearch::default();
    let files = listing(&["src/lib.rs", "README.md"]);

    search.request(runtime.handle(), &files, "lib", Duration::ZERO);
    assert_eq!(search.matches_for("lib"), None, "nothing before it lands");
    assert!(settle(&mut search));

    assert_eq!(tokens(search.matches_for("lib")), vec!["src/lib.rs"]);
    assert_eq!(search.matches_for("li"),
               None,
               "only for exactly the filter asked");
}

/// A query typed before the last finished: only the newest lands.
#[test]
fn a_superseded_ranking_never_shows() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let mut search = FileSearch::default();
    let files = listing(&["src/lib.rs", "src/main.rs"]);

    search.request(runtime.handle(), &files, "lib", Duration::from_millis(200));
    search.request(runtime.handle(), &files, "main", Duration::ZERO);
    assert!(settle(&mut search));
    std::thread::sleep(Duration::from_millis(300));
    search.drain();

    assert_eq!(tokens(search.matches_for("main")), vec!["src/main.rs"]);
    assert_eq!(search.matches_for("lib"), None);
}

/// Asked from every frame, the same filter ranks once.
#[test]
fn asking_again_for_the_same_filter_does_not_rank_again() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let mut search = FileSearch::default();
    let files = listing(&["src/lib.rs"]);

    search.request(runtime.handle(), &files, "lib", Duration::ZERO);
    assert!(settle(&mut search));
    search.request(runtime.handle(), &files, "lib", Duration::ZERO);
    std::thread::sleep(Duration::from_millis(50));

    assert!(!search.drain(), "no second ranking was started");
    assert_eq!(tokens(search.matches_for("lib")), vec!["src/lib.rs"]);
}

/// A new listing makes the shown ranking stale: it goes, and the next
/// request ranks the new listing.
#[test]
fn invalidating_forgets_the_ranking() {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let mut search = FileSearch::default();
    search.request(runtime.handle(),
                   &listing(&["src/lib.rs"]),
                   "lib",
                   Duration::ZERO);
    assert!(settle(&mut search));

    search.invalidate();
    assert_eq!(search.matches_for("lib"), None);
    search.request(runtime.handle(),
                   &listing(&["lib/new.rs"]),
                   "lib",
                   Duration::ZERO);
    assert!(settle(&mut search));
    assert_eq!(tokens(search.matches_for("lib")), vec!["lib/new.rs"]);
}

/// Batches merge to the whole listing's order: a listing larger than one
/// batch ranks the same as ranking it at once.
#[test]
fn batches_merge_to_the_whole_ranking() {
    let paths: Vec<String> = (0..(FILE_SEARCH_BATCH * 2 + 7)).map(|i| format!("dir{i}/file{i}.rs"))
                                                             .collect();
    let entries: Vec<LookupEntry> = paths.iter()
                                         .map(|path| LookupEntry::new(path, ""))
                                         .collect();
    let current = AtomicU64::new(1);

    let batched = rank_in_batches(&entries, "file1", &current, 1).expect("not superseded");
    let whole = fuzzy::matching(&entries, "file1");

    assert_eq!(batched, whole);
}

#[test]
fn a_superseded_ranking_stops() {
    let entries = vec![LookupEntry::new("src/lib.rs", "")];
    let current = AtomicU64::new(2);
    assert_eq!(rank_in_batches(&entries, "lib", &current, 1), None);
}
