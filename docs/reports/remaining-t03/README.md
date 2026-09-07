# T03 — `source.mutex.guard.v1`: lowering one lexical guard onto `CheckedMutexInvariantV1`

Date: 2026-09-08. Base checkpoint: `a4492da` (the remaining-work wave tip).
Depends on [T02](../remaining-t02/README.md) (the model checker being lowered
onto), [T01](../remaining-t01/README.md) (the profile design, §4.3), and the
[W6 frontend gates](../w6-frontend-gates/README.md) (body-local guard binders).

**Scope.** One bounded source surface, one Linux backend. This card adds a
source route; it does not implement general source concurrency, does not retire
`TrustedExplicitTrace`, does not promote any raw Cycle 19 unit, and maps no CT
case (mappings are *proposed* in §8 for the integrator, and
`docs/reevaluation/ct_manifest.json` is untouched).

---

## 1. What landed

| Artifact | Lines | Role |
|---|---|---|
| `crates/qcore-frontend/src/source_mutex.rs` | 820 | the lowering: grammar recognition, event construction, diagnostic location |
| `crates/qcore-frontend/tests/source_mutex.rs` | 453 | 22 source-level tests |
| `crates/qcore-core/src/mutex_c_emit.rs` | 336 | native C11 backend **and** the independent reference execution |
| `crates/qcore-core/tests/unit/mutex_c_emit.rs` | 303 | 3 tests, including the sanitized native runs |
| `compiler/tests/checked_mutex_guards.rs` | 203 | 4 end-to-end driver tests, one of them a sanitized native run from source |
| `proof/QCore/MutexInvariant.lean` §11 | +362 | 14 new public theorems, 9 private lemmas, 8 decidable lowered scenarios |
| `proof/AuditMutexInvariant.lean` | +24 | 14 new `#print axioms` registrations |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c | +65 | the normative admitted grammar |
| `docs/QCORE_SPEC_r0_3.md` §L16 | +2 ¶ | records the checked profile and this one lowered surface |

Edits to existing production files are five, all additive:

| File | Change |
|---|---|
| `crates/qcore-frontend/src/frontend.rs` | five new reserved words (`mutex`, `guards`, `protected`, `close`, `borrow`) and four new statement leaders in `is_statement_start` (`open`, `close`, `read`, `write`). No parser rule, no diagnostic code, no elaboration rule changed. |
| `crates/qcore-frontend/src/lib.rs` | `pub mod source_mutex;` |
| `crates/qcore-core/src/lib.rs` | `pub mod mutex_c_emit;` |
| `crates/qcore-verification/src/driver.rs` | route selection, `CheckedSourceModule::CheckedMutexGuards`, `SourceExecutionProfile::CheckedMutexGuardsV1`, `compile_mutex_source` |
| `compiler/src/lib.rs`, `crates/qcorec/src/main.rs` | re-export and the `check` label |

`check_concurrent`, `check_mutex_invariant_program`,
`recheck_mutex_invariant_certificate`, the 26 pinned `TrustedExplicitTrace`
tests, T02's 48 tests and the `Lifetime` Lean namespace are **unedited**.

### 1.1 One reserved word was rejected, and why it matters

The first cut used `shared` for the protected-resource declaration. That broke
the pinned golden surface in `crates/qcore-frontend/tests/unit/frontend.rs`,
which contains `fn future(shared: i32) … spawn s work(copy shared)`: reserving
the word made a *pinned* program stop parsing. The pin was not edited. The
keyword was changed to `protected`, which collides with nothing in the
repository. The four statement leaders added to `is_statement_start` are
required for correctness, not convenience: `last_top_level_block` scans forward
until the next statement leader, so a `read`/`write`/`open`/`close` statement
between two `with lock` blocks would otherwise have made the *second* block the
body of the *first*.

---

## 2. The admitted grammar, exactly

