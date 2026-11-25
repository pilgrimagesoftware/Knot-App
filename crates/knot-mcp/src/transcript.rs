//! Reading the transcript a hook names, confined to where the agent CLI
//! keeps its transcripts.
//!
//! A hook's `transcript_path` is whatever the request body says. Read as
//! given, a request could make Knot read any file the user can - or a
//! device that never ends. So a path is read only when, with every symlink
//! resolved, it is a regular file inside the agent type's transcript
//! directory, and then only its last [`consts::TRANSCRIPT_TAIL_BYTES`].

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::consts;

/// A directory transcripts may be read from, held canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRoot {
    dir: PathBuf,
}

impl TranscriptRoot {
    /// `dir`, if it exists, with its symlinks resolved.
    pub fn at(dir: impl AsRef<Path>) -> Option<Self> {
        dir.as_ref().canonicalize().ok().map(|dir| Self { dir })
    }

    /// Claude Code's transcript directory: `projects` under
    /// `CLAUDE_CONFIG_DIR`, or under `~/.claude` when that is unset.
    /// `None` when it does not exist, and then no transcript is read.
    pub fn claude() -> Option<Self> {
        let config = std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from)
                                                          .or_else(|| {
                                                              std::env::home_dir().map(|home| {
                                                                             home.join(".claude")
                                                                         })
                                                          })?;
        Self::at(config.join(consts::CLAUDE_TRANSCRIPTS_DIR))
    }

    /// The end of the transcript at `path`: at most
    /// [`consts::TRANSCRIPT_TAIL_BYTES`], starting at a line boundary.
    /// `None` when `path` resolves outside this directory, is not a regular
    /// file, or cannot be read.
    pub fn read_tail(&self, path: impl AsRef<Path>) -> Option<String> {
        let resolved = path.as_ref().canonicalize().ok()?;
        if !resolved.starts_with(&self.dir) {
            return None;
        }
        let mut file = File::open(&resolved).ok()?;
        // Checked on the open handle, not the path, so what is read is what
        // was checked: a regular file, never a device or a pipe.
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() {
            return None;
        }
        let start = metadata.len().saturating_sub(consts::TRANSCRIPT_TAIL_BYTES);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut bytes = Vec::new();
        file.take(consts::TRANSCRIPT_TAIL_BYTES)
            .read_to_end(&mut bytes)
            .ok()?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        // Starting mid-file lands mid-line; that first fragment is not JSON.
        Some(match (start > 0, text.find('\n')) {
                 (true, Some(newline)) => text[newline + 1..].to_owned(),
                 (true, None) => String::new(),
                 (false, _) => text,
             })
    }
}

#[cfg(test)]
mod tests;
