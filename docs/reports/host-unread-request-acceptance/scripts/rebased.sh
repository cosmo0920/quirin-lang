#!/usr/bin/env bash
# rebased.sh -- HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING: re-measure after
# the rebase onto 1f53f9fa0 (QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN, whose change
# sits next to this card's in run_process). Builds the release quirin-audit and
# the probe test binary at this tip into private/hur/bin/ and bin-rebased/,
# reruns the probe campaign for the tip only (SIDES=rebased, 50 runs per
# condition), then the three routes with the null property, and compares them
# with the BEFORE runs (main's arbiters are byte-identical to e0b92ece6's, per
# the integrator). Every exit is in private/hur/*.exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
h="$repo/private/hur"
bash "$here/heavy.sh" build-release-rebased cargo build --release -p qcore-audit --bin quirin-audit || exit 3
cp "$repo/target/release/quirin-audit" "$h/bin/quirin-audit-rebased" || exit 3
bash "$here/heavy.sh" build-probe-rebased cargo test -p qcore-verifier-host --test request_consumption --no-run || exit 3
probe=$(sed -n 's/.*Executable tests\/request_consumption.rs (\(.*\))$/\1/p' "$h/build-probe-rebased.log" | tail -1)
mkdir -p "$h/bin-rebased" && cp "$repo/$probe" "$h/bin-rebased/request_consumption" || exit 3
SIDES=rebased sh "$here/campaign.sh" > "$h/campaign-rebased.log" 2>&1
echo $? > "$h/campaign-rebased.exit"
NULL=1 PARITY=0 bash "$here/measure-binary.sh" rebased "$h/bin/quirin-audit-rebased"
AFTER=rebased python3 "$here/identity.py" > "$h/identity-rebased.txt" 2>&1
echo $? > "$h/identity-rebased.exit"
