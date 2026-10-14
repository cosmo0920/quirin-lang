# OP-CONSTRUCTOR-DECLARATIONS: op constructors and their result states

Card OP-CONSTRUCTOR-DECLARATIONS (wave 48, round 26), gateway rules **12o**
and **13w** amended in place and **14e** added, contract specification
**C04.19.4**, grammar **G05** and **G05d**, Lean `QCore.OpCallbacks` (extended)
and `QCore.BodyClauseStanding`. Base: local `main` at `f3e1d724c` for the
census and the frozen [prediction](PREDICTION.md) (committed at the syntax
pause point, before any implementation code); rebased onto `2f0f65a60`
(CALLBACK-MEMBER-DATA-ONLY-UNITS) and then `a7e7e8cee`
(RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN) before the final measurement.

## 1. The user's decisions

The card paused after its census for the surface words (the prediction's
section 3). The user's decisions of 2026-10-14 are recorded verbatim in the
contract specification ([C04.19.4](../../QCORE_CONTRACT_SPEC_r0_3.md#c04194-a-constructors-result-state)):
**A1**, the four-operand `resource_discriminant_set(result, TAG.FIELD, P,
FLAGS)` (exact; the consumer applies its mask); **B1**,
`resource_callback_member_registered(result, TAG.FIELD, F)`; **C**, the
member-empty fact in this card as a `quant` state clause in maps-to notation
with two levels, a body check and Lean entailments; and the maps-to token
**`|->`** (`->` collides with C member access and the `.qr` return arrow;
`>->` reads as `↣`; `|->` is the ASCII `↦` of Z, Isabelle and TLA+). The
lower level of C's member-empty fact is written
`resource_callback_member_registered(result, TAG.FIELD, absent)`: B1's word
with C's reserved `absent` (the card's reading of "the member-empty fact",
named here so it can be renamed in one place if the user prefers another
spelling).

## 2. What the card built

| piece | where | what it does |
| --- | --- | --- |
| maps-to token | `qcore_frontend::lexer::MAPS_TO`, `Symbol::MapsTo` | one three-byte token, matched before every two-byte one (`\|>` stays the pipe, `\|\|` the or, `\|\|->` is `\|\|` then `->`); tests for each |
| loader | `contract_constructed_state.rs` | reads A1, B1 and C's two levels into one canonical `ConstructedState` (schema `qcore_schemas::constructed_state`); a bare `->`, `result->rko_op_cb`, an undotted member, any value but `absent`, a formal binder, a nullable result and two states of one member each reject the module with a named diagnostic |
| route G | `legacy_constructed_route.rs`, hooks in `legacy_callback_route.rs` | at a constructor call, the decided discriminant (`actual \| FLAGS`, the actual folded from Clang's call) and each member's state go on the tracked result's site, joined by agreement; a release reads the arms the reachable values select (rule 12o); a member invocation the unit's closure refuses reads the registration or `absent` (rule 13w); a registered candidate gets a derivation edge to its caller |
| stability | `constructed_state.rs` (`StateStore`, `collect`, `stability`) | every store into a stated member in every unit (a data-only unit read whole) is classified (section 4); a unit reads a state only where its own stores keep it and the calling function stores into the member nowhere |
| run fold | `callback_member_fold.rs` (`constructed_states`) | demotes a Safe or Pending row resting on a constructor whose stated member the run does not keep (`constructed-state-open`); a demoted function's null and bounds rows read `Unknown` custody, their combined verdict recomputed |
| body check | `constructed_state::result_state_validates` (evidence `14e`) | in the defining unit: the result zero-allocated (`calloc`, `rd_calloc`), never handed on, `absent` members never written, the discriminant stored once from its parameter; otherwise the state is withdrawn there |

## 3. Contracts

`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, written by
`scripts/write_contracts.py` (the token read from the lexer's constant),
every body read by hand at `66910c10` and `046accf6` (byte-identical,
`scripts/body.py`), pinned at both (`scripts/pin.sh`, logs
`private/opctor/pin-a2/`: every librdkafka check 0 at both, the whole tree 1
at both only for the libevent crate's tip-mode `mk_vhost_set`, by design;
`--verify-contracts` 0); `providers/contracts.py sync` and `check` 0.

* `rd_kafka_op_new0(source, type)`: the asserted-allocation premise,
  `resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0)`,
  `quant live #result: rd_kafka_op_s.rko_op_cb |-> absent;`, an owned
  non-null result. Rule 14e validates the state in `rdkafka_op.c` (standing
  `partly-validated`: the result state's clauses validated, the custody
  modes declared).
* `rd_kafka_op_new_cb(rk, type, cb)`: the premise, `rk` retained by the
  result (`rko->rko_rk = rk`), the discriminant with `RD_KAFKA_OP_CB =
  536870912`, `cb` registered in `rko_op_cb`; declared assumption.
* `rd_kafka_op_req0`, `_req`, `_req2`: the request op consumed, the queues
  borrowed, the pop's two member invocations over `supplied_owned` ops (as
  the declared `rd_kafka_q_pop_serve*` state them), an owned nullable reply
  whose type is the replier's: no discriminant stated.

