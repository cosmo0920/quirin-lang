# T03-GUARD-JOIN — a task created and joined strictly inside a guard

Date: 2026-09-09. Base checkpoint: `05c51a5` (wave 8).
Follow-on of [T02](../remaining-t02/README.md) (`CheckedMutexInvariantV1`) and
[T03-F](../remaining-t03-f/README.md), which recorded this rule as *not lifted*.

**Outcome.** The soundness argument closes, but only for a rule much narrower
than the card's suggested one. It is implemented as an **opt-in** admission:
`JoinWhileGuardHeld` still refuses every plain `Join` while a guard is held —
byte-identically, so CT28 and CT32 are untouched — and the admission is
requested by a distinct event (`GuardLocalJoin`) and a distinct source form
(`thread scope c under g { … }`). Nothing was weakened; a refusal was given one
checked exception with eight conditions.

---

## 1. The model argument

### 1.1 What the blanket refusal was actually protecting

The card names three candidate hazards. Two of them turn out to be already
covered by other rules, and the third turns out to be invisible to the model —
which is exactly why the blanket refusal had to exist.

**Custody or authority flowing across the join while the invariant is open.**
Not a hazard the join creates. `CreateMutex` *consumes* the creator's
`FULL_ACCESS` over every declared interval and parks it in the mutex, so while a
generation is live no task holds any share of a protected atom; the pointwise
conservation invariant makes that a fact about the ledger, not a convention. A
child therefore cannot reach a protected footprint except through a guard of its
own, and `Access` with `guard: None` on a protected footprint is
`UnguardedProtectedAccess` with no fallback path. Custody is not consulted for
access authority at all in this profile — it governs `Destroy` and
`TransferCustody` — and `Destroy` already refuses while any live mutex protects
part of the resource.

**Guard-scoped facts and borrows flowing into the child.** Also already covered,
with one measured exception. `Access`, `OpenInvariant`, `CloseInvariant`,
`BorrowUnderGuard`, `AssertFact` and `UseFact` all test `state.holder == task`
and refuse a foreign task with `WrongGuardAffinity` or `StaleProtectedFact`. The
exception is `EndBorrow`, which checks only that the acting task is live — the
asymmetry T03-F recorded, where a child's release of its parent's borrow is a
*grammar* refusal in the lowering but would be **accepted** by the model. That
exception is load-bearing for the rule below and is why condition G7 is stated
over the whole event vocabulary rather than over acquisition alone.

**Deadlock.** This is the real one, and the model cannot see it. A join is a
blocking wait, and nothing in `CheckedMutexInvariantV1` blocks: `canLock`
refuses an acquisition of a held generation outright (`DuplicateLockCapability`)
rather than parking the acquirer, and the *representable* form of "task `t` is
waiting for mutex `m`" is `RegisterWaiter`. A trace is a linearisation of one
execution; if the real execution deadlocks, the trace describes a run that never
happened, and a certificate over it would be vacuous. The blanket
`JoinWhileGuardHeld` refusal is the honest response to a hazard the state
machine has no vocabulary for.

### 1.2 Why the card's suggested rule is too wide

The card proposes: same guard region, no share of the guard's mutex, *no
`with lock` on the same mutex in the child*, no guard fact or borrow in, no
authority over protected footprints in the result.

Two of those clauses are vacuous here (no task can hold a share of a protected
atom while the generation lives; this profile has no completion-result value at
all — the post-join custody transfer is a separate `TransferCustody` event that
the ordinary rules already govern). One is too weak: **"no `with lock` on the
same mutex" is not enough to exclude deadlock.** Suppose the child locks a
*different* generation `m2`. In a two-party trace that is harmless, but in a
general trace some third task may hold `m2` and be a recorded waiter on `m1`,
closing the cycle *parent → child → T → parent*. Deciding that requires a
wait-for graph over every task in the trace, and lock-order analysis is one of
this profile's explicitly named exclusions (a lock-rank declaration is an
`UnsupportedSynchronization` refusal). Widening to "any mutex other than this
one" would therefore be admitting a case the profile has deliberately declined
to be able to decide.

### 1.3 The rule that is implemented

The narrow rule replaces "the child cannot take *this* mutex" with "the child
subtree never names a mutex at all". That converts the deadlock question from an
analysis into a structural impossibility: a subtree that names no mutex, no
guard and no guard-scoped fact anywhere in the trace can never be waiting for
one, so the parent's blocking wait terminates as soon as the child's finite
event sequence ends.

