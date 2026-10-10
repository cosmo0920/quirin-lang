# NULL-CANDIDATE-TRIAGE

Card **NULL-CANDIDATE-TRIAGE** (wave 48, round 22, 2026-10-10; card-high).
Base: local `main` at `cc7a20c3e`, rebased onto `2a9a0e9a1` (NULL-DEREF-QTT-GROUNDING landed; custody and null outputs byte-identical across it, re-measured on the rebased tip, section 6). Prediction: [PREDICTION.md](PREDICTION.md),
frozen at `a4a878764` before any implementation code. Gateway rules
[14r](../../agents/gateway.md#constraint-14r)–[14w](../../agents/gateway.md#constraint-14w);
contract specification
[C09.3](../../QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)
("Triage"); grammar [G05e](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05e-null-property-member-and-result-declarations);
Lean `QCore.NullTriage` (`proof/AuditNullTriage.lean`).

Authority: the ledger entry `NULL-CANDIDATE-TRIAGE` (12,402 candidates by kind
and library, the top 30 read by hand), NULL-SAFETY-PROPERTY's report and its
`candidates.json.gz`, C09.3, and the user decisions "move the NULL question",
`quant null-deref #B` and "handle as safe as possible". The bulk is shrunk
only by checked knowledge; nothing is suppressed.

## 1. What changed

| Piece | Where | Rule |
| --- | --- | --- |
| Path search of strong-source candidates: one path at a time from entry, branch decisions admitted only when decided or over unconstrained values (intervals of values compared with constants, enumeration constants, case labels, pointer identities), a correlated condition unresolved | `crates/qcore-c-gateway/src/null_deref/witness.rs`; decisions `qcore_core::null_triage::admit` | 14r |
| Findings: `finding` check outcome, provisional `Unsafe` row, `null-on-allocation-failure` / `null-on-normal-path`, the row's `triage` list | `null_deref.rs` (`TriageRecord`), `qcore_core::null_triage::triaged_verdict` | 14s |
| `requires null_member_nonnull(TAG, FIELD);` read non-null; every store recorded (`member_stores`), initializer lists included; record tags through typedefs | `bounds_contract.rs` (`BoundsClause::NullMember`), `null_deref/walk.rs`, `null_deref/ast.rs` | 14t |
| `ensures normal null_result_nonnull();` | `bounds_contract.rs` (`BoundsClause::NullResult`), `walk.rs` | 14u |
| Unit exports (`definitions`) and external-call actuals (`external_calls`); cross-unit clause and result travel | `null_deref.rs`, `walk.rs`, `crates/qcore-audit/src/null_fold.rs` | 14v |
| The run fold: standings, withdrawal, travel, recomputed verdicts and premise standings, the summary's `fold` section | `null_fold.rs`, `report.rs` (`null_evidence`), `projection.rs` | 14w |
| `null_deref_beside` admits the member and result clauses; two relations registered | `qcore-contract/src/contract_views.rs` | 14t, 14u |
| Declarations (librdkafka): four member hosts (five members), one non-null result, seven `quant null-deref` checks, pinned at both revisions | `providers/qcore-provider-librdkafka/contracts/librdkafka/null_deref.qv`, `bounds-contracts.lock` | 14o, 14t, 14u |

**Vocabulary placement.** The member and result clauses live in bounds
contract v1, the null property's environment, for the reason rule 14o gives
for `quant null-deref`: resource v1 feeds custody's environment digest and
certificates. A member declaration is type-level; it is hosted on the
record's constructor (the declaration form takes an `extern fn`). The brief
named `resource_member_nonnull(T, f)` "or an equivalent": `null_member_nonnull`
is that equivalent in the null property's own namespace (the `resource_*`
words belong to the resource loader, which rejects the module).

## 2. Claims reproduced and corrected

