# NULL-MEMBER-INVARIANT-SCOPE

Card **NULL-MEMBER-INVARIANT-SCOPE** (wave 48, round 23, 2026-10-10; card-high).
Base: local `main` at `c77efedbe`. Prediction: [PREDICTION.md](PREDICTION.md),
frozen at `61686ccea` before any implementation code. Gateway rules
[14x](../../agents/gateway.md#constraint-14x)–[14z](../../agents/gateway.md#constraint-14z);
contract specification
[C09.3](../../QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)
("Scoped member invariants"); grammar
[G05f](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05f-scoped-member-invariants);
Lean `QCore.NullMemberScope` (`proof/AuditNullMemberScope.lean`).

Authority: the ledger entry `NULL-MEMBER-INVARIANT-SCOPE`, NULL-CANDIDATE-TRIAGE's
report (G05e, rule 14t), and the user decisions "move the NULL question",
decision 2, "handle as safe as possible", "no finding is made by widening what
a free value may be", "no candidate is removed except by checked knowledge".

**Rule numbers.** The brief assigned `14x`–`15b`; `15a` and `15b` are taken
(`source-routes.md`, constraint 15a and 15b), so the card uses `14x`–`14z`
only. (The sibling card's `15c`–`15g` are taken too, by `io.md`.)

**Naming (USER DECISION 2026-10-11).** Asked of this card's first spelling,
verbatim: "null_member_scope -> This is also nonnull or nullable prefix
should be used there?" The decision: every non-NULL clause is spelled with
the prefix that states the claim, `nonnull_`; `nullable_` is reserved for
clauses stating a value may be NULL. This card renamed all four, with no
aliases: `null_member_nonnull` → `nonnull_member`, `null_result_nonnull` →
`nonnull_result` (both landed by NULL-CANDIDATE-TRIAGE), and its own
`null_member_scope` → `nonnull_member_scope`, `null_member_stable` →
`nonnull_member_stable`; diagnostics (`nonnull-member-shape`,
`nonnull-result-pointer`, `nonnull-member-stable-shape`,
`nonnull-member-scope-shape`, `nonnull-member-scope-parameter`) and the
`BoundsClause` variants (`NonnullMember`, `NonnullResult`,
`NonnullMemberStable`, `NonnullMemberScope`) follow; the proof and dependency
names (`result:member:`, `member:`, `result:`, `stable:`) are unchanged. The
control ([rename-control/](artifacts/rename-control/)): with the renamed text
and the lock as it stood, `pin --bounds --check` of the librdkafka crate
reported all **72** changed declarations `contract-changed` at both revisions
(exit 1; the 66 scope entries, `rd_kafka_broker_add`, the four member hosts and
`rd_kafka_topic_keep`); each was then re-pinned at 66910c10 and 046accf6
(`pin --check` 0 at both, `pin --verify-contracts` 0). Measurements before the
rename (a1, a2) carry the old spellings in their row text; run a3, after it,
is compared with a2 in section 3.

## 1. The reading, and why two of the three members have none

A global member declaration (rule 14t) is withdrawn by one NULL store
anywhere. The three members fail it for two different reasons:

* **Fresh-object stores.** `rd_kafka_broker_s.rkb_rk` was withdrawn by four
  initializer lists: the lookup key `skel = {.rkb_nodeid = nodeid}` and three
  `RD_ZERO_INIT` unit-test brokers. An initializer list writes an object no
  use held before it, so it cannot make an *existing* object's member NULL
  (`a_fresh_store_does_not_reach`). Every other store is proved. The member
  is **stable once non-null**.
* **Exit stores on existing objects.** `rd_kafka_buf_s.rkbuf_rkb` is set to
  NULL on the broker's receive buffer (`rd_kafka_recv`,
  `rdkafka_broker.c:2134`) and set again by `rd_kafka_req_response` before the
  response callbacks run; `mk_list.next` and `cfl_list.next` are set to NULL by
  `mk_list_del` and `cfl_list_del`. The invariant holds for objects in a state
  (a buffer handed to its parser, a linked node) that is left in place, on an
  object other code may still hold. A function scope does not rescue it: its
  entry fact survives a call only if no store in the call (or on another
  thread, made visible by a lock the call takes) can reset the object, and
  showing that the reset never targets the object a scoped use holds is a
  question of object identity across memory, calls and threads that no check
  here decides (`an_exit_store_breaks_persistence`). Most of rkbuf_rkb's
  candidates are the parse-failure macros' reads after
  `if (rkbuf->rkbuf_rkb)` and an intervening `rd_snprintf`/`strlen`/
  `mtx_lock`; the response handlers are reached only through the open
  indirect call `rkbuf_cb`. **Not declared**; registered
  (`NULL-MEMBER-EXIT-STORE-OBJECT-IDENTITY`).

## 2. What changed

| Piece | Where | Rule |
| --- | --- | --- |
| `requires nonnull_member_stable(TAG, FIELD);` and `requires nonnull_member_scope(TAG, FIELD, P);` in bounds v1 (`BoundsClause::NonnullMemberStable`, `NonnullMemberScope`), null property only, removed by `for_bounds`, admitted beside a resource view (`null_deref_beside`), `RESERVED_SPELLINGS` 14 → 16 | `qcore-contract/src/bounds_contract.rs`, `contract_views.rs` | 14x, 14z |
| Stable stores by kind (`assignment`, `initializer` exempt, `whole-object`: struct assignment, typed `memset`/`memcpy`/`memmove`/`bzero`), each store's `relies` | `null_deref/scope.rs`, `walk.rs` (`stable_store`, `record_initializer`, `store`, `call`) | 14x |
| Persistence of a non-null fact on `V->FIELD` (`V` tracked) across calls and stores, proof and dependency `stable:TAG.FIELD` | `walk.rs` (`memory_written`, `stable_member_of_key`), `qcore_core::null_member_scope::persist` | 14y |
| Scope entries: the body's entry fact, the row's scope, each caller's `call-precondition` check of `ACTUAL->FIELD` | `walk.rs` (entry facts, `scope_obligation`), `null_deref.rs` | 14z |
| The fold: stable standings, the least withdrawn set closed under "a store proved only by a withdrawn declaration fails" (rule 14t's members too), `stable_declarations` and `scope_declarations` only when declared | `qcore-audit/src/null_member_scope_fold.rs`, thin hooks in `null_fold.rs`, `projection.rs` | 14x |
| Declarations: `rd_kafka_broker_s.rkb_rk` stable (host `rd_kafka_broker_add`) and 66 scope entries `nonnull_member_scope(rd_kafka_broker_s, rkb_rk, rkb)`, pinned at both revisions | `providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv`, `bounds-contracts.lock` (20 → 86 pinned) | 14x, 14z |

The scope entries were chosen mechanically from the stable reading's run
([scope_candidates.py](scripts/scope_candidates.py),
[scope-choice.json](artifacts/scope-choice.json)): every librdkafka function
whose own row kept a candidate on `rkb->rkb_rk` alone, whose first parameter
is `rd_kafka_broker_t *rkb`, whose definition is byte-identical at 66910c10
and 046accf6, whose address the library never takes, and whose parameters all
have bounds v1 types. 117 such functions were not declared (45 definitions
the script does not read, 38 whose broker is not the first parameter, 28 with
a struct-by-value parameter, 6 address-taken). `rd_kafka_broker_add`'s body is
byte-identical at both revisions ([bodies.py](scripts/bodies.py)), as are its
four fresh-object stores and two assignments.

## 3. Measurement

BEFORE `b0` (binary `b985808e…`, base) reproduces the brief: custody corpus
**915 / 9 / 3,910** (`ded14632…`), system 91 / 4 / 353, slice 50 / 1 / 349 / 4
Pending; null corpus 1,115 / 1,421 Safe, 2,363 Unknown, 22 Unsafe, 33 findings,
12,809 candidates = 10,907 own + 1,902 travelled. AFTER `a3` (binary
`0fc0b879…`, the tip's code after the `nonnull_` rename; `run.json`'s
`binary_sha256` checked). `a2` (binary `70d36c4c…`) is the same reading before
the rename: every null row of a2, with the old spellings replaced, equals a3's
apart from the bounds environment's digest (`f9a068b3…` → `d095788e…`) on all
three routes ([rename-identity-corpus.json](artifacts/rename-identity-corpus.json),
-system, -slice: 0 rows differ, 0 verdicts). `a1` is the stable member alone
(no scope entries, same code as a2).

**Custody**: byte-identical on all three routes (corpus `ded14632…`, system
`4f87449c…`, slice `e97e2c81…`); the parity streams are multiset-identical
(corpus 690,246 rows, system 142,512, slice 51,342; no row only on one side):
[parity-corpus.json](artifacts/parity-corpus.json), [-system](artifacts/parity-system.json),
[-slice](artifacts/parity-slice.json).

**Null property, corpus** ([compare-corpus.json](artifacts/compare-corpus.json),
[counts-corpus-after.json](artifacts/counts-corpus-after.json)):

| | BEFORE | stable only (a1) | AFTER (a3) |
| --- | ---: | ---: | ---: |
| Safe unconditional | 1,115 | 1,115 | **1,113** |
| Safe conditional | 1,421 | 1,421 | **1,433** |
| Unknown | 2,363 | 2,363 | **2,353** |
| Unsafe (provisional), findings | 22, 33 | 22, 33 | 22, **33** |
| candidates (own + travelled) | 12,809 (10,907 + 1,902) | 12,584 (10,713 + 1,871) | **12,648** (10,777 + 1,871) |
| memory-field, own checks | 8,605 | 8,411 | **8,475** |
| `->rkb_rk`-sourced (own + travelled) | 682 + 59 | 488 + 28 | **552** + 28 |
| `->rkbuf_rkb`-sourced | 3,474 + 41 | unchanged | unchanged |
| `->next`-sourced | 602 + 2 | unchanged | unchanged |
| checks kept by the stable member | 0 | 265 | **449** (218 proved, 231 moved) |
| scope-entry caller checks | 0 | 0 | **312** (38 proved, 95 moved, 179 candidates) |

Every check change, corpus: 340 candidates removed, 179 added (the scope
entries' caller checks), 265 proved and 248 moved added, 40 moved removed.
Row moves: **Unknown → Safe conditional 26** (every one a declared scope
function whose own `rkb->rkb_rk` reads were its only candidates; 20 also rest
on the stable member); **Safe → Unknown 16** (14 conditional, 2
unconditional), each a caller that now carries a scope entry's obligation it
does not prove: the question moved from the callee's first read to the call,
as decision 2 moves it for `quant null-deref` (`rd_kafka_all_brokers_wakeup`,
`rd_kafka_broker_buf_enq1`, `rd_kafka_broker_producer_serve`,
`rd_kafka_broker_share_consumer_serve`, `rd_kafka_broker_toppars_timeout_scan`,
`rd_kafka_broker_session_update`,
`rd_kafka_broker_session_update_toppars_in_session`,
`rd_kafka_AlterUserScramCredentialsRequest_salted_password`,
`rd_kafka_toppar_set_fetch_state`, `rd_kafka_sasl_auth_done`,
`rd_kafka_topic_partition_available`, `rd_kafka_toppar_delegate_to_leader`,
`rd_kafka_transport_connect_done`, `rd_kafka_transport_io_event`,
`rd_kafka_txn_coord_query`, and the unit test `rd_ut_features_check`, whose
actual `&rkb` is a local the walk reads as a memory field). No move is
unattributed; findings are unchanged (33, all 22 Unsafe rows unchanged).

**System and slice**: no null row moves and no check changes (system 84 / 125
/ 245, slice 71 / 103 / 232; neither has an rkb_rk-sourced candidate; the
stable declaration stands `declared-assumption` there, no store examined).

**Stable standing**, corpus: `rd_kafka_broker_s.rkb_rk` **partly-validated**,
2 assignments examined (both proved), 4 fresh-object stores counted, 0
whole-object stores; nothing withdrawn.

**Custody premises**: premise checks the null property proves 782 → **786**
(candidates 892 → 888); custody Safe rows with an own premise 12, every
premise proved 1 → **1** (system 2 / 0, slice 5 / 1 unchanged): the premises
read `s->iov`, `i->storage` and list elements, not `rkb_rk`.

**Wall time** (inside the lock, null and parity selected): corpus 515.7 s
(b0) → 509.8 s (a1) → 564.4 s (a2) → 527.8 s (a3); system 75.1 → 75.3 (a2) →
87.5 s (a3); slice 33.8 → 33.9 (a2) → 45.6 s (a3). a2 and a3 run the same
reading (only spellings differ), so their spread (system +16 %, slice +35 %
between two runs of one reading) is the shared host's load (the sibling card
built and measured at the same time), not the change; the lock serialises
heavy steps but not the rest of the host. Within that noise: corpus +2 % (a3),
−1 % (a1).

## 4. Prediction against outcome

| Predicted | Measured |
| --- | --- |
| Custody byte-identical | yes, all three, parity multiset-identical |
| Premise rows fully proved 1 → 1 | 1 → 1 |
| `rkb_rk` partly-validated, 0 failures, 4–8 fresh stores | partly-validated, 0, 4 |
| `rkbuf_rkb`, `mk_list.next`, `cfl_list.next` not declared | not declared, sized |
| `->rkb_rk`-sourced −150 to −500 | −161 (741 → 580); −225 with the stable member alone |
| Memory-field own −150 to −600 | −130 (8,605 → 8,475); −194 stable only: **below the range**, the scope entries' caller checks are memory-field candidates |
| Unknown → Safe +3 to +40, no unexplained Safe → non-Safe | +26; 16 Safe → Unknown, each a moved question named above |
| Findings unchanged | 33 |
| System, slice small or none | none |
| Parity identical | yes |
| Wall +0 to +5 % | corpus a1 −1 %, a3 +2 %, a2 +9 %; runs of one reading spread up to 35 % on the shared host (see above) |

## 5. Lean

`QCore.NullMemberScope`, registered in `proof/QCore.lean`, audited by
`proof/AuditNullMemberScope.lean` (13 theorems; `propext` and `Quot.sound` at
most, no placeholder, no project axiom). A heap of objects and a trace of
fresh objects, stores (proved or not), exit stores, ends, reads and other
events, on any thread: `stable_fact_persists` (every store proved, no exit
store, the object not ended: a non-null member stays non-null),
`a_fresh_store_does_not_reach`, `a_scoped_use_sees_non_null` (the entry
obligation and stability give every read in the scope a non-null member),
`an_exit_store_breaks_persistence` and `an_unproved_store_breaks_persistence`
(why rkbuf_rkb and the lists are not declared, and why a failed store
withdraws), `a_kept_fact_is_non_null`, `nothing_is_kept_without_stability`,
`a_stable_declaration_is_never_validated`, `a_failed_store_withdraws_it`,
`failure_is_monotone` (the fold's iteration reaches the least closed set);
`QCore.NullTriage.a_withdrawn_dependency_is_never_safe` covers the demotion.
`stableVectors` and `persistVectors` bind `qcore_core::null_member_scope`
(`crates/qcore-core/tests/null_member_scope.rs`, parsed from the Lean text).

## 6. Controls

`crates/qcore-c-gateway/tests/null_member_invariant_scope.rs` (8): a stable
member's tested fact survives a call (proof `result:stable:S.next`,
dependency and assumption); without the declaration the call forgets it; a
possibly-null fact is not kept; a reassigned base and a memory base keep
nothing; every store kind is recorded (an untested formal is a candidate, an
address proved, an initializer `initializer`, a struct copy and a typed
`memset` `whole-object`) and none goes to rule 14t's list; a scope entry is
read at entry, its row conditional, a caller with the fact proves the
obligation and one without is a candidate; without a stable member the entry
fact ends at the first call; the loader admits and refuses the forms.
`qcore-audit`'s `null_member_scope_fold` tests (2): an initializer is exempt
and an unproved store withdraws; withdrawal cascades to a declared member whose
store rests on the withdrawn stable member, and the row relying on it is
demoted. `qcore-core/tests/null_member_scope.rs` (3).

### Deliberate test and pin updates

`qcore_contract_corpus.rs` (67 pinned rows: the stable host and the 66 scope
entries), `contract_loader.rs` (resource-v1 rejected declarations 246 → 313),
`contract_view_census.rs` and `qcore-contract`'s `contract_views.rs` (pairs
and relations 20 → 21, `null_deref_beside` 7 → 8:
`rd_kafka_toppar_fetch_backoff` has a resource view in `client.qv`),
`RESERVED_SPELLINGS` 14 → 16.

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/) (filled by the gate run).

