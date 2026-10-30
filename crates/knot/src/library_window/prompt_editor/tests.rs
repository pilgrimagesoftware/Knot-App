//! The prompt editor's window (#322): it opens large enough to write a
//! prompt in, and the text area - not the rest of the form - takes the room
//! when the window is made bigger.

use gpui_kit::{TestAppContext, VisualTestContext, WeakEntity, px, size};
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
