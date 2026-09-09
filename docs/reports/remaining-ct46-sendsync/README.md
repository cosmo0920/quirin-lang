# CT46-SENDSYNC — task-boundary transferability, derived from the kind

Card `CT46-SENDSYNC`, wave 12. Branch `claude/ct46-sendsync`, based on main
`67961f1`.

CT46's scenario is *"Known shared wrapper with non-Sync but Send payload"* and
its expectation is *"Accept only through verified mutex protocol"*. Its
recorded blocker across waves 5, 9 and 11 has been the same sentence: **"needs
Send/Sync derivation, which is outside L09 and outside this profile
entirely."**

This card supplies that derivation and then measures what CT46 still needs.
The short version:

* The derivation exists, is total over the real type grammar, is consulted at
  the real boundary under a fail-closed agreement check, and is proved.
* **CT46 stays unmapped**, and its blocker is **retired and replaced**. It is
  no longer "needs Send/Sync derivation" — that is now implemented. It is
  "there is no source form in which a mutex protects a payload that crosses a
  task boundary", which is a different, sharper and measured blocker.
* Nothing is promoted. No verdict moves, no diagnostic code is added or
  changed, no gate is weakened, no CT file is edited.

---

## 1. What "Send/Sync" means here

QCore is not Rust and this card deliberately does not grow a `Send`/`Sync`
trait, a marker, or anything a user can declare. CT46's vocabulary is recast
as a **closed judgment over the implemented kind vocabulary**, with the two
Rust directions kept separate:

| CT46's word | The question here |
|---|---|
| `Send` | May a value of this kind cross a task-creation boundary *carrying its own custody*? |
| `Sync` | May the parent *keep* it while a child also reaches it, and under which discipline? |

The judgment is `qcore_core::transfer` (new,
`crates/qcore-core/src/transfer.rs`). It answers both questions per kind and
decides the operational boundary question `(place kind, capture mode,
parameter kind)`.

### 1.1 The model argument: `transferable(T)`

`TransferClass` has exactly three inhabitants, and the middle one is CT46's
shape:

* `Duplicated` — crosses by duplication; both sides end up with independent
  values and no custody is split, so the sharing question does not arise.
* `MovedOnly { shareable_under: Option<ShareProtocol> }` — crosses by move,
  consuming the parent binding; sharing is admitted **only** under the named
  protocol and refused outright when the field is `None`. *This is
  "Send but not Sync".*
* `Never(NotTransferable)` — does not cross in any direction, with the reason
  carried.

### 1.2 The domain, enumerated from the real grammar

The card brief asked for the type grammar, "not hypotheticals". Measured: there
is **no structural type tree** in this compiler. Types are retained as token
slices plus a coarse one-token classifier. The judgment is therefore stated
over the union of the closed enums that actually classify a name at a creation
site, plus the mutex route's three binders, which have no value type at all:

| `TransferKind` | Real source of the constructor | Class | The rule that answers |
|---|---|---|---|
| `I32` | `source_tasks::Scalar::I32` | `Duplicated` | The ledger records **no resource** for a scalar (`scoped_tasks` header). A copied `i32` has no identity that could be held twice, so both directions are vacuous. |
| `Unit` | `Scalar::Unit` | `Never(ResultPositionOnly)` | A result type. The parameter grammar never produces it, so it is refused by position, not by a property of the value. |
| `OwnedBuffer` | `Scalar::OwnedBuffer` | `MovedOnly(Some(CountedReadShares))` | **CT46's kind.** Move: `use_resource` demands exclusive authority and parks the resource under `Custodian::Reservation`, so failed creation rolls the parent binding back and success consumes it permanently. Share: only under counted read shares — `lend`'s budget (positive, ≤ `FULL_READ_AUTHORITY` outstanding, ≤ `MAX_READERS` holders) together with the write gate, since `WriteBuffer` needs exclusive authority. |
| `ReadLoan` | `Scalar::ReadLoan` | `Never(NotAValue)` | A loan is the *product* of lending, never a source. `AbstractValue::Loan::resource()` returns `None`, which is what structurally prevents drop/return/re-lend; the checker separately refuses it as a result and as an entry parameter. |
| `ThreadScope` | `ValueType::Scope` | `Never(OwedToCreatingFrame)` | A lexical region, not a value. |
| `TaskHandle` | `ValueType::Handle` | `Never(OwedToCreatingFrame)` | Joinable by its creator only; sending it strands the join duty. |
| `TaskResult` | `ValueType::Result` | `Never(OwedToCreatingFrame)` | Must be matched or propagated in the frame that owes it. |
| `ErrorValue` | `ValueType::Error` | `Never(OwedToCreatingFrame)` | Its payload is taken exactly once, in the frame that bound it. |
| `KnownErrorTag` | `ValueType::KnownError` | `Never(OwedToCreatingFrame)` | Same frame duty. |
| `MutexGeneration` | `source_mutex::MutexSourceGeneration` | `Never(NotAValue)` | Fixes its protected footprint at creation and parks the creator's full share over it — there is no second authority to hand anywhere. |
| `Guard` | `source_mutex` guard binder | `Never(NotAValue)` | Minted fresh by each `Lock`; its binder is body-local, so it cannot even be named at a creation site outside its own section. |
| `ProtectedFact` | `source_mutex` fact binder | `Never(NotAValue)` | Guard-scoped; each assertion mints a fresh identity, and an outlived fact is already `Q616`. |
| `Unmodeled` | every other `SourceTypeKind` tag | `Never(UnmodeledType)` | Not elaborated by this profile at all. |

