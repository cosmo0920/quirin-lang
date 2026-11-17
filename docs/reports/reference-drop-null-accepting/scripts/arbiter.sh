#!/bin/sh
# Worktree-private: the declared arbiter, base export against tip export.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa90a3b1b80dcacce
RUN=$1; EXTRA=$2
set --
. /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
set -- $RUN
cd $W || exit 2
OUT=$W/measure/arbiter-$1
mkdir -p $W/measure
$W/private/tip-src/target/release/quirin-audit --plugin-path $W/private/tip-src/target/release \
  --output $OUT --timeout-ms 36000000 measure \
  --inputs $W/private/inputs.json --inputs-base $W --repository $W \
  --side base=$W/private/base-src --side tip=$W/private/tip-src \
  --declared $W/docs/reports/reference-drop-null-accepting/PREDICTION.md \
  --no-slots --jobs 8 --build-jobs 8 $EXTRA > $W/measure/arbiter-$1.log 2>&1
echo "exit $?" >> $W/measure/arbiter-$1.log