Pinned tables: 401 → **406** resource-v1 declarations, 52 → **57** withheld
into the retaining table (`compiler/tests/contract_loader.rs`), the clause
counts in `compiler/tests/qcore_contract_corpus.rs`.

## 4. The discriminant's and the member's run-wide stability

Census ([prediction](PREDICTION.md) 1.3, 1.4) against what the run's
classified stores give ([section](artifacts/constructed-states-corpus.json)):
both `rd_kafka_op_s.rko_type` and `rd_kafka_op_s.rko_op_cb` are **kept** over
the whole run, nothing demoted.

* `rd_kafka_op_new0`'s `rko->rko_type = type` and every designator stored
  into `rko_op_cb` by the body that just made the op are stores into a
  constructor's own result; `rd_kafka_op_call`'s `rko->rko_op_cb = NULL` only
  removes a candidate; `rd_kafka_op_reply` and `_error_reply`'s
  `rko->rko_type |= (… ? RD_KAFKA_OP_CB : RD_KAFKA_OP_REPLY)` are constant
  flag-ors.
* **The `PARTITION_LEAVE` → `PARTITION_JOIN` rewrite**
  (`rd_kafka_broker_op_serve`, `rdkafka_broker.c:3631`) is a case-guarded
  rewrite: it sits in the `case RD_KAFKA_OP_PARTITION_LEAVE:` group of
  `switch (rko->rko_type)`, and the formal `rko` is rebound in the function
  only to `NULL`. A decided value reads as the set closed under it, so a
  decided 17 reads {17, 16}; both fall to `rd_kafka_op_destroy`'s
  `otherwise` (empty) arm, which a decided `PARTITION_LEAVE` therefore still
  selects exactly.