`GuardLocalJoin { parent, child, guard }` is admitted iff:

| | Condition | Refusal when it fails |
|---|---|---|
| **G1** | every ordinary join precondition: `parent` active, `child` its unjoined child, child quiescent (holds no guard, has no unjoined descendant) | `UnknownJoin` / `NonquiescentJoin` (`Q600`) — unchanged |
| **G2** | `guard` is live and held by `parent` | `DeadGuard` / `WrongGuardAffinity` (`Q612`) |
| **G3** | `guard` is the **only** live guard `parent` holds | `JoinWhileGuardHeld` (`Q600`) |
| **G4** | the set of guards `parent` held when it spawned `child` was exactly `{guard}` | `JoinWhileGuardHeld` |
| **G5** | `guard`'s invariant is `Closed` at the join | `JoinWhileGuardHeld` |
| **G6** | `guard` has no live guard-derived borrow | `JoinWhileGuardHeld` |
| **G7** | neither `child` nor any descendant of `child` named a mutex, a guard or a guard-scoped fact anywhere in the trace | `JoinWhileGuardHeld` |
| **G8** | no live mutex records any task of that subtree as a waiter | `JoinWhileGuardHeld` |
| **G9** | no task of that subtree holds a nonzero share of any atom | `JoinWhileGuardHeld` |
| **G10** | no task of that subtree is custodian of a live resource | `JoinWhileGuardHeld` |

Notes on the individual clauses, each derived from the ledger rather than from
plausibility:

* **G4 is exact, not approximate.** Guard identities are minted fresh by every
  successful `Lock` and are never reused, including across two locks of one
  generation. So comparing the recorded spawn-time guard *set* against the
  current one distinguishes "created inside this section" from "created inside
  an earlier section over the same mutex" — which the pinned test
  `a_guard_local_join_of_a_child_created_in_an_earlier_section_is_refused`
  exercises. It also refuses the shape the original pinned test
  `joining_while_holding_a_guard_is_refused` uses (spawn before the lock), so
  opting in does not admit it.
* **G3** exists because two held sections would need the wait-for argument of
  §1.2 across both. One section, one wait, one subtree.
* **G5 is conservative and is labelled so.** Within the model it is not
  required: the child provably cannot observe protected memory, and a racing
  access would be caught by the existing `DataRace` check. It is required anyway
  because `DeclaredFootprintShape` is a *premise* — the model cannot rule out
  that the parent's open-invariant region disturbs memory the declaration does
  not mention, which the child, being real C, might read.
* **G6** closes the `EndBorrow` asymmetry from the parent's side, and **G7**
  closes it from the child's: a child that ends its parent's borrow is accepted
  by `EndBorrow` itself, and refused by the guard-local rule. That asymmetry is
  a pre-existing model gap, recorded again below as still open; the admission
  does not depend on it being fixed and does not make it reachable.
* **G9 and G10** hold vacuously for every trace this profile's own source
  lowering produces — this model has no share-transfer event at all, so shares
  never move between tasks. They are checked anyway so that "the child's
  completion carries no authority" is a fact read off the state, not an argument
  about another rule.

### 1.4 What the admission is, and is not, about

Because a child in this profile can hold no share and no custody, a guard-local
child cannot touch a declared resource at all. What the admission buys is the
**join**: a source program may now create a task strictly inside a critical
section, let it do work this profile does not model, and collect it there,
receiving a certificate — where previously the whole module was refused. It does
not give the child authority, and it does not claim the child's work is correct.

The admission adds **no assumption**. Every one of G1–G10 is checked; the
certificate's assumption set on an accepted guard-local module is exactly
`{InitialAuthorityAndUniqueLifetimeIds, DeclaredFootprintShape}`, the same set
any `CreateMutex` already produces, pinned by
`a_guard_local_join_of_a_mutex_silent_child_is_accepted`.

---

## 2. What landed

