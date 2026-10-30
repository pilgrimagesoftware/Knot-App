---
title: Coordination
version: latest
permalink: /latest/coordination/
---

# Coordination

Knot hosts a local MCP server that its agents connect to as tool clients. Through that server an agent registers itself, messages its teammates, inspects the roster, plans and dispatches work, and shows artifacts to the user. This page describes the observable behaviour: the server's endpoints, the tool surface, and the rules that govern messaging, the registry, the task graph, and agent hooks.

The [Agents]({{ '/latest/agents/' | relative_url }}) page covers what an agent is; coordination is how agents act on each other and on the workspace. [Git and worktrees]({{ '/latest/git-worktrees/' | relative_url }}) covers the repository tools named below.

## The local MCP server

Knot runs the server bound to loopback only (`127.0.0.1`) on a configurable port that defaults to `8766`. It is enabled by default and can be turned off in [Settings]({{ '/latest/settings/' | relative_url }}); when it is off, no port is opened and agents receive no registration prompt. Change the port when another service already holds the default.

Once started, the server runs until Knot quits. Closing a window, including the workspace manager and the last one open, does not stop it, because agents in other windows are still its clients. On quit it stops and releases the port.

| Request | Purpose |
| --- | --- |
| `POST /mcp` | MCP JSON-RPC 2.0: `initialize`, `tools/list`, and `tools/call`. |
| `GET /mcp` | Server-sent events stream. |
| `GET /health` | Liveness check; returns success without initialization. |
| `GET /` | Basic server information. |
| `GET /api/v1/agent/status` | Plain-JSON status array, one entry per agent. |

The status endpoint exists for external tooling. Each entry carries the agent id, name, folder, state, agent-set status text, registered flag, agent type, session id when present, and hook metadata when non-empty, with keys in a stable order.

Each agent connects on a URL that names it (`…/mcp?agent=<id>`), which Knot places in the URL it gives the agent at launch. A connection is bound to that agent, so a tool call naming a different caller fails and changes nothing, and a new connection for an agent replaces the previous one. A request on a bound connection marks its agent registered without any tool call, which is how a resumed agent reappears registered before it does anything.

## Tool catalog

`tools/list` returns the tools below. Every call returns a single text result; a failure sets the error flag with an explanation instead of failing the request. A call that omits a required argument names the missing argument.

### Registration and discovery

| Tool | Purpose |
| --- | --- |
| `register-agent` | Mark the calling agent registered, optionally with a session id; returns the unread count and the visible members. |
| `list-agents` | List the agents the caller may see, each with folder, status, registered flag, and registry fields. |
| `describe-agents` | Find registry candidates carrying every requested capability tag, ranked cheapest-idle-first; omit tags to list all visible entries. |
| `list-repos` | List repositories discovered under the configured source folder, with their worktrees. Takes no arguments. |
| `list-worktrees` | List one repository's worktrees as name and absolute path. |

### Messaging

| Tool | Purpose |
| --- | --- |
| `send-message` | Send one message to one agent in the caller's workspace. |
| `check-messages` | Read the caller's unread messages, marking them read unless a non-destructive read is requested. |
| `broadcast-message` | Send one message to every eligible agent in the caller's workspace; returns the count created. |

### Agents and worktrees

| Tool | Purpose |
| --- | --- |
| `create-agent` | Create an agent from explicit fields, or deploy a bench template; the new agent is attributed to the caller. |
| `close-agent` | Close an agent, allowed only for an agent the caller created. |
| `create-worktree` | Create a worktree on a new branch and return its path; `path` puts it where the project says worktrees go. |

### Status and display

