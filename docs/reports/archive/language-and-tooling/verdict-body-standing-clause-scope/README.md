# VERDICT-BODY-STANDING-CLAUSE-SCOPE: body standing per clause

Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (wave 48, card-high), registered by
[VERDICT-CONTRACT-DEPENDENCIES](../verdict-contract-dependencies/README.md)
(rule 13g), on the user's direction of 2026-10-09: "handle as safe as
possible". Of the ledger entry's two discharges (a per-clause standing, or a
decision that a structural check validates its declaration) the brief chose
the per-clause standing. Base: local `main` at `9564eeb6d`, which it stayed
at. [PREDICTION.md](PREDICTION.md) was committed at `cf3d2aec1`, before any
implementation code.

**Published property and scope.** No new property: custody (`c.custody`) on
the pinned corpus, the system route and the tail-pause slice, unchanged. The
card changes what the per-declaration **assumptions** label of the verdict
contract (contract specification C06.5) means: `validated` now requires every
clause of the declaration to be covered by body evidence. Gateway rules
**13i–13k**; Lean `QCore.BodyClauseStanding`.

## 1. Which clauses each piece of evidence covers (rule 13i)

Read from the code that records each piece of evidence
(`crates/qcore-c-gateway/src/contract_clause_standing.rs`, `covers`):

| evidence | where it is recorded | what it compares or reads | clauses covered |
| --- | --- | --- | --- |
| `W4` | `contract_dependencies::unit_evidence` | the final derivation's validated summary of the body equals the declaration's custody projection (`legacy_summary.rs`: a conflict is `declared.custody_projection() != inferred`, and the projection clears container and counted-user clauses) | every parameter mode with its kind (and the arity), the result mode with its kind |
| `entry` | `analyze_defined_function_unscoped` | the body under the declaration's entry custody: `observations_match_contract` checks every parameter mode (`parameter_matches`), out slots and the result (`result_matches`) at every exit; recorded only for a declaration without container or counted-user clauses | the same |
| `11r` | `CustodyDerivation::relational_evidence` | the body's ordinary, exit-specific or forwarded summary against each exit class's custody projection (`legacy_relational_summary.rs::contradiction`) | every exit class (a class is the whole contract on its exits, unconditional modes included) |
| `12w` | `legacy_retention_guard.rs::settle` | each `when` guard of a guarded retention | the guards, not the retentions |
| `13a` | `legacy_slot_move_borrow.rs::settle` | slot moves and derived out slots of a retaining declaration; the borrowed result of a declaration **in the function map** (the hook reads no other table) | slot moves, derived out slots, a function-map borrowed result |
| `13c` | `legacy_reference_count.rs::settle_clauses` | every mention of the reference parameter | reference keeps and drops |
| `13d` | `legacy_reference_result.rs::settle` | every `return` of a reference result | the returned reference |

Nothing covers a container (C04.8) or counted-user (C04.7) clause, a
retention or holder reset (C04.14), or a clause that lives only in the
retained text: a named premise (C04.18), a variadic tail, `resource_no_return()`.
A text the loader refuses yields an `Unreadable` clause that nothing covers.
A literal-argument premise (C04.5) and the loop-owner, thread-synchronization
and lifecycle obligations bound where a declaration is believed (C04.5: "a
bound on where a trusted assumption is believed, not evidence for it"); they
are callers' obligations and are not clauses a body must establish.

Two readings were made precise by reading the code. A borrowed result of a
**retaining** declaration is an ordinary result clause: rule 13a's hook reads
the function map only, so `13a` evidence for a retaining declaration's slot
moves never covers its result. And 13c's check reads the returns of an alias
keep (`return rktp;`), but only to admit the reference parameter's uses; under
the brief ("a structural check covers only the clause kind it reads") its
evidence covers the reference clause, not the `resource_return_alias` clause.

## 2. Per-clause standing (rules 13j, 13k)

Each unit row's `contract_body_evidence` gains `clauses`: for every
declaration the unit validated, every clause label with the evidence that
covered it there (`unit_coverage`). The run folds it (the composer folds the
same unit rows): rule 13g's standing (every examining unit validated, one at
least) is the precondition; under it a clause is `validated` when every
validated unit covered it, and the declaration is `validated` only when it has
a clause and every clause is (`clause_standing_of`); `partly-validated` when
some clause is (`uncovered_clauses` listed); otherwise a
`declared-assumption`. The sidecar publishes each declaration's `clauses`,
`counts.partly_validated`, `safe_rollup.resting_on_a_partly_validated_declaration`
and `partly_validated_declarations` (Safe rows and uncovered clauses each).
A unit row without `clauses` never makes a declaration validated.

