#!/usr/bin/env bash
# run.sh NAME -- OUTER-CONTRACT: build the release quirin-audit of the
# checked-out tree (build.sh, under the heavy lock), measure it on the corpus,
# system and slice routes with the null property and the runtime projection
# (measure.sh, each route under the heavy lock), then delete the private
# release target and the copy measure.sh writes to target/release (the disk
# rule). Outputs go to private/oc/. Exits to files; nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/oc"
mkdir -p "$out" || exit 2
bash "$here/build.sh" "$name"
echo $? > "$out/run-build-$name.exit"
if [ "$(cat "$out/run-build-$name.exit")" = 0 ]; then
  bash "$here/measure.sh" "$name" "$out/bin/$name" corpus,system,slice
  echo $? > "$out/run-measure-$name.exit"
fi
rm -rf "$out/target-rel" "$repo/target/release/quirin-audit"
echo done > "$out/run-$name.done"
