# CALLBACK-TABLE-MEMBER-PATH-STATES: a table dispatch composes its candidates' member-path states

* **Base:** main `43410b230`. **Card:** the registered follow-up of
  CALLBACK-MEMBER-RELEASED-EFFECT-ORDER. USER DECISION 2026-11-25: "lanich
  four of queued cards", with CALLBACK-TABLE-MEMBER-PATH chosen as the fourth.
* **Commits:** probe fixture `3836b3a40` (fails at base),
  [PREDICTION](PREDICTION.md) `75bd8b10d`, the semantic commit `e60f4522c`
  (the measured tip), the shipped-clause test without a tree walk plus
  records `cdb2db6af` (the gated tree; the change is test-only), and this
  gate section.

## 1. Finding at base

`legacy_callback_route::callback_operands` collected the candidates'
member-path states (`member_path_route::operands`) in the
`CallbackClause::Invoked` branch, but not in the `CallbackClause::Table`
branch. Meanwhile `classified_in` reads a candidate that states member paths
as its custody contract, in either form. So at a table dispatch a candidate's
`#released` payload read as kept, and the state was dropped. The fixture
reproduces the probe over a dispatcher that borrows the op (`c_op_table`),
with the synchronous form (`c_op_call`) over the same body as its control
([`evidence/probe-base.log`](evidence/probe-base.log): exit 101, 2 passed and
4 failed):

| Body (item linked at `o->payload`) | synchronous at base | table at base | table at tip |
|---|---|---|---|
| `c_cb_releases`, then `o->payload = NULL; c_item_free(it); c_op_drop(o)` | `Unsafe` | **`Safe`** (a double release certified) | `Unsafe` |
| `c_cb_releases`, then `c_op_drop(o)` | `Safe` | `Unsafe` (the item reads leaked) | `Safe` |
| `c_cb_stale`, then free | refused `member-path-stale` | **`Safe`** | refused `member-path-stale` |
| `c_cb_maybe` (`{#moved, #kept}`, no arm), then free | refused `member-path-undecided` | `Safe` | refused `member-path-undecided` |

## 2. Change

**Rust (one hook, `legacy_callback_route.rs`, the `CallbackClause::Table`
branch of `callback_operands`).** After the candidates are decided and their
requirements collected, the branch extends `operands.member_paths` with the
same `member_path_route::operands(self.contracts, candidates, actuals,
actual, …)` call as the synchronous branch. `member_path_route::apply` then
joins the candidates' compositions at each linked path before the arms, as
it does for the synchronous form. No new module was needed: the composition,
the join and every refusal are the existing ones.

**Why the synchronous join is sound for a table.** A table callee may call
several different entries, in any order, any number of times. A synchronous
invocation calls one candidate. Rule 30's join decides `gone` only when
every candidate's every reading is `#released` or `#moved` (both clear the
member) or `#kept` under an arm that always releases. So after the first
invocation that takes the object, later entries find the member cleared and
take nothing. It decides `stays` only when every reading is `#kept` under no
arm. Candidates that disagree (`gone` against `stays`) refuse
`member-path-undecided`. A stale reading always refuses (stale or double
release). The Lean model proves this over every sequence of readings (§3).
No refusal is needed beyond rule 30's: each case where composition over
several candidates cannot be stated soundly already refuses. No stated state
is dropped.

**The shared premise, named.** `gone` under no arm reads the invocation as
having run. The clause itself says "zero or more times", and with no
invocation no object leaves (`the_gone_reading_assumes_an_invocation`). C04.19.6's
table, on the user's decision of 2026-10-15, reads this alike at every
invocation form (synchronous, member, now table). This card adds no reading
beyond it, and does not prove it. It is registered as follow-up
MEMBER-PATH-INVOCATION-PREMISE (§6).

**Rules.** [Rule 13v](../../../../agents/rules/13v.md) is amended (text and
summary), and so is [rule 74p](../../../../agents/rules/74p.md): its sentence
"A table dispatch composes no candidate state at all" now says the table
form composes through the same `apply` after the release, so 74p's argument
covers it. No new rule id was taken. C04.19.6 gains an "At a table dispatch"
paragraph. The generated `docs/agents/gateway.md`, `docs/agents/README.md`
and `proof/QCore.lean` were regenerated.

## 3. Lean

