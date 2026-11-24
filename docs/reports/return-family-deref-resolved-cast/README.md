# RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST: a starred may-bind root counts through cast groups

Card RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST (USER DECISION 2026-11-24:
"launch them all"). Base `3ca85c7a6` (`integrate/r65`).
[PREDICTION.md](PREDICTION.md) was committed (`3108d57c0`) before the
implementation (`1eae6f5e6`). This adds gateway rule
[74m](../../agents/gateway.md#constraint-74m), which extends rule 74h and rule
12h(4), and `QCore.ReturnFamilyDerefCast` with its audit.

## 1. Reachability

**It is reachable.** The follow-up accepted either a fix or a proof that no
such return reaches family S. The fix is below.

A scratch end-to-end probe at the base (not committed; log in the worktree's
`private/rfd/probe-base.log`) and the committed pins
(`tests/return_family_deref_resolved_cast.rs`) show:

| body (`k_alloc`/`k_free` declarations + …) | base | tip |
|---|---|---|
| `long w(void **q) { return *(long *)*q; }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *(long *)*q; }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *(long *)(*q); }` | **`Safe`** (by reading) | `Unknown`, return refusal |
| `long w(long **q) { return **q; }` (control) | `Unknown`, return refusal | same |
| `void **q = k_alloc(8); … k_free(q); return *(long *)*q;` | `Unsafe`, `UseAfterRelease` + `unknown_alias:q` | `Unknown`, return refusal |
| `long **q = k_alloc(8); … k_free(q); return *(long *)(long *)*q;` | `Unsafe`, same | `Unknown`, return refusal |
| `long **q = k_alloc(8); … k_free(q); return **q;` (control) | `Unknown`, return refusal | same |
| `long w(void **q) { return (long) *q; }` | `Safe` | same |
| `long w(long *q) { return *(long *)q; }` | `Safe` | same |
| `long *q = k_alloc(8); … k_free(q); return` `(long) *q` / `*(long *)q` / `*q` | `Unsafe`, `UseAfterRelease` + `unknown_alias:q` | same |

"Return refusal" is `unsupported:unmodeled call, aggregate or indirect access
in return expression`. At the base, family S's `stars_before` stopped a
starred root's count at the nearest cast group's `)` (`closing` finite), so
`* ( long * ) * q` counted one `*`. Family S admitted `q` as one dereference
base, and the outer `*`, which dereferences the pointer `*q` reads, carried no
obligation. The statement form `long v = *(long *)*q;` is refused elsewhere
("indirect memory access is outside the resource projection"), so the `return`
was the one route that certified it.

## 2. Change and direction

* **`legacy_return_family.rs`.** `stars_before` takes `through: bool`. With
  it, a `)` that `cast_semantics::type_group_open` reads as a type group is
  crossed at any `closing`. This is the arm rule 12h added for member and
  subscript roots, which now pass `true`. It is not a second detector. A
  starred may-bind root is admitted only when the count is exactly one
  **both** with `through` false and with it true, under **both** of rule
  74h's readings. The module header gains a rule 74m paragraph.
* **Direction.** The count with `through` is never less than the count
  without it, because the loop is identical until the old one stops at the
  `)`. Every admission at the tip was therefore an admission at the base
  (`the_fix_only_refuses_more`). An admitted root counts exactly one `*`
  under every reading C may take of rule 74h's undecided groups
  (`admitted_is_one_dereference`, via `through_mono` and `inner_mono`).
* **The readings the brief keeps.** `*(T *)q` dereferences the converted
  value. Its count without `through` is 0, so family S still refuses it, and
  the route that decided it at the base still does (`Safe` for a formal,
  `Unsafe` after a release). `(T) *q`, `(int)(long) *q`, `( * q )` and
  `(long) (*q)` keep their admission. `sizeof ( long ) * q` still counts one,
  as the follow-up noted. A conforming body cannot multiply by a pointer
  (C11 6.5.5p2), and a declared scalar never reaches the may-bind test.
* **The one movement away from an accusation.** After `k_free(q)`,
  `return *(long *)*q;` was `Unsafe`. The flow check accused the first
  dereference, which C does evaluate. It is now the return refusal, which is
  what `return **q;` gets. That is `Unsafe` -> `Unknown` on a test fixture,
  and it is the cost of no longer admitting the second dereference. Nothing
  moves toward `Safe`.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_deref_cast.rs`, a new module of
  `legacy_return_family`):
  * `rule_74m_deref_vectors_hold` reads `derefVectors` (10 rows) from
    `proof/QCore/ReturnFamilyDerefCast.lean` and checks each spelling's
    admission. `deref_vectors_hold` and `old_vectors_hold` prove the same
    rows, and their base readings, in Lean.
  * `a_star_through_a_cast_group_is_a_second_dereference` covers
    `* ( long * ) * q`, two and three nested casts, `* ( long * ) ( * q )`,
    `* ( ( long * ) * q )` and two arithmetic contexts.
  * `the_base_readings_are_kept` covers `*(T *)q` (resolved and not),
    `* ( T * ) * q`, the admitted casts of one dereference, the untracked
    `p`, and `sizeof`.
* **End to end** (`tests/return_family_deref_resolved_cast.rs`):
  `a_star_through_a_resolved_cast_is_a_second_dereference`,
  `a_released_root_under_a_second_dereference_is_refused`, and
  `one_dereference_and_the_converted_value_are_unchanged`, which passes at
  the base too and pins that nothing else moves.

**Checked at the base.** With the implementation in place, the root's
`[false, true]` was set to `[false]`, which is exactly the base's count. The
new tests were run, with logs in the worktree's `private/rfd/base-fixtures/`.
The source was then restored byte for byte from a saved copy (`cmp` clean,
then `touch`).

* `unit.log`: `rule_74m_deref_vectors_hold` and
  `a_star_through_a_cast_group_is_a_second_dereference` fail.
  `the_base_readings_are_kept` passes, and so do the 16 other
  `return_family` unit tests.
* `e2e.log`: `a_star_through_a_resolved_cast_is_a_second_dereference` fails
  with `("Safe", [])`, and `a_released_root_under_a_second_dereference_is_refused`
  fails with `("Unsafe", ["UseAfterRelease", "unknown_alias:q"])`. The control
  test passes.

**Pins:** none moved. No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyDerefCast` (imports `QCore.ReturnFamilyStarCast` for
rule 74h's readings) and `proof/AuditReturnFamilyDerefCast.lean`, with no
placeholders and no project axioms. Each item before the root (a `*` with its
token class, a cast group, a `*` outside the root's parentheses, anything
else) is the Rust reader's output (trust boundary 28), bound by `derefVectors`.

## 5. Measurement against the prediction

MEASUREMENT

## 6. Findings outside scope (registered)

The same end-to-end probe found two pre-existing admitting gaps. Both read
the same way at the base and the tip, and neither is about counting through a
cast group for a may-bind root, so they are registered rather than fixed:

* **RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST.**
  `long w(void) { const long *p = 0; return *(long *)p; }` is `Safe`, while
  `return *p;` is `Unknown` (`unknown_alias:p`). An untracked root is a
  starred base only when the token right before it is a `*`, so
  `* ( long * ) p` gives `bases: []` (pinned as the current reading in
  `the_base_readings_are_kept`).
* **RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES.**
  `long w(long **q) { return *(*q + 1); }` and `return *((long *)*q + 1);`
  are `Safe`, while `return **q;` is refused. `stars_before` stops at a `*`
  behind more opening parentheses than close right after the root, so the
  outer `*` over the computed pointer is not counted. The probe log is in the
  worktree's `private/rfd/probe-tip.log`.

## 7. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base 3ca85c7a6 --out
private/rfd/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `1eae6f5e6`. It exited 0 with all 8 steps passed. Later commits
change only this report and its evidence. The record is
[gate/gate-1.json](gate/gate-1.json), and the logs are in the worktree's
`private/rfd/gate-1/`. The two measurement runs ran alongside it.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 685 units, 730 summaries; 10,015 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 497 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) (schema
`quirin-amendments/2`, `apply_amendments.py --check` exit 0, not applied)
retires RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST and registers the two
follow-ups in §6. `status_edits` is empty, because STATUS states no fact this
card changes.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs` (`stars_before`, the
  starred-root admission, the module header and one `mod` line);
* the generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74m.md`,
`tests/return_family_deref_resolved_cast.rs`,
`tests/unit/legacy_return_family_deref_cast.rs`,
`proof/QCore/ReturnFamilyDerefCast.lean`,
`proof/AuditReturnFamilyDerefCast.lean`.
