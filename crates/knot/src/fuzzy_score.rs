//! The Swift reference's `FuzzyScorer`, ported statement for statement (#5,
//! `openspec/changes/file-finder`): VS Code's fuzzy matcher, by way of
//! `D0ntPanic/code-fuzzy-match`.
//!
//! Ported rather than re-derived because the ranking is the part a user
//! feels. `tests` replays a snapshot the Swift scorer itself produced, so a
//! difference shows up as a failing test rather than as a finder that feels
//! off. Two quirks are kept on purpose, since fixing them changes the
//! order (`design.md`): the word-start bonus's separator branch can never
//! fire, and the backtrack that recovers the matched offsets is greedy.
//!
//! Offsets are `char` offsets. Swift counts grapheme clusters; for paths
//! the two differ only inside combining sequences.

use crate::consts::FUZZY_MAX_RESULTS;

const BASE_MATCH_SCORE: i32 = 1;
const SAME_CASE_BONUS: i32 = 1;
const START_OF_TARGET_BONUS: i32 = 8;
const END_OF_TARGET_BONUS: i32 = 2;
const PATH_SEPARATOR_BONUS: i32 = 5;
const WORD_SEPARATOR_BONUS: i32 = 4;
const WORD_START_BONUS: i32 = 2;
const SEQUENTIAL_BONUS_MULTIPLIER: i32 = 5;
const FILENAME_BONUS: i32 = 10;
const FILENAME_EXACT_BONUS: i32 = 50;

const SEPARATORS: &[char] = &['_', '-', '.', ' ', '\'', '"', ':'];

/// A match: its score and the `char` offsets of the matched characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScoredMatch {
    pub(crate) score:   i32,
    pub(crate) indices: Vec<usize>,
}

/// Whether every character of `pattern` appears in `target`, in order,
/// ignoring case - the cheap check that rejects most paths before the DP.
pub(crate) fn is_pattern_in_word(pattern: &str, target: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if target.is_empty() {
        return false;
    }
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let mut pi = 0;
    for tc in target.to_lowercase().chars() {
        if tc == pattern[pi] {
            pi += 1;
            if pi == pattern.len() {
                return true;
            }
        }
    }
    false
}

fn same_ignoring_case(a: char, b: char) -> bool {
    a.to_lowercase().eq(b.to_lowercase())
}

/// `pattern` scored against `target`, or `None` when it does not match.
pub(crate) fn score(pattern: &str, target: &str) -> Option<ScoredMatch> {
    if pattern.is_empty() {
        return Some(ScoredMatch { score:   0,
                                  indices: Vec::new(), });
    }
    if target.is_empty() || !is_pattern_in_word(pattern, target) {
        return None;
    }
    let pattern: Vec<char> = pattern.chars().collect();
    let target: Vec<char> = target.chars().collect();
    let (m, n) = (pattern.len(), target.len());
    if m > n {
        return None;
    }

    let mut prev_score = vec![0_i32; n];
    let mut prev_seq = vec![0_i32; n];
    let mut match_table = vec![vec![false; n]; m];
    let mut first_query_char = true;

    for qi in 0..m {
        let mut curr_score = vec![0_i32; n];
        let mut curr_seq = vec![0_i32; n];
        for ti in 0..n {
            let (p_char, t_char) = (pattern[qi], target[ti]);
            let prev_target_score = if ti > 0 { curr_score[ti - 1] } else { 0 };
            let prev_query_score = if ti > 0 { prev_score[ti - 1] } else { 0 };
            let seq_match_count = if ti > 0 { prev_seq[ti - 1] } else { 0 };

            if !first_query_char && prev_query_score == 0 {
                curr_score[ti] = prev_target_score;
                continue;
            }
            if !same_ignoring_case(p_char, t_char) {
                curr_score[ti] = prev_target_score;
                continue;
            }

            let mut char_score = BASE_MATCH_SCORE + seq_match_count * SEQUENTIAL_BONUS_MULTIPLIER;
            if p_char == t_char {
                char_score += SAME_CASE_BONUS;
            }
            if ti == 0 {
                char_score += START_OF_TARGET_BONUS;
            }
            else {
                let prev_t_char = target[ti - 1];
                if prev_t_char == '/' || prev_t_char == '\\' {
                    char_score += PATH_SEPARATOR_BONUS;
                }
                else if SEPARATORS.contains(&prev_t_char) {
                    char_score += WORD_SEPARATOR_BONUS;
                }
                else if seq_match_count == 0 {
                    // The reference checks for a separator here too, which
                    // the branch above has already taken: only camelCase
                    // reaches the bonus. Kept, so the ranking is its.
                    if SEPARATORS.contains(&prev_t_char) || t_char.is_uppercase() {
                        char_score += WORD_START_BONUS;
                    }
                }
            }
            if ti == n - 1 {
                char_score += END_OF_TARGET_BONUS;
            }

            let new_score = prev_query_score + char_score;
            if new_score >= prev_target_score {
                curr_score[ti] = new_score;
                curr_seq[ti] = seq_match_count + 1;
                match_table[qi][ti] = true;
            }
            else {
                curr_score[ti] = prev_target_score;
            }
        }
        prev_score = curr_score;
        prev_seq = curr_seq;
        first_query_char = false;
    }

    let final_score = prev_score[n - 1];
    if final_score <= 0 {
        return None;
    }

    // Greedy from the end, as the reference does.
    let mut indices = Vec::with_capacity(m);
    let (mut qi, mut ti) = (m as isize - 1, n as isize - 1);
    while qi >= 0 && ti >= 0 {
        if match_table[qi as usize][ti as usize] {
            indices.push(ti as usize);
            qi -= 1;
        }
        ti -= 1;
    }
    indices.reverse();
    (indices.len() == m).then_some(ScoredMatch { score: final_score,
                                                 indices })
}

