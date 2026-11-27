#!/usr/bin/env bash
# gate-run.sh NAME [PART] -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: take the
# shared heavy lock once and run gate.sh NAME PART under it (HELD=1), then
# clean the dev profile under the lock again. Lock hold times go to
# private/rkou/gate-NAME-lock.txt.
set -u
name="$1"
part="${2:-all}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$repo/private/rkou/gate-$name" || exit 2
flock "$lock" bash -c '
  echo "lock held from $(date -u +%FT%T.%NZ)" >> "$1"
  HELD=1 bash "$2" "$3" "$4"
  rc=$?
  echo "gate.sh $4 exit $rc, lock released at $(date -u +%FT%T.%NZ)" >> "$1"
' _ "$repo/private/rkou/gate-$name-lock.txt" "$here/gate.sh" "$name" "$part"
if [ "$part" != docs ]; then
  cd "$repo" || exit 2
  flock "$lock" cargo clean --profile dev > "$repo/private/rkou/gate-$name/07z-clean.log" 2>&1
  echo $? > "$repo/private/rkou/gate-$name/07z-clean.exit"
fi
grep -H . "$repo/private/rkou/gate-$name"/*.exit
