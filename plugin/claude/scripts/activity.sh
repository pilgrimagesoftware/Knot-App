#!/bin/bash

# Hook-based activity detection for Knot
# Called by Claude hooks to report agent status changes
# Usage: activity.sh <HookName> <running|idle|input>

source "$(dirname "$0")/log.sh"
source "$(dirname "$0")/post.sh"

hook="$1"
status="$2"
input=$(cat)

skwad_log "Activity" "hook=$hook status=$status agent_id=$KNOT_AGENT_ID"
skwad_log "Activity" "payload=$input"

if [ -z "$hook" ] || [ -z "$status" ] || [ -z "$KNOT_AGENT_ID" ]; then
  exit 0
fi

# Fire and forget — don't block the agent
# Forward raw hook payload for server-side metadata extraction
knot_post /api/v1/agent/status \
  "{\"agent_id\":\"${KNOT_AGENT_ID}\",\"agent\":\"claude\",\"hook\":\"${hook}\",\"status\":\"${status}\",\"payload\":${input:-\{\}}}" \
  >/dev/null &

exit 0
