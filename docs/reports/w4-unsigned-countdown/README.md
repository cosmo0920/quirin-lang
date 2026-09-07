# Unsigned countdown custody inference

Baseline `46b2939`, 2026-09-07. W4 now infers custody for recursive C wrappers
whose single immutable, explicitly unsigned control decreases by exactly one.
This extends the earlier literal-zero context route. Release and fresh-return
wrappers can validate, including mutual recursion with different parameter slots
and unsigned widths. General recursive summary inference remains open.

## Admission and validation

The complete recursive component must use supported explicit unsigned built-in
controls for any countdown call to qualify. Every recursive control actual is
exactly `0` or `control - 1`; all other actuals retain the prior flat pointer/name
vocabulary. Control occurrences outside recognized zero/nonzero guards and these
exact arguments reject, including mutation, aliases, shadowing and other calls.
Signed arithmetic, subtracting two, increments, unchanged counters, typedefs,
volatile/atomic controls, multiple scalar controls and broader expressions stay
outside this rule. Existing signed literal-zero recursion is preserved.

Every zero-context body validates with all component contracts absent. Even an
abstract-state-infeasible remaining Unknown CFG node prevents publication, so
resource preconditions cannot erase a zero-context recursion or underflow case.
For positive unsigned inputs, subtracting one is representable after C integer
promotion. Conversion to another unsigned width can only reduce that natural
value further. Every admitted recursive call therefore uses a strictly lower
counter. The nonzero body checks under the common lower-rank contract and must
reproduce the complete base contract. This is mutual strong induction over
concrete counter values; the finite zero/nonzero dependency table alone would
not justify countdown recursion.

Only the proved scalar argument is replaced with a zero placeholder when
building the resource CFG. This does not assert that its runtime value is zero.
The candidate must borrow the scalar control, and every resource actual, call,
exit and effect stays in the analysis. The two contexts' disjoint union is
revalidated. Original source remains in the certificate; rechecking independently
extracts the source rank conditions, rebuilds the abstraction and re-derives the
summary. Declared contracts still take precedence, and ordinary inference runs
first.

## Evidence and limits

Nine `QCore.UnsignedCountdown` theorems in `ContextualCustody.lean` establish
unsigned conversion monotonicity, decreasing arguments, well-founded calls,
exclusion of zero/same-rank dependencies, mutual rank induction and agreement
with the existing nonempty custody-exit validator. `AuditW4.lean` includes them.
The C parser, complete call inventory, source-to-CFG extraction and scalar
abstraction remain trusted implementation premises. This is resource-projection
evidence; it does not prove native stack capacity, general C termination or
arbitrary machine behavior.

Six new public-API tests exercise real Clang imports and independent certificate
rechecking: all nine supported unsigned type spellings across four guards,
mutual width/slot changes, mixed zero/countdown edges, owned returns and caller
leaks, seventeen invalid step/resource bodies, five unsupported signed/Boolean
types, a mixed signed component, and source/rank/summary tampering. The sixth
certifies the exact source used by the native fixture. Existing literal-zero,
declaration-precedence and other recursion regressions pass unchanged.

The native fixture allocates and releases six objects across zero, one, seven,
owned-return and mutual width-conversion cases. Its nonnull allocation contract
is a harness premise: allocation failure aborts. ASan, UBSan and LSan report no
error on this fixed execution. The first traced run failed because LSan cannot
operate under ptrace; the identical executable passed untraced. No claim about
all recursion depths follows from this bounded observation.

Reproduce from the repository root:

```sh
cargo test -p qcore-c-gateway --test contextual_recursion
clang -std=c11 -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  crates/qcore-c-gateway/tests/fixtures/unsigned_countdown.c \
  docs/reports/w4-unsigned-countdown/native-harness.c \
  -o /tmp/qcore-countdown-native
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-countdown-native
```

The full standing gate passes: 760 workspace tests, formatting and Clippy,
both reevaluation probes, Lean build and all 20 axiom audits. The excluded
provider workspace passes 31 tests and its formatting/Clippy checks; all 43
Python tests pass. No CT mapping changes (8 mapped /42 unmapped).

The full 185-unit audit uses the persistent Monkey compilation database and
configured production process packages. All four report artifacts are
byte-identical to baseline: 2 Safe /42 provisional Unsafe /137 Unknown plus
4 data-only units, and 3 checked /93 Unknown source-effect functions. No raw
corpus result is promoted. `audit-parity.json`, `validation.json`, compressed
outputs and logs record the comparison. Subagents were unavailable due to their
reported usage limit; this increment was implemented and reviewed locally.