| Artifact | Change |
|---|---|
| `crates/qcore-core/src/mutex_invariant.rs` | `+ MutexInvariantEvent::GuardLocalJoin`, `+ AdmittedGuardLocalJoin`, `+ MutexInvariantCertificate::admitted_guard_local_joins`, the admission arm, `mutex_keying_task`, `guards_held_by`, `descendant_closure`, one recheck field, module docs for the rule |
| `crates/qcore-core/src/mutex_c_emit.rs` | `GuardLocalJoin` added to `is_single_task_trace`'s task-structure list |
| `crates/qcore-core/src/lib.rs` | `AdmittedGuardLocalJoin` re-export |
| `crates/qcore-core/tests/unit/mutex_invariant.rs` | 18 new tests (48 → 66) |
| `crates/qcore-frontend/src/source_mutex.rs` | `thread scope NAME under GUARD { … }` header, guard-local join emission, grammar docs |
| `crates/qcore-frontend/src/frontend.rs` | `under` reserved |
| `crates/qcore-frontend/tests/source_mutex.rs` | 10 new tests (38 → 48) |
| `proof/QCore/MutexInvariant.lean` | new §15: `GuardLedger`, `keyingTask`, `State.subtreeOf`, eight obligation predicates, `canGuardLocalJoin`, `stepGuardLocalJoin`, 24 public theorems, 13 decidable scenarios |
| `proof/AuditMutexInvariant.lean` | 24 `#print axioms` registrations |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | the new form documented under G02c |

**`Join` is byte-identical, measured.** `git diff 05c51a5 --
crates/qcore-core/src/mutex_invariant.rs` contains **zero deleted lines**: the
checker change is purely additive. The plain join arm of the checker is unedited; the
admission is a separate event. `check_concurrent`, its 26 pinned tests and the
`Lifetime` Lean namespace are untouched, as are `Q600`–`Q623`: **no diagnostic
code was added, moved or changed**, so `docs/QCORE_SPEC_r0_3.md` §L21 needs no
edit. `qcore-schemas` is untouched (`MutexInvariantCertificate` lives in
`qcore-core`, which has no serialization surface).

### 2.1 The one deliberate pin decision

None of the 48 pinned `mutex_invariant` tests changed, and none needed to. In
particular `joining_while_holding_a_guard_is_refused` still asserts the exact
vector `[JoinWhileGuardHeld { event: 3, task: 0 }]` — its child is spawned
*before* the lock, which G4 refuses even with the opt-in event, and it uses the
plain `Join` in any case. The certificate struct gained a field, which is why
`independent_recheck_rederives_the_certificate` and
`recheck_refuses_every_tampered_certificate_field` are worth naming: both still
pass unmodified, and a ninth tampered field is now covered by a new test.

---

## 3. Adversarial matrix

### 3.1 Core (`crates/qcore-core/tests/unit/mutex_invariant.rs`, 18 new)

Every row is one mutation of the accepted control — lock, spawn a mutex-silent
child, guard-local join, unlock — and asserts the **exact** diagnostic vector
unless noted.

| Case | Test | Result |
|---|---|---|
| accepted control | `a_guard_local_join_of_a_mutex_silent_child_is_accepted` | admitted; certificate names the join and its quiet subtree; assumption set unchanged; recheck rederives |
| plain `Join`, same trace | `a_plain_join_inside_a_guard_is_still_refused` | `[JoinWhileGuardHeld{3,0}]` — the opt-in is real |
| **child locks the creator's generation** | `a_guard_local_child_that_locks_the_creators_generation_is_refused` | `[JoinWhileGuardHeld{4,0}, DuplicateLockCapability{3,1}]` — the acquisition is refused *before* any deadlock is representable, and the join is refused too |
| child locks another, unheld generation | `a_guard_local_child_that_locks_another_generation_is_refused` | `[JoinWhileGuardHeld{6,0}]` — the child's own section is fine on its own terms; §1.2's reason |
| **child registers as a waiter** | `a_guard_local_child_registered_as_a_waiter_is_refused` | `JoinWhileGuardHeld{4,0}` present |
| **child created outside the section** | `a_guard_local_join_of_a_child_created_outside_the_section_is_refused` | `[JoinWhileGuardHeld{3,0}]` |
| child created in an earlier section over the same mutex | `a_guard_local_join_of_a_child_created_in_an_earlier_section_is_refused` | `[JoinWhileGuardHeld{5,0}]` |
| invariant open at the join | `a_guard_local_join_with_the_invariant_open_is_refused` | `[JoinWhileGuardHeld{4,0}]` |
| **live guard-derived borrow** | `a_guard_local_join_with_a_live_guard_borrow_is_refused` | `[JoinWhileGuardHeld{4,0}]` |
| **child acts through the creator's guard** | `a_guard_local_child_that_acts_through_the_creators_guard_is_refused` | `[WrongGuardAffinity{3,10,1}, JoinWhileGuardHeld{4,0}]` |
| **child ends the creator's borrow** | `a_guard_local_child_that_ends_the_creators_borrow_is_refused` | `[JoinWhileGuardHeld{5,0}]` — the recorded `EndBorrow` asymmetry, refused by G7 |
| join names another task's guard | `a_guard_local_join_naming_another_tasks_guard_is_refused` | `WrongGuardAffinity{4,10,0}` |
| join names no live guard | `a_guard_local_join_naming_no_live_guard_is_refused_as_a_dead_guard` | `[DeadGuard{2,10}, OutstandingTask{1}]` |
| two held sections | `a_guard_local_join_with_two_held_sections_is_refused` | `[JoinWhileGuardHeld{5,0}]` |
| quiet grandchild / noisy grandchild | `the_quiet_subtree_rule_reaches_grandchildren` | accepted with `quiet_subtree: [1,2]`; the waiting grandchild refuses |
| child holds share / custody | `a_guard_local_join_of_a_child_holding_share_or_custody_is_refused` | `[JoinWhileGuardHeld]` in both halves |
| forged certificate record | `recheck_refuses_a_tampered_guard_local_join_record` | three mutations (erased, wrong guard, widened subtree) all refuse with `FieldMismatch{admitted_guard_local_joins}` |
| **the whole G7 vocabulary** | `every_mutex_keying_event_kind_refuses_a_guard_local_join` | all **15** mutex-keying event kinds performed by the child refuse the join |

