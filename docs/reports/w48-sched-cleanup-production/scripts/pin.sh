#!/bin/sh
# Pin the new mk_http_thread_destroy declaration at both audited Fluent Bit
# revisions (046accf6, 66910c10), then check the lock at each.
# Usage: pin.sh BINARY LOGDIR
set -u
binary=$1
logs=$2
mkdir -p "$logs"
older=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
"$binary" pin --source-root "$older" --crate qcore-provider-libevent --function mk_http_thread_destroy >"$logs/pin-046accf6.log" 2>&1
echo $? >"$logs/pin-046accf6.exit"
"$binary" pin --source-root "$corpus" --crate qcore-provider-libevent --function mk_http_thread_destroy >"$logs/pin-66910c10.log" 2>&1
echo $? >"$logs/pin-66910c10.exit"
"$binary" pin --source-root "$older" --check --crate qcore-provider-libevent >"$logs/check-046accf6.log" 2>&1
echo $? >"$logs/check-046accf6.exit"
"$binary" pin --source-root "$corpus" --check --crate qcore-provider-libevent >"$logs/check-66910c10.log" 2>&1
echo $? >"$logs/check-66910c10.exit"
"$binary" pin --verify-contracts >"$logs/verify-contracts.log" 2>&1
echo $? >"$logs/verify-contracts.exit"
