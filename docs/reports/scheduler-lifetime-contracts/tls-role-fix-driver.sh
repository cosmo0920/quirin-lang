#!/bin/sh
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
python3 - <<'GUARD'
import hashlib,json
from pathlib import Path
paths=['crates/qcore-contract/src/scheduler_lifetime.rs','crates/qcore-contract/tests/scheduler_lifetime.rs','crates/qcore-contract/examples/scheduler_lifetime_source.rs','proof/QCore/SchedulerLifetime.lean','proof/AuditSchedulerLifetime.lean','docs/reports/scheduler-lifetime-contracts/scheduler.qgd','docs/reports/scheduler-lifetime-contracts/hypotheses.json']
Path('private/scheduler-lifetime-contracts/tls-role-fix-1/inputs-before.json').write_text(json.dumps({p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in paths},indent=2)+'\n')
GUARD
failure=0
for stage in rust source fmt clippy lean-build lean-audit; do
    case "$stage" in
    rust) cargo test -p qcore-contract --test scheduler_lifetime --no-fail-fast > private/scheduler-lifetime-contracts/tls-role-fix-1/rust.log 2>&1;;
    source) cargo run -p qcore-contract --example scheduler_lifetime_source -- private/scheduler-lifetime-contracts/source > private/scheduler-lifetime-contracts/tls-role-fix-1/source.log 2>&1;;
    fmt) cargo fmt --all -- --check > private/scheduler-lifetime-contracts/tls-role-fix-1/fmt.log 2>&1;;
    clippy) cargo clippy -p qcore-contract --all-targets -- -D warnings > private/scheduler-lifetime-contracts/tls-role-fix-1/clippy.log 2>&1;;
    lean-build) (cd proof && lake build QCore.SchedulerLifetime) > private/scheduler-lifetime-contracts/tls-role-fix-1/lean-build.log 2>&1;;
    lean-audit) (cd proof && lake env lean AuditSchedulerLifetime.lean) > private/scheduler-lifetime-contracts/tls-role-fix-1/lean-audit.log 2>&1;;
    esac
    code=$?; echo "$code" > "private/scheduler-lifetime-contracts/tls-role-fix-1/$stage.exit"
    if [ "$code" -ne 0 ]; then failure=1; fi
done

python3 - <<'GUARD'
import hashlib,json
from pathlib import Path
out=Path('private/scheduler-lifetime-contracts/tls-role-fix-1')
before=json.loads((out/'inputs-before.json').read_text());after={p:hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in before}
(out/'inputs-after.json').write_text(json.dumps(after,indent=2)+'\n');assert before==after
GUARD
code=$?; echo "$code" > private/scheduler-lifetime-contracts/tls-role-fix-1/input-guard.exit
if [ "$code" -ne 0 ]; then failure=1; fi
exit "$failure"
