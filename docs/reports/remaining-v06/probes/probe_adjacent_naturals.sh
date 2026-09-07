#!/usr/bin/env bash
# V06 / R07 probe (c): adjacent UNSIGNED literal routes, recorded so the audit
# does not silently leave an unchecked parse next to the two signed ones.
# These are the u64 usage-grade bounds (qcore-frontend/src/frontend.rs
# `usage_grade`) and, through them, the natural index literals. They are a
# different literal profile from R07's signed extrema; this probe only confirms
# they are range-checked and fail closed, and does not claim R07 coverage.
set -u
ROOT="${1:?workspace root}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
BIN="$ROOT/target/debug/qcorec"

rows=(
"U01|fn f(x: i32 use one) -> unit { return; } fn main() -> unit { return; }"
"U02|fn f(x: i32 use 18446744073709551615) -> unit { return; } fn main() -> unit { return; }"
"U03|fn f(x: i32 use 18446744073709551616) -> unit { return; } fn main() -> unit { return; }"
"U04|fn f(x: i32 use [0, 18446744073709551616]) -> unit { return; } fn main() -> unit { return; }"
"U05|fn f(x: i32 use -1) -> unit { return; } fn main() -> unit { return; }"
"U06|fn f(x: i32 use 0) -> unit { return; } fn main() -> unit { return; }"
)

printf '%-5s %-6s %s\n' ID EXIT DIAGNOSTIC
for row in "${rows[@]}"; do
  id="${row%%|*}"; src="${row#*|}"
  printf '%s\n' "$src" > "$WORK/$id.qcore"
  out="$("$BIN" check "$WORK/$id.qcore" 2>&1)"
  code=$?
  diag="$(printf '%s' "$out" | tr '\n' ' ' | sed "s#$WORK/##" | cut -c1-115)"
  printf '%-5s %-6s %s\n' "$id" "$code" "$diag"
done
