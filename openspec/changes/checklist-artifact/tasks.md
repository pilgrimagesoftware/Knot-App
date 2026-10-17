# Tasks

## 1. Model

- [x] 1.1 Add `Checklist`, `ChecklistItem` and `ChecklistItemKind` to
      `knot-agents`, with republish, tick, answer and summary; verify with unit
      tests
- [x] 1.2 Add the `Agent::checklist` slot and the store writes (set, restore,
      clear, edit); verify the workspace builds

## 2. MCP tools

- [x] 2.1 Add `show-checklist` and `get-checklist` to the catalog and dispatch;
      verify with handler tests and the catalog test
- [x] 2.2 Validate items (ids, text, uniqueness, kind) with errors naming the
      item; verify with tests

## 3. Artifact panel

- [x] 3.1 Carry the checklist in `ArtifactSnapshot` and count it as an open
      artifact; verify a checklist alone opens the panel
- [x] 3.2 Draw the checklist section with checkboxes, answer fields and Send;
      verify the fields are created with the panel
- [x] 3.3 Close and reopen the checklist with its answers; verify with tests
- [x] 3.4 Deliver Send to Panel- and Terminal-mode agents and mark it sent;
      verify with a test
- [x] 3.5 Localize the section's copy; verify the keys resolve

## 4. Review fixes (#307)

- [x] 4.1 Key answer fields on `Checklist::revision`, drop them on close, and
      re-seed an unfocused field from the store; verify with a window test of
      the close-then-republish sequence
- [x] 4.2 Mark a checklist sent only once delivered; verify both outcomes
- [x] 4.3 Give `show-checklist`'s `items` an element schema and clean its
      descriptions; verify with catalog tests

## 5. Docs

- [x] 5.1 List the two tools in `docs/site/latest/coordination.md`
