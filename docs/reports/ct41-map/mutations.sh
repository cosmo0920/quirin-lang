#!/usr/bin/env bash
# Card `CT41-MAP` (reduced) — non-vacuity harness for the two controls this card
# adds to the landed CT41 fixture.
#
# Only the added arms are mutated; the landed mapping's own arms are its card's
# to defend. Each mutation edits THE RULE, not the test; the named test must go
# RED; the file is then restored by rewrite (`cp` from a copy taken before the
# mutation, then `touch`) and its sha256 compared with the pre-mutation value.
# A mutation that leaves the suite green is an arm no test drives.
#
# Run from the repository root:
#     bash docs/reports/ct41-map/mutations.sh <output-directory>
# Exit 0 means every mutation reddened its test and every file was restored
# byte-identically.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
OUT="${1:?usage: mutations.sh <output-directory>}"
mkdir -p "$OUT"
cd "$ROOT" || exit 2

GATEWAY="crates/qcore-c-gateway/src/atomic_publication_source.rs"
RUNNER="compiler/tests/support/ct_atomic_publication.rs"

failures=0

# $1 label, $2 file, $3 "old=>>new" (old must occur exactly once)
mutate() {
    local label="$1" file="$2" expression="$3"
    local before after
    before="$(sha256sum "$file" | cut -d' ' -f1)"
    cp "$file" "$OUT/$label.orig"

    python3 - "$file" "$expression" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
old, new = sys.argv[2].split("=>>", 1)
if text.count(old) != 1:
    sys.exit("mutation site is not unique: %d occurrences" % text.count(old))
path.write_text(text.replace(old, new))
PY
    if [ $? -ne 0 ]; then
        echo "MUTATION $label: FAILED to apply" | tee -a "$OUT/summary.txt"
        failures=$((failures + 1))
        return
    fi

    cargo test -p qcore-compiler --test ct_corpus >"$OUT/$label.log" 2>&1
    echo "$?" >"$OUT/$label.exit"

    # Restore by rewrite, never by preserving mtime.
    cp "$OUT/$label.orig" "$file"
    touch "$file"
    after="$(sha256sum "$file" | cut -d' ' -f1)"

    if [ "$before" != "$after" ]; then
        echo "MUTATION $label: RESTORE FAILED ($before != $after)" | tee -a "$OUT/summary.txt"
        failures=$((failures + 1))
        return
    fi
    if [ "$(cat "$OUT/$label.exit")" = "0" ]; then
        echo "MUTATION $label: VACUOUS — the test stayed green" | tee -a "$OUT/summary.txt"
        failures=$((failures + 1))
        return
    fi
    echo "MUTATION $label: red (exit $(cat "$OUT/$label.exit")), restored sha256 $after" \
        | tee -a "$OUT/summary.txt"
}

: >"$OUT/summary.txt"

# A1 — the arm `qualified_atomic.c` exists to pin. Admitting any spelling that
# merely CONTAINS `_Atomic` would admit `volatile _Atomic int`, which this
# bounded route does not model and which the volatile arm must not claim.
mutate a1-atomic-flag-exactness "$GATEWAY" \
    '"_Atomic(int)" => None,=>>spelling if spelling.contains("_Atomic") => None,' \
    || true

# A2 — the volatile arm must not swallow the qualified-atomic flag. Widening it
# back to the substring test this card originally measured against would report
# `volatile _Atomic int` under a sentence that is false of it.
mutate a2-volatile-arm-exactness "$GATEWAY" \
    '"volatile int" => Some(AtomicPublicationSourceRefusal::VolatileFlagHasNoSynchronization),=>>spelling if spelling.contains("volatile") => Some(AtomicPublicationSourceRefusal::VolatileFlagHasNoSynchronization),' \
    || true

# A3 — the language-boundary block must really require C to REFUSE. Inverting
# the expectation must fail, or the block would say nothing about the language.
# (The inverted form still type-checks, so this reddens on the assertion rather
# than on the compiler — a mutation that only breaks the build proves nothing.)
mutate a3-language-boundary "$RUNNER" \
    '.expect_err("a language-boundary control is not admissible C11");=>>.expect("inverted: the boundary control would have to be valid C11");' \
    || true

# A4 — and it must require the refusal to be FOR THE STATED REASON, not merely
# that some refusal occurred.
mutate a4-boundary-reason "$RUNNER" \
    'address argument to atomic operation must be a pointer to _Atomic type=>>a message Clang does not emit' \
    || true

# A5 — the derivation assertion: each added control must remain a single
# declaration substitution of the accepting control, so it changes the flag's
# type and nothing else.
mutate a5-derivation "$RUNNER" \
    'atomic.replace("static _Atomic int done", "static volatile int done"),=>>atomic.clone(),' \
    || true

# A6 — the boundary files must be inventoried. The landed runner asserts the
# fixture directory holds exactly the files it executes; if the boundary keys
# are dropped from that set, `volatile_atomic_ops.c` becomes an untracked file
# the inventory would have to reject.
mutate a6-boundary-inventory "$RUNNER" \
    'wanted_files.extend(boundary.keys().cloned());=>>' \
    || true

echo "failures: $failures" | tee -a "$OUT/summary.txt"
exit "$failures"
