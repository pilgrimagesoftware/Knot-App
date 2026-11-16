# Tasks

## 1. Keep activated and running in step

- [x] 1.1 Decide which agents to start (activated, no session, not already
      started this activation) as a pure function; verify with unit tests
- [x] 1.2 Start them from the repaint poll and forget the claim on teardown;
      verify an agent created `active` into an open workspace is started, once,
      and a `passive` one is not