/// `path`'s last component, as `NSString.lastPathComponent` gives it: a
/// trailing slash is dropped first.
fn last_component(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

/// `name` without its extension, as `NSString.deletingPathExtension` gives
/// it: a name whose only dot leads (`.gitignore`) or ends it (`b.`) has
/// none.
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(0) | None => name,
        Some(dot) if dot == name.len() - 1 => name,
        Some(dot) => &name[..dot],
    }
}

/// `pattern` scored against a file `path`: the better of the full path's
/// score and the file name's, the latter with the file-name bonuses and its
/// offsets moved into the full path.
pub(crate) fn score_file(pattern: &str, path: &str) -> Option<ScoredMatch> {
    if pattern.is_empty() {
        return None;
    }
    let full = score(pattern, path);
    let filename = last_component(path);
    let named =
        score(pattern, filename).map(|found| {
                                    let offset = path.chars().count() - filename.chars().count();
                                    let exact_stem =
                                        stem(filename).to_lowercase() == pattern.to_lowercase();
                                    let first_char = pattern.to_lowercase().chars().next();
                                    let first_pos = first_char.and_then(|first| {
                                                                  filename.to_lowercase()
                                                                          .chars()
                                                                          .position(|c| c == first)
                                                              })
                                                              .unwrap_or(0)
                                                    as i32;
                                    let position_bonus = (5 - first_pos).max(0);
                                    ScoredMatch { score:   found.score
                                                           + FILENAME_BONUS
                                                           + position_bonus
                                                           + if exact_stem {
                                                               FILENAME_EXACT_BONUS
                                                           }
                                                           else {
                                                               0
                                                           },
                                                  indices: found.indices
                                                                .iter()
                                                                .map(|index| index + offset)
                                                                .collect(), }
                                });
    match (full, named) {
        (None, None) => None,
        (Some(full), None) => Some(full),
        (None, Some(named)) => Some(named),
        (Some(full), Some(named)) => Some(if full.score >= named.score {
                                              full
                                          }
                                          else {
                                              named
                                          }),
    }
}

/// The paths `pattern` matches, best first: by score, then by path, so the
/// order is total (the reference left ties to an unstable sort). At most
/// `FUZZY_MAX_RESULTS`. Each comes with its match.
pub(crate) fn rank<'a>(pattern: &str, paths: impl IntoIterator<Item = &'a str>)
                       -> Vec<(&'a str, ScoredMatch)> {
    let mut ranked: Vec<(&str, ScoredMatch)> =
        paths.into_iter()
             .filter_map(|path| score_file(pattern, path).map(|found| (path, found)))
             .collect();
    ranked.sort_by(|(left, a), (right, b)| b.score.cmp(&a.score).then_with(|| left.cmp(right)));
    ranked.truncate(FUZZY_MAX_RESULTS);
    ranked
}

#[cfg(test)]
mod tests;
