# CG22-OP-CALLBACKS-AND-SERVED-QUEUES: op callbacks and served queues

Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (wave 48, round 25), gateway rules
**12m–12p**, contract specification **C04.19.1–C04.19.3**, grammar **G05**,
Lean `QCore.OpCallbacks`. Base: local `main` at `b565f79ab` for the census and
the frozen [prediction](PREDICTION.md) (committed at the syntax pause point,
before any implementation code); rebased onto `98eddb50e` before writing code
and onto `579c2e6e2` before the final measurement.

## 1. The user's decision

> **USER DECISION 2026-10-14.**
> 1. **Scope:** "Clauses + follow-ups". Implement the three clauses with Lean
>    and measurements. Register every blocker as its own follow-up with a
>    DISCHARGE: application rk_conf callbacks via `rd_kafka_poll_cb` (tie it to
>    the pending destroy_app decision; do not decide it), the 29+3 op-callback
>    candidate contracts, the undeclared op constructors, and the
>    forwarded-formal forwarders. No forwarded-formal rule in this card.
> 2. **Clause 1 word:** `supplied_owned`, not `handed`. Example:
>    `resource_callback_invoked(callback, _, _, supplied_owned, cb_type, opaque)`.
>    A callback that takes the object only on some return codes is refused, as
>    proposed.
> 3. **Clause 2 form:** per-arm `when` / `otherwise`, as proposed, with
>    `resource_discriminant`, `resource_arm_released` and `resource_arm_empty`;
>    decimal constants citing the source.
> 4. **Clause 3 spelling:** dotted `TAG.FIELD`. Example:
>    `resource_callback_member_invoked(rko, rd_kafka_op_s.rko_op_cb, _, _, rko)`.
>    The closure semantics stay as proposed, with premise
>    `P-CG22-MEMBER-CLOSED-WORLD`.
>
> Keep C04.18 unchanged.

## 2. What the card built

| clause | spelling | route G | evidence |
| --- | --- | --- | --- |
| an object the callee hands its callback | `supplied_owned` in the actual list of `resource_callback_invoked` / `_member_invoked` | rule 12m: every candidate must take it (`resource_consumed` / `resource_released`) unconditionally, else `callback-supplied-kept`; a relational candidate keeps it | declared assumption (rule 13y still checks a synchronous invocation) |
| a registration carried by the object | `resource_callback_member_invoked(OBJ, TAG.FIELD, A…)` | rule 12n: the unit's closed set of stores, then the run's fold keeps a row only when the run's set is contained in the unit's | declared assumption; premise `P-CG22-MEMBER-CLOSED-WORLD` |
| a release chosen by a discriminant | `requires resource_discriminant(P, FIELD, MASK)`, `resource_arm_released(P, PATH)` / `resource_arm_empty(P)` with `when FIELD == VALUE` / `otherwise` | rule 12o: the cautious join over the arms (no `otherwise`: `arm-undeclared`; a link some arms release: `arm-undecided`) | evidence `12o`: the switch's case groups hand exactly the declared members to calls |

**The member closure (rule 12n)** is computed from Clang's AST, read once per
unit beside rule 11p's tables (`callback_member_closure::MemberFacts`): every
store into a function-pointer member with what it stores (a designator, a null
pointer, another member, a formal of the enclosing function, or open), every
function-pointer actual of a direct call, every function whose address may
escape, the internal functions, and every record an initializer list, a
whole-object assignment or a `memcpy`-family call may write. `closure` is the
least set: a formal of a function-pointer type stands for its callers'
actuals where the function's address is never taken. A unit decides over its
own facts; `qcore-audit`'s fold (`callback_member_fold.rs`, on the pinned,
current and no-build routes) reads every analysed unit's facts and demotes a
`Safe` or `Pending` row resting on a member-invoking declaration unless the
run's set is closed and contained in the unit's; a unit row with no facts
opens every member; a checkpoint composer, which carries no facts, refuses a
run whose contracts invoke a member. The fold never promotes.

