//! The prompt editor's window (#322): it opens large enough to write a
//! prompt in, and the text area - not the rest of the form - takes the room
//! when the window is made bigger.

use gpui_kit::{Focusable, TestAppContext, VisualTestContext, px, size};
use tempfile::TempDir;

use super::open_prompt_editor;
use crate::consts::{PROMPT_EDITOR_WINDOW_HEIGHT, PROMPT_EDITOR_WINDOW_WIDTH};

fn open_editor(cx: &mut TestAppContext) -> (VisualTestContext, TempDir) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          // No library window behind it: saving is not exercised here.
          open_prompt_editor(None, cx);
      });
    let handle = cx.update(|cx| *cx.windows().last().expect("the prompt editor opened"));
    let window = VisualTestContext::from_window(handle, cx);
    window.run_until_parked();
    (window, dir)
}

#[gpui_kit::test]
fn the_prompt_editor_opens_at_its_own_larger_size(cx: &mut TestAppContext) {
    let (mut window, _dir) = open_editor(cx);

    let viewport = window.update(|window, _| window.viewport_size());
    assert_eq!(viewport,
               size(px(PROMPT_EDITOR_WINDOW_WIDTH),
                    px(PROMPT_EDITOR_WINDOW_HEIGHT)));
    let text = window.debug_bounds("prompt-editor-text")
                     .expect("the text area is drawn");
    assert!(text.size.height > px(PROMPT_EDITOR_WINDOW_HEIGHT / 2.),
            "the text area takes most of the window: {text:?}");
}

/// Resizing gives the extra room to the text area, in both directions.
#[gpui_kit::test]
fn the_text_area_grows_with_the_window(cx: &mut TestAppContext) {
    let (mut window, _dir) = open_editor(cx);
    let before = window.debug_bounds("prompt-editor-text")
                       .expect("the text area is drawn");

    window.simulate_resize(size(px(PROMPT_EDITOR_WINDOW_WIDTH + 300.),
                                px(PROMPT_EDITOR_WINDOW_HEIGHT + 240.)));
    window.run_until_parked();
    let after = window.debug_bounds("prompt-editor-text")
                      .expect("the text area is drawn");

    assert_eq!(after.size.height - before.size.height, px(240.));
    assert_eq!(after.size.width - before.size.width, px(300.));
}

/// Which of the editor's two fields holds focus, as `(name, text)`.
fn focus(window: &mut VisualTestContext) -> (bool, bool) {
    window.update(|window, cx| {
              let root = window.root::<gpui_kit::component::Root>()
                               .flatten()
                               .expect("the editor's root");
              let editor = root.read(cx)
                               .view()
                               .clone()
                               .downcast::<super::PromptEditor>()
                               .expect("the root holds the editor");
              let editor = editor.read(cx);
              (editor.name_input.focus_handle(cx).is_focused(window),
               editor.text_input.focus_handle(cx).is_focused(window))
          })
}

/// A new prompt starts at its name, the first thing it needs.
#[gpui_kit::test]
fn a_new_prompt_opens_with_the_caret_in_its_name(cx: &mut TestAppContext) {
    let (mut window, _dir) = open_editor(cx);
    assert_eq!(focus(&mut window), (true, false));
}

/// An existing prompt opens with the caret in its text, which is what an
/// edit is for (#321).
#[gpui_kit::test]
fn an_existing_prompt_opens_with_the_caret_in_its_text(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let prompt = knot_core::Prompt::new("Review", "review it").expect("a valid prompt");
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          open_prompt_editor(Some(prompt), cx);
      });
    let handle = cx.update(|cx| *cx.windows().last().expect("the prompt editor opened"));
    let mut window = VisualTestContext::from_window(handle, cx);
    window.run_until_parked();

    assert_eq!(focus(&mut window), (false, true));
}

/// Saving redraws an open Prompts window, which draws from the settings
/// just written. The editor no longer holds that window: it also opens from
/// the composer's Insert Prompt list, where there is none to hand it
/// (#341), so it finds the window when it saves.
#[gpui_kit::test]
fn saving_redraws_the_open_prompts_window(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::window_registry::WindowRegistry::install(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          let store = std::sync::Arc::new(parking_lot::Mutex::new(knot_agents::AgentStore::new()));
          crate::library_window::open_library_window(crate::library_window::Library::Prompts,
                                                     store,
                                                     cx);
          open_prompt_editor(None, cx);
      });
    let (library, editor) = cx.update(|cx| {
                                  let windows = cx.windows();
                                  (*windows.first().expect("the Prompts window"),
                                   *windows.last().expect("the prompt editor"))
                              });
    let mut library = VisualTestContext::from_window(library, cx);
    library.run_until_parked();
    assert!(library.debug_bounds("prompt-row-0").is_none(),
            "the library starts empty");

    let mut editor = VisualTestContext::from_window(editor, cx);
    editor.update(|window, cx| {
              let root = window.root::<gpui_kit::component::Root>()
                               .flatten()
                               .expect("the editor's root");
              let view = root.read(cx)
                             .view()
                             .clone()
                             .downcast::<super::PromptEditor>()
                             .expect("the root holds the editor");
              view.update(cx, |view, cx| {
                      view.name_input
                          .update(cx, |state, cx| state.set_value("Review", window, cx));
                      view.text_input
                          .update(cx, |state, cx| state.set_value("review it", window, cx));
                      view.save(window, cx);
                  });
          });
    library.run_until_parked();

    assert!(library.debug_bounds("prompt-row-0").is_some(),
            "the saved prompt is drawn in the Prompts window");
}
