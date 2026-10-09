//! The window's single pane: no tab strip, since one window shows one
//! library. The pane manages its own scroll - same `overflow_hidden` body the
//! settings window gave Personas, Prompts and Bench before the move.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;

use super::library::Library;
use super::window::LibraryWindow;

impl Render for LibraryWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.library {
            Library::Personas => self.render_personas(cx).into_any_element(),
            Library::Prompts => self.render_prompts(cx).into_any_element(),
            Library::Bench => self.render_bench(cx).into_any_element(),
        };
        let body = div().id("library-body")
                        .flex_1()
                        .min_h_0()
                        .overflow_hidden()
                        .child(body);

        v_flex().size_full()
                .gap_3()
                .p_4()
                .bg(cx.theme().background)
                .child(body)
    }
}
