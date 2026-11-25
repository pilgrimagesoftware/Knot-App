//! Only a regular file inside the transcript directory is read, and only
//! its tail.

use std::fs;

use tempfile::TempDir;

use super::*;

fn root() -> (TempDir, TranscriptRoot) {
    let dir = TempDir::new().expect("a temporary directory");
    fs::create_dir(dir.path().join("projects")).unwrap();
    let root = TranscriptRoot::at(dir.path().join("projects")).expect("the root exists");
    (dir, root)
}

#[test]
fn a_transcript_inside_the_directory_is_read() {
    let (dir, root) = root();
    let path = dir.path().join("projects/-tmp-project/session.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{\"type\":\"assistant\"}\n").unwrap();

    assert_eq!(root.read_tail(&path).as_deref(),
               Some("{\"type\":\"assistant\"}\n"));
}

#[test]
fn a_path_outside_the_directory_is_refused() {
    let (dir, root) = root();
    let outside = dir.path().join("secrets.txt");
    fs::write(&outside, "not a transcript").unwrap();

    assert_eq!(root.read_tail(&outside), None);
    assert_eq!(root.read_tail(dir.path().join("projects/../secrets.txt")),
               None,
               "a `..` that climbs out is resolved before the check");
}

#[cfg(unix)]
#[test]
fn a_symlink_escaping_the_directory_is_refused() {
    let (dir, root) = root();
    let outside = dir.path().join("secrets.txt");
    fs::write(&outside, "not a transcript").unwrap();
    let link = dir.path().join("projects/session.jsonl");
    std::os::unix::fs::symlink(&outside, &link).unwrap();

    assert_eq!(root.read_tail(&link), None);
}

#[cfg(unix)]
#[test]
fn a_device_is_never_read() {
    let (dir, root) = root();
    let link = dir.path().join("projects/session.jsonl");
    std::os::unix::fs::symlink("/dev/zero", &link).unwrap();

    assert_eq!(root.read_tail(&link), None);
    // Even a root that contains it does not make a device readable.
    let dev = TranscriptRoot::at("/dev").expect("/dev exists");
    assert_eq!(dev.read_tail("/dev/zero"), None);
}

#[test]
fn an_oversized_transcript_is_read_only_to_its_cap() {
    let (dir, root) = root();
    let path = dir.path().join("projects/big.jsonl");
    let cap = usize::try_from(consts::TRANSCRIPT_TAIL_BYTES).unwrap();
    let mut text = String::from("{\"first\":true}\n");
    let filler = format!("{}\n", "x".repeat(1023));
    while text.len() < cap + 4096 {
        text.push_str(&filler);
    }
    text.push_str("{\"last\":true}\n");
    fs::write(&path, &text).unwrap();

    let tail = root.read_tail(&path).expect("read");
    assert!(tail.len() <= cap, "{} bytes read", tail.len());
    assert!(tail.ends_with("{\"last\":true}\n"));
    assert!(!tail.contains("first"),
            "the head beyond the cap is not read");
    assert!(tail.starts_with('x') || tail.starts_with('{'),
            "starts on a line boundary");
}

#[test]
fn a_missing_root_or_file_reads_nothing() {
    let (dir, root) = root();
    assert_eq!(TranscriptRoot::at(dir.path().join("absent")), None);
    assert_eq!(root.read_tail(dir.path().join("projects/absent.jsonl")),
               None);
}

/// Security review of #521: a FIFO inside the directory is refused at once,
/// not opened - opening one blocks until something writes to it.
#[cfg(unix)]
#[test]
fn a_fifo_inside_the_directory_is_refused_without_blocking() {
    let (dir, root) = root();
    let fifo = dir.path().join("projects/session.jsonl");
    let made = std::process::Command::new("mkfifo").arg(&fifo)
                                                   .status()
                                                   .expect("mkfifo runs");
    assert!(made.success());

    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(root.read_tail(&fifo));
    });
    let read = finished.recv_timeout(std::time::Duration::from_secs(5))
                       .expect("the read returned instead of blocking");
    assert_eq!(read, None);
}
