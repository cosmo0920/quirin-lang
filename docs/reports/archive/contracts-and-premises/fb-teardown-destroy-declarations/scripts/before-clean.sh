#!/usr/bin/env bash
# before-clean.sh NAME -- FB-TEARDOWN-DESTROY-DECLARATIONS: the BEFORE
# measurement from a clean export of main (`git archive 8b360766c` extracted
# under private/ftd/src-main, so neither the binary nor the contracts the
# routes read can carry the card's edits). Builds quirin-audit there under the
# heavy lock, then runs this card's measure.sh from the export (which reads
# the export's contracts and writes private/ftd/ of the export).
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
src="$repo/private/ftd/src-main"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$src/private/ftd/bin" "$src/docs/reports/fb-teardown-destroy-declarations/scripts" || exit 2
cp "$here/measure.sh" "$src/docs/reports/fb-teardown-destroy-declarations/scripts/measure.sh" || exit 2
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
flock "$lock" cargo build --release -p qcore-audit --bin quirin-audit \
  > "$src/private/ftd/build-$name.log" 2>&1
echo $? > "$src/private/ftd/build-$name.exit"
[ "$(cat "$src/private/ftd/build-$name.exit")" = 0 ] || exit 1
cp target/release/quirin-audit "$src/private/ftd/bin/$name"
sha256sum "$src/private/ftd/bin/$name" > "$src/private/ftd/bin/$name.sha256"
bash "$src/docs/reports/fb-teardown-destroy-declarations/scripts/measure.sh" "$name" \
  "$src/private/ftd/bin/$name" system,slice,corpus
echo "done $name"