**The arm check (rule 12o)** reads the defining unit's Clang body:
one `switch` reading `P->FIELD` (with `MASK`, `P->FIELD & E`, `E` folding to
`~MASK`); case groups with stacked labels and fall-through, enumerator labels
folded from the unit's enumerations (Clang's JSON gives a case label's
`ConstantExpr` no value; `MemberFacts` records each enumerator); each value's
payload paths, the member paths of `P` under the arms' union handed to a
call (through `&`, casts and a subscript's base).

**The declared-ABI reader** (`legacy_declared_abi.rs`) gains one shape: a
function-type typedef (`typedef R (T)(…)`, or `typedef R T(…)`, read through a
trailing GNU attribute such as `RD_WARN_UNUSED_RESULT`) under exactly one
pointer is the `fnptr` the pointer typedef and the inline declarator already
read (`rd_kafka_q_serve_cb_t *callback`). Before it, the six serve
declarations were withdrawn at every call (`abi-unreadable-declaration`,
measured on `a3`); the type itself and a deeper pointer stay unread.

## 3. Contracts

`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, written by
`scripts/write_contracts.py` between its markers, every body read at
`66910c10` and `046accf6` (byte-identical, `scripts/body.py`), pinned at both
(`scripts/pin.sh`, logs `private/cg22op/pin-a1/`, re-checked with the final
binary in `private/cg22op/pin-a4/`: every librdkafka check 0 at both, the
whole tree 1 at both only for the libevent crate's tip-mode `mk_vhost_set`, by
design; `--verify-contracts` 0); `providers/contracts.py sync` and `check` 0.

* `rd_kafka_q_serve0`, `_serve`, `_serve_maybe_consume`, `rd_kafka_q_pop_serve0`,
  `_pop_serve`, `_pop_serve_maybe_consume`: every formal borrowed;
  `resource_callback_invoked(callback, _, _, supplied_owned, cb_type, opaque)`;
  the op's `rd_kafka_op_s.rko_serve` (which `rd_kafka_op_handle` substitutes
  for the caller's callback) taking it the same way; the op's `rko_op_cb`
  borrowing it (`_`: a handled op is destroyed by the callee); the pop
  variants' `PASS` op an owned nullable result. `cb_type` is `u32`
  (`rd_kafka_q_cb_type_t`, no negative value), `rd_ts_t` `i64`, `rd_bool_t` `u8`.
* `rd_kafka_op_destroy`: `resource_released(rko, nonnull)`; the op's
  `rko_op_cb` over `(_, _, rko)`; the discriminant `rko_type` under
  `RD_KAFKA_OP_FLAGMASK = 1610612736`; **169 arm releases over 60 values**
  and `otherwise` empty, generated from Clang's reading of the body
  (`scripts/op_destroy_arms.py`, [arms](artifacts/op-destroy-arms.json)), each
  value commented with its enumerator and `rdkafka_op.h` line.

Pinned tables: 394 → **401** resource-v1 declarations, 45 → **52** withheld
into the retaining table (`compiler/tests/contract_loader.rs`), the
per-declaration clause counts in `compiler/tests/qcore_contract_corpus.rs`.

**Not declared**, by the decision: `rd_kafka_poll_cb`, `rd_kafka_consume_cb`
and `rd_kafka_background_queue_serve` (each calls a callback the application
registered in `rk`, the pending `rd_kafka_destroy_app` class);
`rd_kafka_op_call` (its callback's `KEEP` takes the op, which no clause here
states); `rd_kafka_q_pop` (`REFCOUNT-Q-POP-OWNED-OP` stands). **C04.18 is
unchanged**: `rd_kafka_q_wait_result` reaches `rko_serve`, whose set holds
`rd_kafka_poll_cb`; `rd_kafka_q_purge0` reaches `rko_op_cb`'s 27 uncontracted
candidates (and the arms' further carried callbacks).

## 4. Census and the prediction

The census and design are [PREDICTION.md](PREDICTION.md) (BEFORE on
`b565f79ab`: corpus 935 / 9 / 3,890 `bdf5f1c8…`, the ledger entry's sizes
reproduced). Its textual census named 29 + 3 op-callback candidates; Clang's
closure over the analysed units names **27** for `rko_op_cb` and **4** for
`rko_serve` ([closure](artifacts/callback-members-corpus.json)); the textual census is
a reading aid and the Clang closure is the authority.

| predicted | measured |
| --- | --- |
| corpus, system, slice unchanged in status | unchanged on all three |
| no Safe → non-Safe, no Unknown → Safe | none |
| leaves only: uncontracted calls become named refusals | 102 rows, 155 leaves → section 5 |
| both op members close | closed over the recorded units (27, 4); the fold reads them open for four data-only units, a gap the prediction did not foresee |
| null property unchanged | byte-identical rows on all three routes |

## 5. Measurement

BEFORE `bin/before2` (`d747d1e8…`, built and run from a checkout of main
`579c2e6e2` in this worktree, so it read main's contracts; its reports are
byte-identical to the census BEFORE on `b565f79ab`); AFTER `bin/a4`
(`21989e3e…`, from `6c1d2ae02`); every route with `QCORE_NULL_PROPERTY`
([summary](artifacts/summary.json)). Each `run.json` names its binary.

| route | BEFORE | AFTER | report sha256 | wall (s, AFTER) |
| --- | --- | --- | --- | ---: |
| corpus | 935 / 9 / 3,890 | **935 / 9 / 3,890** | `bdf5f1c8…` → `527a4cad…` | 514.8 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `6fd1e8fa…` → `6105185b…` (the contract-source digests only) | 76.3 |
| slice | 52 / 1 / 347, 4 Pending | 52 / 1 / 347, 4 Pending | `75578164…` → `5f76680d…` (the same) | 38.1 |
| null, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | identical rows | `a0a1e6d7…` | |
| null, system / slice | 84 / 125 / 245; 71 / 103 / 232 | identical rows | | |

**Every moving row** ([corpus](artifacts/compare-corpus.json),
[system](artifacts/compare-system.json), [slice](artifacts/compare-slice.json)):
102 corpus rows, all Unknown before and after, none on the other routes. They
lose 155 `call to uncontracted function` leaves (132 `rd_kafka_op_destroy`, 23
the serve family) and gain: `callback-member-open` 119 (`rd_kafka_op_destroy`
where the unit stores nothing into `rko_op_cb`), `callback-uncontracted` 16
(`rd_kafka_op_destroy` 13 where the unit's closed set holds
`rd_kafka_share_fetch_reply_op_cb` or `rd_kafka_offset_reset_op_cb`;
`rd_kafka_q_serve` 3, `rd_kafka_poll_cb` and its kin),
`callback-withdrawn` 5 (rule 13y in `rdkafka_queue.c`: the serve bodies hand
the callback on), `callback-undecided` 1 (`rd_kafka_q_pop` passes a null
callback), and 14 access-gate refusals at member actuals of the now-classified
serve calls (`rk->rk_rep`). No leak, violation or `Unsafe` added. Rule keys on
the AFTER corpus: [reasons](artifacts/reasons-after-corpus.json); member sizes:
[after](artifacts/size-after-corpus.json) against
[before](artifacts/size-before-corpus.json).

**Body standing** (contract dependencies, corpus): 360 → 367 declarations
consulted; `rd_kafka_op_destroy` is partly validated (its arms by `12o` in
`rdkafka_op.c`), the six serve declarations declared assumptions (withdrawn by
13y where their bodies are). The Safe roll-up is unchanged (no Safe row rests
on a new declaration).

**The fold**: no row demoted (none rests on a decided member invocation);
`callback_members` lists both members open for the run only because four
data-only units recorded no facts (`ctraces/ctr_utils.c` and three
`librdkafka/opentelemetry/*.pb.c`), and closed over the recorded units.

**Intermediate binaries**, retained and not evidence for the tip: `a1`
(`5e860372…`, before the pinned-route fold and the ABI reader: the six serve
declarations withdrawn as `abi-unreadable-declaration`, corpus `a746066c…`), `a2`
(`f0767aa9…`, the fold), `a3` (`10f4b989…`, the ABI reader without the attribute
form: still withdrawn), all 935 / 9 / 3,890.

## 6. Lean

`proof/QCore/OpCallbacks.lean`, audited by `proof/AuditOpCallbacks.lean` (15
theorems; axioms within the standard `propext`, `Classical.choice`, `Quot.sound`; no placeholder, no project
axiom): the supplied object (`a_kept_supplied_object_is_refused`,
`an_admitted_decision_takes_or_is_identity`,
`a_taking_candidate_takes_the_object`,
`an_identity_candidate_receives_no_supplied_object`,
**`an_admitted_hand_off_never_leaves_the_callee_holding`**), the arms
(**`an_undeclared_arm_is_unknown`**, **`the_cautious_join_is_sound`**,
`a_call_without_otherwise_is_refused`, `an_undecided_link_is_refused`), the
member closure (**`an_open_member_never_decides`**, `an_open_run_demotes`,
`a_kept_decision_covers_every_run_candidate`), and three vector lists bound to
the Rust (`suppliedClassifyVectors`, `suppliedDecisionVectors` in
`callback_contracts::tests`; `armJoinVectors` in `arm_release::tests`).
`QCore.BodyClauseStanding` gains the clause kind `arm` and the evidence `r12o`
(its coverage and standing vectors, bound by `tests/contract_clause_standing.rs`
and `contract_dependencies_report.rs`). Lean audits 317 → **318**.

## 7. Ledger

`scripts/update_ledger.py` (idempotent): **`CG22-OP-CALLBACKS-AND-SERVED-QUEUES`
retired** (its DISCHARGE, "a clause for a callback that takes over an object
the callee supplies, a type-switched release, and a registration carried by an
op, each measured", holds). Registered, each with a DISCHARGE:
`OP-APP-RK-CALLBACKS-VIA-POLL-CB` (tied to the pending `rd_kafka_destroy_app`
decision, not decided), `OP-CALLBACK-CANDIDATE-CONTRACTS` (27 + 4),
`OP-CONSTRUCTOR-DECLARATIONS`, `SERVE-FORWARDED-CALLBACK-FORMALS`,
`CALLBACK-MEMBER-DATA-ONLY-UNITS` and
`RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN` (a control written while building
the tests: a decided member invocation releasing a *formal* read `UnknownAlias`,
a tracked local `Safe`; measured once, the controls use tracked locals).
`dimensions.named_followups` 765 → **770**.

## 8. Gate

GATE-SECTION

## Files

* Vocabulary: `crates/qcore-schemas/src/{callback,arm_release,retained_by}.rs`;
  loader `crates/qcore-contract/src/{contract_callback,contract_arm_release,contract_loader}.rs`;
  spec C04.19.1–C04.19.3, grammar G05.
* Route G: `crates/qcore-c-gateway/src/{callback_contracts,callback_member_closure,arm_release_check}.rs`,
  hooks in `legacy_callback_route.rs` (member candidates, arms, `settle`),
  `legacy_retained_by.rs`, `typed_access_path.rs` (facts beside the tables),
  `contract_dependencies.rs` (`member_evidence`), `contract_clause_standing.rs`
  (`Arm`, `12o`), `legacy_declared_abi.rs` (`read_function_typedef`).
* Run fold: `crates/qcore-audit/src/callback_member_fold.rs`, hooks in
  `report.rs`, `projection.rs`, `pinned_route.rs`, `current.rs`, `lib.rs`,
  `no_build.rs`, `compose.rs`.
* Lean: `proof/QCore/OpCallbacks.lean`, `proof/AuditOpCallbacks.lean`,
  `proof/QCore/BodyClauseStanding.lean`, `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/cg22_op_callbacks.rs` (fixture
  `fixtures/contracts/op_callbacks.qv`), `crates/qcore-contract/tests/op_callback_clause.rs`,
  `crates/qcore-c-gateway/tests/loader_abi.rs` rows, unit tests in the new
  modules; pins `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`.
* Records: `docs/agents/gateway.md` (12m–12p), `docs/agents/README.md`,
  `docs/STATUS.md`, `docs/development/increments/wave-48-part7.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
