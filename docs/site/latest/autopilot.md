---
title: Autopilot
version: latest
permalink: /latest/autopilot/
---

# Autopilot

Autopilot watches your agents for a completed turn. When an agent goes idle and its last message might need your input, Autopilot classifies that message and decides what to do — so you don't have to babysit every agent waiting on a yes/no answer.

## How Autopilot works

Autopilot runs on completed turns. It requires an AI provider and API key (OpenAI, Anthropic, or Google, each called with a fixed, cheap model per provider). With the toggle off or no API key configured, nothing is classified and agents behave exactly as they do without Autopilot.

How Knot sees a turn end depends on how the agent runs:

- **Panel-mode agents** (any coding agent in the agent panel): when a turn finishes on its own, Autopilot reads the agent's final reply from the conversation. A turn that only ran tools, ended in an error, or stopped to ask for a permission is not classified.
- **Terminal-mode agents**: Claude Code on its `Stop` hook and Codex on its turn-complete hook. Other agent types aren't watched.

When a watched agent's turn ends, Autopilot receives its last message and begins the decision loop.

### Classification

Unless the configured action is Custom (which bypasses classification entirely), Autopilot sends the agent's last message to the configured provider with a fixed classification prompt. The provider responds with one word: `completed`, `binary`, or `open`.

- **Completed** — the agent finished its work, is reporting results, or sharing status. No input is needed.
- **Binary** — the agent is asking for simple yes/no approval to proceed.
- **Open** — the agent is asking an open-ended question, requesting you choose between options, or asking for information or feedback.

A classification request that fails (no network, expired key, unsupported provider) returns no classification. The agent is treated as if it had nothing to ask, so a transient outage does not interrupt you on every idle turn.

### Per agent

Autopilot is on or off for the whole app in Settings. While it's on, you can turn it off for one agent: choose **Turn Off Autopilot** from the agent's context menu or the Agents menu, and **Turn On Autopilot** to undo it. That agent's replies are then never classified, whatever Settings says. The choice is kept across a relaunch. A duplicate or fork of the agent starts out following Settings again.

An agent Autopilot is reading shows an **Autopilot** line with a propeller icon on its sidebar row, and the same icon beside its status in the window's title bar. Shell agents never show it: Autopilot has no replies of theirs to read.

### Decision loop

Once classified (if not Custom), the configured action decides what happens:

| Action | What it does |
|---|---|
| Mark conversation | Sets the agent to Awaiting input and raises a notification — the same as if the agent itself had asked. |
| Ask me | Does the same, and opens a dialog carrying the agent's message, the classification (for reference), a Dismiss button, a Switch to Agent button, and — for a binary question only — a default "Yes, continue" button. |
| Auto-continue | Sends "yes, continue" for a binary question. An open question cannot be auto-answered, so this falls back to Mark conversation instead. |
| Custom | Skips classification; see below. |

A `completed` classification never triggers anything, no matter what action is configured. Autopilot only counts a reply as needing you when it asks you, the reader, a question or for a decision. A status report, a reply that says nothing is needed from you, a request addressed to another agent, or "let me know if you need anything else" after finished work is `completed`.

When Mark conversation or Ask me marks an agent Awaiting input, its state dot's tooltip and its dashboard card say so, with the reason, for example `Autopilot: the last reply asks for a go-ahead`. The notification carries the same text. That way a wait Autopilot started can be told apart from one the agent asked for itself.

A reply Autopilot sends goes in as a prompt: in the agent panel it's queued behind a running turn like any other, and in a terminal it's typed and submitted. Sending a reply doesn't mark the agent as Awaiting input, because the reply starts its next turn.

### In the conversation

In the agent panel, Autopilot adds a line to the conversation each time it acts, so you can tell its doing from the agent's:

| Line | When |
|---|---|
| Autopilot: this reply needs your input | Mark conversation, and Auto-continue on an open question. |
| Autopilot: asking you how to continue | Ask me, as its dialog opens. |
| Autopilot replied for you | Auto-continue, Custom, or the dialog's "Yes, continue". The reply follows it as a prompt. |

While a reply waits behind a running turn, its row in the prompt queue reads "Queued Autopilot reply". These lines exist only in the open window; they're not part of the agent's own history, so a resumed conversation doesn't show them.

## Custom action

Custom bypasses classification entirely. Autopilot sends the agent's last message to the configured provider with your own prompt appended to a fixed system framing. The framing tells the provider to respond with exactly what should be sent to the agent, or the word `EMPTY` if nothing should be sent.

- An empty, whitespace-only, or `EMPTY` response (case-insensitive) triggers no action.
- Any other response is trimmed of surrounding whitespace and injected into the agent as if you had typed it.
- An empty custom prompt skips the request entirely and takes no action without calling the provider.
- A request failure falls back to Mark conversation — unlike plain classification, which stays silent on failure. You've configured something to happen, so silence there would look broken rather than quiet.

The custom prompt receives this system framing first:

> You are managing an AI coding agent. You will receive the agent's last message. Your response will be sent directly to the agent as input.
>
> IMPORTANT: Reply with ONLY the exact text to send to the agent. No explanations, no reasoning, no preamble, no quotes — just the raw instruction for the agent. If no action is needed, reply with exactly "EMPTY" (nothing else).

Then your custom prompt is appended. Example custom prompts:

```
If the agent is asking for approval and the message is under 3 lines, 
approve it. Otherwise, ask me to review.
```

```
Respond with "yes" if the agent is asking a yes/no question.
Respond with "EMPTY" otherwise.
```

```
Ask for 3 bullet points of requirements before proceeding.
```

## Settings

Open Settings from the application menu or with Cmd+,. The Autopilot tab carries these controls:

- **Enable autopilot** — toggles the entire feature. When off, no classification runs and agents behave normally.
- **AI Provider** — picker for OpenAI, Anthropic, or Google. Each is called with a fixed model chosen for cost and speed. The read-only model name is shown below the provider.
- **API key** — your API key for the chosen provider. Blank or whitespace-only keys fail fast without making any network request.
- **Action** — picker for Mark conversation, Ask me, Auto-continue, or Custom. The description of the selected action is shown below.
- **Custom prompt** — a text area (shown only when Custom is selected). Your own prompt, appended after the system framing above. Leave it empty to disable the custom action without unchecking it.

Changes take effect immediately; there is no Save button.

## Source contracts

- openspec/specs/autopilot/spec.md
- openspec/specs/agent-hooks/spec.md
