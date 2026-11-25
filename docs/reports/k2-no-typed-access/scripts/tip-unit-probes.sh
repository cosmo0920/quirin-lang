#!/usr/bin/env bash
# tip-unit-probes.sh NAME -- K2: build the scratch probe (probes/k2_probe_scratch.rs.txt,
# copied into the gateway tests only for the build and removed after) in release
# in a `build` gate slot, then run its `unit_probe` over the preprocessed lz4.c and
# rdkafka_share_acknowledgement.c units (private/k2/pp, preprocess.py) with libc's
# string.qv; outputs under private/k2/NAME.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/k2/$1"
mkdir -p "$out"
cd "$repo" || exit 2
scratch="crates/qcore-c-gateway/tests/k2_probe_scratch.rs"
cp docs/reports/k2-no-typed-access/probes/k2_probe_scratch.rs.txt "$scratch"
CARGO_TARGET_DIR="$repo/private/k2/target-release-test" python3 scripts/gate_slot.py build \
  --receipt "$out/build.receipt.json" -- \
  cargo test --release -p qcore-c-gateway --test k2_probe_scratch --no-run > "$out/build.log" 2>&1
code=$?
rm -f "$scratch"
echo "build $code" > "$out/summary.txt"
[ "$code" = 0 ] || exit "$code"
binary="$(grep -o 'private/k2/target-release-test/release/deps/k2_probe_scratch-[a-f0-9]*' "$out/build.log" | head -1)"
cp private/k2/pp/lz4.i private/k2/pp/share.i "$out/"
probe="$here/unit-probe.sh"
string="providers/quirin-provider-libc/contracts/libc/string.qv"
"$probe" "$binary" "$out/lz4.i" LZ4_hashPosition,LZ4_getPosition,LZ4_putPosition "$string" > "$out/lz4.out" 2>&1
"$probe" "$binary" "$out/share.i" rd_kafka_share_partition_offsets_list_new,rd_kafka_share_partition_offsets_new "$string" > "$out/share.out" 2>&1
echo "probes done" >> "$out/summary.txt"