The declared-type tag set (`SourceTypeKind`, fifteen tags) has its own total
map, `of_declared_type`. It is deliberately conservative: `Atomic`, `Owned` and
`Borrow` map to `Unmodeled`, because only the elaborated spelling
(`i32`, `own<Buffer>`, `borrow<Buffer>`) can decide those, and the elaborated
scalar is classified directly. A test pins that **no declared tag is sendable
on its own**.

### 1.3 What the child receives

Transferability alone does not decide a capture: the boundary must also agree
on *what arrives*. `received_kind` states the one deliberate asymmetry —
lending an `own<Buffer>` delivers a `borrow<Buffer>`, not a Buffer — so
`admits_capture(place, mode, parameter)` is the full three-way decision. That
asymmetry was hard-coded at the call site before; it now lives in the judgment.

---

## 2. Composition: checked-redundant, fail-closed

### 2.1 Measured finding — there are no per-profile capture enumerations

The brief assumed per-profile enumerations to replace. Measured, that is not
the shape of the code:

* The five `Scoped*V1` names are **post-hoc labels**, not gates. No
  `Scoped*V1` variant appears in `qcore-frontend` at all; the labels are
  computed *after* checking by inspecting the checked IR
  (`qcore-verification/src/driver.rs:1388-1481`). All five profiles are the
  same lowering and the same checker.
* Capture legality is gated in **exactly one place**:
  `source_tasks.rs::spawn_inputs`, a three-arm `match (&capture.mode,
  parameter_type)` with a catch-all refusal — plus the core's own independent
  re-check in `task_execution.rs`, which the design deliberately keeps
  separate.

So there was one enumeration, not several. This is recorded rather than worked
around.

### 2.2 The composition landed

The judgment is consulted **beside** the existing enumeration at
`spawn_inputs`, and the two must agree:

* Both admit, and on the same transfer → proceed, with the existing operand.
* Both refuse → refuse, with the **existing message**.
* They disagree in either direction → refuse, with a distinct message.

So the judgment **cannot widen admission**. If it admitted something the
enumeration refuses, the result is a refusal; if it refused something the
enumeration admits, the result is also a refusal. It is a strictly fail-closed
second opinion.

A second guard *states*, rather than assumes, that no non-scalar binding is
sendable: if the judgment ever called a handle, scope, `Result` or error value
sendable, the capture refuses instead of letting the type check be the only
thing between it and a boundary.

### 2.3 Why the enumeration was kept rather than deleted

The brief permits replacement only where behavior is provably identical. It is
identical (§3), so replacement was permissible — and was deliberately **not**
taken:

* The refusal message
  `"only copy i32, move own<Buffer> and read own<Buffer> share FULL captures
  are executable"` is pinned verbatim in existing tests
  (`read_loans.rs:299-328` among others). Keeping the enumeration keeps that
  text emitted from its own site.
* Two independent decisions that must agree are strictly more fail-closed than
  one. Deleting the enumeration would convert a cross-check into a single
  point of failure for no measured gain.

This is recorded as a deliberate choice, not an omission.

---

## 3. Identical-behavior evidence

Two independent measurements, both required by the brief.

### 3.1 Whole existing test surface

| | Before | After |
|---|---|---|
| Workspace tests | 2119 passed, 0 failed | **2143 passed, 0 failed** (2119 + 16 core + 8 frontend) |

Every one of the 2119 pre-existing tests passes unchanged. Many of them pin
exact diagnostic vectors on this exact route.

### 3.2 Pre/post measurement of the new adversarial cases

