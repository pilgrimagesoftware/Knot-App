# Design

## Context

- **Knot-Library's contract** (`Knot-Library/openspec/specs`):
  - `content-index`: `index.json` lives at
    `https://raw.githubusercontent.com/pilgrimagesoftware/Knot-Library/master/index.json`
    and has `format` (currently `1`), `commit`, `generatedAt`, `kinds` and
    `items`. Each item has `kind`, `id`, `slug`, `title`, `description`,
    `tags`, `authors`, `path`, `size` and `sha256`. A client fetches an item
    at `.../<commit>/<path>` and SHALL reject bytes whose SHA-256 differs.
  - `content-format`: front matter between `---` lines, then a body that is
    trimmed and non-empty. Ids are permanent and unique across kinds. A
    client SHALL skip kinds it doesn't recognise.
  - The default branch is `master`; the schema `$id`s already name it, which
    settles that open question in Knot-Library's design. The repository is
    public and licensed CC BY 4.0.
- **Knot's import** (`data-import`, `import-ui`) is additive and idempotent,
  works review-then-confirm, and has one section per source in the Import
  window (`crates/knot/src/import_window/`). The subagent import keys
  personas by **name** (`import/personas.rs`, `holds_persona_named`), because
  a subagent definition has no id of its own.
- **Knot's records**: `Persona { id, name, instructions, persona_type, state }`
  soft-deletes System personas (`state: Deleted`) and hard-deletes User ones.
  `Prompt { id, name, text }` has no type or tombstone. `add_persona` and
  `add_prompt` always mint a new UUID.
- **Networking**: nothing in the workspace fetches over HTTP. The only
  network-shaped work runs through subprocesses (`gh` in `knot-forge`, `npm`
  in `knot-terminal`). Blocking work reaches the UI through
  `cx.background_executor().spawn(..)` followed by an entity update
  (`bug_report/dialog.rs`).

## Goals / Non-Goals

**Goals:**

- Import library personas and prompts with their library ids, so a re-import
  is recognised by identity, not by name.
- Meet Knot-Library's client obligations: pin item fetches to the index
  commit, verify sha256 and size, and skip unknown kinds.
- Reuse the Import window's review-then-confirm flow rather than inventing a
  second import UI.
- Keep fetching, parsing and verifying testable without a network.

**Non-Goals:**

- Bench entries. Knot-Library has no `bench` kind. When it gets one, this
  source gains a section; nothing here needs to change shape.
- Updating an imported item when the library changes it. That would break
  the additive rule, so it needs its own change (see Open Questions).
