//! An agent's git state follows its folder (#25): when the folder its git
//! is read from changes, what was read from the old one goes on the next
//! frame, and the panel's watch moves - rather than the old tree's status
//! and diff stat being served until they age out, and the watch following
//! the old tree for good.
//!
//! Checked on the frame after the change, not by waiting for the new
//! tree's numbers: the caches age out within seconds anyway, so only the
//! next frame tells "followed" from "eventually expired".

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::test_repo;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

const INERT_AGENT_TYPE: &str = "git-folder-test-inert";

/// A repository with one committed file, changed in the working tree when
/// `dirty`.
fn repository(dirty: bool) -> TempDir {
    let dir = TempDir::new().expect("a temporary repository");
    test_repo::init(dir.path(), "main");
    std::fs::write(dir.path().join("file.txt"), "one\n").expect("write the file");
    test_repo::git(dir.path(), &["add", "file.txt"]);
    test_repo::git(dir.path(), &["commit", "-qm", "root"]);
    if dirty {
        std::fs::write(dir.path().join("file.txt"), "one\ntwo\n").expect("change the file");
    }
    dir
}

struct Fixture {
    window:    VisualTestContext,
    view:      Entity<WorkspaceWindow>,
    store:     Arc<Mutex<knot_agents::AgentStore>>,
    agent:     Uuid,
    clean:     TempDir,
    dirty:     TempDir,
    _settings: TempDir,
}

/// A window with one agent selected, in the clean repository.
fn fixture(cx: &mut TestAppContext) -> Fixture {
    let (clean, dirty) = (repository(false), repository(true));
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agent = store.create(clean.path().to_string_lossy().into_owned(),
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let settings = TempDir::new().expect("a temporary settings root");
    let view =
        cx.update(|cx| {
              gpui_kit::init(cx);
              WindowRegistry::install(cx);
              crate::settings_global::install(knot_core::Settings::with_store_root(settings.path()),
                                                     cx);
              WorkspaceWindow::open_with_selection(Arc::clone(&store),
                                                   messages,
                                                   workspace_id,
                                                   Some(agent),
                                                   crate::workspace_window::OpenRoute::Manager,
                                                   cx);
              WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
          });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    Fixture { window: VisualTestContext::from_window(handle, cx),
              view,
              store,
              agent,
              clean,
              dirty,
              _settings: settings }
}

impl Fixture {
    fn frame(&mut self) {
        self.view.update(&mut self.window, |_, cx| cx.notify());
        self.window.run_until_parked();
    }

    /// Draws frames until `done` holds of the view, or fails after a few
    /// seconds. The reads run on a blocking pool, not on the test executor.
    fn draw_until(&mut self, what: &str, done: impl Fn(&WorkspaceWindow) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.frame();
            if self.view.read_with(&self.window, |view, _| done(view)) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn files_changed(view: &WorkspaceWindow, agent: Uuid) -> Option<u64> {
        view.diff_stats
            .get(&agent)
            .flatten()
            .map(|stats| stats.files_changed)
    }

    /// Moves the agent to the dirty repository, as Edit Agent does.
    fn move_to_dirty(&mut self) {
        let folder = self.dirty.path().to_string_lossy().into_owned();
        let mut store = self.store.lock();
        let agent = store.agent(self.agent).expect("the agent").clone();
        store.edit(self.agent,
                   knot_agents::EditRequest { name: agent.name.clone(),
                                              avatar: agent.avatar.clone(),
                                              folder: Some(folder),
                                              activation_mode: agent.activation_mode,
                                              ..Default::default() })
             .expect("the edit applies");
    }
}

/// The header's diff stat is the old tree's until the folder changes, and
/// not one frame after.
#[gpui_kit::test]
fn the_diff_stat_follows_a_changed_folder(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let agent = fixture.agent;
    fixture.draw_until("the clean tree's stat", |view| {
               Fixture::files_changed(view, agent) == Some(0)
           });

    fixture.move_to_dirty();
    fixture.frame();

    let stale = fixture.view.read_with(&fixture.window, |view, _| {
                                Fixture::files_changed(view, agent)
                            });
    assert_ne!(stale,
               Some(0),
               "the clean tree's stat is gone on the next frame");
    fixture.draw_until("the dirty tree's stat", |view| {
               Fixture::files_changed(view, agent) == Some(1)
           });
    drop(fixture.clean);
}

/// With the git panel open, its status and its watch move to the new tree.
#[gpui_kit::test]
fn the_git_panel_and_its_watch_follow_a_changed_folder(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let agent = fixture.agent;
    let clean = fixture.clean.path().to_string_lossy().into_owned();
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.toggle_git_panel(agent, &clean, cx)
                });
    fixture.draw_until("the clean tree's status", |view| {
               view.git.git_status.get(&agent).is_some()
           });
    let first_watch = fixture.view.read_with(&fixture.window, |view, _| {
                                      Arc::clone(view.git
                                                     .git_watches
                                                     .get(&agent)
                                                     .expect("the open panel watches its tree"))
                                  });

    fixture.move_to_dirty();
    fixture.frame();

    fixture.view.read_with(&fixture.window, |view, _| {
                    let watch = view.git
                                    .git_watches
                                    .get(&agent)
                                    .expect("the panel still watches a tree");
                    assert!(!Arc::ptr_eq(watch, &first_watch), "the watch moved");
                });
    fixture.draw_until("the dirty tree's change in the panel", |view| {
               view.git.git_status.holds(&agent, |status| match status {
                                      crate::git_panel::state::GitStatusSnapshot::Loaded(repo) => {
                                          !repo.entries.is_empty()
                                      }
                                      _ => false,
                                  })
           });
}

/// An agent that reports a worktree with `set-working-directory` (#25) has
/// its git read from there: the header's stat changes on the next frame,
/// without its folder changing or the agent restarting, and goes back when
/// it clears the working directory.
#[gpui_kit::test]
fn the_diff_stat_follows_a_reported_working_directory(cx: &mut TestAppContext) {
    let mut fixture = fixture(cx);
    let agent = fixture.agent;
    fixture.draw_until("the folder's stat", |view| {
               Fixture::files_changed(view, agent) == Some(0)
           });
    let worktree = fixture.dirty.path().to_string_lossy().into_owned();

    fixture.store.lock().set_working_dir(agent, Some(worktree));
    fixture.frame();

    let stale = fixture.view.read_with(&fixture.window, |view, _| {
                                Fixture::files_changed(view, agent)
                            });
    assert_ne!(stale,
               Some(0),
               "the folder's stat is gone on the next frame");
    fixture.draw_until("the worktree's stat", |view| {
               Fixture::files_changed(view, agent) == Some(1)
           });
    let folder = fixture.store
                        .lock()
                        .agent(agent)
                        .map(|agent| agent.folder.clone());
    assert_eq!(folder.as_deref(),
               Some(fixture.clean.path().to_string_lossy().as_ref()),
               "the folder itself is unchanged");

    fixture.store.lock().set_working_dir(agent, None);
    fixture.draw_until("the folder's stat again", |view| {
               Fixture::files_changed(view, agent) == Some(0)
           });
}
