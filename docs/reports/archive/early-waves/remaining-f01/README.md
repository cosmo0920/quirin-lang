# F01 — one checker/model correspondence boundary

Baseline `9478b94`, 2026-09-08. Work card F01 (`bounded_proof`, W3/W6, Proof
lane). Prerequisites E03, V01, V02 complete.

**Deliverable: a correspondence statement for one finite, generation-indexed
lifetime relation, with its representation relation, its enumerated domain, its
tested/proved split and its premises all named.** V02 closed with
"Rust/Lean/native-C correspondence remains open" and stated that the two sides
carry the R02 obligation *independently*; this card ties one of those pairs
together and measures exactly how far the tie goes.

No production file changed. `check_concurrent`, `QCore.Concurrency.Lifetime` and
every pre-existing theorem are byte-identical; the whole diff is one new Rust
test file, one new Lean module, one new Lean audit and one import line.

## 1. The two sides, by name

| Side | Name | File |
|---|---|---|
| Rust checker | `qcore_core::concurrency::check_concurrent` | `crates/qcore-core/src/concurrency.rs:281` |
| — its terminal/unknown decision | `concurrency::active_task` (private) | `crates/qcore-core/src/concurrency.rs:570` |
| — input | `ConcurrentProgram { permissions, initial_custody, events }` | same, `:117` |
| — output | `Result<ConcurrencyCertificate, Vec<ConcurrencyDiagnostic>>` | same, `:233` / `:124` |
| Lean model | `QCore.Concurrency.Lifetime.step`, `.run` | `proof/QCore/Concurrency.lean:65`, `:300` |
| — state | `QCore.Concurrency.Lifetime.State` | same, `:17` |
| — events | `QCore.Concurrency.Lifetime.Event` | same, `:54` |
| Correspondence (Lean) | `QCore.Concurrency.LifetimeCorrespondence` | `proof/QCore/LifetimeCorrespondence.lean` (new) |
| Correspondence (Rust) | `crates/qcore-core/tests/lifetime_correspondence.rs` (new) | — |
| Audit | `proof/AuditLifetimeCorrespondence.lean` (new) | — |

`check_concurrent`'s state is a tuple of local bindings, not a named type. The
components the relation names are `clocks`, `active`, `joined`, `task_parents`,
`custody`, `destroyed`, `held`, `locks` and `program.permissions`
(`concurrency.rs:287`–`:299`).

Shared terminology is *not* the claim here. `Lifetime` is a deliberate
projection: it carries no vector clock, no mutex identity, no permission share
and no footprint. The relation below says exactly which checker components it
does and does not represent.

## 2. The representation relation `R`

Write `C` for a `check_concurrent` state after some prefix of events and `L` for
a `Lifetime.State`. `R(C, L)` holds when all seven conjuncts hold.

| # | Lean component | Checker component | Relation |
|---|---|---|---|
| R1 | `L.knownTasks` | `C.clocks.keys()` | equal as sets |
| R2 | `L.terminalTasks` | `C.joined` | equal as sets |
| R3 | `L.destroyedResources` | `C.destroyed` | equal as sets |
| R4 | `L.parents` (`(child, parent)`) | `C.task_parents` | equal as sets |
| R5 | `L.custody` (`(resource, task)`) | `C.custody` | equal as maps |
| R6 | `L.permissionHolders` | `{(p.footprint.resource, p.task) : p ∈ permissions, p.share > 0}` | equal as sets |
| R7 | `L.guardHolders` | `C.held` | equal **as multisets** of task ids, one entry per held guard |

R7 is a multiset, not a set, because `Lifetime.lock` conses the task and
`Lifetime.unlock` uses `List.erase` (first occurrence only), matching one
`held[task]` push/removal per checker `Lock`/`Unlock`.

Deliberately **not** in `R`: `C.clocks`' vector-clock values, `C.locks`' mutex
identities and ranks, `C.accesses`, and permission shares/footprints. Those are
the components the abstraction drops; §6 measures what dropping them costs.

