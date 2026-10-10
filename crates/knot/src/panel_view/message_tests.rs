//! Message timestamp visibility (cmd-hold-timestamps): a prompt's or
//! response's timestamp paints only while ⌘ is held, per `acp-panel-ui`'s
//! "Message timestamps are shown only while ⌘ is held" requirement.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Hsla;
use gpui_kit::IntoElement;
use gpui_kit::ListAlignment;
use gpui_kit::ListState;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use gpui_kit::px;

use super::LIST_OVERDRAW;
use super::callbacks::PanelCallbacks;
use super::message::Message;
use super::message::render_message;
use super::style::PanelStyle;
use super::style::RiskLevel;
use crate::panel_state::PanelState;

fn panel_style() -> PanelStyle {
    PanelStyle { permission_risk:    RiskLevel::Neutral,
                 markdown_font_size: px(14.),
                 mono_font_family:   "Menlo".into(),
                 ui_font_family:     "Helvetica".into(),
                 title_font_family:  "Helvetica".into(),
                 danger_color:       Hsla::default(),
                 info_color:         Hsla::default(),
                 border_color:       Hsla::default(),
                 card_color:         Hsla::default(),
                 prompt_color:       Hsla::default(),
                 prompt_foreground:  Hsla::default(),
                 compact_tool_calls: false, }
}

/// A view whose whole body is one rendered message, so a test can check
/// whether its timestamp painted.
struct MessageProbe {
    state:    PanelState,
    list:     ListState,
    cmd_held: bool,
}

impl Render for MessageProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let style = panel_style();
        let message = self.state.messages[0].clone();
        div().size_full()
             .child(render_message(Message { state:    &self.state,
                                             index:    0,
                                             is_last:  false,
                                             style:    &style,
                                             list:     &self.list,
                                             cmd_held: self.cmd_held, },
                                   &message,
                                   &PanelCallbacks::new(|_| {}, || {}, |_| {}, |_| {}, || {})))
    }
}

/// Draws one user message with `cmd_held` and returns whether its timestamp
/// painted - the scenario in `acp-panel-ui`'s "Message timestamps are shown
/// only while ⌘ is held" requirement.
fn message_timestamp_painted(cmd_held: bool, cx: &mut TestAppContext) -> bool {
    let mut state = PanelState::new();
    state.push_user_message("hi".to_owned());
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let list = ListState::new(1, ListAlignment::Top, px(LIST_OVERDRAW));
                             let view = cx.new(|_| MessageProbe { state,
                                                                  list,
                                                                  cmd_held });
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.debug_bounds("panel-message-timestamp").is_some()
}

#[gpui_kit::test]
fn no_timestamp_paints_while_cmd_is_not_held(cx: &mut TestAppContext) {
    assert!(!message_timestamp_painted(false, cx));
}

#[gpui_kit::test]
fn a_timestamp_paints_while_cmd_is_held(cx: &mut TestAppContext) {
    assert!(message_timestamp_painted(true, cx));
}
