//! The prompt editor's window (#322): it opens large enough to write a
//! prompt in, and the text area - not the rest of the form - takes the room
//! when the window is made bigger.

use gpui_kit::{Focusable, TestAppContext, VisualTestContext, WeakEntity, px, size};
use tempfile::TempDir;

use super::open_prompt_editor;
use crate::consts::{PROMPT_EDITOR_WINDOW_HEIGHT, PROMPT_EDITOR_WINDOW_WIDTH};

fn open_editor(cx: &mut TestAppContext) -> (VisualTestContext, TempDir) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          // No library window behind it: saving is not exercised here.
          open_prompt_editor(WeakEntity::new_invalid(), None, cx);
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
          open_prompt_editor(WeakEntity::new_invalid(), Some(prompt), cx);
      });
    let handle = cx.update(|cx| *cx.windows().last().expect("the prompt editor opened"));
    let mut window = VisualTestContext::from_window(handle, cx);
    window.run_until_parked();

    assert_eq!(focus(&mut window), (false, true));
}
