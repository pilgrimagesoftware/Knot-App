# Proposal

## Why

There was no way to keep one agent out of agent-to-agent coordination short of
deactivating or removing it. An agent being used on its own - or one the user
is driving by hand - still received messages from the rest of the knot, was
started by them when stopped, and could message others (#310).

## What Changes

- An agent can be locked and unlocked from its context menu and the menu
  bar's Agents menu. The lock is durable.
- A locked agent cannot be messaged: a direct send is refused, so it is not
  started by messaging; broadcast skips it; `dispatch-task` passes over it.
- A locked agent cannot message others: its sends are refused and its
  broadcasts reach no one.
- The user's Broadcast to All Agents skips locked agents; the sidebar row
  shows a Locked line; `list-agents`/`describe-agents` report `locked`.

## Impact

- `knot-core`: `SavedAgent::locked`.
- `knot-agents`: `Agent::locked`, `AgentStore::set_locked`,
  `RegistryStatus::Live::locked`.
- `knot-messaging`: `SendError::{SenderLocked, RecipientLocked}`.
- `knot-mcp-tools`: `AgentInfo::locked`; dispatch skips locked candidates.
- `knot`: Lock/Unlock menu entries and actions, the sidebar line, the user
  broadcast.
