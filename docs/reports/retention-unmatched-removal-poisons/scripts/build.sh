#!/usr/bin/env bash
# build.sh NAME -- RETENTION-UNMATCHED-REMOVAL-POISONS (from
# FB-EVENT-LOOP-RETENTION-AND-REGISTRATION's build.sh): under the shared heavy
# lock, build the release quirin-audit of the export private/rup/NAME-src
# (export.sh) into a private target directory (private/rup/target-rel), then
# keep a copy as private/rup/bin/NAME with its sha256. Exits to files;
# nothing piped.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/rup"
src="$out/$name-src"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
[ -d "$src" ] || exit 2
cd "$src" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
target="$out/target-rel"
flock "$lock" cargo build --release --locked -p qcore-audit --bin quirin-audit \
  --target-dir "$target" > "$out/build-$name.log" 2>&1
echo $? > "$out/build-$name.exit"
[ "$(cat "$out/build-$name.exit")" = 0 ] || exit 1
cp "$target/release/quirin-audit" "$out/bin/$name"
sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
cat "$out/bin/$name.sha256"
