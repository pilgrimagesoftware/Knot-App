# Tasks

## 1. Setup

- [x] 1.1 File the Knot-App issue for this change, rename the branch and worktree to `<issue>-knot-library-import`, and confirm `git branch --show-current` is not `develop` or `main`

## 2. `knot-library` crate

- [x] 2.1 Add `ureq` (rustls, no default features beyond TLS) and `sha2` to `[workspace.dependencies]`, and create `crates/knot-library` with `thiserror`, `serde` and `serde_json`; verify `cargo build -p knot-library`
- [x] 2.2 Model `Index` and `IndexItem` (camelCase serde) and `ItemKind { Persona, Prompt, Other(String) }`, rejecting any `format` other than 1 with a typed error; test against a copy of Knot-Library's current `index.json` as a fixture
- [x] 2.3 Implement `verify(bytes, &IndexItem)` (size, then lowercase hex SHA-256) and `split_item(bytes)` (drop the `---` front matter, trim, refuse an empty body); test a match, a size mismatch, a hash mismatch, a missing closing delimiter and an empty body
- [x] 2.4 Model `Location { GitHub { repo, branch }, Web { base }, Folder { path } }` as a tagged serde enum with form checks (`owner/repo`, `https://` only, existing folder) and each kind's index and item addresses (GitHub items pinned to the index commit, `HEAD` when no branch); test each kind's addresses and every refused form
- [x] 2.5 Implement `resolve(path)` refusing absolute paths and `..` components; test `../../.ssh/config`, `/etc/passwd` and a normal `personas/x.md`
- [x] 2.6 Define the `LibraryFetcher` trait (`fetch_index`, `fetch_item(&Index, &IndexItem)`), `HttpsFetcher` (ureq, 10 s timeout, `Knot/<version>` user agent), `FolderFetcher`, `fetcher_for(&Location)` and a fixture fetcher for tests; put the built-in location's constants in `knot-core` `consts.rs` next to `KNOT_REPO`

## 3. `knot-core`

- [ ] 3.1 Add `Settings::insert_persona(Persona)` and `Settings::insert_prompt(Prompt)`, refusing a duplicate id and blank fields; test both, including the uppercase built-in id against a lowercase library id
- [ ] 3.2 Add `import::library`: classify each index item as importable, already present, or deleted built-in, and import the selected verified items as `PersonaType::User` personas and prompts, reporting added, skipped and unreadable in an `ImportResult`
- [ ] 3.3 Add the saved-locations collection: `LibraryLocation { id, name, location }` in `library-locations.json` (path in `StorePaths`, atomic collection writer, absent document loads as empty), with add, rename and remove; test a round trip and a store that predates the document
- [ ] 3.4 Test the `data-import` scenarios: built-in already held, renamed copy still held, deleted built-in not revived, re-import after delete, unknown kind skipped, format 2 refused, one bad hash among good items

## 4. Import window

- [ ] 4.1 Add the Library section with the location picker (Knot-Library first, then saved locations by name), Personas and Prompts groups, title and description rows, and disabled rows for held items saying why
- [ ] 4.2 Read the chosen location's index on open, on picking another location (clearing the selection) and on refresh through `background_executor`, with a loading state, and an error state with retry for unreachable and unsupported-format; route the result through `repaint_poll_tick` or an entity update, never the render path
- [ ] 4.3 Add the add-location dialog (name, kind, the kind's field, the folder picker for folders, the trust note, form errors) and Rename and Remove (with confirmation), both disabled for Knot-Library; test adding a web address, refusing `acme` and `http://`, and removing a location keeping its imports
- [ ] 4.4 On import, fetch and verify only the selected items off-thread, write them through `settings_global`, and show the outcome with unreadable items named
- [ ] 4.5 Add catalog keys for every new string, and a test that they resolve

## 5. Library windows

- [ ] 5.1 Add "Import from Library…" to the Personas and Prompts windows' header rows, opening or raising the Import window
- [ ] 5.2 Test that the action opens the Import window, and that an imported persona appears in an open Personas window

## 6. Verification

- [ ] 6.1 Run `make` (fmt-check, size-check, lint, test, build) and confirm it passes
- [ ] 6.2 Run the app, open Import, and import one persona and one prompt from the live Knot-Library; save a local clone of Knot-Library as a folder location and confirm its built-ins show as already in Knot; then repeat with networking off and confirm the offline message
- [ ] 6.3 Check Knot-Library's `docs/your-own-library.md` (pilgrimagesoftware/Knot-Library#5) against what shipped, such as the menu path and the Add Location dialog's fields, and replace its "not shipped yet" status note with the Knot version that adds library locations
