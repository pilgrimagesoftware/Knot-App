//! The window's side of a multi-pane layout: the dividers laid over the
//! content area and the drag that moves them (`workspace-layouts`,
//! "Dividers").
//!
//! The geometry is `render::layout`'s and the single divider element is
//! `render::divider`'s; what lives here is the state a drag needs between
//! mouse events, which only the window can hold.
//!
//! A drag sets the clamped ratio in the store on every move, so the next
//! frame lays out from it, and writes nothing to disk. The release writes
//! the UI-state document once: a drag is dozens of moves, and
//! `persist_workspace_ui` is the write the bounds observer's doc comment
//! calls out as too hot to run per frame.
//!
//! Not here: drawing panes (task 4.1), and holding terminals at their
//! pre-drag size while a drag runs (task 6, `held_sizes`).

use std::collections::BTreeMap;

use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::DispatchPhase;
use gpui_kit::IntoElement;
use gpui_kit::MouseButton;
use gpui_kit::MouseMoveEvent;
use gpui_kit::MouseUpEvent;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::Size;
use gpui_kit::Styled;
use gpui_kit::canvas;
use gpui_kit::div;
use knot_core::LayoutMode;
use uuid::Uuid;

use super::WorkspaceWindow;
use super::render::divider::divider_element;
use super::render::layout::Divider;
use super::render::layout::DividerId;
use super::render::layout::Orientation;
use super::render::layout::divider_rects;
use super::render::layout::dragged_ratio;
use crate::consts;

/// A divider drag in progress: which divider, where its ratio and the
/// pointer were when it began, and the sizes terminals are held at.
pub(super) struct DividerDrag {
    pub(super) divider:     Divider,
    pub(super) start_ratio: f64,
    pub(super) start_mouse: Point<Pixels>,
    /// Each terminal pane's size when the drag began. Empty until task 6
    /// holds terminals at it. Keyed by agent, so `teardown_session` prunes
    /// it.
    pub(super) held_sizes:  BTreeMap<Uuid, Size<Pixels>>,
}

impl WorkspaceWindow {
    /// The mode and both ratios this workspace's layout is in, from the
    /// store. The secondary ratio falls back to the centre when the document
    /// carries none.
    fn layout_state(&self) -> (LayoutMode, f64, f64) {
        let ui = self.store.lock().workspace_ui(self.workspace_id);
        (ui.layout_mode,
         ui.split_ratio,
         ui.split_ratio_secondary
           .unwrap_or(consts::PANE_SPLIT_RATIO_DEFAULT))
    }

    /// `pane`, with the layout's dividers laid over it.
    ///
    /// In single mode - and so for every window that has never been split -
    /// this returns `pane` untouched, which is what keeps single-agent
    /// rendering as it was. Otherwise the pane sits in a `relative()` box
    /// with an overlay that measures the box and places each divider from
    /// `divider_rects`; the overlay draws nothing but the dividers, and
    /// takes no events outside them.
    pub(super) fn with_layout_dividers(&self, pane: AnyElement, cx: &Context<Self>) -> AnyElement {
        let (mode, r, s) = self.layout_state();
        if mode == LayoutMode::Single {
            return pane;
        }
        let view = cx.entity().downgrade();
        let placed = view.clone();
        let overlay =
            canvas(move |bounds, window, cx| {
                       let elements: Vec<AnyElement> =
                                     divider_rects(mode, r, s, bounds.size).into_iter()
                                                                          .map(|rect| {
                                         let mut element =
                                             divider_element(rect, placed.clone(), cx);
                                         element.prepaint_as_root(bounds.origin + rect.hit.origin,
                                                                  rect.hit.size.into(),
                                                                  window,
                                                                  cx);
                                         element
                                     })
                                                                          .collect();
                       (bounds, elements)
                   },
                   move |_, (bounds, mut elements), window, cx| {
                       // The drag's move and release are heard here rather than
                       // on a divider: the pointer
                       // leaves a 12 px strip on the first frame.
                       // Capture phase, so a pane that handles the same event
                       // cannot hide it from the drag.
                       let on_move = view.clone();
                       window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                                 if phase == DispatchPhase::Capture {
                                     let _ = on_move.update(cx, |view, cx| {
                                                        view.divider_mouse_moved(event,
                                                                                 bounds.size,
                                                                                 cx);
                                                    });
                                 }
                             });
                       let on_up = view.clone();
                       window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                                 if phase == DispatchPhase::Capture
                                    && event.button == MouseButton::Left
                                 {
                                     let _ = on_up.update(cx, |view, cx| view.end_divider_drag(cx));
                                 }
                             });
                       for element in &mut elements {
                           element.paint(window, cx);
                       }
                   });
        div().size_full()
             .relative()
             .child(pane)
             // Pinned to the corner: an absolute box without insets sits at
             // its static position, which is below the pane.
             .child(overlay.absolute().top_0().left_0().size_full())
             .into_any_element()
    }

    /// Starts dragging `divider`, remembering its ratio and where the
    /// pointer took hold of it.
    pub(super) fn begin_divider_drag(&mut self, divider: Divider, mouse: Point<Pixels>,
                                     _cx: &mut Context<Self>) {
        let (_, primary, secondary) = self.layout_state();
        let start_ratio = match divider.id {
            DividerId::Primary => primary,
            DividerId::Secondary => secondary,
        };
        self.divider_drag = Some(DividerDrag { divider,
                                               start_ratio,
                                               start_mouse: mouse,
                                               held_sizes: BTreeMap::new() });
    }

    /// A pointer move while the window is open. Sets the dragged ratio, live,
    /// when a drag is running; ends the drag when the button is no longer
    /// down, which is how a release that never reached the window shows up.
    pub(super) fn divider_mouse_moved(&mut self, event: &MouseMoveEvent, area: Size<Pixels>,
                                      cx: &mut Context<Self>) {
        let Some(drag) = self.divider_drag.as_ref()
        else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.end_divider_drag(cx);
            return;
        }
        let (travel, extent) = match drag.divider.orientation {
            Orientation::Vertical => (event.position.x - drag.start_mouse.x, area.width),
            Orientation::Horizontal => (event.position.y - drag.start_mouse.y, area.height),
        };
        let ratio = dragged_ratio(drag.start_ratio, travel, extent);
        let id = drag.divider.id;
        self.set_divider_ratio(id, ratio);
        cx.notify();
    }

    /// Ends the drag, if there is one, and writes the UI-state document
    /// once. A release with no drag running writes nothing.
    pub(super) fn end_divider_drag(&mut self, cx: &mut Context<Self>) {
        if self.divider_drag.take().is_none() {
            return;
        }
        self.persist_workspace_ui(cx);
        cx.notify();
    }

    /// Writes `ratio` to the store's copy of the layout, leaving the other
    /// ratio as it was.
    fn set_divider_ratio(&mut self, id: DividerId, ratio: f64) {
        let mut store = self.store.lock();
        let ui = store.workspace_ui(self.workspace_id);
        let (primary, secondary) = match id {
            DividerId::Primary => (ratio, ui.split_ratio_secondary),
            DividerId::Secondary => (ui.split_ratio, Some(ratio)),
        };
        store.set_workspace_split_ratios(self.workspace_id, primary, secondary);
    }
}
