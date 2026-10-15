# HANDOFF-CANDIDATE-OWN-CALLBACKS: a candidate's own invocations, composed

Card HANDOFF-CANDIDATE-OWN-CALLBACKS (wave 48, card-high). Gateway rule
**13u** amended in place (with **12m** and **12n**); no new rule id. Contract
specification **C04.19.7** added, C04.19.5 and C04.18 amended; no grammar
change. Lean `QCore.CandidateComposition`. Base: local `main` `d708641bc`,
rebased onto `5b1ceecd5` (FB-QUANT-CONTRACT-CENSUS and LEAN-4-35-TOOLCHAIN:
records, Lean and scripts only; no product input differs, so the
measurements below stand). The census and the frozen
[prediction](PREDICTION.md) were committed (`2932c1cd0`) before any
implementation code. No pause: no new surface syntax was needed and the
corpus has no cycle among composed candidates.

## 1. Defect and census

A candidate whose declaration carries callback clauses is a retaining
declaration, withheld from the function map, so rule 13u read it as
uncontracted wherever it was a candidate. Four declarations in force carry
their own member invocations and are candidates ([prediction](PREDICTION.md)
sections 1–2):

| candidate | candidate of | own invocations (run sets) | composition |
| --- | --- | --- | --- |
| `rd_kafka_oauthbearer_refresh_op` | `rko_op_cb` (27) | `rk_conf.sasl.oauthbearer.token_refresh_cb` {six declared identities} | **decides**: three borrowed formals, the identity |
| `rd_kafka_poll_cb` | `rko_serve` (4); the served-queue designators | twelve: nine rk_conf / share members closed (empty or declared identities); `rko_u.offset_commit.cb` and `rko_u.terminated.cb` **open** (the union's type-keyed opaque writes); `rko_op_cb` | **refuses** at `rko_u.offset_commit.cb` (`callback-member-open`); its `rko_op_cb` invocation would refuse too (section 4) |
| `rd_kafka_consume_cb` | `rd_kafka_q_serve` (`rd_kafka_consume_callback0`) | `consume_ctx.consume_cb` (closed, empty) | **decides**: `rko` consumed |
| `rd_kafka_background_queue_serve` | `rd_kafka_q_serve` (`rdkafka_background.c:133`) | nine rk_conf members, all closed | **decides**: `rko` consumed (its call refuses earlier in the corpus) |

**Cycles.** None in the corpus: no composed candidate's inner set contains a
composed candidate that reaches it back (`rd_kafka_poll_cb` is an `rko_serve`
candidate, which no composed candidate invokes). The least fixed point is
built and tested on fixtures. No coinductive reading is proposed: it would
decide nothing more here.

**The copy.** `rd_kafka_op_handle` writes its formal `callback` once, from
`rko->rko_serve` exactly (`rko` a formal it never writes), never takes its
address, and calls it. **Reading taken, flagged for the integrator:** the
brief's "an unmodified local, assigned once" is read to include a formal
(an automatic object of its function); such a site invokes the member's value
where the copy ran and the formal's own value elsewhere, and its result
handling is the same either way. The object must be a formal the body never
writes; a local object is not followed.

## 2. Mechanism

* **Composition** (`crates/qcore-c-gateway/src/candidate_composition.rs`,
  hooks in `legacy_callback_route.rs`: `Parser::classified_in`,
  `Parser::composed`, `handed_with`, `Classified`). A candidate whose
  declaration in force carries only member invocations beside its custody
  contract and a hand-off (`composable`) is decided at the invocation: each
  own invocation over the **run's** set (the pre-pass; a route without it
  refuses, since the fold checks only the members of declarations a row
  lists), each inner candidate classified with the candidate as rule 12m's
  invoker, then `decide_run`; its formals strengthened by the inner
  requirements (`strengthen`); its hand-off checked at the outer invoker
  after the composition; an inner member-path state at a formal the outer
  invocation hands a parameter refuses. A refusal is
  `callback-own-invocation-refused`, naming the invocation and the inner
  reason. **Least fixed point**: `with_stack` refuses a candidate met again
  while its own invocations are decided, naming the cycle.