### Event translation `tau`

`translate` in the harness. Not injective — two distinct checker `Lock` events
collapse onto one `Lifetime.lock`, which is the abstraction working.

| `ConcurrentEvent` | `Lifetime.Event` |
|---|---|
| `Spawn { parent, child }` | `.spawn parent child` |
| `Join { parent, child }` | `.join parent child` |
| `Lock { task, mutex, rank }` | `.lock task` (mutex, rank dropped) |
| `Unlock { task, mutex }` | `.unlock task` (mutex dropped) |
| `Access { task, footprint, kind, guarded_by }` | `.access task footprint.resource` (kind, guard, offsets dropped) |
| `TransferCustody { from, to, resource, synchronized }` | `.transfer from to resource synchronized` |
| `Destroy { task, resource }` | `.destroy task resource true` |

`Destroy` is the one asymmetric case. `Lifetime.destroy` takes
`priorAccessesOrdered` as an **input** — the abstract state has no vector clock —
so `tau` cannot compute it. It carries `true`, which is sound in one direction
only: a `Destroy` the checker *accepts* necessarily computed
`unordered_access = false` (`concurrency.rs:531`–`:541`). §6.2 exhibits the
witness that shows the converse fails, and the Lean alphabet carries the `false`
variant so the model side evaluates both.

## 3. The correspondence statement

Let `eventLevel(t)` be `check_concurrent`'s diagnostics on trace `t` minus
`OutstandingTask` and `OutstandingLock` (end-of-trace sweeps, which are not
per-event decisions and have no counterpart in a one-step relation).

> **F01-C (abstraction soundness).** For every trace `t` in the declared domain,
> if `eventLevel(t)` is empty then `Lifetime.run domainInitial (tau t)` is
> `some L`, and `R(C_t, L)` holds on every component of §2 that the checker's
> own diagnostics can observe.

Its contrapositive is the useful direction and the reason the card exists: a
trace the Lean model refuses is a trace `check_concurrent` refuses. That is what
lets the `Lifetime` terminal-authority theorems say something about the checker.

**The converse is false and is not claimed.** `check_concurrent` is strictly
stronger; §6 counts and classifies every divergence in the domain.

## 4. The declared finite domain

One fixed `ConcurrentProgram` shell for every trace: permissions
`[{task 0, resource 7, [0,4), share 512}, {task 1, resource 7, [0,4), share 512}]`
(summing to `FULL_ACCESS`, so `validate_permissions` contributes nothing) and
`initial_custody = {7 -> 0}`. Its abstraction is `domainInitial`:
`knownTasks = [0]`, `custody = [(7,0)]`, `permissionHolders = [(7,0),(7,1)]`,
everything else empty — the abstraction of the checker's own initial
configuration (`concurrency.rs:287`–`:298`).

Twelve-event alphabets, tasks `{0,1,2}`, resource `7`, mutexes `{4,5,6}`:

Checker alphabet (`checker_alphabet`), in order — each row also gives its `tau`
image, so this table *is* the translation restricted to the domain:

| # | Checker event | `tau` image |
|---|---|---|
| 1 | `Spawn{0,1}` | `.spawn 0 1` |
| 2 | `Spawn{1,2}` | `.spawn 1 2` |
| 3 | `Join{0,1}` | `.join 0 1` |
| 4 | `Join{1,2}` | `.join 1 2` |
| 5 | `Lock{0, mutex 4, rank 1}` | `.lock 0` |
| 6 | `Lock{0, mutex 5, rank 1}` | `.lock 0` |
| 7 | `Lock{1, mutex 6, rank 1}` | `.lock 1` |
| 8 | `Unlock{0, mutex 5}` | `.unlock 0` |
| 9 | `Unlock{1, mutex 6}` | `.unlock 1` |
| 10 | `Access{1, [0,2) of 7, Read, unguarded}` | `.access 1 7` |
| 11 | `TransferCustody{0,1,7,synchronized}` | `.transfer 0 1 7 true` |
| 12 | `Destroy{0,7}` | `.destroy 0 7 true` |

