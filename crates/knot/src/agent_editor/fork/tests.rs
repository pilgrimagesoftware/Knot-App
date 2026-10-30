//! Fork Agent carries the source's whole setup (#38): into the prefill, onto
//! the editor's form, and through Create onto the new agent.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::TestAppContext;
use gpui_kit::component::Root;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::agent_editor::AgentEditor;
use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::agent_editor::parse_capability_tags;

/// A source agent in `folder` with every setup field away from its default,
/// and a conversation to fork.
fn source_store(folder: &str) -> (knot_agents::AgentStore, Uuid, Uuid) {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let source =
        store.create(folder.to_string(),
                     knot_agents::CreateOptions { name: Some("Builder".to_string()),
                                                  avatar: Some("🦀".to_string()),
                                                  agent_type: Some("claude".to_string()),
                                                  persona_id: Some(Uuid::new_v4()),
                                                  activation_mode:
                                                      knot_core::ActivationMode::Active,
                                                  description: "Builds Knot".to_string(),
                                                  capabilities: parse_capability_tags("rust, knot"),
                                                  cost_tier: knot_core::CostTier::High,
                                                  workspace_id: Some(workspace_id),
                                                  ..Default::default() });
    store.set_session_id(source, "sess-source".to_string());
    store.set_session_config_option(source, "model".to_string(), "opus".to_string())
         .unwrap();
    store.set_session_config_option(source, "effort".to_string(), "high".to_string())
         .unwrap();
    (store, source, workspace_id)
}

#[test]
fn a_fork_prefill_carries_the_whole_setup() {
    let (store, source, _) = source_store("/tmp/source");
    let agent = store.agent(source).unwrap();

    let prefill = AgentPrefill::fork_of(agent);

    assert_eq!(prefill.name.as_deref(), Some("Builder (fork)"));
    assert_eq!(prefill.avatar.as_deref(), Some("🦀"));
    assert_eq!(prefill.folder.as_deref(), Some("/tmp/source"));
    assert_eq!(prefill.agent_type.as_deref(), Some("claude"));
    assert_eq!(prefill.persona_id, agent.persona_id);
    assert_eq!(prefill.description.as_deref(), Some("Builds Knot"));
    assert_eq!(prefill.capabilities, Some(agent.capabilities.clone()));
    assert_eq!(prefill.cost_tier, Some(knot_core::CostTier::High));
    assert_eq!(prefill.activation_mode,
               Some(knot_core::ActivationMode::Active));
    assert_eq!(prefill.session_config, agent.session_config);
    assert_eq!(prefill.session_id.as_deref(), Some("sess-source"));
    assert_eq!(prefill.created_by, None, "a fork belongs to no one");
    assert!(!prefill.is_companion);
}

/// The fork editor, open over a source set up by [`source_store`].
struct Opened {
    store:    Arc<Mutex<knot_agents::AgentStore>>,
    expected: knot_agents::Agent,
    window:   gpui_kit::AnyWindowHandle,
    editor:   gpui_kit::Entity<AgentEditor>,
    created:  Arc<Mutex<Option<Uuid>>>,
    _dir:     TempDir,
}

fn open_fork_editor(cx: &mut TestAppContext) -> Opened {
    let dir = TempDir::new().expect("a temporary settings root");
    // Create refuses a folder that does not exist.
    let folder = dir.path().join("source");
    std::fs::create_dir(&folder).unwrap();
    let (store, source, workspace_id) = source_store(&folder.to_string_lossy());
    let expected = store.agent(source).unwrap().clone();
    let store = Arc::new(Mutex::new(store));
    let created = Arc::new(Mutex::new(None));
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          let created = Arc::clone(&created);
          open_agent_editor(Arc::clone(&store),
                            AgentEditorRequest { workspace_id,
                                                 prefill: AgentPrefill::fork_of(&expected),
                                                 insert_after: Some(source),
                                                 edit_target: None },
                            move |id, _, _| *created.lock() = Some(id),
                            cx);
      });
    cx.run_until_parked();
    let window = cx.update(|cx| cx.windows().into_iter().next().expect("the editor opened"));
    let editor = cx.update(|cx| {
                       window.downcast::<Root>()
                             .expect("the editor's root is a Root")
                             .read(cx)
                             .expect("the editor is open")
                             .view()
                             .clone()
                             .downcast::<AgentEditor>()
                             .expect("the Root holds the agent editor")
                   });
    Opened { store,
             expected,
             window,
             editor,
             created,
             _dir: dir }
}

impl Opened {
    /// Presses Create and returns the agent it made.
    fn create(&self, cx: &mut TestAppContext) -> knot_agents::Agent {
        cx.update(|cx| {
              self.window
                  .update(cx, |_, window, cx| {
                      self.editor
                          .update(cx, |editor, cx| editor.create(window, cx));
                  })
                  .expect("the editor is open");
          });
        let id = self.created.lock().expect("Create reports the new agent");
        self.store
            .lock()
            .agent(id)
            .expect("the fork exists")
            .clone()
    }
}

/// The editor opens on the source's setup, and Create makes an agent with
/// it, session setup included, continuing the source's conversation.
#[gpui_kit::test]
fn forking_through_the_editor_creates_the_same_setup(cx: &mut TestAppContext) {
    let opened = open_fork_editor(cx);
    let expected = &opened.expected;

    cx.update(|cx| {
          let form = opened.editor.read(cx);
          assert_eq!(form.description_input.read(cx).value(), "Builds Knot");
          assert_eq!(parse_capability_tags(&form.capabilities_input.read(cx).value()),
                     expected.capabilities);
          assert_eq!(form.cost_tier, knot_core::CostTier::High);
          assert_eq!(form.activation_mode, knot_core::ActivationMode::Active);
      });

    let fork = opened.create(cx);
    assert_eq!(fork.name, "Builder (fork)");
    assert_eq!(fork.description, "Builds Knot");
    assert_eq!(fork.capabilities, expected.capabilities);
    assert_eq!(fork.cost_tier, knot_core::CostTier::High);
    assert_eq!(fork.activation_mode, knot_core::ActivationMode::Active);
    assert_eq!(fork.persona_id, expected.persona_id);
    assert_eq!(fork.session_config,
               BTreeMap::from([("effort".to_string(), "high".to_string()),
                               ("model".to_string(), "opus".to_string())]));
    assert_eq!(fork.resume_session_id.as_deref(), Some("sess-source"));
    assert!(fork.fork_session);
}

/// A fork whose type the user changed in the form starts its own session:
/// a Claude session id or model means nothing to Codex.
#[gpui_kit::test]
fn a_fork_of_another_type_starts_fresh(cx: &mut TestAppContext) {
    let opened = open_fork_editor(cx);
    cx.update(|cx| {
          opened.editor
                .update(cx, |editor, _| editor.agent_type = "codex".to_string());
      });

    let fork = opened.create(cx);

    assert_eq!(fork.agent_type, "codex");
    assert_eq!(fork.resume_session_id, None, "no Claude session to resume");
    assert!(!fork.fork_session);
    assert!(fork.session_config.is_empty(), "no Claude model to replay");
    assert_eq!(fork.description, "Builds Knot",
               "the registry setup still carries");
}

#[test]
fn only_the_source_type_carries_the_session() {
    let prefill = AgentPrefill { agent_type: Some("claude".to_string()),
                                 ..Default::default() };

    assert!(prefill.carries_session_to("claude"));
    assert!(!prefill.carries_session_to("codex"));
    assert!(!AgentPrefill::default().carries_session_to("claude"),
            "nothing to carry without a source type");
}
