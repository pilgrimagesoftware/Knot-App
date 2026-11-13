# Tasks

## 1. Let the user dismiss MCP servers they do not use (#402)

- [x] 1.1 Persist the dismissed server names as a preference
- [x] 1.2 List dismissed, unconnected servers last under a not-connected
      heading, without their fix action
- [x] 1.3 Leave dismissed servers out of the collapsed header's attention
      count and names
- [x] 1.4 Dismiss and restore controls on the agent's own rows
- [x] 1.5 Tests for row order, the header, and persistence across a restart
      and from an older preferences document
- [x] 1.6 Clear a dismissal when a probe reports the server connected, so a
      later lapse needs attention again (#417)
