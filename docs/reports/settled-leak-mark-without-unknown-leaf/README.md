# SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF: a leak marked beside a Pending hold is decided from settled states

Card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF. Base `07833c86a`
(integrate/r77). Commit order:

1. probe `1a7f46c9e` (census and base-pinning fixture; analysis unchanged);
2. [PREDICTION.md](PREDICTION.md) and the measurement scripts `0b56c1ed8`;
3. implementation `8e016d830`;
4. `c7b09ac14`: the fixtures move into `settled_leak_other_refusal`'s test
   target (no new target; see §4).

Gateway rule [84](../../agents/gateway.md#constraint-84) is amended in place.
Rules 74c and 6 gain amendment lines pointing to it. `QCore.SettledLeakJoined`
gains a last section. The card discharges the follow-up of the same name,
which SETTLED-LEAK-OTHER-REFUSAL-SCHEDULE recorded.

## 1. Reproduction at base

A call held for a human act (rule 10i) refuses its statement with a `Pending`
reason and hands its operands on, so it marks them unresolved. Rule 84 read a
marked identity only beside an `Unknown` leaf, so in a function whose only
blocking leaves were holds, rule 74c's held report decided it: published
exactly when an iterate had claimed it before the mark joined the exit.

**Fixtures** (`crates/qcore-c-gateway/tests/settled_leak_other_refusal/mark_without_unknown.rs`,
probe pin `the_verdict_reads_the_schedule_at_base` at `1a7f46c9e`). An inline
`k_*` family, held out of every production library. `k_close` is under a
contract awaiting review, and `k_note` is another held call.

| shape | order | base | tip |
|---|---|---|---|
| held `k_close(h, 0)` on one arm, other arm leaves `h` open | open arm first | Unsafe, 1 leak | **Unsafe, 1 leak** |
| same | held arm first | Unknown, 0 leaks (`Pending` after the first analysis, raised by rule 10i's standing analysis) | **Unsafe, 1 leak** |
| held close on both arms | either | Pending, 0 | Pending, 0 |
| held close on one arm, open arm passes the unrelated held `k_note` | either | Unknown, 0 | Unknown, 0 |

Every row keeps the hold's `awaiting-human-review` reason.

**Gateway suite census at base** (`QCORE_MARK_CENSUS`, feature
`indirect-census`; [suite-census-base.json](evidence/suite-census-base.json)).
It found 130 marked current-generation `(exit, identity)` lines in analyses
with no `Unknown` leaf at the settle point. The follow-up counted 114 exit
marks on its earlier base.

* **121 lines sit beside a `Pending` leaf**, in three fixture functions.
  The holds are `k_close` (108), `w_peek` (7), `app_close` (4) and
  `flb_input_collector_destroy`'s held callee frame (2).
* **9 lines sit beside no leaf at all**:
  `cross_unit_elements_released::an_imported_clause_reaches_the_caller_and_both_bodies_replay`.
  Rule 43's `ElementsReleased` effect (a `ContractCall` to `drain`) marks an
  entry-custody site. That mark is a modelled effect, not a refusal, and the
  site is exempt, so it can never be claimed.
* **None of the 130 was held by an iterate, and none is witnessed.** No
  existing pin rests on the hazard. Only the new fixture shows it.

**Fluent Bit census at base.** The census build of the probe ran the opt-in
route over all of Fluent Bit 046accf6 and wrote **no line**: no analysed
function there has a marked exit identity without an `Unknown` leaf. Its two
`Pending` rows, `rd_kafka_transport_ssl_clear_error` and
`tls_init_library`, carry no diagnostic leaf in the report.

**Stop rule.** The census shows no case where publishing or withdrawing would
have to invent a `Pending` reason, drop one, or let a row become `Safe`.
Beside a `Pending` leaf, a withdrawal keeps that leaf. Beside no leaf at all,
the only marks come from a modelled effect on an exempt site, and that case is
left unchanged (below). So the card is implemented, not stopped.

## 2. The change

**Rule 84, amended** (`crates/qcore-c-gateway/src/legacy_mark_without_unknown.rs`, new).

* **`blocking(unknown_leaf, pending_leaf) = unknown_leaf || pending_leaf`**,
  read through `diagnostic_verdict` on the diagnostics at the settle point
  (`blocking_leaf`). `joined_leak_route::candidates` now calls
  `decides(true, blocking_leaf(..))` where it used the `Unknown` leaf alone.
  The candidates, the restricted replay, the witness and the decision are
  unchanged. The exit loop is factored out as `marked_exits` so the census
  shares it.
* **Publication.** A marked identity beside a `Pending` hold is published
  exactly when rule 84's witness carries it: a path from a producer to the
  exit that passes no mark of the identity and no refusal, the held call
  included. Otherwise it is withdrawn.
* **A withdrawal removes only the `Leak`.** The hold's `Pending` leaf and its
  reason stay, so the row folds to at least `Pending`. Rule 10i's standing
  analysis (W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL) then joins in every
  reason the function would carry with the held declarations in force, so a
  leak that accepting the declaration would leave makes the row `Unknown`
  (the `OTHER_HOLD` fixture). No `Pending` reason is invented, none is
  removed, and nothing becomes `Safe`.
* **Not decided: a mark beside no blocking leaf.** Rule 74c's held report
  still decides it, because withdrawing there could leave a row with no
  non-`Safe` leaf. It is measured as never held in the suite and absent from
  Fluent Bit, and the residual is registered (§6).
* **Moves possible.** `Pending -> Unsafe` or `Unknown -> Unsafe` when a
  witness exists in the order the schedule hid. `Unsafe -> Pending` or
  `Unknown` when a claim only an iterate made has no witness. Never
  anything `-> Safe`.

**Hooks in shared files** (each commented with the card name):

* `legacy_joined_leak_witness.rs` (rule 84's own module):
  * `candidates` reads `blocking_leaf`;
  * `marked_exits` is extracted from it;
  * `Step`, `restricted` and `witness` become `pub(super)`;
  * `step_output` and `claimable` are added for the census;
  * the module doc gains a paragraph.
* `legacy_transfer.rs`, the settle block after `deferred_leaks.settle`: one
  feature-gated census call, `mark_without_unknown_route::record(...)`, after
  `candidates`.
* `legacy_settled_leak.rs`: `Held::holds` (census only) and a doc line on
  `Held::settle`.
* `legacy_c.rs`: one `mod` line, after `joined_leak_route`.

**Lean.** `QCore.SettledLeakJoined`, last section; audit lines are in
`proof/AuditSettledLeakJoined.lean`.

* `blocking` and `decisionAt` define the widened side condition.
* `publication_is_order_independent_beside_a_hold`: two permutations of the
  same runs decide the same beside an `Unknown` or `Pending` leaf, or at an
  unmarked exit.
* `the_pending_case_read_the_schedule` and
  `both_orders_publish_beside_a_hold` show the old and new decisions on the
  fixture shape. `a_hold_on_both_arms_is_withdrawn` and
  `a_refused_only_leak_stays_withdrawn_beside_a_hold` cover withdrawal.
* `a_mark_beside_no_blocking_leaf_is_left_to_rule_74c` states the boundary.
* `settleLeaves` models the settle step, which only appends:
  * `a_pending_reason_is_preserved`;
  * `a_decision_beside_a_blocking_leaf_is_never_safe`;
  * `a_withdrawal_beside_holds_only_is_pending` (with `fold_of_pendings`).

The Rust `blocking` is bound to the Lean definition by
`tests/unit/legacy_mark_without_unknown.rs`. The new theorems depend on
`propext` and `Quot.sound` at most. There is no `sorry` and no project axiom.

## 3. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, with the prepared
inputs at `private/fb-046accf6` (read-only). The scripts are in
[scripts/](scripts/):
* `run.sh` is RETURN-FAMILY-UNTRACKED-MEMBER-CAST's, with this worktree and an
  optional `census` build;
* `opt-in-current.sh`, `totals.py`, `moved.py` and `changed_rows.py` are
  verbatim copies;
* `census_summary.py` is new.

Each run extracted its own `git archive` and built its own release
`quirin-audit` under the worktree's `private/smw/`, and went through
`scripts/gate_slot.py audit`.

| run | tree | binary sha256 | started / finished (UTC) | exit | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---:|---|
| base | `git archive 07833c86a` | `9a2b4a81f813…` | 16:59:07 / 17:48:54 | 0 | 1,205 / 2 / 3,590 / 7 |
| tip | `git archive 8e016d830` | `f5f9cf1eaaed…` | 18:08:38 / 18:58:03 | 0 | 1,205 / 2 / 3,590 / 7 |
| census base | `git archive 1a7f46c9e`, `indirect-census` | `822a93cd140a…` | 16:59:08 / 17:49:25 | 0 | 1,205 / 2 / 3,590 / 7, no census line |
| census tip | `git archive 8e016d830`, `indirect-census` | `79ebc924e03d…` | 18:08:37 / 18:58:33 | 0 | 1,205 / 2 / 3,590 / 7, no census line |

`8e016d830` is the last commit that changes the gateway's Rust source.
`c7b09ac14` moves a test file only. Both measured runs print the same
Safe-row dependency line.

**Moved rows: none.** [moved-rows.json](evidence/moved-rows.json) is
`{"moves": {}}`, and [changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)
finds no row whose status or leaf multiset differs. No Unknown -> Safe move
needs justifying.

**Against the prediction.** Every figure is at its predicted expected value,
0. The prediction allowed `Pending -> Unsafe` 0 to 2, `Unknown -> Unsafe`
0 to a few and `Unsafe -> Pending`/`Unknown` 0 to a few. The census explains
the zeros: no Fluent Bit analysis has an exit mark without an `Unknown` leaf.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host:
there are no `build/cycle19` inputs or baselines. That route was not run and
nothing is reported for it. For the integrator, a pinned row whose leak
rests on an iterate claim of an identity a hold marks, beside no `Unknown`
leaf, would move `Unsafe -> Pending`/`Unknown`. A hidden refusal-free arm
there would move `Pending`/`Unknown -> Unsafe`. Each is to be reported as
unpredicted.

**Gateway suite census at the tip**
([suite-census-tip.json](evidence/suite-census-tip.json)). There are 154
lines: the base's 130 plus the new fixtures' analyses. The
`decided_differently_beside_pending` entry is the fixture's one-arm leak
`(0, 4)`, witnessed and claimed in only one order. No held report sits
beside no blocking leaf.

## 4. Gate

Command: `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base
07833c86a --out <worktree>/private/gate`, run with Clang 21 and Lean 4.35.
Mode: full, because the diff reaches `qcore-c-gateway`. Head `8e016d830`, no
uncommitted scoped path. The record is [gate/gate.json](gate/gate.json), and
the logs are in the worktree's `private/gate/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` (24 packages) | 0 | 726 units, 771 summaries; 10,368 passed, **0 failed**, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `gate_lean.py` (`QCore.SettledLeakJoined`) | 0 | 1 audit |
| `python3 -m unittest discover -s scripts/tests` | **1** | 807 tests, 2 failures (below) |

The Python failures:

* **`test_gate_scope_census.test_the_census_covers_the_suite`: this card's.**
  The new test target took the unit count to 782, and 703 observed units is
  not more than 90% of that. `c7b09ac14` moves the fixtures into
  `settled_leak_other_refusal`'s existing target as a module. The test then
  passes (`python3 -m unittest scripts.tests.test_gate_scope_census`), and
  the moved fixtures pass (`cargo test -p qcore-c-gateway --test
  settled_leak_other_refusal`: 5 passed).
* **`test_build_matrix_sections` (`test_each_derived_dimension_matches_the_section_it_counts`,
  `test_the_derived_facts_are_recomputed_rather_than_carried`): a base
  failure.** It reports `dimensions.named_followups is 1131 against a
  named_followups of 1130`. This card does not touch the acceptance matrix,
  and the same two tests fail in a `git archive 07833c86a` tree. The matrix is
  regenerated at landing.

Other checks:

* `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings`: exit 0.
* The full gateway suite with `indirect-census` at the tip: 4,423 passed, 0
  failed. The `debug_assertions` replay check therefore held on every
  analysis that now has a candidate beside a `Pending` leaf.
* The LLP64 fixture scan over the changed test files: exit 0 ("0 counted, 0
  found"). The paths were passed as arguments, because the sandbox refused
  the `--stdin` pipe form.
* `apply_amendments.py --check --profiles-base 07833c86a`: exit 0.
* No provider file changed, so the provider gate was not selected.

## 5. Ledger and follow-ups

The follow-up **SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF** is retired in
[integration-amendments.json](integration-amendments.json). Beside a
`Pending` hold the marked identity is decided by rule 84's witness, with
every hold's reason kept and the row never `Safe`. It is measured on the
suite and on Fluent Bit. The case beside no blocking leaf is the residual
below.

Registered:

* **SETTLED-LEAK-MARK-LEAFLESS.** A current-generation leak whose settled
  exit marks the identity in a function with no non-`Safe` leaf at all (the
  mark comes from a modelled effect, not a refusal: rule 43's
  `ElementsReleased`) is still decided by rule 74c's held report, and so by
  the schedule. The measured instances are 9 suite lines, all exempt and
  never claimable, and none in Fluent Bit. Deciding it needs a choice
  fail-closed does not settle:
  * withdrawing an unwitnessed claim leaves the row `Safe` on a modelled
    "may have released" effect;
  * publishing it accuses through that effect;
  * adding an `Unknown` leaf invents a reason.

  Options:
  * (a) treat a contract's may-release mark as a discharge, so the row is
    `Safe` by the contract's own statement and needs a rule saying so;
  * (b) record an `Unknown` "mark without refusal" reason whenever such an
    identity is live at a settled exit;
  * (c) leave it as is until an instance is claimable.

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/legacy_joined_leak_witness.rs`: `candidates`
  and `marked_exits`, and the `pub(super)` on `Step`, `restricted` and
  `witness`.
* `crates/qcore-c-gateway/src/legacy_transfer.rs`: one census line after
  `let decided = joined_leak_route::candidates(...)`.
* `crates/qcore-c-gateway/src/legacy_settled_leak.rs`: `Held::holds`.
* `crates/qcore-c-gateway/src/legacy_c.rs`: the `mod
  mark_without_unknown_route` line after `joined_leak_route`.
* `crates/qcore-c-gateway/tests/settled_leak_other_refusal.rs`: the trailing
  `mod mark_without_unknown;`.
* Generated files (`docs/agents/gateway.md`, `docs/agents/README.md`): rerun
  `scripts/gen_rule_docs.py` on conflict.
