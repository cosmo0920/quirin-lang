#!/usr/bin/env bash
# artifacts.sh BEFORE AFTER -- CG08-CASTS-AND-TYPE-QUERIES: the report's
# artifacts from two measure runs under private/cg08 (NAME, NAME-parity.jsonl,
# NAME-system, NAME-system-parity.jsonl, the tail-pause slice cg08-NAME).
# BEFORE's corpus run may be a directory with out/ (the first BEFORE).
set -u
before="$1"; after="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
p=private/cg08
a=docs/reports/cg08-casts-and-type-queries/artifacts
s=docs/reports/cg08-casts-and-type-queries/scripts
slices=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1
mkdir -p "$a"
corpus_before="$p/$before"; [ -d "$p/$before/out" ] && corpus_before="$p/$before/out"
parity_before="$p/$before-parity.jsonl"; [ -d "$p/$before/out" ] && parity_before="$p/$before/out/parity.jsonl"
python3 "$s/compare.py" "$corpus_before" "$p/$after" --json "$a/corpus-compare.json" > /dev/null
python3 "$s/changed.py" "$corpus_before" "$p/$after" --json "$a/corpus-changed.json" > /dev/null
python3 "$s/attribute.py" "$a/corpus-changed.json" "$parity_before" "$p/$after-parity.jsonl" --json "$a/corpus-attribution.json"
python3 "$s/families.py" "$corpus_before" --json "$a/families-before.json" > /dev/null
python3 "$s/families.py" "$p/$after" --json "$a/families-after.json" > /dev/null
python3 "$s/cast_refusals.py" "$parity_before" --json "$a/cast-refusals-before.json"
python3 "$s/cast_refusals.py" "$p/$after-parity.jsonl" --json "$a/cast-refusals-after.json"
python3 "$s/callee_census.py" "$p/$after-parity.jsonl" --json "$a/callee-census.json"
python3 "$s/diagnostic_totals.py" "$corpus_before" "$p/$after" > "$a/diagnostic-totals.txt"
python3 "$s/compare.py" "$p/base-before-system/resource-diagnostics.json" "$p/$after-system/resource-diagnostics.json" --json "$a/system-compare.json" > /dev/null
python3 "$s/changed.py" "$p/base-before-system/resource-diagnostics.json" "$p/$after-system/resource-diagnostics.json" --json "$a/system-changed.json" > /dev/null
python3 "$s/attribute.py" "$a/system-changed.json" "$p/base-before-system-parity.jsonl" "$p/$after-system-parity.jsonl" --json "$a/system-attribution.json"
python3 "$s/compare.py" "$slices/cg08-base-before/resource-diagnostics.json" "$slices/cg08-$after/resource-diagnostics.json" --json "$a/slice-compare.json" > /dev/null
sha256sum "$corpus_before/resource-diagnostics.json" "$p/$after/resource-diagnostics.json" \
  "$p/base-before-system/resource-diagnostics.json" "$p/$after-system/resource-diagnostics.json" \
  "$slices/cg08-base-before/resource-diagnostics.json" "$slices/cg08-$after/resource-diagnostics.json" \
  | sed "s#$repo/##; s#/home/cosmo/Documents/GitHub/qcore-c/private/#<main>/private/#" > "$a/identity.txt"
cat "$p/base-before.binary.sha256" "$p/$after.binary.sha256" | sed "s#$repo/##" >> "$a/identity.txt"