* BEFORE (`b0`, binary `01c2dc82…`) reproduces the brief: corpus **915 / 9 /
  3,910** (`ded14632…`), null **1,219 / 1,442 / 2,260 / 0 Unsafe**, 12,402
  candidates in 2,173 functions, by kind and library as the ledger states;
  system 91 / 4 / 353, slice 50 / 1 / 349 / 4 Pending; 12 custody Safe rows
  with an own premise, 1 with every premise proved.
* The top 30's hand reading reproduces: **24** of its 27 real-looking
  entries are findings below; the other 3 stay unresolved (`mk_plugin_load`,
  `mk_utils_url_decode:481`, `mk_vhost_read:540`: a condition over a value
  the search does not follow). Of its 3 guarded entries, 2 stay candidates
  and 1 (`copy_label_values`) is a finding the reading contradicts (section 3). **Correction:** the brief
  counts "133 candidates, plus any uninitialized or global"; the strong-source
  set is **139** (allocation 37, lookup 1, null-literal 95, uninitialized 5,
  global 1).
* `rd_kafka_buf_s.rkbuf_rkb` (3,474 candidates) cannot be declared:
  `rdkafka_broker.c` resets it to NULL after a response is handled.
  `rd_kafka_broker_s.rkb_rk` (682) was declared and **withdrawn by the run's
  store check** (run `a3`): `rd_kafka_broker_find_by_nodeid0_fl` builds a
  lookup key `{.rkb_nodeid = …}` whose `rkb_rk` is NULL, and three unit tests
  build brokers without it; the declaration was dropped (its pin removed by
  regenerating the lock, `scripts/regen.sh`). `struct mk_list`'s `next` (595)
  is NULLed by `mk_list_del`, so it was not declared.

## 3. Findings (task 1)

[findings.json](artifacts/findings.json) (each with file, line, function,
class, NULL condition, path with original lines, and excerpts from 66910c10),
[triage-corpus.json](artifacts/triage-corpus.json) (every searched candidate).

| Corpus, 139 strong-source candidates | |
| --- | ---: |
| finding, `null-on-allocation-failure` | **30** |
| finding, `null-on-normal-path` | **3** |
| infeasible (every path explored) | 2 |
| unresolved: a condition over a value the search does not follow | 47 |
| unresolved: a correlated condition | 32 |
| unresolved: a loop's iteration bound | 14 |
| unresolved: switch, nested case label, ordered pointer comparison, correlated test | 11 |
| indeterminate | 0 |

The 33 findings are in **22 functions** (null `Unsafe` 0 → 22): ctraces'
OTLP encoder (`set_event`, `set_events_from_ctr`, `otel_span_set_status`,
`otel_span_set_links` ×4, and `ctr_variant_binary_to_otlp_any_value`, which
sets `result = NULL` when the `calloc` of its bytes fails and then `memcpy`s
through it), `rd_kafka_brokers_learned_ids`' `malloc`, 19 unchecked Monkey
allocations (`mk_mem_alloc`/`_z`, `mk_string_dup`), `mk_print`'s
`localtime_r` (a lookup that misses), `rd_kafka_share_fetch_reply_op`
(`reply_rkb` tested in the debug line at `rdkafka.c:3465`, dereferenced
unconditionally at `:3501`), and `copy_label_values`.

**Read by hand:** 32 real-looking; 1 rests on a callee result the callee never
returns: `copy_label_values`' path needs `cfl_list_size()` to be negative
(`s == 0` false, `s > 0` false), which a count never is; the search reads an
uncontracted call's integer result as unconstrained. It stays a provisional
finding (the rule cannot tell), named here. None is presented as an upstream
defect; each is for review.

The two infeasible ones: `rd_kafka_mock_pid_new` and `mk_start` (every path
explored, the pointer is non-null or only possibly null through memory at the
dereference). System and slice carry no strong-source candidate that the
search validates (system has none; slice's one null-literal candidate is
unresolved).

## 4. Checked non-NULL knowledge (task 2)

Run fold, corpus (`artifacts/null-summary-after-corpus.json`, `fold`):

