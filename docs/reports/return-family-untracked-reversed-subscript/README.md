# RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT: a root inside a subscript's brackets may be its pointer

Card RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT (USER DECISION 2026-11-24:
"launch the two arithmetic-access follow-up cards"). The base is `03aadb221`,
the tip of RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC (rule 74w,
approved as r77), on rules 74t (r74), 74r (r72) and 74m (r68).
[PREDICTION.md](PREDICTION.md) was committed (`f06648a7a`) before the
implementation (`0efdfd0c4`). The card adds gateway rule
[74y](../../agents/gateway.md#constraint-74y), which extends rule 74w, and
`QCore.ReturnFamilyUntrackedReversedSubscript` with its audit. It also closes
RETURN-FAMILY-SUBSCRIPT-ENCLOSING-STARRED-ROOT, which r71 (rule 74s, not in
this base) registers.

## 1. Reachability

**Both gaps are reachable.** C11 6.5.2.1p2 defines `E1[E2]` as
`(*((E1)+(E2)))`, so the pointer may be either operand. A scratch end-to-end
probe (not committed; kept as `private/rfrs/zz_probe_rfrs.rs`) ran 33 bodies
with the base's `legacy_return_family.rs` (`private/rfrs/probe-base.log`)
and at the tip (`private/rfrs/probe-tip.log`). `p` is a null local,
`k` a `long` formal, `struct s { long n; long a[2]; struct s *next; }`.

| body (`return …;`) | base | tip |
|---|---|---|
| `1[p]`, `k[p]`, `k[(const long *)p]`, `k[(p)]`, `0[p + k]`, `k[p + 1]`, `1[(const long *)p + k]`, `0[(p + 1)]`, `(k + 1)[p]`, `0[p].n`, `k[p] + 1`; `long k = 1; … k[p]` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `0[p + (q - q)]`, `0[1[p] + p]` (unclassified sum in the brackets) | `Safe` | `Unknown`, return refusal |
| `long w(long **q, long n) { return n[*q]; }`, `n[*q + 1]` (may-bind `q`) | **`Safe`** | `Unknown`, return refusal |
| `long **q = k_alloc(8); … k_free(q); return 0[*q];` | `Unsafe` (`UseAfterRelease`) | `Unknown`, return refusal |
| `p[1]`; `p[i]` with `volatile long i` | `Unknown`, `unknown_alias:p` | same |
| `a[i]`, `a` a declared array, `volatile long i` (no scalar) | `Safe` | same |
| unbound formal `p`: `k[p]`, `1[p]`, `k[(const long *)p + 1]` | `Safe` | same |
| `&k[p] != 0` | `Unknown`, return refusal | same |
| `sizeof(k[p])` | `Safe` | same |
| tracked `p` (`k_alloc`), `0[p].n` | `Unknown`, return refusal | same |
| `long w(long *q) { const long a[2] …; return a[*q]; }`; `n[q]`; `*q` | `Safe` | same |
| `n[q[0]]` with `long **q`; `(0 ? p : p)[n]` | `Safe` | `Safe` (§7) |

At the base the untracked branch made a root a base only by the direct tests
(`p ->`, `p [`, `* p`) and rules 74r's, 74t's and 74w's walks. None reads a
root inside a subscript's brackets, and rule 74w's `enclosing_group` stops at
a `[`. For a may-bind root the starred count reads back only through `(` and
`*`, so the enclosing `[` was not counted. Rule 74s's `outside_dereference`
(r71) reads only parenthesised groups, so it does not cover this either.

## 2. Change and direction

* **`legacy_return_family.rs`.** All new code is in new functions after
  `scalar_read`: `SubscriptIndex`, `subscripted_pointer`,
  `enclosing_bracket`, `subscript_index` and `inside_undecided_subscript`.
  `NameFacts` gains `declared_pointer`, with a default of `false` (only more
  bases and refusals). `ParserNames` implements it as
  `typed_types.pointer_kind(name).is_some()`.
  `scalar_read` gains two inserted blocks: one in the untracked branch after
  the base push, and one in the may-bind branch after `if !root`. It also
  gains a module-header paragraph and one `mod` line. No existing function
  is edited.
* **Untracked roots.** `subscript_index` widens a span from an untracked,
  non-scalar root the way rule 74w's walk does: through a type group before
  it, a `(`/`)` around it, and parenthesised additive groups. It stops at a
  `[`/`]` right around the span, or at a sum whose innermost bracket is the
  `[`. The root is the index only when the subscripted operand is a declared
  pointer or array. A member name (after `.` or `- >`) is never treated as
  one, even when it spells a declared name.
  * An integer operand makes the root the pointer, so the root is a base.
  * An undecided operand makes both bases. The operand already is one when it
    is untracked: `b [ p ]` gives `[b, p]`.
  * A crossed sum that rule 74w would not classify refuses the expression.
  * A declared-scalar root is an index and is not read.
* **May-bind roots.** `inside_undecided_subscript` refuses an admitted
  may-bind root (`* q`, `q - > f` or `q [ i ]`) when any enclosing `[`
  subscripts an operand that is not a declared pointer: `n[*q]`,
  `1[q->m]`, `n[q[0]]`, `b[*q]`. `a[*q]` with a declared array keeps its
  admission, as the follow-up's DISCHARGE asks. The check uses none of rule
  74s's code, so it does not depend on r71.
* **Direction.** Every old base is kept. The admission is the base's,
  conjoined with the new conditions, and the new refusals are the
  expression's. So every result the tip admits, the base admitted
  (`the_fix_only_refuses_more`). The Lean model also proves, from the
  definition of the subscript, that the reader never reads a pointer inside
  the brackets as an index when its outside class is sound
  (`an_inside_pointer_is_never_missed`).
* **The readings the brief keeps.** These are unchanged at unit level and
  end to end: `p[i]` with an untracked `p`, `a[i]` with a declared array,
  tracked and may-bind roots outside brackets, unbound formals (`Safe`),
  `&n[p]` and `sizeof`.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_untracked_reversed_subscript.rs`,
  a new module of `legacy_return_family`; its oracle declares `a` a pointer):
  * `rule_74y_subscript_vectors_hold` reads `subscriptVectors` (20 rows). It
    checks each row's old base against the base's own functions (the direct
    tests, `dereferenced_through_groups`, `accessed_through_groups` and
    `accessed_through_arithmetic`), and the new fields against
    `scalar_read`.
  * `rule_74y_may_bind_vectors_hold` reads `mayBindVectors` (9 rows). It
    checks the inside field against `inside_undecided_subscript`, the
    admission against `scalar_read`, and the old admission where no
    undecided bracket encloses the root.
  * `an_untracked_root_inside_a_subscript_is_a_base`,
    `an_unclassified_sum_in_the_brackets_refuses`,
    `a_may_bind_root_inside_undecided_brackets_refuses` and
    `the_base_readings_are_kept`.
  * **Amended rule 74w pin**
    (`legacy_return_family_untracked_arith_access.rs::the_base_readings_are_kept`).
    It held `a [ p + 1 ]` and `a [ ( p + 1 ) ]` at `bases: ["a"]` under an
    oracle that declares no pointer, so `a` is undecided there and `p` may be
    the pointer: that is this card's gap. The pin now expects `["a", "p"]`.
    Rule 74w's Lean `arithVectors` are unchanged.
* **End to end** (`tests/return_family_untracked_reversed_subscript.rs`):
  * `a_null_untracked_root_inside_a_subscript_is_a_base` (12 rows: `Unknown`,
    `unknown_alias:p`);
  * `an_unclassified_sum_in_the_brackets_refuses` (2 rows);
  * `a_may_bind_root_inside_undecided_brackets_refuses` (3 rows);
  * `the_base_readings_are_kept` (direct subscript, a declared array or
    pointer subscripted by a non-scalar index, 3 unbound-formal rows, `&`,
    `sizeof`, a tracked root, and `a[*q]` under a declared array).

**Checked at the base reading.** With the implementation in place,
`subscript_index` was made to return `SubscriptIndex::None` at entry and
`inside_undecided_subscript` to return `false`. That is exactly the base's
reading, since the old tests are kept. Afterwards the source was restored
byte for byte from a saved copy (`cmp` clean, then `touch`).

* `private/rfrs/base-fixtures/unit.log`: 6 failed, 29 passed. Every new test
  failed except this card's `the_base_readings_are_kept`, and so did the
  amended rule 74w pin.
* `private/rfrs/base-fixtures/e2e.log`: the three change tests fail with
  `("Safe", [])`, and `the_base_readings_are_kept` passes.

No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyUntrackedReversedSubscript` and
`proof/AuditReturnFamilyUntrackedReversedSubscript.lean` have no placeholders
and no project axioms.

The model is grounded in C11 6.5.2.1p2. `subscript e1 e2` is the object a
`*` reads through `add e1 e2`, where `add` follows 6.5.6p2/p8 with the
pointer on either side. The theorems over it are:

* `subscript_comm`;
* `reversed_reads_the_inside_pointer`;
* `a_pointer_outside_reads_its_own_object`;
* `an_integer_outside_puts_the_pointer_inside`.

Over the reader's classes, `index_only_under_a_pointer_outside` and
`an_inside_pointer_is_never_missed` hold. The read-level
`the_fix_only_refuses_more` covers both untracked and may-bind roots. The
layers, the outside class and the may-bind position are the Rust reader's
output (trust boundary 28), bound by the two vector lists.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `03aadb221` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `0efdfd0c4`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset**
(`moved-rows-base-tip.jsonl` is `{"moves": {}}`, and
`changed-rows-base-tip.txt` is empty). There is no moved row to account for.
That matches the prediction (`none` on every route).

The census found no literal reversed subscript on the route. Its 245
call-free ordinary `X[i]` hits keep their readings: their indices are
formals, declared scalars, or locals whose new base obligation discharges.
No precision was lost on the route.

**Pinned routes were not run.** Their inputs are still absent on this host,
as they were for rules 74r, 74t and 74w:
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`
and `build/cycle19`. No move is predicted there. A move would be
`Safe` -> `Unknown` at a `return` family S admits, with a possibly-null
untracked root inside a subscript's brackets under an operand that is not a
declared pointer, or a new return refusal.

## 6. Gate

The scoped card gate was run as `python3 scripts/gate_scope.py --base
03aadb221 --out private/rfrs/gate-1`, through `private/toolchain/env.sh`
(Clang 21, Lean 4.35), at head `0efdfd0c4` with a clean tree. It exited 0,
with all 8 steps passed. Later commits change only this report and its
evidence. The record is [gate/gate-1.json](gate/gate-1.json), and the logs
are in this worktree's `private/rfrs/gate-1/`. Both measurement runs ran
alongside the gate.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 689 units, 734 summaries; 10,042 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 501 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 7. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and has not been applied. It retires two follow-ups:

* RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT, registered by r77;
* RETURN-FAMILY-SUBSCRIPT-ENCLOSING-STARRED-ROOT, registered by r71. It has
  the same shape (which side of a subscript is the pointer) and needs none of
  r71's code.

It registers nothing new, and `status_edits` is empty, because STATUS
states no fact this card changes.

The ids it retires are registered by r77's amendments and by r71's, which
sit on r68's, r72's and r74's. So `--check` against this tree alone is
refused. I extracted a `git archive` of this card's head into
`private/rfrs/amend-root`, added r71's report directory from `42e814d75`,
and applied the amendments of r68, r71, r72, r74 and r77 in that order. Each
exited 0 (`private/rfrs/amend-apply-*.log`). Against that root, `--check` on
this card's file exits 0 (`private/rfrs/amend-check.log`). **The
integrator must apply r71's amendments before this card's.**

**Observed, not registered.**

* `long w(long **q, long n) { return n[q[0]]; }` is `Safe` at both base and
  tip. Family S refuses it at the tip (unit pin `n [ q [ 0 ] ]`), so an older
  route admits it before family S is asked. The decision belongs to that
  route's typed (Clang-AST) reading, which knows which operand is the
  pointer, so it is not this card's syntactic gap.
* `(0 ? p : p)[n]` is RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS, the
  sibling card (rule 74x).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`:
  * the module header (a paragraph after rule 74w's);
  * the `NameFacts` trait (one defaulted method);
  * `ParserNames` (one method);
  * `scalar_read`'s untracked branch (a block after the base push) and its
    may-bind branch (a block after `if !root`);
  * new functions after `scalar_read`;
  * one `mod` line at the end.

  RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS (rule 74x, same base) works in
  the same branch. r71 adds a `&& !outside_dereference(...)` line four lines
  above this card's may-bind block.
* `crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_arith_access.rs`
  (one amended pin).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files:

* `docs/agents/rules/74y.md`;
* `tests/return_family_untracked_reversed_subscript.rs`;
* `tests/unit/legacy_return_family_untracked_reversed_subscript.rs`;
* `proof/QCore/ReturnFamilyUntrackedReversedSubscript.lean`;
* `proof/AuditReturnFamilyUntrackedReversedSubscript.lean`;
* this report, with its scripts and evidence.

## 8. Increment paragraph

The paragraph that `apply_amendments.py` prepends to the current increment
record is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json).
