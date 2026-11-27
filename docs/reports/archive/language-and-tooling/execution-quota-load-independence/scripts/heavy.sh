#!/bin/sh
# Run a command under the shared heavy-step lock (resolved by glob).
LOCK=$(ls -d /tmp/claude-1000/*/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock | head -1)
exec flock "$LOCK" "$@"
