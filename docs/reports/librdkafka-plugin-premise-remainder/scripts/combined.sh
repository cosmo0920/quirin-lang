#!/usr/bin/env bash
# combined.sh -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the combined arbiter
# after the rebase onto main b2fbde2c5 (HANDOFF-CANDIDATE-OWN-CALLBACKS
# landed). Builds (under the shared heavy lock) the release quirin-audit of
# main from an exported copy (private/lppr/main-src, `git archive b2fbde2c5`,
# third_party/libevent a link to this worktree's submodule at d82464a2) and of
# this card's rebased tip, keeps them as private/lppr/bin/main and bin/a2,
# measures each on the corpus, system and slice routes with the null
# property (main with REPO set to its exported tree, so it reads main's
# contracts, profile and scripts), then deletes both release targets. Exits
# to files; nothing piped.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/lppr"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
mkdir -p "$out/bin" || exit 2
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
build() {
  # build NAME TREE COMMIT
  local name="$1" tree="$2" commit="$3"
  ( cd "$tree" && QCORE_AUDIT_BUILD_COMMIT="$commit" flock "$lock" cargo build --release -p qcore-audit \
      --bin quirin-audit --target-dir "$out/target-$name" ) > "$out/build-$name.log" 2>&1
  echo $? > "$out/build-$name.exit"
  [ "$(cat "$out/build-$name.exit")" = 0 ] || return 1
  cp "$out/target-$name/release/quirin-audit" "$out/bin/$name"
  sha256sum "$out/bin/$name" > "$out/bin/$name.sha256"
  rm -rf "$out/target-$name"
}
tip=$(cd "$repo" && git rev-parse HEAD)
echo "$tip" > "$out/combined-tip"
build main "$out/main-src" b2fbde2c5bc59ab9d48d7b5f2d1c739a1b15c751 || exit 1
build a2 "$repo" "$tip" || exit 1
REPO="$out/main-src" OUT="$out" NULL=1 "$here/measure-binary.sh" m1 "$out/bin/main" > "$out/m1-measure.out" 2>&1
echo $? > "$out/m1-measure.step.exit"
NULL=1 "$here/measure-binary.sh" a2 "$out/bin/a2" > "$out/a2-measure.out" 2>&1
echo $? > "$out/a2-measure.step.exit"
rm -f "$out"/m1*-parity.jsonl* "$out"/a2*-parity.jsonl*
echo done > "$out/combined-done"
