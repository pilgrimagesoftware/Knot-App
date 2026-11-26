use std::path::PathBuf;

use knot_git::{ProgramSetting, RepositoryPrograms, TrustPolicy};

use super::*;

fn programs(repository: &str, settings: &[(&str, &str)]) -> RepositoryPrograms {
    RepositoryPrograms { repository: PathBuf::from(repository),
                         settings:   settings.iter()
                                             .map(|(key, value)| {
                                                 ProgramSetting { key:   (*key).to_owned(),
                                                                  value: (*value).to_owned(), }
                                             })
                                             .collect(), }
}

#[test]
fn a_refused_repository_is_queued_once() {
    let trust = RepositoryTrust::default();
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);

    trust.refused(&lfs);
    let after_first = trust.generation();
    trust.refused(&lfs);

    assert_ne!(after_first, 0, "the first refusal changes the queue");
    assert_eq!(trust.generation(),
               after_first,
               "a repeat refusal queues nothing");
    assert_eq!(trust.next_pending(), Some(lfs));
}

#[test]
fn trusting_holds_for_exactly_the_entries_it_was_given() {
    let trust = RepositoryTrust::default();
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);
    trust.refused(&lfs);

    let saved = trust.trust(&lfs);

    assert!(trust.is_trusted(&lfs));
    assert_eq!(saved.settings,
               vec!["filter.lfs.clean=git-lfs clean".to_owned()]);
    assert_eq!(trust.next_pending(), None);
    let more = programs("/src/a",
                        &[("filter.lfs.clean", "git-lfs clean"),
                          ("diff.x.textconv", "cat")]);
    assert!(!trust.is_trusted(&more), "an added program is asked about");
    assert!(!trust.is_trusted(&programs("/src/b", &[("filter.lfs.clean", "git-lfs clean")])));
}

#[test]
fn trust_saved_earlier_is_honoured() {
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);
    let trust = RepositoryTrust::new(vec![TrustedRepository { path:     "/src/a".to_owned(),
                                                               settings: vec!["filter.lfs.clean=git-lfs clean".to_owned()], }]);

    assert!(trust.is_trusted(&lfs));
}

#[test]
fn not_now_stops_asking_this_session() {
    let trust = RepositoryTrust::default();
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);
    trust.refused(&lfs);

    trust.decline(&lfs.repository);
    let after_decline = trust.generation();
    trust.refused(&lfs);

    assert_eq!(trust.next_pending(), None);
    assert_eq!(trust.generation(), after_decline);
    assert!(!trust.is_trusted(&lfs), "declining trusts nothing");
}

#[test]
fn revoking_withdraws_trust_and_asks_again() {
    let trust = RepositoryTrust::default();
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);
    trust.trust(&lfs);
    trust.decline(&lfs.repository);

    trust.revoke("/src/a");
    trust.refused(&lfs);

    assert!(!trust.is_trusted(&lfs));
    assert_eq!(trust.next_pending(), Some(lfs));
}

/// "Not now" is not final: the declined repository is listed, and asking
/// to review it puts it back in front of the user.
#[test]
fn a_declined_repository_can_be_reconsidered() {
    let trust = RepositoryTrust::default();
    let lfs = programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]);
    trust.refused(&lfs);
    trust.decline(&lfs.repository);
    assert_eq!(trust.declined(), vec![lfs.clone()]);
    let before = trust.generation();

    trust.reconsider(&lfs.repository);

    assert_eq!(trust.next_pending(), Some(lfs));
    assert!(trust.declined().is_empty());
    assert_ne!(trust.generation(), before, "windows repaint for it");
}

/// While declined, the repository's latest programs are kept, so a review
/// asks about what it names now.
#[test]
fn a_reconsidered_repository_is_asked_about_as_it_is_now() {
    let trust = RepositoryTrust::default();
    trust.refused(&programs("/src/a", &[("filter.lfs.clean", "git-lfs clean")]));
    trust.decline(Path::new("/src/a"));
    let now = programs("/src/a", &[("diff.x.textconv", "cat")]);
    trust.refused(&now);

    trust.reconsider(Path::new("/src/a"));

    assert_eq!(trust.next_pending(), Some(now));
}

#[test]
fn reconsidering_a_repository_never_declined_does_nothing() {
    let trust = RepositoryTrust::default();
    let before = trust.generation();

    trust.reconsider(Path::new("/src/a"));

    assert_eq!(trust.next_pending(), None);
    assert_eq!(trust.generation(), before);
}