Normative copy in
[`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02c-guarded-fixed-footprint-mutex).

```ebnf
module = { error_declaration } , entry ;
entry  = "fn" , NAME , "(" , ")" , "->" , "unit" , "{" , scope , "}" ;
scope  = "thread" , "scope" , NAME , "{" , { outer } , "}" ;
outer  = "let" , NAME , "=" , "protected" , NAT , ";"
       | "let" , NAME , "=" , "mutex" , "guards" , NAME , ";"
       | ( "read" | "write" ) , NAME , ";"
       | "with" , "lock" , NAME , "as" , NAME , "{" , { inner } , "}" ;
inner  = ( "open" | "close" ) , NAME , ";"
       | ( "read" | "write" ) , NAME , "with" , NAME , ";"
       | "let" , NAME , "=" , "borrow" , NAME , "with" , NAME , ";"
       | "drop" , NAME , ";"
       | "raise" , NAME , "::" , NAME , ";" ;
```

A `raise` must be the block's last statement. Everything else is refused.

### 2.1 Statement → event

| Source | Event | Identity |
|---|---|---|
| `let R = protected N;` | (none) | resource id `1..`, footprint `0..N`; grants the root task `FULL_ACCESS` and custody |
| `let M = mutex guards R;` | `CreateMutex` | mutex id `1..`; `protects = [0..N]`; `invariant = mutex` |
| `with lock M as G {` | `Lock` | a fresh guard id per block, never reused |
| `open G;` / `close G;` | `OpenInvariant` / `CloseInvariant{Declared}` | |
| `read R with G;` / `write R with G;` | `Access{guard: Some}` | |
| `read R;` / `write R;` | `Access{guard: None}` | |
| `let B = borrow R with G;` / `drop B;` | `BorrowUnderGuard` / `EndBorrow` | fresh borrow id |
| `raise E::V;` | `ErrorExit{exit}` | exit id `1..`, name retained in `error_exits` |
| `}` (block end, no raise) | `Unlock` | |
| `}` (scope end) | `DestroyMutex`* then `Destroy`* | reverse declaration order |

`close G;` always supplies `InvariantWitness::Declared`. The checked
`NoWriteOccurred` witness is deliberately *not* selected by the lowering: a
dataflow-dependent witness choice would make the premise set depend on an
analysis this card does not perform. The cost is recorded: every certificate
from this route carries `InvariantPredicateDeclared`.

### 2.2 Refusal taxonomy

Two kinds, deliberately separated so that "the profile refuses X" is testable
rather than "the profile cannot say X":

* **Represented refusals** — the construct is lowered to a real event that the
  *core checker* rejects. A nested `with lock` becomes
  `UnsupportedConstruct{NestedProtectingMutex}` → `Q622`.
* **Grammar refusals** — the construct has no event, and the lowering itself
  refuses at `Q600` with a message naming the profile. `defer` inside a
  critical section, `while`, `return`, a non-`unit` entry, a source
  `contract { … }`, a statement after `raise`, and any `spawn`/`join` anywhere
  in the module are all here.

No code was allocated: `Q6xx` is already the `Concurrency` pass's range
(`docs/QCORE_SPEC_r0_3.md` §L21), `Q610`–`Q623` are T02's, and `Q600` is the
existing out-of-profile concurrency refusal that `source_tasks.rs` already
uses. Name-resolution failures stay `Q013` and are *not* re-labelled: a guard
mentioned after its block, or a mutex that names an undeclared resource, is a
name error, not a concurrency verdict.

---

## 3. Lowering path

```
parse_source_program                     (qcore-frontend)
  → elaborate_source                     guard binders are body-local (W6)
  → has_source_mutex                     route selection, before the task route
  → lower_source_mutex                   → MutexInvariantProgram + per-event spans
  → check_mutex_invariant_program        (qcore-core, T02, unedited)
  → recheck_mutex_invariant_certificate  the driver re-derives before publishing
  → emit_mutex_c11                       (qcore-core, Linux only)
