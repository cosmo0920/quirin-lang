# T02 — `CheckedMutexInvariantV1`: fixed-shape mutex-invariant checker

Date: 2026-09-07. Base checkpoint: `12cb8a2` (T01's design commit).
Specification: [`../remaining-t01/README.md`](../remaining-t01/README.md) and
[`../remaining-t01/profile.md`](../remaining-t01/profile.md).

**Scope.** Model-level checker only. No source lowering (that is T03), no
growing protected graphs, no lock-order analysis, no condition variables,
reader/writer locks or try/timed acquisition, no `Send`/`Sync` derivation, and
no retirement of the abstract profile. `check_concurrent`, its 26 pinned unit
tests and `proof/QCore/Concurrency.lean`'s `Lifetime` namespace are unchanged.

---

## 1. What landed

| Artifact | Lines | Role |
|---|---|---|
| `crates/qcore-core/src/mutex_invariant.rs` | 1,923 | new sibling module, entry point `check_mutex_invariant_program` |
| `crates/qcore-core/tests/unit/mutex_invariant.rs` | 1,636 | 48 tests |
| `proof/QCore/MutexInvariant.lean` | 918 | new namespace `QCore.Concurrency.MutexInvariant`; 31 public theorems, 2 private lemmas, 29 decidable scenarios |
| `proof/AuditMutexInvariant.lean` | 54 | new audit file; 31 `#print axioms` registrations |

Edits to existing files are three, all additive:

| File | Change |
|---|---|
| `crates/qcore-core/src/concurrency.rs` | `+ ConcurrencyProfile::CheckedMutexInvariantV1`, `+ ConcurrencyAssumption::{DeclaredFootprintShape, InvariantPredicateDeclared}`, plus per-variant docs. **`check_concurrent`'s body is byte-identical** (`git diff` touches only the two enum declarations). `GuardFootprintAuthority` and `SynchronizedCustodyEdges` are kept, because `TrustedExplicitTrace` still issues them. |
| `crates/qcore-core/src/lib.rs` | `pub mod mutex_invariant;` and re-exports |
| `proof/QCore.lean` | `import QCore.MutexInvariant` |

`TrustedExplicitTrace` is retained in full. Nothing was deleted, no gate was
weakened, no pinned expectation was edited, no diagnostic was downgraded.

---

## 2. State machine

### 2.1 Identities

`MutexId`, `GuardId`, `InvariantId`, `BorrowId` and `FactId` are all
generation-indexed: never a reusable address, handle or source variable. Guard,
borrow and fact ids must be globally fresh at mint (`IdentityNotFresh`); a
destroyed `MutexId` is dead forever.

`AccessFootprint`, `AccessKind`, `SharedResourceId`, `TaskId` and `FULL_ACCESS`
are reused unchanged from `concurrency`.

### 2.2 Per mutex generation

```
              CreateMutex                          DestroyMutex
 (absent) ──────────────────▶ Unlocked ─────────────────────────▶ Dead
                                │  ▲                              (forever)
                          Lock  │  │  Unlock / ErrorExit
                    (mints g)   │  │  (requires Closed, no live g-borrow)
                                ▼  │
                          Held(g, task)
```

`CreateMutex` fixes `protects: Vec<AccessFootprint>` for the generation's whole
lifetime and **consumes** the creator's `FULL_ACCESS` share over every declared
interval, parking it in the mutex. `DestroyMutex` returns that parked share, and
only to the task that parked it.

### 2.3 Per guard instance

```
   Lock ──▶ Closed ──OpenInvariant──▶ Open ──CloseInvariant(witness)──▶ Closed ──▶ (dead)
```

* Reads need a live guard held by the acting task whose mutex *contains* the
  footprint. Writes additionally need `Open`.
* `Unlock` / `ErrorExit` require `Closed`, no live guard-derived borrow, and
  kill the guard. Every guard-scoped fact is dropped unconditionally.
* `CloseInvariant` drops every fact overlapping a write performed while open.

### 2.4 Access authority — the repair

There is **no fallback path** to a protected footprint. Exactly one of:

| Situation | Outcome |
|---|---|
| footprint unprotected, no guard | pointwise share ledger decides (`FULL_ACCESS` to write, `> 0` to read) |
| footprint unprotected, guard supplied | `UnprotectedFootprint` |
| footprint protected, no guard | `UnguardedProtectedAccess` |
| footprint protected, guard of a *different* mutex | `UnprotectedFootprint` |
| footprint protected, guard held by another task | `WrongGuardAffinity` |
| footprint protected, guard from a closed section | `DeadGuard` |
| footprint protected, correct guard, write, `Closed` | `WriteWithClosedInvariant` |

The unguarded row is not a policy choice: creation *consumed* the creator's
share, so no static permission survives to reach the footprint.

### 2.5 Pointwise share ledger

Boundary coordinates are collected from every interval named anywhere in the
program (initial grants, declared shapes, accesses, borrows, facts, refused
extension proposals). Consecutive boundaries form **atoms**, so every interval
is an exact union of atoms and no atom is ever partially covered.

The invariant checked **after every event, refused events included**, and once
before the first:

```
for every live atom p:  Σ_tasks share(task, p) + parked(p) == initial_total(p)
```

`initial_total` is the entry total at that atom; conservation is invariance of
that total, and on a protected atom it is exactly `FULL_ACCESS` because
`CreateMutex` demands the creator hold `FULL_ACCESS` there. Destroying a
resource retires its atoms rather than leaking their authority. The certificate
records `conservation_checkpoints == events + 1`, so "at every step" is
auditable rather than asserted.

### 2.6 Synchronization witnesses (derive-and-compare)

`TransferCustody` carries no `bool`. Its `SyncWitness` must be *both*
structurally present in the checker's own history *and* consistent with the
vector clocks, in the `weak_memory.rs` style:

| Witness | Structural check | Clock check |
|---|---|---|
| `SpawnEdge { child }` | `child == to`, `to`'s parent is `from`, `to` not joined | `clock(from) ≤ clock(to)` |
| `JoinEdge { child }` | `child == from`, `from` was joined by `to` | as above |
| `MutexHandoff { mutex }` | `mutex.protects` names `resource`, and *some release by `from`* of that generation is visible to `to` | as above |

Failure is `ForgedSynchronizationWitness`. The mutex generation keeps a full
`release_history`, not just the last releaser, so a handoff `A → B` remains
derivable after a third party has since released the same mutex.

### 2.7 Refusals, represented rather than omitted

`ExtendProtection` and `UnsupportedConstruct` are real events that *always*
reject (`DynamicProtectedShape`, `UnsupportedSynchronizationConstruct`). The
construct is expressible so the refusal is testable, instead of the model
quietly lacking a way to say it. `UnsupportedSynchronization` enumerates
condition variables, try/timed locks, rwlocks, upgradeable guards, nested
protecting mutexes, recursive acquisition, lock-rank declaration and
completion-capsule composition.

### 2.8 Diagnostic vocabulary

The fourteen named refusals occupy exactly the free `Q610`–`Q623` range:

| Diagnostic | Code |
|---|---|
| `UnprotectedFootprint` | `Q610` |
| `UnguardedProtectedAccess` | `Q611` |
| `WrongGuardAffinity` | `Q612` |
| `InvariantOpenAtUnlock` | `Q613` |
| `InvariantOpenAtErrorExit` | `Q614` |
| `GuardBorrowEscape` | `Q615` |
| `StaleProtectedFact` | `Q616` |
| `FootprintAlreadyProtected` | `Q617` |
| `DynamicProtectedShape` | `Q618` |
| `NonquiescentMutexDestruction` | `Q619` |
| `ForgedSynchronizationWitness` | `Q620` |
| `MutexAuthorityNotConserved` | `Q621` |
| `UnsupportedSynchronizationConstruct` | `Q622` |
| `DeadMutexGeneration` | `Q623` |

Family members, per T01 §4.1's own `DeadGuard → Q612` precedent: `DeadGuard`
joins the affinity family at `Q612`; `DoubleOpenInvariant`,
`WriteWithClosedInvariant` and `InvalidInvariantWitness` join the
invariant-phase family at `Q613`. The structural refusals carried over from the
bounded concurrency model (`UnknownTask`, `DeadResource`, `UnauthorizedAccess`,
`DataRace`, `OutstandingMutex`, `IdentityNotFresh`, …) keep the existing
concurrency pass code `Q600`; they are not new obligations of this profile.
`named_refusals_occupy_the_free_q610_to_q623_range` pins the whole table.

### 2.9 Certificate

```rust
MutexInvariantCertificate {
    profile, assumptions, checked_events, conservation_checkpoints,
    completed_tasks, destroyed_resources, destroyed_mutexes,
    protected_shape, admitted_invariant_transitions, derived_sync_edges,
}
```

Admissible assumptions: `InitialAuthorityAndUniqueLifetimeIds` always;
`DeclaredFootprintShape` once a mutex is created; `InvariantPredicateDeclared`
only if some `CloseInvariant` used a `Declared` witness (a `NoWriteOccurred`
witness is *checked* and adds nothing).

`recheck_mutex_invariant_certificate` re-runs the checker independently and
refuses any field the re-derivation does not reproduce, and refuses
`GuardFootprintAuthority` / `SynchronizedCustodyEdges` before comparing
anything.

---

## 3. Test inventory (48, all in `crates/qcore-core/tests/unit/mutex_invariant.rs`)

Every rejection asserts the **exact** diagnostic vector, not `is_err()`, and
most carry an accepted positive control that differs by one event.

### Acceptance 1 — protected access, affinity, quiescence (14)

`same_mutex_protected_access_is_accepted` ·
`unrelated_lock_grants_no_authority_over_a_protected_footprint` ·
`two_mutexes_cannot_protect_one_footprint` ·
`unguarded_access_to_a_protected_footprint_has_no_fallback_authority` ·
`wrong_thread_unlock_is_refused` ·
`a_guard_is_not_transferable_to_another_task` (5-way matrix) ·
`a_guard_from_an_earlier_critical_section_is_dead` ·
`mutex_destruction_with_a_holder_is_refused` ·
`mutex_destruction_with_a_recorded_waiter_is_refused` ·
`mutex_destruction_with_a_live_guard_in_another_task_is_refused` ·
`a_refused_mutex_destruction_leaves_the_generation_live` ·
`a_repeat_destruction_reports_a_dead_generation_not_an_ownership_complaint` ·
`every_event_kind_naming_a_destroyed_generation_is_refused` (4-way matrix) ·
`resource_destruction_is_blocked_by_parked_authority`

### Acceptance 2 — invariant phases, borrows, stale facts (11)

`unlock_with_a_broken_invariant_is_refused` ·
`error_exit_with_a_broken_invariant_is_refused` ·
`a_write_requires_the_invariant_to_be_declared_broken_first` ·
`a_no_write_witness_cannot_close_over_a_write` ·
`a_declared_witness_must_name_this_mutex_invariant` ·
`opening_an_already_open_invariant_is_refused` ·
`a_guard_derived_borrow_cannot_escape_unlock` ·
`a_borrow_outside_the_protected_shape_is_refused` ·
`a_fact_established_under_one_guard_generation_is_unusable_under_the_next` ·
`a_fact_invalidated_by_a_write_while_open_cannot_be_used` ·
`guard_identities_are_never_reused`

The four "broken invariant" tests pin the *full* cascade — a refused unlock
leaves the guard live, which then blocks destruction and surfaces in the
end-of-trace sweep. That cascade is the fail-closed behaviour, and pinning it
proves the refusal did not silently release the guard.

### Acceptance 3 — certificates and recheck (5)

`certificates_name_exact_generations_and_admitted_invariant_transitions`
(exact `protected_shape`, `destroyed_mutexes`, and the four-transition
Absent→Closed→Open→Closed→Absent sequence with its witness) ·
`independent_recheck_rederives_the_certificate` ·
`recheck_refuses_every_tampered_certificate_field` (8 field mutations) ·
**`a_checked_certificate_never_carries_trusted_guard_assumptions`** — the
pinned negative: the typed `BTreeSet` excludes both premises, a forged
certificate carrying either is refused by recheck, and
`assumption_names()` shows neither string, closing the free-form
`BTreeMap<String, bool>` route at this layer ·
`recheck_refuses_a_certificate_whose_program_no_longer_checks`

### Acceptance 4 — share conservation (6)

`conservation_is_verified_before_and_after_every_event` ·
`partial_parking_is_refused` ·
`parking_only_part_of_a_declared_interval_is_refused` ·
`overcommitted_entry_authority_is_refused` ·
`only_the_parking_task_may_receive_the_parked_share_back` ·
`parked_authority_is_removed_from_and_returned_to_the_creator`

### Derived witnesses and explicit refusals (12)

`derived_spawn_and_join_edges_are_admitted_and_recorded` ·
`a_forged_spawn_witness_is_refused` ·
`a_mutex_handoff_witness_requires_the_protecting_mutex` ·
`a_derived_mutex_handoff_over_the_protecting_mutex_is_admitted` ·
`conflicting_unordered_accesses_to_an_unprotected_footprint_race` ·
`the_protected_shape_cannot_be_changed_after_creation` ·
`unsupported_synchronization_constructs_are_refused_not_approximated` (9-way) ·
`recursive_acquisition_of_one_generation_is_refused` ·
`a_scope_with_an_outstanding_guard_or_mutex_is_refused` ·
`joining_while_holding_a_guard_is_refused` ·
`named_refusals_occupy_the_free_q610_to_q623_range` ·
`an_empty_program_is_accepted_with_no_footprint_premise`

---

## 4. Lean

New namespace `QCore.Concurrency.MutexInvariant`. `step` is written as an
explicit `can…` guard plus a total `…Effect` per event, so each refusal theorem
is a statement about a guard and each preservation theorem a statement about an
effect. **All 18 theorems T01 §4.2 names are proved.**

| T01 obligation | Theorem |
|---|---|
| `parked_authority_is_conserved` | `parked_authority_is_conserved` |
| `guard_authorizes_only_declared_footprint` | same name |
| `unrelated_guard_grants_no_authority` | same name |
| `unguarded_access_to_protected_footprint_is_rejected` | same name |
| `only_holder_can_unlock` / `guard_is_not_transferable` | both, the latter over all seven guard-keyed operations |
| `unlock_requires_closed_invariant` | same name |
| `error_exit_requires_closed_invariant` | same name |
| `guard_borrows_die_at_unlock` | same name |
| `facts_do_not_survive_unlock` | same name |
| `footprint_has_at_most_one_protector` | same name |
| `protected_shape_is_fixed` | same name |
| `mutex_destruction_requires_quiescence` | same name |
| `destroyed_mutex_generation_is_never_relocked` | same name |
| `refused_mutex_destruction_leaves_generation_live` | same name |
| `synchronization_witness_must_be_derivable` | same name |
| `same_mutex_generation_orders_release_acquire` | same name |
| `distinct_mutexes_do_not_order` | same name |
| `checked_certificate_omits_trusted_guard_assumptions` | same name |

Thirteen supporting theorems also land and are registered:
`createMutexEffect_preserves_authority`,
`destroyMutexEffect_preserves_authority`, `write_requires_open_invariant`,
`release_removes_the_guard`, `use_fact_requires_a_live_guard`,
`unsupported_synchronization_is_refused`,
`critical_section_effects_preserve_declared_shapes`,
`accepted_destruction_kills_the_generation`,
`destroyed_mutex_generation_rejects_repeat_destruction`,
`busy_mutex_rejects_destruction`, `waited_mutex_rejects_destruction`,
`forged_witness_is_refused`, plus two private list lemmas
(`map_preserves_share_sum`, `map_preserves_shapes`).

29 decidable `example`s mirror the Rust tests. `AuditMutexInvariant.lean`
registers 31 theorems; `scripts/check_lean_audits.py` passes **21** audits (was
20), and `AuditAll.lean`'s whole-project axiom closure still admits only
`propext`, `Classical.choice`, `Quot.sound`. **No new axiom, no `sorry`, no
placeholder.**

### 4.1 Named open theorems (not proved here)

Two obligations are deliberately *narrowed* rather than dropped, and are named
so a later card can close them. Neither is a placeholder; the narrower
statement is fully proved.

1. **`step_preserves_declared_shape`** — that *every accepted step* preserves
   the `protects` field of every already-existing mutex generation, stated
   through `State.mutexOf`. What is proved instead is
   `critical_section_effects_preserve_declared_shapes`: the three shape-touching
   effects (`lockEffect`, `releaseEffect`, `registerWaiterEffect`) preserve the
   `(id, protects)` inventory exactly. Combined with `protected_shape_is_fixed`
   (the only shape-changing *event* is refused unconditionally) this covers
   CT34's obligation; the general `find?`-level statement is open.
2. **`pointwise_atom_conservation_refines_authority_total`** — the Lean
   conservation theorem is over the abstract authority total (share is moved
   between custodians, never created or destroyed). The Rust checker verifies
   conservation *pointwise per atom*. That the pointwise ledger refines the
   abstract total is a **premise**, not a theorem, exactly as source-to-trace
   refinement and C layout are.

Source extraction, the real C layout of the declared intervals, and the
invariant predicate itself remain explicit premises throughout.

---

## 5. Gate results

Run at the T02 tip on this worktree branch.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **898 passed, 0 failed, 0 ignored** across 68 suites (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **46 passed, 0 failed** across 16 suites (exit 0) |
| `python3 -m unittest discover -s scripts/tests -p "test_*.py" -t scripts/tests` | **43 tests, OK** |
| `cd proof && lake build` | success, **no warnings** |
| `python3 scripts/check_lean_audits.py` | **Passed 21 Lean audits** |

48 of the 898 workspace tests are new in this increment. The 26 pinned
`check_concurrent` tests, including
`certificates_name_trusted_model_authority_without_removing_guard_semantics`,
`same_mutex_establishes_happens_before` and
`different_mutexes_do_not_hide_a_race`, pass unchanged.

### 5.1 Cycle 19 artifacts

**Not re-executed here, and this is a claim by construction, not a measurement.**
`cargo run --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit`
was attempted and refused before analysing anything:
`could not resolve ../fluent-bit: No such file or directory`. The pinned corpus
lives outside the repository and is absent from this isolated worktree, so no
fresh 185-unit measurement was taken. No artifact file under
`docs/reports/` was modified (`git status` is clean for all of them).

The four full 185-unit audit artifacts are **untouched by construction**: this
increment adds a module that no production route references (the only mentions
of `check_mutex_invariant_program` outside the module and its tests are the
`crates/qcore-core/src/lib.rs` re-exports), and the two enum variants added to
`concurrency.rs` are additive — `check_concurrent` still stamps
`TrustedExplicitTrace` with the same assumption set, and `qcore-core` has no
serialization surface, so no serialized byte can move. The contract corpus (59
modules), the `third_party/libevent` submodule pin and CT coverage (11/50
mapped) are likewise unchanged: no CT case is mapped by this card.

---

## 6. What this card does not establish

* **No source lowering.** `with lock … as …` still routes to `Q600`; the
  `mutex m guards …` creation form of T01 §4.3 does not exist. No CT case among
  CT26–CT34/CT48 becomes mapped, because mapping requires executed source-stage
  evidence.
* **No promotion.** No raw Cycle 19 unit changes verdict. `TrustedExplicitTrace`
  is untouched and still carries `GuardFootprintAuthority` and
  `SynchronizedCustodyEdges`; those premises are discharged **only inside
  `CheckedMutexInvariantV1`, over supplied traces**.
* **No deadlock freedom, no lock-order analysis.** Lock ranks are deliberately
  absent from the program type; a rank declaration is one of the
  `UnsupportedSynchronization` refusals.
* **No completeness of the candidate execution set**, no condition variables,
  reader/writer locks, try/timed acquisition, nested protecting mutexes, or
  composition with the scoped-task ledger's completion capsules. Every one of
  those is a refusal, not an approximation.
* **New premises are introduced, not eliminated.** `DeclaredFootprintShape` and
  `InvariantPredicateDeclared` are the honest residue: this profile converts two
  unconstrained booleans into two narrower, explicitly named declarations plus a
  large body of checked structure. The premise count is not zero.
* **`research_closure.rs` is unchanged.** T01 §3 notes that the aggregate
  closure should record which profile its embedded certificate came from. That
  is a production-route change and is out of scope here; it remains open.
