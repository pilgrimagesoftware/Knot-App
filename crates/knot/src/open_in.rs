//! The "Open In…" submenu: the applications an agent's folder can be
//! opened in, and how each one is launched.
//!
//! Ports `Skwad/Models/OpenWithProvider.swift`. Everything but Zed goes
//! through `/usr/bin/open`, and a launch that fails - the application is not
//! installed - is swallowed rather than surfaced, per `agent-list-ui`'s
//! "fail quietly" requirement: the user cannot act on "VS Code is not
//! installed" from inside a context menu, and a dialog for it would be
//! worse than nothing happening.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One application in the submenu.
///
/// An enum rather than an id string with a `_ => None` arm: the set is
/// fixed, and the arguments each one needs are part of knowing which it is
/// (#224). The only place a string still appears is the menu-bar action,
/// which crosses gpui's action boundary - and [`OpenInApp::from_id`] is the
/// one place that can fail to recognize one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenInApp {
    VsCode,
    Zed,
    Xcode,
    Finder,
    Terminal,
}

/// A submenu entry: an application, or the divider between the editors and
/// the file/terminal pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenInEntry {
    App(OpenInApp),
    Separator,
}

impl OpenInApp {
    /// The stable id the menu-bar action carries.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::VsCode => "vscode",
            Self::Zed => "zed",
            Self::Xcode => "xcode",
            Self::Finder => "finder",
            Self::Terminal => "terminal",
        }
    }

    /// The application's own name, which is not translated: these are
    /// product names, the same in every locale.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::VsCode => "VS Code",
            Self::Zed => "Zed",
            Self::Xcode => "Xcode",
            Self::Finder => "Finder",
            Self::Terminal => "Terminal",
        }
    }

    /// The application an id names, or `None` for one this build does not
    /// know - which can only reach here from a stale menu-bar action.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        [Self::VsCode,
         Self::Zed,
         Self::Xcode,
         Self::Finder,
         Self::Terminal].into_iter()
                        .find(|app| app.id() == id)
    }

    /// The `open` arguments for this application, ahead of the folder
    /// itself.
    ///
    /// `-b` takes a bundle identifier, `-a` an application name, and
    /// getting the two the wrong way round fails silently at the point of
    /// use - which is why this is a value to test rather than a line in
    /// the launcher.
    const fn open_arguments(self) -> &'static [&'static str] {
        match self {
            Self::VsCode => &["-b", "com.microsoft.VSCode"],
            Self::Zed => &["-b", ZED_BUNDLE_ID],
            Self::Xcode => &["-a", "Xcode"],
            // Plain `open <folder>` hands a directory to Finder.
            Self::Finder => &[],
            Self::Terminal => &["-b", GHOSTTY_BUNDLE_ID],
        }
    }
}

/// The submenu's contents: the editors, a divider, then the two system
/// applications. The reference's order, with Zed added beside the other
/// cross-platform editor rather than after the Apple one - Zed is an
/// addition of this port's own, not something `OpenWithProvider` has.
pub(crate) fn open_in_entries() -> Vec<OpenInEntry> {
    vec![OpenInEntry::App(OpenInApp::VsCode),
         OpenInEntry::App(OpenInApp::Zed),
         OpenInEntry::App(OpenInApp::Xcode),
         OpenInEntry::Separator,
         OpenInEntry::App(OpenInApp::Finder),
         OpenInEntry::App(OpenInApp::Terminal),]
}

const GHOSTTY_BUNDLE_ID: &str = "com.mitchellh.ghostty";

/// Read from the installed application's `Info.plist`, not recalled.
const ZED_BUNDLE_ID: &str = "dev.zed.Zed";

/// Zed's command-line tool, relative to its application bundle - the binary
/// `zed` on `PATH` links to once the user installs it.
#[cfg(target_os = "macos")]
const ZED_CLI_IN_BUNDLE: &str = "Contents/MacOS/cli";

/// Opens `folder` in `app`, quietly doing nothing if the platform is not
/// macOS or the application is not installed.
///
/// "Terminal" prefers Ghostty and falls back to Terminal.app, matching the
/// reference - which checks for Ghostty first because that is the terminal
/// Knot itself embeds.
///
/// Zed goes through its own CLI rather than `open`, so it gets a window of
/// its own - see [`open_in_zed`].
pub(crate) fn open_folder(app: OpenInApp, folder: &str) {
    if app == OpenInApp::Zed {
        open_in_zed(folder);
        return;
    }

    let opened = run_open(app.open_arguments(), folder);
    if !opened && app == OpenInApp::Terminal {
        run_open(&["-a", "Terminal"], folder);
    }
}

