#!/bin/bash
# Posts a hook event to Knot's MCP server.
#
# Reads the server's base URL from KNOT_MCP_URL and the agent's hook
# credential from KNOT_MCP_TOKEN, both set by Knot in the agent's
# environment. The credential goes to curl on standard input (`-K -`), so it
# never appears on a command line another process could list.
#
# Usage: knot_post <path> <json-body>   (prints the response body)

knot_post() {
  local path="$1"
  local body="$2"
  if [ -z "$KNOT_MCP_URL" ] || [ -z "$KNOT_MCP_TOKEN" ]; then
    return 1
  fi
  printf 'header = "Authorization: Bearer %s"\n' "$KNOT_MCP_TOKEN" |
    curl -s -K - -X POST \
      -H "Content-Type: application/json" \
      -d "$body" \
      "${KNOT_MCP_URL}${path}" 2>/dev/null
}
