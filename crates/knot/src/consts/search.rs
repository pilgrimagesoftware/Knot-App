//! Conversation find and file-finder limits.

/// The largest compiled program a conversation find's regular expression may
/// build (#251), in bytes. A pattern past it is reported as invalid rather
/// than stalling every keystroke.
pub(crate) const FIND_REGEX_SIZE_LIMIT: usize = 1 << 20;

/// Matches the file finder and `@` ranking keep (#5): the reference's
/// `maxResults`.
pub(crate) const FUZZY_MAX_RESULTS: usize = 50;
/// Paths ranked between checks that a search is still wanted.
pub(crate) const FILE_SEARCH_BATCH: usize = 4096;
/// The finder's wait after the query last changed: the reference's debounce.
pub(crate) const FILE_FINDER_DEBOUNCE_MS: u64 = 150;
