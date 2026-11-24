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
//! The scan is textual: the code after each `cx.open_window(`, up to the
//! next one, must call `Root::new(`. The callback that builds the window's
//! view sits in that stretch, so a window whose callback returns its view
//! bare fails here.

use std::path::Path;
use std::path::PathBuf;

const OPEN: &str = "cx.open_window(";
const MOUNT: &str = "Root::new(";

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

/// How many of `source`'s `open_window` calls are not followed by a
/// `Root::new` before the next call or the end of the file.
fn bare_windows(source: &str) -> usize {
    source.split(OPEN)
          .skip(1)
          .filter(|after_open| !after_open.contains(MOUNT))
          .count()
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
fn every_window_mounts_root() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_files(&src, &mut files);
    let mut windows = 0;
    let mut bare: Vec<String> = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).expect("read a source file");
        windows += source.matches(OPEN).count();
        if bare_windows(&source) > 0 {
            bare.push(file.strip_prefix(&src)
                          .unwrap_or(file)
                          .display()
                          .to_string());
        }
    }
    // The scan found the app's windows at all, so an empty pass means
    // something.
    assert!(windows >= 10,
            "found only {windows} windows under {}",
            src.display());
    assert!(bare.is_empty(),
            "windows opened without Root::new: {bare:?}");
}
