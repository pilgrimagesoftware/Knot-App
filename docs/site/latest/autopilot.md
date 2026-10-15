---
title: Autopilot
version: latest
permalink: /latest/autopilot/
---

# Autopilot

Autopilot watches Claude Code and Codex agents for a completed turn. When an agent goes idle and its last message might need your input, Autopilot classifies that message and decides what to do — so you don't have to babysit every agent waiting on a yes/no answer.

## How Autopilot works

Autopilot runs only on completed turns for Claude Code and Codex agents. It requires an AI provider and API key (OpenAI, Anthropic, or Google, each called with a fixed, cheap model per provider). With the toggle off or no API key configured, nothing is classified and agents behave exactly as they do without Autopilot.

When a watched agent goes idle, Autopilot receives its last message and begins the decision loop.

### Classification

Unless the configured action is Custom (which bypasses classification entirely), Autopilot sends the agent's last message to the configured provider with a fixed classification prompt. The provider responds with one word: `completed`, `binary`, or `open`.

- **Completed** — the agent finished its work, is reporting results, or sharing status. No input is needed.
- **Binary** — the agent is asking for simple yes/no approval to proceed.
- **Open** — the agent is asking an open-ended question, requesting you choose between options, or asking for information or feedback.

A classification request that fails (no network, expired key, unsupported provider) returns no classification. The agent is treated as if it had nothing to ask, so a transient outage does not interrupt you on every idle turn.

### Decision loop

Once classified (if not Custom), the configured action decides what happens:

| Action | What it does |
|---|---|
| Mark conversation | Sets the agent to Awaiting input and raises a notification — the same as if the agent itself had asked. |
| Ask me | Does the same, and opens a dialog carrying the agent's message, the classification (for reference), a Dismiss button, a Switch to Agent button, and — for a binary question only — a default "Yes, continue" button. |
| Auto-continue | Sends "yes, continue" for a binary question. An open question cannot be auto-answered, so this falls back to Mark conversation instead. |
| Custom | Skips classification; see below. |

A `completed` classification never triggers anything, no matter what action is configured.

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
