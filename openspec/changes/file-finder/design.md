# Design

## The scorer

`FuzzyScorer.swift` is a port of VS Code's matcher (via
`D0ntPanic/code-fuzzy-match`): a dynamic programme over pattern × target
that keeps, for each target position, the best score and the length of the
run of consecutive matches ending there, then backtracks a match table for
the indices. `scoreFile` scores the full path and the file name, adds the
file-name bonuses (`filenameBonus`, an exact-stem bonus, a small bonus for
the first pattern character appearing early in the name) and keeps the
better of the two.

The port follows it statement for statement, constants included, rather
than re-deriving it. Two things are kept although they look like mistakes,
because changing them changes the ordering:

- The word-start branch checks `separators.contains(prevTChar)` after the
  branch above it has already handled every separator, so only the
  camelCase half can fire. The port keeps that behaviour.
- The backtrack walks the match table greedily from the end, which can
  report indices that are not the ones the best score came from. The
  marked characters follow the reference, not a corrected path.

Swift `Character`s are grapheme clusters and Rust `char`s are scalars. For
paths these only differ inside combining sequences, so the port works in
`char`s and its indices are `char` offsets. A renderer marking characters
must use the same unit.

### Snapshot

`reference/snapshot.jsonl` comes from the Swift scorer itself, compiled from
the last commit that had it, over 952 real paths and 14 queries. The port's
test loads it and asserts each query's top 50, score and indices included.
The ordering is the reference's except for ties, which it left to an
unstable sort; the snapshot and the port both break them by path.

### Cost

The DP is O(pattern × path) per file. At 50,000 files that is too much for
a keystroke on the main thread, which is why scoring runs on a background
task (below). The `isPatternInWord` pre-check, also ported, rejects most
paths in one linear pass before the DP runs.

## Sharing with `@` mentions

The `@` lookup and the finder search the same list for the same purpose, so
they use one scorer. Keeping the greedy one for `@` would leave two
rankings for one question. The `@` lookup's spec only asks for subsequence
matching, name over directory and runs over scattered characters, which the
real scorer satisfies; its scenario `kgs` → `crates/knot-git/src/lib.rs`
still holds (rank 15 of 50 in the snapshot).

The listing is shared too: the finder asks `panel_commands::files` for the
agent's cached `FolderFiles`, starting the gather if the agent has none.
That listing already follows folder changes, so the finder is never staler
than an `@` lookup.

## Off the render path, and cancelling stale work

- The listing is gathered in `spawn_blocking`, as it is for `@` today, and
  reported through the poll's existing `drain_mention_listings`.
- Each query change starts a 150 ms timer; when it fires, scoring runs on a
  background task tagged with a generation number. The result lands in a
  slot the repaint poll drains. A result whose generation is not the
  current one is dropped, which is how a query typed faster than scoring
  finishes is handled. Scoring also checks the generation between batches
  of paths and stops early once superseded.
- Nothing in render touches the filesystem or scores: it draws the last
  landed result.

Both new off-thread results (scored matches, and the listing the finder
waits on) go into `repaint_poll_tick`'s chain, per
`.claude/rules/rust-structure.md`.

## Delivering the chosen path

- Terminal mode: the path is written to the agent's terminal as typed
  input, with no newline, as the reference's `sendText` did. Paths with
  spaces are written as they are, as the reference did.
- Panel mode: inserted into the composer at the caret. The finder closes
  and focus returns to the composer, or to the terminal.

## Focus

The reference's text field fought the terminal for first responder. In the
port the overlay owns a focus handle while it is open, and the terminal's
key handling sees nothing until it closes, like the panel's find bar.
