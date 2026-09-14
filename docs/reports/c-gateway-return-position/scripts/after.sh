#!/bin/sh
# Build the AFTER binary from this worktree's HEAD and run the pinned audit.
set -e
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a310b80d397bfb2a1
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/c-gateway-return-position/bin/audit-after
sha256sum measure/c-gateway-return-position/bin/audit-after
sh measure/c-gateway-return-position/run-audit.sh after \
   measure/c-gateway-return-position/bin/audit-after
