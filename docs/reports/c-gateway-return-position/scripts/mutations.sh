#!/bin/sh
# Non-vacuity by mutation (CLAUDE.md STABILITY-REEVAL method 1).
#
# Each mutation breaks ONE arm of card C-GATEWAY-RETURN-POSITION's rule. A
# mutation that leaves the suite green proves the arm is unexercised: the green
# suite is then evidence about the paths it did take, not about that arm. Every
# mutation is reverted before the next, and the tree is re-verified at the end.
#
# Usage: sh measure/c-gateway-return-position/mutations.sh
set -e
cd "$(dirname "$0")/../.."
G=crates/qcore-c-gateway/src
B=/tmp/c-gateway-return-position-mutations
mkdir -p "$B"
T="cargo test -p qcore-c-gateway --no-fail-fast --test return_position_pointer_leaf --test reach_gate_positions --test neutral_expression_positions"

run_mutation() {
  name="$1"; shift
  echo "=============================================================="
  echo "MUTATION: $name"
  cp "$G/legacy_c.rs" "$B/legacy_c.rs.bak"
  cp "$G/neutral_position.rs" "$B/neutral_position.rs.bak"
  "$@"
  if $T >"$B/mut.log" 2>&1; then
    echo "RESULT: GREEN  <-- VACUOUS ARM, the rule is unverified here"
  else
    echo "RESULT: RED (arm is driven) by:"
    sed -n '/^failures:$/,/^test result/p' "$B/mut.log" \
      | grep -E '^    [a-z_0-9]+$' | sort -u | sed 's/^/     /'
  fi
  cp "$B/legacy_c.rs.bak" "$G/legacy_c.rs"
  cp "$B/neutral_position.rs.bak" "$G/neutral_position.rs"
}

# M1. The escape obligation: answer an admitted pointer-leaf return with the
# canonical place `canonical_place` would spell, instead of `None`. This is the
# discharging direction the card forbids.
m1() {
  python3 - <<'PY'
p = 'crates/qcore-c-gateway/src/legacy_c.rs'
s = open(p).read()
old = """                    returned_place: if untracked_pointer {
                        None
                    } else {
                        canonical_place(&texts)
                    },"""
assert s.count(old) == 1
open(p, 'w').write(s.replace(old, "                    returned_place: canonical_place(&texts),"))
PY
}

# M2. The widening itself: never admit the pointer leaf in a return.
m2() {
  python3 - <<'PY'
p = 'crates/qcore-c-gateway/src/legacy_c.rs'
s = open(p).read()
old = "expression_supported: strict || untracked_pointer,"
assert s.count(old) == 1
open(p, 'w').write(s.replace(old, "expression_supported: strict,"))
PY
}

# M3. The strict-first ordering: let the widening claim an expression the strict
# rule already admitted, so a plain `return p;` would lose its escape.
m3() {
  python3 - <<'PY'
p = 'crates/qcore-c-gateway/src/legacy_c.rs'
s = open(p).read()
old = """                let untracked_pointer =
                    !strict && self.supported_expression(&expression, ExpressionRole::Decides);"""
assert s.count(old) == 1
new = """                let untracked_pointer =
                    self.supported_expression(&expression, ExpressionRole::Decides);"""
open(p, 'w').write(s.replace(old, new))
PY
}

# M4. The base premise: drop `!may_bind(base)` from the chain admission, so a
# chain rooted at a name this body binds — an acquisition included — is
# admitted. This is the premise the whole soundness argument rests on.
m4() {
  python3 - <<'PY'
import re
p = 'crates/qcore-c-gateway/src/neutral_position.rs'
s = open(p).read()
# only the executable occurrence, never the doc comments
n, k = re.subn(r'(?m)^(\s+)if types\.may_bind\(base\) \{', r'\1if false {', s)
assert k == 1, k
open(p, 'w').write(n)
PY
}

# M5. M4 AND M1 together: with the base premise gone AND the escape answered by
# canonical_place, is the discharging direction actually reachable? This is the
# question M1 alone cannot answer, and the reason M1 is green.
m5() { m4; m1; }

run_mutation "M1 escape the canonical place instead of NoOp" m1
run_mutation "M2 never admit the pointer leaf in a return" m2
run_mutation "M3 drop the strict-rule-first ordering" m3
run_mutation "M4 drop the !may_bind(base) base premise" m4
run_mutation "M5 = M4 + M1 (both premises of the escape argument removed)" m5
echo "=============================================================="
echo "restored; re-verifying the tree"
$T 2>&1 | grep -E 'test result'
