# UNTRACKED-BASE-NULL-JOIN-DISCHARGE: a null constant on some reaching path is never rule 14n's premised alias

Card UNTRACKED-BASE-NULL-JOIN-DISCHARGE, launched on USER DECISION
2026-11-24 ("launch the null-join card"). The base is `2a598d2af`, the tip of
RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT (rule 74v, approved as r80), on rules
74t (r74), 74r (r72) and 74m (r68), on main `3ca85c7a6`.
[PREDICTION.md](PREDICTION.md) was committed (`1f121e6c1`) before the
implementation (`bd9aa123e`). The card adds gateway rule
[83](../../agents/gateway.md#constraint-83) and `QCore.NullJoinDischarge`
with its audit.

**Trade-off, measured:** no row moves status on the opt-in route; 15 rows
that were already `Unknown` gain 36 refusals. No `Unknown -> Safe` move.

## 1. Diagnosis

The registered gap reproduces, and the cause is not the base obligation
itself but **rule 14n's premise reading after the pointer join**. An
uncommitted probe (`private/unj/probe/zz_probe_null_join.rs`, logs
`private/unj/probe/run2-base.log` and `run2-tip.log`) shows every admitting
row at the base was `Safe` **under the premise `null-deref:w:LINE:p`**.

The code path for `long w(const struct s *f) { const struct s *p = 0; if (f) p = f; return p->n + 1; }`:

1. `p = 0` lowers to `Clear`. Its transfer (`legacy_transfer.rs`,
   `LegacyInstruction::Clear`) binds `p` to
   `{ targets: ∅, unknown: false, may_be_null: true, Identity }`, the null
   constant.
2. `p = f` lowers to `Alias { p, f }`. `f` is an unbound formal, so rule 62's
   custody-free arm (`AG2-UNRESOLVED-ALIAS hook`) **removes** `p`'s binding.
3. At the merge, `AbstractState::join` sees `(Some(null), None)`. Its
   one-sided arm sets `unknown = true` and `may_be_null = true`. With a
   pointer read on the other side (`p = f->next`), the two-sided arm joins
   `unknown` by disjunction to the same result.
4. Family S lowers `p->n + 1` to `IndirectScalarAccess { bases: ["p"] }`. The
   transfer asks `null_premise_route::base_resolves`.
   `indirect_scalar_base_resolves` refuses (`may_be_null`), but
   `legacy_null_premise::premised` accepts: the joined binding is target-free,
   `unknown`, possibly null and the identity derivation, which is exactly
   `null_premise::premised_shape`. The premise is noted and the base
   discharges.

Rule 14n never premises a *known* binding, so the null constant alone
refuses (`const struct s *p = 0; return p->n + 1;` is `Unknown`). The join
forgets that knowledge, and the premise then names an assumption the
function's own null constant refutes on one path. The bare `return p->n;`
(rule 74v routes it to the same node) and the declaration route take the same
path.

**Why not "the join is sound".** Under decision 1 the `Safe` was conditional
on a listed premise, and the null property (rule 14r) would treat the null
constant on a path as a strong source. But rule 14n scopes its premise to
what a pointer read and rule 14b bind, values the reader cannot see, and
excludes a known binding. Letting a join with an unbound path turn the null
constant into a premised alias contradicts that exclusion. The fail-closed
reading settles it, so this is no design question. Unbound formals stay out
of scope: their nullness is unchanged.

**The guards.** A null test of the joined binding (`if (p)`, `if (!p) return
0;`) was already `Unknown` at the base. `refine_pointer_null` answers
`PointerRefinement::Unknown` for an `unknown` binding (rule 10b), and the
work list reports `unknown_alias:p`. This card keeps that verdict.

## 2. Census of the mechanism

Probe rows, base → tip (premise variable in brackets at the base):

| shape | base | tip | reachable with a null? |
|---|---|---|---|
| `if (f) p = f;` then `p->n + 1` / `p->n` / `long x = p->n;` | `Safe` [p] | `Unknown` `unknown_alias:p` | yes, when `f` is null |
| `if (c) p = f;` (no guard) | `Safe` [p] | `Unknown` | yes, when `!c` |
| `if (!f) p = 0; else p = f;` (inverse guard), also over `f->next` | `Safe` [p] | `Unknown` | yes |
| `if (c) p = f->next;` / `if (f->next) p = f->next;` | `Safe` [p] | `Unknown` | yes |
| statement `x = p->n;`, condition `if (p->n)`, store `p->n = 1;` | `Safe` [p] | `Unknown` | yes |
| `p[0] + 1`, `*p + 1`, chain `p->next->n + 1` | `Safe` [p] | `Unknown` | yes |
| copy `q = p; return q->n + 1;` | `Safe` [q] | `Unknown` `unknown_alias:q` | yes |
| `p = c ? 0 : f;` | `Unknown` `f` | `Unknown` `f`, `p` | yes, already refused |
| `p = 0; if (c) { p = k_alloc(…); if (!p) return 0; … }` (tracked) | `Unknown` | `Unknown` | tracked: never premised |
| loop-carried `while (c--) p = f;`, list walk | `Unknown` | `Unknown` | already refused |
| `k_read(p->n, 0, 0)` call actual | `Safe`, no premise | `Safe`, no premise | yes: RETURN-CALL-ACTUAL-NULL-UNTRACKED-ROOT, open |

Every admitting row erred in the admitting direction. The call actual is a
different route (C02-P's borrowed actual, no base obligation). 74v registered
it, and it stays open.

## 3. Change

* **New module** `crates/qcore-c-gateway/src/legacy_null_join.rs`
  (`null_join_route`). It keeps `AbstractState::null_on_path`, the names some
  reaching path bound definitely null (no target, known, possibly null) since
  their last rebinding.
  * `join` is the union of both sides' sets plus every name either side
    binds definitely null. It keeps only names the joined state still
    binds.
  * `transfer` removes every name the node rebinds (`rebound_names`, and an
    `Alias` destination of any spelling). It carries a flagged source to a
    copy's destination.
  * `refuses_premise` is the only reader.
* **Hook points** (all marked `UNTRACKED-BASE-NULL-JOIN-DISCHARGE hook (rule 83)`):
  1. `legacy_transfer.rs`: the `#[path] mod null_join_route;` line after
     `out_slot_call_write`.
  2. `legacy_transfer.rs`: the `null_on_path` field, last in
     `AbstractState`.
  3. `legacy_transfer.rs`: one call, `null_join_route::join(self, &before, incoming)`,
     just before `*self != before` at the end of `AbstractState::join`.
  4. `legacy_transfer.rs`: in `transfer`, a clone of the entry set before
     `transfer_dispatch` and one `null_join_route::transfer` call after it.
  5. `legacy_null_premise.rs`: `premised` gains
     `&& !…::refuses_premise(state, base)`. This covers the base, chain and
     chain-owner readings.
* `legacy_c.rs` is not touched.
* **Direction.** Only the premise reads the set, and it can only refuse
  more. A null-free join leaves the set empty for that name. A guard-narrowed
  non-null binding is not possibly null, so it needs no premise. A tracked
  binding is never premised.

## 4. Pins

* **End to end** (`tests/untracked_base_null_join.rs`, which asserts status,
  diagnostic keys and premise variables for every row):
  * `the_registered_spellings_are_refused`: the 3 registered spellings.
  * `every_census_shape_is_refused`: 11 shapes, the copy, and the ternary's
    added `p` leaf.
  * `guards_and_null_free_bindings_keep_their_verdicts`, `Safe` with no
    premise:
    * a copy of a formal;
    * `p = f; if (!p) return 0;` (family S and bare);
    * `p = f; if (p) …`;
    * the read inside the assigning arm;
    * both arms overwriting the null;
    * a declared non-null result (`k_make`, `return_owned(nonnull)`);
    * a tested allocation.
  * The same test, `Safe` with premise `p`: a member read, a null overwritten
    by a member read, and both arms overwriting with a member read.
  * The same test, kept `Unknown`: the null constant alone and the three
    tests of the joined binding (`if (p)` twice, `if (!p) return`).
  * `near_misses_are_refused`, `Unknown`:
    * a test of another name;
    * a test before a later `p = 0`;
    * a test inside a loop;
    * a test of the correlated formal (`if (f) p = f; if (!f) return 0;`).
      This is a precision loss from the base's `Safe` (§7).
* **Unit** (`tests/unit/legacy_null_join.rs`):
  * `rule_83_join_vectors_hold`: 14 rows read from
    `QCore.NullJoinDischarge.joinVectors`, joined with the real
    `AbstractState::join` and read with and without the refusal;
  * the flagging, copy and rebinding cases;
  * the guard case.
* **Checked at the base.** `legacy_transfer.rs` and `legacy_null_premise.rs`
  were replaced byte for byte with the base's copies, so the new module is
  not compiled. The end-to-end test was run
  (`private/unj/base-fixtures/e2e.log`), and the tip bytes were restored
  (`git diff` clean, then `touch`).
  * `the_registered_spellings_…`: 3 rows differ, `Safe` with premise `p`.
  * `every_census_shape_…`: 13 rows differ.
  * `near_misses_…`: 3 rows differ (two `Safe` with a premise, one premise
    withdrawn).
  * `guards_…_keep_…` passes at the base.
  * The unit module tests new code.
* **Pins:** no contract, lock, submodule or CT pin is touched.

## 5. Lean

`QCore.NullJoinDischarge` and `proof/AuditNullJoinDischarge.lean` have no
placeholders and no project axioms. Every theorem depends on `propext` at
most.

* It models one name's target-free binding: four path bindings, the Rust
  join of the two bits, the flag, rule 14n's premise and the base obligation.
* **Soundness:** `a_null_on_some_path_is_never_discharged` holds for any
  number and order of joined paths, by induction with a taint invariant.
  `a_null_on_some_path_is_never_premised` follows.
* **Refusal only:** `the_fix_only_refuses_more`.
* **Precision:** `no_null_keeps_the_verdict`, by a clean invariant.
* **Guards:** `a_guard_proved_non_null_is_discharged` holds whatever the
  flag. Also proved: `an_overwritten_null_keeps_the_premise` and
  `an_unbound_base_discharges`.
* **The gap:** `old_premises_a_joined_null`.
* **Vectors:** `join_vectors_hold`.

The lowering from C to path bindings is trust boundary 28's, bound by the
vectors and the end-to-end pins.

## 6. Measurement against the prediction

Opt-in `current --monkey-linked` over Fluent Bit 046accf6. Each side ran
from its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `2a598d2af` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `bd9aa123e`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,994 |

* **Status.** No row moves ([moved-rows](evidence/moved-rows-base-tip.jsonl)
  is `{"moves": {}}`), which matches the prediction (0 expected, range 0 to
  5). `pack_summary_metric` and `get_array_item` did not move: their null
  constant is overwritten on every path before the premised read, as
  predicted. The 44 `Safe` rows still carry their premises, and the console
  line is unchanged.
* **Changed rows.** 37 rows change
  ([rows-by-unit](evidence/rows-by-unit-base-tip.jsonl)), all `Unknown` on
  both sides. Each drops premises that now refuse.
  * 15 of them gain 36 diagnostics, and none loses one
    ([leaf-diff](evidence/leaf-diff-base-tip.jsonl)). 34 are `unknown_alias`
    on the premised name at the premised line, in 13 rows. 2 are
    `indirect memory access is outside the resource projection`, in
    `decode_numerical_data_point` and `decode_numerical_samples`. Those are
    the access-leaf +2.
  * The other 22 rows keep their diagnostics exactly and only drop
    premises. Each was `Unknown` on other refusals already.
* **Against the prediction.** The prediction expected fewer than 10 rows to
  gain an `unknown_alias` leaf. **13 did**, plus the 2 access leaves, so the
  leaf figure exceeded the prediction. The census counted only premise
  variables written a null constant lexically. It missed nulls that reach
  through a conditional assignment of a different spelling (`ic = NULL;` in
  one branch of `input_chunk_get`) and nulls from a test's null arm.
  * Spot checks: `input_chunk_get` writes `ic = NULL;` on one branch before
    the later reads. `mk_plugin_load`'s `plugin` is joined with a null on a
    path to its member reads. The null reaches the access through a join, so
    these are the mechanism's genuine refusals.
  * No status depends on them, so none is a verdict-relevant precision loss.

**Pinned routes.** These were not run. Their inputs are absent on this host,
as rule 74r's attempt recorded (no
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`,
no `build/cycle19`). No result is fabricated. A move there would be
`Safe -> Unknown` at a dereference of a binding joined from the null
constant.

## 7. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 2a598d2af
--out private/unj/gate-1` (Clang 21, Lean 4.35), head `bd9aa123e`, **exit 0,
all 8 steps passed**. While it ran, this report and its evidence were
uncommitted docs files, and the later commits change only those. The record
is [gate/gate-1.json](gate/gate-1.json).

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 689 units, 734 summaries; 10,041 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 501 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected. The Rust
suite's 7,357 s include its wait for the slot queue.

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied.

* It retires UNTRACKED-BASE-NULL-JOIN-DISCHARGE.
* It registers **NULL-JOIN-CORRELATED-GUARD**. After `if (f) p = f;`, a test
  of `f` (`if (!f) return 0;`) proves `p` non-null at the read, but this
  reader does not relate the two. The row was `Safe` under a premise at the
  base and is `Unknown` now. The dominating test of the joined binding itself
  (`if (p)`) is `Unknown` on both sides, because rule 10b refuses a null test
  of an `unknown` binding. A sound restoration would narrow the joined
  binding on a test of the name it was copied from, or let a null test of a
  target-free `unknown` binding narrow without refusing. It moves no corpus
  status today.
* `status_edits` is empty: no STATUS fact changes.

The id this card retires is registered by r80's amendments, which are
committed here but not applied, so `--check` against this tree alone is
refused. A `git archive` of HEAD was extracted into `private/unj/amend-root`,
and r68's, r72's, r74's and r80's amendments were applied there in order
(`return-family-deref-resolved-cast`, `return-family-untracked-deref-cast`,
`return-family-untracked-member-cast`, `return-strict-member-null`). All
four exited 0 (`private/unj/amend-apply-*.log`). Against that root, with this
report directory copied in, `--check` exits 0
(`private/unj/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_transfer.rs`, at the four hooks of §3.
  Any card adding an `AbstractState` field or a line at the end of
  `AbstractState::join` or around `transfer_dispatch` (r66, r67 and queued
  gateway cards) conflicts textually at those lines only.
* `crates/qcore-c-gateway/src/legacy_null_premise.rs`: `premised`.
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. On a conflict, rerun `gen_rule_docs.py` and
  `gen_lean_roots.py`.

New files:

* `docs/agents/rules/83.md`;
* `src/legacy_null_join.rs`;
* `tests/untracked_base_null_join.rs`;
* `tests/unit/legacy_null_join.rs`;
* `proof/QCore/NullJoinDischarge.lean`;
* `proof/AuditNullJoinDischarge.lean`;
* this report, with its scripts and evidence.
