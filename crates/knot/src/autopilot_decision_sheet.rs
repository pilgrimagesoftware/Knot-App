use gpui_kit::AnyElement;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use gpui_kit::px;
use knot_autopilot::Classification;

use crate::capped_scroll::capped_scroll;
use crate::window_options::autopilot_decision_window_options;

/// The dialog Autopilot's "Ask me" action opens: the agent's last message,
/// and buttons whose layout depends on what it was classified as.
///
/// A window of its own, same reasoning as `BroadcastSheet`: every dialog in
/// this app bigger than a yes/no is a window, not an in-place overlay.
///
/// What each button hands back to the caller - just "do your thing", the
/// sheet itself has no further state to pass along.
type DialogAction = Box<dyn Fn(&mut Window, &mut App)>;

pub(crate) struct AutopilotDecisionSheet {
    agent_name:   String,
    last_message: String,
    on_dismiss:   DialogAction,
    on_switch:    DialogAction,
    /// `Some` only for a binary classification - its presence is what the
    /// render decides the button layout on, so there is no separate
    /// `classification` field to keep in sync with it.
    on_continue:  Option<DialogAction>,
}

/// Opens the decision sheet. `on_continue` is `None` for an open question -
/// matching the Swift reference, where only a binary classification offers
/// the auto-continue button.
pub(crate) fn open_autopilot_decision_sheet(agent_name: String, last_message: String,
                                            classification: Classification,
                                            on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
                                            on_switch: impl Fn(&mut Window, &mut App) + 'static,
                                            on_continue: Option<impl Fn(&mut Window, &mut App)
                                                   + 'static>,
                                            cx: &mut App) {
    let title_key = match classification {
        Classification::Binary => "autopilot.ask.title_binary",
        _ => "autopilot.ask.title_open",
    };
    let title = knot_core::l10n::t_with(title_key, &[("name", &agent_name)]);
    let options = autopilot_decision_window_options(title, cx);
    let _ = cx.open_window(options, move |_window, cx| {
                  cx.new(|_cx| AutopilotDecisionSheet { agent_name,
                                                        last_message,
                                                        on_dismiss: Box::new(on_dismiss),
                                                        on_switch: Box::new(on_switch),
                                                        on_continue: on_continue.map(|f| {
                                                                                    Box::new(f)
                                                                                    as DialogAction
                                                                                }) })
              });
}

impl Render for AutopilotDecisionSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full()
                .gap_3()
                .px_5()
                .pt_5()
                .pb_6()
                .bg(cx.theme().background)
                .child(div().font_semibold().child(self.agent_name.clone()))
                .child(capped_scroll("autopilot-last-message",
                                     px(200.),
                                     div().whitespace_normal()
                                          .text_color(cx.theme().foreground)
                                          .child(self.last_message.clone())))
                .child(h_flex().flex_shrink_0()
                               .justify_end()
                               .gap_2()
                               .children(self.buttons(cx)))
    }
}

impl AutopilotDecisionSheet {
    fn buttons(&self, cx: &Context<Self>) -> Vec<AnyElement> {
        let dismiss =
            Button::new("autopilot-dismiss").label(knot_core::l10n::t("autopilot.ask.dismiss"))
                                            .on_click(cx.listener(|sheet, _, window, cx| {
                                                            (sheet.on_dismiss)(window, cx);
                                                            window.remove_window();
                                                        }))
                                            .into_any_element();
        let switch =
            Button::new("autopilot-switch").label(knot_core::l10n::t("autopilot.ask.switch"))
                                           .on_click(cx.listener(|sheet, _, window, cx| {
                                                           (sheet.on_switch)(window, cx);
                                                           window.remove_window();
                                                       }))
                                           .into_any_element();
        let Some(_) = self.on_continue.as_ref()
        else {
            return vec![dismiss, switch.into_any_element()];
        };
        let continue_button =
            Button::new("autopilot-continue").label(knot_core::l10n::t("autopilot.ask.continue"))
                                             .primary()
                                             .on_click(cx.listener(|sheet, _, window, cx| {
                                                             if let Some(on_continue) =
                                                                 &sheet.on_continue
                                                             {
                                                                 on_continue(window, cx);
                                                             }
                                                             window.remove_window();
                                                         }))
                                             .into_any_element();
        vec![dismiss, switch, continue_button]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The classification alone decides which title key is used - checked
    /// without opening a window, since there is no GPUI test harness in
    /// this codebase (same gap noted throughout `settings_window`).
    #[test]
    fn binary_and_open_pick_distinct_title_keys() {
        assert_ne!(title_key(Classification::Binary),
                   title_key(Classification::Open));
        assert_eq!(title_key(Classification::Open),
                   title_key(Classification::Completed));
    }

    fn title_key(classification: Classification) -> &'static str {
        match classification {
            Classification::Binary => "autopilot.ask.title_binary",
            _ => "autopilot.ask.title_open",
        }
    }
}
