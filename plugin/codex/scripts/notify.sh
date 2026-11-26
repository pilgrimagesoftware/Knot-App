#!/bin/bash

# Codex notify hook for Knot
# Called by Codex with a single JSON argument ($1) on agent-turn-complete
# Usage: notify.sh '<json-payload>'

source "$(dirname "$0")/log.sh"
source "$(dirname "$0")/post.sh"

json="$1"
event_type=$(echo "$json" | jq -r '.type')

skwad_log "Notify" "event_type=$event_type agent_id=$KNOT_AGENT_ID"
skwad_log "Notify" "payload=$json"

if [ -z "$KNOT_AGENT_ID" ]; then
  exit 0
fi

case "$event_type" in
  agent-turn-complete)
    # agent-turn-complete = agent finished its turn, now idle
    knot_post /api/v1/agent/status \
      "{\"agent_id\":\"${KNOT_AGENT_ID}\",\"agent\":\"codex\",\"hook\":\"notify\",\"status\":\"idle\",\"payload\":${json}}" \
      >/dev/null &
    ;;
esac

exit 0
