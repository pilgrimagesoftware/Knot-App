---
title: Settings
version: latest
permalink: /latest/settings/
---

# Settings

Knot's settings live in one surface shown through its own window. This page
covers that window, how settings survive a relaunch, and the menus around it.

## Settings window

Open Settings from the application menu or with Cmd+,. Only one settings window
exists; choosing the item again raises the open one, and General is selected
when it opens. Switching tabs preserves in-progress text.

| Tab | Sections and controls |
|---|---|
| General | Appearance (Auto/System/Light/Dark); Startup ("Restore agents on launch", "Restore last conversation" - enabled only while agent restore is on, keeping its value when off, "Keep running in menu bar when closed"); Notifications ("Desktop notifications"); Agent Panel ("Shift+Enter to send" - when off, Enter sends and Shift+Enter adds a newline) |
| Coding | Source Folder (current folder, folder picker, clear with confirmation); Agent Options (agent-type picker, per-type options field) |
| Autopilot | Enable autopilot, AI Provider picker, API key, Action picker, and custom prompt configuration (see [Autopilot]({{ '/latest/autopilot/' | relative_url }})) |
| Voice | "Enable voice input"; engine picker (Apple SpeechAnalyzer only, disabled); read-only push-to-talk key, not changeable here; "Auto-insert transcription"; controls disabled while voice input is off |
| MCP | "Enable MCP server"; port field (default 8766); read-only `/mcp` URL, updated with the port; installation-command generator with copy (offers Claude, Codex, OpenCode, Gemini and Copilot, not Shell) |
| Appearance | Fonts: one row each for UI, Title and Terminal |
| Keyboard | The workspace and navigation shortcuts |

Copy actions put the exact displayed text on the clipboard. The Appearance
tab's Fonts section has one row per font, each opening the OS font panel to its
current family and size, governing only the text its label names: UI the
interface and body text, Title titles and headers, Terminal terminal text only.
A family the system cannot resolve is still named and marked unavailable; its
stored value is not rewritten.

The Keyboard tab's contents are covered in
[Keybindings]({{ '/latest/reference/keybindings/' | relative_url }}).
Personas, Prompts and Bench are not tabs; each has its own window, described in
[Personas and prompts]({{ '/latest/personas-prompts/' | relative_url }}).

## Where settings are stored

The surface is backed by three kinds of document. Preferences, the scalar
values, live in the platform's user-preferences directory (on macOS, under
`~/Library/Preferences`). Durable collections - saved agents, workspaces,
personas, bench templates, library prompts, recent repos and recorded pull
requests - live one document each in the application-data directory (on macOS,
under `~/Library/Application Support`).

Window arrangement is kept separately as UI state, so a saved workspace holds
only its identity, name, colour and agent membership. Each write touches only
the document it belongs to and is atomic; a missing or unreadable document
falls back to defaults without affecting the others. An installation upgrading
from the older single settings file is migrated once and that file is renamed
so it is never read again. A preference write applies to every open window
without a relaunch. On first launch, Knot picks the first existing of `~/src`,
`~/source` and `~/sources`.

## Appearance mode

The stored mode resolves to the appearance Knot paints: Light and Dark paint
that appearance whatever the OS reports, while System and Auto follow the OS
and behave identically. The resolved appearance is applied at startup before
the first window is shown. An OS appearance change re-resolves through the
stored mode, so a flip repaints only under System or Auto, and changing the
mode repaints every open window without a relaunch.

## Desktop notifications

Knot requests notification authorization once at startup, whether or not the
setting is on. When an agent enters Awaiting input with desktop notifications
enabled, it raises a notification titled "Knot - <agent name>", bodied with the
transition's message, or "Needs your attention" when there is none. A
notification is raised only on the transition into Awaiting input, not on later
reports, and not for an agent already awaiting input or one currently visible
in the active workspace. Clicking a notification with a valid agent id brings
the application to the front.

## Autopilot

Autopilot watches Claude Code and Codex agents for completed turns and decides
what to do when an agent goes idle and might need your input. When enabled, it
requires an AI provider and API key (OpenAI, Anthropic, or Google, each called
with a fixed, cheap model). Full details, including the decision loop, custom
action templates, and configuration options, are in
[Autopilot]({{ '/latest/autopilot/' | relative_url }}).

## Import window

Choose Import… from the File menu. The Import window is its own window, not a
settings pane, and a single instance. It reads the store as it opens and
rescans its sources on request.

Each source has its own section and is never hidden: the personas section
imports a coding-agent tool's subagent definitions and lists only tools with a
confirmed reader (Claude), and the workspaces section imports from a Skwad
installation. A source with nothing to offer says so in place of its list.

Nothing is written until the user selects records and confirms; closing without
confirming adds nothing. An import only adds records: it never deletes,
overwrites, renames or reorders what Knot already holds, never modifies the
source, and skips records it already holds, so a second run adds nothing. A
record that cannot be read is reported by name without failing the run, and
every import reports what was added, skipped and unreadable, with failure told
from success. Imported subagent definitions become editable, deletable user
personas; importing a Skwad workspace brings its agents and the personas or
bench templates they reference.

