//! The application's named actions - menu items and key bindings, wired or
//! not yet - and the app-wide handlers for the ones that need no window:
//! quit, hide, show all, Knot Help, and quitting on a terminal signal.

use gpui_kit::App;
use gpui_kit::actions;

use crate::quit_guard;

actions!(knot_app,
         [Quit,
          HideApp,
          HideOthers,
          ShowAllWindows,
          AboutKnot,
          ReportIssue,
          OpenSettings,
          OpenImport,
          PanelPermissionAllow,
          PanelPermissionAllowAlways,
          PanelPermissionDeny,
          PanelOpenPermissionSelector,
          PanelFind]);

// View > Collapse Tool Calls (#488): every tool call in the selected agent's
// conversation back to its collapsed form. Fixed rather than configurable,
// beside the panel's other fixed keys; enabled by the workspace window while
// a conversation is shown, like Jump to Bottom.
actions!(knot_app, [CollapseToolCalls]);

// UNWIRED: the standard-menu items the port has not implemented yet. They
// exist as named actions rather than `NoAction` only so each can carry its
// own key equivalent - a menu item's shortcut is looked up by action, so
// items sharing `NoAction` would all have to show the same one, and binding
// a key to `NoAction` *unbinds* that key everywhere (`Keymap::add_bindings`
// treats it as a disabling binding).
//
// Nothing registers a handler for any of these, so `is_action_available`
// answers false and AppKit draws the items disabled with their shortcut
// greyed beside them, which is how macOS presents a standard item an app
// does not currently offer. Wiring one is a matter of registering its
// handler on the window that owns the behavior; the binding is already
// here.
actions!(knot_app, [NewWorkspace]);

// Help > Knot Help (⌘?). Opens Knot's page in the browser; wired app-wide in
// `install_actions_and_keys`, so it is enabled whatever is focused.
actions!(knot_app, [KnotHelp]);

// File > Close Window and Window > Minimize / Zoom. Wired app-wide in
// `window_actions`, so they act on whichever window is focused. Zoom is named
// only so it can have a handler; macOS gives it no key equivalent, and
// nothing binds one.
actions!(knot_app, [CloseWindow, MinimizeWindow, ZoomWindow]);

// Enter Full Screen is deliberately absent. macOS adds its own item to the
// View menu when it does not find an equivalent one, and it judges
// equivalence by the action behind the item rather than by its label - so a
// Knot item it does not recognize was added beside rather than instead, and
// the menu showed the same command twice under the same key. cmd-ctrl-f is
// also a key the platform reserves, and `app-menu` forbids a Knot item from
// holding one: AppKit claims a menu key equivalent ahead of the window, so
// such an item takes the key rather than sharing it. The same rule moved Fork
// Agent off cmd-f.

// File > New Agent… (⌘T, the Swift reference's key). Wired, unlike the items
// above, but only inside a workspace window: its handler is registered on the
// window's root element (`WorkspaceWindow::with_shortcut_actions`), so the
// item is disabled wherever there is no workspace to add an agent to.
actions!(knot_app, [NewAgent]);
// Edit > Find File… (#5): the file finder over the selected agent's folder.
actions!(knot_app, [FindFile]);

// The Window menu's openers. Unlike the items above these are wired, and
// enabled at all times: they are how a user gets back to a window, so an
// enablement rule that depended on a window being focused would disable them
// exactly when they are needed.
actions!(knot_app, [OpenCommandCenter, OpenWorkspaces]);

// The Window menu's library openers (#20): Personas, Prompts and Bench each
// moved out of the settings window into their own single-instance window,
// reached the same way Command Center and Workspaces are.
actions!(knot_app, [OpenPersonas, OpenPrompts, OpenBench]);

/// Every user-facing quit path lands here - the application menu's Quit
/// Knot item and the `cmd-q` binding both dispatch `Quit` - so the guard
/// only has to be applied once. See `quit_guard` for why the check cannot
/// live in `on_app_quit` instead.
pub(crate) fn quit(_: &Quit, cx: &mut App) {
    quit_guard::request_quit(cx);
}

/// Quits on Ctrl-C (or `kill`) from the launching terminal.
///
/// Under `cargo run` the signal appeared to be swallowed: the Cocoa run
/// loop keeps the process alive and nothing here handled it, so the only
/// way out was Cmd+Q or killing the process from another shell. A
/// terminal-launched process is expected to die on Ctrl-C, so this restores
/// that. It exits rather than routing through `cx.quit()` because the
/// handler runs off the main thread and cannot reach the app; the child
/// PTYs go with the process, and the MCP server's listener is closed by the
/// same exit.
#[cfg(unix)]
pub(crate) fn quit_on_terminal_signals() {
    use tokio::signal::unix::{SignalKind, signal};

    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all()
                                                                         .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("failed to start the signal runtime: {error}");
                return;
            }
        };
        runtime.block_on(async {
                   let (Ok(mut interrupt), Ok(mut terminate)) =
                       (signal(SignalKind::interrupt()), signal(SignalKind::terminate()))
                   else {
                       eprintln!("failed to install terminal signal handlers");
                       return;
                   };
                   tokio::select! {
                       _ = interrupt.recv() => {}
                       _ = terminate.recv() => {}
                   }
                   crate::app_log::flush();
                   // 128 + SIGINT, the conventional shell exit code.
                   std::process::exit(130);
               });
    });
}

#[cfg(not(unix))]
pub(crate) fn quit_on_terminal_signals() {}

/// Opens Knot's help page. Through the platform rather than `open_in`: that
/// runs `/usr/bin/open` and waits for it, and this is a menu handler on the
/// main thread.
pub(crate) fn knot_help(_: &KnotHelp, cx: &mut App) {
    cx.open_url(crate::consts::KNOT_HELP_URL);
}

pub(crate) fn hide_app(_: &HideApp, cx: &mut App) {
    cx.hide();
}

pub(crate) fn hide_others(_: &HideOthers, cx: &mut App) {
    cx.hide_other_apps();
}

pub(crate) fn show_all_windows(_: &ShowAllWindows, cx: &mut App) {
    cx.activate(true);
}
