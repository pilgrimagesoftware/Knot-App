# Tasks

## 1. Branch setup

- [x] 1.1 Create the worktree/feature branch from PR #579's branch (not
  `develop` — `relative_timestamp`/`render_timestamp` don't exist on
  `develop` yet) and verify `git branch --show-current` and `git merge-base`
  confirm the expected ancestry.

  PR #579 had already merged into `develop` by the time this change
  started (its merge commit is an ancestor of `origin/develop`'s tip), so
  the worktree branches from `develop` directly instead of from PR #579's
  now-closed branch.

## 2. Thread the hold state into the panel

- [x] 2.1 Add `cmd_held: bool` to `panel_view::message::Message<'a>`, set by
  its caller from `workspace.key_hints.shown().is_some()`, and verify
  `cargo check -p knot` passes with the new field wired through every call
  site.
- [x] 2.2 Gate `render_timestamp`'s call in both the `PanelMessage::User` and
  `PanelMessage::Assistant` arms of `render_message` behind
  `ctx.cmd_held`, so the `.children(...)` `Option` is `None` (no layout
  space) whenever the hold is not showing, and verify by reading the
  updated `render_message` diff against the `#577` version — the `Option`
  chain should now test `cmd_held` as well as `state.sent_at.get(index)`.

## 3. Tests

- [x] 3.1 Add a test alongside `relative_timestamp_buckets_by_elapsed_time`
  in `message.rs`'s `#[cfg(test)] mod tests` asserting `render_message`
  with `cmd_held: false` produces no timestamp child, and with
  `cmd_held: true` produces one, and verify `cargo test -p knot
  message::tests` passes.

  `message.rs` has no `#[cfg(test)] mod tests` of its own — its tests live
  in a sibling file under `panel_view/`, following the module's existing
  `shimmer_tests.rs`/`tests.rs` split. Added
  `no_timestamp_paints_while_cmd_is_not_held` and
  `a_timestamp_paints_while_cmd_is_held` in a new
  `panel_view/message_tests.rs` (declared in `mod.rs` beside the other
  two): they draw `render_message` into a real test window and check a
  new `panel-message-timestamp` debug selector on `render_timestamp`'s
  div. Put in their own file rather than appended to `tests.rs` because
  that pushed it to 740 lines, over the workspace's 700-line cap.
  Verified with `cargo test -p knot panel_view::message_tests::`.
- [x] 3.2 Extend `workspace_window/key_hints_tests.rs` (or add a sibling
  test) covering that a ⌘ hold recorded by `KeyHintHold` is visible to a
  panel-side reader via `shown().is_some()`, matching the sidebar's own
  assertions, and verify the test passes under `cargo test -p knot
  key_hints`.

## 4. Verification and cleanup

- [x] 4.1 Run `make fmt-check`, `make size-check`, `make lint`, `make test`,
  `make build` and verify all five pass.

  `fmt-check`, `size-check`, `test`, and `build` all pass clean.
  `make lint` fails, but on one pre-existing error unrelated to this
  change — `crates/knot/src/workspace_window/creation.rs:76` trips
  `clippy::needless-borrows-for-generic-args` under the currently
  installed clippy. Confirmed by running `make lint` on an unmodified
  `develop` checkout: same single failure, same line, before this
  change touched anything. Not fixed here since it is out of this
  change's scope; logged in `~/code/papercuts.md`.
- [x] 4.2 Manually run the app (`cargo run -p knot` or `make run` if
  defined), send a prompt, hold ⌘ over the panel, and verify timestamps
  appear after the sidebar's own key-hint delay and disappear immediately
  on release — matching this change's spec scenarios.

  Verified manually by the user: holding ⌘ reveals timestamps after the
  sidebar's own delay and releasing hides them immediately.
