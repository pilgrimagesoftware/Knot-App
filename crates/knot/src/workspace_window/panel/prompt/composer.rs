//! The composer entity: its state type, auto-grow bounds, send chord, and
//! the window-side cache and event wiring that builds one per agent.
//!
//! Attachments, queue editing and delivery live in the sibling modules.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::input::Editor;
use gpui_kit::component::input::EditorState;
use gpui_kit::component::input::InputEvent;
use uuid::Uuid;

use crate::composer_style::Palette;
use crate::workspace_window::WorkspaceWindow;

/// How far the prompt box grows with its content before it starts
/// scrolling, collapsed and expanded. It auto-grows rather than sitting at
/// a fixed height: a fixed height fights the textarea's own layout, so a
/// second line made it scroll and jump on every keystroke instead of
/// simply getting taller.
pub(crate) const PANEL_INPUT_ROWS_COLLAPSED: usize = 6;
pub(crate) const PANEL_INPUT_ROWS_EXPANDED: usize = 20;

/// The state type behind the panel composer.
///
/// Named once so the widget the composer is built on is a single
/// declaration rather than a type repeated across the window.
///
/// It is an `EditorState` and not a `TextareaState` for one reason:
/// styled ranges. Decorations are stored in `state.extras`, keyed off the
/// **mode marker**, and `TextareaMode`'s extras are `()`, whose default
/// `decoration_layers()` is empty - so a textarea cannot carry a
/// decoration, ever. What the composer is *not* is a code editor; see
/// [`new_panel_input`] for how that is arranged.
pub(crate) type PanelInputState = EditorState;

/// The widget that draws [`PanelInputState`].
pub(crate) type PanelInput = Editor;

/// How far `id`'s composer may grow, given whether it is expanded.
///
/// The bound is read in two places - when the entity is built and when the
/// expand control re-issues it - and they have to agree, so the choice
/// lives here rather than twice.
pub(crate) fn panel_input_max_rows(expanded: bool) -> usize {
    if expanded {
        PANEL_INPUT_ROWS_EXPANDED
    }
    else {
        PANEL_INPUT_ROWS_COLLAPSED
    }
}

/// Whether a `PressEnter` carrying `shift` is the send chord, given the
/// `agent_panel_shift_enter_sends` setting.
///
/// The widget reports *both* chords as `PressEnter` and distinguishes them
/// only by this flag - `submit_on_enter` changes which one also types a
/// newline, not which one reports - so this predicate is the whole of the
/// send-chord decision.
pub(crate) fn sends_on(shift_to_send: bool, shift: bool) -> bool {
    shift == shift_to_send
}

/// Whether a `PressEnter` carrying `shift` from `input` sends, per the
/// setting as it stands when the key arrives.
///
/// Read here, not captured when the composer is built: the composer and its
/// subscription outlive a settings change, and a captured copy kept sending
/// on the old chord while the hint named the new one (#429). The widget's
/// `submit_on_enter` is brought in line on the way, so its own newline
/// handling agrees with the chord that sends.
pub(crate) fn enter_sends(input: &Entity<PanelInputState>, shift: bool, cx: &mut App) -> bool {
    let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
    input.update(cx, |state, cx| {
             state.set_submit_on_enter(!shift_to_send, cx)
         });
    sends_on(shift_to_send, shift)
}

/// Builds the composer's state entity: the placeholder, the send chord
/// implied by `shift_to_send`, and the auto-grow bounds.
///
/// Separate from [`WorkspaceWindow::panel_prompt_input`], which owns the
/// caching and the event subscription, so the widget's own configuration
/// can be exercised in a test without a window's worth of state behind it.
pub(crate) fn new_panel_input(shift_to_send: bool, max_rows: usize, window: &mut Window,
                              cx: &mut App)
                              -> Entity<PanelInputState> {
    cx.new(|cx| {
          PanelInputState::new(window, cx).placeholder(WorkspaceWindow::panel_prompt_placeholder())
                                          // `submit_on_enter` is the
                                          // inverse of the setting: the
                                          // chord that does *not* send is
                                          // the one that inserts a newline.
                                          .submit_on_enter(!shift_to_send)
                                          // `EditorState::new` turns the
                                          // built-in search panel on. A
                                          // composer is written, not
                                          // searched, and off also lets
                                          // Cmd-F bubble to the window.
                                          .searchable(false)
                                          // `EditorState` counts as a code
                                          // editor whatever its layout mode
                                          // (`is_code_editor` reads the
                                          // mode marker, not `auto_grow`),
                                          // so it reserves three or more
                                          // empty rows below the last line
                                          // to scroll into. A composer that
                                          // fits its text has nothing to
                                          // scroll, and the wheel pushed the
                                          // whole draft out of view (#41).
                                          .scroll_beyond_last_line(Some(0))
                                          // Last, and load-bearing:
                                          // `EditorState::new` starts in
                                          // `LayoutMode::CodeEditor` and
                                          // this replaces the mode
                                          // outright. Line numbers, the
                                          // gutter, indent guides,
                                          // folding, auto-closing brackets
                                          // and smart indent are all
                                          // fields of that variant or
                                          // gated on `is_code_editor()`,
                                          // so leaving it is what makes
                                          // "the composer stays a
                                          // composer" true by
                                          // construction rather than by
                                          // turning flags off one at a
                                          // time.
                                          .auto_grow(1, max_rows)
      })
}

