# Proposal

## Why

An agent that read a `send-message` brief and then lost its conversation had
no record of the work it was given (#405). `check-messages` marks messages read
by default and returns only unread ones, so after a restart into a new
conversation the inbox was empty, even with `markAsRead: false`. The new
conversation reported no messages and no work, although it had already done
part of the work. The `resetContext` guard from #331 covers only tasks sent
with `dispatch-task`, so a plain brief had no protection from a reset.

## What Changes

1. **A reset Knot starts waits for open threads** (issue item 1). A plan step's
   fresh-context restart waits while the agent holds a message it has read and
   not answered. "Answered" means the agent has written to that message's
   sender since reading it. A broadcast is never owed a reply, nor is a
   dispatched task's goal (the task graph tracks that work and `complete-task`
   writes no message back), nor a brief whose sender is gone.
2. **A new conversation gets a handoff note** (item 2). Restarting into a new
   conversation, whoever asks for it, saves the agent's last status and the
   senders it owed replies to. That note is added to the new conversation's
   first turn, after the registration request.
3. **An unanswered brief goes back in the inbox** (item 3). The same restart
   marks the agent's unanswered messages unread again, so the new conversation
   reads them in full through `check-messages`, and the idle nudge fires for
   them.
4. **`check-messages` can return read messages** (item 5). It gains
   `includeRead` (default false) and `since` (an RFC 3339 time, as `timestamp`
   is reported), and each listed message carries `read`. A check now marks read
   only the messages it returned, so one that `since` filtered out stays
   unread.

A restart that keeps the conversation changes nothing: the conversation that
read the brief is still there.

## Impact

- `knot-messaging`:
  - `Message` gains `read_at`, `broadcast` and `task`.
  - `dispatch-task` marks the goal it delivers as a task's.
  - `MessageStore` gains `for_agent`, `unanswered_for`, `has_unanswered`,
    `reopen_unanswered` and `mark_read_ids`.
  - `check` takes `CheckOptions`.
- `knot-mcp-tools`: `check-messages` reads `includeRead` and `since`;
  `MessageInfo` gains `read`.
- `knot-agent-launch`: `handoff_note` and `OpenThread`.
- `knot`: `restart_agents` builds the note and reopens the messages; the panel
  connect appends the note to the first turn; `restart_for_fresh_context` waits
  for open threads.
- Specs:
  - `mcp-tools`: MODIFIED "send-message, check-messages, broadcast-message".
  - `mcp-messaging`: MODIFIED "Message model" and "Check and mark-read", ADDED
    "A read message is owed a reply".
  - `agent-lifecycle`: ADDED "A new conversation inherits the old one's open
    threads" and "Knot's own reset waits for open threads".
- Not in this change: telling the sender that a recipient's conversation was
  reset (issue item 4), listed as a follow-up in `tasks.md`.
