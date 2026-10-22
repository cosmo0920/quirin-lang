#!/bin/sh
# `pin --check` of the Monkey provider at both audited revisions, for one
# binary and load path (BASE snapshot or TIP worktree).
# Usage: pin_check.sh BINARY LOAD_PATH LOGDIR
set -u
binary=$1
load=$2
logs=$3
mkdir -p "$logs"
older=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
"$binary" pin --source-root "$older" --check --load-path "$load" --crate qcore-provider-libevent >"$logs/check-046accf6.log" 2>&1
echo $? >"$logs/check-046accf6.exit"
"$binary" pin --source-root "$corpus" --check --load-path "$load" --crate qcore-provider-libevent >"$logs/check-66910c10.log" 2>&1
echo $? >"$logs/check-66910c10.exit"
"$binary" pin --verify-contracts --load-path "$load" >"$logs/verify-contracts.log" 2>&1
echo $? >"$logs/verify-contracts.exit"
