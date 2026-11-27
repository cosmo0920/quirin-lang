#!/bin/bash
# Two heavy test targets alone, dynamic (gate configuration) then static, inside
# the rust-suite slot hold. Libtest's "finished in" is the figure.
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a8181d93e05d7d24f
cd $W
WRAP="build.rustc-wrapper=\"$W/scripts/rustc_test_dylib.py\""
SEL="-p qcore-contract -p qcore-audit --test contract_views --test provider_selection"
echo "== dynamic"
CARGO_TARGET_DIR=$W/private/tsb-after-dyn/target cargo test --config build.incremental=false --config "$WRAP" $SEL 2>&1 | grep -E 'Running|test result|Compiling'
echo "== static build"
CARGO_BUILD_JOBS=6 CARGO_TARGET_DIR=$W/private/tsb-static-time/target cargo test --config build.incremental=false $SEL --no-run 2>&1 | tail -1
echo "== static"
CARGO_TARGET_DIR=$W/private/tsb-static-time/target cargo test --config build.incremental=false $SEL 2>&1 | grep -E 'Running|test result|Compiling'
echo "== dynamic again"
CARGO_TARGET_DIR=$W/private/tsb-after-dyn/target cargo test --config build.incremental=false --config "$WRAP" $SEL 2>&1 | grep -E 'Running|test result|Compiling'
