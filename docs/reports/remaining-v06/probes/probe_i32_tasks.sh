#!/usr/bin/env bash
# V06 / R07 probe (a3): the SECOND i32 source literal route -- the checked task
# IR operand parser (crates/qcore-frontend/src/source_tasks.rs::scalar_operand).
# Reached from real source via `concurrency checked;` + `let x = <literal>;`
# (T-rows) and via a task function `return <literal>;` (R-rows).
set -u
ROOT="${1:?workspace root}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
BIN="$ROOT/target/debug/qcorec"

emit_bind() {
  cat <<EOF
concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = $1;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let r = join move h;
        let y = r?;
        return y;
    }
}
EOF
}

emit_return() {
  cat <<EOF
concurrency checked;
fn worker(x: i32) -> i32 { return $1; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 1;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let r = join move h;
        let y = r?;
        return y;
    }
}
EOF
}

rows=(
"T01|accept|bind|2147483647"
"T02|accept|bind|-2147483648"
"T03|reject|bind|2147483648"
"T04|reject|bind|-2147483649"
"T05|accept|bind|0"
"T06|accept|bind|-0"
"T07|accept|bind|- 2147483648"
"T08|accept|bind|00000000002147483647"
"T09|accept|bind|-00000000002147483648"
"T10|reject|bind|00000000002147483648"
"T11|reject|bind|-00000000002147483649"
"T12|reject|bind|4294967296"
"T13|reject|bind|-4294967296"
"T14|reject|bind|18446744073709551615"
"T15|reject|bind|-18446744073709551615"
"T16|reject|bind|18446744073709551616"
"T17|reject|bind|-18446744073709551616"
"T18|reject|bind|-170141183460469231731687303715884105728"
"T19|reject|bind|--2147483648"
"T20|reject|bind|-(-1)"
"R01|accept|return|2147483647"
"R02|accept|return|-2147483648"
"R03|reject|return|2147483648"
"R04|reject|return|-2147483649"
"R05|accept|return|-0"
"R06|reject|return|-18446744073709551616"
)

printf '%-5s %-8s %-7s %-7s %-8s %s\n' ID EXPECT ROUTE EXIT RESULT DIAGNOSTIC
for row in "${rows[@]}"; do
  IFS='|' read -r id expect route lit <<< "$row"
  file="$WORK/$id.qcore"
  if [ "$route" = bind ]; then emit_bind "$lit" > "$file"; else emit_return "$lit" > "$file"; fi
  out="$("$BIN" check "$file" 2>&1)"
  code=$?
  if [ $code -eq 0 ]; then actual=accept; else actual=reject; fi
  if [ "$actual" = "$expect" ]; then verdict=MATCH; else verdict=MISMATCH; fi
  diag="$(printf '%s' "$out" | tr '\n' ' ' | sed "s#$WORK/##" | cut -c1-100)"
  printf '%-5s %-8s %-7s %-7s %-8s %s\n' "$id" "$expect" "$route" "$code" "$verdict" "$diag"
done
