#!/usr/bin/env bash
# Pin the twelve storage-backlog checked specifications, after review, at the
# re-pinned slice source (046accf6) and at the 66910c10 corpus source (both
# read-only; `pin` only reads them), then check both.
# usage: pin.sh BINARY LOG_DIR
set -u
binary="$1"; logs="$2"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
mkdir -p "$logs"
cd "$repo" || exit 2
slice=/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-backlog-custody/attempt-2/fluent-bit-clang-source
corpus=/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source
functions=()
for f in sb_allocate_chunk sb_destroy_chunk sb_destroy_backlog sb_allocate_backlogs \
         sb_destroy_backlogs sb_remove_chunk_from_segregated_backlog \
         sb_remove_chunk_from_segregated_backlogs sb_append_chunk_to_segregated_backlog \
         sb_append_chunk_to_segregated_backlogs sb_segregate_chunks \
         sb_release_output_queue_space cb_queue_chunks; do
  functions+=(--function "$f")
done
step() {
  local name="$1"; shift
  "$@" > "$logs/$name.log" 2>&1
  echo $? > "$logs/$name.exit"
  echo "$name $(cat "$logs/$name.exit")"
}
step pin-046accf6 "$binary" pin --source-root "$slice" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit "${functions[@]}"
step pin-66910c10 "$binary" pin --source-root "$corpus" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit "${functions[@]}"
step check-046accf6 "$binary" pin --source-root "$slice" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit --check
step check-66910c10 "$binary" pin --source-root "$corpus" --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit --check
step verify-contracts "$binary" pin --verify-contracts --load-path providers/contract-load-path.json --crate quirin-provider-fluent-bit
