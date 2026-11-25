//! The file operations a history delete makes, with the two rules every
//! provider shares: a path that is already gone counts as deleted, and a file
//! another tool owns is rewritten whole or not at all.

use std::fs;
use std::io;
use std::path::Path;

use crate::consts::REWRITE_TEMP_EXTENSION;

/// Removes `path`, a file or a directory with everything in it. A path that
/// does not exist is already deleted, so that is not an error.
pub(crate) fn remove_if_present(path: &Path) -> io::Result<()> {
    let removed = match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) => Err(error),
    };
    match removed {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Replaces `path` with `contents` through a sibling temporary file and a
/// rename, so a failure part way leaves the old file intact rather than a
/// truncated one. The pattern of `knot-core`'s settings documents.
///
/// For files another tool owns and reads back - Gemini's `logs.json` holds
/// every session's entries, not only the one being deleted.
pub(crate) fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    // Same directory as the target: `rename` is only atomic within one
    // filesystem.
    let temporary = path.with_extension(REWRITE_TEMP_EXTENSION);
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path).inspect_err(|_| {
                                    // A stale temporary would shadow the next
                                    // attempt's write.
                                    let _ = fs::remove_file(&temporary);
                                })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_path_is_already_removed() {
        let tmp = tempfile::tempdir().unwrap();
        remove_if_present(&tmp.path().join("gone")).unwrap();
    }

    #[test]
    fn a_directory_is_removed_with_its_contents() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("session");
        fs::create_dir_all(dir.join("inner")).unwrap();
        fs::write(dir.join("inner/file"), "x").unwrap();

        remove_if_present(&dir).unwrap();

        assert!(!dir.exists());
    }

    #[test]
    fn a_rewrite_replaces_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("logs.json");
        fs::write(&path, "old").unwrap();

        write_atomically(&path, "new").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert!(!path.with_extension(REWRITE_TEMP_EXTENSION).exists());
    }

    /// The temporary cannot be written - here a directory is in its way. The
    /// error comes back and the original file is untouched.
    #[test]
    fn a_failed_rewrite_leaves_the_original_intact() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("logs.json");
        fs::write(&path, "original").unwrap();
        fs::create_dir(path.with_extension(REWRITE_TEMP_EXTENSION)).unwrap();

        assert!(write_atomically(&path, "new").is_err());

        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}
