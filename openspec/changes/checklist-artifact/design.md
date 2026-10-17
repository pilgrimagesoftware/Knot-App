# Design

## Context

The artifact panel holds two fixed slots, a markdown file and a diagram, laid
out by a two-way split with per-section collapse. Artifacts are fields on
`Agent` written by MCP tools from the server thread; the window notices them by
comparing an `ArtifactSnapshot` each repaint poll.

## Goals / Non-Goals

- Goal: a list the user works through in place and sends back once.
- Goal: the agent can read the current answers without waiting for Send.
- Non-goal: more than one checklist per agent, nested items, or item types
  beyond check and question.
- Non-goal: persisting checklists across restarts; the other artifacts are
  runtime-only too.

## Decisions

### A third slot, outside the split

The checklist is `Agent::checklist`, a third slot beside `markdown_file` and
`mermaid_source`. In the panel it gets an area of its own below the markdown
and diagram sections, sharing the height evenly, and fills the panel when it is
the only artifact. Generalising the split, its divider and per-section collapse
to three sections would touch every two-way type in the panel for a list that
needs none of it; the section is never collapsible.

### The store holds the answers

Ticks and answers are written to the agent's checklist as the user makes them,
so `get-checklist` reads the same state the panel draws with no extra plumbing,
and the snapshot comparison repaints any window showing it. A republish carries
answers over by item id when the kind is unchanged: an agent can reword or add
items without wiping what the user said, and a changed kind starts over because
a tick is not an answer.

### Send reuses the review delivery

Send delivers `Checklist::summary` - plain text, one line per item, not
localized because it is addressed to the agent - through the same paths as the
markdown section's Approve: `deliver_panel_prompt` with `PromptOrigin::User`
for a Panel-mode agent, and the terminal's typed command for a Terminal-mode
one. A terminal gets the summary on one line, its lines joined with ` | `:
whether a newline typed at a CLI's prompt submits early depends on the CLI.
Sending marks the list sent; any later change marks it unsent so the control
offers to send again.

### Items are parsed by hand

`PropertySchema` declares only a type and a description, so `items` is declared
as an `array` with its object shape described in prose, as `plan-tasks` does,
and validated in the handler: non-empty ids and text, unique ids, a known kind.
