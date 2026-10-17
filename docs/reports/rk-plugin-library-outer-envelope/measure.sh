#!/usr/bin/env bash
# measure.sh CHECKOUT BINARY NAME [ROUTES]
# Fresh, worktree-private outputs for the corpus, system and tail-pause slice.
set -u
repo="$1"; binary="$2"; name="$3"; routes="${4:-corpus,system,slice}"
out="$repo/private/rk-outer/measure-$name"
inputs=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19
system=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
mkdir -p "$out" "$repo/target/release"
cd "$repo" || exit 2
cp "$binary" target/release/quirin-audit || exit 2
sha256sum target/release/quirin-audit > "$out/binary.sha256"
git rev-parse HEAD > "$out/head"
step() {
  local label="$1"; shift
  "$@" > "$out/$label.log" 2> "$out/$label.stderr"
  echo $? > "$out/$label.exit"
  echo "$label $(cat "$out/$label.exit")"
  [ "$(cat "$out/$label.exit")" = 0 ]
}
case ",$routes," in
  *,corpus,*)
    export QCORE_NULL_PROPERTY="$out/corpus-null.jsonl" QCORE_OUTER_PROJECTION="$out/corpus-outer.json"
    step corpus python3 scripts/run_pinned_audit.py "$out/corpus" 66910c10 --inputs-root "$inputs" --jobs 4 || exit 3
    unset QCORE_NULL_PROPERTY QCORE_OUTER_PROJECTION;;
esac
case ",$routes," in
  *,system,*)
    mkdir -p "$out/system"
    export QCORE_NULL_PROPERTY="$out/system-null.jsonl" QCORE_OUTER_PROJECTION="$out/system-outer.json" QCORE_CLANG=/usr/lib/llvm-21/bin/clang
    step system "$binary" current --checkout "$system/fluent-bit-clang-source" --profile-directory "$repo/contract/profiles/fluent-bit" --compile-database "$system/fluent-bit-clang-production/compile_commands.json" --selection "$repo/contract/profiles/fluent-bit/system-selection.json" --jobs 4 --report "$out/system/resource-diagnostics.json" || exit 4
    unset QCORE_NULL_PROPERTY QCORE_OUTER_PROJECTION QCORE_CLANG;;
esac
case ",$routes," in
  *,slice,*)
    mkdir -p "$out/slice"
    export QCORE_NULL_PROPERTY="$out/slice-null.jsonl" QCORE_OUTER_PROJECTION="$out/slice-outer.json" QCORE_CLANG=/usr/lib/llvm-21/bin/clang
    step slice "$binary" current --checkout "$slice/fluent-bit-clang-source" --profile-directory "$repo/contract/profiles/fluent-bit" --compile-database "$slice/fluent-bit-clang-production/compile_commands.json" --selection "$repo/contract/profiles/fluent-bit/tail-pause-selection.json" --jobs 4 --report "$out/slice/resource-diagnostics.json" || exit 5
    unset QCORE_NULL_PROPERTY QCORE_OUTER_PROJECTION QCORE_CLANG;;
esac
