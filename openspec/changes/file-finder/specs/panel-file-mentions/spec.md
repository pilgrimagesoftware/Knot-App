# Spec Delta

## MODIFIED Requirements

### Requirement: Matching is fuzzy and ranked

The lookup SHALL match the typed text against each file's path as a
subsequence rather than a contiguous substring, so `kgs` reaches
`knot-git/src`. Matches SHALL be ordered best-first, and the ordering SHALL
prefer matches on the file's name over matches on its parent directories, and
consecutive matched characters over scattered ones.

The matched characters SHALL be marked in each listed row, so the user can
see why a row is a match.

The ranking SHALL be the one `file-finder` specifies, so the lookup and the
finder order the same files the same way.

#### Scenario: A subsequence matches

- **WHEN** the user types `@kgs`
- **THEN** `crates/knot-git/src/lib.rs` is among the matches

#### Scenario: Name matches outrank directory matches

- **WHEN** the typed text matches one file's name and another file's parent
  directory equally well
- **THEN** the file matched on its name is listed first

#### Scenario: Matched characters are marked

- **WHEN** the lookup lists a match
- **THEN** the characters that matched the typed text are distinguished
  within the row's path

#### Scenario: The lookup and the finder agree

- **WHEN** the user types `lib` after `@`, and then types `lib` into the file
  finder for the same agent
- **THEN** both list the same files in the same order