One shared item changed: `unit_evidence` now records `W4` whenever the
validated summary equals the custody projection, a declaration with counted-
user or container clauses included (before, such a declaration got no
evidence at all). Under rule 13i that evidence covers its custody clauses
only, so such a declaration can be `partly-validated`, never `validated`.

## 3. Closing the gap (task 3)

The brief allowed closing an uncovered plain `resource_borrowed` formal with
an **existing exact** reading of it, such as the W4 summary's. None exists for
the gaps measured below:

* every one of the 13 retaining declarations among them is skipped by the
  derivation on purpose (`derive_custody_summaries`: "a retaining declaration
  is never replaced by an inference", rule 12t), so no W4 summary of its body
  is ever computed;
* the 12 function-map declarations (five 13a borrowed results, seven 13d
  returned references) have no validated W4 summary in any unit that examined
  them (otherwise `W4` would be recorded beside 13a/13d), and the `entry`
  check did not pass on them either (no `entry` evidence): a body that is not
  `Safe` without diagnostics under its entry custody gives no exact reading of
  one formal.

Computing a new W4 inference for these bodies as evidence only would be a new
derivation, not an existing reading, and it would build CFGs whose
`QCORE_TYPED_PATH_PARITY` rows change the parity multisets this card must
keep identical. So the clauses stay uncovered (registered:
`VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES`).

## 4. BEFORE

Binary `private/vbs/bin/before` `eb6d8599…` (= `run.json`), every route exit
0: corpus **881 / 9 / 3,944** (report `96852f4e…`, the previous card's AFTER
bytes; verdict rows `97cbdcf3…`; 473.2 s), system 87 / 4 / 357 (`adbdd52b…`,
74.0 s), slice 45 / 1 / 354 / 4 Pending (`6e514e4c…`, 30.8 s): the brief's
figures.

## 5. AFTER and the identity proof

Binary `private/vbs/bin/a1` `65d7c0d8…` (= `run.json` and the corpus binary
digest), built at `8e9529095` (later commits change formatting, tests and
records only), every route exit 0: corpus 449.6 s, system 63.2 s, slice 29.0 s.
[`scripts/identity.sh`](scripts/identity.sh) runs
[`verdict_projection.py`](scripts/verdict_projection.py) (removes exactly the
unit key `contract_body_evidence` from the report, **keeping every row's
`contract_dependencies`**, and exactly the `contract_dependencies` section
from the sidecar), `parity.py` and [`clause_rollup.py`](scripts/clause_rollup.py)
on each route:

| route | verdict rows (BEFORE = AFTER) | report projection (BEFORE = AFTER) | sidecar projection | parity multiset | report bytes added |
| --- | --- | --- | --- | --- | ---: |
| corpus | `97cbdcf3…` | `cd4187bb…` | `db79bb2c…` | identical (662,667 rows) | 190,331 (+1.7%) |
| system | `09b7a59e…` | `9378de8c…` | `f5f9262d…` | identical (139,962) | 13,217 (+0.8%) |
| slice | `ce1be80e…` | `b3d078c7…` | `e02c0ee0…` | identical (50,407) | 7,100 (+0.4%) |

No differing path after projection on any route: every row field, every
verdict, every other sidecar byte is unchanged. The only unit-field outcome
moves are the counted-user declarations `W4` now records (§6). The
`mk_vhost_set` tip probe ([`scripts/tip-probe.sh`](scripts/tip-probe.sh)):
verdict rows and projections identical on the tip and the master copy
(`mk_vhost_set` Unknown, 1 leaf at the tip and 6 on master;
`mk_vhost_set_property` Unknown, 2 leaves). Artifacts:
[`artifacts/`](artifacts/).

## 6. The new roll-up

| route | Safe rows | on no declaration | only on validated | on a partly validated | on a declared assumption | declarations (validated / partly / declared assumption / held) |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| corpus BEFORE | 881 | 418 | 85 | — | 378 | 330 (73 / — / 256 / 1) |
| corpus AFTER | 881 | 418 | **64** | 22 | 377 | 330 (**48** / 29 / 252 / 1) |
| system AFTER | 87 | 47 | 3 | 2 | 35 | 99 (8 / 5 / 84 / 2) |
| slice AFTER | 45 | 29 | 0 | 2 | 14 | 83 (4 / 1 / 69 / 9) |

**Of the 85 corpus Safe rows that rested only on validated declarations, 64
still do**; 21 move to the partly-validated bucket. AFTER's validated set is a
subset of BEFORE's on every route, and every AFTER "only validated" row was
one BEFORE (`clause_rollup.py` checks both). The partly-validated bucket also
takes one corpus row (system 2, slice 2) from the declared-assumption bucket:
it rests on a counted-user declaration that now has `W4` evidence for its
custody clauses (`flb_input_chunk_is_mem_overlimit`, `flb_input_chunk_is_up`,
`flb_task_get_route_status`, `flb_task_set_route_status`; system also
`flb_task_users_inc`; slice `flb_input_collector_running`), each with its
counted-user clause (`resource_user_neutral()`, `resource_user_added(#0, users)`)
uncovered.

**The 28 structural-alone declarations** (corpus; system and slice have none;
the count reproduces). Correction to the ledger's description ("the rule
13a/13c/13d declarations of `librdkafka/client.qv` and five 13a borrowed
results"): the 28 are 24 declarations of `librdkafka/client.qv` (13c 10,
13d 9, 13a 5: four borrowed results and `rd_slice_reader0`'s derived out
slot), `get_object_item` (`librdkafka/cjson.qv`, 13a, the fifth borrowed
result) and three of `ctraces/context_lifetime.qv` (`sds_cat_safe`, a 13a slot
move; `ctr_attributes_set_array` and `_kvlist`, 11r).
three stay `validated` because their evidence covers every clause they have:
`ctr_attributes_set_array` and `ctr_attributes_set_kvlist` (11r compares each
exit class's whole contract) and `ut_create_mock_queue` (no parameter; 13d
covers its only clause, the returned reference). The other 25 are
`partly-validated`. Their gaps (`#i` is the parameter index):

