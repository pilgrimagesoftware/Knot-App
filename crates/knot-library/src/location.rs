use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{LibraryError, Result};
use crate::index::IndexItem;
use crate::resolve::resolve;

/// Where a library is published: a GitHub repository, a plain web address,
/// or a folder on disk. Each kind knows how to address its own `index.json`
/// and item files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Location {
    GitHub {
        /// `owner/repo`.
        repo:   String,
        /// Defaults to the repository's default branch (`HEAD`) when absent.
        branch: Option<String>,
    },
    Web {
        /// An `https://` base URL, no trailing slash.
        base: String,
    },
    Folder {
        path: PathBuf,
    },
}

impl Location {
    /// Checks the location's own form: `owner/repo` for GitHub, `https://`
    /// for Web, an existing directory for Folder.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::GitHub { repo, .. } => {
                let parts: Vec<&str> = repo.split('/').collect();
                let valid = parts.len() == 2 && parts.iter().all(|p| !p.is_empty());
                if !valid {
                    return Err(LibraryError::InvalidGitHubRepo(repo.clone()));
                }
                Ok(())
            }
            Self::Web { base } => {
                if !base.starts_with("https://") {
                    return Err(LibraryError::InvalidWebBase(base.clone()));
                }
                Ok(())
            }
            Self::Folder { path } => {
                if !path.is_dir() {
                    return Err(LibraryError::InvalidFolder(path.display().to_string()));
                }
                Ok(())
            }
        }
    }

    /// The address of this location's `index.json`.
    pub fn index_address(&self) -> String {
        match self {
            Self::GitHub { repo, branch } => {
                let branch = branch.as_deref().unwrap_or("HEAD");
                format!("https://raw.githubusercontent.com/{repo}/{branch}/index.json")
            }
            Self::Web { base } => format!("{base}/index.json"),
            Self::Folder { path } => path.join("index.json").display().to_string(),
        }
    }

    /// The address of `item`'s file within this location, pinned to
    /// `commit` for GitHub (the only kind with one to pin to).
    pub fn item_address(&self, commit: &str, item: &IndexItem) -> Result<String> {
        let path = resolve(&item.path)?;
        Ok(match self {
            Self::GitHub { repo, .. } => {
                format!("https://raw.githubusercontent.com/{repo}/{commit}/{path}")
            }
            Self::Web { base } => format!("{base}/{path}"),
            Self::Folder { path: dir } => dir.join(path).display().to_string(),
        })
    }
}

#[cfg(test)]
#[path = "location/tests.rs"]
mod tests;
