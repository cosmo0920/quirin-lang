#!/usr/bin/env bash
# regen.sh BINARY LOG_DIR CRATE -- NULL-CANDIDATE-TRIAGE: rebuild CRATE's
# bounds lock from the declarations it now carries, at both audited
# revisions: `pin --bounds --regenerate` at 66910c10 (the read-only corpus
# tree), which drops the entry of a declaration the card withdrew, then
# `pin --bounds` at 046accf6 (the prepared system tree, read only), which adds
# that revision's identity to every entry; then `pin --bounds --check` at both
# and `pin --verify-contracts`. Each step's output and exit go to files.
set -u
binary="$1"; logs="$2"; crate="$3"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step regenerate-old "$binary" pin --bounds --source-root "$old" --crate "$crate" --regenerate
step pin-new "$binary" pin --bounds --source-root "$new" --crate "$crate"
step check-old "$binary" pin --bounds --source-root "$old" --check
step check-new "$binary" pin --bounds --source-root "$new" --check
step verify "$binary" pin --verify-contracts
