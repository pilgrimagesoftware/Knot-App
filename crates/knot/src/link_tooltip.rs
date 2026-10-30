//! A link's URL beside the pointer while it rests on the link (#326), in
//! every Markdown surface: a panel response, the Markdown pane, anywhere a
//! `TextView` draws a link.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`, "A link shows where it
//! goes".
//!
//! The rendered text is gpui-kit's, and only it knows which link is under
//! the pointer. The `gpui-base` fork publishes that (`text::hovered_link`,
//! `docs/adr/0004-gpui-base-link-hover.md`); this plugin, mounted on every
//! window's `Root`, draws it. One plugin rather than a wrapper per surface,
//! so a new Markdown surface shows link URLs without doing anything.
//!
//! The label takes no input. It has no mouse handlers and so no hitbox,
//! which keeps it from taking the hover away from what is under it - the
//! response whose action bar the same pointer reveals (#313), or the link
//! itself, which must stay clickable through it.

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::anchored;
use gpui_kit::base::RootPlugin;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::deferred;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;

use crate::consts::{LINK_TOOLTIP_MAX_WIDTH, LINK_TOOLTIP_OFFSET, LINK_TOOLTIP_WINDOW_MARGIN};

/// Draws the URL of the link under the pointer in its window, or nothing.
pub(crate) struct LinkTooltip;

impl Render for LinkTooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(link) = gpui_kit::base::text::hovered_link(window, cx)
        else {
            return gpui_kit::Empty.into_any_element();
        };
        let theme = cx.theme();
        // Below and to the right of the pointer, where the cursor does not
        // cover it; `snap_to_window_with_margin` flips it inside the window
        // near an edge.
        let at = link.position + point(px(LINK_TOOLTIP_OFFSET), px(LINK_TOOLTIP_OFFSET));
        deferred(anchored().position(at)
                           .snap_to_window_with_margin(px(LINK_TOOLTIP_WINDOW_MARGIN))
                           .child(div().debug_selector(|| "link-url-tooltip".into())
                                       .max_w(px(LINK_TOOLTIP_MAX_WIDTH))
                                       .px_2()
                                       .py_1()
                                       .rounded_md()
                                       .border_1()
                                       .border_color(theme.border)
                                       .bg(theme.popover)
                                       .text_color(theme.popover_foreground)
                                       .text_xs()
                                       .shadow_md()
                                       .overflow_hidden()
                                       .whitespace_nowrap()
                                       .text_ellipsis()
                                       .child(link.url))).with_priority(1)
                                                         .into_any_element()
    }
}

impl RootPlugin for LinkTooltip {}

/// Shows link URLs in every window opened from here on. After
/// `gpui_kit::init`, like every root plugin.
pub(crate) fn install(cx: &mut App) {
    Root::register_plugin::<LinkTooltip>(cx, |_, _| LinkTooltip);
}

#[cfg(test)]
mod tests;
