//! Every window mounts `Root` (`.claude/rules/knot-ui-conventions.md`,
//! "Dialog layer").
//!
//! `Root` styles the window's surface with the theme's font and foreground
//! colour, and hosts the dialog and notification layers. A window built
//! without it still opens and still works, so nothing failed when three
//! sheets skipped it - the prompt inputs, broadcast and autopilot decision
//! sheets drew black text on the dark background and the system font on
//! their buttons until someone looked at one in dark mode (#239).
//!
//! Every window also observes the system appearance, so a light/dark flip
//! repaints it in the new palette. The commit and About windows skipped it
//! and kept the old palette until reopened (#480, #481) - again nothing
//! failed, because a window that never repaints on a flip looks fine in
//! whichever appearance it opened in.
//!
//! The scan is textual: the code after each `cx.open_window(`, up to the
//! next one, must call `Root::new(` and `observe_system_appearance(`. The
//! callback that builds the window sits in that stretch, so a window whose
//! callback skips either fails here.

use std::path::Path;
use std::path::PathBuf;

const OPEN: &str = "cx.open_window(";
const MOUNT: &str = "Root::new(";
const OBSERVE: &str = "observe_system_appearance(";

/// Every non-test `.rs` file under `crates/knot/src`.
fn production_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir)
    else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "tests" {
                production_files(&path, found);
            }
        }
        else if name.ends_with(".rs") && !name.ends_with("tests.rs") {
            found.push(path);
        }
    }
}

/// How many of `source`'s `open_window` calls are not followed by `call`
/// before the next `open_window` or the end of the file.
fn windows_without(source: &str, call: &str) -> usize {
    source.split(OPEN)
          .skip(1)
          .filter(|after_open| !after_open.contains(call))
          .count()
}

/// How many of `source`'s `open_window` calls are not followed by a
/// `Root::new` before the next call or the end of the file.
fn bare_windows(source: &str) -> usize {
    windows_without(source, MOUNT)
}

/// Every production file's path under `src`, paired with its source.
fn production_sources() -> (PathBuf, Vec<(String, String)>) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_files(&src, &mut files);
    let sources = files.iter()
                       .map(|file| {
                           (file.strip_prefix(&src)
                                .unwrap_or(file)
                                .display()
                                .to_string(),
                            std::fs::read_to_string(file).expect("read a source file"))
                       })
                       .collect();
    (src, sources)
}

/// The files with a window that skips `call`, after checking the scan found
/// the app's windows at all - so an empty result means something.
fn files_with_windows_without(call: &str) -> Vec<String> {
    let (src, sources) = production_sources();
    let windows: usize = sources.iter()
                                .map(|(_, source)| source.matches(OPEN).count())
                                .sum();
    assert!(windows >= 10,
            "found only {windows} windows under {}",
            src.display());
    sources.into_iter()
           .filter(|(_, source)| windows_without(source, call) > 0)
           .map(|(path, _)| path)
           .collect()
}

#[test]
fn the_scan_tells_a_bare_window_from_a_mounted_one() {
    let mounted =
        "cx.open_window(o, |w, cx| { let v = cx.new(V); cx.new(|cx| Root::new(v, w, cx)) })";
    let bare = "cx.open_window(o, |_, cx| cx.new(|_| Sheet))";
    assert_eq!(bare_windows(mounted), 0);
    assert_eq!(bare_windows(bare), 1);
    assert_eq!(bare_windows(&format!("{bare}\n{mounted}")), 1);
    assert_eq!(bare_windows(&format!("{mounted}\n{bare}")), 1);
}

#[test]
fn the_scan_tells_an_observing_window_from_one_that_is_not() {
    let observing = "cx.open_window(o, |w, cx| { observe_system_appearance(w); cx.new(V) })";
    let blind = "cx.open_window(o, |_, cx| cx.new(|_| Sheet))";
    assert_eq!(windows_without(observing, OBSERVE), 0);
    assert_eq!(windows_without(blind, OBSERVE), 1);
    assert_eq!(windows_without(&format!("{observing}\n{blind}"), OBSERVE),
               1);
}

#[test]
fn every_window_mounts_root() {
    let bare = files_with_windows_without(MOUNT);
    assert!(bare.is_empty(),
            "windows opened without Root::new: {bare:?}");
}

#[test]
fn every_window_observes_the_system_appearance() {
    let blind = files_with_windows_without(OBSERVE);
    assert!(blind.is_empty(),
            "windows opened without observe_system_appearance: {blind:?}");
}
