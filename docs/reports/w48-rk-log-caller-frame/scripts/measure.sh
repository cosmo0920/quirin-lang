#!/usr/bin/env bash
# measure.sh NAME -- W48-RK-LOG-CALLER-FRAME (adapted from
# HANDOFF-DIRECT-CALL-READING's scripts/measure.sh): the pinned corpus arbiter
# with this worktree's release quirin-audit (target/release/quirin-audit, as
# scripts/run_pinned_audit.py requires) into a fresh private/rkcf/NAME, under
# the shared card validation lock, --jobs 4, with typed-path parity rows beside
# it (NAME-parity.jsonl). The route's wall time is measured inside the lock.
# Every exit is written to a file; nothing is piped. The system and tail-pause
# selections hold no librdkafka unit (contract/profiles/fluent-bit/*-selection.json),
# so only the corpus route can see rd_kafka_log0.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="${REPO:-$(cd "$here/../../../.." && pwd)}"
out="$repo/private/rkcf"
lock=/tmp/qcore-c-card-validation-20261022.lock
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
sha256sum target/release/quirin-audit > "$out/$name.binary.sha256"
QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" flock "$lock" /usr/bin/env bash -c '
  out="$1" name="$2"
  start=$EPOCHREALTIME
  python3 scripts/run_pinned_audit.py "$out/$name" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 \
    > "$out/$name.log" 2> "$out/$name.stderr"
  rc=$?
  end=$EPOCHREALTIME
  echo "$rc" > "$out/$name.exit"
  awk -v s="$start" -v e="$end" "BEGIN { printf \"%.3f\n\", e - s }" > "$out/$name.seconds"
' _ "$out" "$name"
