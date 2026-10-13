#!/usr/bin/env bash
# probe-binary.sh SIDE -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: build the
# probe test binary (crates/qcore-verifier-host/tests/request_consumption.rs)
# at this tree under the shared heavy lock and save it as
# private/hdd/bin-SIDE/request_consumption, with its digest. SIDE `base` is
# built from the base's host with the base-compatible probe file (refusals
# assert only "not Completed, exit 0"); `after` from this card's host.
set -u
side="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/hdd/bin-$side"
mkdir -p "$out" || exit 2
git -C "$repo" rev-parse HEAD > "$out/head"
git -C "$repo" diff --stat HEAD > "$out/diff-stat"
bash "$here/heavy.sh" "probe-build-$side" cargo test -p qcore-verifier-host --test request_consumption --no-run --message-format=json || exit 3
python3 - "$repo/private/hdd/probe-build-$side.log" "$out/request_consumption" <<'PY'
import json, shutil, sys
path = None
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    if not line.startswith("{"):
        continue
    message = json.loads(line)
    if message.get("reason") == "compiler-artifact" and message.get("executable") \
            and message["target"]["name"] == "request_consumption":
        path = message["executable"]
if path is None:
    sys.exit("no request_consumption executable in the build log")
shutil.copyfile(path, sys.argv[2])
PY
rc=$?
[ "$rc" = 0 ] || exit 4
chmod +x "$out/request_consumption"
sha256sum "$out/request_consumption" > "$out/request_consumption.sha256"
