---
title: Agents
version: latest
permalink: /latest/agents/
---

# Agents

An agent is one coding CLI running in its own pane, bound to a working folder. Knot launches it,
gives you a conversation view, and connects it to Knot's tooling so it can work alongside the other
agents in its [workspace]({{ '/latest/workspaces/' | relative_url }}). See [Coordination]({{ '/latest/coordination/' | relative_url }}) for how agents message one another.

## Add an agent

Open the agent editor from the sidebar's new-agent control, the sidebar background menu's New Agent,
or a row's Edit Agent…. Its fields are the agent's name and avatar, then the folder it works in,
then its agent type (default `claude`), shell command (only meaningful for a `shell` agent),
persona, activation mode, registry metadata (a one-line description, capability tags, and a cost
tier of `low`, `medium`, or `high`, default `medium`), and a startup prompt. The folder is required
and must be an existing directory, and the primary action stays disabled until a name and an
existing folder are both present.

Choosing a folder while the name field is empty fills the name with the folder's last path component
(`/Users/dana/code/widget` becomes `widget`), leaving it editable; a name you typed is never
replaced.

Activation mode is `passive` by default or `active`. A `passive` agent starts the first time you
select it and then keeps running; an `active` agent starts with its workspace. A new agent is
selected when you create it, but creating a `passive` agent doesn't start it: it stays stopped
until you click it or another agent messages it. Changing the mode never restarts, starts, or
stops the agent.

For every type except `shell`, the Startup Prompt control offers None (the default), a [prompt library]({{ '/latest/personas-prompts/' | relative_url }}) prompt by name, or Custom text. It is sent as its own turn after the agent registers,
on a fresh conversation and never on a resumed one, and changing it does not restart the agent.

## Agent types

| Type | Launch | View |
|---|---|---|
| `claude` | Claude Code through its ACP adapter | Panel |
| `codex` | Codex through its ACP adapter | Panel |
| `opencode` | OpenCode through its ACP adapter | Panel |
| `gemini` | Gemini through its ACP adapter | Panel |
| `copilot` | GitHub Copilot through its ACP adapter | Panel |
| `shell` | A terminal shell in the agent's folder | Terminal |

The type fixes how the agent launches and which view it gets. A non-shell agent always launches
through its adapter and shows the conversation panel, with no Terminal toggle; a shell agent always
launches a terminal. The adapter starts in the agent's folder, connects to Knot's tool server, and
is sent a registration request and any startup prompt; a shell agent's terminal changes to the
folder, clears the screen, then runs its custom shell command when set, and does not register.
Per-CLI flags are set under [Settings]({{ '/latest/settings/' | relative_url }}) - Coding - Agent Options and documented in [Agent CLI arguments]({{ '/latest/reference/agent-cli-arguments/' | relative_url }}); shell behaviour is in [Agent shells]({{ '/latest/reference/agent-shells/' | relative_url }}).

## Agent lifecycle

Creating an agent gives it a fresh identity and appends it to the agent list and its workspace.
Selecting a passive agent starts it. Renaming an agent, or changing its avatar, description,
capabilities, cost tier, or startup prompt, does not restart it.

Restart Agent and the sidebar's Restart All ask for confirmation, name whether the conversation is
kept, then restart the agent with its id preserved and its state reset to Idle. With Restore
conversation on launch enabled it reloads the conversation it had; disabled, it starts fresh, and
Restart with New Conversation always starts fresh. Changing an agent's folder, type, or persona also
restarts it fresh. An agent can be resumed into a prior conversation, which reloads it rather than
starting fresh.

Deactivate stops a running agent without removing it: its session is torn down and its pane shows it
stopped, while the agent keeps its name, folder, position, persona, and activation mode. It then
stays stopped, except that selecting it or a direct message from another agent starts it again.
Remove Agent and Close All ask for confirmation, then remove the agents and any companions they own,
unregister them, and tear down their sessions. Save to Bench stores an agent as a reusable template;
Bench Agent… saves it and then closes it. Deactivate All stops every running agent, and a shell
agent is removed on its own when its terminal process exits.

## The sidebar agent list

Each agent has a row: its avatar and name, a state dot, and detail lines under the name - agent
type, persona, status, and folder - each with a leading icon identifying what the line is. The
status line shows the agent's own status text when it has one, otherwise its terminal title. A row
that is not running is dimmed, whether it is passive and never started or was deactivated.

Turn Off Autopilot / Turn On Autopilot sit beside Lock Agent while Autopilot is on in Settings; see
[Autopilot]({{ '/latest/autopilot/' | relative_url }}).

Lock Agent shuts an agent out of agent messaging: no agent can message it, so a message cannot
start it, and it cannot message other agents. Broadcast to All Agents skips it too. The row shows a
**Locked** line while it lasts, and the lock is kept across a relaunch. Unlock Agent undoes it.
Typing into the agent yourself is unaffected, and so is **Send prompt to**: a locked agent is still
listed there, marked with a lock and "(locked)", so you know before you send. Both items are also
in the menu bar's Agents menu.

