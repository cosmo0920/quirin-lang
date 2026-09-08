# T03-F — guard-profile completions: a checked witness, guard-scoped facts, bounded task composition, and the Lean destruction event

Date: 2026-09-08. Base checkpoint: `f8dbb2f` (the final observation increment).
Depends on [T03](../remaining-t03/README.md) (the lowering being completed),
[T02](../remaining-t02/README.md) (the unedited checker it lowers onto), and
[the T03 CT integration](../remaining-t03-ct-integration/README.md) (the seven
mutex cases left unmapped and their recorded blockers).

**Scope.** The four items T03 left open in its §10, at their stated boundaries.
No diagnostic code was added, removed or changed; `check_concurrent`,
`check_mutex_invariant_program` and `recheck_mutex_invariant_certificate` are
byte-identical; T02's 48 tests and the 26 pinned `TrustedExplicitTrace` tests
are untouched. `docs/reevaluation/ct_manifest.json` is **not** edited — four CT
mappings are *proposed* in §6 with measured stage evidence, for the integrator.
Nothing is promoted.

---

## 1. What landed

| Artifact | Change | Role |
|---|---|---|
| `crates/qcore-frontend/src/source_mutex.rs` | +~520 / −~180 | the three new source forms; `lower_outer`/`lower_inner` are replaced by one `lower_statement` over an explicit `Frame` |
| `crates/qcore-frontend/src/frontend.rs` | +5 reserved words, +3 statement leaders | `unchanged`, `fact`, `assert`, `use`, `wait`; leaders `assert`, `use`, `wait` |
| `crates/qcore-core/src/mutex_c_emit.rs` | +`is_single_task_trace`, +2 emitted/interpreted event kinds | facts are traced like the invariant phases; task structure is classified, not silently dropped |
| `crates/qcore-verification/src/driver.rs` | +8 lines | a checked trace with task structure is published **without** C rather than refused or faked |
| `proof/QCore/MutexInvariant.lean` | +2 `Event` constructors, +1 `Inner` constructor, +1 `Scope` field, +19 public theorems, +12 decidable scenarios | §12 resource destruction, §13 the checked witness, §14 bounded task composition |
| `proof/AuditMutexInvariant.lean` | 45 → 64 registrations | every new public theorem is axiom-audited |
| `crates/qcore-frontend/tests/source_mutex.rs` | 22 → 38 tests | source acceptance, both directions |
| `crates/qcore-core/tests/unit/mutex_c_emit.rs` | 3 → 5 tests | one more sanitized native run; the backend classification |
| `compiler/tests/checked_mutex_guards.rs` | 4 → 7 tests | end to end, including one more sanitized native run and the located refusals |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c, `docs/QCORE_SPEC_r0_3.md` §L16 | grammar and normative rules | the documented example is executed by a test (§4.5) |

**Nothing was widened to make this fit.** Every refusal below is either the
core checker's own, unedited, or a lowering refusal that is *stricter* than the
core would be. The one place the two differ is recorded in §3.4.

---

## 2. Sub-item 1 — `NoWriteOccurred` from source

### 2.1 The form and the rule

```qcore
with lock m as g { open g; read buf with g; close g unchanged; }
```

`close g unchanged;` selects `InvariantWitness::NoWriteOccurred`. `close g;` is
unchanged and still selects `Declared`.

The lowering admits the checked witness **only** for a decidable region shape:
every statement since this guard's `open` must be a guarded or unguarded
`read`, or a `use FACT;`. Neither can write. Anything else — a write, a
guard-derived borrow, an announced acquisition, a nested section, a child task,
or no open region at all — is refused at `Q600` with a message naming the rule
and pointing at `close GUARD;`.

Two things this deliberately is **not**:

* it is not a downgrade. An undecidable region does not quietly become a
  declared close, because that would erase the premise difference the whole
  form exists to record;
* it is not the core's rule. The core accepts `NoWriteOccurred` whenever
  `writes_while_open` is empty for that guard. The lowering is **stricter**: it
  additionally refuses regions containing a borrow, because this profile models
  no write *through* a borrow and must not claim one could not have happened.

### 2.2 The measured effect