The eight new source-level cases in
`crates/qcore-frontend/tests/transferability.rs` were run **twice**: once
against the card's lowering, and once against `67961f1`'s `source_tasks.rs`
restored byte-for-byte in place. Both runs pass, with the same exact codes and
messages:

```
running 8 tests   (baseline lowering)          running 8 tests   (card lowering)
test result: ok. 8 passed; 0 failed            test result: ok. 8 passed; 0 failed
```

Because every assertion is an exact `(code, message)` equality, that is a
byte-identical pre/post result on the whole new surface, not a "still refuses"
result.

### 3.3 The structural reason it is identical

`the_derived_judgment_reproduces_the_enumeration_exactly` quantifies over every
reachable `(mode, place kind, parameter kind)` triple — all sixteen — and shows
the derived judgment reproduces the enumeration cell for cell, with three of
sixteen admitting. The disagreement branch is therefore unreachable from
source, which is *why* the diagnostics could not move. The enumeration is
restated independently in the test, so a change to the production copy
disagrees rather than being followed.

**No divergence was found, so nothing was adopted.**

---

## 4. Adversarial surface

All measured, all asserted with exact codes and messages.

| Case | Measured |
|---|---|
| `copy` an `own<Buffer>` (duplicating affine custody) | `Q500` capture message |
| `read ... share FULL` into an `own<Buffer>` parameter (lend delivers a loan) | `Q500` capture message |
| `write` capture of a Buffer (keyword parses, no lowering) | `Q500` capture message |
| `move` a scalar (no custody to move) | `Q500` capture message |
| `read ... share FULL` a scalar (nothing to split) | `Q500` capture message |
| `write` capture of a scalar | `Q500` capture message |
| `move` a Buffer into a `borrow<Buffer>` parameter | `Q500` capture message, with the `read` variant checking as control |
| capture a `thread scope` name | `Q020` `capture type does not match worker parameter` |
| capture a task handle | `Q020` same |
| capture a bound error value | `Q020` same |
| bind a `Result` to capture it | `Q500` `buffer allocation requires ? or explicit Result match` — the `TaskResult` arm is **unreachable from source**, classified rather than omitted |
| compound `worker(copy count, move buffer)` | control checks; each component's mutation refuses that component; reversing the pair refuses at the type check |
| `with lock m as g` on the task route | `Q013` `unknown name \`m\`` |
| `let r = footprint 0 8;` on the task route | `Q500` `task scalar expressions require an i32 literal or immutable scalar binding` |
| mutex + `spawn` in one module (CT46's own shape) | `Q600`, the mutex route's refusal, verbatim |

Three of my own expectations were wrong and were corrected **to the
measurement**, not the other way round: lending a scalar refuses at `Q500` not
`Q020`; a `Result` binding cannot be created at all on this route; and a guard
on the task route refuses at name resolution.

### 4.1 The guard result is stronger than "refused at the boundary"

A guard binder comes only from a `with lock` body, and the mutex it locks comes
only from `let m = mutex guards r;` — which selects the **mutex route for the
whole module** (`has_source_mutex`, a whole-module token scan). So on the task
route the mutex name is simply unbound. **A guard can never be named at a task
creation site at all.** The judgment's `Guard`/`ProtectedFact` arms are
therefore classified-but-unreachable, exactly like `TaskResult`, and that is
recorded rather than presented as an exercised refusal.

---

## 5. CT46: the honest boundary

### 5.1 What CT46 asks for, decomposed

1. A payload that is sendable but not freely shareable. **Supplied.**
   `own<Buffer>` is exactly that kind, and
   `exactlyOneKindIsSendableButNotFreelyShareable` proves it is the only one.
2. A "known shared wrapper" holding it. **Partly present.** QCore has a
   checked mutex (`CheckedMutexInvariantV1`, `source.mutex.guard.v1`) that
   protects declared byte-interval footprints.
3. Acceptance of that payload across a task boundary **only through the
   verified mutex protocol**. **Not supplied, and not by this card.**

### 5.2 The measured reason (3) has no source form

Two independent facts, both measured on this tree:

* **The mutex route's child task takes no captures at all.**
  `source_mutex.rs::lower_child_task` (lines 1049-1149) emits only
  `Spawn`/`Join`; the child name is explicitly discarded
  (`let _ = child_name;`). The module header states it plainly: *"The child
  receives no share and no custody: every access it makes is either
  mutex-mediated or refused."*
* **The task route has no mutex.** A module mentioning `mutex guards` selects
  the mutex route whole-module, which then refuses the task profile's
  `spawn`/`join` expressions outright. Measured verbatim:

  > `Q600` — the scoped task profile's `spawn`/`join` expressions are outside
  > source.mutex.guard.v1; the profile refuses completion-capsule composition.
  > Its own child-task form is a nested `thread scope NAME { … }`

So a program in which a mutex protects a Buffer that crosses a task boundary
cannot be written. CT46's *accept* half has no source text, and the refusal a
CT46-shaped module actually gets is a **route refusal**, not a transferability
decision. Mapping CT46 on that refusal would be an overclaim: the harness would
record a case whose expectation is "accept ... through verified mutex protocol"
against evidence that never reaches the protocol.

**No CT rows are proposed. CT stays 31/50. No CT file was edited.**

### 5.3 Why counted read shares are not the answer

`own<Buffer>` *is* shareable — under `ShareProtocol::CountedReadShares`. That
is a verified protocol, and it is not the one CT46 names, for a stated reason:
counted shares make sharing safe by **removing mutation** for the window
(`WriteBuffer` requires exclusive authority, so no write can occur while any
share is outstanding). A mutex protocol would instead **order** mutation. A
payload is non-`Sync` precisely because it is mutated; the share protocol
admits readers only, so it cannot stand in for the mutex CT46 asks for.

### 5.4 The residue, made testable

Rather than leaving that as prose, the gap is data:

* `ShareProtocol::CheckedMutex` exists in the judgment as a **represented
  refusal**.
* `MUTEX_SHARING_GAP` records the kind, the protocol and `requestable: false`.
* `noModeRequestsTheMutexProtocol` decides over the whole 13x4 matrix that
  nothing anywhere produces it.
* `theGapKindIsSendableAndOtherwiseShareable` proves the kind it would apply to
  really is the sendable, otherwise-shareable one — so this is CT46's question,
  not an uninteresting refusal.

Adding a mutex-mediated capture mode later must therefore update these tests
and supply a rule for `Backed (.lendShare .checkedMutex)`, whose current value
is `False`.

### 5.5 The blocker, retired and replaced

Recorded for whoever next touches CT46 — the wave-5/9/11 sentence is now
**stale** and should be superseded:

> **Old:** "Needs Send/Sync derivation, which is outside L09 and outside this
> profile entirely."
>
> **New:** "The Send/Sync derivation exists (`qcore_core::transfer`, card
> CT46-SENDSYNC) and classifies `own<Buffer>` as sendable-but-not-freely-
> shareable, which is CT46's payload. What remains is a **capture form**: the
> mutex route's child task carries no captures
> (`source_mutex.rs:1049-1149`) and the task route refuses mutexes
> (`source_mutex.rs:414-424`), so no source text can ask for mutex-mediated
> sharing across a task boundary. CT46's `accept` half is unwritable until the
> mutex and task ledgers compose over a capture."

