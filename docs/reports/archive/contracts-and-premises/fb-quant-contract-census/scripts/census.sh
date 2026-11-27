#!/usr/bin/env bash
# census.sh -- FB-QUANT-CONTRACT-CENSUS: classify the three BEFORE runs of
# measure.sh (corpus private/fqc/before, system private/fqc/before-system,
# slice private/w47-tail/attempt-1/fqc-before) into private/fqc/*-classified.json,
# map the obligations, and write the committed artifacts (artifacts/*.json,
# the per-row leaf dump omitted) with summarize.py. Exits to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/fqc"
cd "$repo" || exit 2
python3 "$here/source_census.py" > "$out/source-census.log" 2>&1; echo $? > "$out/source-census.exit"
python3 "$here/classify.py" corpus "$out/before/resource-diagnostics.json" "$out/before-null.jsonl" \
  /home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/src \
  "$out/corpus-classified.json" flb_input_chunk.c > "$out/classify-corpus.log" 2>&1
echo $? > "$out/classify-corpus.exit"
python3 "$here/classify.py" system "$out/before-system/resource-diagnostics.json" "$out/before-system-null.jsonl" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source \
  "$out/system-classified.json" > "$out/classify-system.log" 2>&1
echo $? > "$out/classify-system.exit"
python3 "$here/classify.py" slice /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/fqc-before/resource-diagnostics.json \
  "$out/before-slice-null.jsonl" \
  /home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source \
  "$out/slice-classified.json" > "$out/classify-slice.log" 2>&1
echo $? > "$out/classify-slice.exit"
python3 "$here/obligations.py" "$out/system-classified.json" "$out/slice-classified.json" \
  "$repo/docs/reports/fb-quant-contract-census/artifacts/obligations.json" > "$out/obligations.log" 2>&1
echo $? > "$out/obligations.exit"
python3 "$here/summarize.py" "$out" > "$out/summarize.log" 2>&1
echo $? > "$out/summarize.exit"
cat "$out"/source-census.exit "$out"/classify-*.exit "$out"/obligations.exit "$out"/summarize.exit