The premise set is the observable. For
`open g; read buf with g; close g unchanged;`:

```
{ InitialAuthorityAndUniqueLifetimeIds, DeclaredFootprintShape }
```

and for the same section with `close g;`:

```
{ InitialAuthorityAndUniqueLifetimeIds, DeclaredFootprintShape, InvariantPredicateDeclared }
```

`DeclaredFootprintShape` stays on both: this card removes one premise, not two.
Tracking is per guard *and* per region, so a section whose first region wrote
and whose second only read carries the declared premise for the first and emits
`[Declared { invariant: 1 }, NoWriteOccurred]` in that order
(`the_checked_witness_is_tracked_per_guard_and_per_region`).

### 2.3 Why the source rule is sound inside this profile

A write to the protected footprint while the invariant is open can only arrive
by (a) a guarded write through this guard — recorded in `writes_while_open`,
and excluded by the source shape anyway; (b) a guarded write through another
guard on the same generation — impossible, one holder; (c) an unguarded write —
`Q611`, no fallback authority; (d) a child task through this guard — `Q612`
before the write is recorded; or (e) a write through a guard-derived borrow —
not an event this model has, and refused by the region rule regardless. That
enumeration is the argument; it is bounded by this profile's own grammar and
does not generalize.

---

## 3. Sub-item 2 — task composition inside the guard profile

### 3.1 What is admitted, and what the smallest sound subset turned out to be

The card asked for "a scoped task created and joined strictly inside the guard,
… or the smallest sound subset". **A task created and joined strictly inside a
guard is not achievable without editing the checker**, and it was not attempted:
`check_mutex_invariant_program` refuses a join while the joining task holds any
guard (`JoinWhileGuardHeld`). Widening that rule is exactly the ledger widening
this card forbids.

What landed instead:

```qcore
thread scope c { with lock m as gc { open gc; write buf with gc; close gc; } }
with lock m as g { read buf with g; }
```

A nested `thread scope NAME { … }` is **one child task**, created where the
block opens and joined where it closes. It is the profile's own child-task
form; the scoped-task `spawn`/`join` expressions stay refused, because their
captures, owned values and completion capsules have no representation in this
model.

The composition is inside the **mutex** ledger only. The scoped task ledger
(`ScopedCopyTasksV1` and its successors) is *not* composed with it, and this
card makes no claim that it is. What is checked here is what the mutex model
checks: task liveness, guard affinity, the pointwise share ledger, the join
quiescence rules, and the vector-clock happens-before used for data races and
for destruction ordering.

### 3.2 The authority interaction is checked, not assumed

A child receives **no share and no custody**. Consequences, each measured:

| Child does | Outcome |
|---|---|
| `read buf;` (unguarded, protected footprint) | `Q611 UnguardedProtectedAccess` |
| `read buf with g;` naming the creator's guard | `Q612 WrongGuardAffinity` |
| `open g;` / `close g;` / `borrow … with g;` on the creator's guard | `Q612` |
| `use f;` on the creator's fact | `Q616 StaleProtectedFact` |
| its own `with lock m as gc { … }` | **accepted**; the creator's later acquisition observes the child's release |
| holds a section at its join | `NonquiescentJoin` (model rule, unedited) |
| `drop b;` for the creator's borrow | `Q600`, refused by the lowering (§3.4) |
| declares a resource, generation or fact | `Q600`, refused by the lowering |

The accepted row is the substantive one: a real two-task handoff over one
generation, where the creator's read after the join is ordered behind the
child's write by the model's own clocks, and the scope-exit destruction is
admitted only because the join merged them.

### 3.3 A child block inside a critical section

It is admitted *lexically* — the child can name the enclosing guard, which is
what makes the guard-affinity refusal reachable from source at all — and then
refused by the core, because its join lands while the creator holds the guard:

```
with lock m as g { read buf with g; thread scope c { } }
    → ["Q600"]   JoinWhileGuardHeld, and nothing else
with lock m as g { thread scope c { read buf with g; } }
    → ["Q612", "Q600"]   WrongGuardAffinity leads; the join refusal follows
```

The control for both moves the same block out of the section and checks.

