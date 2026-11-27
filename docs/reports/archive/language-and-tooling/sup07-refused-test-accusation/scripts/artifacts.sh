#!/usr/bin/env bash
# artifacts.sh -- copy SUP07-REFUSED-TEST-ACCUSATION's measurement summaries from
# private/sup07 into artifacts/: change lists, censuses, probe rows and digests.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
p="$repo/private/sup07"
a="$here/../artifacts"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$a/census" || exit 2
cd "$repo" || exit 2
for pair in before:after before:before2 before2:after2 before:after2; do
  b="${pair%%:*}"; c="${pair##*:}"
  python3 "$here/changed.py" "$p/$b" "$p/$c" --json "$a/changed-$b-$c.json" > /dev/null || exit 1
done
for run in before after before2 after2; do
  python3 "$here/census.py" "$p/$run/resource-diagnostics.json" > "$a/census/$run-corpus.json" || exit 1
  python3 "$here/census.py" "$p/$run-system/resource-diagnostics.json" > "$a/census/$run-system.json" || exit 1
  python3 "$here/census.py" "$slices/sup07-$run/resource-diagnostics.json" > "$a/census/$run-slice.json" || exit 1
  python3 "$here/probe_rows.py" "$p/probe-$run" > "$a/probe-$run-rows.json" || exit 1
done
python3 - "$p" "$slices" "$a/digests.json" <<'EOF'
import hashlib, json, pathlib, sys
p, slices, out = map(pathlib.Path, sys.argv[1:])
digest = lambda f: hashlib.sha256(f.read_bytes()).hexdigest()
rows = {}
for run in ("before", "after", "before2", "after2"):
    rows[f"{run}/corpus"] = digest(p / run / "resource-diagnostics.json")
    rows[f"{run}/system"] = digest(p / f"{run}-system" / "resource-diagnostics.json")
    rows[f"{run}/slice"] = digest(slices / f"sup07-{run}" / "resource-diagnostics.json")
    rows[f"{run}/binary"] = (p / f"{run}.binary.sha256").read_text().split()[0]
    head = p / f"{run}.head"
    rows[f"{run}/head"] = head.read_text().strip() if head.exists() else "cffe232d4b3ca8d8306cd4e953de74f105869dd9"
for census in ("census-before", "census-before-system"):
    rows[f"{census}/diagnostics"] = digest(p / census / "resource-diagnostics.json")
rows["census-before-slice/diagnostics"] = digest(slices / "sup07-census-before" / "resource-diagnostics.json")
out.write_text(json.dumps(rows, indent=1, sort_keys=True) + "\n")
EOF
