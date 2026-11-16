# Tasks

## 1. Scorer

- [ ] 1.1 Port `FuzzyScorer` (`is_pattern_in_word`, `score`, `score_file`)
      statement for statement, constants included
- [ ] 1.2 Snapshot test over `reference/corpus.txt` against
      `reference/snapshot.jsonl`: top 50 per query, scores and indices
- [ ] 1.3 Switch the `@` lookup to it and remove the greedy scorer;
      `panel-file-mentions` scenarios still pass
- [ ] 1.4 Move `@` ranking off the render path: `panel_mention_matches`
      requests a search and reads the last landed result; nothing in render
      scores (#438 review)
- [ ] 1.5 Highlight `@` rows by `char` offset; test with a multi-byte path

## 2. Search

- [ ] 2.1 Background scoring with a generation number; a stale result is
      dropped; scoring stops early once superseded
- [ ] 2.2 150 ms debounce; empty query lists nothing; top 50, ties by path
- [ ] 2.3 Reuse the agent's cached listing, starting the gather when absent
- [ ] 2.4 Results and listing reach a frame through `repaint_poll_tick`

## 3. Overlay

- [ ] 3.1 Edit > Find File… (⌘⇧F), enabled with an agent selected
- [ ] 3.2 Query field holding focus; gathering, too-many-files and
      no-matches states; matched characters marked
- [ ] 3.3 ↑/↓ clamp, Return and click choose, Esc and outside click dismiss,
      selecting another agent closes it
- [ ] 3.4 Chosen path typed into the terminal (Terminal mode) or inserted
      at the composer caret (Panel mode)
- [ ] 3.5 l10n keys

## 4. Tests

- [ ] 4.1 A stale scoring result is discarded, for the finder and for `@`
- [ ] 4.5 Rendering the `@` popup over a large listing does not score: a
      test that render only reads the landed result
- [ ] 4.2 Each delivery mode receives the path, with no newline in Terminal
      mode
- [ ] 4.3 Keyboard selection and dismissal
- [ ] 4.4 The menu item's enablement

## 5. Docs

- [ ] 5.1 `docs/site/latest`: Find File
