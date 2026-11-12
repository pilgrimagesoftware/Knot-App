# Tasks

## 1. Messages know when they were read and whether they are owed a reply

- [x] 1.1 Record `read_at` when a message is marked read, and mark broadcast
      copies as broadcasts
- [x] 1.2 `unanswered_for` / `has_unanswered`: read, direct, from another
      agent, and no message back to its sender since it was read
- [x] 1.3 `reopen_unanswered`: mark them unread again and hand them back

## 2. check-messages returns read messages on request

- [x] 2.1 `includeRead` and `since` (RFC 3339), with `read` on each message;
      a malformed `since` is an error
- [x] 2.2 Mark read only the messages a check returned

## 3. A reset keeps the work in hand

- [x] 3.1 A restart into a new conversation reopens unanswered messages and
      leaves a handoff note (last status and senders owed a reply)
- [x] 3.2 The next panel connect appends the note to its first turn
- [x] 3.3 A fresh-context restart waits while the agent has an unanswered
      message

## 4. Tests

- [x] 4.1 Store: unread is not owed, a reply settles it, writing to a third
      agent does not, broadcasts are never owed, reopening restores it,
      `includeRead`/`since` filtering
- [x] 4.2 Tool: `includeRead` returns a read brief with `read: true`; `since`
      filters and leaves the filtered message unread; a malformed `since`
      errors
- [x] 4.3 Window: a new conversation gets the brief back and a note; keeping
      the conversation hands nothing off; an answered brief stays read; a
      fresh context waits for the reply
- [x] 4.4 Mutation-check the deferral and the handoff

## 5. Follow-ups

- [ ] 5.1 Tell the sender that a recipient's conversation was reset and its
      brief reopened (#405 item 4)
