#!/usr/bin/env bash
# campaign.sh LABEL RUNS [SET] -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE:
# the base-vs-tip stress. For each saved binary under private/pfw/bin/LABEL/
# (NAMES, default all three), run it RUNS times with the window amplifier
# preloaded (a CLOSE_US delay, default 20 ms, before closing a write
# descriptor on the binary's executable fixtures) and every forked,
# not-yet-exec'ed child of the test process stopped for CHILD_MS (default 20)
# ms (loadbisect.py's child stall). Results:
# private/pfw/stress/LABEL[-SET]-NAME.json and its per-run logs; each exit in
# private/pfw/stress/LABEL[-SET]-NAME.exit.
# Nothing is piped; every load process is bounded and killed by captured PID
# inside loadbisect.py.
set -u
label="$1"; runs="${2:-20}"; set_name="${3:+-$3}"
close_us="${CLOSE_US:-20000}"; child_ms="${CHILD_MS:-20}"
names="${NAMES:-cycle20_verifier_plugins standalone_host subcommand_plugins}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
bin="$repo/private/pfw/bin/$label"
out="$repo/private/pfw/stress"
mkdir -p "$out" || exit 2
shim="$repo/private/pfw/amplify.so"
if [ ! -f "$shim" ]; then
  cc -shared -fPIC -O2 -o "$shim" "$here/amplify.c" -ldl || exit 2
fi
measure() {
  # measure NAME CRATE-DIR SUFFIXES
  local name="$1" cwd="$2" suffixes="$3" tag="$label$set_name-$1"
  case " $names " in *" $name "*) ;; *) return 0;; esac
  python3 "$here/loadbisect.py" --binary "$bin/$name" --cwd "$repo/$cwd" \
    --levels 0:0:0:0:0:0:$child_ms --runs "$runs" --warmup 0 --duration 1800 \
    --env "LD_PRELOAD=$shim" --env QCORE_AMPLIFY_CLOSE_US=$close_us \
    --env "QCORE_AMPLIFY_CLOSE_SUFFIXES=$suffixes" \
    --out "$out/$tag.json" --logs "$out/$tag-logs" \
    > "$out/$tag.console" 2>&1
  echo $? > "$out/$tag.exit"
}
measure cycle20_verifier_plugins compiler "/provider.sh:/hang.sh:/provider"
measure standalone_host crates/qcore-verifier-host "/provider.sh"
measure subcommand_plugins crates/qcore-audit "~/plugins/quirin-audit-"
