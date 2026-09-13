# MUTEX-SHARE-CROSS — a payload crosses because a verified mutex protects it (wave 32)

Card brief: wave 32's language head — the composition CT46 needs, where a value
that is **Send but not Sync** crosses a task boundary **only** through a
verified mutex protocol. Base: `origin/main` at `1c7d8e45` (the wave-31 close
plus `STABILITY-REEVAL`), worktree branch `worktree-agent-a18bfaba15868e7a9`.

**What landed, in one sentence.** An opt-in clause on the checked-mutex route's
existing child-task form — `thread scope NAME share PLACE under MUTEX { … }` —
declares that a `protected N` payload crosses into a child task, admitted by
the transferability judgment's **fifth** row and gated on the named generation
existing and protecting that very payload; the child then reaches it only
inside a critical section of that generation, which is the **unedited**
`CheckedMutexInvariantV1` checker's own rule, audited in Lean as
`aSharedPayloadIsReachableOnlyThroughTheProtocol` — at every point of every
interleaving.

Zero new diagnostic codes (the `Q-NAMESPACE-DECISION` icebox is untouched;
widenings stated in spec §L21). **+0 reserved words, +0 tokens, +0
name-resolution exclusions.** Lean audits 125 → **126**.

**This card runs no arbiter and touches no gateway, contract, audit or provider
file**, per the wave instruction; it edits nothing under
`compiler/tests/fixtures/ct/`, `docs/reevaluation/ct_manifest.json` or the
acceptance matrix.

---

## 1. Scope, stated before the evidence

**Landed (the checked half, whole).**

* The construct, the judgment's fifth row, and both protocol gates.
* All three refusals the brief names — absent, mismatched, unverified — each
  driven by a named test (§6 says which, and which arms are *stated* rather
  than exercised).
* The declaration made load-bearing, opt-in, in the refusing direction only.
* The soundness theorem, over the interleaving, against the unedited model.

**Bounded, deliberately (each a named residual, §8).**

