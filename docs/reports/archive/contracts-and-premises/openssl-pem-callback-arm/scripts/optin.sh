#!/bin/sh
# Worktree-private: the opt-in current run over the OpenSSL callers' units
# (REFERENCE-DROP-NULL-ACCEPTING's selection), as that card's scripts/optin.sh.
# Usage: sh private/optin.sh NAME TREE BINARY
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a395ccb0d1574cb49
NAME=$1; TREE=$2; BIN=$3
set --
. /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
OUT=$W/private/optin/$NAME
mkdir -p $OUT
cd $TREE || exit 2
P=$W/private/prep/system
set -- $(python3 docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py .)
export SOURCE_DATE_EPOCH=1767225600
$BIN current --checkout $P/fluent-bit-clang-source --profile-directory contract/profiles/fluent-bit \
  --compile-database $P/fluent-bit-clang-production/compile_commands.json \
  --selection $W/docs/reports/reference-drop-null-accepting/artifacts/optin-selection.json \
  --load-path providers/user-crates-load-path.json \
  "$@" --report $OUT/resource-diagnostics.json --jobs 6 > $OUT/run.log 2>&1
echo "exit $?" >> $OUT/run.log
