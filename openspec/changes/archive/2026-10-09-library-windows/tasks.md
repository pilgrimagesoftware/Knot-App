# Tasks

## 1. Setup

- [x] 1.1 Create the worktree and feature branch `20-library-windows` from `develop`, and confirm the current branch is not `develop` or `main`

## 2. Implementation

- [x] 2.1 Add `library_window/` with `Library`, `LibraryWindow`, `open_library_window` and its render, and move the Personas, Prompts and Bench panes and their editors into it
- [x] 2.2 Add `WindowKey::{Personas, Prompts, Bench}` and open each window through `activate_or_open`
- [x] 2.3 Add the `OpenPersonas`, `OpenPrompts` and `OpenBench` actions and their Window menu group
- [x] 2.4 Remove the three tabs from the settings window
- [x] 2.5 Add the window titles and menu labels to the catalog

## 3. Tests

- [x] 3.1 Update the settings tab order, Window menu and catalog tests
- [x] 3.2 Cover opening a library window twice raising the first

## 4. Verification

- [x] 4.1 Run `make` in the worktree (fmt-check, size-check, lint, test, build) and confirm it passes
