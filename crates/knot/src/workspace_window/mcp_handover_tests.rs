//! A handover terminal does not outlive the window that opened it (#443).
//!
//! A window that is opening has started nothing, so a handover terminal
//! already in its workspace was left by an earlier window that closed before
//! the terminal exited. Opening the workspace again removes it before
//! anything starts, rather than re-running the MCP flow unasked.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "mcp-handover-test-inert";

struct Leftover {
    store:     Arc<Mutex<knot_agents::AgentStore>>,
    owner:     Uuid,
    shell:     Uuid,
    terminal:  Uuid,
    workspace: Uuid,
    _folder:   TempDir,
}

/// A workspace whose owner has an ordinary shell companion and a handover
/// terminal from an earlier window, with the terminal as the remembered
/// selection.
fn leftover() -> Leftover {
    let folder = TempDir::new().expect("a temporary agent folder");
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace = space.id;
    store.add_workspace(space);
    let owner = store.create(folder.path().to_string_lossy().into_owned(),
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(workspace),
                                                          ..Default::default() });
    let shell = store.create_shell_companion(owner)
                     .expect("an owner takes a companion");
    let terminal = store.open_mcp_handover(knot_agents::McpHandoverRequest { owner,
                                                                             server: "github",
                                                                             command: "true",
                                                                             name: "MCP" })
                        .expect("an owner can hand over")
                        .id();
    store.set_workspace_active_agents(workspace, vec![terminal]);
    Leftover { store: Arc::new(Mutex::new(store)),
               owner,
               shell,
               terminal,
               workspace,
               _folder: folder }
}

#[gpui_kit::test]
fn opening_the_workspace_removes_a_leftover_handover_terminal(cx: &mut TestAppContext) {
    let leftover = leftover();
    let settings = TempDir::new().expect("a temporary settings root");
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));

    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     crate::settings_global::install(
                         knot_core::Settings::with_store_root(settings.path()),
                         cx,
                     );
                     WorkspaceWindow::open(Arc::clone(&leftover.store),
                                           messages,
                                           leftover.workspace,
                                           cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(leftover.workspace), cx)
                         .expect("opening a workspace registers its view")
                 });

    {
        let store = leftover.store.lock();
        assert!(store.agent(leftover.terminal).is_none(),
                "the leftover handover terminal must go");
        assert!(store.agent(leftover.owner).is_some() && store.agent(leftover.shell).is_some(),
                "the owner and its ordinary companion stay");
    }
    let selected = cx.read(|cx| view.read(cx).selected_agent_id());
    assert_ne!(selected,
               Some(leftover.terminal),
               "the window must not select a terminal it removed");
    assert!(selected.is_some(), "it falls back to an agent that remains");

    let saved = cx.read(|cx| crate::settings_global::read(cx).saved_agents.clone());
    assert!(saved.iter().all(|agent| agent.id != leftover.terminal),
            "the removal is persisted, not only made in memory");
}