* **Dependencies.** A decided synchronous, table or member invocation notes
  each candidate, and a composed candidate's inner candidates, for the row
  (`contract_consultation::note_relied`, read by `row_dependencies` without
  the body-spelling filter). Added after the first AFTER binary (`a1`)
  measured two Safe gains whose rows listed only `rd_kafka_op_destroy`: the
  user's standing rule that the per-verdict dependency set stays correct for
  new evidence. `a2` is `a1` plus this; their verdicts are identical.
* **The exact copy** (`handoff_branch.rs`: `exact_copy`, `variable_writes`,
  `exact_copy_class`, `Site::copy`, followed in `search`).

Tests: `crates/qcore-c-gateway/tests/handoff_candidate_own_callbacks.rs` (a
plain candidate as before; a decided own invocation contracted, including a
run-closed empty set; an undecided one refused for an effectful inner
candidate, an open set, and no run facts; a self-cycle; a two-candidate
cycle; an exact copy followed for a formal and a local; five inexact copies
refused; a row resting on a composed candidate lists every declaration
the decision read), the unit tests of `candidate_composition` (vectors bound to
Lean, `strengthen`, the cycle text) and `handoff_branch::copy_vectors_hold`.

## 3. Lean

`proof/QCore/CandidateComposition.lean`, audited by
`proof/AuditCandidateComposition.lean`, 13 theorems, standard axioms only:
`a_composed_candidate_is_contracted_iff_each_own_invocation_decides`,
`an_undecided_own_invocation_refuses`, `a_cycle_never_supports_itself` (any
set of candidates each needing one of the set is never contracted at any
round), `a_self_cycle_never_stands`, `a_two_candidate_cycle_never_stands`,
`compose_vectors_hold` (the least fixed point and the Rust's depth-first
search agree on `COMPOSE_VECTORS`); `exec_held`,
`an_exact_copy_calls_a_member_value`, `a_copied_local_calls_a_member_value`,
`an_exact_copy_stays_in_the_closed_set`,
`an_inexact_copy_may_call_another_value`, `copy_vectors_hold`
(`COPY_VECTORS`), `an_admitted_copy_is_the_only_write`. Checked under Lean
4.34.0 and, after the rebase, 4.35.0. Lean audits 324 → **325**.

## 4. Measurement

BEFORE `before` (`1517b4d5…`, base code) and AFTER `a2` (`049bf574…`, built
at `082e87fd3`, the code of `78753b4f8` after the rebase), each `run.json`
naming its binary, every route with the null property
([summary](artifacts/summary.json), scripts [`before.sh`](scripts/before.sh),
[`after.sh`](scripts/after.sh)):

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 936 / 9 / 3,889 | **938 / 9 / 3,887** | `71387bd3…` → `2a1e37c2…` | 796.9 → 679.4 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `7ba418a3…`, byte-identical | 99.3 → 78.1 |
| slice | 52 / 1 / 347, 4 Pending | the same | `503ad53a…`, byte-identical | 49.6 → 40.5 |
| null | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | tallies identical; corpus: 2 rows' `custody` and `combined` fields follow the two Safe gains, system and slice rows identical ([compare](artifacts/null-compare.txt)) | | |

Wall times measured once each on a shared host (other cards' runs queued on
the same lock); not a performance claim. `a1` (`a7403a59…`, without the
dependency notes) gave the same statuses and leaves (`a4de7033…`), system
and slice byte-identical.

**Moving rows** ([compare](artifacts/compare-corpus.json), leaves
[before](artifacts/leaves-before-corpus.json) and
[after](artifacts/leaves-after-corpus.json)): 88, all Unknown before.