impl WorkspaceWindow {
    /// Toggles `id`'s input area between its default and expanded
    /// multi-line editing size.
    pub(in crate::workspace_window) fn toggle_panel_input_expanded(&mut self, id: Uuid,
                                                                   cx: &mut Context<Self>) {
        let expanded = if self.panel_input_expanded.remove(&id) {
            false
        }
        else {
            self.panel_input_expanded.insert(id);
            true
        };
        // The cap is part of the textarea's own layout mode, so expanding
        // has to update the live entity rather than just the render height.
        let max_rows = panel_input_max_rows(expanded);
        if let Some(input) = self.panel_prompt_inputs.get(&id).cloned() {
            cx.update_entity(&input, |state, cx| state.set_auto_grow(1, max_rows, cx));
        }
    }

    /// Gets or creates the prompt input entity for `id`'s panel, wired so
    /// `Enter`/`Shift+Enter` submits per `agent_panel_shift_enter_sends`
    /// as it stands at each keypress (the other chord always inserts a
    /// newline) - see
    /// `render_panel_input_area`'s Send button tooltip for the matching
    /// user-facing hint.
    pub(in crate::workspace_window) fn panel_prompt_input(&mut self, id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>)
                                                          -> Entity<PanelInputState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
        let max_rows = panel_input_max_rows(self.panel_input_expanded.contains(&id));
        let input = new_panel_input(shift_to_send, max_rows, window, cx);
        let palette = Palette::of(cx);
        let subscription =
            cx.subscribe_in(&input,
                            window,
                            move |view: &mut Self, input, event, window, cx| {
                                match event {
                                    InputEvent::PressEnter { shift, .. } => {
                                        if enter_sends(input, *shift, cx) {
                                            view.send_panel_prompt(id, window, cx);
                                        }
                                    }
                                    // Every way text arrives reports
                                    // here - typing, paste, undo, redo,
                                    // cut, a drag of text and the
                                    // lookup's own insertion - so one
                                    // arm restyles for all of them.
                                    InputEvent::Change => {
                                        let palette = Palette::of(cx);
                                        // The buffer decides: a chip
                                        // this edit removed detaches
                                        // its row, and the restyle
                                        // below then draws the table
                                        // that is left.
                                        if view.reconcile_panel_attachments(id, cx) {
                                            view.restyle_panel_attachments(id, palette, cx);
                                        }
                                        view.restyle_panel_composer(id, palette, cx);
                                        cx.notify();
                                    }
                                    // Focus leaving the input closes the
                                    // slash lookup, per its dismissal rules
                                    // - a popup left open behind another
                                    // pane is exactly what the shared
                                    // dismissal path exists to prevent.
                                    InputEvent::Blur => {
                                        let input = input.clone();
                                        view.dismiss_panel_lookup(id, &input, cx);
                                        cx.notify();
                                    }
                                    _ => {}
                                }
                            });
        self.panel_prompt_inputs.insert(id, input.clone());
        self.panel_prompt_input_subscriptions
            .insert(id, subscription);
        // After the entity is in the map, since the styling reads it back
        // out: a draft restored into a fresh composer is styled here, with
        // no edit to trigger it.
        self.ensure_panel_styling(id, palette, cx);
        input
    }

    /// The prompt textarea's placeholder - just the prompt, not the key
    /// chord (see `panel_prompt_send_hint` for that, rendered below the
    /// textarea instead of inside it).
    pub(in crate::workspace_window) fn panel_prompt_placeholder() -> &'static str {
        "Send a message…"
    }

    /// The send-chord hint shown below the prompt textarea, naming the
    /// active chord per `agent_panel_shift_enter_sends`.
    pub(in crate::workspace_window) fn panel_prompt_send_hint(shift_to_send: bool) -> &'static str {
        if shift_to_send {
            "Shift+Enter to send, Enter for a newline"
        }
        else {
            "Enter to send, Shift+Enter for a newline"
        }
    }
}
