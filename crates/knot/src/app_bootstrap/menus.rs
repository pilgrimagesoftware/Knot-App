//! The menu bar: every menu and item, rebuilt from a [`MenuBarSnapshot`]
//! whenever the store-dependent submenus change.

use gpui_kit::App;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use gpui_kit::SystemMenuType;
use gpui_kit::base::input;

use super::actions::*;
use crate::agent_menu::agents_menu;
use crate::menu_bar::MenuBarSnapshot;
use crate::view_menu::view_menu;

/// Installs the menu bar.
///
/// Called again whenever `snapshot` changes, because a `Menu` is a static
/// snapshot: the Agents menu's Move to Workspace and Markdown Files
/// submenus cannot re-read the store on their own (`app-menu`). Everything
/// else in the bar is rebuilt identically, which is cheap and keeps the
/// whole bar described in one place.
pub(crate) fn set_app_menus(snapshot: &MenuBarSnapshot, cx: &mut App) {
    cx.set_menus([
        Menu::new("Knot").items([
            MenuItem::action("About Knot", AboutKnot),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::action(knot_core::l10n::t("menu.window.personas"), OpenPersonas),
            MenuItem::action(knot_core::l10n::t("menu.window.prompts"), OpenPrompts),
            MenuItem::action(knot_core::l10n::t("menu.window.bench"), OpenBench),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Knot", HideApp),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAllWindows),
            MenuItem::separator(),
            MenuItem::action("Quit Knot", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Workspace", NewWorkspace).disabled(true),
            MenuItem::action(knot_core::l10n::t("menu.file.new_agent"), NewAgent),
            MenuItem::separator(),
            MenuItem::action("Import…", OpenImport),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        // The text actions gpui already defines and binds for a focused
        // input, rather than placeholders of our own. Reusing them is what
        // puts the standard shortcuts beside these labels, and it is also
        // the only safe way to get them: a placeholder action of ours bound
        // to cmd-c would out-rank the input's own binding - a context-less
        // binding ranks at the deepest context, and later bindings win ties
        // - and copying in a text field would stop working.
        //
        // They carry no `disabled`, because these five do work: AppKit asks
        // whether each action is available along the focused element's
        // dispatch path, so they enable with a text field focused and grey
        // out elsewhere. In the terminal pane, where nothing claims them,
        // the disabled items let cmd-c fall through to the pane's own
        // copy-selection handler.
        Menu::new("Edit").items([
            MenuItem::action("Undo", input::Undo),
            MenuItem::action("Redo", input::Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", input::Cut),
            MenuItem::action("Copy", input::Copy),
            MenuItem::action("Paste", input::Paste),
            MenuItem::separator(),
            // Enabled only with an agent selected: the window registers its
            // handler then, and AppKit greys out an item nothing handles.
            MenuItem::action(knot_core::l10n::t("menu.edit.find_file"), FindFile),
        ]),
        // The navigation shortcuts. macOS still appends its own Enter Full
        // Screen below them; see the note beside the `actions!` block.
        view_menu(&snapshot.view),
        agents_menu(&snapshot.agents),
        // Knot's own items, then the list of open windows macOS appends;
        // see `window_menu`.
        crate::window_menu::window_menu(&snapshot.window),
        // Report an Issue has no key equivalent: macOS gives it none, and the
        // item is enabled everywhere because it is how a user asks for help.
        Menu::new("Help").items([
            MenuItem::action("Knot Help", KnotHelp),
            MenuItem::action(knot_core::l10n::t("menu.help.report_issue"), ReportIssue),
        ]),
    ]);
}
