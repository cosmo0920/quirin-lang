#!/usr/bin/env bash
# build-measure.sh NAME -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: build this
# worktree's release quirin-audit under the shared heavy lock, save its bytes as
# private/hdd/bin/quirin-audit-NAME, then measure the corpus, system and slice
# routes with the null property selected and no parity stream
# (measure-binary.sh). Every exit is in private/hdd/*.exit; nothing is piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$repo/private/hdd/bin" || exit 2
git -C "$repo" rev-parse HEAD > "$repo/private/hdd/$name.head"
bash "$here/heavy.sh" "build-$name" cargo build --release -p qcore-audit --bin quirin-audit || exit 3
cp "$repo/target/release/quirin-audit" "$repo/private/hdd/bin/quirin-audit-$name" || exit 4
NULL=1 PARITY=0 bash "$here/measure-binary.sh" "$name" "$repo/private/hdd/bin/quirin-audit-$name"
echo done > "$repo/private/hdd/$name.measured"