* **`rd_kafka_op_destroy`'s 131 leaves in 83 rows decide** (the composed
  `rd_kafka_oauthbearer_refresh_op`; the three `== KEEP` hand-offs admitted
  at `rd_kafka_op_destroy`, [census](artifacts/handoff-census.json)). 81 rows
  stay Unknown on their other leaves and gain none; **2 become Safe**.
* **The 4 `rd_kafka_poll_cb` rows**: `rd_kafka_op_req` and `rd_kafka_op_req2`
  now name `rd_kafka_toppar_op_serve` (`callback-uncontracted`, the next
  `rko_serve` candidate); `rd_kafka_offsets_for_times` and
  `rd_kafka_query_watermark_offsets` name `rd_kafka_poll_cb`'s own
  `rko_u.offset_commit.cb` (`callback-own-invocation-refused`).
* **`rd_kafka_consume_callback0`**: `rd_kafka_consume_cb` composes; the call
  refuses at its next clause, naming `rd_kafka_toppar_op_serve`.

**Safe gains, read by hand** (no Safe → non-Safe change; no leak, violation
or Unsafe added; the fold demoted nothing):

| row | body | rests on |
| --- | --- | --- |
| `rdkafka_event.c::rd_kafka_event_destroy` (`:109`) | `if (!rkev) return; rd_kafka_op_destroy(rkev);` | `rd_kafka_op_destroy` (`resource_released(rko, nonnull)`, the discriminant and its arms, rule 12o, partly validated; its `rko_op_cb` member invocation, a declared assumption); the run's closed `rko_op_cb` set of 27, each declaration a declared assumption read at both revisions: 17 identities, the 8 member-path declarations (rule 30; no tracked object is linked below the op here), the three `== KEEP` hand-offs admitted by the caller branch check, and `rd_kafka_oauthbearer_refresh_op` composed over the run's closed `token_refresh_cb` set of six identities (Fluent Bit's `oauthbearer_token_refresh_cb`, librdkafka's five) |
| `rdunittest_fetcher.c::ut_op_destroy_free` (`:114`) | `rd_kafka_op_destroy((rd_kafka_op_t *)ptr);` | the same |

Premises: `P-RK-NO-PLUGIN-LIBRARY` (`rk_no_plugin_library`, on every
librdkafka candidate), `share_ack_batches_nonempty`
(`rd_kafka_share_fetch_fanout_op`), `P-CG22-MEMBER-CLOSED-WORLD` (both run
sets, with the pre-pass's scope file), and the union-layout reading that
closes `rko_op_cb` (`RKO-U-ARMS-BY-DISCRIMINANT`, Clang's x86-64 layout).
Each row's `contract_dependencies` now lists the 34 declarations; the Safe
roll-up moves both into "resting on a declared assumption" (382 → 384);
declarations consulted 428 → 431; 91 rows' dependency lists grow (89
Unknown).

**Next blocker per moved row** ([grouping](artifacts/blockers-corpus.json),
[`blockers.py`](scripts/blockers.py)): the 81 destroy rows keep their other
leaves, led by `unresolved resource identity` (82 of the 88 moved rows),
indirect memory access (76), uncontracted calls (73) and branch conditions
(59); the poll_cb rows as above, `rd_kafka_toppar_op_serve`
(`OP-CALLBACK-CANDIDATES-APP-CALLBACKS`) for three and the union's opaque
writes (`UNION-OPAQUE-RECORD-WRITES-BY-TYPE`) for two; behind the latter,
the nested hand-off at `rd_kafka_poll_cb`'s `rko_op_cb`
(`HANDOFF-THROUGH-A-HANDING-INVOKER`).

**The caller branch check with the copy** ([census](artifacts/handoff-census.json),
`examples/handoff_census.rs`, [`census.sh`](scripts/census.sh)):
`rd_kafka_poll_cb` at `rd_kafka_q_serve0`'s `rko_serve` is **admitted** (it
read "no invocation" before); refused at the pop serves as at their formal;
still not read at `rd_kafka_q_serve`, `_serve_maybe_consume` (the op is
created below the invoker) and `rd_kafka_op_req0` (no recorded site):
`HANDOFF-MEMBER-OBJECT-BELOW-INVOKER`. The two other `== KEEP` hand-offs are
admitted at `rd_kafka_op_destroy`; at `rd_kafka_poll_cb`'s own `rko_op_cb`
they are refused ("the body of `rd_kafka_consume_cb` is not read").