/// Opens `folder` in a new Zed window, launching Zed first if it is not
/// running.
///
/// Zed is single-instance: a plain `open -b dev.zed.Zed <folder>` delivers
/// the folder as an Apple "open document" event, which the running instance
/// routes into its *existing* window (#567). The fix for that passed
/// `<folder> --new` through `open --args`, but `--args` reaches the
/// *application* binary, and `--new` is a flag only Zed's CLI knows: the
/// application's parser rejects it and the launched process exits at once.
/// That made the menu item do nothing when Zed was not running, and only
/// activate it when it was (#585, #244).
///
/// So this runs the CLI from inside Zed's bundle, which takes `--new`,
/// starts Zed when needed and hands the folder to it. It runs on its own
/// thread, because a cold start is slow and the caller is a menu handler.
/// Without the CLI - Zed not installed, or a bundle without one - it falls
/// back to the Apple-event path: the folder in an existing window, or
/// nothing at all if Zed is not installed.
fn open_in_zed(folder: &str) {
    let Some(cli) = zed_cli_path()
    else {
        run_open(OpenInApp::Zed.open_arguments(), folder);
        return;
    };
    let folder = folder.to_string();
    std::thread::spawn(move || {
        if !run_zed_cli(&cli, &folder) {
            run_open(OpenInApp::Zed.open_arguments(), &folder);
        }
    });
}

/// The arguments for Zed's CLI to open `folder` in a new window.
///
/// `--new` (`-n`) is the CLI's flag, per `zed --help`. Zed's application
/// binary has no such flag and exits on it, which is #244.
fn zed_cli_arguments(folder: &str) -> [&str; 2] {
    ["--new", folder]
}

/// The CLI inside the Zed bundle Launch Services knows by
/// [`ZED_BUNDLE_ID`], if Zed is installed and the bundle has one.
///
/// Located through Launch Services rather than `PATH`: the `zed` link on
/// `PATH` exists only if the user installed it, and an app launched from
/// the Dock or Finder does not get the shell's `PATH` anyway.
#[cfg(target_os = "macos")]
fn zed_cli_path() -> Option<PathBuf> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;

    let bundle = NSWorkspace::sharedWorkspace()
        .URLForApplicationWithBundleIdentifier(&NSString::from_str(ZED_BUNDLE_ID))?
        .path()?
        .to_string();
    let cli = zed_cli_in_bundle(Path::new(&bundle));
    cli.is_file().then_some(cli)
}

/// Nothing to launch elsewhere: Knot ships on macOS, and the workspace only
/// builds on Linux for CI.
#[cfg(not(target_os = "macos"))]
fn zed_cli_path() -> Option<PathBuf> {
    None
}

/// Where the CLI sits inside the Zed bundle at `bundle`.
#[cfg(target_os = "macos")]
fn zed_cli_in_bundle(bundle: &Path) -> PathBuf {
    bundle.join(ZED_CLI_IN_BUNDLE)
}

/// Runs Zed's CLI on `folder`, returning whether it reported success. The
/// CLI exits once Zed has the folder, so this waits only for that, not for
/// the window to close.
fn run_zed_cli(cli: &Path, folder: &str) -> bool {
    Command::new(cli).args(zed_cli_arguments(folder))
                     .stdin(Stdio::null())
                     .stdout(Stdio::null())
                     .stderr(Stdio::null())
                     .status()
                     .is_ok_and(|status| status.success())
}

/// Opens `url` in the user's default browser, returning whether it worked.
///
/// The same `/usr/bin/open` path as [`open_folder`], with no `-a`: the point
/// is the browser the user has chosen, not one Knot picked. A pull request
/// opens there rather than in an embedded view, so it arrives already signed
/// in, with the user's extensions and their session.
///
/// The return value is read, unlike `open_folder`'s: a row that could not be
/// opened has to say so rather than looking like it did nothing.
pub(crate) fn open_url(url: &str) -> bool {
    run_open(&[], url)
}

/// Whether this platform can show a file in its file manager at all.
///
/// The artifact panel leaves Reveal in Finder out entirely where this is
/// false, as the processes section does with its process viewer.
pub(crate) const fn can_reveal_files() -> bool {
    cfg!(target_os = "macos")
}

/// Shows `path` selected in a Finder window, returning whether it worked.
///
/// `open -R` rather than opening the file: the point is where it is, not
/// another application's view of what is already on screen.
pub(crate) fn reveal_path(path: &Path) -> bool {
    can_reveal_files() && run_open(&["-R"], &path.to_string_lossy())
}

