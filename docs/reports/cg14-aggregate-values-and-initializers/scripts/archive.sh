#!/usr/bin/env bash
# archive.sh -- CG14-AGGREGATE-VALUES-AND-INITIALIZERS: copy the measurement
# records of private/cg14 (runs `before`, `after` = first AFTER at d4f80fd7,
# `after2` = AFTER at 7ebbdc45) into artifacts/. Every exit is checked.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
private="$repo/private/cg14"
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
out="$here/../artifacts"
mkdir -p "$out/after" "$out/after1"
cd "$repo"
python3 "$here/identity.py" before after2 > "$out/after/identity.json"
python3 "$here/identity.py" before after > "$out/after1/identity.json"
python3 "$here/census.py" "$private/after2-parity.jsonl" > "$out/after/census.json"
python3 "$here/census.py" "$private/after-parity.jsonl" > "$out/after1/census.json"
python3 "$here/residuals.py" "$private/after2-parity.jsonl" > "$out/after/residuals.json"
python3 "$here/census.py" "$private/after2-system-parity.jsonl" > "$out/after/census-system.json"
python3 "$here/census.py" "$private/after2-slice-parity.jsonl" > "$out/after/census-slice.json"
python3 "$here/entries.py" docs/reports/cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz \
  "$private/before" "$private/after2" > "$out/after/entries.json"
python3 "$here/compare.py" "$private/before" "$private/after2" --json "$private/c.json" > /dev/null
gzip -c "$private/c.json" > "$out/after/compare-corpus.json.gz"
python3 "$here/compare.py" "$private/before-system" "$private/after2-system" --json "$out/after/compare-system.json" > /dev/null
python3 "$here/compare.py" "$slices/cg14-before" "$slices/cg14-after2" --json "$out/after/compare-slice.json" > /dev/null
python3 "$here/leaves.py" "$private/before" "$private/after2" > "$private/l.json"
gzip -c "$private/l.json" > "$out/after/leaves-corpus.json.gz"
cat "$private/before.binary.sha256" "$private/after.binary.sha256" "$private/after2.binary.sha256" > "$out/binaries.sha256"
sha256sum "$private"/before/*.json "$private"/after2/*.json > "$out/after/sidecars.sha256"
echo archived
