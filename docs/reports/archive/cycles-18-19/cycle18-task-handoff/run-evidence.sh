#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 OUTPUT_DIRECTORY" >&2
  exit 2
fi

script_dir=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$script_dir/../../.." && pwd)
out=$(mkdir -p "$1" && cd "$1" && pwd)
negative="$out/controls"
mkdir -p "$negative"

compiler=${QCORE_CLANG:-$(command -v clang)}
if [[ "$compiler" != /* ]]; then
  compiler=$(command -v "$compiler")
fi
source_file="$repo/compiler/tests/fixtures/cycle18_task_handoff.c"
profile_file="$repo/contract/profiles/cycle18-task-handoff/profile.json"
policy_file="$repo/contract/qcore/monkey/callback_task_handoff.qcore"
binary="$out/taskful-observer"
observation="$out/taskful-observation.json"
runtime_stderr="$out/taskful-stderr.txt"
manifest="$out/taskful-run-manifest.json"
report="$out/taskful-audit-report.json"
config="$out/taskful-config.json"

compile_argv=(
  "$compiler" -std=c11 -pthread -Wall -Wextra -Werror -Wno-unused-parameter
  -fsanitize=address,undefined "$source_file" -o "$binary"
)
"${compile_argv[@]}"
set +e
ASAN_OPTIONS=detect_leaks=0 "$binary" >"$observation" 2>"$runtime_stderr"
run_exit=$?
set -e
if [[ $run_exit -ne 0 ]]; then
  echo "native taskful observer exited $run_exit" >&2
  exit 1
fi

callback_thread=$(jq -er '[.records[] | select(.action.action != "task_start" and .action.action != "task_cancel_ack") | .thread] | unique | if length == 1 then .[0] else error("callback thread set is not singleton") end' "$observation")
worker_thread=$(jq -er '[.records[] | select(.action.action == "task_start" or .action.action == "task_cancel_ack") | .thread] | unique | if length == 1 then .[0] else error("worker thread set is not singleton") end' "$observation")
compiler_version=$("$compiler" --version)
compiler_target=$("$compiler" -dumpmachine)

jq -n \
  --arg platform linux \
  --arg threading pthreads \
  --arg source_sha256 "$(sha256sum "$source_file" | cut -d' ' -f1)" \
  --arg profile_sha256 "$(sha256sum "$profile_file" | cut -d' ' -f1)" \
  --arg policy_sha256 "$(sha256sum "$policy_file" | cut -d' ' -f1)" \
  --arg compiler_program "$compiler" \
  --arg compiler_version "$compiler_version" \
  --arg compiler_target "$compiler_target" \
  --arg binary_sha256 "$(sha256sum "$binary" | cut -d' ' -f1)" \
  --arg runtime_stdout_sha256 "$(sha256sum "$observation" | cut -d' ' -f1)" \
  --arg runtime_stderr_sha256 "$(sha256sum "$runtime_stderr" | cut -d' ' -f1)" \
  --arg observations_sha256 "$(sha256sum "$observation" | cut -d' ' -f1)" \
  --argjson compile_argv "$(printf '%s\n' "${compile_argv[@]}" | jq -R . | jq -s .)" \
  --argjson run_argv "$(printf '%s\n' "$binary" | jq -R . | jq -s .)" \
  --argjson run_environment '{"ASAN_OPTIONS":"detect_leaks=0"}' \
  --argjson run_exit "$run_exit" \
  --argjson callback_thread "$callback_thread" \
  --argjson worker_thread "$worker_thread" \
  '{schema_version: 1, $platform, $threading, $source_sha256, $profile_sha256,
    $policy_sha256, $compiler_program, $compiler_version, $compiler_target,
    $compile_argv, $binary_sha256, $run_argv, $run_environment, $run_exit,
    $runtime_stdout_sha256, $runtime_stderr_sha256, $observations_sha256,
    $callback_thread, $worker_thread}' >"$manifest"

make_config() {
  local destination=$1 artifact=$2 profile_root=$3 source=$4 observations=$5 stderr=$6 run_manifest=$7 result=$8
  jq -n \
    --arg artifact "$artifact" --arg profile_directory "$profile_root" \
    --arg source "$source" --arg policy "$policy_file" \
    --arg observations "$observations" --arg observer_binary "$binary" \
    --arg runtime_stderr "$stderr" --arg compiler "$compiler" \
    --arg run_manifest "$run_manifest" --arg report "$result" \
    '{schema_version: 1, $artifact, $profile_directory, $source, $policy,
      $observations, $observer_binary, $runtime_stderr, $compiler,
      $run_manifest, $report}' >"$destination"
}

make_config "$config" compiler/tests/fixtures/cycle18_task_handoff.c \
  "$repo/contract/profiles" "$source_file" "$observation" "$runtime_stderr" \
  "$manifest" "$report"

cargo build --manifest-path "$repo/Cargo.toml" -p qcore-audit
audit="${CARGO_TARGET_DIR:-$repo/target}/debug/quirin-audit"
QCORE_CLANG="$compiler" "$audit" callback-task-handoff-fixture "$config" \
  >"$out/positive.log" 2>&1

expect_refusal() {
  local name=$1 refusal_config=$2
  set +e
  QCORE_CLANG="$compiler" "$audit" callback-task-handoff-fixture "$refusal_config" \
    >"$out/control-$name.log" 2>&1
  local status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    echo "control $name was accepted" >&2
    exit 1
  fi
  printf '%s %s\n' "$name" "$status" >>"$out/control-status.txt"
}

jq '(.records[] | select(.action.action == "task_spawn") | .action.task) = 11' \
  "$observation" >"$negative/observation.json"
jq --arg digest "$(sha256sum "$negative/observation.json" | cut -d' ' -f1)" \
  '.observations_sha256 = $digest | .runtime_stdout_sha256 = $digest' \
  "$manifest" >"$negative/observation-manifest.json"
make_config "$negative/observation-config.json" compiler/tests/fixtures/cycle18_task_handoff.c \
  "$repo/contract/profiles" "$source_file" "$negative/observation.json" "$runtime_stderr" \
  "$negative/observation-manifest.json" "$negative/observation-report.json"
expect_refusal observation "$negative/observation-config.json"

sed 's/released = 1;/released = 2;/' "$source_file" >"$negative/source.c"
jq --arg digest "$(sha256sum "$negative/source.c" | cut -d' ' -f1)" \
  --arg source "$negative/source.c" \
  '.source_sha256 = $digest | .compile_argv[8] = $source' \
  "$manifest" >"$negative/source-manifest.json"
make_config "$negative/source-config.json" compiler/tests/fixtures/cycle18_task_handoff.c \
  "$repo/contract/profiles" "$negative/source.c" "$observation" "$runtime_stderr" \
  "$negative/source-manifest.json" "$negative/source-report.json"
expect_refusal source "$negative/source-config.json"

mkdir -p "$negative/profile-root/cycle18-task-handoff"
jq '.libraries.fixture.typed_import.functions.cycle18_taskful_callback = "wrapper"' \
  "$profile_file" >"$negative/profile-root/cycle18-task-handoff/profile.json"
jq --arg digest "$(sha256sum "$negative/profile-root/cycle18-task-handoff/profile.json" | cut -d' ' -f1)" \
  '.profile_sha256 = $digest' "$manifest" >"$negative/profile-manifest.json"
make_config "$negative/profile-config.json" compiler/tests/fixtures/cycle18_task_handoff.c \
  "$negative/profile-root" "$source_file" "$observation" "$runtime_stderr" \
  "$negative/profile-manifest.json" "$negative/profile-report.json"
expect_refusal profile "$negative/profile-config.json"

printf 'synthetic sanitizer diagnostic\n' >"$negative/stderr.txt"
jq --arg digest "$(sha256sum "$negative/stderr.txt" | cut -d' ' -f1)" \
  '.runtime_stderr_sha256 = $digest' "$manifest" >"$negative/stderr-manifest.json"
make_config "$negative/stderr-config.json" compiler/tests/fixtures/cycle18_task_handoff.c \
  "$repo/contract/profiles" "$source_file" "$observation" "$negative/stderr.txt" \
  "$negative/stderr-manifest.json" "$negative/stderr-report.json"
expect_refusal stderr "$negative/stderr-config.json"

make_config "$negative/artifact-config.json" another.c \
  "$repo/contract/profiles" "$source_file" "$observation" "$runtime_stderr" \
  "$manifest" "$negative/artifact-report.json"
expect_refusal artifact "$negative/artifact-config.json"
