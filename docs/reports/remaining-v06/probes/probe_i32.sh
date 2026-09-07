#!/usr/bin/env bash
# V06 / R07 probe (a): i32 source literal parser, live `qcorec check`.
# Each row: ID | expectation (accept|reject) | source body.
# Prints ID, expectation, actual exit status, first diagnostic line, verdict.
set -u
ROOT="${1:?workspace root}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

BIN="$ROOT/target/debug/qcorec"

rows=(
"A01|accept|fn main() -> i32 { return 2147483647; }"
"A02|accept|fn main() -> i32 { return -2147483648; }"
"A03|reject|fn main() -> i32 { return 2147483648; }"
"A04|reject|fn main() -> i32 { return -2147483649; }"
"A05|accept|fn main() -> i32 { return 0; }"
"A06|accept|fn main() -> i32 { return -0; }"
"A07|accept|fn main() -> i32 { return - 2147483648; }"
"A08|accept|fn main() -> i32 { return -
2147483648; }"
"A09|reject|fn main() -> i32 { return --2147483648; }"
"A10|reject|fn main() -> i32 { return -(-1); }"
"A11|reject|fn main() -> i32 { return (1); }"
"A12|accept|fn main() -> i32 { return 00000000002147483647; }"
"A13|accept|fn main() -> i32 { return -00000000002147483648; }"
"A14|reject|fn main() -> i32 { return 00000000002147483648; }"
"A15|reject|fn main() -> i32 { return -00000000002147483649; }"
"A16|reject|fn main() -> i32 { return 999999999999999999999999999999999999999999; }"
"A17|reject|fn main() -> i32 { return -999999999999999999999999999999999999999999; }"
"A18|reject|fn main() -> i32 { return 170141183460469231731687303715884105727; }"
"A19|reject|fn main() -> i32 { return -170141183460469231731687303715884105728; }"
"A20|reject|fn main() -> i32 { return +1; }"
"A21|reject|fn main() -> i32 { return 2_147_483_647; }"
"A22|reject|fn main() -> i32 { return 0x7fffffff; }"
"A23|reject|fn main() -> i32 { return 2147483647 + 1; }"
"A24|reject|fn main() -> i32 { return -2147483648 - 1; }"
"A25|reject|fn main() -> i32 { return 1 + 1; }"
"A26|reject|fn main() -> i32 { return 2147483647.0; }"
"A27|reject|fn main() -> i32 { return 4294967296; }"
"A28|reject|fn main() -> i32 { return 4294967295; }"
"A29|reject|fn main() -> i32 { return -4294967296; }"
"A30|accept|fn main() -> i32 { return -1; }"
"A31|reject|fn main() -> i32 { return -18446744073709551616; }"
"A32|reject|fn main() -> i32 { return 9223372036854775808; }"
"A33|reject|fn main() -> i32 { return -9223372036854775808; }"
"A34|reject|fn main() -> unit { return 0; }"
"A35|accept|fn main() -> unit { return; }"
)

printf '%-5s %-8s %-7s %-8s %s\n' ID EXPECT EXIT RESULT DIAGNOSTIC
for row in "${rows[@]}"; do
  id="${row%%|*}"; rest="${row#*|}"
  expect="${rest%%|*}"; src="${rest#*|}"
  file="$WORK/$id.qcore"
  printf '%s\n' "$src" > "$file"
  out="$("$BIN" check "$file" 2>&1)"
  code=$?
  if [ $code -eq 0 ]; then actual=accept; else actual=reject; fi
  if [ "$actual" = "$expect" ]; then verdict=MATCH; else verdict=MISMATCH; fi
  diag="$(printf '%s' "$out" | tr '\n' ' ' | cut -c1-120)"
  printf '%-5s %-8s %-7s %-8s %s\n' "$id" "$expect" "$code" "$verdict" "$diag"
done
