# NULL-SAFETY-PROPERTY

Card **NULL-SAFETY-PROPERTY** (wave 48, 2026-10-10; card-high). Base: local
`main` at `0085fa346`. Prediction: [PREDICTION.md](PREDICTION.md), frozen at
`6e759268a` before any implementation code. Gateway rules
[14k](../../agents/gateway.md#constraint-14k)–[14p](../../agents/gateway.md#constraint-14p);
contract specification
[C09.3](../../QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref);
grammar [G05d](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks);
Lean `QCore.NullDeref` (`proof/AuditNullDeref.lean`).

The user's decisions of 2026-10-10 are recorded verbatim in C09.3 and in the
ledger entry `NULL-CUSTODY-PREMISE-RESIDUALS`: (1) move the NULL question to the
dominating dereference, which becomes a published check of a new null-safety
property, a custody verdict relying on the non-NULL reading listing a named
assumption; (2) the collocated clause `quant null-deref #B;`.

All four stages of the brief landed: the property with its sources, sinks,
guards and dominating dereferences; custody's premise reading; the clause; and
in-unit inference.

## 1. What changed

| Piece | Where | Rule |
| --- | --- | --- |
| The property `c.null.deref.v1`: verdicts, join, check, after-dereference, offset, refine, inference lattice, premise name | `crates/qcore-core/src/null_deref.rs` | 14k, 14m |
| The route-G consumer: every definition compacted as read; a forward, path-joining walk; loops to a fixed point; labels, switches, `goto`; checks at `*p`, `p->f`, `p[i]`, atomics, clause actuals | `crates/qcore-c-gateway/src/null_deref.rs`, `null_deref/{ast,walk}.rs` | 14k–14m |
| Sources and guards (nullable, lookup and unstated results, formals, memory, null on a path; NULL tests, early exits, no-return calls, `assert` as a listed premise, `errno`, `__builtin_alloca`, constant failing assertions) | `null_deref/walk.rs` | 14l |
| Custody's moved NULL question: a target-free, possibly-null borrowed alias read non-null at a dereference under `null-deref:F:LINE:VAR` | `legacy_null_premise.rs` (hook), `null_premise.rs` (the premises), `legacy_transfer.rs` (two call sites), `LegacyFunctionSummary::null_premises` | 14n |
| The premise in the dependency set and roll-up | `contract_dependencies.rs` (gateway and audit), `contract_consultation.rs` | 14n |
| `quant null-deref #B;` in bounds contract v1 only; `BoundsContracts::for_bounds()` keeps it from the bounds consumer | `qcore-frontend` (`QuantNullDeref`), `bounds_contract.rs` | 14o |
| In-unit inference and non-null results (callees first, recursion cut) | `null_deref.rs` | 14p |
| Audit selection `QCORE_NULL_PROPERTY` (rows, `.summary.json`, the custody premises' standing, the join; checkpoints refuse it) | `projection.rs`, `report.rs`, `null_tally.rs`, `bounds_selection.rs` (`null_environment`), `checkpoint_run.rs` | 14k |
| 13 declarations | `providers/{qcore-provider-librdkafka,qcore-provider-cmetrics,quirin-provider-fluent-bit}/contracts/*/null_deref.qv`, pinned in each crate's `bounds-contracts.lock` | 14o |

**Vocabulary choice (task 1).** The clause is admitted in **bounds contract v1
only**. It is a check form like `quant checks`, read by a separately selected
property; resource v1 declarations feed custody's environment digest and its
certificates, which the clause would change for a clause custody never reads;
the bounds vocabulary already has the check-form machinery (C06.2 pins in
`bounds-contracts.lock`, holds, the selection on the audit routes). The bounds
consumer reads the environment without the clause (a declaration and a module
left with none leave the bounds identity), so no bounds row moves.

## 2. Claims reproduced and corrected

* BEFORE reproduces the brief: corpus **903 / 9 / 3,922** (`24b59d52…`, CG22's
  a5 digest), system **89 / 4 / 355**, slice **45 / 1 / 354 / 4 Pending**.
* `CG10-INTERIOR-ALIAS-NULLABILITY`'s 7 functions: on the base **6** are still
  blocked only by unresolved identities (`scripts/alias_only.py`: 67 Unknown
  functions, 66 names, have only `unresolved resource identity` leaves).
  **Correction:** `rd_kafka_topic_partition_list_has_duplicates` now also
  carries an uncontracted `rd_kafka_topic_partition_list_sort_by_topic` (CG22
  withdrew the comparator reading) and two indirect accesses.
* Rule 14b's alias is the pointer read's (possibly null, unknown, no target):
  reproduced (`indirect_access_sole_residuals_r2.rs`'s control read
  `UnknownAlias { n }`; it now reads `Safe` under rule 14n, the test updated
  and the premise controls in `null_premise_custody.rs`). A *plain* pointer
  read of a never-bound root (`n = a->next; return n->a;`) was already `Safe`
  on the base with no binding at all (W43-INDIRECT-ACCESS-GENERAL's unbound
  reading): custody's older reading of unbound names lists no premise (residual
  below).
* The tip probe's base shape: the round-12 probe copy was gone from the
  scratchpad, so `scripts/tip-probe.sh` rebuilds it privately from the
  read-only corpus copy of Monkey plus the fix-branch's one line. On the base
  `mk_vhost_set` is Unknown with **1** leaf at the tip (the uncontracted
  `mk_vhost_set_property`) and 6 on master, not the 3 the round-12 brief lists:
  earlier cards removed the loop-condition and `va_arg` refusals.

## 3. Custody: the AFTER, attributed

Binary `private/nsp/bin/a4` (`a8c8cb38…`, built at `3563762ea`; the later
commits are formatting and clippy rewrites, no behaviour change). Every
route was run with the null selection and again without it (`a4-custody`):
the custody report is **byte-identical** either way on all three routes.

| Route | BEFORE | AFTER | Digest | Wall, custody only / with null (BEFORE) |
| --- | --- | --- | --- | --- |
| corpus 66910c10 | 903 / 9 / 3,922 | **915 / 9 / 3,910** | `24b59d52…` → `ded14632…` | 454.1 s / 477.5 s (453.3 s) |
| system | 89 / 4 / 355 | **91 / 4 / 353** | `bcfe6265…` → `f4f19c2d…` | 61.0 s / 64.9 s (60.3 s) |
| tail-pause slice | 45 / 1 / 354, 4 Pending | **50 / 1 / 349, 4 Pending** | `b2467434…` → `249aa27a…` | 27.0 s / 29.3 s (26.8 s) |

**Every move is Unknown → Safe and every one rests on its own listed null
premise** (`scripts/compare.py`, label `null-premise`;
[compare-corpus.json](artifacts/compare-corpus.json),
[-system](artifacts/compare-system.json), [-slice](artifacts/compare-slice.json)):

* corpus +12: `peek`, `sink_peek` (snappy.c), `rd_kafka_share_ack_type_from_msg_op`,
  `rd_kafka_topic_partition_list_regex_cnt` (4 of the 7 named), and
  `append_histogram_metric_value`, `append_summary_metric_value`,
  `flb_input_chunk_is_storage_overlimit`, `mk_iov_print`, `fifo_drop_msg`,
  `fifo_is_msg_ready`, `mk_sched_broadcast_signal`,
  `rd_kafka_assignor_topic_cmp`;
* system +2: `flb_input_chunk_is_storage_overlimit` (two configurations);
* slice +5: `flb_input_thread_collectors_signal_start`,
  `flb_input_thread_wait_until_is_ready` (two configurations each),
  `dormant_file_unchanged`.

**No Safe → non-Safe move, no correction; Unsafe 9 / 4 / 1 unchanged; no leaf
added anywhere.** Leaves removed (corpus, 287 moving rows): 592 unresolved
identities, 13 uncontracted calls, 2 indirect accesses, 1 branch condition;
system 107 and slice 97 unresolved identities (+1 branch condition). The three
named functions left: `append` (after `iov++` the binding is a stepped
derivation; decision 1 ends the moved question at a reassignment, and custody
reads nothing more), `mk_vhost_fdt_chain_lookup` (its remaining leaf is the
*return* of the unknown alias `hc`, an identity refusal of any unknown binding,
not a nullness one), `rd_kafka_topic_partition_list_has_duplicates` (other
blockers, section 2).

**Premises in the dependency set and roll-up** (the sidecar's
`contract_dependencies`): corpus Safe rows **399 on no declaration / 101 only
validated / 24 partly validated / 367 on a declared assumption / 24 only on a
null premise**, **31 carrying a null premise** (BEFORE 418 / 101 / 23 / 361).
The 12 new rows list their own premises; 19 rows Safe before now carry a
premise through an in-unit summary their derivation consulted (rule 13f's
over-approximation: the consultation log of a nested analysis is the
enclosing function's too); none of them moved. System 2 and slice 5 Safe rows
carry their own premises.

**The parity stream, in full** (`scripts/parity.py`, addresses erased):
corpus 690,513 → 690,246 rows, 521 only before, 254 only after, in 43
functions; every difference is in a moving function except 9 unit-level
`summary-premises-graph` rows and 3 `bound-untracked-root` census rows of
`rd_kafka_q_concat0` (an in-unit header summary with no row: `path_refused`
true → false, a dereference earlier on the path now read under a premise,
both answers unchanged). System 76 / 48 rows in 4 functions (4 unit rows
outside); slice 109 / 33 in 7 (3 unit rows and one `ml_flush_callback` census
row, the same `path_refused` change). [parity-corpus.json](artifacts/parity-corpus.json),
[-system](artifacts/parity-system.json), [-slice](artifacts/parity-slice.json).

**The `mk_vhost_set` tip probe** ([tip-probe.json](artifacts/tip-probe.json)):
custody rows identical before and after (tip: `mk_vhost_set` Unknown, 1 leaf;
`mk_vhost_set_property` Unknown, 1; master: 6 and 1). Null property:
`mk_vhost_set` `Safe` conditional on its inferred `quant null-deref #ctx` (its
`vh` is tested); `mk_vhost_set_property` Unknown with formal candidates on `vh`
(it dereferences `vh` on some paths only, so no clause is inferred).

## 4. The null property

Corpus (4,921 rows: the 4,834 projected and 87 pinned-build exclusions,
`null-import-failure`; [null-summary-corpus.json](artifacts/null-summary-corpus.json),
[census-corpus.json](artifacts/census-corpus.json)):

| | corpus | system | slice |
| --- | ---: | ---: | ---: |
| Safe, unconditional | 1,219 | 84 | 72 |
| Safe, conditional on own (declared or inferred) preconditions | 1,442 | 125 | 103 |
| Pending | 0 | 0 | 0 |
| Unknown (with a candidate) | 2,260 (2,173) | 245 (239) | 231 (229) |
| Unsafe | 0 | 0 | 0 |
| checks proved / moved / candidate / excluded | 55,865 / 16,602 / 12,402 / 135 | 3,337 / 1,492 / 1,005 / 4 | 3,828 / 898 / 1,270 / 0 |
| combined with custody: Safe / Unknown / Unsafe (/ Pending) | 819 / 4,093 / 9 | 61 / 389 / 4 | 29 / 374 / 1 / 2 |

Inference: 3,512 inferred preconditions on 2,931 corpus functions. No row is
`Unsafe`: the four definitely-null dereferences of an earlier run were
`TAILQ_INSERT_AFTER`'s `(elm->next = listelm->next) != NULL`, whose true arm the
walk did not prune when the tested value had no place; fixed before the
measurement (a contradicted arm is infeasible; an assignment tests its left
operand), with controls.

**Custody premises against the null rows** (the custody rows' own premises,
matched by line and variable; `nearby:` within three lines, never counted
proved): corpus 676 proved, 1,000 candidates, 41 nearby proved, 62 nearby
candidates, 2 excluded, 21 unmatched. **Of the 12 corpus custody Safe rows with
an own premise, 1 has every premise proved by the null property**; system 0 of
2; slice 1 of 5. The other premises read pointers from memory (`s->iov`,
`i->storage`, a list's elements), which the property cannot prove.

**Candidates, ranked** ([candidates.json.gz](artifacts/candidates.json.gz), 12,402
entries; [by kind and library](artifacts/candidates-by-kind.json)). By
strongest source kind: memory-field 9,607 (1,604 functions), formal 1,816 (920),
uncontracted-result 738 (415), other 102 (labels a later `goto` reaches),
null on a path 95 (55), allocation 37 (20), uninitialized 5, lookup 1, global
1. By library: librdkafka 10,225, cmetrics 1,028, monkey 565, ctraces 234,
chunkio 176, fluent-bit's chunk units 174. 235 calls through a function pointer
in 185 functions are counted, not checked.

**The top 30 read by hand** ([candidates-top30.json](artifacts/candidates-top30.json),
each with its reading): **27 real-looking, 3 guarded by something the walk
does not read, 0 infeasible.** The 27 are allocation results dereferenced
with no test, NULL only when the allocation fails (ctraces' OTLP encoder
`calloc`s, `rd_kafka_brokers_learned_ids`' `malloc`, 20 unchecked
`mk_mem_alloc`/`_z`/`mk_string_dup` results across monkey's server and core)
and `mk_print`'s `localtime_r`; they are candidates, not validated defects,
and none is presented as an upstream defect. The 3 guarded ones are a pointer
correlated with a counter (`rd_kafka_produce_batch`) or with an empty list
(`cmt_cat`'s two label copies). **A stratified sample of the bulk** (24
entries, six each of memory-field, formal, uncontracted-result and null on a
path; [candidates-sample.json](artifacts/candidates-sample.json)) is
**guarded by something unread in all 24**: structural invariants (a buffer's
broker, a list head, a broker's rk), caller preconditions the every-path
inference cannot state, counts or flags correlated with the pointer, and
callees that never return NULL without a contract saying so (rd_calloc-based
constructors, glibc's ctype table).

**Wall time.** The custody arbiter is unchanged (+0.2 % corpus, +1.2 % system,
+0.4 % slice); selecting the null property adds one AST dump and walk per unit:
+5.2 % corpus, +6.4 % system, +8.9 % slice over custody alone.

## 5. The clause and its contracts

13 declarations (bounds contract v1, `quant null-deref #B;`), each body read at
66910c10 and 046accf6 ([bodies.py](scripts/bodies.py); byte-identical except
`flb_output_set_property`, whose first statement reads `ins->config` at both),
chosen from the ranking of inferred clauses by the files that call them
outside their definition ([clause_rank.py](scripts/clause_rank.py),
[clause-rank-corpus.json](artifacts/clause-rank-corpus.json),
[-system](artifacts/clause-rank-system.json)):

* `qcore-provider-librdkafka`: `rd_kafka_destroy`, `rd_kafka_topic_partition_list_destroy`,
  `rd_kafka_topic_partition_list_add`, `rd_list_destroy`, `rd_list_add`,
  `rd_kafka_queue_destroy`, `rd_kafka_message_destroy`;
* `qcore-provider-cmetrics`: `cmt_destroy`;
* `quirin-provider-fluent-bit`: `flb_output_set_context`, `flb_output_net_default`,
  `flb_output_name`, `flb_output_set_property`, `flb_input_chunk_get_name`.

Pinned with `quirin-audit pin --bounds` at both revisions; `pin --bounds
--check` 0 at both for each crate and the whole tree; `pin --verify-contracts`
0 ([pin.sh](scripts/pin.sh)). Every body that defines one in a measured unit
checks **`necessary`** (corpus 9, system 10 rows), none stronger than
necessary. Their callers: corpus 741 clause checks (413 proved, 59 moved, 261
candidates, 8 excluded), system 125 (55 / 52 / 18), slice 4 proved. Left out
on measured grounds: the cmetrics setters (a `double` or `char **` parameter,
which G05b admits only with a closed frame), `mk_event_add` (its
`MK_HAVE_TRACE` configuration tests `data` first), `ctr_destroy` (tests `ctx`
first at 046accf6).

## 6. Prediction against outcome

| Predicted | Measured |
| --- | --- |
| 6 of the 7 named still alias-only; those 6 Unknown → Safe | 6 alias-only (correction: `has_duplicates`); **4** freed (`append`: reassignment; `mk_vhost_fdt_chain_lookup`: a return identity) — below |
| corpus +6 to +30, no Safe → non-Safe, Unsafe unchanged | **+12**, none, unchanged |
| Safe rows on a null premise under 40 | 31 carrying one (12 own, 19 transitively) |
| system +0 to +3 | +2 |
| slice: no status move | **+5** (miss: pointer reads such as `thi = ins->thi` in `flb_input_thread.c` are bound there as target-free, possibly-null aliases, the shape the reading covers) |
| null Safe 25–55 % | 54.1 % (2,661 of 4,921) |
| Unsafe 0–3 | 0 |
| candidates: thousands; memory first, formals, uncontracted, allocations last | 12,402; that order (allocations 37, null on a path 95) |
| premises discharged under half | 1 of 12 Safe rows |
| custody wall ±10 %; null selection +30 % to +100 % | +0.2 %; **+5.2 %** (cheaper than predicted) |

## 7. Lean

`QCore.NullDeref`, registered in `proof/QCore.lean` and audited by
`proof/AuditNullDeref.lean` (25 theorems, no placeholder, no project axiom:
`propext` and `Quot.sound` only). The walk over a path is sound against a
concrete semantics in which a NULL dereference faults (`walk_sound`); the
dominating dereference: after a non-faulting dereference and no assignment the
pointer's null-ness is the one that check judged (`moved_question_is_the_origins`,
`nullness_kept_without_assignment`); guards (`guard_sound`); the join
(`join_sound_left`, `_right`); the verdict (`a_safe_function_discharged_every_check`,
`no_safe_on_an_undecided_pointer`, `unsafe_only_for_a_reachable_violation`);
the clause's caller and callee obligations
(`caller_obligation_establishes_the_entry`); inference soundness
(`an_inferred_clause_is_necessary`: a path that dereferences the formal first
cannot complete with it null); custody composition
(`a_relied_on_reading_is_listed`, `a_tracked_binding_is_never_premised`,
`a_known_or_widened_binding_is_never_premised`, `combined_safe_needs_null_safe`).
Six vector lists bind the Rust: `joinVectors`, `checkVectors`,
`verdictVectors`, `firstVectors`, `inferredVectors`
(`crates/qcore-core/tests/null_deref.rs`, parsed from the Lean text) and
`premisedVectors` (`crates/qcore-c-gateway/src/null_premise.rs`).

## 8. Controls

`crates/qcore-c-gateway/tests/null_safety_property.rs` (16): the brief's ten
(unchecked allocation → candidate; checked allocation → proved; a NULL test then
a dereference, in nine spellings; a dereference then `&B->F` → moved; a
reassignment between → a new candidate; `sizeof(*p)` → no check; a clause
caller with an unproved actual → candidate; a clause body with a prior test →
`stronger-than-necessary`; inference on every path versus some; `assert` with
and without `NDEBUG`), plus a definitely-null dereference on the entry prefix
(`Unsafe`) and behind a branch (Unknown), sources, `T *const` formals, `errno`,
`_Generic`, an assignment's test, a placeless contradicted arm, copies, a
constant failing assertion and `__builtin_alloca`.
`tests/null_premise_custody.rs` (3): the premise is listed where an interior
alias is dereferenced (and one per line and variable), none where nothing is
read through it, and a literal NULL, a tracked allocation, a store through the
alias, a bound chain root and a release keep their decisions.
`qcore-core/tests/null_deref.rs` (5), `null_premise.rs` and `null_tally.rs`
unit tests.

### Deliberate test and pin updates

Each is commented at its site with this card's name; none turns a refusal
into `Safe` except under the named premise of rule 14n, which the summary
lists.

* Rule 14n moves (the possibly-null base `storage`/`s` read non-null under
  its premise): `borrowed_field_actuals.rs` (the partial-argument and
  parenthesized-actual rows certify resting on the premise on `storage`; the
  consumed-alias and released-second-read rows stay `Unknown`, refused by
  `alias` and `other`, with the premise listed; the later-reassignment row's
  `storage` refusal is its premise, the uncontracted `b_pick` still refuses);
  `clang_gap_phase2.rs` (`struct S *s = *v; s->in.x = 1;` certifies on the
  premise on `s`; the shared `support/clang_gap.rs` reports `null_premises`);
  `neighbours.rs` (before-state 20 -> 19, data tail 21 -> 20: the premise
  `null-deref:flb_input_chunk_create:206:storage` replaces `UnknownAlias
  storage`); `o04_constructor_caller.rs` (with an unresolvable member added to
  the read record the three cleanup arms discharge on the premise on
  `storage`, the function stays `Unknown`; the other three substitutions still
  withdraw every discharge).
* The three `null_deref.qv` modules: `contract_loader.rs` (modules 127 -> 130,
  resource-v1 rejected declarations 221 -> 234, selection and v1 declarations
  unchanged), `qcore_contract_corpus.rs` (corpus 127 -> 130, 13 pinned
  declarations of 2 clauses each), `bounds_contract.rs` (tracked bounds
  modules 2 -> 5), `contract_view_census.rs` and `qcore-contract`'s
  `contract_views.rs` (five functions declared in a resource view and in a
  `null_deref.qv` module, registered as the checked relation
  `null_deref_beside`: signature agreement and only null-deref clauses; pairs
  13 -> 18), `contract/profiles/cmetrics-counter/source-contracts.json` (the
  cmetrics manifest's digest, `25bd82ef` -> `2a38c6fe`),
  `quirin-provider-fluent-bit/src/lib.rs`'s contract index and
  `providers/tests/test_contract_load_path.py`'s bounds view expectation,
  `scripts/tests/test_contract_corpus.py` (canonical modules 127 -> 130:
  `qcore-provider-*` 78 -> 80, `quirin-provider-*` 49 -> 50) and the
  acceptance matrix's derived `contract_modules` (130) and `lean_audits`
  (308, `AuditNullDeref.lean`), regenerated before the docs steps.

## 9. Gate

Standing gate **g2** on `fa2d839f5` ([gate.sh](scripts/gate.sh) `g2`, every
step under the shared lock, exits in [gate/](gate/), logs
`private/nsp/gate-g2/`): `cargo check --workspace --all-targets` 0; submodule
0; `cargo fmt --check` 0; `clippy --locked --all-targets -D warnings` 0;
`run_test_binaries.py` **472 units, 485 summaries, 7,832 passed, 0 failed, 12
ignored**, exit 0 (`private/nsp/tests-gate-g2/`, 353.7 s inside the lock);
`reevaluation_probe` 0; `reevaluation_followup` 0; `check_lean_audits.py`
**308** audits passed; `cargo clean --profile dev` 0 (07z); the Python suite
and the docs index (08, 09), then `build_matrix.py . --gate gate.json` (09a),
the Python suite again (09b) and the docs index (10b, 10c), per
[gate/](gate/) and [gate.json](gate.json). Provider gate **g2** at the same
tip ([provider-gate.sh](scripts/provider-gate.sh), `private/nsp/provider-gate-g2/`):
`cargo check --locked` 0, `cargo test` 129 passed (exit 0), fmt 0, clippy 0,
`contracts.py check` 0, package build 0, package verify of all five packages
0, providers Python suite 25 tests OK. Clang: `gate/clang-version.txt`.

Earlier attempt, retained and not evidence for the tip: g1 on `498068eb3`
(logs `private/nsp/gate-g1/`, `private/nsp/tests-gate-g1/`): every step 0
except the suite, 7,817 passed and **15 failed**, each a pin or control this
card moves deliberately and had not yet updated (the list and the reason for
each are under "Deliberate test and pin updates", section 8); provider gate
g1 failed `quirin-provider-fluent-bit`'s contract index test and the load-path
manifest test for the same reason. Both are fixed in `fa2d839f5`. The
first docs part of g2 (on `68be7cce7`) failed the Python suite twice for
the same reason (the canonical corpus pin and the matrix's derived audit
count), both updated before the recorded docs steps.

## 10. Follow-ups

* Retired: **`CG10-INTERIOR-ALIAS-NULLABILITY`** (decision 1 taken and
  implemented; the three functions it does not free are named in
  `NULL-CUSTODY-PREMISE-RESIDUALS`).
* Registered (follow-ups 746 → 748):
  * **`NULL-CUSTODY-PREMISE-RESIDUALS`** (the user decision verbatim; `append`,
    `mk_vhost_fdt_chain_lookup`, `has_duplicates`; custody's unlisted reading of
    unbound names: 37 custody Safe rows have a formal-sourced null candidate);
  * **`NULL-CANDIDATE-TRIAGE`** (12,402 candidates in 2,173 functions, by kind
    and library; a vocabulary for non-null fields and results);
  * **`NULL-CLAUSE-COVERAGE`** (inferred clauses stay in their unit; the
    undeclarable cmetrics setters; 235 indirect calls unchecked).

## Scripts

`scripts/`: `build.sh`, `measure.sh` (`NULL=1`, `PARITY=0`), `measure-binary.sh`,
`heavy.sh`, `totals.py`, `alias_only.py`, `compare.py`, `parity.py`,
`null_census.py`, `candidates.py`, `stratified.py`, `show_sites.py`,
`clause_rank.py`, `bodies.py`, `pin.sh`, `tip-probe.sh`, `artifacts.py`,
`lean.sh`, `gate.sh`, `provider-gate.sh`, `write_gate_json.py`,
`finish-gate.sh`, `update_ledger.py`.
