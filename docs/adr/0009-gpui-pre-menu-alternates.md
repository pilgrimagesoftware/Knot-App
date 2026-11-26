---
id: ADR-0009
title: Fork gpui-pre for native alternate menu items
status: accepted
date: 2026-11-25
tags: [ui, dependencies, gpui]
---

# ADR-0009: Fork gpui-pre for native alternate menu items

## Context

Issue #475 folds each agent action and its variant into one menu line that
changes while Shift is held: Restart Agent becomes Restart with New
Conversation, and Deactivate becomes Deactivate with New Session. Knot's
context menus do this themselves (`menus::alternate_line`). The menu bar's
Agents menu can't: the maintainer asked for native alternates there in the
same change, and macOS supports them (`NSMenuItem.alternate`), but gpui has no
way to request one.

Checked against the pinned crates:

- `gpui-pre` 0.3.7's `MenuItem::Action` carries `name`, `action`,
  `os_action`, `checked` and `disabled`. It has no alternate flag and no
  modifier mask of its own.
- `gpui-pre-macos` 0.3.7's `create_menu_item` builds each `NSMenuItem` from
  the item's key binding and never calls `setAlternate:`.

`gpui-pre` is published by a single crates.io owner as a snapshot of
`zed-industries/zed` at `f126950` (its description says so; its `repository`
points at all of Zed). There is no repository for the snapshot and no `0.3.7`
tag to branch from.

## Decision

We consume forked `gpui-pre` and `gpui-pre-macos` through `[patch.crates-io]`,
from branch `knot/menu-alternates-0.3.7` on `pilgrimagesoftware/gpui-pre`.

The fork's `main` holds one commit, `c9766ab`, that imports both crates'
published 0.3.7 sources unchanged (tagged `v0.3.7-import`). The branch adds
one commit on top, `c0496ea`, "feat: native alternate menu items":

- `MenuItem::Action` and `OwnedMenuItem::Action` gain
  `alternate: Option<Modifiers>`, set by the builder
  `MenuItem::alternate(modifiers)` and read by `alternate_modifiers()`. The
  item becomes an alternate of the item before it, shown in that item's
  place while `modifiers` are held.
- `gpui-pre-macos` calls `setAlternate: YES` on such an item. For an item
  without a key binding, it also sets the key equivalent modifier mask from
  `modifiers`, because AppKit pairs alternates by key equivalent and tells
  them apart by mask. An item with a binding keeps the binding's mask: ⌘R
  and ⇧⌘R already differ.
- Other platforms ignore the field. Knot shows only the primary there
  (`agent_menu::NATIVE_MENU_ALTERNATES`).

The diff against the import is two files, +97/-1. Knot's own tests cover the
menu it builds (`tests::menu_alternates`): the alternate follows its primary
with `Modifiers::shift()` on macOS, and is left out elsewhere. The fork's unit
test for the builder cannot run in the snapshot, because the published crate's
test build needs font assets from Zed's repository.

## Options considered

- **Fork and add a field to `MenuItem::Action`** (chosen). It is the smallest
  change that expresses an alternate, and it matches how AppKit models one.
  The cost is source compatibility: code that builds `MenuItem::Action` with a
  struct literal must add `alternate: None`. Knot has three such sites, and no
  crate in our build has others (`gpui-component` and `gpui-pre-windows`
  match with `..`).
- **A new `MenuItem::Alternate` variant.** It would break every exhaustive
  `match` on `MenuItem`, including gpui-component's native menu, which is a
  wider break than a field.
- **List each variant as its own item (the proposal's option A).** It needs
  no fork, but it is the line-per-variant growth #475 exists to stop, and the
  maintainer chose native alternates.
- **Wait for upstream.** It could take an unknown number of gpui releases,
  and Knot pins gpui-kit to a release that Fernrohr shares.

## Consequences

- `cargo tree` must show one `gpui-pre` and one `gpui-pre-macos`, both from
  the fork. Two would be two incompatible `gpui` type sets.
- Deleting or force-pushing `knot/menu-alternates-0.3.7` breaks CI. The branch
  only ever gains commits.
- Bumping gpui-kit, and with it gpui-pre, means re-cutting the fork. Import
  the new published sources as a new commit on `main`, then cherry-pick
  `c0496ea` onto a `knot/menu-alternates-<version>` branch.
- On Windows and Linux the Shift variants stay reachable from the context
  menus. (Restart with New Conversation's binding is a literal `cmd-shift-r`,
  so off macOS it is Super-based; that predates this change.)

### Upstream PR (drafted, not opened)

To `zed-industries/zed`, touching `crates/gpui` and the macOS platform:

> **gpui: support alternate menu items**
>
> macOS menus can show an alternate item in place of the item before it
> while modifier keys are held (`NSMenuItem.alternate`). Examples are Finder's
> Close Window / Close All, and Save / Save All. gpui can't express one today.
>
> This adds `alternate: Option<Modifiers>` to `MenuItem::Action` (and
> `OwnedMenuItem::Action`), a `MenuItem::alternate(modifiers)` builder and an
> `alternate_modifiers()` getter. On macOS the item is marked with
> `setAlternate:`. An item without a key binding takes `modifiers` as its key
> equivalent modifier mask, so AppKit pairs it with the previous item; an
> item with a binding keeps the binding's mask. Other platforms ignore the
> field.
>
> Breaking: code that builds `MenuItem::Action` with a struct literal must add
> `alternate: None`.

### Exit condition

When a gpui-pre release that Knot can pin (that is, one gpui-kit moves to)
supports alternate menu items:

1. Delete the `gpui-pre` and `gpui-pre-macos` entries from
   `[patch.crates-io]` in the root `Cargo.toml`.
2. Adapt `agent_menu::agent_menu_item` to the upstream API if it differs.
3. Verify that `make` passes and that `cargo tree` shows the registry crates.
4. Archive `pilgrimagesoftware/gpui-pre` and supersede this record.
