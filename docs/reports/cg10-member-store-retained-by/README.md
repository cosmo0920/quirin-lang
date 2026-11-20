# CG10-MEMBER-STORE-RETAINED-BY: a formal's pointer stored into a member is `retained_by` (rule 70)

Card CG10-MEMBER-STORE-RETAINED-BY. USER DECISION 2026-11-20 on round 3's
decision class 1, question 2, verbatim: "Read it as retention, option 1 with
creating the relevant proof(s)." Base `0111ddf38` (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
round 3's card tip, which merged main `b7f86a0de`; it lands as integrate/r41).
Gateway [rule 70](../../agents/gateway.md#constraint-70); rule 10v's text
cross-references it; contract specification §C04.14.4 added (no grammar).
Lean `QCore.MemberStoreRetention` (21 audited theorems).
[PREDICTION.md](PREDICTION.md) was committed (`75bd9f8b3`) before the first
implementation commit.

## 1. Census (method step 1)

Round 3's tip census (`premises.jsonl.gz` of the opt-in `current
--monkey-linked` run over Fluent Bit 046accf6), read with round 3's own
classifier by [scripts/pointer_name_rows.py](scripts/pointer_name_rows.py):
19 sole-blocker functions hold a `pointer-name-into-member` leaf, 13 blocked
by it alone ([census/pointer-name-rows.jsonl](census/pointer-name-rows.jsonl);
the per-row table, with what is stored, into which holder, formal, local or
function pointer, and the other blockers, is [PREDICTION.md](PREDICTION.md)
section 1). By kind: **formal into a member of a formal's object, nothing
else blocking** 4 (`cio_set_log_callback`, `suffix_object`,
`rd_kafka_buf_set_maker`, `rd_kafka_mock_partition_set_leader0`); **a local
holding the body's own allocation** into a formal's member 6 (the cmetrics
`create_*_instance`: a custody move, not a retention); **a formal into a
member of the body's own tracked result** 2 alone + 2 mixed
(`PartitionMemberInfo_new`, `rd_kafka_op_new_cb`;
`rd_kafka_interceptor_method_new`, `rd_kafka_member_assigned_partitions_pair_new`);
**a formal into a record returned by value** 1 + 1
(`rd_kafka_ListOffsetRequest_parameters_make`, `pb_ostream_from_buffer`);
three with other blockers (`rd_kafka_msgq_split`, `set_attributes`,
`mk_clock_headers_preset`). Function pointers: `rd_kafka_op_new_cb`'s `cb`,
`rd_kafka_buf_set_maker`'s `make_cb` (`T *` for a function typedef `T`) and
`free_make_opaque_cb` (already admitted by rule 10v); `cio_set_log_callback`'s
`log_cb` is declared `void (*log_cb)`, a parenthesised **object** pointer
stored into a function-pointer member.

## 2. The reading (rule 70) and how it relates to rule 10v and `retained_by`

**No new vocabulary.** The contract specification already has the notion:
§C04.14 `resource_retained_by(P, H)` (USER DECISION 2026-10-05, the caller
keeps custody and must show `P` outlives the holder, never `Safe` otherwise)
and §C04.14.3's holder path `FORMAL.FIELD[.FIELD]` (USER DECISION 2026-10-16),
decided at a call by rule 12t's table. Rule 70 **derives** that clause from a
body instead of reading it from a declaration:

* **The store.** `F->m[.n…] = q` on rule 10v's typed statement route, with
  `F`, `q` distinct formals never written, incremented or address-taken, `q`
  an object pointer, the member in `*F` (first step `->`, then `.` only), and
  on the flow `F` and the member's object untracked, no binding rooted at `F`
  carrying a target or unknown, `q` bound to no identity: admitted with no
  custody moved (`member_store_retention::retained_formal`, `decide`). A
  failing premise is rule 10v's fallback, which rule 38i names as before.
* **Rule 10v's missing owner premise (Q36).** Rule 10v recorded that example
  C (`s->p = q`) needs "an owner identity for `*s` that bounds `*q`'s
  lifetime, or a derived Stored summary mode". Rule 70 supplies the second
  half exactly: the holder `s.p` is named in the summary and the bound on
  `*q`'s lifetime becomes the caller's obligation, decided where the caller
  can see both objects. It does not bypass the premise: a body whose summary
  cannot be published leaves every caller uncontracted (`Unknown`).
* **The summary.** Rule 10v's summary gate (`has_pointer_store`) is relaxed
  only while rule 70 validates a body with at least one rule-70 store whose
  every admitted pointer store is into a member of a formal; the validated W4
  contract must be every formal `Borrowed` and a `None` result. The summary
  is a `RetainingContract` with one `path_retentions` entry per rule-70 store
  and one `overwritten_paths` entry (new schema field, never set by the
  loader) per admitted store into a formal's member, installed by
  `insert_retaining`: withheld from the function map, a declared contract
  never replaced, never crossing a unit.
* **The callers' two obligations.** At each call route G (rule 12t) decides
  each retention: a static or null actual records nothing; a caller's object
  or tracked identity is recorded, so a release of it while the holder keeps
  it refuses (`retention-released-while-held`, *must not release while held*)
  and an exit with the record standing refuses (`retention-undecided`, *the
  holder must not outlive it*: a caller-supplied holder outlives the frame).
  Every overwritten path must be rule 12t's path holder of the caller's own
  unrebound, untracked formal, with no binding rooted there that carries a
  target (P-CG11-CALLER-SLOT carried up one frame); otherwise the call refuses
  whatever was stored. P-HOLDER-PATH-STABLE frames the call's own root actual
  and another derived call's disjoint ones (`member_store_retention::framed`).
* **Function pointers** (the subclass): a value rule 10v classes as a
  function pointer carries no lifetime and adds only an overwritten path.

## 3. Proofs (`proof/QCore/MemberStoreRetention.lean`, `proof/AuditMemberStoreRetention.lean`)

Built on `QCore.RetentionPath` (rule 12t's table and records) and
`QCore.PointerStore` (rule 10v's table). Every theorem is audited (`#print
axioms`), no placeholder, no project axiom.

* Flow decision: `decision_vectors_hold` (bound to `DECISION_VECTORS` by
  `rule_70_lean_vectors_match_the_rust_table`), `admitted_iff_untracked_and_carrying_nothing`,
  `rule_10v_refuses_every_formal_value`.
* Retention as an obligation on the summary: `a_call_takes_no_custody`,
  `a_summary_call_states_its_retention`, `a_callers_object_is_recorded`,
  `a_tracked_object_is_held`, `an_undecided_actual_refuses`,
  `an_unnamed_overwrite_refuses`.
* Soundness over a concrete memory whose holders keep pointers (`exec` faults
  on a read through a holder of a released object and on a double release,
  also by the holder): `step_sound`, `checked_traces_do_not_fault`,
  `an_admitted_caller_never_faults`.
* Caller discharge composes: `checkAll_append`, `caller_discharge_composes`,
  `summary_calls_compose`.
* Counterexample: `releasing_the_retained_pointer_is_refused`,
  `the_refused_trace_faults_when_the_holder_reads`; and the discharge
  `a_reset_before_the_release_discharges_it`; the exit:
  `a_caller_that_cannot_discharge_is_refused`.
* Function-pointer subclass: `a_function_pointer_adds_no_obligation`,
  `a_static_store_adds_no_obligation`.

The model's boundary: the Clang reading (`F->m = q`, never rebound, the member
in `*F`) and the CFG are the gateway's (trust boundary 28); the model proves
the checker's discipline sound, not that the gateway implements it (the Rust
decision table is bound by shared vectors; the call side reuses rule 12t's
bound tables).

## 4. Fixtures

`crates/qcore-c-gateway/tests/member_store_retention.rs` (6 tests): the four
census shapes admitted (example C, a `.` member, `suffix_object`'s pair, a
retention beside a function pointer and a scalar); refusals kept for a rebound
or address-taken formal, a rebound holder root, a deeper or element place, a
local's owned pointer, a tracked owner, and a body releasing the retained
formal; callers: a static, null or function-designator actual at the caller's
own formal path discharges (`Safe`), a caller's own formal retained keeps the
record to its exit (`retention-undecided`), a tracked object released while
held (`retention-released-while-held`), a holder in the caller's own
allocation or a rebound formal (`retention-undecided`); a body with only rule
10v's stores still publishes no summary. Unit tests: `rule_70_decision_vectors_hold`,
`rule_70_lean_vectors_match_the_rust_table`, `rule_70_admits_only_an_untracked_owner_and_an_identity_free_value`.
**Pins flipped deliberately** (each keeps a rebound-formal control with the
old refusal): `cg11_pointer_stores` (example C and the `formal-value` vector
row at route level), `clang_gap_phase2::m02_…`, `access_path_statements::pointer_store_and_union_still_refuse`,
`indirect_scalar_access::a_pointer_member_store_stays_refused`,
`member_effect_summary` (`setname`), `sub_object_stores` (`h->name = (char *) s`),
`typed_field_access` (`pointer field`), and two `typed_access_path` unit rows.

## 5. Measurement (method step 5)

Opt-in `quirin-audit current --monkey-linked` over Fluent Bit 046accf6,
exactly as round 3 (`openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`),
prepared inputs of round 3's worktree (`private/fb-046accf6`, read-only), one
side tree (`git archive 0111ddf38`) for every run, the binary the only
difference; release `indirect-census` builds. Evidence: [evidence/](evidence/).

| run | binary | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---|---:|---:|---:|
| base `0111ddf38` | `99ed0224…` | 1,140 / 2 / 3,655 / 7 | 2,190 | 240 | 9,149 |
| tip1 `1e746f9ba` | `55198bdd…` | 1,150 / 2 / 3,645 / 7 | | | |
| **tip2 `a07d6e8b5`** | `93b45b67…` | **1,150 / 2 / 3,645 / 7** | **2,184** | **239** | **9,125** |

Every run exit 0 ([evidence/runs.md](evidence/runs.md)); the base reproduces
round 3's tip exactly. tip2 differs from tip1 only by keeping rule 38i's name
on a rule-70 store whose flow premise fails (tip1 had replaced 18 such leaves
by the gate's text in 11 rows; no status moved between them,
[compare-tip1-tip2.json](evidence/compare-tip1-tip2.json)). Later commits
change tests, fixtures and documents only.

**Ten moves, every one Unknown -> Safe, each the callee whose body holds the
store** ([evidence/changed-rows-base-tip2.txt](evidence/changed-rows-base-tip2.txt)):
`suffix_object`, `rd_kafka_buf_set_maker`, `rd_kafka_mock_partition_set_leader0`
(whose only leaves were the access refusal) and `pb_field_iter_begin`,
`otel_span_set_name`, `otel_span_set_trace_state`, `rd_kafka_confval_disable`,
`rd_kafka_confval_init_int`, `init_snappy_decompressor`, `mk_ptr_set` (whose
only leaves were rule 38i's `member-store-through-formal` name of the same
refusal, which round 3's access census did not count; `pb_field_iter_begin`
had one of each). Safe rows on a declared assumption 524 -> 527 (three moved rows rest on existing declarations; no declaration is
introduced by this card). **No Safe -> non-Safe move, no new Unsafe, no Pending moved,
no caller moved.**

**Leaves** (48 rows change; 38 stay Unknown): removed 28 access refusals, 27
rule-38i store names, 12 `call to uncontracted function` (the ten callees now
have a summary), 10 `unresolved resource identity` (`mpart` 7, `msetw` 2,
`rkbmon` 1: re-attributions in rows whose stores are now read); added 8 route-G
refusals, the callers' obligations (`suffix_object` in the four
`cJSON_Create*Array` and `add_item_to_array`'s neighbours, `pb_field_iter_begin`
in `pb_field_iter_begin_const`, `rd_kafka_mock_partition_set_leader0` twice,
one an overwritten path the caller cannot name, `init_snappy_decompressor`),
and 4 access refusals that replace a `call to uncontracted function` leaf at a
call whose actual is a member read (`set_span` x2, `add_item_to_array`,
`pb_field_iter_begin_extension`): the call now meets the gate before route G.
Every caller of a newly summarised function stays Unknown, as predicted.

## 6. Prediction against outcome

* **Moved as predicted:** `suffix_object`, `rd_kafka_buf_set_maker`,
  `rd_kafka_mock_partition_set_leader0` (predicted 3-4 of the census rows).
* **Missed:** `cio_set_log_callback` did not move: its formal is declared
  `void (*log_cb)` (an object pointer) and the member is
  `int (*log_cb)(struct cio_ctx *, …)`, so the store converts an object
  pointer to a function pointer (`BitCast`), a value class rule 10v does not
  read; it keeps the access refusal (follow-up
  CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS).
* **Not predicted:** six more callees moved (`pb_field_iter_begin`, the two
  ctraces `otel_span_set_*`, two `rd_kafka_confval_*`,
  `init_snappy_decompressor`, `mk_ptr_set`). They are the same shape, but rule
  38i had renamed their refusal `member-store-through-formal`, so round 3's
  access census (and so the prediction's census) did not list them.
* **Callers:** predicted to carry the obligation and stay Unknown; they do
  (eight route-G refusals, four re-attributions to the access gate). The
  0-2 possible caller moves did not occur.
* **Invariants:** no Safe -> non-Safe, no new Unsafe, no Pending moved, as
  predicted; access leaves 9,149 -> 9,125 (-24, inside the predicted 0-60
  beyond the moved rows' own five).

## 7. Pinned routes

`quirin-audit measure --declared` cannot run on this host (no prepared
pinned-route inputs or baselines). Prediction for the integrator, replacing
PREDICTION.md's block (`cio_set_log_callback` withdrawn, the six rule-38i rows
added):

```measure-prediction
route corpus
move suffix_object Unknown -> Safe
move rd_kafka_buf_set_maker Unknown -> Safe
move rd_kafka_mock_partition_set_leader0 Unknown -> Safe
move pb_field_iter_begin Unknown -> Safe
move otel_span_set_name Unknown -> Safe
move otel_span_set_trace_state Unknown -> Safe
move rd_kafka_confval_disable Unknown -> Safe
move rd_kafka_confval_init_int Unknown -> Safe
move init_snappy_decompressor Unknown -> Safe
move mk_ptr_set Unknown -> Safe
route system
none
route slice
none
```

Each where the route analyses the unit (corpus and twelve-home); no Safe ->
non-Safe move and no new Unsafe expected on any route.

## 8. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 0111ddf38 --out
private/gate-2` through `private/toolchain/env.sh` (Clang 21, Lean 4.35) on
`b6f561628` (every later commit is this record and README section only);
the diff reaches `qcore-c-gateway` and `qcore-schemas`, so the Rust suite is
the full one and Lean is `all`. **Exit 0, `gate_scope: 8 steps passed`**
(record [gate/gate-2.json](gate/gate-2.json), logs in the worktree's
`private/gate-2/`):

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 668 units, 713 summaries; 9,797 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 466 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 779 tests |

No flaky rerun was needed. Earlier attempt, not evidence: `private/gate-1`
(on `b44a5777a`) exit 1: `cargo fmt` (formatting only) and six gateway pins
the rule moves deliberately (section 4; fixed in `e9f68b7bc`), every other
step exit 0 (clippy, probes, Lean 466 audits, Python 779 tests). The measured
binary `93b45b67` is of `a07d6e8b5`; later commits change tests and
documents only.

**Amendments check.** `python3 scripts/apply_amendments.py
integration-amendments.json --check --profiles-base 0111ddf38` on this tree
exits **1** by design: CG10-MEMBER-STORE-RETAINED-BY is registered on main
during r41, not on this base. On a scratch copy of this tree
(`git archive HEAD`, worktree `private/m/amend-root`) with round 3's
amendments applied and the r41 entry registered (as the integrator does), the
same check exits **0** (`amended CG10-MEMBER-STORE-RETAINED-BY`, two
registrations). **Gate read census** (`gate_read_census.py record`) was not
re-recorded for the new test target `qcore-c-gateway::test::member_store_retention`:
it traces every gateway test executable under `strace`, and this host ran
two measurement runs and other cards' gates in parallel; left to the
integrator, as round 3's was.

## 9. What stays refused, and follow-ups

* **Partly discharged: CG10-MEMBER-STORE-RETAINED-BY is amended, not
  retired** ([integration-amendments.json](integration-amendments.json)): the
  formal-into-a-formal's-member half is read; the rest of the 13 + 6 rows is
  not this shape.
* **CG10-RESULT-HOLDER-RETAINED-BY** (new): a formal stored into a member of
  the body's own tracked result (`PartitionMemberInfo_new`,
  `rd_kafka_op_new_cb`, and two mixed rows) is §C04.14.2's
  `resource_retained_by(p, result)`; deriving it needs the tracked-owner
  member semantics rule 10v keeps refused (O04-B08's field link must not read
  a retained pointer as owned).
