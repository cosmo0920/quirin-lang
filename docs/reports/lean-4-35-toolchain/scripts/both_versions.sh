#!/bin/bash
# LEAN-4-35-TOOLCHAIN: the three Lean runs, in one hold of the shared heavy lock:
#   base-434    the base's two renamed files restored, pin 4.34.0 (main's tree)
#   renamed-434 the renamed tree, pin 4.34.0 (before the pin moves)
#   renamed-435 the renamed tree, pin 4.35.0 (after the pin moves)
# Each run builds with `lake build` and runs every audit through
# scripts/check_lean_audits.py, whose version guard checks the active Lean
# against proof/lean-toolchain first. Versions are selected only through
# proof/lean-toolchain. Run as
#   flock <heavy.lock> bash both_versions.sh OUT BASE
# from the repository root, on a tree whose proof/lean-toolchain pins 4.34.0.
set -u
out="$1"; base="$2"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(git rev-parse --show-toplevel)"
files="proof/QCore/SummaryValidationPremises.lean proof/QCore/SourceRecord.lean"
mkdir -p "$out"
date -u +%FT%TZ > "$out/lock-acquired.txt"
for f in $files; do git -C "$root" show "$base:$f" > "$root/$f"; done
bash "$here/run_lean.sh" "$out/base-434"
git -C "$root" checkout HEAD -- $files
git -C "$root" diff --quiet HEAD -- $files || { echo "restore failed" >&2; exit 2; }
bash "$here/run_lean.sh" "$out/renamed-434"
printf 'leanprover/lean4:v4.35.0\n' > "$root/proof/lean-toolchain"
bash "$here/run_lean.sh" "$out/renamed-435"
date -u +%FT%TZ > "$out/lock-released.txt"
for run in base-434 renamed-434 renamed-435; do
  echo "$run: lean exit $(cat "$out/$run/lean.exit"), $(head -1 "$out/$run/lean-version.txt"), $(cat "$out/$run/audit-total.txt"), $(cat "$out/$run/wall-seconds.txt") s"
done
for pair in "base-434 renamed-434" "renamed-434 renamed-435"; do
  set -- $pair
  for list in axiom-lines.txt audit-passes.txt; do
    if cmp -s "$out/$1/$list" "$out/$2/$list"; then r=identical; else r=DIFFERENT; fi
    echo "$1 vs $2 $list: $r ($(wc -l < "$out/$1/$list") / $(wc -l < "$out/$2/$list") lines)"
  done
done