```

Route selection is `mutex` immediately followed by `guards` in the token
stream. It runs **before** `has_source_tasks`, because a bare `thread scope`
also selects the task route and a module declaring a mutex generation must not
be handed to a route that models no mutex at all. Detection selects a checker;
it grants nothing. A detected module that leaves the grammar is refused, never
forwarded.

`compile_mutex_source` refuses to publish a certificate it did not
independently re-derive, and refuses emission on a non-Linux target (`Q500`)
rather than emitting something unchecked.

Diagnostics are located by a parallel `event_spans` vector: a core refusal
naming event *i* is reported at statement *i*'s own span, under the core's own
code. End-of-trace refusals (`OutstandingGuard`, `OutstandingMutex`) name the
`thread scope` statement. The `compiler/tests/checked_mutex_guards.rs` tests
pin the line numbers, so attribution is measured rather than asserted.

---

## 4. Acceptance

### 4.1 A guard-derived borrow cannot escape, move to a child task, or survive unlock

Four independent mechanisms, each with its own test:

| Escape route | Outcome | Where |
|---|---|---|
| naming the guard after its block | `Q013` (name resolution) — the binder is body-local | `a_guard_name_does_not_exist_after_its_block` |
| a borrow still live at the block end | `Q615` `GuardBorrowEscape`, leading the fail-closed cascade `["Q615","Q619","Q600","Q600","Q600"]` | `a_borrow_that_outlives_its_critical_section_is_refused` |
| any task-creation form in the module | `Q600`, "completion-capsule composition" — task creation is not admitted at all | `task_creation_composed_with_a_checked_mutex_is_refused` |
| a child acting through the parent's guard | `Q612` `WrongGuardAffinity` from the core, on the program this route lowers | `a_child_task_cannot_act_through_its_parents_guard` |

The fourth is honest about its own limit: since the grammar has no task form,
the escape is exercised by splicing `Spawn` + a guarded `Access` by the child
into the **lowered** program and checking that the core refuses it. The source
refusal is the reason it is unreachable from the grammar; the model refusal is
the reason it would be caught if it ever became reachable. Neither is a claim
that a guard is unmovable in some future profile that admits tasks.

The full cascade is pinned rather than only its first element. A refused
release leaves the guard live, which blocks the scope-exit destruction of the
generation and of the resource it parks, and surfaces in the end-of-trace
sweep. Pinning all five entries proves the refusal did not silently release the
guard.

### 4.2 Typed-error cleanup restores the invariant before unlock; unsupported cleanup is rejected

| Fixture | Outcome |
|---|---|
| `open; write; close; raise Fault::Broken;` | **accepted**; `ErrorExit{exit: 1}` with `Fault::Broken` retained |
| `open; write; raise Fault::Broken;` | `Q614` `InvariantOpenAtErrorExit` (+ cascade) |
| `open; write; }` (normal end) | `Q613` `InvariantOpenAtUnlock` (+ cascade) |
| `defer buf; …` inside the section | `Q600` — unsupported cleanup, refused, not dropped from the trace |
| `raise …; read buf with g;` | `Q600` — nothing may follow the typed-error exit |
| `close g;` with no preceding `open g;` | `Q613` `InvalidInvariantWitness` |
| `write buf with g;` with the invariant closed | `Q613` `WriteWithClosedInvariant` |

`Q614` and `Q613` stay distinct codes: the error path is named separately from
the normal path, which is the whole reason T02 has both. The Lean theorem
`typed_error_exit_obeys_the_normal_release_rule` proves the two exits are
subject to the *same* release rule, so the separate code is attribution, not a
weaker check.

### 4.3 Positive and negative source cases, native/reference checks, and their boundaries

Positive (all check, and the certificate is independently re-derived):

* `open` / `write` / `close` over one generation;
* two critical sections over one generation, minting guards 1 and 2;
* a guard-derived borrow taken and released inside its own section;
* an unprotected resource written without any guard, beside a protected one;
* a typed-error exit after the invariant is restored.

Negative, each asserting the exact diagnostic vector:

* `Q610` an unrelated lock over another mutex's footprint (**the R03 sentence,
  reached from source**);
* `Q611` an unguarded write to a protected footprint — no fallback authority;
* `Q613` write-with-closed-invariant, double `open`, `close` without `open`,
  normal exit with a broken invariant;
* `Q614` typed-error exit with a broken invariant;
* `Q615` borrow escape;
* `Q617` a second `mutex guards` over one resource;
* `Q622` a nested critical section;
* `Q600` six out-of-grammar constructs, plus `Q013` for two unresolved names.

**Native evidence.** Four fixed scenarios are compiled with
`-std=c11 -Wall -Wextra -Werror -pedantic -fsanitize=address,undefined` and run
with `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`. LeakSanitizer is left
**enabled** here, because reclaiming every emitted allocation is exactly what
these runs must witness.

| Run | Fixture | What it exercises |
|---|---|---|
| 1 | `open/write/close` (core IR) | one generation, one section, `mtx_init`/`lock`/`unlock`/`destroy`, `malloc`/`free` |
| 2 | two sections + borrow + unprotected resource (core IR) | guard identity across a release, a live borrow pointer, two allocations |
| 3 | typed-error exit after `close` (core IR) | the release is emitted on the error path and the allocation is still freed |
| 4 | two sections + borrow, **from source** through `compile_files` | the whole stage stack: parse → lower → check → emit → run |

Each run's stdout observation is compared line for line with
`interpret_mutex_program`, which is written against the checked event list and
does not read the emitted C. Run 3 additionally asserts, on the emitted text,
that exactly one `mtx_unlock(&m…)` is emitted on the error path and that it
precedes `free(r1)`.

Boundaries, stated as limits and not as caveats:

* **One thread.** The profile admits no task creation, so `mtx_lock` is never
  contended in these runs. Nothing here is evidence about scheduling, fairness,
  publication or the C memory model.
* **One target, one compiler invocation.** Linux, `cc` (or `QCORE_CLANG`).
  Non-Linux emission is refused at `Q500`, not approximated.
* **The invariant predicate is not executed.** `open`/`close` have no runtime
  representation; they are traced only so the two observations stay comparable.
  Their meaning is the `InvariantPredicateDeclared` premise.
* **Source-to-trace refinement is a premise.** That `lower_source_mutex`
  produces the trace the source means is not proved, in Rust or in Lean.

---

## 5. Backend refusals

`emit_mutex_c11` and `interpret_mutex_program` both refuse, identically and
before producing anything:

* a certificate whose `profile` is not `CheckedMutexInvariantV1`
  (`UncheckedProfile`) — a `TrustedExplicitTrace` certificate cannot authorize
  emission;
* any event outside the single-task subset — `Spawn`, `Join`,
  `TransferCustody`, `RegisterWaiter`, `UnsupportedConstruct`,
  `ExtendProtection`, `AssertFact`, `UseFact`, atomic accesses
  (`UnsupportedEvent { event }`);
* a footprint whose extent no initial grant declares (`UndeclaredResource`).

`emission_refuses_anything_outside_the_single_task_subset` pins all of these.

---

## 6. Lean

New `§11` in the existing `QCore.Concurrency.MutexInvariant` namespace. `step`,
`run`, `canRelease`, `releaseEffect` and every T02 theorem are unedited.

| Theorem | Obligation |
|---|---|
| `lowerSection_length` | `n` source statements lower to exactly `n + 2` events — nothing dropped, nothing invented |
| `lowerSection_brackets` | every block begins with its acquisition and ends with its exit |
| `lowerSection_head` | the first event is the `Lock` that mints the guard |
| `lowerSection_emits_no_task_event` | no `spawn`/`join`/`transferCustody` in a lowered block — a guard has nothing to travel on |
| `lowerScope_emits_no_task_event` | the same for the whole scope |
| `lowerSection_emits_no_refused_construct` | no `extendProtection`/`unsupportedConstruct` is produced by an admitted statement |
| `lowerSection_mints_exactly_its_own_guard` | one authority per critical section |
| `lowerScope_mints_one_guard_per_section` | the scope's minted guards are exactly its sections' guards, in order |
| `lowerScope_destroys_every_declared_generation` | scope exit attempts destruction of every declared generation |
| `run_append` | the trace decomposition the exit lemmas need |
| `typed_error_exit_obeys_the_normal_release_rule` | `errorExit` and `unlock` have the *same* transition |
| `lowered_exit_refuses_an_open_invariant` | neither exit admits a broken invariant |
| `lowered_exit_refuses_a_live_borrow` | neither exit admits a live guard-derived borrow |
| `accepted_section_leaves_no_live_guard` | an accepted block leaves no live guard behind |

Nine private lemmas support them. Eight decidable `example`s mirror the Rust
source fixtures (accepted `open/write/close`; the same block without `close`
under both exits; the restored error exit; the unreleased and released borrow;
two sections over one generation; a write with the invariant closed).

`AuditMutexInvariant.lean` registers the fourteen; `check_lean_audits.py`
passes **21** audits with warnings as errors, and the axiom closure is
`propext` / `Quot.sound` only. **No new axiom, no `sorry`, no placeholder.**

### 6.1 Named Lean boundaries

1. **`Event` has no resource-destruction constructor.** T02's Lean model
   destroys mutex generations but not resources, so `lowerScope` models
   generation lifetime only. The Rust lowering additionally emits `Destroy` for
   each declared resource and the Rust checker applies its quiescence rule; that
   half of scope exit is covered by the Rust tests, not by these theorems.
2. **Source extraction is a premise.** No theorem says the Rust lowering
   produces `lowerSection` for the source it reads.
3. T02's two named-open obligations (`step_preserves_declared_shape` and the
   pointwise-atom refinement) are unchanged and still open.

---

## 7. Gate results

Run at this card's tip, on Linux.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1026 passed, 0 failed, 0 ignored** across 75 suites (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **47 passed, 0 failed** across 16 suites |
| `python3 -m unittest discover -s scripts/tests …` | **43 tests, OK** |
| `cd proof && lake build` | success, **no warnings** |
| `python3 -B scripts/check_lean_audits.py` | **Passed 21 Lean audits** |

1026 − 997 = **29 new tests**, exactly the 22 + 3 + 4 added here.

### 7.1 One flaky pre-existing test, reported honestly

`qcore-verifier-host`'s
`a_declared_process_limit_stops_the_fixture_from_forking` (P02) failed on one
intermediate run of this session with `Io (Broken pipe)` instead of
`Terminated`, and passed on the next. It was then run three times in isolation
at the same tip: **fail, pass, fail**. It was also run at the *unmodified* base
with the whole worktree stashed, where it passed. The test installs
`RLIMIT_NPROC = 1`, which — as its own comment states — counts every process of
the real UID, so its outcome depends on the machine's current process count
rather than on anything in this change. Nothing in this card touches
`qcore-verifier-host`, process limits or the provider host. It passed in both
full `cargo test --workspace` runs recorded above. This is recorded as a
pre-existing environment-dependent flake, not repaired here.

### 7.2 Pinned artifacts

* **Not re-measured, and this is by construction rather than by measurement.**
  The four pinned 185-unit Cycle 19 artifacts and the configured CMetrics audit
  were not re-run: the pinned corpus lives outside this worktree. The route
  added here is a *source-language* compilation route inside `compile_files`;
  the C audit path (`qcore-audit` → `qcore-c-gateway`) does not call it, and no
  gateway, contract, profile, schema or provider file was edited.
* The contract corpus stays at 59 modules, the `third_party/libevent` submodule
  pin is unchanged, and `docs/reevaluation/ct_manifest.json` is untouched at
  11/50 mapped.
* No raw Cycle 19 unit is promoted. `qcore-schemas` stays 3.0.0; protocol v1,
  evidence v2 and program schema/profile v2 are unchanged.

---

## 8. Proposed CT mappings — for the integrator, not applied

`docs/reevaluation/ct_manifest.json` is **not edited** by this card, and the CT
harness fails closed on a fixture directory whose case still reads `unmapped`,
so the candidate fixtures live in
[`fixtures/`](fixtures/) here rather than under `compiler/tests/fixtures/ct/`.
Each row below states the stages this card actually executes; the manifest's
own convention (`CT02`, `CT19`: rejection cases claim `parser` and
`type_checker` only) is followed.

| Case | Fixture | Measured result | Proposed stages |
|---|---|---|---|
| **CT26** two critical sections, same mutex, same payload → Accept | `fixtures/CT26/case.qcore` | `checked 1 function(s); entry main; checked mutex guards (Linux C11)`, guards 1 and 2 | parser, type_checker, reference_execution, native_backend **executed**; cfg, proof **unassessed** |
| **CT27** two mutex identities over one payload → Reject | `fixtures/CT27/case.qcore` | `Q617` then `Q600 UnknownMutex` | parser, type_checker **executed** |
| **CT29** typed error leaves a broken invariant → Reject unless restored | `fixtures/CT29/case.qcore` + `restored.qcore` control | `Q614` + cascade; the control checks | parser, type_checker **executed** |
| **CT30** guard-derived borrow escapes → Reject | `fixtures/CT30/case.qcore` + `released.qcore` control | `Q615` + cascade; the control checks | parser, type_checker **executed** |

`cfg` is proposed as `unassessed` for CT26 on purpose: this route emits
directly from the checked event list and has no CFG stage at all, so claiming
`executed` there would be false.

**Deliberately not proposed**, with the reason:

* **CT28** (guard moved to a child / unlocked by another thread) — the source
  refusal is "this module creates a task, which this profile does not admit"
  (`Q600`), which is weaker than the case asks. The `Q612` evidence is
  model-level only.
* **CT31** (mutex destroyed with an outstanding borrower/waiter) — `Q619`
  `NonquiescentMutexDestruction` does appear, but only as the *second* entry of
  CT30's cascade, and no source form produces a waiter. Insufficient on its own.
* **CT32** (join while a guard is held) — no join form in this profile.
* **CT33** (lock-order cycle) — rank mode is a represented refusal, not a
  feature.
* **CT34** (protected graph grows) — a growth form is not expressible; the
  nearest source case is CT27's second `guards`, which is a different scenario.
* **CT48** (stale refinement across release/reacquire) — the grammar has no
  fact statement. The model rule and its T02 test are unchanged.
* **CT46** — needs `Send`/`Sync` derivation, outside L09.

If all four proposed rows are accepted, CT coverage would become 15/50 mapped.
This card claims **11/50, unchanged**, until the integrator applies them.

---

## 9. What this card does not establish

* **No general source concurrency.** One entry function, one thread, one
  `thread scope`, no calls, no control flow, no task creation, no shares, no
  condition variables, no try/timed locks, no reader/writer locks, no lock
  ranks, no nested protecting mutexes, no `Send`/`Sync` derivation. Every one
  of those is a refusal with a named code, not an omission.
* **No source-to-trace refinement.** That the lowering means what the source
  means is a premise in both Rust and Lean.
* **No deadlock freedom, no contention evidence.** A single-threaded
  acquisition/release pair proves pairing and extent, nothing about schedules.
* **No new premise removal at the language level.** `DeclaredFootprintShape`
  and `InvariantPredicateDeclared` are on every certificate this route issues,
  and `close g;` is what puts the second one there.
* **`TrustedExplicitTrace` is untouched and not retired.** Invariant 3 (parity
  before deletion) still applies to it.
* **No promotion.** No raw Cycle 19 unit changes verdict; no pinned artifact,
  corpus count or CT mapping changes.

## 10. Open items

1. The `NoWriteOccurred` witness is unreachable from source; selecting it when
   no write occurred would drop `InvariantPredicateDeclared` from those
   certificates, and needs a small dataflow pass over the block.
2. Composing this profile with `ScopedCopyTasksV1`/`ScopedOwnedTasksV1` — the
   prerequisite for CT28, CT31's waiter half and CT32 — is refused today
   (`CompletionCapsuleComposition`).
3. Guard-scoped refinement facts (`AssertFact`/`UseFact`, CT48) exist in the
   model with no source form.
4. Resource destruction has no Lean event; §6.1 item 1.
5. Non-Linux emission is refused rather than implemented.
6. `research_closure.rs` still does not record which profile an embedded
   concurrency certificate came from (carried over from T02 §6).
