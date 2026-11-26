//! The repository trust banner in a real workspace window: drawn while a
//! refused repository waits for an answer, gone once it is answered, and a
//! "Trust" answer saved. Each window gets its own registry, never the
//! process's, so no test changes how git runs for the others.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use knot_git::{ProgramSetting, RepositoryPrograms, TrustPolicy};
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::repository_trust::RepositoryTrust;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

struct Fixture {
    window:   VisualTestContext,
    view:     Entity<WorkspaceWindow>,
    registry: Arc<RepositoryTrust>,
    _dir:     TempDir,
}

fn lfs() -> RepositoryPrograms {
    RepositoryPrograms { repository: PathBuf::from("/src/repo"),
                         settings:   vec![ProgramSetting { submodule: None,
                                                           key:       "filter.lfs.clean".to_owned(),
                                                           value:
                                                               "git-lfs clean -- %f".to_owned(), }], }
}

fn window(cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    store.create("~/agent".to_string(),
                 knot_agents::CreateOptions { workspace_id: Some(workspace_id),
                                              ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view = cx.update(|cx| {
                   gpui_kit::init(cx);
                   WindowRegistry::install(cx);
                   crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()),
                                                      cx);
                   WorkspaceWindow::open(Arc::clone(&store),
                                         messages,
                                         workspace_id,
                                         crate::workspace_window::OpenRoute::Manager,
                                         cx);
                   WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
               });
    let registry = Arc::new(RepositoryTrust::default());
    cx.update(|cx| {
          view.update(cx, |view, _| {
                  view.repository_trust = Some(Arc::clone(&registry))
              })
      });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    Fixture { window: VisualTestContext::from_window(handle, cx),
              view,
              registry,
              _dir: dir }
}

impl Fixture {
    /// What the poll does each tick: notices the queue moved and repaints.
    fn tick(&mut self) -> bool {
        let moved = self.view.update(&mut self.window, |view, cx| {
                                 let moved = view.trust_queue_moved();
                                 if moved {
                                     cx.notify();
                                 }
                                 moved
                             });
        self.window.run_until_parked();
        moved
    }

    fn banner_drawn(&mut self) -> bool {
        self.window
            .debug_bounds("repository-trust-banner")
            .is_some()
    }
}

#[gpui_kit::test]
fn nothing_waiting_draws_no_banner(cx: &mut TestAppContext) {
    let mut fixture = window(cx);

    assert!(!fixture.tick());
    assert!(!fixture.banner_drawn());
}

/// A refusal arrives from a git call's thread with no context to notify
/// from; the poll's tick is what brings the banner to the screen.
#[gpui_kit::test]
fn a_refused_repository_is_asked_about_on_the_next_tick(cx: &mut TestAppContext) {
    let mut fixture = window(cx);

    fixture.registry.refused(&lfs());

    assert!(fixture.tick(), "the poll sees the queue move");
    assert!(fixture.banner_drawn());
}

#[gpui_kit::test]
fn trusting_saves_the_repository_and_hides_the_banner(cx: &mut TestAppContext) {
    let mut fixture = window(cx);
    fixture.registry.refused(&lfs());
    fixture.tick();

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.trust_repository(&lfs(), cx);
                });
    fixture.window.run_until_parked();

    assert!(!fixture.banner_drawn());
    assert!(fixture.registry.is_trusted(&lfs()));
    let saved = fixture.window.update(|_, cx| {
                                  crate::settings_global::read(cx).trusted_repositories
                                                                  .clone()
                              });
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].path, "/src/repo");
    assert_eq!(saved[0].settings,
               vec!["filter.lfs.clean=git-lfs clean -- %f".to_owned()]);
}

#[test]
fn the_banner_and_settings_text_resolve() {
    for key in ["repository_trust.trust",
                "repository_trust.not_now",
                "repository_trust.settings_group",
                "repository_trust.settings_hint",
                "repository_trust.settings_empty",
                "repository_trust.revoke"]
    {
        assert_ne!(knot_core::l10n::t(key), key, "{key} resolves");
    }
    for key in ["repository_trust.banner", "repository_trust.revoke_confirm"] {
        let text = knot_core::l10n::t_with(key, &[("repository", "/src/repo")]);
        assert!(text.contains("/src/repo"),
                "{key} names the repository: {text}");
    }
}

/// "Not now" is not a dead end (#523 review): the git panel says why it is
/// empty, and its Review button brings the banner back.
#[gpui_kit::test]
fn a_declined_repository_is_reviewed_again_from_the_git_panel(cx: &mut TestAppContext) {
    use gpui_kit::Modifiers;

    use crate::git_panel::state::GitStatusSnapshot;

    let mut fixture = window(cx);
    fixture.registry.refused(&lfs());
    fixture.tick();
    fixture.registry.decline(&lfs().repository);
    fixture.tick();
    assert!(!fixture.banner_drawn(), "Not now closes the banner");

    // The status read a refused call produces, recorded before the panel
    // opens so the panel draws it rather than reading the folder itself.
    fixture.view.update(&mut fixture.window, |view, cx| {
                    let id = view.store.lock().agents()[0].id;
                    let folder = view.agent_folder(id).expect("the agent's folder");
                    view.git
                        .git_status
                        .claim_refresh(id, crate::consts::GIT_STATUS_MAX_AGE)
                        .expect("nothing read yet")
                        .record(GitStatusSnapshot::Untrusted(lfs().repository));
                    view.toggle_git_panel(id, &folder, cx);
                });
    fixture.window.run_until_parked();
    assert!(fixture.window.debug_bounds("git-panel-untrusted").is_some());

    let review = fixture.window
                        .debug_bounds("git-panel-review-trust")
                        .expect("the Review button")
                        .center();
    fixture.window.simulate_click(review, Modifiers::none());
    fixture.window.run_until_parked();

    assert!(fixture.tick(), "the queue moved");
    assert!(fixture.banner_drawn(), "the banner asks again");
}
