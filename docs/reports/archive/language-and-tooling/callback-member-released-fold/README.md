# CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD: a candidate's requirement at every position

* **Base:** main `2c60986b0`. **Card:** the registered follow-up of
  OPENSSL-RELEASED-PARAMETER-SLOTS. USER DECISION 2026-11-23: picked in the
  "soundness first" set of four cards.
* **Commits:** [PREDICTION](PREDICTION.md), census and base fixture log
  `92fa566ab` (before any implementation); implementation, Lean, rule and
  spec `9180fc26f` (the measured tip); measurement scripts `30bb37c11` (the
  gated tree); this report.

## 1. Census

**Clause forms.** `legacy_callback_route::callback_operands` puts a
candidate's `resource_owned(n)` requirement on a parameter actual of the call
(`require`) for four forms. The loader (`contract_callback.rs`) restricts no
actual's custody mode, so any mode can carry a requirement.

| Form | Requirement on the call | Before | Now |
|---|---|---|---|
| `resource_callback_invoked` / `_or_default` (13u) | yes | folded at borrowed only | folded (fold) |
| `resource_callback_table` (13v) | yes | folded at borrowed only | folded (fold) |
| `resource_callback_member_invoked` (12n) | yes; object may be borrowed, released or owned(nonnull) | folded at borrowed only | folded (fold) |
| `resource_callback_slot_invoked` (12n) | yes; object may be borrowed or released (r60) | borrowed, and a released slot object | folded (fold, r60's rule made general) |
| `resource_callback_registry_invoked` (13w) | none: any requirement refuses (`callback-effectful`) | refuse | unchanged |
| member reading deferred to a constructed state (14e) | none: any requirement refuses (`callback-effectful`) | refuse | unchanged |
| registrations, element add/drop, unregister, released-if-constructed, member/slot registration | none (no invocation at the call) | n/a | n/a |

**Positions.** Here "fold" means the call's own mode at the position, given a
requirement `n` (`callback_contracts::fold_requirement`):

| Mode | Fold | Why |
|---|---|---|
| borrowed | `owned(n)` | rule 13u, unchanged |
| owned(r), consumed(r), released(r), stored(r) | same mode, `r && n` | each already requires the object held, or null where nullable, before the call's effects. Only the nullability is lost if the requirement is dropped |
| out, removed | **refuse** (`callback-requirement-unfolded`) | an out position names a destination and a removed one an element its owner holds. No mode states "held and live" there |
| no such parameter | refuse | fail-closed (unreachable: the loader checks indices) |

I chose to fold rather than refuse wherever a mode can state the requirement,
because that matches r60's slot fold (`QCore.CallbackRequirementFold.the_fold_agrees_with_the_slot_fold`).
Both choices are fail-closed. The fold only strengthens a position
(`the_fold_only_strengthens`), so it cannot flip `Unknown` to `Safe`.

**Shipped declarations** ([`evidence/census.py`](evidence/census.py),
[`census.json`](evidence/census.json)): 624 invocation clauses in 49
declarations (562 slot, 45 member, 13 invoked, 2 `_or_default`, 2 table).
Four parameter actuals are not borrowed. Three are non-null:
`rd_kafka_op_destroy`'s `rko` (released), `rd_kafka_background_queue_serve`'s
`rko` (consumed) and `rd_kafka_poll_cb`'s `rko` (owned). The fourth is
`EVP_MD_CTX_free`'s `ctx` (released nullable), which is r60's slot object and
already folded. The member objects that are not borrowed are
`mk_thread_destroy`'s `th` and `rd_kafka_op_destroy`'s `rko` (both released
nonnull), `rd_kafka_poll_cb`'s `rko` (owned nonnull), and the slot objects of
`EVP_MD_CTX_free` and `PKCS12_free`. `mk_thread_destroy`'s candidate is
handed `_` (the object it supplies), so it puts no requirement on the call.
No shipped requirement reaches a position whose mode the fold changes, and
none reaches out, stored or removed.

## 2. Changes

* `crates/qcore-c-gateway/src/callback_contracts.rs`: adds
  `fold_requirement(mode, nullable) -> Option<mode>`, the reason
  `REQUIREMENT_UNFOLDED = "callback-requirement-unfolded"`, and the unit test
  `the_requirement_fold_covers_every_mode`.
* `crates/qcore-c-gateway/src/legacy_retained_by.rs`: after r60's
  `callback-beside-release` check, a requirement whose position does not
  fold refuses the call. After classification, every requirement is folded
  through `fold_requirement`. This replaces the borrowed-only and
  released-slot-only `match`. `released_slot_objects` is kept for r60's
  refusal.
* [Rule 74j](../../../../agents/rules/74j.md) (new; generated
  `docs/agents/gateway.md` and `docs/agents/README.md`). Contract
  specification C04.19 item 2 gains one sentence for non-borrowed positions.
* Lean `QCore.CallbackRequirementFold` with `proof/AuditCallbackRequirementFold.lean`
  (11 theorems; axioms at most `propext`; no `sorry`, no project axiom).
  Generated `proof/QCore.lean`. The module imports r60's
  `QCore.OpensslReleasedSlots` and proves its `folded` for one requirement
  equal to this fold (`the_slot_fold_is_one_requirement`). It also proves
  `the_old_fold_dropped_a_non_null_requirement`,
  `the_old_fold_differed_by_form`, `the_fold_never_drops_a_non_null_requirement`,
  `the_fold_agrees_where_the_old_one_folded`, `the_fold_only_strengthens`,
  `a_non_null_position_is_a_fixed_point`,
  `a_nullable_requirement_changes_no_held_position`, `out_and_removed_refuse`
  and `fold_vectors_hold` (the Rust unit test's rows).
* No provider contract changes, so no lock changes.

## 3. Pins

`crates/qcore-c-gateway/tests/callback_requirement_fold.rs` (new target, 6
tests) uses a synthetic `mobj` module and a run whose second unit stores the
candidate into `mobj.on_free`:

* `a_member_invocation_through_a_nullable_release_folds_the_requirement` (the
  registered defect): `mobj_free` has `resource_released(o, nullable)` and
  `resource_callback_member_invoked(o, mobj.on_free, o)`. With `on_ok`
  (borrowed), both the checked and unchecked frees are `Safe`. With
  `on_live` (`resource_owned(o, nonnull)`), the checked free is `Safe` and
  the unchecked one is not.
* `an_owned_nullable_member_actual_folds_the_requirement`: a member actual at
  `resource_owned(o, nullable)` behaves the same way.
* `a_synchronous_invocation_folds_at_owned_and_consumed_positions`: the
  13u form at owned-nullable and consumed-nullable positions behaves the
  same way.
* `a_requirement_at_an_out_position_refuses`: an `on_live` candidate handed
  a `resource_out` actual refuses with `callback-requirement-unfolded`. With
  `on_ok`, the call is `Safe`.
* Controls: `the_shipped_non_null_release_shapes_are_unchanged`
  (`rd_kafka_op_destroy`'s shape, a non-null release handed to the
  candidate, and `mk_thread_destroy`'s, a non-null release with `_`: checked
  `Safe`, unchecked refused, with either candidate, as at base) and
  `the_shipped_member_releases_are_non_null` (loads the shipped
  `librdkafka/client.qv` and `monkey/intrusive_list.qv`: the member
  invocations through a released object are exactly `mk_thread_destroy` and
  `rd_kafka_op_destroy`, both `Released { nullable: false }`, and the fold is
  the identity at each requirement-bearing actual).

**At base** the same fixture, built against the base sources and without
the test that names the new function, gave these results:
[`evidence/base-fixture.log`](evidence/base-fixture.log) (first four tests)
and [`evidence/base-fixture-2.log`](evidence/base-fixture-2.log) (with the
out test). `free_unchecked`, `visit_unchecked`, `each_unchecked` and `make`
were **`Safe`** with `on_live`: the requirement was dropped. The control
passed. The r60 target `openssl_undeclared_handshake_callbacks` (33 tests)
passes unchanged at the tip.

## 4. Measurement against the prediction

The opt-in run was `quirin-audit current --monkey-linked` over Fluent Bit
046accf6 (`private/fb-046accf6`), driven by [`scripts/run.sh`](scripts/run.sh).
It uses CAST-BEFORE-AMPERSAND-READ-AS-BINARY's scripts with this worktree's
paths: each side is a `git archive` tree with its own release binary and its
own `package.py checkers`. Outputs are in `private/cmr/run-{base,tip}` (not
committed). The two runs overlapped each other and the gate, so their wall
times are not comparable.

| Run | Tree | Binary sha256 | Exit (build / checkers / run) | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---|
| base | `2c60986b0` | `81618f42…fa32625` | 0 / 0 / 0 | 1172 / 2 / 3622 / 8 |
| tip | `9180fc26f` | `79b483c5…0ccf0f` | 0 / 0 / 0 | 1172 / 2 / 3622 / 8 |

[Totals](evidence/totals-base.json) ([tip](evidence/totals-tip.json)) are
equal, and so are the indirect-access figures (2118 rows, 215 alone, 8992
leaves). [`moved-rows-base-tip.jsonl`](evidence/moved-rows-base-tip.jsonl)
is `{"moves": {}}`. [`changed-rows-base-tip.txt`](evidence/changed-rows-base-tip.txt)
is empty, so no row changes status or leaf multiset. The console dependency
lines are identical. **As predicted: 0 status moves, 0 content changes.**

**Pinned routes:** not run. This host has no `build/cycle19` inputs or
baselines, so `quirin-audit measure --declared` cannot run here. I did not
fabricate a result. The prediction's `measure-prediction` block (none on
corpus, system and slice) is left for the integrator.

## 5. Gate

**Record:** [`gate/gate-1.json`](gate/gate-1.json). **Command:** `sh
private/toolchain/env.sh python3 scripts/gate_scope.py --base 2c60986b0
--out private/gate-1` (Clang 21, Lean 4.35), log `private/logs/gate-1.log`.
Tree `30bb37c11`. Scoped run, Lean all, no provider gate (`providers/`
unchanged). **Exit 0, 8 steps passed.**

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt`, `clippy` | 0, 0, 0 | `third_party/libevent` checked out |
| Rust suite (scoped) | 0 | 681 units, 726 summaries; 9978 passed, 0 failed, 22 ignored, 148 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 492 audits |
| Python suite | 0 | 807 tests |

Outside the gate (`private/logs/`): the base fixture runs
(`base-fixture-1.log`, `base-fixture-2.log`, exit 101 as expected), the tip
fixture and r60 target (`tip-fixture-1.log`, `tip-fixture-2.log`), the unit
tests (`tip-lib-1.log`), `lake build` (`lean-1.log`) and the audit's axioms
(`lean-audit-1.log`).

## 6. Conflicts and follow-ups

**Files likely to conflict:** `crates/qcore-c-gateway/src/legacy_retained_by.rs`
(the callback block after r60's `callback-beside-release` check, and the fold
in the classification `match`), `crates/qcore-c-gateway/src/callback_contracts.rs`
(the reason constants and the test module), and
`docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19 item 2). Generated files, to be
resolved by rerunning their generators: `docs/agents/gateway.md`,
`docs/agents/README.md` and `proof/QCore.lean`.

**Follow-up registered:** CALLBACK-MEMBER-RELEASED-EFFECT-ORDER. r60's
refusal of a candidate member effect beside a release (member-path state,
deferred member reading, registry use: `callback-beside-release`) is keyed on
`released_slot_objects`, so it covers slot invocations only. For a plain
member invocation through a released object (`rd_kafka_op_destroy`), the
callback route still applies such effects after the custody contract's
release, as it did before r60. This card did not show that ordering to be
unsound; it is outside this card's requirement fold.

**Increment paragraph** (in [`integration-amendments.json`](integration-amendments.json)):
it retires CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD and registers the
follow-up above. It has no `status_edits`: no headline figure moves, and the
Lean audit count is the landing gate's.
