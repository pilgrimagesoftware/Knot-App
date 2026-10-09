# Design

## Context

- The three panes are `impl SettingsWindow` blocks
  (`settings_window/panes/{personas,prompts,bench}.rs`). Their only use of
  the window is `self.store`, the live agent store they count references
  against, and `cx.entity()`, which their editor windows hold weakly as
  their parent so a save can repaint the list.
- Single-instance windows that aren't settings or about go through
  `window_registry::activate_or_open` with a `WindowKey`. Command Center and
  the workspace manager both work this way.

## Decisions

- **One entity, three libraries.** `LibraryWindow { store, library }`, with
  `Library` as a closed enum. The panes move over almost unchanged:
  `impl SettingsWindow` becomes `impl LibraryWindow`. Three separate entity
  types would mean three copies of the window boilerplate for no behavioral
  difference.
- **The Window menu, not the app menu.** These are windows you go back to,
  like Command Center and Workspaces, not preferences. They're a group of
  their own because they manage content rather than navigate workspaces.
- **No key equivalents.** None of the three had a shortcut as a tab, and
  adding three global chords would be a separate decision that touches
  `keybindings`.
- **Keep the existing l10n keys.** The panes' copy keeps its
  `settings.personas.*`, `settings.prompts.*` and `settings.bench.*` keys.
  Renaming them would churn the catalog without changing a word the user
  sees. Only the tab-title keys go away.

## Risks / Trade-offs

- Someone used to Settings > Personas has to learn the new place. The menu
  items sit in the menu that already holds Knot's other global windows,
  which keeps them easy to find.