### 3.2 Source (`crates/qcore-frontend/tests/source_mutex.rs`, 10 new)

| Case | Test | Codes |
|---|---|---|
| accepted control | `a_guard_local_child_created_and_joined_inside_its_section_checks` | checks; one `GuardLocalJoin` and no `Join` in the lowered trace; assumptions unchanged |
| plain form inside a section | `the_plain_child_form_inside_a_section_still_refuses` | `["Q600"]` — **CT32's cascade, reproduced verbatim** |
| child takes a generation | `a_guard_local_child_that_locks_a_generation_is_refused` | `["Q622", "Q600"]` |
| child acts through the creator's guard | `a_guard_local_child_cannot_act_through_the_creators_guard` | `["Q612", "Q600"]` — **CT28's cascade shape** |
| `under` outside a section / naming a sibling's guard | `the_guard_local_header_must_name_its_own_enclosing_section` | `["Q013"]` both |
| `under` after the block ends | `a_guard_local_header_cannot_escape_its_section` | plain form checks; `under` is `["Q013"]` |
| quiet / noisy grandchild | `the_quiet_subtree_rule_reaches_a_grandchild` | checks; `["Q619","Q600","Q600","Q600","Q600"]` |
| open invariant | `a_guard_local_join_with_an_open_invariant_is_refused` | `["Q600"]` |
| live borrow | `a_guard_local_join_with_a_live_borrow_is_refused` | `["Q600"]` |
| child announces an acquisition of the held generation | `a_guard_local_child_that_waits_for_the_held_generation_is_refused` | `["Q619","Q600","Q600","Q600","Q600"]` |

Two measured observations worth naming. First, at the source level a child's
`with lock` is refused as `Q622 NestedProtectingMutex` before the join rule is
reached, because the child inherits the enclosing section lexically; the "child
locks the same generation" adversarial case is therefore exercised at the *core*
level, where the acquisition itself reaches `DuplicateLockCapability`. Second,
the `wait m;` cascades are five entries long and that length is the evidence: the
waiter stays outstanding and blocks the generation's destruction (`Q619`), so the
refusal demonstrably did not release anything.

---

## 4. Lean

New `QCore.Concurrency.MutexInvariant` §15. `State`, `Event`, `step` and `run`
are **unedited**; the guard-local rule is an additive decision procedure over
`State` plus a `GuardLedger` (spawn-time sections, mutex-keyed tasks), which is
how the Rust checker computes it too — from auxiliary maps beside the main
state. Every theorem above §15 is untouched.

**24 new public theorems, all registered in `AuditMutexInvariant.lean`, none
depending on anything beyond `propext` and `Quot.sound`. No new axiom, no
`sorry`, no placeholder.** `AuditAll.lean`'s whole-project closure still admits
only `propext`, `Classical.choice` and `Quot.sound` across 19,404 declarations.

What an accepted join does and does not disturb — the soundness half the card
asked for ("preserves conservation and drops nothing the invariant needs"):