Click a row to select its agent, drag a row up or down to reorder agents - with companions moving
with their owner and the Dashboard and Pull Requests rows pinned at the top - or right-click a row
for its context menu.

The context menu offers companion creation (New Companion…, New Shell Companion), editing (Edit
Agent…, Fork Agent, Duplicate Agent), placement (Move to Workspace, Save to Bench, Bench Agent…),
opening (Open In…, Markdown Files), and session actions (Deactivate, Register Agent, Lock Agent or
Unlock Agent, Restart Agent,
Restart with New Conversation, Remove Agent), showing the items that apply. A companion shows only
Edit Agent…, Open In…, and Remove Agent; a shell agent hides Register Agent and Restart with New
Conversation. Fork carries the source's conversation into a new agent, Duplicate copies the agent
without it, and Register Agent re-sends the registration prompt by hand.

Move to Workspace moves the agent's session along with it. The agent leaves its current window
and its session there stops. It restarts in the destination workspace in the same conversation,
the way Restart Agent does when the conversation is kept. If the destination's window is open, the
agent appears there immediately; if not, it starts the next time that workspace is opened. A turn
in progress when the agent moves is interrupted.

Open In… opens the agent's folder in VS Code, Zed, Xcode, Finder or Terminal; an application that is
not installed fails quietly. Zed is opened through the command-line tool inside `Zed.app`, so the
folder opens in a new Zed window whether or not Zed is running, and without the `zed` shell command
installed. If that tool is missing, Knot asks macOS to open the folder in Zed instead.

Right-clicking the empty space in the sidebar opens a workspace menu: New Agent, New from Bench,
Restart All, Close All, Deactivate All, and Broadcast to All Agents…. Every item is shown, with the
ones that do not apply disabled; New from Bench deploys a saved template.

## Activity status

Each running agent has one status, shown on its sidebar state dot and its dashboard card and used to
sort the dashboard.

| Status | Colour | Meaning |
|---|---|---|
| Working | Orange | Running a turn or producing terminal output. |
| Idle | Green | Quiet and waiting. |
| Awaiting input | Red | Needs you - for example a permission prompt is open. |
| Error | Red | The session ended with an error. |
| Session limit | Red | The agent's provider refused the turn: its session or usage limit is reached. The header says when it resets, if the provider said. |

Terminal output moves an agent to Working and back to Idle after a short quiet period; typing keeps
it Working. A permission request moves it to Awaiting input immediately; Return moves it to Working
and Escape to Idle. A permission answered any other way, such as cancelled by Stop or by the session
ending, takes the agent out of Awaiting input as soon as it is answered. A shell agent stays Idle
whatever its output.

