#!/bin/bash
# Card TEST-STORAGE-BUDGET measurement runs, under one rust-suite slot.
set -u
W=/media/Data/GitHub/quirin-lang/.claude/worktrees/agent-a8181d93e05d7d24f
M=$W/private/tsb-measure
PEAK="python3 $W/docs/reports/test-storage-budget/measure_peak.py --interval 5"
export CARGO_BUILD_JOBS=6
cd "$W"

# 0. Baseline: the base clone's fresh `cargo test --workspace --no-fail-fast`
#    (its build was measured separately; this measures the run on top of it).
mkdir -p /tmp/tsb-b
(cd $W/private/tsb-base && TMPDIR=/tmp/tsb-b \
  $PEAK --watch $W/private/tsb-base/target --watch /tmp/tsb-b --out $M/base-run \
  -- cargo test --workspace --no-fail-fast > $M/base-run.log 2>&1)
echo "0 exit $?" >> $M/after.status
rm -rf $W/private/tsb-base/target

# A. Root workspace, gate form, default configuration (Linux: dynamic), fresh target.
mkdir -p /tmp/tsb-ad
CARGO_TARGET_DIR=$W/private/tsb-after-dyn/target TMPDIR=/tmp/tsb-ad \
  $PEAK --watch $W/private/tsb-after-dyn/target --watch /tmp/tsb-ad --out $M/after-dyn \
  -- python3 scripts/run_test_binaries.py --log-dir $M/after-dyn-tests > $M/after-dyn.log 2>&1
echo "A exit $?" >> $M/after.status

# B. Root workspace, static with --batch-size 32 (the Windows configuration), fresh target.
mkdir -p /tmp/tsb-as
CARGO_TARGET_DIR=$W/private/tsb-after-static/target TMPDIR=/tmp/tsb-as \
  $PEAK --watch $W/private/tsb-after-static/target --watch /tmp/tsb-as --out $M/after-static-batch \
  -- python3 scripts/run_test_binaries.py --link static --batch-size 32 \
     --log-dir $M/after-static-batch-tests > $M/after-static-batch.log 2>&1
echo "B exit $?" >> $M/after.status
rm -rf $W/private/tsb-after-static

# C. Provider workspace after (line tables, no incremental, as CI), fresh target.
mkdir -p /tmp/tsb-pa
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=$W/private/tsb-prov-after/target TMPDIR=/tmp/tsb-pa \
  $PEAK --watch $W/private/tsb-prov-after/target --watch /tmp/tsb-pa --out $M/prov-after \
  -- cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline \
  > $M/prov-after.log 2>&1
echo "C exit $?" >> $M/after.status

# D. Provider workspace before (base clone; its build was measured separately).
mkdir -p /tmp/tsb-pb
cd $W/private/tsb-base
TMPDIR=/tmp/tsb-pb \
  $PEAK --watch $W/private/tsb-base/providers/target --watch /tmp/tsb-pb --out $M/prov-base-run \
  -- cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline \
  > $M/prov-base-run.log 2>&1
echo "D exit $?" >> $M/after.status
rm -rf $W/private/tsb-base/providers/target

# E. The standing card gate on the final commit, inside this slot hold (--no-slots),
#    started once the report commit exists (marker file written by hand).
cd $W
while [ ! -f $M/go-gate ]; do sleep 20; done
python3 scripts/gate_scope.py --base 62c8e461c --no-slots > $M/gate.log 2>&1
echo "E exit $?" >> $M/after.status
