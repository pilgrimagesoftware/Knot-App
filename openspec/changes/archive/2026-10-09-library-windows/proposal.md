# Proposal

## Why

The settings window has ten tabs, and three of them are not settings. The
Personas, Prompts and Bench tabs each manage a library of things: personas
to assign to agents, prompts to send or launch with, and agent templates to
deploy. They are lists of content you add to and edit over time, each with
its own editor window. As tabs they make the strip crowded and hard to scan,
they force the settings window to change height whenever you move between a
library and a preference, and you can't see two libraries at once (for
example a bench entry's startup prompt beside the Prompts list).

Issue: pilgrimagesoftware/Knot-App#20.

## What Changes

- **Three new windows.** Personas, Prompts and Bench each open in their own
  window. Each window draws the same pane the tab used to draw, with the same
  list, row actions and editor windows.
- **Opened from the Window menu.** A second group of openers (Personas,
  Prompts, Bench) follows Command Center and Workspaces. Like those two, each
  window is a single window: choosing the item again brings it to the front.
- **Settings loses the three tabs.** The settings window keeps General,
  Coding, Autopilot, Voice, MCP, Appearance and Keyboard, in that order.

## Capabilities

### New Capabilities

- `library-windows`: the Personas, Prompts and Bench windows. Their contents
  are the requirements that `settings-ui` used to state for the three tabs.

### Modified Capabilities

- `settings-ui`: the tab list shrinks to seven. The Personas, Prompts and
  Bench tab requirements move to `library-windows`.
- `app-menu`: the Window menu gains the three library openers.
- `window-lifecycle`: each library window is a single window.

## Impact

- `crates/knot/src/library_window/` (new): the `LibraryWindow` entity, its
  render, the three panes and their editors. They move there from
  `settings_window/`.
- `crates/knot/src/settings_window/`: the three tabs are removed.
- `crates/knot/src/window_registry.rs`: three new `WindowKey` variants.
- `crates/knot/src/app_bootstrap.rs`, `window_menu.rs`, `window_options.rs`:
  the actions, menu items and window options.
- `crates/knot-core/locales/en.yml`: titles and menu labels.
- No data-model or persistence changes. Personas, prompts and bench entries
  are stored exactly as before.