Rows 5 and 6 share an image: that is the mutex-identity abstraction, and §6.1
counts what it costs. Two mutexes for task 0 are what make
`DuplicateLockCapability` and `LockOrderViolation` reachable at all.

Lean alphabet (`alphabet`), in order — the eleven distinct images above plus
`.destroy 0 7 false`, which `tau` never produces (§2) but which the model side
evaluates:

`.spawn 0 1`, `.spawn 1 2`, `.join 0 1`, `.join 1 2`, `.lock 0`, `.lock 1`,
`.unlock 0`, `.unlock 1`, `.access 1 7`, `.transfer 0 1 7 true`,
`.destroy 0 7 true`, `.destroy 0 7 false`.

| Domain | Depth | Traces | Checked by |
|---|---|---|---|
| Lean-kernel domain | `<= 3` | 1885 | Lean `decide` **and** the Rust harness |
| Rust extension | `<= 4` | 22621 | the Rust harness only |

The Lean domain stops at depth 3 on purpose. Depth 4 elaborates (41 s, verified
during this card) but needs `maxRecDepth 1000000`, which effectively disables
Lean's stack-overflow guard and would make `lake build` host-dependent. At depth
3 the module elaborates in 3.3 s under `maxRecDepth 100000`. The four
correspondence properties are cheap on the Rust side and run at **both** depths;
the digests and counts are pinned at depth 3, where both sides check them.

## 5. Tested versus proved — the precise split

### 5.1 Proved in Lean (kernel-checked, no new axiom)

Eight theorems in `QCore.Concurrency.LifetimeCorrespondence`, all closed by
`decide`, all registered in `AuditLifetimeCorrespondence.lean`. Measured axiom
closure: `propext` and `Quot.sound` only — inside the `AuditAll.lean` allowance
(`propext`, `Classical.choice`, `Quot.sound`). No `sorry`, no placeholder, no
model definition changed.

| Theorem | What the kernel evaluates | Pinned value |
|---|---|---|
| `domain_size` | `domainTraces.length` | 1885 |
| `domain_trace_digest` | FNV fold over every trace of the domain | `16753757158032361952` |
| `domain_outcome_digest` | `run domainInitial` on every trace, folded with **every field** of each reached state | `12252691954508661170` |
| `domain_accepted_trace_count` | traces the model accepts | 53 |
| `domain_refuses_every_terminal_participant` | the selected model lemma (below) | `true` |
| `domain_terminal_reaching_trace_count` | traces reaching a nonempty `terminalTasks` | 6 |
| `domain_terminal_participant_probe_count` | terminal-participant probes the lemma discharges | 42 |
| `domain_terminal_participant_probes_are_nonvacuous` | that count is positive | — |

**The selected model lemma.** `domain_refuses_every_terminal_participant`: in
every state the domain reaches, every alphabet event naming a terminal task
among its `participants` is refused by `step`. It is the finite-domain,
whole-alphabet instantiation of V02's per-constructor theorems
(`terminal_task_rejects_lock`, `terminal_child_rejects_join`, …), which remain
the general proofs for arbitrary states and are unchanged. `spawn` is included
and passes for a different reason than the rest — a terminal child is still a
*known* task, so `child ∉ knownTasks` fails rather than `State.Active`.

### 5.2 Checked by cross-evaluation (not premised, not proved)

Two things that would otherwise have been premises are instead *measured*
equalities between the two sides:

* **Domain agreement.** The Rust harness enumerates its own domain and computes
  `trace_digest`; the Lean kernel computes `traceDigest domainTraces`. Both pin
  `16753757158032361952` and both pin 1885 traces. A different enumeration on
  either side fails one of the two.
