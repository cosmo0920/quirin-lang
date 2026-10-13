#!/usr/bin/env bash
# repeat.sh RUNS PROVIDER-RUNS -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE:
# every test binary this card touched (and the verifier-host binaries that
# include the moved writer), built at the current tree, each run RUNS times
# (PROVIDER-RUNS for the provider conformance binaries, which build the
# packages) with --test-threads 32 from its crate root, as cargo runs it.
# The follow-up example runs RUNS times. One log and one exit per run under
# private/pfw/repeat/; nothing is piped. Run under heavy.sh.
set -u
runs="${1:-10}"; provider_runs="${2:-3}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/pfw/repeat"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/HEAD"
status=0
locate() {
  # locate MESSAGES NAME: the test executable cargo built for target NAME
  python3 - "$1" "$2" <<'PY'
import json, sys
found = None
for line in open(sys.argv[1]):
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") == "compiler-artifact" and message.get("executable") \
            and message["target"]["name"] == sys.argv[2] and message["profile"]["test"]:
        found = message["executable"]
print(found or "")
PY
}
repeat() {
  # repeat COUNT CRATE-DIR MESSAGES NAME
  local count="$1" cwd="$2" messages="$3" name="$4" binary run rc
  binary="$(locate "$messages" "$name")"
  if [ -z "$binary" ]; then
    echo "missing" > "$out/$name.missing"; status=1; return
  fi
  sha256sum "$binary" > "$out/$name.sha256"
  for run in $(seq 1 "$count"); do
    ( cd "$repo/$cwd" && "$binary" --test-threads 32 ) > "$out/$name-$run.log" 2>&1
    rc=$?
    echo "$rc" > "$out/$name-$run.exit"
    [ "$rc" = 0 ] || status=1
  done
}
build() {
  # build MESSAGES CARGO-ARGS...
  local messages="$1"; shift
  cargo test --no-run --message-format=json "$@" > "$messages" 2> "$messages.stderr"
  local rc=$?
  echo "$rc" > "$messages.exit"
  [ "$rc" = 0 ] || status=1
}
root="$out/root.json"
build "$root" -p qcore-verifier-host -p qcore-compiler -p qcore-audit -p qcore-c-gateway \
  -p qcore-verification -p qcorec
for name in standalone_host package_identity execution_quota confined_process_execution \
    enforced_process_limits request_consumption; do
  repeat "$runs" crates/qcore-verifier-host "$root" "$name"
done
for name in cycle20_verifier_plugins w3_evidence_v2; do
  repeat "$runs" compiler "$root" "$name"
done
for name in user_contract_plugins configured_cli prepare_cli subcommand_plugins; do
  repeat "$runs" crates/qcore-audit "$root" "$name"
done
for name in c_numeric_abi_llp64 prepared_failure; do
  repeat "$runs" crates/qcore-c-gateway "$root" "$name"
done
repeat "$runs" crates/qcore-verification "$root" source_effect_helper_target
repeat "$runs" crates/qcorec "$root" plugin_packages
providers="$out/providers.json"
build "$providers" --manifest-path providers/Cargo.toml -p qcore-provider-conformance --locked --offline
for name in configured_audit configured_source_contract packages; do
  repeat "$provider_runs" providers/conformance "$providers" "$name"
done
cargo build -p qcore-compiler --example reevaluation_followup > "$out/example-build.log" 2>&1
echo $? > "$out/example-build.exit"
for run in $(seq 1 "$runs"); do
  cargo run -q -p qcore-compiler --example reevaluation_followup > "$out/reevaluation_followup-$run.log" 2>&1
  rc=$?
  echo "$rc" > "$out/reevaluation_followup-$run.exit"
  [ "$rc" = 0 ] || status=1
done
exit "$status"