* **The crossing is checked and not executed** (`MUTEX-SHARE-CROSS-EMISSION`).
  A multi-task mutex trace has received **no C at all** since long before this
  card — `mutex_c_emit::is_single_task_trace` excludes `Spawn`/`Join`/
  `GuardLocalJoin`/`TransferCustody`/`RegisterWaiter` — and this crossing is
  multi-task by construction. That boundary is **pre-existing and unmoved**: no
  native run, no byte comparison, no sanitizer claim, and **no data-race claim
  about C**. The brief anticipated exactly this ("if the native emission half is
  not reachable, land the checked half and name the emission residual").
* **One wrapper, one payload, one mutex, spawn..join, no nesting** — a
  declaring child inside a declaring child is refused **by name**, not admitted
  on an untested reading of what it would inherit.
* **The task route's Buffer still cannot cross under a mutex**
  (`MUTEX-SHARE-CROSS-TASK-BUFFER`). This is why `MUTEX_SHARING_GAP` is left
  open and still reads `requestable: false` — see §4.

## 2. The construct

```qcore
import std.concurrency;
fn main() -> unit {
  thread scope s {
    let buf = protected 4;                  // the payload: Send, not Sync
    let m = mutex guards buf;               // the known shared wrapper
    thread scope c share buf under m {      // the crossing, declared
      with lock m as gc { open gc; write buf with gc; close gc; }
    }
    with lock m as g { read buf with g; }   // the creator, same discipline
  }
}
```

Both tasks reach the payload; neither reaches it unguarded. The clause is told
apart from the two pre-existing header forms by **one token at a fixed
position** (index 3 of the header): nothing for the plain child, `under` for
`T03-GUARD-JOIN`'s guard-local join, `share` for this card's crossing. A module
that does not spell `share` there cannot reach any of this card's code.

## 3. Why this shape, and not a `spawn` capture

CT46's composition could have been built on the *task* route's
`spawn s worker(…)` captures instead. It was not, and the reason is the card's
central discipline.

The checked-mutex model is **already multi-task**: `MutexInvariantEvent` has
`Spawn`, `Join` and `GuardLocalJoin`, every event carries a `TaskId`, and
`Access` carries `guard: Option<GuardId>` where `None` is legal *only* for a
footprint no live mutex protects. So the safety property CT46 asks about was
already enforced, by rules that are already proved. What was missing was a
crossing that is **declared, judged and gated on the protocol** — and the task
route has no mutex at all (`source_mutex.rs` refuses its `spawn`/`join`
expressions by name, and that refusal is **unmoved**, pinned by
`the_task_routes_spawn_still_refuses_a_mutex_generation`). Building the
crossing on the task route would have meant constructing a second mutex model
beside the checked one — the opposite of composing it.

## 4. The judgment: a fifth row, and a generalisation in the refusing direction

`qcore_core::transfer` grows by two kinds, one mode, one delivered transfer:

| Addition | What it is |
|---|---|
| `TransferKind::ProtectedResource` | CT46's payload. **Send** — custody moves in the model over a *derivable* synchronization edge (`TransferCustody`), never a caller-supplied flag — and **not freely Sync**, shareable only under `CheckedMutex` |
| `TransferKind::MutexShare` | what the crossing delivers: the right to *acquire* the generation and reach the payload inside the section. The product of a crossing, never a source of one, like `ReadLoan` and `WriteLease` |
| `TransferMode::ShareUnderMutex` | `share PLACE under MUTEX` |
| `AdmittedTransfer::ShareUnderMutex` | nothing duplicated, no custody crossing, **no authority parked with the child** |

`ProtectedResource` is shareable under `CheckedMutex` in **both** directions —
`shareable_under` *and* `mutably_shareable_under`. That is the first kind for
which the two agree, and it is the point: a mutex **orders** mutation, where
counted read shares **remove** it and disjoint leases **partition** it, so a
guard holder may read *and* write the very same bytes another task will later
reach.

**The generalisation.** Before this card the read-lend arm accepted whatever
protocol the kind happened to name, which was harmless only while one kind
shared at all. With two sharing disciplines in each direction that would have
let a `read … share N` capture through under a mutex's name. Each sharing mode
now requests **one** named protocol (`ShareProtocol::requested_by`) and is
admitted exactly when the kind's own discipline **is** that protocol. This is
what makes the brief's "refuses when it crosses any other way" *quantified*
rather than exemplified:

* `admits(OwnedBuffer, ShareUnderMutex)` → `ProtocolUnavailable(CountedReadShares)`
* `admits(ProtectedResource, Lend)` → `ProtocolUnavailable(CheckedMutex)`
* `admits(ProtectedResource, LendWrite)` → `ProtocolUnavailable(CheckedMutex)`

Each refusal names **the protocol the kind actually carries**, so a reader is
told which discipline governs the payload rather than only that this one does
not. Proved over the whole matrix as
`eachProtocolIsRequestedByExactlyOneMode`.

### 4.1 The gap constant: half closed, half left open, both stated

`MUTEX_SHARING_GAP` is **not** flipped to `true`. It names the *task* route's
`OwnedBuffer`, and that kind still has no mutex to cross under — so flipping it
would have been a false statement dressed as progress. The constant stays, with
`requestable: false`, as the residual `MUTEX-SHARE-CROSS-TASK-BUFFER`; the half
this card closes is the **new** `MUTEX_SHARE_CROSSING`
(`ProtectedResource`, `CheckedMutex`, `requestable: true`). Both are pinned,
quantified against the judgment in both directions, and `theTaskBuffersMutexGapStaysOpen`
audits the open half in Lean so it cannot be quietly assumed closed.

## 5. Where each decision lives — the composition, not a re-implementation

The brief's instruction was to compose the unedited mutex checker rather than
re-implement its rules beside the judgment. The three refusals divide as
follows, and the division is the evidence:

| Refusal | Decided by | Code |
|---|---|---|
| the payload is a kind that does not cross this way (a generation, a guard, a fact) | **the judgment**, `qcore_core::transfer`, consulted and not restated | `Q600` |
| **absent** — the `under` name is not a mutex generation of this module | the crossing's own gate | `Q600` |
| **mismatched** — that generation guards some *other* resource | the crossing's own gate | `Q600` |
| **unverified** — the child reaches the payload outside a section | the **unedited core**, `UnguardedProtectedAccess` | **`Q611`** |
| a declaring child reaches an undeclared resource | the declaration's own rule (§5.1) | `Q600` |
| a clause naming an unbound name | **name resolution**, before any of the above | `Q013` |
| a malformed clause | shape | `Q600` |

The two middle gates are the crossing's because they are *not* classification
questions — the judgment cannot see a generation's declared footprint — and
*not* the core's, because the core is handed a trace in which the crossing has
already been decided. The third is emphatically not this card's: `Q611` is the
core's own code, carrying the core's own message, and
`a_child_that_reaches_the_shared_payload_outside_the_protocol_is_refused_by_the_core`
asserts the message does **not** mention the transferability judgment.

### 5.1 The declaration is load-bearing, and opt-in

Without a further rule the clause would decorate a reach the child already had:
a plain child could already open its own section over the creator's generation.
So a declaring child reaches **the payload it named and no other declared
resource**, checked at every resource-resolution site (guarded access,
unguarded access, guard-derived borrow, protection growth).

This is an **opt-in strengthening** on the lock-rank precedent (standing
constraint 15): a frame with no declared share answers `Ok` for everything, so
every module written before this card lowers **byte-identically**, and the rule
never admits anything — a reach it allows still faces the core's guard rules
unchanged. The control that shows it is the declaration's rule and not a ban on
second resources is one word apart: the child that declares `other` reaches
`other`.

## 6. Proved, observed, and stated — the `STABILITY-REEVAL` discipline

Stated explicitly per the brief. **Every refusal this card adds is driven by a
named test**; there are no unexercised arms in the card's own rules.

| Rule | Status | Driven by |
|---|---|---|
| the admission (CT46's shape checks end to end) | **exercised** | `a_protected_payload_crosses_into_a_child_under_the_generation_that_protects_it` |
| **absent** protocol | **exercised** | `a_crossing_whose_protocol_names_no_generation_is_refused` |
| **mismatched** protocol | **exercised** | `a_crossing_whose_protocol_guards_a_different_payload_is_refused` |
| **unverified** (the core's `Q611`) | **exercised**, with an accepting control | `a_child_that_reaches_the_shared_payload_outside_the_protocol_is_refused_by_the_core` |
| the judgment's refusal at the payload position | **exercised** | `sharing_the_generation_itself_is_refused_by_the_judgment`, `the_judgment_decides_the_payload_before_the_protocol_gate_runs` |
| declaration load-bearing (access **and** borrow) | **exercised**, with a one-word control | `a_declaring_child_may_not_reach_a_resource_it_did_not_declare`, `..._may_not_borrow_...` |
| no nesting | **exercised** | `a_declared_crossing_does_not_nest` |
| malformed clause shapes (3) | **exercised** | `a_malformed_crossing_clause_names_the_form` |
| pre-existing forms unmoved | **exercised** | `the_pre_existing_child_forms_are_untouched`, `the_task_routes_spawn_still_refuses_a_mutex_generation` |
| `Q013` boundary for an unbound clause name | **exercised** | `a_clause_naming_an_unbound_name_is_refused_by_name_resolution` |

**Proved (Lean, §7):** the soundness statement over the interleaving, mutual
exclusion of the generation, holder-only release, and both judgment matrices.

**Stated boundaries, not verified rules** — labelled as such:

* `admits(ProtectedResource, Move)` is admitted by the class and **has no
  source spelling on this route**, so the cell is pinned in the matrix and
  reachable from no program. It is recorded in the matrix pin's own comment.
  The classification is faithful (the model really does move custody via
  `TransferCustody`), but nothing exercises it from source.
* The **emission** half entirely (`MUTEX-SHARE-CROSS-EMISSION`): checked, not
  executed. No native evidence of any kind is claimed.
* `theCreatorAndTheChildAreHeldToOneRule` is a *definitional* statement
  (`Iff.rfl` twice) — it records that the rule does not branch on which task
  reaches, and proves nothing beyond that reading.

## 7. Lean (invariant 6)

`proof/QCore/MutexShareCross.lean`, registered in `proof/QCore.lean` (so it is
in the whole-project axiom closure) and audited by
`proof/AuditMutexShareCross.lean`. Placeholder-free, no project axiom, every
closure within `{propext, Classical.choice, Quot.sound}`. Lean audits
125 → **126**.

Everything is proved against `QCore.MutexInvariant`'s **existing** `step`/`run`
model, **unedited by this card**. That is the discipline rather than an
economy: a soundness proof that had to add a rule to the mutex model would be
evidence that the composition had not happened.

| Theorem | Statement |
|---|---|
| **`aSharedPayloadIsReachableOnlyThroughTheProtocol`** | **the card's theorem.** Any admitted access to a footprint a live generation protects — by *any* task, at *any* state — names a guard; that guard exists; the accessing task holds it; and its generation is that footprint's protector. The three conjuncts are the three ways the property could fail: reaching without naming a guard, naming a guard someone else holds, and naming a guard over a different mutex |
| `everyReachAlongAnAdmittedTraceHoldsTheProtocol` | the same, lifted along an **arbitrary admitted schedule** via `run_append`. The prefix may interleave creator and child in any admitted order, spawn, join, and open and close any number of sections — so *"at every point of every interleaving"* is the statement's own quantifier, not a reading of it |
| `aSecondAcquisitionOfALiveGenerationIsRefused` | the generation admits one holder at a time (`canLock` demands it be unheld). With the theorem above this is CT46's expected answer in full: reachable only through the generation, and the generation orders the reachers |
| `onlyTheHolderReleasesTheGeneration` | a reacher cannot be displaced by another task shortening its window |
| `aSpawnedChildReachesThePayloadOnlyThroughTheProtocol` | the child is held to the identical rule. Its spawn hypothesis is named and **deliberately unused** — that is the content: a proof that *needed* the spawn would mean the crossing had bought the child some authority, which is exactly what a mutex share does not do |
| `theMutexProtocolIsReachableForExactlyOneKind` | replaces `noModeRequestsTheMutexProtocol`. One kind, one mode, and still **no kind is *lent*** under a mutex |
| `eachProtocolIsRequestedByExactlyOneMode` | over the whole matrix — the quantified form of "refuses any other way" |
| `theTaskBuffersMutexGapStaysOpen` | replaces `theGapKindIsSendableAndOtherwiseShareable`; the half that stays open, stated so it cannot be assumed closed |
| `theMutexPayloadCrossesOnlyUnderTheMutexMode` | CT46's admitting cell, beside CT47's refusing one |
| `aCrossProtocolRequestNamesTheKindsOwnProtocol` | a cross-protocol request refuses with the protocol the kind *does* carry, named |
| `everyAdmittedTransferIsBacked` | the fifth row carries `MutexShareObligation`, discharged by the theorem above — the same record-which-rule-answers discipline as the move, read-lend and write-lease arms |

`theBoundaryIsDecidedExactlyOnce` (CT47's partition) and
`aWriteCaptureIsRefusedByName` are **re-decided over the grown 16 x 6 x 16
matrix rather than weakened**, which is what shows that adding an admitting row
moved no cell this card did not touch. Two audit entries in the existing
`AuditTransferability.lean` are **replaced, not deleted**, by their successors.

**Not proved, carried as the Rust side's boundary:** that the source lowering
produces the trace it intends (tested, §6), any C-memory-model or native
schedule statement, and liveness — this says a reach is *impossible* without
the protocol, not that any reach happens. **Deadlock-freedom is not
established**; the lock-rank discipline's own note is unchanged.

## 8. Named follow-ons

| Name | What is missing | Discharge condition |
|---|---|---|
| `MUTEX-SHARE-CROSS-EMISSION` | the crossing is checked and never executed: `mutex_c_emit::is_single_task_trace` gives **no C** to any multi-task mutex trace, a pre-existing boundary this card does not move | a C backend for multi-task mutex traces (real `thrd_t` plus the existing `mtx_t` emission), then a sanitized native run byte-compared against the reference interpreter, on the `WRITE-LEND-TASKS` native-run precedent |
| `MUTEX-SHARE-CROSS-TASK-BUFFER` | the task route's `own<Buffer>` still cannot cross under a mutex; `MUTEX_SHARING_GAP` stays open and still reads `false` | a mutex for the task route — or a decision that the two routes' payloads stay distinct kinds, recorded as such |
| `MUTEX-SHARE-CROSS-MOVE-SPELLING` | `admits(ProtectedResource, Move)` is admitted by the class and reachable from no program | either a source spelling for moving a protected resource between tasks (the model has `TransferCustody` over a derivable edge), or a narrowing of the class with the model's move arm re-read |
| `MUTEX-SHARE-CROSS-MULTI` | one wrapper, one payload, one mutex, no nesting | a second declared crossing in one child, or a nested declaring child, each with its own rule for what the inner child inherits — measured, not assumed |
| `MUTEX-SHARE-CROSS-GUARD-LOCAL` | the clause and `T03-GUARD-JOIN`'s `under GUARD` are alternatives; a child cannot be both guard-local and declare a crossing | a reading of what a guard-local join means for a declared crossing, since the guard-local rule requires the joined subtree to name no mutex at all — which a declaring child necessarily does |

## 9. Keyword census (standing constraint 14)

**+0 reserved words, +0 tokens, +0 name-resolution exclusions** — the cleanest
outcome in the constraint-14 ledger. Archived worktree-private at
`measure/keyword_census_share_under.md` with the reproduction commands and the
raw hit list.

Both clause words were **already globally reserved**: `share` by the
scoped-task route's `read PLACE share N`, `under` by card `T03-GUARD-JOIN`. The
card reserves nothing and claims only a **position** — the fourth token of a
child-task header — which was a parse failure before it. Measured over **490**
`.qcore`/`.qc` files outside `third_party/`: 33 total hits, **8** in code
rather than comments, **0** occurrences of `share` at any position other than
the read capture's clause, **0** occurrences of `under` in code anywhere, and
**0** occurrences of `thread scope NAME share|under` in any fixture. The three
Rust string literals spelling the claimed position are all this card's own doc
comments and its own refusal message.

No exclusion was carved out of `referenced_names`, unlike
`SOCKET-INET-ADDRESSES` and `WRITE-LEND-TASKS`, because the card claims no
word — and both clause **names** stay genuine references, which is measured
rather than asserted (§5's `Q013` row).

## 10. CT46 readiness statement

For a future mapping lane. **This card maps nothing**: it edits neither
`docs/reevaluation/ct_manifest.json` nor `docs/reviews/r0_3/acceptance_matrix.json`,
and touches no fixture under `compiler/tests/fixtures/ct/` — a parallel lane
owns that tree. CT46's manifest reason recorded the blocker as *"the mutex
ledger and the task ledger compose today by mutual exclusion rather than over a
capture, so no source text can ask for mutex-mediated sharing across a task
boundary"* (open card `CT46-MUTEX-CAPTURE`). What a lane can now claim:

* **The blocker sentence is discharged.** Source text can now ask for
  mutex-mediated sharing across a task boundary:
  `thread scope c share buf under m { … }`. The composition is over a
  **declared crossing** judged by `qcore_core::transfer`, not over mutual
  exclusion.
* **The accept half** — *"Accept only through verified mutex protocol"* — has
  a fixture shape and an audited soundness statement: the flagship program of
  §2, checking end to end, with both tasks reaching the payload and every reach
  guarded; and `aSharedPayloadIsReachableOnlyThroughTheProtocol` for the *only*.
* **Three attribution controls, in the wave-26 style, come free**: removing the
  clause (unchanged, still checks), replacing the guarded reach with an
  unguarded one (`Q611`), and naming a generation that guards something else
  (`Q600`, mismatched). Supplying the verified protocol is exactly what flips
  the verdict, and each control differs from the admission by one clause.
* **What a lane must still decide.** (a) Whether CT46's "known shared wrapper"
  premise is satisfied by a **mutex generation** declared over the payload
  rather than by a wrapper *type* — this language has no `Arc<Mutex<T>>` and no
  wrapper type at all, so the generation is the nearest expressible shape.
  (b) Whether the case requires **execution**: it does not run, and cannot
  today (`MUTEX-SHARE-CROSS-EMISSION`), so a mapping would be checked-only and
  its stage may not read `executed`, per standing constraint 23. (c) Whether
  the `protected N` payload satisfies "non-Sync but Send": the judgment says so
  and names the model rule for each direction (§4), but the "Send" direction
  has no source spelling (`MUTEX-SHARE-CROSS-MOVE-SPELLING`) and a lane should
  say so rather than imply a move was demonstrated.
* The sanitized-native and proof axes: **proof yes** (an audited soundness
  theorem over the interleaving), **sanitized-native no**, by §1's residual.

## 11. Files

* `crates/qcore-core/src/transfer.rs` — the fifth row, the protocol-comparing
  admission, `ShareProtocol::{ALL, requested_by}`, `MUTEX_SHARE_CROSSING`.
* `crates/qcore-core/tests/unit/transfer.rs` — matrices re-pinned whole;
  the headline pin renamed and extended (§12).
* `crates/qcore-frontend/src/source_mutex.rs` — `DeclaredShare`,
  `Frame::{shared, admits_reach}`, `lower_declared_share`, the header form.
* `crates/qcore-frontend/tests/mutex_share_cross.rs` — **16 tests**, §6.
* `crates/qcore-frontend/tests/source_mutex.rs` — three lines: the documented
  grammar example gains a declared crossing, in step with the doc below.
  Keeping those two in sync is exactly what
  `the_documented_grammar_example_checks` exists for; the other 74 tests of
  that file are untouched and pass unchanged.
* `proof/QCore/MutexShareCross.lean`, `proof/AuditMutexShareCross.lean`,
  `proof/QCore/Transferability.lean`, `proof/AuditTransferability.lean`,
  `proof/QCore.lean` — §7.
* `docs/QCORE_SPEC_r0_3.md` §L21 `Q6xx` row — the condition widening, stated.
* `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c — the clause in both EBNF
  productions and in the worked example.
* `docs/development/increments/wave-32.md` (new; index pointer and wave table
  updated in `docs/development/increments/README.md`).

Nothing else is touched: **no** gateway, contract, audit or provider file, no
fixture under `compiler/tests/fixtures/ct/`, no frozen archive, and neither
`ct_manifest.json` nor the acceptance matrix. The Cycle 19 corpus rows and the
leak pin are untouched by construction, and this wave runs no arbiter.

## 12. Deliberate pin moves, disclosed

Constraint 5 is satisfied by disclosure, not bypassed. **Nothing is weakened
and no expectation is deleted.**

* `crates/qcore-core/tests/unit/transfer.rs`: both matrices re-pinned whole at
  16 x 6 (six admitting cells of 96) and 16 x 6 x 16 (six of 1,536); the
  inventory counts 14 → 16 and 5 → 6; the sendable/shareable filters grow by
  the new kind. `write` is still refused for every kind, verbatim.
* `the_mutex_protocol_is_named_and_unreachable` →
  `the_mutex_protocol_is_requestable_for_exactly_one_kind`. This is **the
  update that test's own doc comment asked for**: it said "adding a
  mutex-mediated capture mode later must update this test". The new test pins
  strictly more — both gap constants, both quantified directions, the surviving
  no-kind-is-lent-under-a-mutex property, and the one-mode-per-protocol rule.
* `proof/AuditTransferability.lean`: `noModeRequestsTheMutexProtocol` and
  `theGapKindIsSendableAndOtherwiseShareable` **replaced by their successors**,
  not dropped; two renamed inventory entries followed their theorems.
* No `Q` code moved, no message of any pre-existing route changed, no fixture
  edited, no frozen archive touched.

## 13. Integrator reconciliation list

This card may not edit `AGENTS.md`, `README.md` or the acceptance matrix.

1. **Proposed new standing constraint (or an append to 15)** — the opt-in
   clause and its division of labour:

   > **(card `MUTEX-SHARE-CROSS`.)** A `protected N` payload crosses a task
   > boundary only through `thread scope NAME share PLACE under MUTEX { … }` on
   > `source.mutex.guard.v1`, and the decision is in **three** halves that must
   > not be merged: classification is `qcore_core::transfer`'s (the fifth row;
   > each sharing mode requests **one** protocol and is admitted exactly when
   > the kind's discipline is that protocol — do not restore the arm that took
   > whatever protocol the kind named); the **absent** and **mismatched**
   > protocol gates are the crossing's, because the judgment cannot see a
   > generation's declared footprint; and **unverified** is the *unedited*
   > `CheckedMutexInvariantV1` checker's own `Q611`. Do not re-implement a
   > guard rule beside the judgment. The clause is **opt-in**: a child without
   > it lowers byte-identically, and the load-bearing reach rule
   > (`Frame::admits_reach`) applies only to a declaring child and only in the
   > refusing direction. `MUTEX_SHARING_GAP` stays `false` — the *task* route's
   > Buffer still has no mutex — and the crossing is **checked, never
   > executed** (`MUTEX-SHARE-CROSS-EMISSION`); do not claim native or
   > data-race evidence for it. `mutex-share-cross`.

2. **`AGENTS.md`**: the `Q600` and `Q610`–`Q627` namespace rows should record
   the widenings (authority: spec §L21, updated by this card); the
   constraint-14 census ledger gains this card's **+0 / +0 / +0** row; the
   headline Lean-audit number moves 125 → **126**.

3. **The derived `lean_audits` count** moves by one — the usual one-line matrix
   reconciliation (constraint 25), and the Python tripwire below.

4. **Public API changes** (workspace-internal; `qcore-schemas` untouched, so
   constraint 18's provider rebuild is **not** owed):
   * `qcore_core::transfer`: `TransferKind::{ProtectedResource, MutexShare}`,
     `TransferMode::ShareUnderMutex`, `AdmittedTransfer::ShareUnderMutex`,
     `ShareProtocol::{ALL, requested_by}`, `MUTEX_SHARE_CROSSING`. A downstream
     exhaustive match on any of those enums needs arms.
   * No public change in `qcore-frontend`: `DeclaredShare` and
     `lower_declared_share` are private to `source_mutex`.

5. **CT46's mapping and its manifest reason** may be updated per §10 by a
   mapping lane, never by this card.

## 14. Gate

Run in worktree `worktree-agent-a18bfaba15868e7a9` on `origin/main` at
`1c7d8e45` plus this card, with `$HOME/.elan/bin` on `PATH` for the Lean row
and the submodule initialised before the workspace run.

<!-- GATE TABLE INSERTED BELOW -->