`QCore.CallbackTableMemberPathStates`, with
`proof/AuditCallbackTableMemberPathStates.lean`: 17 audited theorems, using
only the axioms `propext` and `Quot.sound`, or none. It has no `sorry` and no
project axiom (build and audit: `private/logs/lean-2.log`, exits 0 and 0).
It imports `QCore.MemberPathRelease` (rule 30's `compose`, `join`, `joinAll`)
and `QCore.CallbackReleasedEffectOrder` (the r70 effect-order model).

* `tableOutcome`: the join over the candidates, which is the synchronous
  form's composition. `step` and `run`: one path's concrete effect over a
  sequence of invocations and then the arm (`run_one_is_count` ties it to
  `MemberPathRelease.count`).
* `the_join_is_at_least_each_candidate`: the join is each candidate's
  composition, or a refusal. `a_decided_join_is_every_candidate` holds for
  `gone` and for `stays`.
* `a_gone_table_leaves_exactly_once`: for every non-empty sequence of the
  candidates' readings, in any order, and every arm the join allows, the
  object leaves the caller's custody exactly once.
  `a_stays_table_never_leaves`: for every sequence, the empty one included,
  it never leaves.
* `disagreeing_candidates_refuse`, `the_gone_reading_assumes_an_invocation`
  (the premise), and `a_table_composition_after_a_release_is_the_c_order`
  (74p's `late_is_the_c_order`, which covers the table form).
* `table_vectors_hold`: the fixture's rows, plus `run [stale, stale] = 2`,
  the double take a stale reading would cause.

## 4. Pins

`crates/qcore-c-gateway/tests/callback_table_member_path_states.rs` is a new
target with 6 tests. It uses rule 30's `member_paths.qv` plus a synthetic
module (`c_op_table`, `c_op_call`, `c_op_drop`, `c_item_free`, and
`c_cb_plain`, a contracted identity). Each row is read in both forms over
the same body.

* `a_table_candidate_releasing_the_linked_payload_then_a_free_is_never_safe`
  (the probe): `Unsafe` in both forms.
* `a_table_candidate_releasing_the_linked_payload_takes_it`: `Safe` in both
  forms.
* `table_candidates_join_and_disagreeing_ones_refuse`: `{releases, moves}`
  takes the payload (a later free is not `Safe`). `{releases, other}`,
  `{other, releases}` and `{plain, releases}` refuse `member-path-undecided`
  after either tail.
* `rule_30_refusals_read_at_a_table_dispatch`: stale and may-set, in both
  forms.
* `a_table_candidate_stating_nothing_at_the_path_keeps_the_payload` (the
  controls): `c_cb_plain`, `c_cb_other` and both together keep the payload.
  The caller's free is `Safe`, and dropping the op without it is `Unsafe`.
  These read the same as at base (this test passed at base).
* `the_shipped_table_clauses_are_the_map_unpackers`: cmetrics
  `mpack_utils.qv` and ctraces `context_lifetime.qv` (`include_str!`) each
  load with exactly one table declaration, `cmt_mpack_unpack_map` and
  `ctr_mpack_unpack_map`, and neither module states a member path. The
  census showing these are the only shipped `.qv` files naming
  `resource_callback_table`, and that neither provider states a member path
  anywhere, is [`scripts/census.sh`](scripts/census.sh), with its output in
  [`evidence/shipped-table-census.txt`](evidence/shipped-table-census.txt).
  The first version of this test walked `providers/` at run time. The Python
  suite refuses an unreviewed tree walk (`test_tracked_files`,
  `test_gate_scope_census`; gate 1, §6), so the walk moved into the census
  script.

At the tip the target passes: 6 passed (`private/logs/fixture-1.log`,
`fixture-2.log` after the test rewrite).

## 5. Measurement against the prediction

The opt-in run is `quirin-audit current --monkey-linked` over Fluent Bit
046accf6 (`private/fb-046accf6`, read-only), driven by
[`scripts/run.sh`](scripts/run.sh). These are
CALLBACK-MEMBER-RELEASED-EFFECT-ORDER's scripts with this worktree's paths.
Each side is a `git archive` tree with its own release binary and its own
`package.py checkers`, run in a `gate_slot.py audit` slot. Outputs are in
`private/ctmp/run-{base,tip}` and are not committed. The two runs did not
overlap each other. The base run's peak resident set (10.3 GB) exceeded the
slot's 7 GiB admission budget; the slot recorded this, and the run exited 0.

| Run | Tree | Binary sha256 | Exit (build / checkers / run) | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---|
| base (00:40-01:41Z) | `43410b230` | `3534e966...7a2ff70d` | 0 / 0 / 0 | 1187 / 2 / 3607 / 8 |
| tip (02:17-03:10Z) | `e60f4522c` | `a846a116...e0e2868` | 0 / 0 / 0 | 1187 / 2 / 3607 / 8 |

The [totals](evidence/totals-base.json) ([tip](evidence/totals-tip.json))
are equal, and so are the indirect-access figures (2105 rows, 215 alone,
8917 leaves). [`moved-rows-base-tip.jsonl`](evidence/moved-rows-base-tip.jsonl)
is `{"moves": {}}`, and [`changed-rows-base-tip.txt`](evidence/changed-rows-base-tip.txt)
is empty. **Controls:** the 42 rows naming `cmt_mpack_unpack_map` or
`ctr_mpack_unpack_map` are 38 `Unknown` and 4 `Safe` on both sides.

**Against the prediction:** 0 status moves, 0 content changes and equal
totals, as predicted. No row moved, so there is no Unknown→Safe move to
justify.

**Pinned routes:** not run. This host has no `build/cycle19` inputs or
baselines, so `quirin-audit measure --declared` cannot run here, and no
result was fabricated. The prediction's `measure-prediction` block (none on
corpus, system and slice) is for the integrator.

## 6. Gate, conflicts and follow-ups

**Record:** [`gate/gate-2.json`](gate/gate-2.json). **Command:** `sh
private/toolchain/env.sh python3 scripts/gate_scope.py --base 43410b230
--out private/gate-2` (Clang 21, Lean 4.35). The log is
`private/logs/gate-2.log`. The tree was `cdb2db6af`, with this section and
the gate record uncommitted (docs only). Scoped run, Lean all, no provider
gate (`providers/` unchanged). **Exit 1, from the six known base failures
alone.**

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | `third_party/libevent` checked out |
| Rust suite (scoped) | 101 | 697 units; 10109 passed, **6 failed**, 24 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 509 audits |
| Python suite | 0 | 807 tests |

The 6 Rust failures are exactly the K5-LOCAL-ORIGIN-ALIASES (rule 77) pins
that fail at base and are being re-pinned by a separate card:
`scalar_read_identity::pointer_destination_retains_opaque_identity_refusal`,
`detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor`,
`access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch`,
`typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read`,
`lexical_closure_nested_assignment::a_never_bound_root_still_certifies`
and `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local`.
None of them was edited.

**Gate 1** ([`gate/gate-1.json`](gate/gate-1.json), `private/gate-1`, tree
`e60f4522c`) also exited 1. Its Rust suite had the same 6 base failures
(10109 passed). It also had 3 Python failures. Two of them,
`test_gate_scope_census.test_every_directory_literal_is_declared` and
`test_tracked_files.test_every_remaining_tree_walk_in_the_tests_is_a_reviewed_one`,
came from the shipped-clause test's walk of `providers/`. That walk was
removed in `cdb2db6af` (§4). The third,
`test_gate_slot_lane.test_a_short_job_overtakes_queued_audits_and_a_draining_suite`
(`short/1` against `short/0`), ran while other worktrees held audit slots.
It passed when rerun alone, as did the other two modules after the fix, and
the whole suite passes in gate 2. Lean passed with 509 audits.

**LLP64 fixture scan:** `llp64_fixture_scan.py --stdin` over the changed
test paths exits 0. The fixture uses no size type and no enum.

**Files likely to conflict, with the hook points:**
`crates/qcore-c-gateway/src/legacy_callback_route.rs` (13 lines inside the
`CallbackClause::Table` arm of `callback_operands`, after
`self.require(...)`); `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.6, after
the "Through a released object" paragraph); `docs/agents/rules/13v.md`
(appended amendment) and `docs/agents/rules/74p.md` (one sentence). The
generated `docs/agents/gateway.md`, `docs/agents/README.md` and
`proof/QCore.lean` are resolved by rerunning their generators.

**Follow-ups.** This card retires CALLBACK-TABLE-MEMBER-PATH-STATES and
registers MEMBER-PATH-INVOCATION-PREMISE: rule 30 reads a `#released` or
`#moved` candidate state under no arm as `gone` at every invocation form,
although the clauses say the callback is called zero or more times. If the
callee calls no candidate, the object stays live while the caller reads it
released, so a leak is missed and a later free reads as a double release.
The premise is named in Lean and not proved. Discharge: a stated premise
per invoking declaration, or a user decision to refuse `gone` under no arm
where the invocation may not run.
