#!/usr/bin/env bash
# arbiter_compare.sh MAINNAME TIPNAME -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: after arbiter.sh,
# compare main's and the tip's null streams per route (compare.py), check the
# movement against the first base's (rebase_identity.py, written to
# artifacts/rebase-identity.json and artifacts/compare-rebased-*.json), and
# print the custody report digests of both sides and the combined moves.
set -u
m="$1"; t="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/fbn"
a="$repo/docs/reports/fb-plugin-descriptor-nonnull/artifacts"
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
python3 "$here/compare.py" "$p/$m-null.jsonl" "$p/$t-null.jsonl" "$a/compare-rebased-corpus.json" > /dev/null || exit 1
python3 "$here/compare.py" "$p/$m-system-null.jsonl" "$p/$t-system-null.jsonl" "$a/compare-rebased-system.json" > /dev/null || exit 1
python3 "$here/compare.py" "$p/$m-slice-null.jsonl" "$p/$t-slice-null.jsonl" "$a/compare-rebased-slice.json" > /dev/null || exit 1
python3 "$here/rebase_identity.py" "$a/rebase-identity.json" \
  "corpus=$p/cmp-corpus.json,$a/compare-rebased-corpus.json" \
  "system=$p/cmp-system.json,$a/compare-rebased-system.json" \
  "slice=$p/cmp-slice.json,$a/compare-rebased-slice.json" > /dev/null || exit 1
sha256sum "$p/$m/resource-diagnostics.json" "$p/$t/resource-diagnostics.json" \
  "$p/$m-system/resource-diagnostics.json" "$p/$t-system/resource-diagnostics.json" \
  "$slice/fbn-$m/resource-diagnostics.json" "$slice/fbn-$t/resource-diagnostics.json"
for r in "" -system -slice; do
  python3 "$here/combined.py" "$p/$m$r-null.jsonl" "$p/$t$r-null.jsonl" | tail -1
done
python3 -c "
import json, sys
d = json.load(open(sys.argv[1]))
for r, v in d.items():
    print(r, v['same_moving_rows'], v['same_check_changes'], v['same_added_candidates'], v['custody_column_moves'], v['verdicts_main'], v['verdicts_tip'])
for name in sys.argv[2:]:
    s = json.load(open(name)); print(name.rsplit('/', 1)[-1], s['custody'])
" "$a/rebase-identity.json" "$p/$m-null.summary.json" "$p/$t-null.summary.json"
