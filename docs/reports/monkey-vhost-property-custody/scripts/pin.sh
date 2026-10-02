#!/usr/bin/env bash
# MONKEY-VHOST-PROPERTY-CUSTODY: record the reviewed `mk_string_dup`
# declaration with `quirin-audit pin` at both audited Fluent Bit revisions
# (66910c10, the corpus tree; 046accf6, the system tree), and record the
# Monkey checkout's identities (read only; the branch the tip contract tracks)
# of `mk_string_dup` and of the two helpers `mk_vhost_set_property` calls whose
# only identities are at Fluent Bit's `lib/monkey/` paths (`mk_mem_alloc`,
# `mk_list_add`). Then check every crate's lock at each tree. Each step's
# output and exit go to their own files.
# usage: pin.sh BINARY LOG_DIR [check-only]
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
monkey=/home/cosmo/Documents/GitHub/monkey
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
if [ "$mode" = pin ]; then
  step pin-66910c10 "$binary" pin --source-root "$old" --crate qcore-provider-libevent \
    --function mk_string_dup
  step pin-046accf6 "$binary" pin --source-root "$new" --crate qcore-provider-libevent \
    --function mk_string_dup
  step pin-monkey "$binary" pin --source-root "$monkey" --crate qcore-provider-libevent \
    --function mk_string_dup --function mk_mem_alloc --function mk_list_add
fi
step check-66910c10 "$binary" pin --source-root "$old" --check
step check-046accf6 "$binary" pin --source-root "$new" --check
step check-monkey "$binary" pin --source-root "$monkey" --crate qcore-provider-libevent \
  --function mk_string_dup --function mk_mem_alloc --function mk_list_add --function mk_vhost_set --check
step verify "$binary" pin --verify-contracts