* **CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS** (new): `T *` for a function
  typedef `T` is read as an object pointer (retained), a formal declared
  `R (*f)(A)` makes the W4 parameter reader publish no summary, and an object
  pointer stored into a function-pointer member (`cio_set_log_callback`'s
  `void (*log_cb)`, a `BitCast`) is not a rule-70 value.
* Not registered again: the cmetrics constructors (a custody move into a
  caller's object, MEMBER-EFFECT-STORES), records returned by value, and a
  caller forwarding a retention into its own summary
  (FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME's T2: every caller that hands its own
  formals today keeps the record to its exit and stays `Unknown`).

**Files likely to conflict** (with CG10-EMBEDDED-LIST-BORROWED-CONTAINER,
rule 69, and r41): `crates/qcore-c-gateway/src/lib.rs` (one `mod` line),
`pointer_store.rs`, `legacy_pointer_store.rs`, `legacy_summary.rs` (gate and
one derivation arm), `legacy_retained_by.rs`, `legacy_member_custody_attribution.rs`,
`typed_access_path.rs` (two unit rows), `crates/qcore-schemas/src/retained_by.rs`
and the three `RetainingContract` literals (`legacy_reference_result.rs`,
`contract_clause_standing.rs`, `qcore-contract/src/contract_retained_by.rs`),
the flipped fixture files of section 4, `docs/agents/rules/10v.md`, the
generated `docs/agents/gateway.md` and `docs/agents/README.md`,
`proof/QCore.lean`, `docs/QCORE_CONTRACT_SPEC_r0_3.md` (§C04.14.4),
`docs/reports/INDEX.md`. None of `typed_field_access.rs`,
`residual_access_reading.rs` or `legacy_residual_access.rs` is touched.