* Two refinements were measured necessary (build `a3`): the rewrite's later
  `rko = NULL` first read as a rebinding, and
  `rd_kafka_cgrp_propagate_commit_result`'s `rko_reply->rko_u.offset_commit =
  rko_orig->rko_u.offset_commit` first read as a whole-object write of the op.
  A null rebinding reaches no other object, and a sub-object write touches
  only the members under it; with both, the run keeps both members.

## 5. Measurement

BEFORE `bin/before3` (`1dc623e9…`, built from main `a7e7e8cee` checked out
in this worktree); AFTER `bin/a5` (`47492b59…`, from the rebased tip); every
route with `QCORE_NULL_PROPERTY` ([summary](artifacts/summary.json); each
`run.json` names its binary).

| route | BEFORE | AFTER | report sha256 | wall (s, AFTER) |
| --- | --- | --- | --- | ---: |
| corpus `66910c10` | 935 / 9 / 3,890 | **936 / 9 / 3,889** | `527a4cad…` → `ec4ccfeb…` | 523.6 (a4) |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `b2929d30…` → `4232495b…` (contract digests only) | 78.2 (a4) |
| slice | 52 / 1 / 347, 4 Pending | 52 / 1 / 347, 4 Pending | `5203a285…` → `4184eb2e…` (the same) | 40.2 (a4) |
| null, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | identical tallies; one row's custody column (below) | | |
| null, system / slice | 84 / 125 / 245; 71 / 103 / 232 | identical rows | | |

(`a4`, `d60926af…`, built before the second rebase, produced byte-identical
reports on all three routes; its wall times are the ones recorded.)

**The Safe gain, read by hand.** `rdunittest_fetcher.c ::
ut_create_fetch_op_with_offset` (Unknown → **Safe**): `rko =
rd_kafka_op_new(RD_KAFKA_OP_FETCH); rko->rko_u.fetch.rkm.rkm_rkmessage.offset
= offset; return rko;` — the op is the constructor's owned non-null result,
one scalar member is written, and the op is returned to the caller, which
owns it. The row rests on `rd_kafka_op_new0`'s declaration
(`resource_return_owned(nonnull)` under the `asserted_allocation` premise, a
declared assumption; its result state is validated by rule 14e). The
corpus Safe roll-up moves 24 → 25 rows resting on a partly validated
declaration. Its null row's `custody` and `combined` columns read Safe; its
null verdict is unchanged.

**Every moving row** ([corpus](artifacts/compare-corpus.json); system and
slice: none): 178 corpus rows, 177 Unknown before and after. They lose 116
`call to uncontracted function` leaves (`rd_kafka_op_new0` 87, `_new_cb` 14,
`_req` 2, `_req0` 1, `_req2` 6 and their summaries), 115 `unresolved
resource identity` leaves and one destroy refusal, and gain the next refusal
on the now-tracked op: `unresolved resource identity` 161, `indirect memory
access` 15 (actuals such as `rkcg->rkcg_ops` the access gate refuses, masked
before by the uncontracted call), `loop re-acquisition` 9 (`do { rko =
rd_kafka_op_new(…); … }`), `nullable resource used by a non-null contract` 7
(a possibly null identity handed to a non-null position), retention-undecided 7
(`rd_kafka_op_new_cb`'s `rk`, `OP-NEW-CB-RK-RETENTION`), and
`callback-member-open` at two request calls. No leak, violation or `Unsafe`
is added; diagnostic categories: leak 40 → 40.

**The decided states at work.** `rd_kafka_share_build_response_rko`
(`rdkafka_fetcher.c`) destroys the op it made with `rd_kafka_op_new` on an
error path: its destroy no longer refuses `callback-member-open` (the op's
`rko_op_cb` is `absent` on the path, so nothing is invoked); the row stays
Unknown on other leaves. `rd_kafka_op_destroy`'s leaves: 132 → 131 (118
`callback-member-open`, 13 `callback-uncontracted`). No corpus row carries an
arm refusal before or after (`arm-undecided`, `arm-undeclared`,
`arm-withdrawn`: 0 → 0): almost every destroy refuses at its member
invocation first, so a decided arm changes no corpus row; the fixture tests
exercise it (`tests/op_constructor_declarations.rs`).

**Prediction against outcome.** Predicted: no status move, no Safe gain,
leaves only, null unchanged. Measured: one Unknown → Safe (an owned result
returned, not a decided state), leaves as predicted, the null verdicts
unchanged. The prediction's expected in-body destroy decisions at the
`new_cb` sites did not materialise: those rows refuse earlier.

## 6. Lean

`proof/QCore/OpCallbacks.lean` (audited by `proof/AuditOpCallbacks.lean`, no
placeholder, no project axiom; the audit count stays **319**): the brief's
three theorems — **`a_decided_discriminant_selects_exactly_its_arm`**,
**`an_undecided_discriminant_gives_the_join`** /
**`an_unstable_discriminant_gives_the_join`**,
**`a_registration_then_destroy_invokes_only_the_registered_candidate`** —;
the decomposition entailment, derived from the parts' meanings over
`QCore.DerefDecomposition`'s pointer value:
**`the_shorthand_and_its_parts_mean_the_same`**,
`the_shorthand_is_live_and_absent`, `the_shorthand_entails_quant_live`,
**`both_levels_are_one_canonical_set`**;
**`an_absent_member_invokes_no_candidate`**;
**`an_unclassified_store_voids_the_absent_state`**; and
`a_decided_value_with_no_arm_is_unknown`, `a_rewrite_within_one_arm_keeps_it`,
`the_decided_join_refines_the_cautious_join`, `decided_join_vectors_hold`
(bound to `constructed_state.rs`'s `DECIDED_JOIN_VECTORS`).
`QCore.BodyClauseStanding` gains the clause kind `memberState` and the
evidence `r14e` (coverage and standing vectors bound by
`tests/contract_clause_standing.rs` and `contract_dependencies_report.rs`).

## 7. Ledger

`scripts/update_ledger.py` (idempotent): **`OP-CONSTRUCTOR-DECLARATIONS`
retired** (its DISCHARGE holds: the constructors and the request family
declared, the discriminant stated from the decided type actual with its
run-wide stability checked, each body read at both revisions, measured).
Registered: `OP-CONSTRUCTORS-UNDECLARED-RESIDUAL` (`rd_kafka_op_new_reply`
2/1, `_new_fetch_msg` 1/1, `_new_ctrl_msg` 1/1), `OP-NEW-CB-RK-RETENTION`
(7 retention-undecided leaves), `OP-STATE-FOLD-PER-ROW-READS` (the fold's
conservative demotion). `dimensions.named_followups` 769 → **771**.

## 8. Gate

GATE

## Files

* Lexer and loader: `crates/qcore-frontend/src/lexer.rs` (+ unit tests),
  `crates/qcore-contract/src/{contract_constructed_state,contract_loader}.rs`,
  `crates/qcore-contract/tests/op_constructor_clause.rs`; schema
  `crates/qcore-schemas/src/{constructed_state,retained_by}.rs`.
* Route G: `crates/qcore-c-gateway/src/{constructed_state,legacy_constructed_route}.rs`,
  hooks in `legacy_callback_route.rs` (operands, `apply`, `apply_arms`,
  `settle`, `callback_edges`), `legacy_retained_by.rs`, `legacy_transfer.rs`
  (the state map and its join), `typed_access_path.rs` (the per-unit cache),
  `contract_dependencies.rs`, `contract_clause_standing.rs`;
  `tests/op_constructor_declarations.rs`, fixture
  `tests/fixtures/contracts/op_constructors.qv`.
* Run fold: `crates/qcore-audit/src/{callback_member_fold,projection,report}.rs`.
* Lean: `proof/QCore/OpCallbacks.lean`, `proof/AuditOpCallbacks.lean`,
  `proof/QCore/BodyClauseStanding.lean`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`; pins
  `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* Records: `docs/agents/gateway.md` (12o, 13w, 14e), `docs/agents/README.md`,
  `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.2, C04.19.4),
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (G05, G05d), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part7.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