* **Transcription agreement.** The harness contains a Rust transcription of
  `Lifetime.step`/`run` (`model_step`, `model_run`). `outcome_digest` folds its
  verdict *and* full final state, in the model's own list order, over the whole
  domain; `domain_outcome_digest` does the same in Lean. Both pin
  `12252691954508661170`. A transcription that diverged anywhere in the domain —
  in a verdict, a list order, or a single field — would fail here. It did not:
  both digests matched on the first run.

This is what turns "I reimplemented the Lean model in Rust, trust me" into a
checked claim. It is still cross-evaluation, not a proof of extraction; §7
names the residual.

### 5.3 Tested in Rust (executable, exhaustive over the domain)

Nine tests in `crates/qcore-core/tests/lifetime_correspondence.rs`.

| Test | What it establishes | Scale |
|---|---|---|
| `declared_domain_matches_the_lean_enumeration` | domain agreement (§5.2) | 1885 traces |
| `model_transcription_matches_the_lean_evaluation` | transcription agreement (§5.2) | 1885 traces |
| `lean_pinned_constants_are_recomputed_here` | the non-vacuity counts Lean pins | 42 probes |
| `checker_acceptance_implies_model_acceptance_over_the_declared_domain` | **F01-C**, the soundness direction | depths 3 and 4 |
| `certificate_projections_equal_the_model_state` | `certificate.completed_tasks` = sorted `L.terminalTasks`; `certificate.destroyed_resources` = sorted `L.destroyedResources` | every certificate in both domains |
| `representation_relation_holds_on_every_observable_component` | R1–R5, R7 (§5.4) | every clean trace, both depths |
| `terminal_tasks_are_refused_before_every_authority_check` | the card's named relation, on the checker side | 36 refusals at depth 3, 144 at depth 4 |
| `the_abstraction_is_strictly_coarser_than_the_checker` | the divergence census (§6.1) | 35 traces at depth 3, 285 at depth 4 |
| `the_vector_clock_ordering_bit_is_outside_the_abstraction` | the one class the domain cannot reach (§6.2) | one named witness |

### 5.4 How `R` is checked without instrumenting the checker

`check_concurrent` exposes no intermediate state, and this card may not add a
hook. Instead each component of `R` is read off the checker's *own diagnostics*
by appending one probe event to a clean trace and diffing. Every probe is chosen
so that only the decision under test can fire.

| Probe appended | Checker decision | Component |
|---|---|---|
| `Lock{t, mutex 90+t, rank 9}`, `t ∈ {0,1,2}` | `UnknownTask` / `TerminalTask` / nothing — a three-way exact match | **R1, R2** |
| `Destroy{t, 7}` | `DeadResource` iff `¬Live` | **R3** |
| `Destroy{t, 7}` | `WrongCustodian` iff `(7,t) ∉ custody` | **R5** |
| `Destroy{t, 7}` | `NonquiescentDestruction` whenever `¬OtherAuthorityQuiescent` | **R6, R7** |
| `Unlock{t, m}` over `m ∈ {4,5}` (`t=0`), `{6}` (`t=1`), `{}` (`t=2`) | count of clean unlocks equals `L.guardHolders`' multiplicity of `t` | **R7** |
| `Join{p,c}` over `(0,1),(1,2),(0,2),(2,1)` | `UnknownJoin` iff `(c,p) ∉ parents ∨ c ∈ terminalTasks` | **R4, R2** |

The mutex `90+t` and rank `9` are chosen so `DuplicateLockCapability` and
`LockOrderViolation` cannot fire, leaving `active_task` as the only decision —
which is what makes R1/R2 an exact three-way match rather than an implication.

R6 (`permissionHolders`) is fixed by the domain's constant permission set and is
not separately probed.

## 6. Divergences found

**No divergence between the Rust checker and the Lean model was found in the
direction F01-C claims.** Every one of the 1885 (and 22621) traces satisfies it,
and the whole-domain outcome digest matched between the two implementations on
the first run. There was nothing to repair on either side, so neither side was
edited.

