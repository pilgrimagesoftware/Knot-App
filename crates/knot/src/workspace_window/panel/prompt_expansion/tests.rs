//! A library prompt in the `/` lookup: listed as a prompt, and written in
//! place of the slash token - or dropped when the buffer moved on while it
//! resolved. Run against the composer the window builds, as
//! `tests::panel_lookup` does, since the caret and the surrounding text are
//! what the spec is about.
//!
//! The `{{clipboard}}` cases run the whole path through a real workspace
//! window, since what they pin is *when* the clipboard is read relative to
//! the hop to the background executor.

use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::ClipboardItem;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::{Context, Entity, IntoElement, Render, Window, div};
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::apply_expansion;
use crate::panel_commands::{LookupEntry, Matcher, active_token};
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::{
    PanelInputState, new_panel_input, panel_input_max_rows,
};

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "prompt-expansion-test-inert";

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn composer(cx: &mut TestAppContext, value: &str) -> (VisualTestContext, Entity<PanelInputState>) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let input =
        cx.update(|window, cx| new_panel_input(false, panel_input_max_rows(false), window, cx));
    let value = value.to_string();
    let caret = value.len();
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(value, window, cx);
             state.set_selected_range(caret..caret, cx);
         });
    (cx, input)
}

#[test]
fn a_library_prompt_is_listed_by_name_and_marked_as_a_prompt() {
    let gate = knot_core::Prompt::new("Run the gate", "make,\nthen commit").unwrap();
    let entries = [LookupEntry::library_prompt(&gate)];

    let matches = Matcher::Substring.matching(&entries, "gate");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].entry.token, "Run the gate");
    assert_eq!(matches[0].entry.description, "make, then commit");
    assert_eq!(matches[0].entry.prompt, Some(gate.id));
    assert!(LookupEntry::new("send", "x").prompt.is_none());
}

/// The spec's example: a `/gat` token on the second line is replaced by the
/// prompt's text, and the first line survives.
#[gpui_kit::test]
fn the_expansion_replaces_the_token_in_place(cx: &mut TestAppContext) {
    let (mut cx, input) = composer(cx, "note:\n/gat");
    let (snapshot, range) = cx.update(|_, cx| {
                                  let state = input.read(cx);
                                  let token = active_token(&state.value(), state.cursor())
                                      .expect("an active token");
                                  (state.value().to_string(), token.range)
                              });

    cx.update(|window, cx| {
          apply_expansion(&input, &snapshot, range, "make, then commit", window, cx)
      });

    assert_eq!(cx.update(|_, cx| input.read(cx).value().to_string()),
               "note:\nmake, then commit");
}

/// An edit made while the expansion resolved wins; the expansion is dropped.
#[gpui_kit::test]
fn a_stale_expansion_is_dropped(cx: &mut TestAppContext) {
    let (mut cx, input) = composer(cx, "/gat");
    let (snapshot, range) = cx.update(|_, cx| {
                                  let state = input.read(cx);
                                  let token = active_token(&state.value(), state.cursor())
                                      .expect("an active token");
                                  (state.value().to_string(), token.range)
                              });
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value("/gate typed on", window, cx)
         });

    cx.update(|window, cx| apply_expansion(&input, &snapshot, range, "expanded", window, cx));

    assert_eq!(cx.update(|_, cx| input.read(cx).value().to_string()),
               "/gate typed on");
}

struct Fixture {
    window: VisualTestContext,
    view:   Entity<WorkspaceWindow>,
    agent:  Uuid,
    prompt: Uuid,
    input:  Entity<PanelInputState>,
    _dir:   TempDir,
}

/// A workspace window over one inert agent, a library holding one prompt
/// whose text is `text`, and an empty composer.
fn window_with_prompt(cx: &mut TestAppContext, text: &str) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agent = store.create("/nonexistent/agent",
                             knot_agents::CreateOptions { agent_type:
                                                              Some(INERT_AGENT_TYPE.into()),
                                                          workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let prompt = knot_core::Prompt::new("Explain", text).unwrap();
    let prompt_id = prompt.id;
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.prompts.push(prompt);
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     crate::settings_global::install(settings, cx);
                     WorkspaceWindow::open(store, messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let mut window = VisualTestContext::from_window(handle, cx);
    let input =
        window.update(|window, cx| new_panel_input(false, panel_input_max_rows(false), window, cx));
    Fixture { window,
              view,
              agent,
              prompt: prompt_id,
              input,
              _dir: dir }
}

impl Fixture {
    fn copy(&mut self, item: ClipboardItem) {
        self.window.update(|_, cx| cx.write_to_clipboard(item));
    }

    /// Choose the prompt at the composer's empty caret, as the picker does.
    fn choose(&mut self) {
        let (agent, prompt, input) = (self.agent, self.prompt, self.input.clone());
        self.view.update_in(&mut self.window, |view, window, cx| {
                     view.expand_library_prompt(agent, prompt, 0..0, &input, window, cx)
                 });
    }

    fn composer(&mut self) -> String {
        self.window.run_until_parked();
        let input = self.input.clone();
        self.window
            .update(|_, cx| input.read(cx).value().to_string())
    }
}

/// The spec's "Clipboard text is inserted" scenario.
#[gpui_kit::test]
fn the_clipboard_text_is_inserted(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, "Explain: {{clipboard}}");
    fixture.copy(ClipboardItem::new_string("panic at main.rs:12".into()));

    fixture.choose();

    assert_eq!(fixture.composer(), "Explain: panic at main.rs:12");
}

/// "Clipboard holds an image": only text is used.
#[gpui_kit::test]
fn an_image_only_clipboard_expands_to_nothing(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, "Explain: {{clipboard}}");
    let image =
        gpui_kit::Image::from_bytes(gpui_kit::ImageFormat::Png, vec![0x89, b'P', b'N', b'G']);
    fixture.copy(ClipboardItem::new_image(&image));

    fixture.choose();

    assert_eq!(fixture.composer(), "Explain: ");
}

/// "Clipboard changes during expansion": the text captured when the prompt
/// was chosen is the one that lands.
#[gpui_kit::test]
fn a_copy_made_while_expanding_does_not_change_the_text(cx: &mut TestAppContext) {
    let mut fixture = window_with_prompt(cx, "Explain: {{clipboard}}");
    fixture.copy(ClipboardItem::new_string("one".into()));

    fixture.choose();
    fixture.copy(ClipboardItem::new_string("two".into()));

    assert_eq!(fixture.composer(), "Explain: one");
}