An agent in Awaiting input says why when it knows: hover its state dot, or read the line under its
dashboard card. A pending permission request shows its title (`permission requested for Bash`); a
wait Autopilot started says what it read in the agent's last reply (`Autopilot: the last reply asks
you a question`). The reason goes as soon as the agent leaves Awaiting input. A session that exits non-zero, or
a panel connection that ends with an error, moves the agent to Error. A panel agent whose turn is
refused with a session or usage limit ("You've hit your session limit · resets 5:20pm") shows Session
limit instead, in red, until you send it another prompt. The dashboard's cards add a
glanceable working/thinking indicator that reflects the same status and animates on each change; the
sidebar row does not draw it.

## The conversation panel

A non-shell agent's pane is a conversation panel: the messages you and the agent exchange as they
stream, each tool call as its own card - with an icon for the tool and, for a file edit, an
added/removed line diff - and the shell commands you run from the prompt. A completed tool call
collapses to its header, while failed and running calls stay open and any call can be opened or
closed by hand. An active turn shows a turn-in-progress indicator as the conversation's last row,
below any permission prompt.

A permission request appears inline with its options as buttons, names the tool it is for, and
blocks further prompts until you answer. A message sent while a response is in progress is queued
and delivered when the turn ends; a stop control interrupts the active turn. You can switch one
agent between the panel and the raw terminal without interrupting its turn or terminal process, and
each agent remembers the mode you last chose.

Resting the pointer on a link in a response, or in a Markdown pane, shows the address it goes to
beside the pointer, so you can see where a link leads before you click it.

Point at a message to act on it. A prompt you sent shows a copy button; an agent's response shows
its controls - Copy, Reply (quotes it into the composer), Scroll to your message, and Scroll to
top. The latest response keeps its controls in view, so you can copy or reply to the answer you
just got without hunting for them; earlier ones show theirs only while you point at them, and
showing them moves nothing.

Scrolled back to read earlier messages, you stay where you are: replies streaming in and long
replies finishing their layout do not move the conversation. Only a prompt you send yourself
scrolls to the bottom to show its answer. One Knot sends for you - an inbox check or a startup
prompt - leaves your place alone unless you were already at the bottom.

The composer grows with your draft and does not scroll past its last line, so a draft cannot be
scrolled out of view. The control bar's model and effort pickers open at a fixed width, so long
model names fit whichever one is selected.

### Finding text in a conversation

Press ⌘F in an agent's panel, even while typing in the composer, to open a find bar above the
conversation. What you type is found in your prompts and the agent's replies, ignoring case, and
every match is highlighted; the bar shows which match you are on, like `2 of 5`. Return and
Shift-Return, or the arrow buttons, step to the next and previous match and scroll the
conversation to it. The `.*` button switches to regular expressions (for example `v\d+\.\d+`);
add `(?-i)` to a pattern to match case. Tool calls, shell output and notices are not searched.
Escape closes the bar and puts you back in the composer.

### Inserting a saved prompt

To start a message from a [library prompt]({{ '/latest/personas-prompts/' | relative_url }}),
type `/` in the composer, or click the Insert Prompt button (the book icon in the control bar)
or press ⌥⌘I while the composer has focus. The button opens a searchable list of your prompts;
typing filters them by name or text, Up and Down move, Return inserts and Escape closes the list
without changing anything. The prompt replaces any selected text, or goes in at the caret.
Nothing is sent until you send it yourself. With an empty library, the list offers to open the
Prompts window. ⌥⌘I can be changed on the settings window's Keyboard tab.

Variables in the prompt are expanded for the agent whose composer it is:

| Variable | Expands to |
|---|---|
| `agent.name` | The agent's name. |
| `agent.id` | The agent's Knot agent ID. |
| `agent.type` | The agent's type. |
| `folder` | The folder's absolute path. |
| `folder.name` | That path's last component. |
| `workspace` | The workspace's name (empty if none). |
| `branch` | The folder's current branch (empty outside a repository). |
| `date` | Today's date as `YYYY-MM-DD`. |
| `clipboard` | The text on the clipboard when you choose the prompt (empty if it holds no text). |

For example, copy an error message, then insert a prompt reading
`{% raw %}Explain this error: {{clipboard}}{% endraw %}`. The copied text lands in the composer,
and copying something else while it expands does not change it. `{% raw %}{{clipboard}}{% endraw %}`
is always empty in a startup prompt, which is sent without you choosing it, and the agent editor
warns when a custom startup prompt uses it.

## Subagents and processes

A coding agent can delegate work to a subagent from within its turn. Knot records each subagent as
its kind, its task, when it started, and whether it is running, finished, or failed. Subagents come
only from what the agent reports itself - never inferred from processes or output - and no
operating-system process is attached to one. A finished subagent's record stays until the agent's
turn ends, when records are discarded. An agent whose type cannot report subagents shows the list as
unavailable rather than empty.

A panel agent's pane also has an MCP servers section listing the servers the agent has, with each
one's state. The chevron at the start of a row lists the tools that server provides, by name and
description, fetched when you open it. Knot's own tools are listed directly. Another server is
asked with `tools/list`, without credentials: a server needing sign-in says so, and a disabled or
unconnected one is not contacted. A stdio server is started for the listing only with its exact
command and arguments, and is stopped as soon as the list arrives. Codex's listing gives them.
Claude Code and Gemini print only one line, so Knot looks the server up by name in the agent's own
configuration (`~/.claude.json` and `.mcp.json`, or `.gemini/settings.json`) and uses an entry only
when it reproduces that line exactly and every matching entry agrees. A matching entry that uses
`${VAR}` placeholders, or a same-named entry Knot can't read, stops the server being started.
Servers that come only from managed or enterprise MCP configuration, or from Claude Code plugins or
Gemini extensions, aren't found this way.

The selected agent's pane has a collapsible processes section, present in both views and collapsed
by default. An artifact panel open beside the pane leaves it in place; it is hidden only while that
panel is expanded over the content area. Expanded, it lists subagents first, then the processes running underneath the agent's
session - the shell for a shell agent, the adapter for a panel agent - however deep. Each process
row shows its command line, elapsed runtime, process identifier, and background or foreground
classification; each subagent row shows its kind, task, elapsed time, and state, and no process
identifier. The header names what is running when collapsed, gives the counts when expanded, and
says so when nothing is.

Every process row offers a terminate action, which asks for confirmation naming the command line
before ending that process alone - never its siblings, its parent, or the agent's session root. A
row can also copy the process identifier, copy the full command line, or open the platform's process
viewer (Activity Monitor on macOS). A subagent row offers only copying its task.

## Source contracts

- openspec/specs/agent-editor-ui/spec.md
- openspec/specs/agent-launch-command/spec.md
- openspec/specs/agent-lifecycle/spec.md
- openspec/specs/agent-list-ui/spec.md
- openspec/specs/activity-detection/spec.md
- openspec/specs/working-indicator/spec.md
- openspec/specs/acp-panel-ui/spec.md
- openspec/specs/agent-subagents/spec.md
- openspec/specs/agent-processes/spec.md