| Declaration | Standing | Stores examined | Not proved |
| --- | --- | ---: | ---: |
| `rd_kafka_toppar_s.rktp_rkt` | partly-validated | 2 | 0 |
| `rd_kafka_topic_s.rkt_rk` | partly-validated | 2 | 0 |
| `rd_kafka_topic_s.rkt_topic` | partly-validated | 2 | 0 |
| `rd_kafka_cgrp_s.rkcg_rk` | partly-validated | 1 | 0 |
| `rd_kafka_transport_s.rktrans_rkb` | partly-validated | 1 | 0 |
| (`rd_kafka_broker_s.rkb_rk`, run `a3` only) | withdrawn | 6 | 4 |
| `rd_kafka_topic_keep` non-null result | declared-assumption (64 bodies examined, 0 validated, 0 refuted) | | |

Each member's `stores` clause is covered; `construction` stays a premise, so
none is `validated` ("handle as safe as possible"). `rd_kafka_topic_keep`'s
body returns `rkt` after `rd_kafka_rkt_get_lw(rkt)` reads it; the walk does not
prove that return non-null, so the result declaration is a declared
assumption (it makes `rktp->rktp_rkt = rd_kafka_topic_keep(rkt)` a proved
store). Memory-field candidates on the functions' own checks: **9,607 →
8,605** (−1,002). The circular-list, preallocated-element and
counter-correlated invariants have no sound checked reading yet: residuals
below.

**Custody premises.** The custody report is byte-identical. Premise checks
the null property proves: 676 → **782** (candidates 1,000 → 892; 2 now
findings). Custody Safe rows with an own premise: 12, every premise proved 1
→ **1**: none of the 31 premise-carrying rows is discharged by the new
declarations (their premises read `s->iov`, `i->storage` and list elements,
which no declaration covers).

## 5. Cross-unit travel and declarations (task 3)

