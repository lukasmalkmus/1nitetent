#!/usr/bin/env bash
input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)
if [[ -z "$command" ]] || ! echo "$command" | grep -q '1nt'; then
  exit 0
fi
session_id=$(echo "$input" | jq -r '.session_id // empty' 2>/dev/null)
marker="${TMPDIR:-/tmp}/.1nt-skill-nudge-${session_id:-$PPID}"
[ -f "$marker" ] && exit 0
touch "$marker"
nudge='<system-reminder>The "1nitetent" skill provides guided 1nt workflows. Invoke it with /1nitetent or the Skill tool.</system-reminder>'
jq -n --arg nudge "$nudge" '{
  hookSpecificOutput: {
    hookEventName: "PostToolUse",
    additionalContext: $nudge
  }
}'
