//! The Codex usage banner (#291) in a real workspace window: drawn over a
//! Codex agent near its limit, not over any other agent, and gone once
//! dismissed for that reset. Usage is read through the window's
//! `codex_usage_source`, which these swap for a stub.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::codex_usage::{CodexUsage, UsageWindow};
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

struct Fixture {
    window: VisualTestContext,
    view:   Entity<WorkspaceWindow>,
    codex:  Uuid,
    other:  Uuid,
    _dir:   TempDir,
}

/// Codex at 82%, resetting in an hour.
fn near_limit() -> Option<CodexUsage> {
    usage(82.0)
}

/// Codex at 40%: room left.
fn room_left() -> Option<CodexUsage> {
    usage(40.0)
}

fn usage(used_percent: f64) -> Option<CodexUsage> {
    Some(CodexUsage { windows: vec![UsageWindow { used_percent,
                                                  resets_at: Some(SystemTime::now()
                                                                  + Duration::from_secs(3600)) }], })
}

/// A window over a Claude agent, then a Codex one added once the window is
/// open and reading usage through `source` - so no frame ever reads this
/// machine's own `~/.codex`.
fn window(source: fn() -> Option<CodexUsage>, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let create = |store: &mut knot_agents::AgentStore, agent_type: &str| {
        store.create("~/agent".to_string(),
                     knot_agents::CreateOptions { agent_type: Some(agent_type.to_string()),
                                                  workspace_id: Some(workspace_id),
                                                  ..Default::default() })
    };
    let other = create(&mut store, knot_core::agent_type::DEFAULT);
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
    cx.update(|cx| view.update(cx, |view, _| view.codex_usage_source = source));
    let codex = create(&mut store.lock(), knot_core::agent_type::CODEX);
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    Fixture { window: VisualTestContext::from_window(handle, cx),
              view,
              codex,
              other,
              _dir: dir }
}

impl Fixture {
    fn select(&mut self, id: Uuid) {
        self.view.update(&mut self.window, |view, cx| {
                     view.selected_agent = Some(id);
                     cx.notify();
                 });
        self.window.run_until_parked();
    }

    fn banner_drawn(&mut self) -> bool {
        self.window.debug_bounds("codex-usage-banner").is_some()
    }
}

#[gpui_kit::test]
fn a_codex_agent_near_its_limit_shows_the_banner(cx: &mut TestAppContext) {
    let mut fixture = window(near_limit, cx);

    fixture.select(fixture.codex);
    assert!(fixture.banner_drawn(), "Codex at 82% is near its limit");

    fixture.select(fixture.other);
    assert!(!fixture.banner_drawn(),
            "the usage is Codex's, not every agent's");
}

#[gpui_kit::test]
fn a_codex_agent_with_room_left_shows_nothing(cx: &mut TestAppContext) {
    let mut fixture = window(room_left, cx);

    fixture.select(fixture.codex);

    assert!(!fixture.banner_drawn());
}

/// Dismissed for this reset, it stays gone; the next window's own fill-up
/// would show it again.
#[gpui_kit::test]
fn a_dismissed_banner_stays_dismissed_for_that_reset(cx: &mut TestAppContext) {
    let mut fixture = window(near_limit, cx);
    fixture.select(fixture.codex);
    assert!(fixture.banner_drawn());

    fixture.view.update(&mut fixture.window, |view, cx| {
                    let resets = view.codex_usage
                                     .get(&())
                                     .flatten()
                                     .and_then(|usage| usage.windows[0].resets_at);
                    view.codex_usage_dismissed = Some(resets);
                    cx.notify();
                });
    fixture.window.run_until_parked();

    assert!(!fixture.banner_drawn());
}
