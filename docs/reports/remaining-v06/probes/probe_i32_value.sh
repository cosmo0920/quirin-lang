#!/usr/bin/env bash
# V06 / R07 probe (a2): value fidelity for accepted i32 source literals.
# Emits C for each accepted spelling and reports the emitted return constant.
set -u
ROOT="${1:?workspace root}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
BIN="$ROOT/target/debug/qcorec"

rows=(
"V01|2147483647|fn main() -> i32 { return 2147483647; }"
"V02|-2147483648|fn main() -> i32 { return -2147483648; }"
"V03|0|fn main() -> i32 { return 0; }"
"V04|0|fn main() -> i32 { return -0; }"
"V05|-2147483648|fn main() -> i32 { return - 2147483648; }"
"V06|2147483647|fn main() -> i32 { return 00000000002147483647; }"
"V07|-2147483648|fn main() -> i32 { return -00000000002147483648; }"
"V08|-1|fn main() -> i32 { return -1; }"
"V09|1|fn main() -> i32 { return 000001; }"
)

printf '%-5s %-14s %s\n' ID EXPECT "EMITTED RETURN CONSTANT"
for row in "${rows[@]}"; do
  id="${row%%|*}"; rest="${row#*|}"
  expect="${rest%%|*}"; src="${rest#*|}"
  printf '%s\n' "$src" > "$WORK/$id.qcore"
  if "$BIN" emit -o "$WORK/$id.c" "$WORK/$id.qcore" >/dev/null 2>&1; then
    line="$(grep -n 'INT32' "$WORK/$id.c" | grep -i 'value\|return' | head -1)"
    [ -z "$line" ] && line="$(grep -n 'INT32' "$WORK/$id.c" | head -1)"
  else
    line="EMIT FAILED"
  fi
  printf '%-5s %-14s %s\n' "$id" "$expect" "$line"
done