| Tool | Purpose |
| --- | --- |
| `set-status` | Set the agent's human-readable status text; an empty string clears it, separate from the automatic state. |
| `display-markdown` | Show a Markdown file in the calling agent's panel, optionally expanded. |
| `view-mermaid` | Show a Mermaid diagram in the calling agent's panel, optionally titled. |
| `show-checklist` | Show a checklist of questions and checkbox items in the calling agent's panel. The user answers in place and presses Send to deliver the answers. Calling it again keeps answers for unchanged items. |
| `get-checklist` | Read the checklist as the user has left it: ticks, answers, and whether it was sent. |

#### Checklists

Use a checklist when an agent needs several answers or confirmations from the
user at once. The list stays in the artifact panel instead of scrolling away in
the conversation. Each item has an `id`, the `text` to show, and a `kind`:
`check` for a checkbox (the default) or `question` for a free-text answer.

```json
{
  "agentId": "<your agent id>",
  "title": "Before I release",
  "items": [
    { "id": "tests", "text": "The test suite passes on your machine" },
    { "id": "version", "text": "Which version number should this be?", "kind": "question" }
  ]
}
```

The user's ticks and answers are recorded as they make them, and
`get-checklist` returns them at any time. When the user presses Send, the agent
receives the whole list as one message, each item with its id and its tick or
answer. Calling `show-checklist` again updates the list and keeps the answers
for items whose id and kind are unchanged. Closing the checklist and reopening
the panel restores it with its answers.

### Planning tasks

| Tool | Purpose |
| --- | --- |
| `plan-tasks` | Commit a task graph for the caller; returns each task's resulting state. |
| `dispatch-task` | Deliver a ready task's goal to its assignee and mark it dispatched. |
| `complete-task` | Report a task done or failed and apply the downstream rules. |
| `task-status` | List every task in the caller's committed graph with state, assignee, and dependencies. |

## Sending messages

`send-message` is a direct message to one agent; `broadcast-message` creates one message per eligible recipient in the caller's workspace, excluding the sender, unregistered agents, and shell agents. The sender must be registered or the send is rejected. Messages are held in memory only and do not survive a restart.

Routing is workspace-scoped: a recipient outside the sender's workspace is treated as not found. Shell agents cannot receive messages. A companion agent can only send to and receive from its owner, and broadcast applies the same filter per recipient.

A new message is unread. `check-messages` returns the caller's unread messages and, by default, marks them read; the caller can request a non-destructive read that leaves the flags alone. Unread queries report whether anything is unread and the most recent unread message id. Cleanup caps stored read messages at 100, dropping the oldest read messages and never an unread one.

When a message is stored and the recipient is idle, Knot delivers a short "check your inbox" prompt. If the recipient is working, mid-turn, or has a permission request outstanding, the message waits and is surfaced at its next transition to idle. A recipient is nudged at most once per message. A direct send to a deactivated agent starts its session; a broadcast never starts one. A nudge queues behind the agent's active work rather than interrupting it, and its text tells the agent to continue its previous work if the inbox is empty.

## The agent registry

Every agent and bench template carries a one-line description, a set of capability tags, and a cost tier of `low`, `medium`, or `high`, defaulting to `medium`. Tags are opaque: matched by case-insensitive equality, never parsed, and never restricted to a fixed list. Each entry also reports derived fields the caller does not set: the tools reachable by its agent type, and its status, which for a live agent is the automatic state, the agent-set status text, and the registered flag, and for a template is `template`.

`describe-agents` answers a capability query with the entries carrying every requested tag, ranked so the cheapest agent able to start soonest comes first: idle live agents, then live agents in any other state, then deployable templates; within each group by ascending cost tier, then name. The ordering is a recommendation, not an assignment. A tag no entry carries returns an empty list rather than an error.

An orchestrating agent learns its team by querying the registry rather than from stored text that names teammates, so adding, retagging, removing, or re-pricing an agent changes the next query's answer with no edit elsewhere.

## Planning work with the task graph