## Bug report window

Choose Report an Issue… from the Help menu. It opens as a window of its own,
beside the window you were using, even when no other window is open, and is a
single instance: choosing it again brings the open one forward. It collects a one-line subject, a multi-line
description, and a kind of bug (the default) or feature request. The kind files
the issue under the `bug` or `enhancement` label, changes the field hints, and
controls whether logs are offered - logs appear for a bug only.

An app diagnostics section is filled in automatically and is not editable. It
shows the running binary's version and build identifier (as the About window
does), the host OS and version, the hardware architecture, and whether the
forge tool Knot uses is available and authenticated - not installed, not
authenticated, or ready. The diagnostics are selectable text for copying.

The window can attach the application log and the MCP server log, each chosen
separately and neither by default; a chosen log is attached as its recent
lines, and screenshots (PNG, JPEG or GIF, up to a fixed number) are listed by
name with a remove control.

An issue is public, so an attached log is scrubbed before it goes in. Knot
replaces tool-call arguments with the tool's name, argument names and size, and
redacts session ids, credentials (authorization headers, bearer tokens, API
keys, GitHub tokens, passwords in URLs) and email addresses. Your home
directory is written as `~`. The scrub covers every line of the log, including
lines that agent adapters and MCP servers wrote, and it errs towards removing
too much. When Knot cannot file the issue and opens the GitHub page in your
browser instead, the page cannot carry the logs. Knot writes scrubbed copies to a
temporary folder, names those copies on the page, and shows them in Finder for
you to drag in. It never names the logs themselves. The copies stay until
your next report or the next time Knot starts. Knot does not upload screenshots; it names them in
the issue and shows them to drag onto it.

Report is disabled while the subject or description is empty or whitespace,
and Cancel or Escape closes the window without filing. With the forge tool
ready and authenticated, Report files the issue using credentials it already
holds - never a stored token - then closes the window and confirms the filed
issue in the window you opened it from. Otherwise
it falls back to a pre-filled GitHub compose page, naming any chosen log file.
A failed submission keeps what was typed for a retry.

## About window

Choose About Knot from the application menu. The window is non-modal and a
single instance, so the rest of the app stays usable behind it. It shows the
app icon and name, the running binary's released version number, a build
identifier, and a copyright line naming the publisher. Version and build come
from the running binary, so two builds of one version differ in their build
identifier. Clicking the version and build puts both on the clipboard.

A credits area names the author, the app's own license, each embedded
third-party work with its license (the bundled fonts, the UI toolkit and the
terminal engine), and the app Knot was derived from. There is no update
control. The window is fixed-size and dismissed with its close button or
Escape; it carries no in-window Close or OK button on macOS, and a Close button
on other platforms.

## Application menu bar

| Menu | Carries |
|---|---|
| Knot | About Knot, Settings… (Cmd+,), Hide Knot (Cmd+H), Hide Others (Option-Cmd+H), Show All, Quit Knot (Cmd+Q) |
| File | New Workspace (Cmd+N), New Agent… (Cmd+T), Close Window (Cmd+W), Import… |
| Edit | Undo (Cmd+Z), Redo (Shift-Cmd+Z), Cut (Cmd+X), Copy (Cmd+C), Paste (Cmd+V) |
| View | Dashboard (Option-Cmd+O), Pull Requests (Option-Cmd+P), Focus Agent Input (Cmd+L), Jump to Bottom (Ctrl-Cmd+Down), Artifacts (Option-Cmd+A), Select Agent, Enter Full Screen (Ctrl-Cmd+F) |
| Agents | The same items as the selected agent's row menu, acting on the selected agent |
| Window | Command Center (Option-Cmd+0), Workspaces (Cmd+0), Personas, Prompts, Bench, Minimize (Cmd+M), Zoom, then macOS's list of open windows |
| Help | Knot Help (Cmd+?), Report an Issue… |

The Edit menu's items operate on the focused text field and are enabled exactly
when one is focused, so a key the terminal claims, such as Cmd+C, still reaches
the terminal. The Agents menu acts on the selected agent; every item is
disabled when none is selected or no workspace window is focused. The menus
reflect current state when opened, including a rebinding without a restart.
Personas, Prompts and Bench open their windows and stay enabled with no window
open; Enter Full Screen is the platform's own item.

## Source contracts

- openspec/specs/settings-ui/spec.md
- openspec/specs/settings-persistence/spec.md
- openspec/specs/appearance-mode/spec.md
- openspec/specs/desktop-notifications/spec.md
- openspec/specs/autopilot/spec.md
- openspec/specs/data-import/spec.md
- openspec/specs/import-ui/spec.md
- openspec/specs/bug-reporting/spec.md
- openspec/specs/about-ui/spec.md
- openspec/specs/app-menu/spec.md