Named follow-on: **CT46-MUTEX-CAPTURE**. Note that
`T03-GUARD-JOIN`'s admission is explicitly the opposite direction — it admits a
guard-local join only when *"the whole joined subtree named no mutex, guard or
guard-scoped fact anywhere"* — so the two ledgers are currently composed by
mutual exclusion, and CT46 needs them composed by sharing.

---

## 6. Lean

`proof/QCore/Transferability.lean`, audited by
`proof/AuditTransferability.lean`. **27 theorems, no new axiom, no
placeholder.** Only `propext` and `Quot.sound` appear; nine theorems depend on
no axiom at all.

### Totality and closure (10)
`kindInventoryIsComplete`, `kindInventoryHasNoDuplicates`,
`kindInventoryHasThirteenEntries`, `tagInventoryIsComplete`,
`tagInventoryHasNoDuplicates`, `tagInventoryHasFifteenEntries`,
`modeInventoryIsComplete`, `modeInventoryHasFourEntries`,
`everyTagMapsIntoTheVocabulary`, `noDeclaredTagIsSendableOnItsOwn`.

### The classification and CT46's two directions (3)
`theClassificationIsPinned`, `exactlyOneKindIsSendableButNotFreelyShareable`,
`guardsFactsAndMutexesNeverCross`.

### Both matrices decided whole (4)
`theAdmissionMatrixIsPinned` (13x4, three admitting cells),
`noKindAdmitsAWriteCapture`, `theCaptureMatrixIsPinned` (13x4x13, three of
676), `captureAdmissionRefinesKindAdmission`.

### Internal coherence (3)
`everyAdmissionIsSendable`, `refusalIsExactlyNonAdmission` (an equality of
decisions, so neither direction can be lost),
`anAdmittedLendNamesItsClassProtocol`.

