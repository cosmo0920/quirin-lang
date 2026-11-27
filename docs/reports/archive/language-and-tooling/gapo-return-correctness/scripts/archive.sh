#!/usr/bin/env bash
# archive.sh -- GAPO-RETURN-CORRECTNESS: copy the comparison outputs of the
# BEFORE, task-a and task-ab runs (private/gapr) into artifacts/. Outputs to
# files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/gapr"
a="$here/../artifacts"
mkdir -p "$a" || exit 2
cd "$repo" || exit 2
for run in task-a task-ab; do
  for route in corpus system slice; do cp "$p/compare-$run-$route.json" "$a/" || exit 3; done
  python3 "$here/runs.py" before "$run" > "$a/runs-$run.txt" 2>&1 || exit 4
  python3 "$here/parity.py" "$p/before-parity.jsonl" "$p/$run-parity.jsonl" "$a/parity-$run.json" > "$a/parity-$run.txt" || exit 5
  python3 "$here/parity.py" "$p/before-system-parity.jsonl" "$p/$run-system-parity.jsonl" > "$a/parity-$run-system.txt" || exit 5
  python3 "$here/parity.py" "$p/before-slice-parity.jsonl" "$p/$run-slice-parity.jsonl" > "$a/parity-$run-slice.txt" || exit 5
done
python3 "$here/attribute.py" "$a/parity-task-ab.json" "$p/before" "$p/task-ab" > "$a/attribute-task-ab.txt" || exit 6
python3 "$here/residual_casts.py" "$p/task-ab-parity.jsonl" > "$a/residual-casts.txt" || exit 7
for name in before task-a task-ab; do
  for suffix in "" -system -slice; do
    printf '%s%s %s seconds\n' "$name" "$suffix" "$(cat "$p/$name$suffix.seconds")"
  done
done > "$a/seconds.txt"
cat "$p"/bin/*.sha256 > "$a/binaries.sha256"
cp "$p/before-null.summary.json" "$a/before-null.summary.json"
echo ok
