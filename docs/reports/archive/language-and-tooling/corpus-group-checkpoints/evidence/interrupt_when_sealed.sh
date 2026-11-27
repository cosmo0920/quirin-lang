#!/usr/bin/env bash
# Kill the interrupted evidence run (only the quirin-audit process whose
# arguments name this card's private qo-two directory) once MARK is sealed.
set -u
WT=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a50c061771c033423
LOG=$WT/private/evidence/fresh/stderr.log
MARK=$1
until grep -q "checkpoint ready: .*$MARK" "$LOG" 2>/dev/null; do sleep 2; done
for p in /proc/[0-9]*; do
  cmd=$(tr '\0' ' ' < "$p/cmdline" 2>/dev/null) || continue
  case "$cmd" in
    "$WT/private/bin/quirin-audit pinned"*"$WT/private/evidence/qo-one"*)
      pid=${p#/proc/}
      echo "killing $pid at $(date --iso-8601=seconds): $cmd" > "$WT/private/evidence/fresh/kill.txt"
      kill -9 "$pid"
      ;;
  esac
done