Work split across more than one agent, or into more than one task for a single agent, is dispatched from a committed graph. A single hand-off stays an ordinary message and needs no plan. Each task carries an id unique within its graph, a goal, an optional assignee (a visible agent, or a set of capability tags resolved at dispatch time), dependencies on other tasks in the same graph, and a state. A graph is owned by one agent, is runtime state, and is discarded when its owner is removed.

A commit is rejected, leaving any previous graph intact, when a dependency names a task absent from the graph, a task depends on itself, the dependencies form a cycle, or the graph exceeds the task limit. Validation happens at commit, not at dispatch.

| State | Meaning |
| --- | --- |
| `pending` | Waiting for one or more dependencies to finish. |
| `ready` | Every dependency is done; it may be dispatched. |
| `dispatched` | Delivered to its assignee; only its owner reports it done or failed. |
| `done` | Reported complete. |
| `failed` | Reported unsuccessful; tasks that depend on it become blocked. |
| `blocked` | A dependency failed; it stays blocked until a new plan changes its dependencies. |

On commit, a task with no dependencies is `ready` and every other task is `pending`. A `pending` task becomes `ready` as soon as all its dependencies are `done`. `dispatch-task` delivers a ready task's goal to its assignee under the messaging rules and marks it dispatched; it is refused, leaving the task ready, when the caller has no committed graph, the task is not ready, no candidate resolves for a tag assignee, or messaging rejects the delivery, and a refusal names the dependencies that are not yet done. `complete-task` reports a task done or failed and returns which tasks became ready and which became blocked. `task-status` lists the graph, and is an empty list rather than an error before anything is planned.

Committing a graph while one exists preserves the state of tasks present in both and never cancels, recalls, or duplicates a dispatched task. Committing also shows the plan in the owning agent's panel as a diagram of tasks and dependencies, with each task's current state.

## Agent hooks

Knot ingests plugin hook events from Claude and Codex agents over `POST /api/v1/agent/register` and `POST /api/v1/agent/status`. Every body is JSON carrying at least `agent_id`; a missing or unparseable id returns 400. The body may also name the agent type (`agent`, defaulting to `claude`), a `session_id`, a `source`, a `hook`, a `status`, and a nested `payload`. The routes dispatch on the agent type: `claude` uses the Claude handler, `codex` uses the Codex handler on the status route, and any other type is rejected.

For Claude, a `source` of `startup` is full registration and `source` of `resume` is a session-id update; successful registration marks the agent registered and returns the member list. On the status route, `running` maps to Working, `idle` to Idle, and `input` to Awaiting input, and any other value is 400; an `input` status also raises a desktop notification carrying `payload.message` when present. Codex acts on a `payload.type` of `agent-turn-complete`, setting the agent Idle and recording `payload.thread-id` as its session id; any other event is 400.

Hooks also merge known non-empty fields into the agent's metadata: for Claude `transcript_path`, `cwd`, `model`, and `session_id`; for Codex `cwd`, `thread-id`, and `turn-id`. The status route additionally accepts subagent lifecycle events identified by their `hook` field. A dispatch records a running subagent with its task, and its kind when given; a completion marks it finished or failed with its reason. Reporting a subagent never moves the agent's own activity status. An event missing a required subagent identifier is rejected, while a completion for an identifier Knot never dispatched is accepted and ignored.

For a Terminal-mode agent, a Claude `Stop` hook or a Codex turn-complete event also triggers Autopilot classification of the agent's last message, when Autopilot is enabled and an API key is configured. Agents in the agent panel post no hooks; their workspace window triggers Autopilot when a turn ends. See [Autopilot]({{ '/latest/autopilot/' | relative_url }}) for what Autopilot does with that classification.

## Source contracts

- openspec/specs/mcp-server/spec.md
- openspec/specs/mcp-tools/spec.md
- openspec/specs/mcp-messaging/spec.md
- openspec/specs/agent-registry/spec.md
- openspec/specs/task-graph/spec.md
- openspec/specs/agent-hooks/spec.md
- openspec/specs/autopilot/spec.md
