---
title: Keybindings
version: latest
permalink: /latest/reference/keybindings/
---

# Keybindings

Knot's keyboard shortcuts, grouped by where they act, with the default each one starts from. Every shortcut in this reference can be changed in the settings window's Keyboard tab.

## Application and window

| Key | Action |
| --- | --- |
| ⌥⌘1 … ⌥⌘9 | Select workspace 1–9: bring that workspace's window to the front and focus it, if it is open. |
| ⌥⌘0 | Open the Command Center, or raise it if it is already open. |

Selecting a workspace works from any Knot window, and even with no Knot window focused. It only raises a window that is already open; it never opens one. The Command Center shortcut is the key equivalent shown beside Window > Command Center.

## Workspace and agent navigation

| Key | Action |
| --- | --- |
| ⌘1 … ⌘9 | Select agent 1–9 in the focused workspace window. |

Selecting an agent acts only on the focused workspace window, and does nothing in any other window. It selects the Nth agent in that window's sidebar, top to bottom, and shows it. Showing an agent leaves a Dashboard or Pull Requests panel and returns to the agent view.

## Agent input and conversation

| Key | Action |
| --- | --- |
| ⌘L | Focus the selected agent's input. |
| ⌃⌘↓ | Jump to the bottom of the conversation and follow new output. |
| ⌥⌘I | Insert a library prompt into the focused composer. |

Focus agent input moves keyboard focus to the selected agent's input in the focused workspace window: the composer for a panel-mode agent, the terminal for a terminal-mode agent. It works from anywhere in the window, including a search field or the git panel, and leaves a Dashboard or Pull Requests panel first.

Jump to bottom scrolls the selected agent's conversation to its latest output and resumes following new output, the same as the conversation's "Scroll to latest" control. It applies to a panel-mode agent; a terminal-mode pane always shows the bottom of its output, so the shortcut does nothing there, and it does nothing while a Dashboard or Pull Requests panel is showing.

Insert prompt opens the focused panel composer's Insert Prompt list ([Prompts in the panel]({{ '/latest/personas-prompts/' | relative_url }})). It acts only while a composer has focus; anywhere else, such as the agent list, it does nothing.

With no agent selected, or outside a workspace window, these shortcuts do nothing.

## Panels

| Key | Action |
| --- | --- |
| ⌥⌘O | Toggle Dashboard. |
| ⌥⌘P | Toggle Pull Requests. |
| ⌥⌘A | Toggle the selected agent's artifact panel. |

Each toggle behaves like the matching workspace sidebar row: it shows its panel, and hides it again if it is already showing, which returns the window to the agent view. Switching from one panel to another replaces the first. Outside a workspace window these do nothing.

## Customizing shortcuts

Open the settings window's Keyboard tab to view and change every shortcut in this reference. Each shortcut is listed with its current binding, a control to reset it to its default, and a control to reset all of them.

- Numbered families always keep the digits 1 through 9; only their modifier is configurable, from any combination of ⌃, ⌥, ⇧ and ⌘.
- The other shortcuts are single chords. Their recorder takes the next chord you press as the new binding; pressing Escape cancels the recording and leaves the binding unchanged.
- An accepted change is saved immediately and takes effect in every open window without a restart. The previous chord stops triggering the shortcut.

## Constraints

- A single-chord binding, and a numbered family's modifier, must include ⌘, ⌃ or ⌥. A chord with none of them would take a key away from typing and is rejected.
- A binding that duplicates a chord already used by another Knot shortcut is rejected, whether it conflicts with another shortcut in this reference or a fixed shortcut such as ⌘Q, ⌘W, or the Agents menu's. A rejected binding leaves the previous one in effect, and the Keyboard tab names the conflicting shortcut.
- Agent selection uses the plain ⌘ modifier while workspace selection uses ⌥⌘, because moving between the agents of one workspace is more frequent than moving between workspaces. ⌘0 opens the workspace manager, so the Command Center uses ⌥⌘0.
- Jump to bottom uses ⌃⌘↓ rather than the platform's ⌘↓, because the composer holds ⌘↓, ⇧⌘↓ and ⌥⌘↓ for its own caret movement and takes them first while it is focused.
- ⇧⌘A is reserved for the permission prompt's fixed Allow, so it is not available as a binding.
- ⌘F is reserved for finding text in a conversation ([The conversation panel]({{ '/latest/agents/' | relative_url }})), so it is not available as a binding.
- A stored binding that breaks these rules, or that does not parse, is ignored at launch in favour of its default, so a hand-edited or older preferences document cannot leave Knot without its shortcuts.

## Source contracts

- `openspec/specs/keybindings/spec.md`
