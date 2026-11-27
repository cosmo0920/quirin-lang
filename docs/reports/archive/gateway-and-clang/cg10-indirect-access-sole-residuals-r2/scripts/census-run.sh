#!/usr/bin/env bash
# census-run.sh NAME CENSUS-BINARY -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 (from CG10-INDIRECT-ACCESS-SOLE-BLOCKERS): the corpus arbiter
# with the measurement-only `indirect-census` feature on (the classifier's own
# census streams: QCORE_INDIRECT_CENSUS, QCORE_CALL_ARGUMENT_CENSUS,
# QCORE_STATEMENT_CENSUS, the return census) and the parity stream, under the
# shared heavy lock. The census binary is built into private/cg10r2/target-census
# (`cargo build --release -p qcore-audit --bin quirin-audit --features
# qcore-c-gateway/indirect-census`) and placed at target/release/quirin-audit for
# the run only (run_pinned_audit.py runs that path); the measured binary is saved
# beside it and restored afterwards. Exits to files; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/cg10r2"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
cd "$repo" || exit 2
census_binary="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
flock "$lock" bash -c '
  set -u
  repo="$1"; out="$2"; name="$3"; census_binary="$4"
  cp "$repo/target/release/quirin-audit" "$out/$name.saved-binary" || exit 3
  cat "$census_binary" > "$repo/target/release/quirin-audit" || exit 3
  sha256sum "$repo/target/release/quirin-audit" > "$out/$name.binary.sha256"
  QCORE_TYPED_PATH_PARITY="$out/$name-parity.jsonl" \
  QCORE_INDIRECT_CENSUS="$out/$name-indirect.jsonl" \
  QCORE_CALL_ARGUMENT_CENSUS="$out/$name-call-argument.jsonl" \
  QCORE_STATEMENT_CENSUS="$out/$name-statement.jsonl" \
    python3 scripts/run_pinned_audit.py "$out/$name" 66910c10 \
    --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19 --jobs 4 --census \
    > "$out/$name.log" 2>&1
  echo $? > "$out/$name.exit"
  cat "$out/$name.saved-binary" > "$repo/target/release/quirin-audit"
  sha256sum "$repo/target/release/quirin-audit" > "$out/$name.restored.sha256"
' _ "$repo" "$out" "$name" "$census_binary"
