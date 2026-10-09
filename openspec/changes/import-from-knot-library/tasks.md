# Tasks

## 1. Setup

- [ ] 1.1 File the Knot-App issue for this change, rename the branch and worktree to `<issue>-knot-library-import`, and confirm `git branch --show-current` is not `develop` or `main`

## 2. `knot-library` crate

- [ ] 2.1 Add `ureq` (rustls, no default features beyond TLS) and `sha2` to `[workspace.dependencies]`, and create `crates/knot-library` with `thiserror`, `serde` and `serde_json`; verify `cargo build -p knot-library`
- [ ] 2.2 Model `Index` and `IndexItem` (camelCase serde) and `ItemKind { Persona, Prompt, Other(String) }`, rejecting any `format` other than 1 with a typed error; test against a copy of Knot-Library's current `index.json` as a fixture
- [ ] 2.3 Implement `verify(bytes, &IndexItem)` (size, then lowercase hex SHA-256) and `split_item(bytes)` (drop the `---` front matter, trim, refuse an empty body); test a match, a size mismatch, a hash mismatch, a missing closing delimiter and an empty body
- [ ] 2.4 Define the `LibraryFetcher` trait (`fetch_index`, `fetch_item(commit, path)`), the `HttpsFetcher` implementation (10 s timeout, `Knot/<version>` user agent) and a fixture fetcher for tests; put the URLs in `knot-core` `consts.rs` next to `KNOT_REPO`

## 3. `knot-core`

- [ ] 3.1 Add `Settings::insert_persona(Persona)` and `Settings::insert_prompt(Prompt)`, refusing a duplicate id and blank fields; test both, including the uppercase built-in id against a lowercase library id
- [ ] 3.2 Add `import::library`: classify each index item as importable, already present, or deleted built-in, and import the selected verified items as `PersonaType::User` personas and prompts, reporting added, skipped and unreadable in an `ImportResult`
- [ ] 3.3 Test the `data-import` scenarios: built-in already held, renamed copy still held, deleted built-in not revived, re-import after delete, unknown kind skipped, format 2 refused, one bad hash among good items

## 4. Import window

- [ ] 4.1 Add the Knot-Library section with Personas and Prompts groups, title and description rows, and disabled rows for held items saying why
- [ ] 4.2 Fetch the index on open and on refresh through `background_executor`, with a loading state, and an error state with retry for unreachable and unsupported-format; route the result through `repaint_poll_tick` or an entity update, never the render path
- [ ] 4.3 On import, fetch and verify only the selected items off-thread, write them through `settings_global`, and show the outcome with unreadable items named
- [ ] 4.4 Add catalog keys for every new string, and a test that they resolve

## 5. Library windows

- [ ] 5.1 Add "Import from Library…" to the Personas and Prompts windows' header rows, opening or raising the Import window
- [ ] 5.2 Test that the action opens the Import window, and that an imported persona appears in an open Personas window

## 6. Verification

- [ ] 6.1 Run `make` (fmt-check, size-check, lint, test, build) and confirm it passes
- [ ] 6.2 Run the app, open Import, and import one persona and one prompt from the live library; then repeat with networking off and confirm the offline message
