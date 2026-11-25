use std::sync::Arc;

use gpui_kit::App;
use parking_lot::Mutex;
use uuid::Uuid;

mod assets;
mod visual_identity;

#[cfg(test)]
mod tests;

pub(crate) use assets::*;
pub(crate) use visual_identity::*;

/// The automatic nudge Knot sends an agent with unread mail.
///
/// The closing sentence is load-bearing: the nudge can land behind work the
/// agent had already started, and without it the agent reads the prompt as
/// a new task and abandons what it was doing. `mcp-messaging`'s "Inbox
/// nudges preserve interrupted session work" requires it.
pub(crate) const CHECK_INBOX_PROMPT: &str = "Check your inbox for questions or instructions from other agents. Update your status and \
     immediately execute what is being asked without confirmation. If there is nothing to do, \
     continue your previous work.";

pub(crate) type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// The awaiting-input queue, reachable from any window.
///
/// The MCP tool catalog pushes an entry whenever an agent's hook or ACP
/// session reports Awaiting input; the workspace window that owns the agent
/// drains it and raises the desktop notification
/// `openspec/specs/desktop-notifications/spec.md` asks for. A global rather
/// than a constructor argument because every workspace window needs it and
/// `WorkspaceWindow::open` is already at the crate's argument ceiling.
#[derive(Clone)]
pub(crate) struct AwaitingInput(pub(crate) AwaitingInputQueue);

impl gpui_kit::Global for AwaitingInput {}

pub(crate) type ActivationQueue = Arc<Mutex<Vec<Uuid>>>;

/// The activation queue, reachable from any window.
///
/// The MCP tool catalog pushes a recipient's id whenever a direct
/// `send-message` was delivered to an agent that is not activated; the
/// workspace window that owns the agent drains it and starts the session,
/// as `mcp-messaging`'s "Direct send activates a deactivated recipient"
/// requires. A global for the same reason [`AwaitingInput`] is one: every
/// workspace window needs it, and the tool layer runs off the main thread
/// with no window to call.
///
/// Ids belonging to another workspace stay in the queue rather than being
/// dropped, so the window that does own them can still pick them up - the
/// draining window is whichever repaints first, not necessarily the right
/// one.
#[derive(Clone)]
pub(crate) struct Activation(pub(crate) ActivationQueue);

impl gpui_kit::Global for Activation {}

/// The autopilot effect queue, reachable from any window.
///
/// The MCP tool catalog pushes `(agent id, effect)` whenever Autopilot
/// classification lands on something to do - see
/// `knot_mcp_tools::AutopilotEffect`. The workspace window that owns the
/// agent drains it in `workspace_window::autopilot` and carries out the
/// effect: marking + notifying, opening the decision dialog, or injecting
/// text. A global for the same reason [`AwaitingInput`] is one: the tool
/// layer runs off the main thread, with no window to call directly.
#[derive(Clone)]
pub(crate) struct Autopilot(pub(crate) knot_mcp_tools::AutopilotQueue);

impl gpui_kit::Global for Autopilot {}

/// The three cross-thread queues `start_mcp_server` hands the tool catalog,
/// bundled into one value so that function stays within the repo's "keep
/// functions to <= 5-6 args" convention (`CLAUDE.md`) as queues are added.
#[derive(Clone)]
pub(crate) struct McpQueues {
    pub(crate) awaiting_input: AwaitingInputQueue,
    pub(crate) activation:     ActivationQueue,
    pub(crate) autopilot:      knot_mcp_tools::AutopilotQueue,
}

/// Which setting a font panel session is editing. Plain data, referenced
/// from platform-independent UI code (button labels/handlers); only the
/// panel-driving logic that reads/writes it is macOS-only, in
/// `native_font_panel` below.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FontPanelTarget {
    Ui,
    Title,
    Terminal,
}

/// Drives the real macOS font panel (`NSFontPanel`) for the font pickers,
/// since the user wants the system chooser rather than an in-app dropdown.
///
/// The panel has no OK button by design: it reports a choice by sending the
/// font manager's action (`changeFont:`) to a receiver, which is expected to
/// send `convertFont:` straight back. Only after that round trip does the
/// manager record a new selected font. GPUI installs nothing in the responder
/// chain that answers `changeFont:`, so with no target set the action was
/// dropped and `NSFontManager.selectedFont` stayed at whatever [`open`] had
/// set - which is why polling that property observed every choice as "no
/// change". [`FontPanelReceiver`] is that missing receiver; the poll
/// ([`poll_selection`]) now just drains what it recorded, so the choice still
/// reaches GPUI on its own frame rather than inside an AppKit callback.
#[cfg(target_os = "macos")]
pub(crate) mod native_font_panel;

