#!/usr/bin/env bash
# prepass.sh NAME -- MEMBER-PREPASS-OTHER-ROUTES: the member-facts pre-pass
# alone (examples/member_prepass_run, built by build-example.sh into
# private/mpor/target-rel) over each route's compile database with the
# profile's scope file re-checked, each under the shared heavy lock at
# --jobs 4. Sections to private/mpor/NAME-prepass-ROUTE.json, console and
# exit beside them. Development and wall-time evidence: no unit is analysed.
set -u
name="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/mpor"
lock=/tmp/claude-1000/-home-cosmo-Documents-GitHub-qcore-c--claude-worktrees-quirin-spec-cards-compass-bf79e7/e050a85b-f6ef-4b98-a2ab-618145106af2/scratchpad/heavy.lock
binary="$out/target-rel/release/examples/member_prepass_run"
sha256sum "$binary" > "$out/$name-prepass.binary.sha256"
run() {
  local route="$1" database="$2" source="$3"
  flock "$lock" "$binary" "$database" "$source" "$repo/contract/profiles/fluent-bit" \
    "$out/$name-prepass-$route.json" 4 > "$out/$name-prepass-$route.log" 2>&1
  echo $? > "$out/$name-prepass-$route.exit"
}
pin=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin
system=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep
run corpus "$pin/fluent-bit-clang-source/build/compile_commands.json" "$pin/fluent-bit-clang-source"
run system "$system/fluent-bit-clang-production/compile_commands.json" "$system/fluent-bit-clang-source"
run slice "$slice/fluent-bit-clang-production/compile_commands.json" "$slice/fluent-bit-clang-source"
