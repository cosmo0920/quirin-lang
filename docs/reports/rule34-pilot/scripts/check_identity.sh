#!/bin/sh
# RULE34-PILOT: the identity beyond the measure tool's byte comparison. BASE is
# the published baseline:80c8aea98 and TIP is built from the card's code
# revision, so the analyzer revision and the digests derived from it differ.
# For each route, every report artifact that is not byte-identical is checked
# with substitution.py (RULE37-PILOT's checker: equal after normalising the
# revision and every 24-64 hex digest, lists as multisets, and the differing
# values form one bijection per route). Every route's artifacts are also
# checked with substitution_ids.py (RULE35-PILOT's stricter pairing, extended to the
# 40-hex revision). Console and stderr logs and invocation records are not
# report artifacts (they name paths and times).
# Usage: check_identity.sh WORKTREE MEASURE_OUT OUT_PREFIX
set -u
W=$1
M=$2
OUT=$3
BASE_REV=80c8aea989029c10b3dfbd0179715b7ea050262c
TIP_REV=a78c190dd70ab68dba99bf919fbe5696642e8fdc
BL=/media/Data/GitHub/quirin-lang/private/baselines/$BASE_REV
cd "$W" || exit 2
S=docs/reports/rule34-pilot/scripts
for pair in system:fluent-bit-system slice:fluent-bit-tail-pause corpus:eight-home; do
  route=${pair%%:*}
  profile=${pair#*:}
  B=$BL/$profile/$route
  T=$M/sides/tip/$route
  files=$(python3 - "$B" "$T" <<'EOF'
import filecmp, os, sys
b, t = sys.argv[1:]
skip = {"console.log", "stderr.log", "invocation.json", "resource-diagnostics.invocation.json"}
out = []
for root, _, names in os.walk(b):
    for name in names:
        rel = os.path.relpath(os.path.join(root, name), b)
        if os.path.basename(rel) in skip or rel.endswith(".exit"):
            continue
        other = os.path.join(t, rel)
        if not os.path.exists(other) or not filecmp.cmp(os.path.join(b, rel), other, shallow=False):
            out.append(rel)
print(" ".join(sorted(out)))
EOF
)
  echo "$route differing: $files" >"$OUT-$route-files.txt"
  python3 $S/substitution.py "$BASE_REV" "$TIP_REV" "$B" "$T" $files >"$OUT-$route.json"
  echo $? >"$OUT-$route.exit"
  args=""
  for f in $files; do args="$args $B/$f $T/$f"; done
  python3 $S/substitution_ids.py $args >"$OUT-$route-ids.jsonl"
  echo $? >"$OUT-$route-ids.exit"
done
