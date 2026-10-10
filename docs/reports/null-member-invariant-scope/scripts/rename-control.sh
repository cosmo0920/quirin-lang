#!/usr/bin/env bash
# rename-control.sh BINARY LOG_DIR -- NULL-MEMBER-INVARIANT-SCOPE: the control
# before the `nonnull_` re-pin (USER DECISION 2026-10-11). With the renamed
# contract text and the lock as it stood, `pin --bounds --check` of the
# librdkafka crate at both audited revisions must report every renamed
# declaration out of force (`contract-changed`) and exit nonzero. Output and
# exit of each to LOG_DIR; nothing piped.
set -u
binary="$1"
logs="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  "$binary" pin --bounds --source-root "$root" --crate qcore-provider-librdkafka --check \
    > "$logs/control-$tag.log" 2>&1
  echo $? > "$logs/control-$tag.exit"
  echo "control-$tag $(cat "$logs/control-$tag.exit")"
done
