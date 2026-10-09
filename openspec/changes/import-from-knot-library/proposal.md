# Proposal

## Why

Knot-Library (`pilgrimagesoftware/Knot-Library`) now publishes personas and
prompts in a fixed format. Every item is a Markdown file with a front-matter
envelope keyed by a permanent UUID, and the repository regenerates a single
`index.json` after every merge (Knot-Library#1). That change left one thing
undone: Knot-App has no way to bring any of it in. Knot-Library's design
names this change as the consumer ("Knot-App's import change"), and its
`content-format` spec already says how an item maps onto Knot's records: a
persona's `title` becomes `Persona.name` and its body becomes
`Persona.instructions`, a prompt's `title` becomes `Prompt.name` and its body
becomes `Prompt.text`, and in both cases the item's `id` becomes the record's
id.

Today, getting a library persona into Knot means opening the file on GitHub
and pasting its body into the persona editor by hand. That loses the item's
identity, so the next copy-paste makes a duplicate.

## What Changes

- **A third import source.** The Import window gains a Knot-Library section
  beside the Claude subagent and Skwad sections. It lists the library's
  personas and prompts, each with its title and one-line description, for the
  user to pick from. It runs on the same review-then-confirm flow as the
  other sources.
- **Library items keep their identity.** An imported persona or prompt takes
  the library item's UUID. "Already held" for this source means Knot has a
  record with that id, so importing twice adds nothing. That also covers the
  seven built-in personas, which the library publishes under the same UUIDs
  Knot ships them with.
- **Downloads are verified.** Knot fetches `index.json` from the library's
  default branch, then fetches each selected item pinned to the index's
  `commit`. It rejects any item whose bytes don't match the index's `sha256`
  and `size`, which is what Knot-Library's `content-index` spec asks of a
  client.
- **Reachable from the library windows.** The Personas and Prompts windows
  get an "Import from Library…" action that opens the Import window.
- **Fails in place.** If the library can't be reached, or publishes an index
  format this Knot doesn't know, the Knot-Library section says so and offers
  to retry. The other sources keep working.

## Capabilities

### New Capabilities

None. This extends the existing import.

### Modified Capabilities

- `data-import`: a Knot-Library source, the identity rule for its items, and
  download verification.
- `import-ui`: the Knot-Library section, including what it shows while
  fetching and when the fetch fails.
- `library-windows`: the "Import from Library…" action on the Personas and
  Prompts windows.

## Impact

- **New crate `knot-library`**: the `index.json` model, splitting an item
  into front matter and body, the sha256 check, and a `LibraryFetcher` trait
  with an HTTPS implementation. It has no knowledge of Knot's settings.
- **`knot-core`**: `import::library`, which maps fetched items onto
  `Persona` and `Prompt` records, and `Settings` inserts that take a
  caller-supplied UUID. Today `add_persona` and `add_prompt` always mint a
  fresh one. Also the library URL constants in `consts.rs`.
- **`knot`**: the Import window's new section, and the action on the
  Personas and Prompts windows.
- **New dependencies** (workspace): `ureq` (blocking HTTPS, rustls) and
  `sha2`. Nothing in the workspace makes outbound HTTP requests today.
- **Network access**: this is the first feature in Knot that reaches the
  internet without the user running a tool themselves. It only happens while
  the Import window is open, and it only reads from
  `raw.githubusercontent.com`.
- **No data-model change.** `Persona` and `Prompt` gain no fields, and nothing
  persisted changes shape.
- **Out of scope**: bench entries (Knot-Library has no `bench` kind yet),
  updating an already-imported item when the library changes it, submitting
  items to the library (Knot-App#43), and making Restore Defaults read the
  library.
