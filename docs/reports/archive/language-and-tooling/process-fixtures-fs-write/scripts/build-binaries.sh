#!/usr/bin/env bash
# build-binaries.sh LABEL -- PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE:
# build the stress test binaries and the release quirin-audit at the current
# tree, then copy each to private/pfw/bin/LABEL/ (a private copy no later
# build can overwrite) with its sha256. Run under heavy.sh. Every exit is
# written to a file; nothing is piped.
set -u
label="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/pfw/bin/$label"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
git rev-parse HEAD > "$out/HEAD"
status=0
build() {
  # build NAME CARGO-ARGS...: one test target, its JSON messages to NAME.json
  local name="$1"; shift
  cargo test --no-run --message-format=json "$@" > "$out/$name.json" 2> "$out/$name.stderr"
  local rc=$?
  echo "$rc" > "$out/$name.build.exit"
  [ "$rc" = 0 ] || status=1
  python3 - "$out/$name.json" "$name" "$out" <<'PY'
import json, shutil, sys, hashlib, pathlib
messages, name, out = sys.argv[1], sys.argv[2], pathlib.Path(sys.argv[3])
found = None
for line in open(messages):
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") == "compiler-artifact" and message.get("executable") \
            and message["target"]["name"] == name and message["profile"]["test"]:
        found = message["executable"]
if found is None:
    sys.exit(f"no test executable for {name}")
target = out / name
shutil.copyfile(found, target)
target.chmod(0o755)
digest = hashlib.sha256(target.read_bytes()).hexdigest()
(out / f"{name}.sha256").write_text(f"{digest}  {found}\n")
PY
  [ $? = 0 ] || status=1
}
build cycle20_verifier_plugins -p qcore-compiler --test cycle20_verifier_plugins
build standalone_host -p qcore-verifier-host --test standalone_host
build subcommand_plugins -p qcore-audit --test subcommand_plugins
build user_contract_plugins -p qcore-audit --test user_contract_plugins
cargo build --release -p qcore-audit --bin quirin-audit > "$out/release.log" 2>&1
rc=$?
echo "$rc" > "$out/release.exit"
if [ "$rc" = 0 ]; then
  cp target/release/quirin-audit "$out/quirin-audit" && sha256sum "$out/quirin-audit" > "$out/quirin-audit.sha256"
else
  status=1
fi
exit "$status"
