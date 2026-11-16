# Design

## No file checks when building the menu

The menu bar's Agents menu is rebuilt from a snapshot on the repaint poll, and
the row's context menu is built from the same facts. Checking which files
exist there would put file I/O on the paint loop, which
`.claude/rules/rust-structure.md` forbids. So the table is pure: an entry
names the paths its file may be at, and the first that exists is chosen only
when the user clicks. That also covers OpenCode, which accepts either
`opencode.jsonc` or `opencode.json`: one entry, two candidates.

The cost is an entry for a file the user has never created, which does
nothing when clicked. That matches Open In's handling of a missing
application, and the user-level files exist for any agent that has run.

## Default text editor, not the default app

`open -t` opens the file in the user's default text editor. A plain `open`
would hand `.json` to whatever owns the extension, which on many Macs is
Xcode. It runs on its own thread, so a slow launch doesn't block the menu.

## Agent types

`agent_type` is an open vocabulary with one roster
(`knot_core::agent_type::ALL`). The table lives in `knot` beside the menus,
not in the roster, so it keeps a test that every roster entry other than the
shell and the custom slots has files listed.