| declaration | evidence | Safe rows | uncovered clauses |
| --- | --- | ---: | --- |
| `get_object_item` | 13a | 3 | `resource_borrowed(#0)`, `(#1)`, `(#2)` |
| `rd_kafka_admin_result_ret_groups` | 13a | 4 | `resource_borrowed(#0)`, `(#1)` |
| `rd_kafka_admin_result_ret_resources` | 13a | 3 | `resource_borrowed(#0)`, `(#1)` |
| `rd_kafka_admin_result_ret_topics` | 13a | 3 | `resource_borrowed(#0)`, `(#1)` |
| `rd_kafka_topic_partition_get_private` | 13a | 4 | `resource_borrowed(#0)` |
| `rd_slice_reader0` | 13a | 2 | `resource_borrowed(#0)`, `(#1)`, `(#2)`, `resource_return_none()` |
| `sds_cat_safe` | 13a | 5 | `resource_borrowed(#0)`, `(#1)`, `resource_return_none()` |
| `rd_kafka_toppar_keep0` | 13c | 1 | `resource_borrowed(#0)`, `(#1)`, `(#2)`, `resource_return_alias(#2, nonnull)` |
| `rd_kafka_q_keep`, `rd_kafka_q_keep_nolock`, `rd_kafka_topic_keep` | 13c | 0 | `resource_borrowed(#0)`, `resource_return_alias(#0, nonnull)` |
| `rd_kafka_toppar_destroy0` | 13c | 0 | `resource_borrowed(#0)`, `(#1)`, `(#2)`, `resource_return_none()` |
| `rd_kafka_enq_once_trigger` | 13c | 0 | `resource_borrowed(#0)`, `(#1)`, `(#2)`, `resource_return_none()` |
| `rd_kafka_q_destroy0` | 13c | 0 | `resource_borrowed(#0)`, `(#1)`, `resource_return_none()` |
| `rd_kafka_q_destroy_owner`, `rd_kafka_topic_destroy0`, `ut_destroy_mock_queue` | 13c | 0 | `resource_borrowed(#0)`, `resource_return_none()` |
| `rd_kafka_q_new0` | 13d | 0 | `resource_borrowed(#0)`–`(#3)`, `resource_retained_by(#0, result)`, `resource_retained_by(#2, result)` |
| `rd_kafka_toppar_get0`, `rd_kafka_toppar_get2` | 13d | 0 | `resource_borrowed(#0)`–`(#4)` |
| `rd_kafka_toppar_get_avail`, `rd_kafka_toppar_new0` | 13d | 0 | `resource_borrowed(#0)`–`(#3)` |
| `rd_kafka_toppar_desired_add`, `rd_kafka_toppar_desired_get` | 13d | 0 | `resource_borrowed(#0)`, `(#1)` |
| `ut_create_mock_toppar` | 13d | 0 | `resource_borrowed(#0)`, `(#1)`, `(#2)` |

The W4/entry-backed 45 all stay validated: none carries a named premise, a
variadic tail or a no-return clause.

## 7. Prediction against outcome

