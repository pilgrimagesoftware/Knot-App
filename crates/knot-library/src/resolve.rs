use std::path::{Component, Path};

use crate::error::{LibraryError, Result};

/// Refuses a `path` that is absolute or has a `..` component, so an index
/// entry can't point outside the location it came from.
pub fn resolve(path: &str) -> Result<&str> {
    let parsed = Path::new(path);
    for component in parsed.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(LibraryError::PathEscapesLocation(path.to_string()));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }
    Ok(path)
}

#[cfg(test)]
#[path = "resolve/tests.rs"]
mod tests;
