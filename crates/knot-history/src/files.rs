//! The file operations a history delete makes, with the two rules every
//! provider shares: a path that is already gone counts as deleted, and a file
//! another tool owns is rewritten whole or not at all.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::consts::REWRITE_TEMP_EXTENSION;
use crate::error::{HistoryError, Result};

/// Refuses an `id` that is not one plain path component: empty, `.`, `..`, or
/// holding a `/` or `\`. A session id is joined onto a directory before
/// anything is removed, and must never reach outside it.
pub(crate) fn check_session_id(id: &str) -> Result<()> {
    let plain = !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', '\0']);
    if plain {
        Ok(())
    }
    else {
        Err(HistoryError::InvalidSessionId(id.to_owned()))
    }
}

/// Removes `path` - a file, or a directory with everything in it - only if it
/// lies inside `root` once resolved, and is not `root` itself. A symlink is
/// removed as a link, never followed. A path that does not exist is already
/// gone.
pub(crate) fn remove_inside(root: &Path, path: &Path) -> Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let root = root.canonicalize()?;
    if meta.file_type().is_symlink() {
        // The link's own location has to be inside; where it points does not
        // matter, because only the link goes.
        let parent = path.parent()
                         .ok_or_else(|| HistoryError::OutsideHistory(path.to_owned()))?
                         .canonicalize()?;
        if !parent.starts_with(&root) {
            return Err(HistoryError::OutsideHistory(path.to_owned()));
        }
        return Ok(fs::remove_file(path)?);
    }
    let target = path.canonicalize()?;
    if target == root || !target.starts_with(&root) {
        return Err(HistoryError::OutsideHistory(path.to_owned()));
    }
    Ok(remove_if_present(&target)?)
}

/// The resolved `path` if it is a regular file inside `root`, `None` if it
/// does not exist, an error otherwise - for a path another tool recorded,
/// which is checked before anything is changed and removed after.
pub(crate) fn checked_file_inside(root: &Path, path: &Path) -> Result<Option<PathBuf>> {
    let target = match path.canonicalize() {
        Ok(target) => target,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let root = root.canonicalize()
                   .map_err(|_| HistoryError::OutsideHistory(path.to_owned()))?;
    if !target.starts_with(&root) {
        return Err(HistoryError::OutsideHistory(path.to_owned()));
    }
    if !fs::metadata(&target)?.is_file() {
        return Err(HistoryError::NotAFile(path.to_owned()));
    }
    Ok(Some(target))
}

/// Removes `path`, a file or a directory with everything in it. A path that
/// does not exist is already deleted, so that is not an error.
///
/// Unchecked: callers go through [`remove_inside`] or [`checked_file_inside`],
/// which confine it to a history directory first.
fn remove_if_present(path: &Path) -> io::Result<()> {
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