### The recorded gap (2)
`noModeRequestsTheMutexProtocol`, `theGapKindIsSendableAndOtherwiseShareable`.

### Per-kind soundness, tied to existing ledger theorems (5)
`moveObligationHolds` — discharged by
`QCore.ScopedTasks.rollback_restores_original_custody` **and**
`reserved_resource_is_unavailable_to_parent`.
`lendObligationHolds` — discharged by
`QCore.ReadShare.an_outstanding_share_suspends_exclusive_authority`.
`everyAdmittedTransferIsBacked` — over the whole matrix, whichever cell admits,
the corresponding rule is available; the `checkedMutex` arm maps to `False`,
so admitting it later demands a rule rather than a comment.
`aRefusingKindNeedsNoRule`, `duplicationCarriesNoCustody`.

Nothing here proves the ledger rules — `QCore.ScopedTasks` and
`QCore.ReadShare` do, and neither is edited. What is established is that the
judgment's recording of *which rule answers for which kind* is accurate.

---

## 7. Gate

Run in this worktree at `2dac1a7`.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2143 passed, 0 failed, 0 ignored** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **54 audits pass** (was 53) |
| `providers/` `cargo test` | 59 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 124 pass, **1 fail by design** — see §7.1 |

### 7.1 Two items for the integrator

1. **Expected-by-design failure.**
   `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
   fails: `['8.0.0', 53] != ['8.0.0', 54]`. The committed acceptance matrix
   records 53 Lean audits and this tree has 54. `acceptance_matrix.json` is on
   the card's do-not-edit list, so it is deliberately untouched; regenerating
   the matrix at merge clears it.

2. **Pre-existing environmental flake, not caused by this card.**
   `qcore-verifier-host` `a_confined_fixture_cannot_write_outside_the_admitted_paths`
   failed under the first concurrent workspace run
   (`Broken pipe (os error 32)`, `Io` where `Terminated` was expected). It was
   **verified to fail identically with `67961f1`'s `source_tasks.rs` and
   `lib.rs` restored byte-for-byte in place**, and it passes under
   `--no-fail-fast`. This is the P03 confined-write flake named in the brief.
   The assertion was not weakened and the file was not touched.

---

## 8. What this does not establish

* **No data-race proof.** The judgment says which kinds the implemented rules
  already govern well enough to let cross a boundary. Data-race freedom on the
  share route still rests on the checker's no-write-while-loaned rule, not on
  measurement (T04 records that ThreadSanitizer does not intercept C11
  `thrd_create`).
* **No source-to-native refinement** and no statement about C layout.
* **No new admission.** Every kind the judgment admits was already admitted;
  the composition can only cause refusals.
* **No mutex-mediated sharing.** Named as a refusal, supplied by nothing.
* **No verdict, coverage or pin movement.** Raw Cycle 19 units stay
  2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only; CT stays
  31/50; the contract corpus stays 60 modules; `qcore-schemas` stays 8.0.0.
  No diagnostic code added or changed; no reserved word added.

## 9. Open, named

| Id | What remains |
|---|---|
| `CT46-MUTEX-CAPTURE` | The capture form CT46 needs: composing the mutex ledger and the task ledger over a capture, so a mutex-protected payload can cross a task boundary. Until then CT46's *accept* half is unwritable and the case stays unmapped. |
| `CT46-SENDSYNC-UNREACHABLE-ARMS` | Four judgment arms are classified but unreachable from source on the task route: `TaskResult`, `Guard`, `ProtectedFact`, `MutexGeneration` (and `Unit` by position). They are classified deliberately so the judgment stays total; a future route that can name them will exercise them. |
| `CT46-SENDSYNC-ENUM-RETAINED` | The `spawn_inputs` enumeration is retained beside the judgment by decision (§2.3), not by necessity. Deleting it is possible and would move the pinned refusal text's emission site. |

## 10. Files

| Path | Change |
|---|---|
| `crates/qcore-core/src/transfer.rs` | new — the judgment |
| `crates/qcore-core/src/lib.rs` | `pub mod transfer;` |
| `crates/qcore-core/tests/unit/transfer.rs` | new — 16 tests |
| `crates/qcore-frontend/src/source_tasks.rs` | the checked-redundant composition at `spawn_inputs`, plus the three total mapping functions and the non-scalar guard |
| `crates/qcore-frontend/tests/transferability.rs` | new — 8 source-level tests |
| `proof/QCore/Transferability.lean` | new — 27 theorems |
| `proof/QCore.lean` | one import |
| `proof/AuditTransferability.lean` | new — the audit |
