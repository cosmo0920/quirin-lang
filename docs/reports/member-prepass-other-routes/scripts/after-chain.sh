#!/usr/bin/env bash
# after-chain.sh NAME -- MEMBER-PREPASS-OTHER-ROUTES: build the AFTER binary
# of the checked-out tree (build.sh NAME), measure it on every route with the
# null property (after.sh NAME), then the system route whole and
# checkpointed without it (checkpoint-system.sh NAME). Each step under the
# shared heavy lock inside its own script; step exits to private/mpor/.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor"
"$here/build.sh" "$name" > "$out/$name-build.out" 2>&1
echo $? > "$out/$name-build.step.exit"
[ "$(cat "$out/$name-build.step.exit")" = 0 ] || exit 1
"$here/after.sh" "$name" "$out/bin/$name"
"$here/checkpoint-system.sh" "$name" "$out/bin/$name" > "$out/$name-ckpt.out" 2>&1
echo $? > "$out/$name-ckpt.step.exit"
