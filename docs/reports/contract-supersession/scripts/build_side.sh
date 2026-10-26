#!/bin/sh
# Build one side's release quirin-audit at the common path (inside ns_side.sh)
# with one explicit build commit for both sides, so the hypothesis graph's
# evidence_version cannot differ for a reason other than the code.
# Usage: build_side.sh WORKTREE LOG_PREFIX
set -u
W=$1
log=$2
export CARGO_BUILD_JOBS=6 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export QCORE_AUDIT_BUILD_COMMIT=contract-supersession-identity
date -u +%Y-%m-%dT%H:%M:%SZ >"$log.start"
(cd "$W/private/cs/side-src" && cargo build --release -p qcore-audit --bin quirin-audit) >"$log.log" 2>&1
echo $? >"$log.exit"
date -u +%Y-%m-%dT%H:%M:%SZ >"$log.end"
sha256sum "$W/private/cs/side-src/target/release/quirin-audit" >"$log.sha256" 2>&1
