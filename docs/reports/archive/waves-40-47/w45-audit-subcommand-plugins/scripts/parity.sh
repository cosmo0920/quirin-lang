#!/usr/bin/env bash
# Side-by-side parity of `quirin-audit libevent prepare` with the retired
# `scripts/prepare_libevent_core_audit.py` on the pinned third_party/libevent.
#
# usage: parity.sh SCRATCH LOGS
#   SCRATCH  a fresh directory outside the repository for the build outputs
#   LOGS     where every command's output, exit and the comparisons go
#
# Pass 1 prepares into two different roots and compares after replacing the
# script's root with the plugin's. Pass 2 prepares into the same path, one
# after the other (the script's tree moved aside first), and compares bytes.
# Pass 3 runs the plugin with --write-profile and compares the regenerated
# profile with the committed one. The script is run in place; at a revision
# where it is already deleted it is restored from the base 37e1b82d for the
# run and removed again.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
scratch="$1"
logs="$2"
mkdir -p "$logs"
mkdir "$scratch" || exit 2
cd "$repo" || exit 2
export CARGO_INCREMENTAL=0
clang=/usr/lib/llvm-21/bin/clang
clangxx=/usr/lib/llvm-21/bin/clang++
script=scripts/prepare_libevent_core_audit.py
restored=0
if [ ! -e "$script" ]; then
  git show 37e1b82d:"$script" > "$script" || exit 2
  restored=1
fi
step() {
  local name="$1"; shift
  printf '%q ' "$@" > "$logs/$name.command"
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
host_files=(quirin-audit-libevent.report.json quirin-audit-libevent.report.invocation.json
            quirin-audit-libevent.report.stderr.log)

step build cargo build -p qcore-audit -p quirin-audit-plugin-libevent
step versions sh -c "$clang --version; cmake --version; git -C third_party/libevent rev-parse HEAD"

# Pass 1: different roots.
step p1-script python3 "$script" --output "$scratch/p1-script" --clang "$clang" --clangxx "$clangxx"
step p1-plugin target/debug/quirin-audit --plugin-path target/debug --output "$scratch/p1-plugin" \
  libevent prepare --clang "$clang" --clangxx "$clangxx"
step p1-compare python3 "$here/compare_trees.py" "$scratch/p1-script" "$scratch/p1-plugin" \
  "$logs/p1-compare.json" --normalize --host-files "${host_files[@]}"

# Pass 2: the same path, one after the other.
step p2-script python3 "$script" --output "$scratch/p2" --clang "$clang" --clangxx "$clangxx"
mv "$scratch/p2" "$scratch/p2-script"
step p2-plugin target/debug/quirin-audit --plugin-path target/debug --output "$scratch/p2" \
  libevent prepare --clang "$clang" --clangxx "$clangxx"
step p2-compare python3 "$here/compare_trees.py" "$scratch/p2-script" "$scratch/p2" \
  "$logs/p2-compare.json" --host-files "${host_files[@]}"
step p2-explain python3 "$here/explain_logs.py" "$scratch/p2-script" "$scratch/p2" \
  "$logs/p2-compare.json" "$logs/p2-explained.json" "$repo" "$scratch/p2"
cp "$scratch/p2/quirin-audit-libevent.report.json" "$logs/p2-plugin.report.json"
cp "$scratch/p2/quirin-audit-libevent.report.invocation.json" "$logs/p2-plugin.report.invocation.json"
for side in p2-script p2; do
  for file in libevent-core-audit.json.config libevent-build-provenance.json; do
    cp "$scratch/$side/libevent-clang-build/$file" "$logs/$side.$file"
  done
done

# Pass 3: --write-profile regenerates the committed profile byte for byte.
step p3-plugin target/debug/quirin-audit --plugin-path target/debug --output "$scratch/p3" \
  libevent prepare --clang "$clang" --clangxx "$clangxx" --write-profile
step p3-compare cmp contract/profiles/libevent-core/profile.json \
  "$scratch/p3/libevent-clang-build/profile.json"

# Protected paths: an output inside this checkout's build/cycle19 is refused
# before any build (the host creates the empty output directory; it is removed
# again). The third_party refusal is unit-tested only, so nothing is ever
# created inside the submodule.
step p4-refuse-cycle19 target/debug/quirin-audit --plugin-path target/debug \
  --output build/cycle19/libevent-parity-refusal libevent prepare
rm -f build/cycle19/libevent-parity-refusal/quirin-audit-libevent.report.*
rmdir build/cycle19/libevent-parity-refusal build/cycle19 build 2>/dev/null
step p4-submodule-clean git -C third_party/libevent status --porcelain --untracked-files=all

if [ "$restored" = 1 ]; then rm -f "$script"; fi
