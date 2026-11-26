#!/bin/bash

source "$(dirname "$0")/log.sh"
source "$(dirname "$0")/post.sh"

input=$(cat)
source_type=$(echo "$input" | jq -r '.source')
session_id=$(echo "$input" | jq -r '.session_id')
cwd=$(pwd)

skwad_log "SessionStart" "agent_id=$KNOT_AGENT_ID session_id=$session_id source=$source_type cwd=$cwd"
skwad_log "SessionStart" "payload=$input"

# Process startup and resume events (others are ignored)
if [ "$source_type" != "startup" ] && [ "$source_type" != "resume" ]; then
  exit 0
fi

# Need the agent ID and Knot's server to register
if [ -z "$KNOT_AGENT_ID" ] || [ -z "$KNOT_MCP_URL" ]; then
  exit 0
fi

# Check if Knot's MCP server is running (`/health` needs no credential)
health=$(curl -s -o /dev/null -w "%{http_code}" "$KNOT_MCP_URL/health" 2>/dev/null)

if [ "$health" = "200" ]; then

  # Register agent — forward raw hook payload + source for server-side handling
  register_response=$(knot_post /api/v1/agent/register \
    "{\"agent_id\":\"${KNOT_AGENT_ID}\",\"agent\":\"claude\",\"source\":\"${source_type}\",\"session_id\":\"${session_id}\",\"payload\":${input:-\{\}}}")

  skwad_log "SessionStart" "register_response=$register_response"

  # Only return context to Claude on startup (not resume)
  if [ "$source_type" = "startup" ]; then
    # Build context from register response (skwad members)
    members=$(echo "$register_response" | jq -r '.skwadMembers // [] | map("\(.name) (\(.id)) - \(.folder) [\(.status)]") | join(", ")' 2>/dev/null)

    context="Knot detected at $KNOT_MCP_URL. You are registered with the knot. Your agent ID is: ${KNOT_AGENT_ID}."
    if [ -n "$members" ] && [ "$members" != "" ]; then
      context="$context Your skwad members: $members."
    fi

    # Escape for JSON
    context=$(echo "$context" | sed 's/"/\\"/g')

    echo "{ \"hookSpecificOutput\": { \"hookEventName\": \"SessionStart\", \"additionalContext\": \"$context\" } }"
  fi
else
  if [ "$source_type" = "startup" ]; then
    echo "{ \"hookSpecificOutput\": { \"hookEventName\": \"SessionStart\", \"additionalContext\": \"Register in the skwad using your agent ID and session ID.\" } }"
  fi
fi

exit 0