Corpus: 2,590 external definitions exported, 3 ambiguous (two units define
the name). **462** candidates proved by a cross-unit non-null result (all
sole-source `uncontracted-result` candidates the base counted:
`uncontracted-result` 738 → 277 on the functions' own checks). **6,125**
caller checks added by travelled clauses (3,340 proved, 883 moved, **1,902
candidates**) in 1,364 rows. Seven `quant null-deref` declarations
(`rd_kafka_flush`, `rd_kafka_message_errstr`, `rd_kafka_queue_poll`,
`rd_kafka_event_error`, `rd_kafka_outq_len`, `rd_kafka_op_destroy`,
`rd_kafka_topic_name`; ranked by NULL-SAFETY-PROPERTY's clause ranking, each
body byte-identical at both revisions and dereferencing the formal first;
every defining body the corpus analyses checks `necessary`: 16 rows, none stronger than necessary).
System: 129 ambiguous names (each unit is analysed in two configurations, so
every definition is defined twice), 3 exported, 2 caller checks added (proved).
Slice: 68 exported, 83 caller checks (48 proved, 26 moved, 9 candidates).

## 6. Measurement

**Custody**: byte-identical on all three routes with and without the change
(corpus `ded14632…`, system `00ed1293…`, slice `f6ecc830…`); the parity
streams are multiset-identical (corpus 690,246 rows, system 142,512, slice
51,342; their order differs between runs with four jobs):
[parity-corpus.json](artifacts/parity-corpus.json), [-system](artifacts/parity-system.json),
[-slice](artifacts/parity-slice.json).

**Null property** (BEFORE `b0` → AFTER `a4`, binary `81c61ca8…`;
[compare-corpus.json](artifacts/compare-corpus.json) attributes every moving row):

| | corpus | system | slice |
| --- | --- | --- | --- |
| Safe unconditional | 1,219 → **1,115** | 84 → 84 | 72 → **71** |
| Safe conditional | 1,442 → **1,421** | 125 → 125 | 103 → 103 |
| Unknown | 2,260 → **2,363** | 245 → 245 | 231 → **232** |
| Unsafe (provisional) | 0 → **22** | 0 → 0 | 0 → 0 |
| candidates | 12,402 → **12,809** (own 10,907 + travelled 1,902) | 559 → 559 | 1,101 → 1,110 |
| findings | 0 → **33** | 0 | 0 |

Corpus moves (rows; a row may carry several causes): Unknown → Unsafe **22**
(findings); Unknown → Safe **87** (66 by a declared member, 25 by a
cross-unit result, 38 with travelled checks all discharged, 7 by a new
declared clause); **Safe → Unknown 212, every one a named correction**: each
row now carries a travelled `call-precondition` check of an actual passed to a
function in another unit that dereferences it on every path (211), or a new
declared clause's check (16), whose actual the row does not prove non-null.
Before, nothing checked those calls. One Safe row moved from unconditional
to conditional (it defines a newly declared function). 650 of the 1,902 travelled candidates are
the caller's own untested formal: a second analysis would infer the caller's
clause instead (residual below). Slice: one Safe → Unknown, the same cause.
System: no move.

Candidates by strongest kind on the functions' own checks
([candidates-by-kind-after.json](artifacts/candidates-by-kind-after.json)
counts both): allocation 37 → 8 (29 findings), lookup 1 → 0, null-literal 95
→ 92, uncontracted-result 738 → 277, memory-field 9,607 → 8,605, formal
1,816 → 1,817, other 102, uninitialized 5, global 1. Travelled: memory-field
1,006, formal 650, uncontracted-result 200, null-literal 38, other 6,
allocation 2 (the travelled strong-source ones are not path-searched:
residual).

**Wall time** (inside the lock, null and parity selected): corpus 478.9 s →
481.7 s (+0.6 %), system 64.29 → 64.19 s, slice 29.45 → 29.41 s.

## 7. Prediction against outcome

| Predicted | Measured |
| --- | --- |
| Custody byte-identical | yes, all three |
| Premise rows fully proved 1 → 1–4 | 1 → 1 |
| Findings 35–80 | **33** (below: 104 unresolved, mostly conditions over values the search does not follow) |
| Allocation-failure 25–40 | 30 |
| Normal-path 10–45 | **3** (below) |
| Infeasible 5–35 | **2** (below) |
| Unresolved 20–80 | **104** (above) |
| Null Unsafe rows 25–60 | **22** (below) |
| rkbuf_rkb withdrawn | not declared (an explicit NULL reset); rkb_rk withdrawn by the check instead |
| Members kept 2–6 | 5 |
| Memory-field discharged 200–1,500 | 1,002 |
| Cross-unit results 300–470 | 462 |
| Travelled checks 1,000–6,000, candidates 200–2,000 | 6,125, 1,902 |
| Total candidates 10,000–14,000 | 12,809 |
| Null Safe 2,650–2,900 | **2,536** (below: travel's 212 corrections) |
| Wall +0–15 % | +0.6 % |

## 8. Lean

`QCore.NullTriage`, registered in `proof/QCore.lean` and audited by
`proof/AuditNullTriage.lean` (20 theorems; `propext` and `Quot.sound` at
most, no placeholder, no project axiom): the path model
(`a_validated_path_is_satisfiable`: a path whose decided steps agree and
whose free steps read distinct values has a valuation taking every arm;
`a_validated_finding_faults`, `a_repeated_free_value_is_not_validated`,
`a_correlated_condition_is_never_taken`, `a_decided_condition_takes_its_arm`,
`interval_path_is_satisfiable`), the verdict (`triage_never_makes_safe`,
`unsafe_only_for_a_violation_or_a_finding`, over `QCore.NullDeref.verdict`),
members (`member_invariant_holds`: with the construction premise and every
store proved, every read is non-null; `a_member_declaration_is_never_validated`,
`a_failed_store_withdraws`), results (`a_refuting_unit_withdraws`,
`validated_needs_a_validating_body`), withdrawal and travel
(`a_withdrawn_dependency_is_never_safe`, `travel_never_improves_a_row`).
Three vector lists bind the Rust (`crates/qcore-core/tests/null_triage.rs`,
parsed from the Lean text): `admitVectors`, `memberVectors`, `resultVectors`.

## 9. Controls

`crates/qcore-c-gateway/tests/null_candidate_triage.rs` (15): an unchecked
allocation is an allocation-failure finding with its decision and event; a
null constant reaching the dereference is a normal-path finding; a repeated
condition over unchanged values, and a flag set with the pointer, are decided
(infeasible); a condition over an expression of a formal and a memory value
retested after a call are unresolved; the function's own test falling through
and a lookup that misses are findings; an uninitialized pointer is
indeterminate; a formal without a test is never searched; a declared member is
read non-null and every store is evidence (an unproved formal, a proved
address), an initializer omitting it stores null, a member read through a
typedef names its struct; a declared result is non-null for callers and its
body exported; inferred clauses and external calls are exported; the loader
admits and refuses the new clauses and `for_bounds` drops them.
`qcore-audit`'s `null_fold` tests (3): a cross-unit result proves a
sole-source candidate only; clause travel adds a check and never improves a
row; an ambiguous definition exports nothing. `qcore-core/tests/null_triage.rs`
(3).

### Deliberate test and pin updates

* `null_safety_property.rs`'s unchecked-allocation control: the candidate is
  now a validated finding (`finding`, row `Unsafe`), its sources unchanged.
* The twelve `librdkafka/null_deref.qv` declarations:
  `qcore_contract_corpus.rs` (twelve pinned rows), `contract_loader.rs`
  (resource-v1 rejected declarations 234 → 246; modules unchanged at 130),
  `contract_view_census.rs` (pairs 18 → 20: `rd_kafka_toppar_new0`,
  `rd_kafka_topic_keep`), `qcore-contract`'s `contract_views.rs`
  (`null_deref_beside` 5 → 7), `RESERVED_SPELLINGS` 12 → 14.

## 10. Gate

See [gate/](gate/) and [gate.json](gate.json); filled in section 10a after the
run.

## 11. Follow-ups

* **Retired:** `NULL-CANDIDATE-TRIAGE` (the vocabulary exists and is measured;
  the strong-source candidates are triaged with their findings published for
  review; the rest are registered below with sizes).
* **Amended:** `NULL-CLAUSE-COVERAGE` (travel implemented; seven declarations
  added; left: floating and slot parameters in bounds v1, the function
  pointer at an indirect call, 235 calls in 185 functions),
  `NULL-CUSTODY-PREMISE-RESIDUALS` (re-measured: premise checks proved 782,
  rows fully proved 1 of 12).
* **Registered** (follow-ups 751 → 756): `NULL-TRIAGE-UNRESOLVED-PATHS` (104),
  `NULL-MEMBER-INVARIANT-SCOPE` (rkbuf_rkb 3,474, rkb_rk 682, mk_list next
  595: invariants that hold only on linked or received objects),
  `NULL-TRAVEL-CALLER-REINFERENCE` (650 travelled formal-only candidates in
  364 rows; the 212 corrections), `NULL-TRAVELLED-CHECK-TRIAGE` (40
  travelled strong-source candidates not searched), `NULL-RESULT-BODY-UNPROVED`
  (`rd_kafka_topic_keep`, 64 bodies, the walk does not prove its return),
  `NULL-SYSTEM-CONFIGURATION-AMBIGUITY` (129 system names defined in two
  configurations export nothing).

## Scripts

`scripts/`: `build.sh`, `measure.sh`, `measure-binary.sh`, `heavy.sh`,
`totals.py`, `candidates.py`, `triage.py`, `compare.py`, `parity.py`,
`artifacts.py`, `bodies.py`, `pin.sh`, `regen.sh`, `lean.sh`, `gate.sh`,
`provider-gate.sh`, `write_gate_json.py`, `finish-gate.sh`, `update_ledger.py`.
