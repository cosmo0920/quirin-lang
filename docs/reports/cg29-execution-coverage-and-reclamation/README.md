# CG29-EXECUTION-COVERAGE-AND-RECLAMATION

Wave 48, round 11, High. Card CG29 of the
[Clang gap queue](../../plans/clang-gap-cards/README.md), gap entries W07 and
W10 (evidence Q33, Q34, Q46). The card adds two core models and one
production caller, the atomic-publication verdict (rule 11i). Gateway rule
[11z](../../agents/gateway.md#constraint-11z).

Lean:
- `QCore.ExecutionCoverage` (new,
  [`proof/QCore/ExecutionCoverage.lean`](../../../proof/QCore/ExecutionCoverage.lean),
  audited in `proof/AuditExecutionCoverage.lean`);
- `QCore.Reclamation` (new, audited in `proof/AuditReclamation.lean`);
- `QCore.WeakMemory` (a spawn event, additive);
- `QCore.AtomicSourceRoute` (the publication verdict).

Base `f6336c9d` (= `main` at launch). The card was rebased onto `main`
`a6adb483` (CG27, CG21 and CG30 landed), then onto `6cab1ad9` (CG16), before
the AFTER and the gate.

**Published guarantees (phase 4).** Both are reported apart from custody
coverage. Projection coverage is not safety coverage.

1. **Race freedom over every candidate execution of a bounded, loop-free
   fragment (W07).**
   - *Standard*: C11 as published (N1570), each candidate checked by
     `check_c11_execution`.
   - *Scope*: the stated bounds (threads, static events, values per
     location, candidates), which are part of every verdict's evidence
     identity.
   - *Production caller*: the atomic-publication verdict, which now claims
     race freedom over every execution of its extracted shape instead of
     one graph.
2. **Reclamation safety of a checked hazard-pointer or epoch protocol trace
   (W10).**
   - *Property*: no access to a reclaimed object.
   - *Premises*: helpers are `Declared` boundaries; the trace is supplied.
   - *Consumer*: none in the corpus (§2).

## 1. What changed

### Execution coverage (`crates/qcore-core/src/execution_coverage.rs`, new)

**The fragment.** A `ProgramFragment` declares atomic locations with their
initial values. Each thread is a list of `GuardedInstruction`s. A guard is a
conjunction of register equalities, evaluated when the instruction is
reached. The instructions are:
- an atomic load, store, exchange or compare-exchange (with success and
  failure orders and strength);
- a fence (thread or signal);
- a non-atomic read or write;
- `Loop`, which is not enumerated.

A load or an exchange writes the value it read to its register. A
compare-exchange writes 1 on success and 0 on failure.

**The enumeration** (`check_program_executions`) produces every candidate
execution:
- a path per thread, for every read value in the location's domain and
  every outcome of a weak compare-exchange on its expected value;
- every reads-from source of equal location and value;
- every modification order (a permutation of the location's writes, encoded
  as successive pairs);
- every S order of the seq_cst operations and thread fences.

Synchronises-with is derived from these (`weak_memory::derived_synchronizes_with`,
the shared item this card adds, crate-visible). Thread 0 holds each
location's initialisation and one spawn per thread, so initialisation
happens before every thread, as in C.

Each candidate is checked by the unchanged `check_c11_execution`. A
candidate whose errors are at most data races is consistent.

**The value domain** of a location is its initial value and every literal
written to it by any instruction of any thread. Every write in the fragment
writes such a literal, and a read returns the value of the write it reads
from. So every consistent execution reads only domain values. The domain is
computed without control flow, so an out-of-thin-air reads-from cycle cannot
escape it. This is the reason arithmetic read-modify-writes are not in the
fragment: their values need the modification-order rank argument
(`CG29-ARITHMETIC-RMW-VALUE-DOMAIN`).

**The verdict.**
- `RaceFree`: every candidate was checked and no consistent one races.
- `Race`: a consistent candidate races. It is carried as the witness, and
  it is an execution of the fragment.
- `Unknown`, with a named reason:
  - a loop;
  - a malformed fragment: a location declared twice, an undeclared atomic
    location, or a location accessed both atomically and non-atomically
    (the checker races only non-atomic accesses against each other, so a
    mixed location would be half-checked);
  - an exceeded bound;
  - no consistent candidate.

`CoverageEvidence` records the standard, the bounds, the measured sizes and
each location's domain. `announce_validate_handshake` builds the
store-buffering handshake both reclamation protocols rest on.

### Reclamation (`crates/qcore-core/src/reclamation.rs`, new)

`check_hazard_pointer_trace` and `check_epoch_trace` check a trace of
protocol steps. Each object moves through fresh → linked → unlinked →
retired → reclaimed.

| Protocol | Rules |
|---|---|
| Hazard pointers | Protect a slot. Validate only while the object is linked. Access only through a validated hazard. Retire only once unlinked. Reclaim only when retired and no slot of any thread holds the object. |
| Epochs | Enter at the global epoch. Access a linked object, or one unlinked during the current critical section. Advance only when every active thread is in the current epoch. Reclaim two epochs after the retire. |

Neither checker tests "not reclaimed" at an access. That no accepted trace
accesses a reclaimed object is proved from the rules (§6).

`check_reclamation` applies a `ReclamationContract`, which maps each helper
operation to a protocol role. The contract is checked, never trusted:
- a duplicate operation, or a role of the other protocol, is refused, and so
  is an uncontracted call or a missing operand;
- every used helper is a `Declared` boundary on the certificate (proving its
  body is a separate obligation);
- the announce/validate handshake with the contract's declared ordering is
  decided by execution coverage:
  - `RaceFree`: the premise is discharged within `HANDSHAKE_BOUNDS`;
  - `Race`: the contract is refused (`HandshakeRaces`);
  - `Unknown`: the premise stays `Declared`.

A refused trace is `Unknown`, never `Unsafe`, because the trace is supplied
rather than derived from source.

### The atomic-publication verdict (rule 11i, amended)

- **Before.** `qcore_verification::atomic_publication_source` was `Safe`
  when extraction and the one-graph model accepted. Every verdict carried
  `SINGLE_GRAPH_ASSUMPTION`.
- **Now.** `Safe` also needs `publication_fragment`, the extracted shape, to
  be `RaceFree` within `PUBLICATION_COVERAGE_BOUNDS` (threads 2, events 7,
  values 2, candidates 16). The shape is: the flag initialised; the
  publisher's payload write and flag store; the consumer's load guarding its
  payload read. A `Safe` verdict carries `ALL_EXECUTIONS_ASSUMPTION` in place
  of `SINGLE_GRAPH_ASSUMPTION`. `Unknown` keeps the single-graph assumption.
  The certificate gains `execution_coverage`, and the refusal gains
  `ExecutionCoverage`.
- **Effect.** Every shape the extractor admits (release or seq_cst store,
  acquire or seq_cst load) is `RaceFree`, so no verdict changed status
  (§3). `quirin-audit atomic-publication` writes the new assumption.

### Shared foundations

The only shared items added are:
- `weak_memory::derived_synchronizes_with` (`pub(crate)`);
- `atomic_source_route::ALL_EXECUTIONS_ASSUMPTION`;
- the Lean `Kind.spawn` and `Execution.spawnEdges`.

`legacy_c.rs`, `typed_source.rs` and `elaboration.rs` are untouched.

## 2. Census

**Where program-level atomic or race claims would be needed: nowhere in the
measured routes.**
- *Corpus.* The corpus arbiter is a custody property. Its only atomic route
  rows are cmetrics' three wrappers (`cmt_atomic_load`, `cmt_atomic_store`,
  `cmt_atomic_compare_exchange`), as CG28 measured. Route G admits each as
  its custody effect and claims no ordering, so no verdict there asks for
  race freedom over executions.
