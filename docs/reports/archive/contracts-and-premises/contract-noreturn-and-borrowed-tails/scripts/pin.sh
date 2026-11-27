#!/usr/bin/env bash
# CONTRACT-NORETURN-AND-BORROWED-TAILS: record the reviewed declarations with
# `quirin-audit pin` at both audited revisions, then check every crate's lock
# at both (adapted from w48-callee-closure-r3/scripts/pin.sh). The libc
# declarations are `revision flexible` (the lock records their contract
# digests only); `mk_print`, `flb_log_print` and `rd_kafka_crash` are pinned
# to their reviewed source identities. Run from anywhere; each step's exit goes
# to its own file.
# usage: pin.sh BINARY LOG_DIR [check-only]
set -u
binary="$1"
logs="$2"
mode="${3:-pin}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
old=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
new=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/contract-noreturn-and-borrowed-tails/attempt-1/fluent-bit-clang-source
mkdir -p "$logs"
cd "$repo" || exit 2
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
if [ "$mode" = pin ]; then
  for root in "$old" "$new"; do
    tag=$(basename "$(dirname "$root")")
    step "pin-libc-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-libc \
      --function printf --function fprintf --function snprintf
    step "pin-libevent-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-libevent \
      --function mk_print
    step "pin-fluent-bit-$tag" "$binary" pin --source-root "$root" --crate quirin-provider-fluent-bit \
      --function flb_log_print
    step "pin-librdkafka-$tag" "$binary" pin --source-root "$root" --crate qcore-provider-librdkafka \
      --function rd_kafka_crash
  done
fi
# The whole-lock check exits 1 at both Fluent Bit revisions on the base too:
# `mk_vhost_set` is a tip-tracked entry (CONTRACT-PIN-TIP-MODE) whose sites at
# Fluent Bit's vendored `lib/monkey/` differ from the Monkey tip, so it is not
# in force there by design (MONKEY-VHOST-PROPERTY-CUSTODY's artifacts/pin/
# check-*.exit are 1 for the same line). The per-crate checks below cover
# every declaration this card touched and must exit 0.
for root in "$old" "$new"; do
  tag=$(basename "$(dirname "$root")")
  step "check-$tag" "$binary" pin --source-root "$root" --check
  step "check-$tag-crates" "$binary" pin --source-root "$root" --check \
    --crate quirin-provider-libc --crate quirin-provider-fluent-bit --crate qcore-provider-librdkafka
  step "check-$tag-mk_print" "$binary" pin --source-root "$root" --check \
    --crate qcore-provider-libevent --function mk_print
done
