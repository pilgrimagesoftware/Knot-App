# Tasks

## 1. Scorer

- [x] 1.1 Port `FuzzyScorer` (`is_pattern_in_word`, `score`, `score_file`)
      statement for statement, constants included
- [x] 1.2 Snapshot test over `fuzzy_score/corpus.txt` against
      `fuzzy_score/snapshot.jsonl`: top 50 per query, scores and indices
- [x] 1.3 Switch the `@` lookup to it and remove the greedy scorer;
      `panel-file-mentions` scenarios still pass
- [x] 1.4 Move `@` ranking off the render path: `panel_mention_matches`
      requests a search and reads the last landed result; nothing in render
      scores (#438 review)
- [x] 1.5 Highlight `@` rows by `char` offset; test with a multi-byte path

## 2. Search

- [x] 2.1 Background scoring with a generation number; a stale result is
      dropped; scoring stops early once superseded
- [x] 2.2 150 ms debounce; empty query lists nothing; top 50, ties by path
- [x] 2.3 Reuse the agent's cached listing, starting the gather when absent
- [x] 2.4 Results and listing reach a frame through `repaint_poll_tick`

## 3. Overlay

- [x] 3.1 Edit > Find File… (⌘⇧F), enabled with an agent selected
- [x] 3.2 Query field holding focus; gathering, too-many-files and
      no-matches states; matched characters marked
- [x] 3.3 ↑/↓ clamp, Return and click choose, Esc and outside click dismiss,
      selecting another agent closes it
- [x] 3.4 Chosen path typed into the terminal (Terminal mode) or inserted
      at the composer caret (Panel mode)
- [x] 3.5 l10n keys

## 4. Tests

- [x] 4.1 A stale scoring result is discarded, for the finder and for `@`
- [x] 4.5 Rendering the `@` popup over a large listing does not score: a
      test that render only reads the landed result
- [x] 4.2 Each delivery mode receives the path: Panel end to end into a real
      composer (created if missing); Terminal mode routed to the terminal, with
      the text written pinned as the bare path, no newline (the fixture has no
      PTY, so the write itself is `TerminalSession::send_text`)
- [x] 4.3 Keyboard selection and dismissal
- [x] 4.4 The menu item's enablement

## 5. Docs

- [x] 5.1 `docs/site/latest`: Find File
