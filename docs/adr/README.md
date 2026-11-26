# Architecture Decision Records

Each ADR captures one architectural decision, the options weighed, and the
consequences. New records start `proposed` and flip to `accepted` once the
decision survives contact with the code. A reversed decision is a new ADR that
supersedes the old one; accepted records are not edited.

Scaffold a new record with `/adr "<title>"` (see `.claude/commands/adr.md`), or
copy `0000-template.md` by hand and add a row below.

| ADR | Title | Status | Decision | Date |
| --- | ----- | ------ | -------- | ---- |
| [0001](0001-git-flow.md) | Git-flow branching model | accepted | `develop` integration branch, `main` release-only | 2026-09-10 |
| [0002](0002-gpui-base-auto-grow-fork.md) | Fork gpui-base to allow auto-grow on any multi-line mode | superseded by [0003](0003-gpui-base-fork-on-0-7.md) | `[patch.crates-io]` fork until upstream relaxes the bound | 2026-09-23 |
| [0003](0003-gpui-base-fork-on-0-7.md) | Carry the gpui-base fork onto gpui-kit 0.7.0 | accepted | Same four fork commits, re-cut from `v0.7.0`; 0.7 did not relax the bound | 2026-10-05 |
| [0004](0004-gpui-base-link-hover.md) | Publish the hovered link from the gpui-base fork | accepted | One more fork commit, `text::hovered_link`, on `knot/link-hover-0.7.0`; Knot draws the URL in a `Root` plugin | 2026-10-30 |
| [0005](0005-agent-reports-working-directory.md) | Agents report their working directory | accepted | `set-working-directory` MCP tool; any git working-tree root; runtime only, read through `effective_folder()` | 2026-11-05 |
| [0006](0006-gpui-base-table-columns.md) | Size Markdown table columns in the gpui-base fork | accepted | Two fork commits on `knot/table-columns-0.7.0`: CSS auto table layout from min-/max-content widths, cut where gpui wraps | 2026-11-12 |
| [0007](0007-acp-steering-extension.md) | Steer mid-turn prompts through the `_session/steering` ACP extension | accepted | Steer only when `initialize` advertises `_meta.steering.supported`, with `idleBehavior: promptRequired`, falling back to the queue; move to ACP v2's mid-turn `session/prompt` once it lands | 2026-11-13 |
| [0008](0008-croner-for-cron-schedules.md) | Evaluate cron schedules with `croner` over `jiff` | accepted | `croner` 4 with only its `jiff` feature (no `chrono`), five-field parsing, presets evaluated through the same `next_run`; hand-rolled evaluator rejected | 2026-11-16 |
| [0009](0009-gpui-pre-menu-alternates.md) | Fork gpui-pre for native alternate menu items | accepted | `[patch.crates-io]` fork of the published 0.3.7 sources plus one commit (`MenuItem::alternate`, `setAlternate:`) until upstream supports alternates | 2026-11-25 |
| [0010](0010-scoped-mcp-credentials.md) | Authenticate the MCP server with separately scoped credentials | accepted | Per-launch agent tool and hook credentials plus one Keychain client token; identity from the credential; confirmation for MCP-created commands and distant worktrees | 2026-11-25 |
