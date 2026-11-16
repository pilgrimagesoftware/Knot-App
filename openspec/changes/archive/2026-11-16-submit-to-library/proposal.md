# Proposal

## Why

Knot imports personas and prompts from Knot-Library but offers no way back:
sharing one meant hand-writing the library's front matter and opening a pull
request outside Knot (#43).

## What Changes

- Each row in the Personas and Prompts windows gets a "Submit to
  Knot-Library…" action.
- A dialog asks for what the library needs and Knot does not keep: a title
  (starting as the name), a one-line description and optional tags, checked
  against Knot-Library's contribution rules before anything opens.
- `Prompt` gains an optional `description` (`#[serde(default)]`): filled by
  a library import, asked for and kept at submit time.
- A review step shows the exact file, editable, before anything leaves Knot.
  A prompt saved from a conversation holds the text as sent, so it can carry
  expanded local values; text the bug report's scrub would remove (home
  folder, email, credentials) is flagged.
- While the details are filled in, the library's index is read. An item
  whose id the library already has - imported, then edited - opens as an
  edit of that file, not a new item.
- A new item opens as a new file in Knot-Library on GitHub's new-file page.
  GitHub forks the library for a user who cannot write to it, and "Propose
  changes" opens the pull request. A file too long for a web address goes to
  the clipboard and the page opens empty.

## Decisions

- **The browser, not the GitHub CLI.** The new-file page handles the fork,
  the branch and the pull request for any signed-in user, and the user sees
  the exact file before it is proposed. Driving `gh repo fork` and `gh pr
  create` would need git operations against a fork Knot does not otherwise
  keep, for no gain in what is submitted.
- **The item keeps its id, and library ids are permanent.** Importing the
  item back later recognises it as held, and an edited copy of a library
  item is an update to that id's file. GitHub's edit page takes no content
  from its address, so an update's file goes by clipboard.
- **Review before publishing, always.** The user confirms the exact file;
  Knot never submits on the details alone.

## Impact

- `knot-library`: `contribution` - the file, its path and the library's
  rules for it.
- `knot-core`: `Prompt::description`, `set_prompt_description`; library
  import carries the description.
- `knot`: `library_window::submit` (dialog) and `submit::publish` (where the
  file goes), the bug report's `scrub` shared for the local-values check, the Personas and
  Prompts rows; `percent_encode` moves to `open_in` beside `open_url` so the
  bug report and this share it.
- Docs: `docs/site/latest/personas-prompts.md`.
