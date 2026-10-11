use sha2::{Digest, Sha256};

use crate::error::{LibraryError, Result};
use crate::index::IndexItem;

/// Checks `bytes` against the index's recorded size and sha256, size first
/// since it's cheap and catches most mismatches without hashing.
pub fn verify(bytes: &[u8], item: &IndexItem) -> Result<()> {
    let actual_size = bytes.len() as u64;
    if actual_size != item.size {
        return Err(LibraryError::SizeMismatch { expected: item.size,
                                                actual:   actual_size, });
    }
    let actual_hash = hex_sha256(bytes);
    if !actual_hash.eq_ignore_ascii_case(&item.sha256) {
        return Err(LibraryError::HashMismatch { expected: item.sha256.clone(),
                                                actual:   actual_hash, });
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Drops the `---`-delimited front matter and returns the trimmed body,
/// refusing an unterminated front matter or an empty body. The title and id
/// come from the index entry, not the front matter, so no YAML parser is
/// needed here.
pub fn split_item(bytes: &[u8]) -> Result<String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start();
    let rest = text.strip_prefix("---")
                   .ok_or(LibraryError::UnterminatedFrontMatter)?;
    let close = rest.find("\n---")
                    .ok_or(LibraryError::UnterminatedFrontMatter)?;
    let after_front_matter = &rest[close + 4..];
    let body = after_front_matter.strip_prefix('\n')
                                 .unwrap_or(after_front_matter)
                                 .trim();
    if body.is_empty() {
        return Err(LibraryError::EmptyBody);
    }
    Ok(body.to_string())
}

#[cfg(test)]
#[path = "verify/tests.rs"]
mod tests;
