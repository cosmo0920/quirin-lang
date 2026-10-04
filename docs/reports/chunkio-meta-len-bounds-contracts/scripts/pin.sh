#!/usr/bin/env bash
# pin.sh MODE -- CHUNKIO-META-LEN-BOUNDS-CONTRACTS: `quirin-audit pin --bounds`
# for qcore-provider-chunkio at both pinned Fluent Bit revisions, after the
# declarations were read against the header at both (byte-identical). MODE is
# `pin` (record the identities; writes the crate's bounds-contracts.lock) or
# `check` (`pin --bounds --check` at each revision, writes nothing). Sources:
# 046accf6, a private clone prepared by `quirin-audit prepare` for
# W48-CALLEE-CLOSURE-R3 (read only); 66910c10, the read-only corpus source.
# Exits and output to private/chunkio-meta-len-bounds-contracts/MODE-REV.*.
set -u
mode="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
out="$repo/private/chunkio-meta-len-bounds-contracts"
binary="${BINARY:-$repo/target/debug/quirin-audit}"
mkdir -p "$out" || exit 2
cd "$repo" || exit 2
extra=()
[ "$mode" = check ] && extra=(--check)
for pair in \
  "046accf6:/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w48-callee-closure-r3/attempt-1/fluent-bit-clang-source" \
  "66910c10:/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"; do
  revision="${pair%%:*}"; source="${pair#*:}"
  "$binary" pin --bounds --source-root "$source" --crate qcore-provider-chunkio "${extra[@]}" \
    > "$out/$mode-$revision.log" 2>&1
  echo $? > "$out/$mode-$revision.exit"
  echo "$mode $revision exit $(cat "$out/$mode-$revision.exit")"
  cat "$out/$mode-$revision.log"
done
