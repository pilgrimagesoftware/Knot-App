//! An agent's `set-status` text reaches the screen without riding on
//! another repaint (#17): the MCP server's thread writes it with no context
//! to notify from, so the repaint poll has to notice it.

use gpui_kit::TestAppContext;

use super::repaint::revised;
use super::shortcuts_tests::window_with_agents;

/// The reason for a revision rather than a flag: two windows over the one
/// store each see the change. A flag the first cleared would hide it from
/// the second.
#[test]
fn every_window_sees_a_change_once() {
    let (mut first, mut second) = (0, 0);

    assert!(revised(&mut first, 1));
    assert!(revised(&mut second, 1),
            "the second window still has it to draw");
    assert!(!revised(&mut first, 1), "and each draws it once");
    assert!(!revised(&mut second, 1));
}

/// Written straight to the store, as `set-status` does from the MCP thread,
/// for an agent nothing else is repainting.
#[gpui_kit::test]
fn status_text_written_off_the_ui_thread_is_noticed(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.store_revised();
                    view.store
                        .lock()
                        .set_status_text(id, "Reviewing #17".to_string());

                    assert!(view.store_revised(), "the new text has to be drawn");
                    assert!(!view.store_revised(), "once");

                    view.store
                        .lock()
                        .set_status_text(id, "Reviewing #17".to_string());
                    assert!(!view.store_revised(), "the same text again needs no frame");
                });
}