### 3.4 One deliberate asymmetry, recorded

A child's `drop b;` for a borrow its creator took is refused by the **lowering**
at `Q600`, not represented as an event. That is the only place in this card
where a grammar refusal was preferred to a represented one, and the reason is
soundness: the model's `EndBorrow` checks only that the acting task is live, so
lowering a child's `drop` of its parent's borrow would be **accepted** there.
Representing it would have created a real hole. The child's `borrow` *is*
represented, and is refused at `Q612`.

### 3.5 Native emission: none, and it says so

`qcore_core::mutex_c_emit` emits one thread. A checked trace carrying task
structure — `Spawn`, `Join`, `TransferCustody`, or `RegisterWaiter` — is
therefore outside it. `is_single_task_trace` classifies this, and
`compile_mutex_source` publishes such a module with `c_source: None`: checked,
with no emitted C, rather than C the backend did not produce or a refusal of a
module the checker accepted. `emit_mutex_c11` still refuses such a trace if
handed one; the classification is a caller's guard, not a replacement for the
gate, and a test asserts both halves.

Consequence for the CLI: `qcorec check` reports such a module normally;
`qcorec emit` reports the existing "standalone C emission requires an
executable module" message, which is accurate about the outcome and imprecise
about the reason. That message was not changed here.

Also unchanged, and conservative in the safe direction: the non-Linux/macOS/
Windows `Q500` refusal is still evaluated before the emission decision, so a
multi-task module on another target is refused for an emission that would not
have been attempted.

---

## 4. Sub-item 3 — guard-scoped facts

### 4.1 The form

```qcore
let f = fact buf;                      -- declared at thread-scope level
with lock m as g { assert f with g; read buf with g; use f; }
```

The **name** is lexical and scope-level; the **fact** is guard-scoped. That
split is the point: if the binder were body-local, a cross-section use would be
`Q013 unknown name` — the weaker, lexical refusal that
[the CT integration report](../remaining-t03-ct-integration/README.md) §3.1
already records as insufficient for CT30's escaped-borrow control. Here the
name resolves and the *model* refuses the use.

Each `assert` mints a fresh model identity. A declaration seeds the name with an
identity no assertion ever mints, so a use before any assertion is
`UnknownFact` rather than a grammar refusal.

### 4.2 Measured outcomes

| Source | Outcome |
|---|---|
| assert and use under one guard | **accepted** |
| assert under `g`, use under a later `h` | `Q616 StaleProtectedFact` |
| assert under `g`, re-assert under `h`, use under `h` | **accepted** — a fresh fact, not a revival |
| declared, never asserted, then used | `Q600 UnknownFact` |
| assert under `g`, use outside every section | `Q616` |
| child task uses the creator's fact | `["Q616", "Q600"]` — stale leads, the join refusal follows |
| `assert nothing with g;` (undeclared name) | `Q013`, deliberately not relabelled |

The drop is the checker's existing rule — every guard-scoped fact dies at
unlock or error exit — reached from source for the first time.

### 4.3 Emission

`AssertFact` and `UseFact` have no runtime representation, in exactly the sense
`open`/`close` do not: the core decided their extent statically. They are
emitted and interpreted as trace lines (`assert N`, `use-fact N`) so the native
and reference observations stay comparable, and a sanitized native run compares
them line for line. **The pinned refusal test
`emission_refuses_anything_outside_the_single_task_subset` is unchanged**: its
list is `Spawn`, `TransferCustody`, `RegisterWaiter` and `UnsupportedConstruct`,
and all four still refuse.

`wait m;` (`RegisterWaiter`) is *not* emitted. An announced acquisition only
another thread could satisfy has no honest single-thread rendering, so it stays
in the refused set and puts its module on the no-emission path of §3.5.

### 4.4 `wait m;`

`RegisterWaiter` existed in the model with no source form, which is why CT31's
waiter half was unreachable. `wait NAME;` announces an acquisition the task has
not performed. Performing it withdraws the announcement (the core's `Lock`
removes the waiter); never performing it blocks the generation's destruction.

### 4.5 The documented grammar is executed