| Theorem | Statement |
|---|---|
| `guard_local_join_effect_is_the_ordinary_join` | the admitted effect **is** `joinEffect`; the rule changes whether a join is admitted, never what it does |
| `guard_local_join_preserves_authority` | `authorityTotal` and `holdings` unchanged — conservation |
| `guard_local_join_preserves_declared_shapes` | `mutexes` and `custody` unchanged |
| `guard_local_join_drops_nothing_the_invariant_needs` | `guards`, `guardOf guard` and `guardsHeldBy parent` unchanged: every phase, borrow and guard-scoped fact survives |
| `guard_local_join_completes_exactly_the_child` | exactly the joined child becomes terminal |
| `accepted_guard_local_join_discharges_its_obligations` | all eight obligations were decided; none is a comment |
| `guard_local_join_keeps_the_creator_in_its_only_section` | on the far side the creator still holds exactly `[guard]`, closed and unborrowed |

Everything outside the rule keeps refusing:

`the_quiet_subtree_contains_the_child` (G7 non-vacuity) ·
`guard_local_join_requires_a_quiet_subtree` ·
`guard_local_join_refuses_a_keyed_descendant` ·
`guard_local_join_requires_creation_in_the_same_section` ·
`guard_local_join_requires_exactly_one_held_section` ·
`guard_local_join_requires_the_joining_task_to_hold_the_section` ·
`guard_local_join_requires_a_closed_invariant` ·
`guard_local_join_requires_no_live_borrow` ·
`guard_local_join_refuses_a_waiting_subtree` ·
`guard_local_join_refuses_a_sharing_child` ·
`guard_local_join_refuses_a_custodian_child` ·
`guard_local_join_refuses_a_guard_holding_child`

The ledger's own vocabulary, so G7 cannot be widened by omission:
`spawn_records_the_creators_open_sections` ·
`a_lock_is_recorded_as_mutex_keyed` ·
`a_registered_waiter_is_recorded_as_mutex_keyed` ·
`guarded_and_unguarded_access_are_recorded_differently` ·
`an_end_borrow_is_recorded_as_mutex_keyed`

Thirteen decidable scenarios mirror the Rust matrix, including a real grandchild
shown to be inside `subtreeOf`, and — the load-bearing one —
`step sectionHeld (.join 0 1) = none`: the **ordinary** join over the very same
state is still refused.

**No theorem was renamed, restated or retired.** The two obligations T02 left
open (`step_preserves_declared_shape` in general, and the pointwise-atom
refinement of the abstract conservation total) are unchanged and still open.

### 4.1 One recorded difference between Rust and Lean

The Rust checker marks a task mutex-keyed when the event *occurs*, before its own
admission check, so a child whose `Lock` was itself refused still counts. Lean's
`GuardLedger.observe` is threaded alongside steps. On an accepted trace no event
is refused, so the two coincide; on a refused trace the Rust rule is strictly
more refusing, never less. This is stated in both the module doc and the Lean
section header. The correspondence between `stepGuardLocalJoin` and the Rust arm
is the standing transcription premise, exactly as for `step` itself — it is not
mechanically extracted.

---

## 5. CT impact

**No mapped cascade changes. The CT manifest was not edited.**

Both CT28 and CT32 map the *refused* direction, and both fixtures use the plain
`thread scope c { … }` form, which lowers to `Join` and is byte-identical:

| Case | Fixture shape | Pinned cascade | After |
|---|---|---|---|
| **CT28** | `with lock m as g { thread scope c { read buf with g; } }` | `["Q612","Q600"]` — `WrongGuardAffinity` then `JoinWhileGuardHeld` | unchanged |
| **CT32** | `with lock m as g { read buf with g; thread scope c { } }` | `["Q600"]` — `JoinWhileGuardHeld`, one entry | unchanged |