/// Whether this platform has a process viewer to offer at all.
///
/// The processes section leaves the action out entirely where this is
/// false, rather than showing a control that does nothing.
pub(crate) const fn has_process_viewer() -> bool {
    cfg!(target_os = "macos")
}

/// Opens the platform's process viewer - Activity Monitor on macOS.
///
/// The application only. macOS exposes no supported way to select a given
/// process inside Activity Monitor from outside it, so the row pairs this
/// with copy-identifier rather than claiming a selection it cannot perform.
pub(crate) fn open_process_viewer() -> bool {
    if !has_process_viewer() {
        return false;
    }

    run_open(&["-a"], "Activity Monitor")
}

#[cfg(target_os = "macos")]
fn run_open(arguments: &[&str], folder: &str) -> bool {
    Command::new("/usr/bin/open").args(arguments)
                                 .arg(folder)
                                 .status()
                                 .is_ok_and(|status| status.success())
}

/// Nothing to open elsewhere: Knot ships on macOS, and the workspace only
/// builds on Linux for CI.
#[cfg(not(target_os = "macos"))]
fn run_open(_arguments: &[&str], _folder: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_submenu_matches_the_swift_reference_order() {
        assert_eq!(open_in_entries(),
                   vec![OpenInEntry::App(OpenInApp::VsCode),
                        OpenInEntry::App(OpenInApp::Zed),
                        OpenInEntry::App(OpenInApp::Xcode),
                        OpenInEntry::Separator,
                        OpenInEntry::App(OpenInApp::Finder),
                        OpenInEntry::App(OpenInApp::Terminal)]);
    }

    /// The ids the menu-bar action carries survive a round trip, which is
    /// the only place an application is still named by string.
    #[test]
    fn every_application_round_trips_through_its_id() {
        for app in [OpenInApp::VsCode,
                    OpenInApp::Zed,
                    OpenInApp::Xcode,
                    OpenInApp::Finder,
                    OpenInApp::Terminal]
        {
            assert_eq!(OpenInApp::from_id(app.id()), Some(app));
        }
        assert_eq!(OpenInApp::from_id("nothing-by-that-name"), None);
    }

    /// `-b` takes a bundle id and `-a` an application name; swapping them
    /// is a silent no-op at the point of use, which is exactly the failure
    /// this mapping exists to keep out of the menu handlers.
    #[test]
    fn each_application_maps_to_the_right_open_flag() {
        assert_eq!(OpenInApp::VsCode.open_arguments(),
                   ["-b", "com.microsoft.VSCode"]);
        // Zed is launched by bundle id like VS Code, not by name like
        // Xcode. The id was read from the installed application's
        // Info.plist, not recalled.
        assert_eq!(OpenInApp::Zed.open_arguments(), ["-b", "dev.zed.Zed"]);
        assert_eq!(OpenInApp::Xcode.open_arguments(), ["-a", "Xcode"]);
        assert!(OpenInApp::Finder.open_arguments().is_empty());
        assert_eq!(OpenInApp::Terminal.open_arguments(),
                   ["-b", "com.mitchellh.ghostty"]);
    }

    /// `--new` goes to Zed's CLI, which knows it, along with the folder
    /// (#567). Passing it to the application binary instead made it exit
    /// at once and opened nothing (#244).
    #[test]
    fn zed_is_told_to_open_the_folder_in_a_new_window() {
        assert_eq!(zed_cli_arguments("/tmp/project"), ["--new", "/tmp/project"]);
    }

    /// The CLI is the bundle's `cli` binary - the one `zed` on `PATH` links
    /// to - not the application binary beside it, `Contents/MacOS/zed`,
    /// which rejects `--new`.
    #[cfg(target_os = "macos")]
    #[test]
    fn zed_cli_is_the_bundle_cli_not_the_application_binary() {
        assert_eq!(zed_cli_in_bundle(Path::new("/Applications/Zed.app")),
                   Path::new("/Applications/Zed.app/Contents/MacOS/cli"));
    }

    /// The processes section offers the process-viewer action only where
    /// there is a viewer to open, so this is what decides whether the
    /// control exists at all.
    #[test]
    fn a_process_viewer_is_offered_only_on_macos() {
        assert_eq!(has_process_viewer(), cfg!(target_os = "macos"));
    }

    /// Where there is no viewer, the action is not merely hidden - calling
    /// it reports that nothing was opened rather than claiming success.
    #[test]
    fn opening_a_process_viewer_where_there_is_none_reports_failure() {
        if !has_process_viewer() {
            assert!(!open_process_viewer());
        }
    }
}
