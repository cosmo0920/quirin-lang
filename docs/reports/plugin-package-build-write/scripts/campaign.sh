#!/usr/bin/env bash
# campaign.sh LABEL RUNS -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS:
# the base-vs-tip stress. Runs the saved `sdk` test binary
# private/ppb/bin/LABEL/sdk RUNS times, filtered to
# a_multi_threaded_caller_builds_and_executes_packages_while_siblings_spawn,
# with the window amplifier preloaded (a CLOSE_US delay, default 20 ms, before
# closing a write descriptor on a path ending in
# /quirin-plugin-example-checker: only the package's executable is ever opened
# for writing under that name) and every forked, not-yet-exec'ed child of the
# test process stopped for CHILD_MS (default 20) ms (loadbisect.py's child
# stall, made parallel by --parallel-child-stall unless PARALLEL=0, so the
# two sibling threads' children and the test thread's are stopped at once).
# amplify.c is copied unchanged from PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE
# (itself from EXECUTION-QUOTA-LOAD-INDEPENDENCE); loadbisect.py too, plus the
# one flag its header names.
# Results: private/ppb/stress/TAG.json (TAG defaults to LABEL), its per-run
# logs and TAG.exit.
# Nothing is piped; every load process is bounded and killed by captured PID
# inside loadbisect.py.
set -u
label="$1"; runs="${2:-30}"; tag="${TAG:-$1}"
close_us="${CLOSE_US:-20000}"; child_ms="${CHILD_MS:-20}"
parallel=(--parallel-child-stall); [ "${PARALLEL:-1}" = 0 ] && parallel=()
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bin="$repo/private/ppb/bin/$label"
out="$repo/private/ppb/stress"
mkdir -p "$out" || exit 2
shim="$repo/private/ppb/amplify.so"
if [ ! -f "$shim" ]; then
  cc -shared -fPIC -O2 -o "$shim" "$here/amplify.c" -ldl || exit 2
fi
python3 "$here/loadbisect.py" --binary "$bin/sdk" \
  --cwd "$repo/providers/quirin-provider-plugin" \
  --levels 0:0:0:0:0:0:$child_ms --runs "$runs" --warmup 0 --duration 7200 \
  "${parallel[@]}" \
  --test-args "a_multi_threaded_caller_builds_and_executes_packages_while_siblings_spawn" \
  --env "LD_PRELOAD=$shim" --env QCORE_AMPLIFY_CLOSE_US=$close_us \
  --env "QCORE_AMPLIFY_CLOSE_SUFFIXES=/quirin-plugin-example-checker" \
  --out "$out/$tag.json" --logs "$out/$tag-logs" \
  > "$out/$tag.console" 2>&1
echo $? > "$out/$tag.exit"
