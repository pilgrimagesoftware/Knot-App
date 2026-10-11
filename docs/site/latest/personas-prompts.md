---
title: Personas and prompts
version: latest
permalink: /latest/personas-prompts/
---

# Personas and prompts

A persona is standing instruction text that shapes how an agent works. A library
prompt is text written once and reused, as an agent's startup prompt or in the panel.

## Personas

An agent is assigned a persona, and its instructions reach the agent through its system
or developer-instruction channel, alongside Knot's own, on every launch - resumed
sessions included. An agent with no persona, or a persona with no instructions, is sent
Knot's instructions alone.

Each persona is `system` or `user`; a new persona created from a name and instructions
is a user persona, enabled. Personas are identified by id, not by name.

## Enabled, disabled and deleted

A persona is enabled, disabled, or deleted. The stored list keeps every persona,
including deleted ones; the active list - what an agent can select and what lookup
resolves against - excludes deleted personas and is sorted by name. A system persona
is soft-deleted, its record kept; a user persona is hard-deleted, its record removed.
A persona assigned to agents cannot be deleted: its delete action is disabled and its
tooltip reports how many agents use it.

## Shipped personas and restore defaults

The app ships a fixed set of system personas with stable ids. On startup, a shipped
persona whose id is not stored is added, and stored personas are never overwritten, so
a user's rename survives a restart. "Restore Defaults" resets every shipped system
persona to its original name, instructions, and enabled state, re-adds any missing, and
leaves user personas untouched; it runs only after a confirmation.

## Library windows

Each opens from the Window menu, is titled with its library's name, and comes to the
front rather than opening a second when already open.

### The Personas window

It lists every non-deleted persona with its name and a truncated instructions preview,
with per-row edit and delete actions, "Add Persona…", and "Restore Defaults". Adding or
editing opens an editor for the name and instructions; canceling discards the changes.

### The Prompts window

It lists every library prompt in the order added, with its name and a truncated,
single-line text preview, and offers per-row edit and delete and an "Add Prompt…".
Adding and editing share one editor with a name field and a multi-line text field;
save stays disabled while either field is blank, and the editor lists the prompt
variables, inserting a chosen one at the caret.

A prompt's name and text must be non-empty after trimming, the text may span several
lines and is stored exactly as entered, and every change is persisted before success.
Editing a prompt changes every agent that references it: each agent's next fresh session
is sent the new text. Deleting an unreferenced prompt removes it immediately; deleting a
referenced one asks for confirmation naming how many agents and bench entries will
launch without a startup prompt.

### The Bench window

It lists every bench entry - avatar, name, the folder's last path component, and its
startup prompt's name, "Custom", or nothing - with edit and remove actions. Editing
changes the entry's name and startup prompt only; its other fields come from the agent
it was saved from. Removing asks for confirmation naming the entry.

## Startup prompts

An agent's or bench entry's startup prompt is none, a reference to a library prompt by
id, or custom text on the agent itself. The
[agent editor]({{ '/latest/agents/' | relative_url }}) offers it as a Startup Prompt
control: None, each library prompt by name, or Custom with a multi-line text field.
Resolving it gives the text to send - nothing, the referenced prompt's current text, or
the held text - with variables expanded for the agent it is sent to. Blank custom text
is stored as none.

On a fresh session the startup prompt is sent as its own user turn after the
initialization prompt. It is not sent on resume or load, on Register Agent, or when the
initialization prompt is not sent, and a shell-type agent never has one. Changing only
the startup prompt does not restart the agent; it takes effect next fresh session. A
reference to a prompt no longer in the library resolves to nothing - the launch
proceeds, and the reference is kept so the agent editor can show it as missing.

## Prompt variables

Prompt text supports variables written between `{% raw %}{{{% endraw %}` and `{% raw %}}}{% endraw %}`,
with optional whitespace inside the braces — so `{% raw %}{{agent.name}}{% endraw %}` and
`{% raw %}{{ agent.name }}{% endraw %}` are the same variable:

| Variable | Expands to |
|---|---|
| `agent.name` | The agent's name. |
| `agent.id` | The agent's Knot agent ID. |
| `agent.type` | The agent's type. |
| `folder` | The folder's absolute path. |
| `folder.name` | That path's last component. |
| `workspace` | The workspace's name (empty if none). |
| `branch` | The folder's current branch (the short commit hash when HEAD is detached, empty outside a repository). |
| `date` | Today's date in the local time zone as `YYYY-MM-DD`. |

These are the only names that expand, matched case-sensitively. Expansion happens when
the text is used, never when stored, so one prompt serves every agent, and values are
inserted literally rather than expanded again.

An unknown name is left exactly as typed and does not stop the prompt from being sent;
anything that edits prompt text flags it as a warning without blocking save. A backslash
before `{% raw %}{{{% endraw %}` writes a literal `{% raw %}{{{% endraw %}` and suppresses expansion, and an
unclosed `{% raw %}{{{% endraw %}` is left as typed.

## Prompts in the panel

Typing `/` in a panel composer opens the lookup, listing the built-in panel commands,
the selected agent's skills, and the library prompts. A library prompt's entry uses the
prompt's name and a single-line preview of its text, and is distinguishable from
commands and skills. A prompt added in the Prompts window reaches an open composer's
next lookup.

Inserting a library prompt replaces the partially typed slash token with the prompt's
full text, line breaks included, with variables expanded for the selected agent. The
text is editable before sending and inserting sends nothing; if the token is edited or
the lookup dismissed before the expansion resolves, nothing is inserted.

## Source contracts

- openspec/specs/personas/spec.md
- openspec/specs/prompt-library/spec.md
- openspec/specs/library-windows/spec.md
