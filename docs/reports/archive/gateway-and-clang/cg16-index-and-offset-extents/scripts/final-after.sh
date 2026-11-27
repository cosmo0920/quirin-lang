#!/bin/sh
# CG16-INDEX-AND-OFFSET-EXTENTS: the final AFTER on the newest main, under the
# integrator's shared heavy-step lock: build the release quirin-audit, run the
# corpus arbiter with the bounds property selected (scripts/corpus.sh), then
# remove the build output. usage: final-after.sh LOCK NAME. Every exit is
# captured to a file; nothing is piped.
set -u
lock="$1"; name="$2"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
out="$root/private/cg16ix"
mkdir -p "$out"
cd "$root" || exit 2
CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 flock "$lock" sh -c '
  cargo build --release -p qcore-audit --bin quirin-audit > "$1/build-$2.log" 2>&1
  echo $? > "$1/build-$2.exit"
  bash "$3/corpus.sh" "$2"
' final "$out" "$name" "$here"
cargo clean > "$out/clean-$name.log" 2>&1
echo $? > "$out/clean-$name.exit"