/// Opens the OS emoji/character picker (the same panel as Edit > Emoji &
/// Symbols) so the user can pick an agent avatar from the full system
/// catalog instead of a curated list. AppKit inserts the chosen character
/// straight into whatever text field currently has keyboard focus, so the
/// caller must focus its input first - no callback or polling needed.
#[cfg(target_os = "macos")]
pub(crate) mod native_character_picker;

/// Replaces a `$HOME` prefix with `~` - matches the Swift reference's
/// `AgentTerminalView.shortenPath`.
pub(crate) fn shorten_path(path: &str) -> String {
    std::env::var("HOME").ok()
                         .and_then(|home| path.strip_prefix(&home).map(|rest| format!("~{rest}")))
                         .unwrap_or_else(|| path.to_string())
}

/// Joins text onto one line: every run of whitespace - line breaks, tabs,
/// runs of spaces - collapses to a single space, and both ends are trimmed.
///
/// Call this wherever a render declares a single line. The style flags do
/// not achieve it on their own: `gpui::text_system::shape_text` splits its
/// input on `\n` and shapes each piece as its own line before any wrapping
/// or truncation decision, and `whitespace_nowrap` only disables *soft*
/// wrapping. An element styled `whitespace_nowrap().text_ellipsis()`
/// therefore still renders one line per line break, truncating each of them
/// separately. Flattening the text first is what makes the declaration true;
/// the width-based ellipsis then works as intended.
///
/// Presentational only. Flatten at the point of render, never on the way in,
/// so what is stored, delivered to an agent, or drawn somewhere that permits
/// several lines keeps the text as it was written.
pub(crate) fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Names the running process, so macOS shows "Knot" in the application
/// menu rather than the executable's own lowercase name.
///
/// AppKit ignores the title given to the first `Menu` and labels the
/// application menu from the process name, which for an unbundled binary
/// is `argv[0]` - hence `knot`. `CFBundleName` would cover it once the
/// Rust app ships as a real `.app`; until then this sets the same thing at
/// runtime. Must run before the menu bar is built.
#[cfg(target_os = "macos")]
pub(crate) fn set_process_name(name: &str) {
    use objc2_foundation::{NSProcessInfo, NSString};

    let info = NSProcessInfo::processInfo();
    let name = NSString::from_str(name);
    // `setProcessName:` is declared on NSProcessInfo but not exposed by
    // objc2-foundation's generated bindings, so it goes through a raw
    // message send.
    unsafe {
        let _: () = objc2::msg_send![&*info, setProcessName: &*name];
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn set_process_name(_name: &str) {}

/// Every agent's subagents, reachable from any window.
///
/// A global for the same reason [`AwaitingInput`] is one: every workspace
/// window needs it, `WorkspaceWindow::open` is already at the crate's
/// argument ceiling, and the two writers - each panel session's drain task
/// and the MCP hook route - run off the main thread with no window to be
/// handed one.
///
/// One registry for the process, not one per window. A subagent belongs to
/// an agent, and an agent belongs to exactly one workspace window, so the
/// keying already separates them; a registry per window would have to be
/// found by the hook route from an `agent_id` alone, which is the lookup the
/// shared map makes unnecessary.
#[derive(Clone)]
pub(crate) struct Subagents(pub(crate) SubagentRegistryHandle);

impl gpui_kit::Global for Subagents {}

impl Subagents {
    /// The process's registry, installing an empty one if nothing has.
    ///
    /// The install branch is the test path. `app_bootstrap` sets the global
    /// explicitly, before any window opens and from the same `Arc` the MCP
    /// tool catalog was built with, so in a running app this only ever
    /// reads. A test that opens a window without going through `run` gets an
    /// empty registry of its own rather than a panic - the same bargain
    /// `WindowRegistry::install` makes, and the reason both are idempotent.
    pub(crate) fn handle(cx: &mut App) -> SubagentRegistryHandle {
        if !cx.has_global::<Self>() {
            cx.set_global(Self(SubagentRegistryHandle::default()));
        }

        cx.global::<Self>().0.clone()
    }
}

pub(crate) type SubagentRegistryHandle = Arc<Mutex<knot_subagents::registry::SubagentRegistry>>;