`the_documented_grammar_example_checks` runs the exact program printed in
`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c through parse → elaborate → lower →
check → recheck. A documented grammar that does not check is a defect, and this
test is what makes that measurable rather than editorial.

---

## 5. Sub-item 4 — the Lean resource-destruction event

T03 §6.1 recorded: "`Event` has no resource-destruction constructor, so
`lowerScope` models generation lifetime only … that half of scope exit is
covered by the Rust tests, not by these theorems." **That boundary is closed.**

### 5.1 Model changes

* `Event.destroy (task) (resource)`, with `canDestroy` / `destroyEffect`.
* `Event.closeUnwritten (task) (guard)`, with `canCloseUnwritten` (§13).
* `Inner.closeChecked`, lowered by `lowerInner` to `closeUnwritten`.
* `Scope.resources : List ResourceId := []` and `scopeExit`, so `lowerScope`
  emits every generation's destruction and then every resource's. The field is
  defaulted, so every scope written before it existed denotes exactly the trace
  it denoted then, and every §10/§11 theorem statement is unchanged.

`step`, `run`, `canRelease`, `releaseEffect`, the `Lifetime` namespace in
`Concurrency.lean` and every T02/T03 theorem *statement* are unedited. Three
`lowerScope` proofs were re-derived for the extra append; one theorem,
`guard_is_not_transferable`, gained a conjunct for the new close event — its
name is unchanged and the statement is strictly stronger, which is the point:
leaving it out would have left the new event outside the "a guard does not
travel" claim.

### 5.2 Two modeling decisions, stated

1. **`destroyEffect` leaves the destroyed resource's holding in place.** The
   conserved total therefore stays exactly invariant and
   `parked_authority_is_conserved` needed no change. The Rust ledger instead
   retires those atoms from its conservation check. The two agree on what is
   *reachable*, because every access guard tests `destroyedResources` first;
   they differ in bookkeeping only, and that difference is recorded in the
   definition's own docstring.
2. **No clocks.** This model has none — as it already had none for
   `canDestroyMutex` — so the Rust checker's happens-before conditions on
   destruction (`unordered_access`) are **not** among these theorems. Custody,
   liveness, parked authority and foreign shares are.

### 5.3 New public theorems (19 registered)

*Resource destruction (§12):* `destroyEffect_preserves_authority`,
`destroy_requires_custody`, `protected_resource_rejects_destruction`,
`foreign_share_rejects_destruction`, `dead_resource_rejects_repeat_destruction`,
`destroyed_resource_rejects_access`, `accepted_destruction_kills_the_resource`,
`refused_destruction_stops_the_trace`,
`lowerScope_destroys_every_declared_resource`,
`scopeExit_destroys_generations_before_resources`.

*The checked witness (§13):* `checked_witness_requires_no_write`,
`checked_witness_requires_an_open_invariant`,
`checked_witness_ignores_the_declared_predicate`,
`checked_and_declared_close_agree_on_effect`,
`checked_close_carries_no_declared_predicate_premise`,
`a_section_without_a_declared_close_emits_none`.

*Bounded task composition (§14):* `join_while_holding_a_guard_is_refused`,
`join_of_a_guard_holding_child_is_refused`,
`a_shareless_task_cannot_access_unguarded`.

Twelve new decidable scenarios mirror the Rust fixtures: whole scope exit; the
resource refused while its generation is live; admitted after it; double
destruction; a destroyed resource unreachable; the checked witness accepted for
a read-only region, refused after a write, and the declared close accepted for
the same body; one child taking the generation and the creator reading after
the join; a child inside a section refused at its join; a child naming the
creator's guard; and an announced acquisition blocking destruction.

`propext` and `Quot.sound` only; **no new axiom, no `sorry`, no placeholder.**
`scripts/check_lean_audits.py` passes **34** audits.

### 5.4 Still a premise

Source extraction — that the Rust lowering produces `lowerScope` for the source
it reads — is unproved, in Rust and in Lean, exactly as before. T02's two named
obligations (`step_preserves_declared_shape`, the pointwise-atom refinement)
are unchanged and still open.

---

## 6. Proposed CT mappings — for the integrator, not applied

`docs/reevaluation/ct_manifest.json` is **not edited**. Each row states the
stages this card actually executes and the exact ordered diagnostic vector its
fixture produces. The CT harness fails closed on a fixture directory whose case
still reads `unmapped`, so no fixture files are placed under
`compiler/tests/fixtures/ct/`; the source of each is inline in the test named
below.

| Case | Scenario | Fixture source | Measured result | Proposed stages |
|---|---|---|---|---|
| **CT28** | guard moved to a child / unlocked by another thread → Reject | `a_child_task_cannot_act_through_the_guard_it_lexically_sees` (`crates/qcore-frontend/tests/source_mutex.rs`); located variant in `the_composed_refusals_are_located_on_their_own_statements` (`compiler/tests/checked_mutex_guards.rs`) | `["Q612","Q600"]` = `WrongGuardAffinity { event: 3, guard: 1, task: 1 }` then `JoinWhileGuardHeld { event: 4, task: 0 }`; `Q612` located at the child's own statement | parser, type_checker **executed**; cfg, reference_execution, native_backend, proof **unassessed** |
| **CT31** | mutex destroyed with an outstanding waiter → Reject | `an_announced_acquisition_blocks_the_generations_destruction`; located variant in the compiler test | `["Q619","Q600","Q600","Q600"]` = `NonquiescentMutexDestruction { event: 4, mutex: 1 }`, `NonquiescentDestruction { event: 5, resource: 1 }`, `OutstandingMutex { mutex: 1 }`, `OutstandingWaiter { mutex: 1, task: 1 }` | parser, type_checker **executed** |
| **CT32** | join while a guard is held → Reject | `a_child_task_created_inside_a_critical_section_cannot_be_joined_there` | `["Q600"]` = `JoinWhileGuardHeld`, and nothing else — the child itself is well formed | parser, type_checker **executed** |
| **CT48** | stale refinement across release/reacquire → Reject | `a_guard_scoped_fact_does_not_survive_the_release_that_dropped_it`; located variant in the compiler test | `["Q616"]` = `StaleProtectedFact`, located at the `use f;` statement | parser, type_checker **executed** |

Every row carries an accepted control that differs by one statement: CT28 and
CT32 move the child block out of the critical section; CT31 has the announcing
task perform the acquisition; CT48 re-asserts the fact under the second guard.
**Three of the four controls have no native evidence**, because their traces
carry task structure and this backend emits one thread (§3.5). CT48's control
is single-task and does run natively. `cfg` is `unassessed` for all four, for
T03's reason: this route has no CFG stage.

If all four are accepted, CT coverage would become **24/50 mapped**. This card
claims **20/50, unchanged**.

Still unmapped after this card, with the reason:

* **CT33** (lock-order cycle) — rank mode is a represented refusal, not a
  feature; unchanged.
* **CT34** (protected graph grows) — no growth form; unchanged.
* **CT46** — needs `Send`/`Sync` derivation, outside L09; unchanged.

---

## 7. Gate results

Run at this card's tip, on Linux x86_64, submodule initialised at the pinned
`d82464a`.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1494 passed, 0 failed, 1 ignored** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 -B scripts/check_lean_audits.py` | **Passed 34 Lean audits** |
| `cd proof && lake build` | success, no warnings |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **58 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests` | **95 tests, OK** |

### 7.1 Test counts

| Suite | Before | After |
|---|---|---|
| `crates/qcore-frontend/tests/source_mutex.rs` | 22 | **38** |
| `crates/qcore-core` unit `mutex_c_emit` | 3 | **5** |
| `compiler/tests/checked_mutex_guards.rs` | 4 | **7** |

1494 − 1473 = **21 new tests**, exactly the 16 + 2 + 3 added here. The Lean
audit count moves 34 → 34: this card adds no new `Audit*.lean` file, and its 19
new registrations land inside the existing `AuditMutexInvariant.lean`, whose
`#print axioms` count moves 45 → 64.

