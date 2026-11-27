#!/usr/bin/env bash
# run.sh -- QUANT-INDEX-CHECKS: the bounds property of chunkio's header
# functions (pinned corpus source, read only) under the probe's test-only quant
# checks, held as written and with the review markers removed (the resolved
# reading a person's review would produce). The only route that selects a
# bounds module today is the API (the audit routes run with an empty bounds
# environment). Output: private/qic/probe-{held,resolved}.json and .exit.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/qic"
include=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/lib/chunkio/include
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
# The test-only declarations are written here, not tracked as a .qv file: the
# CG16 census (`the_reserved_spellings_are_unused_by_tracked_contracts`) keeps
# every tracked contract module free of bounds vocabulary.
cat > "$out/chunkio-held.qv" <<'QV'
module contract;

// QUANT-INDEX-CHECKS probe input: TEST-ONLY, review-held declarations of two
// chunkio header functions. Not a provider contract and not reviewed; no
// audit route loads this file.

extern fn cio_file_st_get_meta_len(map: ptr<mut, u8>) -> u16
contract {
    requires bounds_contract_v1;
    requires bounds_review_pending("probe only: the 24-byte chunkio header layout (CIO_FILE_HEADER_MIN)", "QUANT-INDEX-CHECKS");
    quant checks map, Idx #i: 0 <= #i && #i < 24;
};

extern fn cio_file_st_set_meta_len(map: ptr<mut, u8>, len: u16) -> unit
contract {
    requires bounds_contract_v1;
    requires bounds_review_pending("probe only: the metadata length field at bytes 22 and 23", "QUANT-INDEX-CHECKS");
    quant checks map, Idx #i: 22 <= #i && #i <= 23;
};
QV
grep -v bounds_review_pending "$out/chunkio-held.qv" > "$out/chunkio-resolved.qv" || exit 2
functions="cio_file_st_get_meta_len cio_file_st_set_meta_len cio_file_st_get_hash cio_file_st_get_content probe_full_header probe_short_header"
for mode in held resolved; do
  contract="$out/chunkio-held.qv"
  [ "$mode" = resolved ] && contract="$out/chunkio-resolved.qv"
  # shellcheck disable=SC2086
  CARGO_BUILD_JOBS=4 cargo run -q -p qcore-c-gateway --example quant_index_probe -- \
    "$here/probe.c" "$contract" "$include" $functions \
    > "$out/probe-$mode.json" 2> "$out/probe-$mode.stderr"
  echo $? > "$out/probe-$mode.exit"
done
