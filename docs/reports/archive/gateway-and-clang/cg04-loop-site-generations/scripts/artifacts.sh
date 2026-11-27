#!/usr/bin/env bash
# artifacts.sh -- CG04-LOOP-SITE-GENERATIONS: derive the report's artifacts
# from the card's measurement runs (private/cg04lsg of this worktree and the
# shared tail-pause attempt directory). Every exit is written to a file.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
runs="$repo/private/cg04lsg"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
out="$repo/docs/reports/cg04-loop-site-generations/artifacts"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
step() {
  name="$1"; shift
  "$@" > "$out/$name.txt" 2>&1
  echo $? > "$runs/artifact-$name.exit"
}
for route in "" "-system"; do
  step "exposure-before$route" python3 "$here/exposure.py" "$runs/before$route-parity.jsonl" "$runs/before$route" --json "$out/exposure-before$route.json"
  step "exposure-final$route" python3 "$here/exposure.py" "$runs/final$route-parity.jsonl" "$runs/final$route" --json "$out/exposure-final$route.json"
  step "withheld-final$route" python3 "$here/withheld.py" "$runs/final$route-parity.jsonl" "$runs/final$route" --json "$out/withheld-final$route.json"
  step "changed-step1$route" python3 "$here/changed.py" "$runs/before$route" "$runs/step1$route" --json "$out/changed-step1$route.json"
  step "changed-step2$route" python3 "$here/changed.py" "$runs/step1$route" "$runs/final$route" --json "$out/changed-step2$route.json"
  step "comparison-final$route" python3 "$here/compare.py" "$runs/before$route" "$runs/final$route" --json "$out/comparison-final$route.json"
done
step exposure-before-slice python3 "$here/exposure.py" "$runs/before-slice-parity.jsonl" "$slice/cg04lsg-before" --json "$out/exposure-before-slice.json"
step exposure-final-slice python3 "$here/exposure.py" "$runs/final-slice-parity.jsonl" "$slice/cg04lsg-final" --json "$out/exposure-final-slice.json"
step withheld-final-slice python3 "$here/withheld.py" "$runs/final-slice-parity.jsonl" "$slice/cg04lsg-final" --json "$out/withheld-final-slice.json"
step changed-step1-slice python3 "$here/changed.py" "$slice/cg04lsg-before" "$slice/cg04lsg-step1" --json "$out/changed-step1-slice.json"
step changed-step2-slice python3 "$here/changed.py" "$slice/cg04lsg-step1" "$slice/cg04lsg-final" --json "$out/changed-step2-slice.json"
step comparison-final-slice python3 "$here/compare.py" "$slice/cg04lsg-before" "$slice/cg04lsg-final" --json "$out/comparison-final-slice.json"
step totals python3 "$here/totals.py" "$runs/before" "$runs/step1" "$runs/final" "$runs/before-system" "$runs/step1-system" "$runs/final-system" "$slice/cg04lsg-before" "$slice/cg04lsg-step1" "$slice/cg04lsg-final"
cat "$runs"/*.binary.sha256 > "$out/binaries.txt"
echo $? > "$runs/artifact-binaries.exit"