| prediction | outcome |
| --- | --- |
| no verdict moves; projections identical; parity multisets identical | held on all three routes (the report projection keeps the row fields, stronger than predicted) |
| all 28 structural-alone declarations partly-validated | wrong for 3: 11r covers whole exit classes (2), and `ut_create_mock_queue` has no parameter; 25 partly |
| task 3 closes none of the 28 | held (§3) |
| corpus validated 73 → 40–45 | 48 (the three above) |
| partly-validated 28–35 | 29 (25 + 4 counted-user declarations) |
| "only validated" 85 → 55–80; no-declaration 418 and declared-assumption 378 unchanged | 64; 418 held; declared assumption 377 (one row moved to the partly bucket, not foreseen) |
| system 8 and slice 4 validated unchanged, "only validated" 3 and 0 unchanged | held; partly 5 and 1 (counted-user declarations, not foreseen) |
| under 5% size | +1.7% / +0.8% / +0.4% |

## 8. Lean

`proof/QCore/BodyClauseStanding.lean`, audited by
`proof/AuditBodyClauseStanding.lean` (16 theorems, no placeholder, no project
axiom): coverage `a_structural_check_covers_only_what_it_reads`,
`a_structural_check_never_covers_a_formal`, `nothing_covers_these_clauses`;
standing `standing_eq_validated`, `standing_eq_partly`,
`validated_needs_every_clause`, `an_uncovered_clause_is_never_validated`,
`a_partly_validated_declaration_has_an_uncovered_clause`,
`validated_needs_the_evidence_standing`,
`validated_implies_previously_validated` (against
`QCore.ContractDependencies.standing`),
`a_declaration_with_a_premise_is_never_validated`,
`structural_evidence_alone_never_validates_a_formal`,
`a_held_declaration_is_never_validated`; roll-up
`the_validated_only_rollup_is_a_subset`; shared vectors `coverage_vectors_hold`
(bound to `covers` and `ClauseKind::ALL` by
`crates/qcore-c-gateway/tests/contract_clause_standing.rs`) and
`clause_standing_vectors_hold` (bound to the audit's fold, `render` and
`clause_standing_of` by `crates/qcore-audit/tests/contract_dependencies_report.rs`).

## 9. Gate

The first attempt `g1` (at `3360cf4c9`) is **discarded as validation
evidence**: the shared disk reached 0 bytes free during its test build
(`No space left on device` in rustc and a crashed `rust-lld`, step 04); steps
00–03 had exited 0. It was stopped, this card's dev build output was freed
(`cargo clean --profile dev`), and the gate was rerun once the disk recovered
above the 40 GB floor (logs kept in `private/vbs/gate-g1-disk-exhausted/`).

Gate `g2` (every step at `e089a6086`, the code of the AFTER binary plus
formatting, tests and records; each step under the shared lock; logs
`private/vbs/gate-g2/`): `cargo check --workspace --all-targets` 0; submodule
0; fmt 0; clippy `--locked` 0; `run_test_binaries.py` 461 units, 474
summaries, **7,728 passed, 0 failed**, 11 ignored, exit 0
(`private/vbs/tests-gate-g2/`, 392 s); both reevaluation probes 0;
`check_lean_audits.py` **300** audits passed; the Python suite 1 before the
matrix regeneration (it reads the committed Lean count, 299), then
`build_matrix.py --gate gate.json` 0 and the suite **365 OK** (09b); docs
index build and `--check` 0. No provider gate: `providers/` is unchanged from
the base; `gate.json` carries `provider_tests` 129 from
VERDICT-CONTRACT-DEPENDENCIES's provider gate and says so. Disk fell to 26 GB
free during `g2`'s test build (shared host); the step completed, and the dev
build output was freed right after it. Records: [gate.json](gate.json) and
[gate/](gate/).

## 10. Ledger

* `VERDICT-BODY-STANDING-CLAUSE-SCOPE` **retired**: per-clause standing, the
  clauses each piece of evidence covers, `validated` only with every clause
  covered, `partly-validated` listed, roll-up corrected, recorded in C06.5,
  Lean parity, verdicts identical.
* Registered `VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES` (the 25 corpus
  declarations of §6; 21 corpus Safe rows rest on them and no declared assumption) and
  `VERDICT-COUNTED-USER-CLAUSE-EVIDENCE` (the counted-user pass checks bodies
  but records no evidence: corpus 4, system 5, slice 1 declarations).

## Files

* Code: `crates/qcore-c-gateway/src/contract_clause_standing.rs` (new),
  `crates/qcore-c-gateway/src/contract_dependencies.rs` (the W4 record),
  `crates/qcore-c-gateway/src/lib.rs`, `crates/qcore-audit/src/contract_dependencies.rs`.
* Tests: `crates/qcore-c-gateway/tests/contract_clause_standing.rs` (new),
  `crates/qcore-audit/tests/contract_dependencies_report.rs`.
* Lean: `proof/QCore/BodyClauseStanding.lean`, `proof/AuditBodyClauseStanding.lean`,
  `proof/QCore.lean`.
* Specification: contract C06.5. Rules: gateway 13i–13k, router rows.
* Scripts: [`scripts/`](scripts/).