The divergences below are all in the *converse* direction — the model accepting
what the checker refuses. That is the abstraction being coarser, which is
correct and expected; it is recorded so nobody reads F01-C as an equivalence.

### 6.1 Census: model accepts, checker refuses

Pinned as `COARSER_ABSTRACTION_TRACES_AT_KERNEL_DEPTH = 35` and
`..._AT_EXTENDED_DEPTH = 285`.

| Diagnostic class | Depth 3 | Depth 4 |
|---|---:|---:|
| `DuplicateLockCapability` | 12 | 100 |
| `DuplicateLockCapability` + `LockOrderViolation` | 6 | 52 |
| `DuplicateLockCapability` + `LockOrderViolation` + `UnlockWithoutCapability` | — | 4 |
| `DuplicateLockCapability` + `UnlockWithoutCapability` | 2 | 16 |
| `JoinWhileLocked` + `UnlockWithoutCapability` | — | 3 |
| `LockOrderViolation` | 10 | 72 |
| `LockOrderViolation` + `UnlockWithoutCapability` | 1 | 8 |
| `UnlockWithoutCapability` | 4 | 30 |
| **Total** | **35** | **285** |

Every class traces to one dropped component: **mutex identity** (R7 records
*how many* guards a task holds, never *which*). `JoinWhileLocked` appears only
at depth 4 and only as a downstream consequence — after an
`UnlockWithoutCapability` the checker still holds a guard the model has already
released, so the model admits a join the checker refuses.

### 6.2 The vector-clock ordering bit

Absent from the census at both depths, and that absence is itself a finding:
inside the declared domain the ordering bit is never the deciding factor,
because a permission holder can only become quiescent by being joined, and a
join merges its clock into the parent (`concurrency.rs:350`–`:355`). So every
checker-accepted `Destroy` there has ordered prior accesses, which is why `tau`
carrying `true` is sound.

`the_vector_clock_ordering_bit_is_outside_the_abstraction` leaves the domain to
show the class is real: with a *single* permission holder (the destroying task)
and a guarded access by an unjoined child, the checker refuses with
`NonquiescentDestruction { event: 4, resource: 7 }` while the model — told
`prior_accesses_ordered = true` — destroys the resource. Feeding the model the
bit the checker actually computed (`false`) refuses it again, so the divergence
is exactly the missing input and not a modelling error.

## 7. Premises — what remains asserted

1. **Extraction.** `model_step`/`model_run` in the Rust harness are a hand
   transcription of `Lifetime.step`/`Lifetime.run`. §5.2 checks them by
   whole-domain outcome digest rather than by inspection, but no mechanised
   extraction from Lean to Rust exists in this project, so *outside* the
   enumerated domain the transcription is asserted.
2. **Digest injectivity.** Domain and transcription agreement are equalities of
   64-bit FNV-style folds. Two different domains, or two different
   transcriptions, agreeing on a digest is a collision. `encodeEvent` is
   injective for task ids below 8 and resource ids below 64 (which the alphabet
   respects); the folds are not claimed collision-resistant against an adversary,
   only against accident.
3. **Enumeration completeness.** `domainTraces` is *defined* to be every trace of
   length at most 3 over `alphabet`. That the alphabet is a useful choice — that
   it reaches the states worth relating — is a judgement, evidenced by the
   non-vacuity counts (§5.1) and the probe counts (§5.4), not a theorem.
4. **Depth.** F01-C is established for traces of length at most 4. It is not an
   inductive theorem over arbitrary traces; the existing general theorems in
   `Concurrency.lean` (`step_preserves_terminal`, `run_preserves_terminal`, the
   per-constructor family) carry that weight on the model side alone.
5. **Trace trust.** Everything here sits above `TrustedExplicitTrace`.
   `ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds`,
   `GuardFootprintAuthority` and `SynchronizedCustodyEdges` are unchanged
   declared premises.

## 8. Out of scope, and explicitly not claimed