Verified by execution, not by argument: `cargo test -p qcore-compiler --test
ct_corpus` passes (5 tests), and the frontend tests reproduce both cascades on
their own artifacts (`the_plain_child_form_inside_a_section_still_refuses` gives
CT32's exactly; `a_guard_local_child_cannot_act_through_the_creators_guard`
gives CT28's shape on the new form). The accepted direction is a **new control**
living in the frontend and core test suites, not a CT mapping: CT coverage stays
at **24/50 mapped**, unchanged, and no `cfg`/native/proof stage claim is added
to any case.

CT32's `expected.json` note reads "No claim is made about whether some future
profile could admit such a join safely." That note remains accurate: this card
admits a *different, explicitly marked* form, and CT32's own artifact still
refuses.

---

## 6. Native evidence

**None, and this is a route property rather than an omission.** The mutex
backend emits one thread; `is_single_task_trace` classifies any trace carrying
task structure as outside it, and `GuardLocalJoin` is added to that list. Every
accepted guard-local module is multi-task by construction, so it publishes
checked with `c_source: None` — the same rule T03-F recorded for CT28/CT32's
accepted controls. `emit_mutex_c11` refuses a `GuardLocalJoin` trace outright
(`UnsupportedEvent`) rather than emitting C it did not model. No ASan/UBSan/LSan
run applies, because no fixture on this route executes natively; the T03
precedent's fixed sanitizer runs cover the single-task guard fixtures, which are
unchanged.

---

## 7. Gate results

Run at `6bb633c` plus the report commit, on this worktree.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1815 passed, 0 failed, 3 ignored** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **Passed 44 Lean audits** (count unchanged; 24 registrations added to an existing audit file). `AuditAll.lean`: 19,404 declarations checked, only standard logical axioms |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **67 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests -p "test_*.py" -t scripts/tests` | **108 tests, OK** |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed — CT28/CT32 cascades unchanged |

28 of the 1815 workspace tests are new in this increment (18 core, 10 frontend).
No known-flaky gate (P02 `RLIMIT_NPROC`, P03 confined-write, vp04) failed in this
run, so no isolated rerun was needed.

### 7.1 Cycle 19 artifacts

**Not re-executed, and this is a claim by construction rather than a
measurement.** No file this increment touches is on the resource-analysis path:
`qcore-core::mutex_invariant` and `mutex_c_emit` have no production caller in the
audit, and the frontend change is confined to `source_mutex.rs` plus one entry in
the reserved-word list, which the C gateway does not consult. `git diff --stat`
against `05c51a5` touches no file under `crates/qcore-c-gateway`,
`crates/qcore-audit`, `crates/qcore-contract`, `crates/qcore-verification` or
`compiler/src`. The four pinned 185-unit artifacts, the contract corpus (60
modules) and the `third_party/libevent` submodule pin are unchanged.

---

## 8. What this card does not establish

* **No deadlock freedom.** The admission removes the possibility of a wait
  inside the joined subtree; it proves nothing about waits anywhere else. Lock
  ranks, wait-for graphs and lock-order analysis remain absent from the profile
  by design, and a lock-rank declaration is still an `UnsupportedSynchronization`
  refusal.
* **No widening to a child that locks another generation.** §1.2 records why:
  that case needs exactly the analysis this profile declines to do. It is the
  obvious next increment and it should not be taken without a wait-for
  representation.
* **The `EndBorrow` asymmetry is still open.** `EndBorrow` remains keyed to task
  liveness rather than to the guard holder, so a child's release of its parent's
  borrow is still *accepted by the model* and refused only by the lowering's
  grammar and (now) by G7. Repairing `EndBorrow` itself is a separate semantic
  change to a pinned checker and was deliberately not taken here; the admission
  does not depend on it and does not make it reachable. Recorded as
  **T03-GUARD-JOIN-B1**.
* **A guard-local child can do nothing this profile models.** With no share and
  no custody available to a child, the admitted child's body is necessarily
  empty of modelled effect (or a tree of equally silent grandchildren). Giving a
  guard-local child real work needs a share- or loan-transfer event that this
  profile does not have. Recorded as **T03-GUARD-JOIN-B2**.
* **G5 is conservative.** The closed-invariant requirement is not forced by the
  model; it is forced by `DeclaredFootprintShape` being a premise. If that
  premise is ever discharged, G5 could be revisited. Recorded as
  **T03-GUARD-JOIN-B3**.
* **No promotion, no new CT mapping, no new diagnostic code, no schema change.**
  Raw Cycle 19 units, the contract corpus (60 modules), CT coverage (24/50) and
  full language conformance are all exactly as they were.
* **Source-to-trace refinement, the real C layout of declared intervals and the
  invariant predicate itself remain premises**, as everywhere else in this
  profile.
* **One new reserved word, `under`**, with T03-F's recorded hazard applying: a
  source binding named `under` would now be a profile keyword. Checked against
  every fixture in the workspace by the full test run; no occurrence exists.
