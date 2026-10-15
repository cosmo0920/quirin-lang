#!/bin/bash
# Build the Lean project and run every audit with the toolchain proof/lean-toolchain
# selects, recording the active version, the pin, the transcript and the exit.
# Usage: run_lean.sh OUT_DIR [extra check_lean_audits.py args...]
# Run from the repository root. Heavy: hold the shared heavy lock around it.
set -u
out="$1"; shift
mkdir -p "$out"
root="$(git rev-parse --show-toplevel)"
git -C "$root" rev-parse HEAD > "$out/head.txt"
git -C "$root" status --short -- proof scripts > "$out/tree-status.txt"
cat "$root/proof/lean-toolchain" > "$out/lean-toolchain.txt"
(cd "$root/proof" && lake env lean --version) > "$out/lean-version.txt" 2>&1
echo $? > "$out/lean-version.exit"
start=$(date +%s)
python3 "$root/scripts/check_lean_audits.py" "$@" > "$out/lean.log" 2>&1
echo $? > "$out/lean.exit"
end=$(date +%s)
echo $((end - start)) > "$out/wall-seconds.txt"
# The audit output list: every axiom line AuditAll prints, plus each audit's pass line
# without its timing, so two runs compare by content.
grep -E "depends on axioms|does not depend on any axioms" "$out/lean.log" | sort > "$out/axiom-lines.txt"
grep -E '^Passed Audit' "$out/lean.log" | sed -E 's/ in [0-9.]+s$//' > "$out/audit-passes.txt"
grep -E '^Passed [0-9]+ Lean audits' "$out/lean.log" > "$out/audit-total.txt"
echo "lean exit $(cat "$out/lean.exit"); $(cat "$out/lean-version.txt" | head -1); $(cat "$out/audit-total.txt")"