- Submitting to the library (Knot-App#43).
- Caching the index across launches, or browsing offline.
- Searching or filtering by tag. With ten items, a list is enough. The index
  carries tags, so a filter can come later without a format change.

## Decisions

### A source in the Import window, not a browser of its own

The library is listed as a third section of the Import window, "Knot-Library",
with a sub-list each for Personas and Prompts. Each row shows the item's
title and description and has a checkbox. The Personas and Prompts windows'
"Import from Library…" action opens the Import window.

- *Rejected: a separate library browser window.* It would duplicate the
  selection, outcome and refresh behavior `import-ui` already specifies, and
  the user would have two places to import from that behave slightly
  differently.

### Identity is the library id

For this source, a record counts as "already held" when Knot holds a
persona or prompt with the item's id, in any state. A held item is listed,
but it's not selectable, and its row says why:

- **"Already in Knot"**: a live record has that id.
- **"Deleted in Knot"**: the only record is a soft-deleted built-in. The row
  points to the Personas window's Restore Defaults. Reviving the tombstone
  would overwrite a record, which `data-import` forbids.

A new persona from the library is `PersonaType::User`, like a subagent
import: editable, and hard-deleted when removed. After a user deletes it,
the id is free again and the item becomes importable again. That is the
behavior you'd expect of something you can always fetch again.

The seven built-in personas match by id (Knot ships them uppercase, the
library lowercase; `Uuid` compares parsed values, so case doesn't matter).
They show as "Already in Knot", not as duplicates.

- *Rejected: match by name, like subagent imports.* Library items have
  permanent ids precisely so a rename doesn't orphan an imported copy, and
  matching by name would bring that problem back.
- *Rejected: record the origin on the record (`library_id`, `sha256`).*
  That's a data-model and persistence change. It only pays off when there's
  an update flow that needs to tell a local edit from a library change, and
  updating is a non-goal here.

### `Settings` inserts with a caller-supplied id

`knot-core` gains `Settings::insert_persona(Persona)` and
`Settings::insert_prompt(Prompt)`. Both refuse an id that is already
present, and blank fields, the same way the existing adders do. The
import maps an item onto a record and calls these, and they persist through
the same `write_persisting` path as any other edit.

### A `knot-library` crate for the remote side

`knot-library` holds:

- `Index` and `IndexItem` (serde, camelCase), with `format` checked against
  a supported version (`1`).
- `ItemKind`: `Persona` and `Prompt`, plus `Other(String)` for kinds this
  Knot skips.
- `split_item(bytes) -> Result<ItemBody>`: drops the front matter and
  returns the trimmed body. The title and id come from the index entry, not
  the front matter, so no YAML parser is needed. The index is generated from
  the same files, and the sha256 check proves the bytes are the ones it
  describes.
- `verify(bytes, &IndexItem)`: compares the size, then the lowercase hex
  SHA-256 (`sha2`).
- A `LibraryFetcher` trait: `fetch_index()` and
  `fetch_item(commit, path)`. `HttpsFetcher` implements it with `ureq`
  (rustls, 10 s timeout, a `Knot/<version>` user agent). Tests use a fixture
  fetcher.

The crate knows nothing about `Settings`. `knot-core::import::library`
turns verified items into records and applies the identity rule.

- *Rejected: `curl` as a subprocess, like `gh` and `npm`.* It would work on
  macOS, but every failure would mean parsing stderr, and verification needs
  the exact bytes either way. A small blocking client is easier to test
  through the trait and doesn't depend on a binary being on `PATH`.
- *Rejected: `reqwest`.* It's only in `knot-mcp`, without TLS, and its async
  API needs a tokio runtime, which GPUI's executor doesn't provide. `ureq`
  is blocking, so it runs as-is on `background_executor`.

### When fetching happens

The section fetches `index.json` when the Import window opens and again on
the window's existing refresh action. Never on the render path. While the
fetch is in flight, the section shows that it's loading. Item files are
fetched only when the user confirms the import, and only for the selected
items, each pinned to the index's `commit`. An item that fails to download
or verify is reported as unreadable by title, and the others still import
(`data-import` - partial failure).

If the index fetch fails, or `format` isn't supported, the section shows one
message saying why ("Couldn't reach Knot-Library" or "This library needs a
newer Knot") with a retry, and the other sections are unaffected.

## Risks / Trade-offs

- **First outbound network access.** Opening the Import window now makes a
  request to GitHub. Mitigation: it only happens in that window, it's
  read-only and anonymous, and it's stated in the spec. The proposal names
  it so it's a decision, not a side effect.
- **GitHub raw caching.** The default-branch `index.json` can lag a merge by
  a few minutes. That's harmless: the index names its own commit, so it is
  always internally consistent.
- **A local edit to an imported item isn't distinguishable from the
  library's version.** That's fine while there's no update flow, and it's
  why recording the origin is deferred.
- **New dependencies.** `ureq` and `sha2` are small and widely used, and
  `rustls` is already in the lockfile through other crates.

## Open Questions

- **Updating imported items.** When the library changes an item Knot already
  holds, should the section offer "Update"? That needs Knot to remember what
  it imported (`sha256`), to tell a local edit apart, and an exception to the
  additive rule. Left for a follow-up.
- **Restore Defaults and the library.** Knot-Library's design deferred this
  to the import change. This change answers no: Restore Defaults keeps using
  the copies shipped in the binary, so it works offline and can't be changed
  by a library merge.