* **No claim that a Lean theorem proves the Rust implementation correct.** The
  Lean lemma constrains `Lifetime`; F01-C connects `check_concurrent`'s
  acceptances to `Lifetime`'s. Composing them bounds the checker's *acceptances*
  by the model's, on this domain, and nothing more.
* No full preservation/progress theorem for QCore.
* No source-to-trace and no trace-to-native relation (that is F02); arbitrary C
  source is not shown to lower into these traces.
* The `CheckedMutexInvariantV1` profile (`mutex_invariant.rs`, T02) is a
  different checker with its own Lean twin and is untouched; this card
  deliberately took the simpler pair.
* Nothing is promoted. Raw Cycle 19 units remain 2 Safe / 42 provisional Unsafe /
  137 Unknown plus four data-only units, CT coverage remains 11/50 mapped, the
  contract corpus remains 59 modules, `qcore-schemas` remains 4.0.0 and full
  language conformance remains false. The four pinned 185-unit artifacts are
  untouched by construction (no production file changed) and were not
  re-measured; CORPUS-REPIN remains open.

## 9. Changes

| File | Change |
|---|---|
| `crates/qcore-core/tests/lifetime_correspondence.rs` | new, 1144 lines, 9 tests |
| `proof/QCore/LifetimeCorrespondence.lean` | new, 223 lines, 8 theorems |
| `proof/AuditLifetimeCorrespondence.lean` | new, 29 lines, 8 `#print axioms` |
| `proof/QCore.lean` | +1 import |

`check_concurrent`, `proof/QCore/Concurrency.lean`, `mutex_invariant.rs`,
`AGENTS.md`, `CLAUDE.md` and `docs/reevaluation/ct_manifest.json` are unchanged.

## 10. Gate results

| Gate | Result |
|---|---|
| `cargo test --workspace` | exit 0 — **1089 passed**, 0 failed (1080 at baseline; +9 new) |
| `cargo fmt --all -- --check` | exit 0, clean |
| `cargo clippy --all-targets -- -D warnings` | exit 0, clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B -m unittest discover -s scripts/tests` | exit 0 — 43 passed |
| `cd proof && lake build` | exit 0 — 65 jobs (was 64) |
| `python3 -B scripts/check_lean_audits.py` | exit 0 — **22 audits** (was 21) |
| `cargo test --manifest-path providers/Cargo.toml` | exit 0 — 51 passed |

### 10.1 Environment notes, recorded not worked around

* `third_party/libevent` was uninitialised in this worktree and
  `libevent_submodule_matches_the_audited_revision` failed until
  `git submodule update --init --recursive` ran, exactly as AGENTS.md documents.
  No pin was touched.
* `crates/qcore-verifier-host/tests/confined_process_execution.rs` (P03) is
  load-flaky on this host: it failed twice with `Broken pipe (os error 32)`,
  each time on a *different* test, and it failed the same way with **all F01
  changes stashed**, so it is pre-existing and unrelated. It passed in the
  final full run. This matches the P02 `RLIMIT_NPROC` flakiness already recorded
  in `docs/reports/remaining-p03/README.md` §9.1. No assertion was weakened.

## 11. Named follow-ups

* **F01-B1.** F01-C is bounded at trace length 4. An inductive Rust-side
  argument, or a Lean simulation theorem against a Lean model *of the checker*,
  would remove the depth bound. Neither exists.
* **F01-B2.** The extraction premise (§7.1) is the weakest link. Generating the
  Rust transcription from the Lean source, or the domain from a single shared
  description, would close it.
* **F01-B3.** The alphabet covers no data race, no unauthorised access and no
  permission overflow, because the model represents none of them; those checker
  decisions are outside every correspondence stated here.
* **F01-B4.** The `CheckedMutexInvariantV1`/`QCore.Concurrency.MutexInvariant`
  pair has no correspondence statement. It is the natural second boundary and is
  strictly harder — that checker derives, rather than declares, its guard
  authority.
