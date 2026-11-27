#!/bin/sh
# Worktree-private: the opt-in current run over the OpenSSL callers' units.
# Usage: sh private/optin.sh NAME TREE BINARY
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-aa90a3b1b80dcacce
NAME=$1; TREE=$2; BIN=$3
set --
. /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh
OUT=$W/private/optin/$NAME
mkdir -p $OUT
cd $TREE || exit 2
P=$W/private/prep/system
set -- $(python3 docs/reports/openssl-provider-plugin/evidence/opt_in_providers.py .)
$BIN current --checkout $P/fluent-bit-clang-source --profile-directory contract/profiles/fluent-bit \
  --compile-database $P/fluent-bit-clang-production/compile_commands.json \
  --selection $W/private/optin-selection.json --load-path providers/user-crates-load-path.json \
  "$@" --report $OUT/resource-diagnostics.json --jobs 6 > $OUT/run.log 2>&1
echo "exit $?" >> $OUT/run.log
