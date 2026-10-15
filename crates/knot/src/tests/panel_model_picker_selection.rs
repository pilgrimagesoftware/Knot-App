//! Opening the model dropdown must not start a text selection in the
//! conversation beside it (issue #243).
//!
//! The window-level text selection begins on any left mouse-down that no
//! control claims, and ends on the matching mouse-up. The kit's `Select`
//! opens on click - the mouse-up - and stops that event's propagation, so a
//! selection begun by the press was never ended: every later pointer move
//! extended it across the conversation, with no button held.
//!
//! These draw the dropdown through `config_select`, the builder the panel
//! draws it with, beside a selectable `TextView`, in a real window.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::DismissEvent;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::base::TextSelection;
use gpui_kit::component::Root;
use gpui_kit::component::select::SelectState;
use gpui_kit::component::text::TextView;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;

use crate::workspace_window::panel::input::ConfigSelectorDelegate;
use crate::workspace_window::panel::input::config_select;
use crate::workspace_window::panel::input::new_config_select_state;

/// Where the conversation text is drawn: the top band of the window.
const TEXT_Y: f32 = 15.;

/// Where the dropdown's trigger is drawn: the band below the text.
const TRIGGER_Y: f32 = 55.;

/// A conversation line above a model dropdown.
struct ConversationAndPicker {
    picker: Entity<SelectState<ConfigSelectorDelegate>>,
}

impl Render for ConversationAndPicker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .child(div().h(px(40.)).child(TextView::markdown(
            "conversation",
            "The agent's reply, selectable as \
                                                    conversation text is.",
        )))
             .child(div().h(px(30.))
                         .w(px(200.))
                         .child(config_select(&self.picker, "model-picker")))
    }
}

fn model_option() -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { id:            "model".to_string(),
                             name:          "Model".to_string(),
                             category:      Some("model".to_string()),
                             kind:          "select".to_string(),
                             current_value: serde_json::Value::String("opus".to_string()),
                             options:
                                 ["Opus", "Sonnet", "Haiku"].iter()
                                                            .map(|name| {
                                                                knot_acp::ConfigOptionValue {
                                                         value:       name.to_lowercase(),
                                                         name:        (*name).to_string(),
                                                         description: None,
                                                     }
                                                            })
                                                            .collect(), }
}

/// The window, drawn once, and whether the dropdown has been dismissed - a
/// dismissal is only emitted by a menu that was open.
fn setup(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Cell<bool>>) {
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::default(), cx);
      });
    let dismissed = Rc::new(Cell::new(false));
    let on_dismiss = Rc::clone(&dismissed);
    let (_, cx) = cx.add_window_view(|window, cx| {
                        let picker = new_config_select_state(&model_option(), window, cx);
                        cx.subscribe(&picker, move |_, _, _: &DismissEvent, _| {
                              on_dismiss.set(true);
                          })
                          .detach();
                        let view = cx.new(|_| ConversationAndPicker { picker });
                        Root::new(view, window, cx)
                    });
    cx.run_until_parked();
    cx.update(|window, cx| {
          let _ = window.draw(cx);
      });
    (cx, dismissed)
}

/// The press that opens the dropdown is the dropdown's, not the start of a
/// selection: moving the pointer over the conversation afterwards - the
/// "dragging around the menu" in the report - selects nothing.
#[gpui_kit::test]
fn opening_the_model_dropdown_does_not_select_conversation_text(cx: &mut TestAppContext) {
    let (cx, dismissed) = setup(cx);
    let trigger = point(px(20.), px(TRIGGER_Y));

    cx.simulate_mouse_down(trigger, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(trigger, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    cx.update(|window, cx| {
          let _ = window.draw(cx);
      });
    cx.simulate_mouse_move(point(px(300.), px(TEXT_Y)), None, Modifiers::default());
    cx.simulate_mouse_move(point(px(2.), px(TEXT_Y)), None, Modifiers::default());
    cx.run_until_parked();

    let selected = cx.update(TextSelection::selected_text);
    assert!(selected.is_empty(),
            "opening the dropdown left a selection following the pointer: {selected:?}");

    // The click above really did open the dropdown: only an open menu
    // reports a dismissal when Escape closes it.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(dismissed.get(),
            "the click at the trigger never opened the dropdown");
}
