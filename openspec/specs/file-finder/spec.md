# file-finder Specification

## Purpose
The Find File overlay (⌘⇧F, Edit > Find File…) over the active agent's
folder: how it opens, how matches are ranked by the port of the reference
`FuzzyScorer`, how ranking stays off the typing path, the states it shows, and
what choosing a file does. The `@` lookup in `panel-file-mentions` shares the
same listing and scorer.

## Requirements

### Requirement: Opening the file finder

Edit > Find File… (⌘⇧F) SHALL open the file finder over the selected agent's
working folder in the focused workspace window, and SHALL be disabled when no
agent is selected. The finder SHALL hold keyboard focus while open, and SHALL
close on Esc, on a click outside it, or when another agent is selected.

#### Scenario: Open from the menu

- **WHEN** an agent is selected and the user presses ⌘⇧F
- **THEN** the file finder opens over that agent's folder with its query field
  focused

#### Scenario: Dismiss

- **WHEN** the finder is open and the user presses Esc
- **THEN** it closes and nothing is delivered to the agent

#### Scenario: No agent selected

- **WHEN** no agent is selected
- **THEN** Find File… is disabled

### Requirement: Ranking matches the reference scorer

Matches SHALL be scored with a port of the Swift reference's `FuzzyScorer`:
the same character bonuses, file-name bonuses and choice of the better of
the full-path and file-name scores, giving the same scores and matched
character offsets. Results SHALL be ordered by score, highest first, with
equal scores ordered by path, and SHALL be limited to the first 50.

#### Scenario: The snapshot

- **WHEN** each query in the change's `reference/snapshot.jsonl` is scored
  against `reference/corpus.txt`
- **THEN** the top 50 paths, their scores and their matched offsets equal the
  snapshot's

#### Scenario: An empty query

- **WHEN** the query is empty
- **THEN** no results are listed

### Requirement: Searching never blocks typing

The finder SHALL search the agent's cached file listing, gathering it off the
render path when the agent has none, and SHALL score off the render path,
150 ms after the query last changed. A result for a query that is no longer
current SHALL be discarded.

#### Scenario: Typing faster than scoring

- **WHEN** the user types `ab` and then `abc` before the scoring of `ab`
  finishes
- **THEN** only the results for `abc` are shown

#### Scenario: The listing is still being gathered

- **WHEN** the finder opens for an agent whose folder has not been listed
- **THEN** the finder says files are being gathered and accepts typing

### Requirement: The finder's states

The finder SHALL show its results with the matched characters marked, say
that the folder is too large when the listing reached its cap and the query
is empty, and say that nothing matched when a non-empty query has no result.

#### Scenario: No match

- **WHEN** the query matches no file
- **THEN** the finder says no files match

#### Scenario: A folder over the cap

- **WHEN** the agent's folder holds more files than the listing's cap and the
  query is empty
- **THEN** the finder says the folder has too many files to list

### Requirement: Choosing a file

↑ and ↓ SHALL move the selection within the results, stopping at either end,
and Return or a click SHALL choose a result. The chosen path, relative to the
agent's folder, SHALL be typed into a Terminal-mode agent's terminal without a
newline, or inserted at the caret of a Panel-mode agent's composer, and the
finder SHALL close. The path SHALL NOT be copied to the clipboard.

#### Scenario: Choose for a Terminal-mode agent

- **WHEN** the user chooses `src/lib.rs` for a Terminal-mode agent
- **THEN** `src/lib.rs` is typed at the agent's prompt, not submitted, and
  the finder closes

#### Scenario: Choose for a Panel-mode agent

- **WHEN** the user chooses `src/lib.rs` for a Panel-mode agent
- **THEN** `src/lib.rs` is inserted at the composer's caret and the finder
  closes