**Prediction against outcome.** As predicted: the 131 leaves decide, the
poll_cb and consume_cb leaves trade as stated, system, slice and null
unchanged, no cycle. **Not as predicted:** two Safe gains. The prediction
read the base's leaves artifact ("none decided by these leaves alone"); both
rows also carried `unresolved resource identity` on the same line, a
consequence of the refusal that disappears with it.

## 5. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 783 → **784**.

* **Retired:** `HANDOFF-CANDIDATE-OWN-CALLBACKS` (its DISCHARGE holds).
* **Amended:** `MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED` (narrowed to registry
  invocations and decided registrations), `UNION-OPAQUE-RECORD-WRITES-BY-TYPE`
  (now read by `rd_kafka_poll_cb`'s composition, 2 rows),
  `OP-CALLBACK-CANDIDATES-APP-CALLBACKS` (the named blocker of 3 rows).
* **Registered:** `HANDOFF-THROUGH-A-HANDING-INVOKER`,
  `HANDOFF-MEMBER-OBJECT-BELOW-INVOKER`.

## 6. Gate

Gate **g1** ([gate.json](gate.json), exits in [gate/](gate/), logs
`private/hcoc/gate-g1/`): every code step at `066c51b54` (rebased onto
`5b1ceecd5`; later commits change records only), the standing gate and the
provider gate (due: `crates/qcore-schemas` changed) under one exclusive flock
on the shared heavy lock (13:30:57Z–13:50:54Z;
[`gate-run.sh`](scripts/gate-run.sh), [`gate.sh`](scripts/gate.sh),
[`finish-gate.sh`](scripts/finish-gate.sh)). Lean 4.35.0, the pin.

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update | 0 | |
| `cargo fmt --check` | 0 | |
| `clippy --locked --all-targets -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 500 units, 513 summaries, **8,111 passed, 0 failed, 15 ignored** (`private/hcoc/tests-g1/`), no rerun |
| both reevaluation probes | 0, 0 | |
| `check_lean_audits.py` | 0 | **325** audits passed |
| provider gate | 0 each | `package.py` build, verify `qcore.libevent` and `qcore.librdkafka`; `cargo check --locked` in `providers/`; `contracts.py check`; `cargo test` 133 passed / 0 failed; providers Python 25 OK; fmt; clippy |
| `cargo clean --profile dev` | 0 | |
| Python suite before the matrix regeneration (08) | **1** | fails only `test_the_derived_facts_are_recomputed_rather_than_carried` (the committed matrix's Lean count, 324 → 325), as for every card adding an audit |
| docs index check (09) | 0 | |
| `build_matrix.py . --gate gate.json` (09a), Python suite (09b, 374 tests), docs index (10b) and its check (10c) | 0, 0, 0, 0 | |

The bold rule-id duplicate check is empty. Clang:
[`gate/clang-version.txt`](gate/clang-version.txt).

## Files

* `crates/qcore-c-gateway/src/{candidate_composition,legacy_callback_route,callback_contracts,handoff_branch,contract_dependencies,lib}.rs`,
  `tests/handoff_candidate_own_callbacks.rs`, `tests/contract_dependencies.rs`;
  `crates/qcore-schemas/src/contract_consultation.rs`;
  `crates/qcore-audit/examples/handoff_census.rs`.
* Lean: `proof/QCore/CandidateComposition.lean`,
  `proof/AuditCandidateComposition.lean`, `proof/QCore.lean`.
* Records: `docs/agents/gateway.md` (13u, 12m, 12n), the contract
  specification, `docs/STATUS.md`,
  `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
