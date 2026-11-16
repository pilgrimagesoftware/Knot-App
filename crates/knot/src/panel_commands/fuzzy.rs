//! Fuzzy ranking for completing file paths: the Swift reference's scorer
//! (`crate::fuzzy_score`), shaped into lookup matches.
//!
//! `panel-file-mentions` asks for subsequence matching, name over directory
//! and runs over scattered characters; the reference's scorer gives all
//! three, and the `@` lookup and the file finder share it so they order the
//! same files the same way (`openspec/changes/file-finder`). It replaced a
//! greedy scorer that only approximated it.
//!
//! **Costly over a large listing.** Call it off the render path - the `@`
//! lookup and the finder both go through `crate::file_search`.

use crate::fuzzy_score;
use crate::panel_commands::entry::LookupEntry;
use crate::panel_commands::entry::LookupMatch;

/// The entries `filter` selects, best first, at most
/// `consts::FUZZY_MAX_RESULTS`.
pub(crate) fn matching(entries: &[LookupEntry], filter: &str) -> Vec<LookupMatch> {
    ranked(entries, filter).into_iter()
                           .map(|(_, found)| found)
                           .collect()
}

/// [`matching`] with each match's score, which merging rankings of
/// separate batches needs.
pub(crate) fn ranked(entries: &[LookupEntry], filter: &str) -> Vec<(i32, LookupMatch)> {
    let by_token: std::collections::HashMap<&str, &LookupEntry> =
        entries.iter()
               .map(|entry| (entry.token.as_str(), entry))
               .collect();
    fuzzy_score::rank(filter, entries.iter().map(|entry| entry.token.as_str()))
        .into_iter()
        .map(|(token, found)| {
            let entry = by_token[token];
            (found.score,
             LookupMatch { entry:     entry.clone(),
                           matched:   byte_offsets(token, &found.indices),
                           is_status: false, })
        })
        .collect()
}

/// The score order [`ranked`] uses, for merging: score first, then token.
pub(crate) fn order(left: &(i32, LookupMatch), right: &(i32, LookupMatch)) -> std::cmp::Ordering {
    right.0
         .cmp(&left.0)
         .then_with(|| left.1.entry.token.cmp(&right.1.entry.token))
}

/// `char` offsets into `text` as byte offsets, which is what a lookup row
/// highlights by.
fn byte_offsets(text: &str, chars: &[usize]) -> Vec<usize> {
    let starts: Vec<usize> = text.char_indices().map(|(byte, _)| byte).collect();
    chars.iter()
         .filter_map(|index| starts.get(*index).copied())
         .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_become_byte_offsets() {
        assert_eq!(byte_offsets("é/lib.rs", &[2, 3, 4]), vec![3, 4, 5]);
    }
}