- *System and slice.* 0 atomic leaves and 0 `atomic-route` parity rows
  (CG28's census, re-read on this card's runs).
- *Production caller.* The one route that makes an ordering claim is the
  atomic-publication verdict, which the arbiter does not run (0 functions).
  It now uses the enumerator.

**Hazard-pointer and epoch reclamation: none.** A search of the prepared
Fluent Bit source tree (the system route's checkout) for hazard pointers,
`hazptr`, epoch-based reclamation, QSBR, `urcu`, `rcu_read_lock`, `call_rcu`
and `synchronize_rcu` in `.c` and `.h` files finds only the eBPF
`vmlinux.h` type names (`call_rcu` twice, `rcu_read_lock` once, kernel
declarations, not a protocol). Atomics are used in 122 files, mostly vendored
libraries (jemalloc, cmetrics, WAMR, libbacktrace), but none implements a
deferred-reclamation protocol. The reclamation property therefore has **no
corpus consumer**, and its controls are synthetic, as the brief allows.

## 3. Cited claims, prediction and outcome

**Reproduced claims.**
- **Q34** (`weak_memory.rs`: "Acceptance checks one supplied graph, not
  program exploration"): reproduced by reading the base. CG00's control
  `w07_core_accepts_one_graph_not_the_program` passes unchanged. The program
  it describes has an accepted graph, and the enumerator now reports its
  race (`w07_the_program_level_verdict_is_not_the_one_graph`).
- **Q33** (`disjoint_footprint.rs` excludes `AtomicReclamation`):
  reproduced. CG00's `core_footprint_refuses_every_listed_construct` still
  refuses it, deliberately: the footprint prototype decides no reclamation.
  The protocols are a separate core model.
- **Q46** (`concurrency.rs`: an explicit event trace, not general C11
  executions): reproduced by reading. It is not changed.
- **W07 and W10** reproduced as stated, and no correction to the input.

The CG00 controls ran on the rebased tip. The code they test is unchanged
except for one additive function (`private/cg29/t1-clang-gap-core.log`,
`t2-clang-gap-core.log`).

**Prediction.** [PREDICTION.md](PREDICTION.md) was frozen and committed
(`ef82bea9` on the base, rebased as `4d694e71`), before any implementation code.

| Prediction | Outcome |
|---|---|
| Corpus BEFORE on `f6336c9d`: 633 / 10 / 4,191 | **633 / 10 / 4,191** (`9319bb9e…`) |
| Corpus AFTER identical to the BEFORE | **Held**: on the rebased tip the AFTER is byte-identical (`ceefb6c4…`, 645 / 10 / 4,179) to the integrator's measurement of `main` `6cab1ad9` without this card; the 12 moves from the BEFORE are CG21's (§4) |
| System and slice identical | **Held**: same totals, no move; the few changed leaves are other cards' landings (§4); 0 atomic rows in either |
| No Safe → non-Safe move, no added leak or violation | none |
| Publication fixtures keep their statuses; the enumerator answers `RaceFree` for every admitted shape | Kept: `Safe` stays `Safe` and every `Unknown` control stays `Unknown`. All four admitted order pairs are `RaceFree`, with 4 to 8 candidates. |
| MP release/acquire `RaceFree`; MP relaxed `Race`; SB relaxed `Race` with both loads reading initial values; SB with seq_cst fences or seq_cst accesses `RaceFree`; bounds and loops `Unknown` | As predicted. Also: release/acquire SB races, as C11 allows. |
| Reclamation controls | As predicted (§5) |

## 4. Measurement

The release `quirin-audit` ran with `QCORE_TYPED_PATH_PARITY` on, through
[`scripts/measure.sh`](scripts/measure.sh): the corpus arbiter
(`run_pinned_audit.py … 66910c10 --jobs 4`), the system route and the
tail-pause slice (`attempt-1`, run `cg29-NAME`). Heavy steps ran under the
coordinator's shared lock (`scripts/after.sh`). Every exit was 0.

| Run | Tree | Binary | Corpus | System | Tail-pause slice |
|---|---|---|---|---|---|
| BEFORE | base `f6336c9d` | `3fafc8fc…` | **633 / 10 / 4,191** `9319bb9e…` | 44 / 2 / 188 `c917a7a2…` | 29 / 1 / 239 / 1 P `aac9543f…` |
| AFTER | tip on `main` `6cab1ad9` | `f9dfbb1a…` | **645 / 10 / 4,179** `ceefb6c4…` | 44 / 2 / 188 `8dd9ba20…` | 29 / 1 / 239 / 1 P `1acc0f9d…` |

System and slice totals are deduplicated (`scripts/compare.py`). Their raw
digests carry path-dependent compile-identity fields, as CG28 and CG17
reported.

**Attribution** ([`scripts/comparisons.sh`](scripts/comparisons.sh),
[corpus](artifacts/corpus-comparison.json),
[system](artifacts/system-comparison.json),
[slice](artifacts/slice-comparison.json)).
- *Corpus.* The AFTER digest equals the integrator's measurement of `main`
  `6cab1ad9` (`ceefb6c4…`), the tree this card was rebased onto. This card
  therefore moves nothing in the corpus. The 12 Unknown → Safe moves from
  the BEFORE are CG21-CALL-EXITS-AND-BOUNDARIES' no-return calls
  (`__assert_fail`, `exit`, `abort`), the 12 moves the integrator
  attributed. The other leaf changes belong to CG21 (inline assembly,
  `__builtin_offsetof`), CG30 (C extension types) and CG27/CG16. There is no
  Safe → non-Safe move, so no correction.
- *System and slice.* No status move. The changed leaves are 5 functions
  (system) and 3 (slice): `__builtin_offsetof` and `exit` (CG21), inline
  assembly (CG21), and a vector type (CG30). None names an atomic, a fence,
  a fragment or a reclamation helper.
- *Parity streams.* 12 `atomic-route` rows in both corpus streams, and 0 in
  the system and slice streams. These are unchanged: route G is untouched.

**Published properties, measured separately.** No corpus, system or slice
route reaches the enumerator or the reclamation checker. Their results are
the tests' and the Lean vectors' (§5, §6), and the atomic-publication
fixtures'.

## 5. Tests

**Core, execution coverage** (`crates/qcore-core/tests/unit/execution_coverage.rs`,
6 tests):
- `lean_coverage_vectors_agree`: the 12 `coverageVectors` and their
  fragments, **parsed from `ExecutionCoverage.lean`**, give the Lean
  verdict, and the Lean candidate count where enumerated.
- `mp_release_acquire_is_race_free_over_all_executions`: the brief's MP
  control, with 4 candidates.
- `sb_relaxed_reports_the_race_in_some_execution`: the brief's SB control.
  The witness's loads read (1, 1) and (2, 0), the initial values, and the
  checker reports only the race. Also: release/acquire SB `Race`; fenced SB
  `RaceFree`; seq_cst SB `RaceFree` over 384 candidates;
  `announce_validate_handshake` equals the Lean `sbRelaxed` and `sbFences`.
- `exceeded_bounds_are_unknown`: the brief's bound control. Each bound, a
  loop, a mixed location, an undeclared and a duplicated location.
- `w07_the_program_level_verdict_is_not_the_one_graph`.
- `weak_compare_exchange_failures_are_enumerated`: 5 candidates weak, 4
  strong.

**Core, reclamation** (`tests/unit/reclamation.rs`, 4 tests):
- `lean_trace_vectors_agree`: 6 hazard-pointer and 5 epoch traces, **parsed
  from `Reclamation.lean`**, with the first refused step's index and
  violation equal to the Lean tables.
- `hazard_pointer_contract_is_checked_not_trusted`: the protected read is
  accepted, every helper `Declared`, and the handshake discharged by
  coverage. A relaxed unfenced handshake refuses the contract, and seq_cst
  accesses are accepted. A role of the other protocol, a duplicate, an
  uncontracted call and a missing operand are each refused.
- `hazard_pointer_use_after_retire_is_never_safe`: the brief's control.
  - An access with no validated hazard is `UnprotectedAccess`.
  - A hazard published after the unlink fails validation.
  - A reclaim while the hazard is held is `HazardHeld`.
  - An access after an admitted reclaim is `UnprotectedAccess`.
- `epoch_contract_waits_two_epochs`: a grace period, an early reclaim and a
  stalled advance.

**Publication route.**
- `qcore-verification` `cg19_publication_verdict_gates_on_the_composed_check`
  (amended): `Safe` carries the all-executions assumption and not the
  single-graph one; each `Unknown` control the reverse.
- New `cg29_extracted_shape_is_race_free_over_every_execution`, over the
  four admitted order pairs, each with its relaxed variant racing.
- `qcore-audit` `cg19_atomic_publication_route_reports_safe_and_unknown`
  (amended).
- The gateway unit `cg19_qualified_forms_and_single_graph_assumption`
  (amended).
- CT41 (`ct_corpus` `ct41_attributed_publication_case_and_controls`)
  unchanged and passing.

## 6. Lean

**`QCore.ExecutionCoverage`** (21 audited theorems).

| Theorem | What it states |
|---|---|
| `perm_mem_perms`, `mem_choices_iff` | Every permutation and every choice is enumerated. |
| `paths_complete`, `paths_sound` | A thread run with read values in the domain is generated, and every generated path is a run. |
| `run_write_literal`, `assemble_write_literal` | Every write writes a literal of its location. |
| `read_values_are_literals` | **Value-domain soundness**: in a candidate that passes reads-from validation, every read returns a domain value, for any choices. |
| `encode_mem_candidates` | **Enumeration completeness**: every candidate execution (any path choices, reads-from sources, modification orders as permutations, S as a permutation) that passes reads-from validation is generated. |
| `candidate_is_execution` | Every candidate is an execution of the fragment. |
| `verdict_race_free_sound`, `verdict_race_free_accepts` | **Verdict soundness**: `raceFree` means every consistent candidate execution, for any read values, is race-free and accepted by `check`. |
| `verdict_race_witnessed` | `race` has a consistent racing candidate. |
| `loop_is_unknown`, `race_free_needs_bounds`, `verdict_decided` | Refusals and bounds. |
| `check_nil_iff` | `check` accepts exactly a consistent, race-free graph. |
| `coverage_vectors_hold` | The 12 shared vectors, verdict and candidate count, by kernel `decide`. |

**`QCore.Reclamation`** (15 audited theorems).

| Theorem | What it states |
|---|---|
| `hp_step_preserves`, `hp_init_inv` | The hazard-pointer invariant: a validated hazard names a linked, unlinked or retired object; out-of-range slots stay empty. |
| `hp_access_safe`, `hp_accepted_safe`, `hp_trace_safe` | **Hazard-pointer safety**: no accepted trace accesses a reclaimed object. |
| `ebr_step_preserves`, `ebr_init_inv` | The epoch invariant: active threads lag at most one epoch, and a thread that may hold an unlinked object entered no later than its retirement. |
| `ebr_access_safe`, `ebr_accepted_safe`, `ebr_trace_safe` | **Epoch safety**. |
| `hp_vectors_hold`, `ebr_vectors_hold`, `hp_use_after_retire_refused` | The shared traces. |

**Also changed:**
- `QCore.WeakMemory`: `Kind.spawn` and spawn edges in happens-before, which
  is the Rust `spawn_join_edges`. The 20 litmus vectors and every theorem
  are unchanged.
- `QCore.AtomicSourceRoute`: `publicationVerdict checked covered`.
  `publication_verdict_carries_single_graph` gave way to
  `unknown_publication_carries_single_graph`,
  `safe_publication_covers_every_execution` and
  `uncovered_publication_is_unknown`. `safe_publication_only_when_checked`
  now also requires coverage.

**Axioms and placeholders:** none of either (§9).

**Premises by review.**
- The Rust-to-Lean correspondence of the enumerator and the protocol
  checkers. It is bound in both directions on the shared vectors, which the
  Rust tests parse from the Lean files.
- The checker's own premises (rule 11t).
- `check_c11_execution` as C11. Its mutex and join parts are unused by
  fragments.

## 7. Scope and axes

- **Covered:**
  - *Execution coverage*: two or more threads; loads, stores, exchanges,
    compare-exchanges (strong, weak with spurious failure); fences; every
    order except consume (refused upstream, rule 11t); guards on read
    values. Bounded by threads, events, values and candidates.
  - *Reclamation*: hazard pointers with per-thread slots, and epochs with a
    global counter and per-thread announcements.
- **Refused by name:**
  - *Execution coverage*: loops; mixed or undeclared locations; exceeded
    bounds.
  - *Reclamation*: protocol violations, foreign events and contract
    mistakes.
- **Not claimed:**
  - liveness or eventual observation (W11);
  - arithmetic RMWs;
  - mutexes and joins inside fragments;
  - any source route to fragments beyond the publication shape;
  - any source route to reclamation traces;
  - the bodies of reclamation helpers.
- **Route.** Core consumers, plus the atomic-publication production route
  (source profile `c11-bounded-atomic-publication-v1`). Routes G and T are
  unchanged.
- **Evaluation order.** None introduced. A thread's instructions are its
  program order, and the publication fragment is the extracted shape's
  sequenced statements.

## 8. Follow-ups

- **Retired: `CG29-EXECUTION-COVERAGE-AND-RECLAMATION`.** Every DISCHARGE
  clause holds:
  - the BEFORE reproduces the cited refusals (§3);
  - `PREDICTION.md` was frozen before the code;
  - the AFTER attributes every move;
  - no follow-up was absorbed;
  - there are Rust refusal controls and audited Lean with no placeholders or
    project axioms;
  - the §4 acceptance holds (axes §7, route named, no order introduced).
- **Registered** (0 corpus consumers each):
  - `CG29-ARITHMETIC-RMW-VALUE-DOMAIN`;
  - `CG29-SOURCE-FRAGMENT-EXTRACTION`;
  - `CG29-RECLAMATION-SOURCE-ROUTE`.
- **Amended:** `CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD`. Its one shape is
  now checked over every execution.
- **Ledger:** 678 → 680 on `main` `6cab1ad9`
  ([`scripts/update_ledger.py`](scripts/update_ledger.py), applied once).

## 9. Gate

Scripts:
- [`scripts/heavy.sh`](scripts/heavy.sh): [`after.sh`](scripts/after.sh),
  then [`gate.sh`](scripts/gate.sh), then `cargo clean --profile dev`,
  under the coordinator's shared lock;
- [`scripts/finish.sh`](scripts/finish.sh): the records.

Record [`gate.json`](gate.json), exits in [`gate/`](gate/), logs in
`private/cg29/gate` of this worktree. The **full standing gate** ran at tip
`90c3f071` on `main` `6cab1ad9`.

| Step | Command | Exit | Count |
|---|---|---:|---|
| 00 | `git submodule update --init --recursive`; `cargo check --workspace --all-targets` | 0; 0 | |
| 01 | `git submodule update --init --recursive` | 0 | |
| 02 | `cargo fmt --all -- --check` | 0 | |
| 03 | `python3 scripts/run_test_binaries.py` | 0 | 425 units, 438 summaries, **7,378 passed, 0 failed**, 11 ignored (`private/test-binaries/20261001T232524Z-2724640`) |
| 05 | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| 06 | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| 07 | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| 08 | `python3 scripts/check_lean_audits.py` | 0 | **274** audits (272 on `6cab1ad9`, plus `AuditExecutionCoverage.lean` and `AuditReclamation.lean`) |
| 09 | `python3 -m unittest discover -s scripts/tests` | 1 | 365; the expected failure is the derived Lean audit count |
| 09a | `docs/reports/remaining-d02/build_matrix.py . --gate gate.json` | 0 | |
| 09b | `python3 -m unittest discover -s scripts/tests` | 0 | 365 |
| 10 | `python3 scripts/build_docs_index.py --check` | 0 | after regenerating the index |
| 11 | `python3 scripts/ct_stage_summary.py . --check` | 0 | mapped 50 / unmapped 0 |

**Also run.**
- *The card's own targets* ([`scripts/targeted.sh`](scripts/targeted.sh),
  run `t2` on the tip): 65 passed, 0 failed, every exit 0.
- *Corpus, system and slice*: §4.

**Not due:** the provider gate, because `providers/` is unchanged.

**Axioms and placeholders.** `AuditAll.lean` passes, so there are no
placeholders and no project axioms. The new modules use only `propext`,
`Classical.choice` and `Quot.sound`. Clang 21.1.8.