## 8. Follow-ups

* **Retired:** `NULL-MEMBER-INVARIANT-SCOPE` (the vocabulary exists, checked
  against the stores and the scope's entry points; the registered broker is
  declared and measured; the received buffer and the linked list entry are
  measured and have no checked reading, registered below).
* **Amended:** `NULL-CUSTODY-PREMISE-RESIDUALS` (re-measured: 786 premise
  checks proved, rows fully proved unchanged).
* **Registered** (follow-ups 756 → 759): `NULL-MEMBER-EXIT-STORE-OBJECT-IDENTITY`
  (rkbuf_rkb 3,474 + 41, `->next` 602 + 2), `NULL-MEMBER-SCOPE-ENTRY-INFERENCE`
  (552 own rkb_rk-sourced candidates left, 117 functions not declared, 179
  caller candidates), `NULL-MEMBER-STABLE-CENSUS` (the next memory-field
  members by size), `NULL-MEMBER-WHOLE-OBJECT-WRITES` (rule 14t's members do not
  record attributable whole-object writes; untyped ones stay a premise).

## Scripts

`scripts/`: `build.sh`, `measure.sh`, `measure-binary.sh`, `heavy.sh`,
`pin.sh` (`PINS_FILE`), `lean.sh`, `totals.py`, `member_counts.py`,
`compare.py`, `parity.py`, `routes.sh`, `summary.py`, `scope_candidates.py`,
`bodies.py`, `candidates.py`, `artifacts.py`, `update_ledger.py`, `rename-control.sh`,
`rename_identity.py`, `gate.sh`,
`provider-gate.sh`, `write_gate_json.py`, `finish-gate.sh`.
