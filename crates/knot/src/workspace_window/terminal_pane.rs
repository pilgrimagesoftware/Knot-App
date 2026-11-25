//! A shell agent's terminal surface: the `gpui_terminal::TerminalView` that
//! draws its session, and what the window does with the view's events.
//!
//! The view owns input, sizing and repaint; the window owns the agent store
//! and the pasteboard policy, so the two events that need either - a title, an
//! OSC 52 clipboard write - come back here.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, Focusable as _,
    IntoElement as _, ParentElement as _, Pixels, Size, Styled as _, Subscription, div, px,
};
use gpui_terminal::{PtyTransport, Terminal, TerminalEvent, TerminalStyle, TerminalView};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// One agent's view, and the subscription that routes its events.
///
/// Created with the session rather than on first draw: titles and clipboard
/// writes from an agent nobody is looking at still have to land, and the
/// view's pump is what delivers them.
pub(super) struct TerminalPane {
    pub(super) view: Entity<TerminalView<PtyTransport>>,
    /// The size the view was last drawn at, which a divider drag holds it at
    /// (`terminal_surface`). Written while the frame is prepainted, so it is
    /// a shared cell rather than window state.
    pub(super) size: Rc<Cell<Option<Size<Pixels>>>>,
    _events:         Subscription,
}

impl WorkspaceWindow {
    /// Builds `id`'s view over `terminal` and starts listening to it.
    pub(super) fn open_terminal_pane(&mut self, id: Uuid, terminal: Terminal<PtyTransport>,
                                     cx: &mut Context<Self>) {
        let style = self.terminal_style(cx);
        let view = cx.new(|cx| TerminalView::new(terminal, style, cx));
        let events = cx.subscribe(&view, move |window, _, event, cx| {
                           window.on_terminal_event(id, event, cx);
                       });
        self.terminal_panes.insert(id,
                                   TerminalPane { view,
                                                  size: Rc::new(Cell::new(None)),
                                                  _events: events });
    }

    fn on_terminal_event(&mut self, id: Uuid, event: &TerminalEvent, cx: &mut Context<Self>) {
        match event {
            TerminalEvent::Title(title) => {
                self.store.lock().set_terminal_title(id, title.clone());
                cx.notify();
            }
            TerminalEvent::ClipboardStore(text) => {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
            }
            TerminalEvent::TransportFailed(error) => {
                eprintln!("agent {id}'s terminal failed: {error}");
            }
            // Exit is already handled on the reader thread by
            // `ensure_session`'s hook, which queues the agent for removal; a
            // reset title and the bell have no surface in Knot yet.
            _ => {}
        }
    }

    /// Brings the selected agent's view up to the configured font. A no-op
    /// on the frames where nothing changed - `set_style` compares first - so
    /// it runs every frame, after `refresh_terminal_font`.
    pub(super) fn sync_terminal_style(&mut self, cx: &mut Context<Self>) {
        let Some(pane) = self.selected_agent
                             .and_then(|id| self.terminal_panes.get(&id))
        else {
            return;
        };
        let style = self.terminal_style(cx);
        pane.view
            .clone()
            .update(cx, |view, cx| view.set_style(style, cx));
    }

    /// `id`'s terminal as the pane shows it, or `None` while it has no view.
    ///
    /// While a divider drag holds `id` (`layout::DividerDrag::held_sizes`) the
    /// view sits in a box fixed at its pre-drag size, inside a clipping box
    /// that follows the pane. The view's fit sees the same bounds every frame
    /// and does not resize the PTY; releasing drops the box, and the next
    /// prepaint fits once (`workspace-layouts`, design decision 6).
    pub(super) fn terminal_surface(&self, id: Uuid) -> Option<AnyElement> {
        let pane = self.terminal_panes.get(&id)?;
        let measured = Rc::clone(&pane.size);
        let view = div().size_full()
                        .child(pane.view.clone())
                        .on_children_prepainted(move |bounds, _, _| {
                            measured.set(bounds.first().map(|bounds| bounds.size));
                        });
        let held = self.divider_drag
                       .as_ref()
                       .and_then(|drag| drag.held_sizes.get(&id));
        Some(match held {
                 Some(size) => {
                     div().size_full()
                          .overflow_hidden()
                          .child(div().w(size.width).h(size.height).flex_none().child(view))
                          .into_any_element()
                 }
                 None => view.into_any_element(),
             })
    }

    /// Focuses `id`'s terminal surface, if it has one.
    pub(super) fn focus_terminal(&self, id: Uuid, window: &mut gpui_kit::Window, cx: &mut App) {
        if let Some(pane) = self.terminal_panes.get(&id) {
            window.focus(&pane.view.focus_handle(cx), cx);
        }
    }

    fn terminal_style(&self, cx: &App) -> TerminalStyle {
        TerminalStyle::new(self.terminal_font_family(),
                           px(crate::settings_global::read(cx).terminal_font_size as f32))
    }
}