### 7.2 Sanitizer runs

Two new fixed runs, both compiled `-std=c11 -Wall -Wextra -Werror -pedantic
-fsanitize=address,undefined` and run with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1` (LeakSanitizer left
**enabled**), each compared line for line with the reference execution computed
from the checked event list:

1. core IR: one section, a guard-scoped fact asserted and used, a read-only
   open region closed with the checked witness;
2. **from source** through `compile_files`: the same shape written in QCore,
   through parse → lower → check → recheck → emit → run.

The four pre-existing T03 runs are unchanged. As before: one thread, one Linux
target, one compiler invocation — no contention, no schedule, no memory-model
evidence, and the invariant predicate has no runtime representation.

### 7.3 Honest notes

* The `third_party/libevent` submodule was absent in this worktree on the first
  workspace run and `libevent_submodule_matches_the_audited_revision` failed;
  `git submodule update --init --recursive` restored it at the pinned revision
  and the gate was re-run. That failure was environmental, not a finding.
* The load-flaky P02 test
  `a_declared_process_limit_stops_the_fixture_from_forking` is unchanged and
  untouched here; see [remaining-p03 §9.1](../remaining-p03/README.md).

### 7.4 Pinned artifacts

* The four pinned 185-unit Cycle 19 artifacts were **not re-measured**, and by
  construction rather than by measurement: this card touches only the
  source-language route inside `compile_files`, the mutex model and its
  backend. No gateway, contract, profile, schema, provider or audit file was
  edited. `CORPUS-REPIN` remains open.
* The contract corpus stays at **60 modules**; `docs/reevaluation/ct_manifest.json`
  is untouched at **20/50 mapped**; `qcore-schemas` stays **5.0.0** and
  `qcore-compiler` **0.2.0**. No raw Cycle 19 unit changes verdict.

---

## 8. Five new reserved words, and why that is a risk worth naming

`unchanged`, `fact`, `assert`, `use` and `wait` joined
`is_reserved_source_word`; `assert`, `use` and `wait` joined
`is_statement_start`. T03 §1.1 records that reserving `shared` broke a *pinned*
program, so this is a real hazard and not a formality. All five were checked
against every `.qcore` file and every inline source fixture in the repository
before landing, and the full workspace suite is the standing check. `use` is
the one to watch: it already appears as a grade keyword in parameter types
(`own<Buffer> use one`), which parses inside a group `skip_group` skips and in a
declaration header `referenced_names` drains — measured, not assumed, by the
suite passing.

---

## 9. What this card does not establish

* **No composition with the scoped task ledger.** The child-task form is
  composed inside the mutex model only. Captures, owned values, completion
  capsules, task results and typed error payloads have no representation here
  and are refused.
* **No native evidence for any multi-task module.** The backend emits one
  thread. Everything in §3 is model-level plus source-level, with no contention,
  no schedule and no C memory-model claim.
* **No source-to-trace refinement.** Unproved in Rust and in Lean, as before.
* **The checked witness removes one premise on the sections that qualify.**
  `DeclaredFootprintShape` is on every certificate this route issues, and
  `InvariantPredicateDeclared` is on every section that closes by declaration.
* **No general fact language.** A fact has no predicate: it is an identity
  scoped to a guard and an interval. Nothing checks what it *says*.
* **`TrustedExplicitTrace` is untouched and not retired.**
* **No promotion, no CT mapping, no pinned-artifact change.**

## 10. Open items carried forward

1. Composition with `ScopedCopyTasksV1`/`ScopedOwnedTasksV1` — captures, owned
   values and completion capsules — remains refused (T03 §10 item 2, narrowed:
   the mutex-side composition now exists, the capsule-side does not).
2. Multi-threaded native emission for the guard profile, and with it native
   evidence for CT28/CT31/CT32's controls.
3. A predicate language for facts, so a fact says something checkable.
4. The Rust checker's happens-before conditions on destruction are outside the
   Lean model (§5.2 item 2).
5. Non-Linux/macOS/Windows emission is still refused rather than implemented.
6. `research_closure.rs` still does not record which profile an embedded
   concurrency certificate came from (carried over from T02 §6 and T03 §10).
7. The four concurrency diagnostic bands are still not recorded in
   `docs/QCORE_SPEC_r0_3.md` §L21 (carried over from the third delegated
   increment's integration notes; not this card's edit to make).
