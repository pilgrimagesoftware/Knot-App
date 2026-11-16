# Tasks

## 1. Flag only replies that ask the user something

- [x] 1.1 Say in the classification prompt what counts as asking, with
      "completed" examples of reports that need nothing

## 2. Say why an agent waits

- [x] 2.1 Store a reason with Awaiting input, cleared when the agent leaves it
- [x] 2.2 Record Autopilot's reason in `land`, the permission title in the
      panel tracker, the hook message in the hook tracker
- [x] 2.3 Show it in the state dot's tooltip and on the dashboard card

## 3. Take a resolved permission off screen whichever way it was resolved

- [x] 3.1 Report every answered permission request from the ACP client
- [x] 3.2 Drop it from the panel by id; clear all when the session ends
