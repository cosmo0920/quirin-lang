# T04 — `ScopedReadLoansV1`: one bounded scoped read loan

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Date: 2026-09-08. Base checkpoint: `9478b94` (the delegated-increment tip).
Depends on the owned-task profile it extends
([W6 owned source tasks](../w6-owned-source-tasks/README.md),
[completed-child custody](../w6-completed-child-custody/README.md)) and follows
[T03](../remaining-t03/README.md)'s shape for a bounded source increment.

**Scope.** One parent-owned `Buffer`, at most one scoped child read loan, and an
explicit join-dependent return of authority. This card adds one borrowing form
to an existing profile. It does not implement shares, does not implement
borrowing in general, does not promote any raw Cycle 19 unit, and maps no CT
case: mappings are *proposed* in §8 and `docs/reevaluation/ct_manifest.json` is
untouched at 11/50.

---

## 1. What landed

| Artifact | Change | Role |
|---|---|---|
| `crates/qcore-core/src/scoped_tasks.rs` | +379/−7 | loan custody in the checked ledger, plus four tests |
| `crates/qcore-core/src/task_execution.rs` | +174/−17 | `ReadLoan` type, `Loan` operand, all-paths checking |
| `crates/qcore-core/src/task_execution/reference.rs` | +95/−13 | reference execution and its own refusals |
| `crates/qcore-core/src/task_c_emit.rs` | +78/−7 | native lending, and the loan/return observation |
| `crates/qcore-frontend/src/source_tasks.rs` | +72/−10 | `borrow<Buffer>` typing and the `read … share FULL` capture |
| `crates/qcore-verification/src/driver.rs` | +21/−2 | `SourceExecutionProfile::ScopedReadLoansV1` and its selection |
| `crates/qcorec/src/main.rs` | +3 | the `check` label |
| `crates/qcore-frontend/tests/read_loans.rs` | +427 (new) | 10 source-level acceptance tests |
| `compiler/tests/scoped_read_loans.rs` | +342 (new) | 4 tests, 3 of them sanitized native runs |
| `proof/QCore/ReadLoan.lean` | +318 (new) | 17 public theorems, 4 private lemmas, 5 decidable scenarios |
| `proof/AuditReadLoan.lean` | +19 (new) | axiom registration for all 17 |
| `proof/QCore.lean` | +1 | the import |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02bb | +~60 | the normative admitted grammar |

Nine existing ledger tests, the 26 pinned `TrustedExplicitTrace` tests, T02's
mutex checker, T03's guard lowering and every pinned CT case are unedited.
`qcore-schemas` is untouched: no wire version, plugin protocol, evidence
version or manifest field changed.

### 1.1 Why this extends the task profile instead of adding a parallel one

T02 and T03 built a *new* checker beside an unedited one because a mutex
invariant is a different model. A read loan is not: it is one more custody
state in the model the scoped-task profile already has, and the three
acceptance clauses are about mechanisms — reservation rollback, completion
capsules, creator-only join — that profile already implements. Building a
second, parallel task checker would have duplicated `match` lowering, the
all-paths explorer, the C11 runtime and the reference execution, and would have
left two checkers to keep in agreement. The cost of the choice is that four
pinned production files changed; the mitigation is that every existing test
passes unedited and every new behaviour is reachable only through the new
syntax.

---

## 2. The admitted grammar, exactly

Normative copy in
[`QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02bb](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).
No parser change was needed: `read PLACE share EXPR` captures and
`borrow<…>` type annotations already parse (the latter since T03 reserved
`borrow`). This card adds their *typing*, not their syntax.

```qcore
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}
…
match spawn s reader(read buffer share FULL) {
    Result::Ok { value: handle } => {
        let during = qcore_buffer_read(buffer);   /* lender reads while lent */
        let seen = join move handle?;             /* the loan closes here */
        let after = qcore_buffer_read(buffer);
        drop buffer;                              /* admitted only now */
        return seen;
    }
    Result::Err { error: failure } => {
        let restored = qcore_buffer_read(buffer); /* nothing was lent */
        drop buffer;
        return restored;
    }
}
```

A loan parameter is exactly `NAME: borrow<Buffer>` with **no** usage grade. The
only capture that produces one is `read PLACE share FULL` against a place of
type `own<Buffer>`.

### 2.1 The two-level authority split

| Authority | Owned | Reserved | Loaned | Dropped |
|---|---|---|---|---|
| exclusive (mutate, release, move, re-lend) | owner | nobody | **nobody** | nobody |
| shared read | owner | nobody | owner **and** borrower | nobody |

`use_resource` is the exclusive gate and `read_resource` the shared one. The
owner never stops owning a loaned resource — the refusal is
`LoanOutstanding { resource, borrower }`, deliberately not `NotCustodian`,
because a custody complaint would be a false statement about where ownership
sits.

### 2.2 Structural bounds, enforced rather than assumed

* at most one loan argument per creation (frontend *and* core);
* at most one outstanding loan at a time (core, checked against the ledger);
* a resource is never both moved and lent by one creation;
* `share FULL` only — an integer, a sum or a named share is refused with
  "fractional read shares are outside this profile", because splitting read
  authority is T05's profile and not a larger version of this one.

### 2.3 A loan is not a value

Refused as a function result, an entry parameter, a copy source, a move
source, a `drop` operand and a re-loan source. `qcore_buffer_read` is the only
operation that accepts one. Two independent reasons make transitive borrowing
unreachable: a loan is not an `own<Buffer>` place (measured refusal: "capture
type does not match worker parameter"), and a spawned worker is a first-order
leaf (measured refusal: "first-order leaf functions").

---

## 3. Acceptance

### 3.1 Parent mutation and destruction are rejected while the loan is outstanding

| Attempt | Measured refusal | Where |
|---|---|---|
| `drop buffer;` between spawn and join | `Q600` `LoanOutstanding { resource: 1, borrower: 2 }` | `the_lender_cannot_release_or_relend_while_the_loan_is_outstanding` |
| lend the same Buffer to a second child | `Q600` `LoanOutstanding { resource: 1, borrower: 3 }` | same |
| lend a *different* Buffer while one is outstanding | `Q600` "at most one outstanding read loan at a time" | same |
| exit the scope without joining | `Q600` `ScopeBusy` | `a_path_that_never_joins_its_borrower_cannot_leave_the_scope` |
| ledger-level: `use_resource`/`drop_resource`/`reserve` by owner **or** borrower | `LoanOutstanding` | `an_outstanding_read_loan_suspends_exclusive_authority_and_admits_two_readers` |

The source grammar has no Buffer-write statement, so "mutation" is exercised
through the exclusive-authority gate that a write would have to pass, not
through a write. That gap is recorded in §7.

Read authority is checked in both directions: the owner and the borrower may
read a loaned Buffer, a third task may not (`NotReadable { task: 2, resource: 7 }`),
and the reservation window grants nobody anything, read included.

### 3.2 Create failure restores capture state; failed join does not reclaim; successful join closes the loan

| Clause | Evidence |
|---|---|
| creation failure restores the lender exactly | ledger: the restored map equals the pre-reservation map and an unrelated live sibling is untouched; source: the failure arm reads and releases its Buffer; native: a forced `thrd_error` from `thrd_create` prints no `QCORE_BUFFER_LOAN` and still releases the one allocation |
| a rolled-back reservation cannot be replayed | `commit`/`rollback` both refuse afterwards (`ReservationResolved`) |
| completion is not the close | a completed-but-unjoined child still holds the loan; `drop` still refuses |
| failed join reclaims nothing | `fail_join` leaves the loan outstanding, refuses a retry (`JoinFailed`), refuses a second failure, refuses the scope exit and refuses the release; natively, `forbidden_free` would exit 91 and the run exits 77 at the trapped abort with one allocation outstanding, no `QCORE_BUFFER_RETURN` and no `QCORE_TASK_JOIN` |
| successful join closes it | authority returns to the original owner exactly; `QCORE_BUFFER_RETURN` is printed only after a collection that returned |

### 3.3 Stack/environment expiry and worker-error paths preserve outstanding lifetime duties

* **Scope expiry.** A `thread scope` cannot close while its borrower's handle is
  unconsumed, so the lending window can never outlive the scope that opened it.
  A failed join makes that permanent: the handle is never consumed, so the scope
  is refused forever.
* **Environment.** The lender's binding is *not* consumed by lending (unlike a
  move), and the emitted C reflects exactly that: a moved capture is nulled, a
  lent one is not. That is what makes the post-join read sound.
* **Worker-error paths.** A worker that raises still had its loan closed by the
  parent's join, because the loan is closed by the *join*, not by the outcome;
  the parent's own duty to release its Buffer survives the error arm and is
  checked on it. A worker cannot smuggle its loan out through any exit: it is
  not returnable, droppable, copyable or lendable.

---

## 4. Refusal taxonomy

Two kinds, separated so that "the profile refuses X" is testable:

* **Ledger refusals** — the operation reaches the custody model and is refused
  there: `LoanOutstanding`, `NotReadable`, `LoanNotOutstanding`, `JoinFailed`,
  `ScopeBusy`. These surface as `Q600` through the existing task route.
* **Typing refusals** — the construct never becomes a loan operand:
  "a read loan cannot be copied", "drop requires an owned buffer", "move return
  requires own<Buffer>", "task functions support i32, unit and own<Buffer>
  results", "task parameters require i32, own<Buffer> use one, or an ungraded
  borrow<Buffer>", "only copy i32, move own<Buffer> and read own<Buffer> share
  FULL captures are executable", "fractional read shares are outside this
  profile".

**No diagnostic code was added or changed.** `Q6xx` is already the
`Concurrency` pass's range and `Q600` is the existing scoped-task refusal
(`docs/QCORE_SPEC_r0_3.md` §L21); typing refusals keep the `Q020`/`Q500` codes
the task route already emits at those sites.

### 4.1 One pinned expectation was preserved rather than re-pinned

Routing `ReadBuffer` through the new shared-read gate initially changed the
message for a *stale* slot from "slot N is not an initialized owned buffer" to
"…readable buffer", which broke CT02's pinned rejection. The pin was not
edited: the new gate now reports the original wording verbatim for a slot that
is neither owned nor a loan, so a moved-away, consumed or scalar slot reports
exactly what it reported before. Adding a borrowing form must not change what a
stale slot says.

---

## 5. Native evidence

Three fixed scenarios, each compiled from the real emitted C with
`-std=c11 -pthread -Wall -Wextra -Werror -g -O0 -fsanitize=address,undefined`
and run with `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, then compared line for line
with the independently computed reference execution.

| Run | Fixture | What it witnesses |
|---|---|---|
| 1 | lend / read from both sides / join / read / release | full agreement with the reference trace; exactly one heap object allocated and released exactly once; `SPAWN < LOAN < child READ < JOIN < RETURN < DROP` |
| 2 | forced `thrd_create` failure | no `QCORE_BUFFER_LOAN` at all, the lender still releases its object, full trace agreement |
| 3 | forced `thrd_join` failure | exit 77 at the trapped abort, one allocation outstanding, zero releases, no `RETURN`, no `JOIN` |

LeakSanitizer is enabled for runs 1 and 2 and deliberately **disabled** for run
3, where the retained allocations are the observation rather than a defect;
`forbidden_free` would exit 91 if anything were reclaimed.

That exactly one heap object exists for the whole lifetime is what makes the
borrower's read and the lender's reads reads *of the same object*: lending
produced no copy.

### 5.1 No race detector covers this backend, and that is measured

This is the first source profile in which two live threads touch one heap
object, so a ThreadSanitizer run would be the natural evidence. It is not
available: ThreadSanitizer does not intercept C11 `thrd_create`, and a
`-fsanitize=thread` build of *any* emitted task program dies with
`SEGV on unknown address 0x18` in the child thread before the worker body runs
(exit 66). That was confirmed against the pre-existing CT01 owned-task fixture,
which lends nothing, so it is a property of the backend's thread primitive and
not of this card. **Data-race freedom for the lending window rests on the
checker's rule that nothing may write a loaned resource, not on a dynamic
detector.** Moving the backend off `thrd_create`, or adding a pthread emission
mode purely for race testing, is recorded as open.

---

## 6. Lean

New `QCore.ReadLoan` namespace over a four-state authority model
(`owned`/`reserved`/`loaned`/`dropped`), with `canUseExclusively`, `canRead`
and the four transitions `reserve`, `rollback`, `commit`, `closeLoan`, plus
`release`.

| Theorem | Obligation |
|---|---|
| `reserve_demands_exclusive_authority` | lending needs exclusive authority |
| `a_borrower_cannot_lend_onward` | no transitive borrowing |
| `an_outstanding_loan_cannot_be_lent_again` | no second loan, by anyone |
| `failed_creation_restores_the_owner_exactly` | acceptance 2, pointwise everywhere |
| `a_reserved_resource_grants_no_authority_at_all` | the creation window grants nothing |
| `an_outstanding_loan_suspends_exclusive_authority` | acceptance 1, for every task |
| `an_outstanding_loan_cannot_be_released` | the same for release specifically |
| `a_loan_admits_exactly_its_owner_and_its_borrower` | read authority, both directions |
| `commit_lends_only_the_reserved_resource` | frame |
| `commit_requires_its_own_reservation` | no reservation confusion |
| `a_rolled_back_creation_cannot_be_committed` | rollback is terminal |
| `closing_a_loan_requires_the_recorded_pair` | a join closes only its own loan |
| `a_retained_loan_refuses_every_reclamation` | the failed-join state, forever |
| `a_successful_join_restores_exclusive_authority_to_the_owner` | to the owner and nobody else |
| `a_successful_join_revokes_the_borrower_read` | the borrower loses it |
| `closing_a_loan_preserves_every_other_resource` | frame |
| `the_admitted_lending_round_trip_is_authority_neutral` | reserve∘commit∘close = identity |

Four private lemmas support them; five decidable `example`s mirror the Rust and
source fixtures. `AuditReadLoan.lean` registers all seventeen and
`check_lean_audits.py` passes **22** audits (was 21) with warnings as errors.
Every theorem depends on `propext` or on nothing: **no new axiom, no `sorry`,
no placeholder.**

### 6.1 Named Lean boundaries

1. **Source and native extraction are premises.** No theorem says the Rust
   lowering produces this model for the source it reads, or that the emitted C
   implements it.
2. **The model is custody only.** It has no threads, no schedule, no memory
   model and no time; "the borrower reads concurrently" is a Rust/native
   observation, not a Lean one.
3. **One resource at a time.** The theorems quantify over one resource and are
   framed over the others; there is no theorem about an aggregate of loans,
   because the profile admits at most one.

---

## 7. Boundaries, stated as limits

* **No write statement.** The source grammar has no Buffer mutation, so
  "parent mutation is rejected" is discharged through the exclusive-authority
  gate a write would have to pass, not by refusing an actual write. Adding
  `qcore_buffer_write` is the natural next increment and would turn acceptance 1
  into a directly executed case (and would make CT04 mappable).
* **The exit rule's loan check is unreachable from source.** A loan always has
  an unconsumed handle in its scope, so `ScopeBusy` fires before the
  return/raise exit rule can report "lent to an unjoined child". The check is
  kept as a fail-closed backstop and is recorded here as redundant rather than
  quietly left looking load-bearing.
* **One loan, one Buffer, one borrower, `share FULL`.** Fractional and
  multi-reader shares (T05), write loans, loans of anything but a Buffer, loans
  across nested workers and loans held past a scope are all refused, not
  approximated.
* **One target.** Linux, C11 threads, one compiler invocation. Non-Linux
  emission is refused at `Q500` as before.
* **No race detector** (§5.1).
* **`fail_join` is a ledger operation, not a checker branch.** The all-paths
  checker does not explore a join-failure successor, because the native
  behaviour is `abort()` and there is no continuation to check. The retained
  state is pinned by ledger tests, a Lean theorem and a native run instead.

---

## 8. Proposed CT mappings — for the integrator, not applied

`docs/reevaluation/ct_manifest.json` is **not edited**, and the CT harness
fails closed on a fixture directory whose case still reads `unmapped`, so the
candidate fixtures live in [`fixtures/`](fixtures/) here.

| Case | Scenario | Fixture | Measured result | Proposed stages |
|---|---|---|---|---|
| **CT25** | cleanup destroys owner after joins and loan expiry → Accept | `fixtures/CT25/case.qcore` | `checked 2 function(s); entry main; scoped owned-buffer tasks with one read loan (Linux C11)`, exit 0 | parser, type_checker, reference_execution, native_backend **executed**; cfg, proof **unassessed** |
| **CT24** | cleanup destroys an owner before child join → Reject | `fixtures/CT24/case.qcore` | `case.qcore:11:9: Q600: LoanOutstanding { resource: 1, borrower: 2 }`, exit 1 | parser, type_checker **executed** |

`cfg` is proposed `unassessed` for CT25 on purpose: this route emits from the
checked task IR and has no sequential CFG stage, so claiming `executed` would
be false.

**Deliberately not proposed**, with the reason:

* **CT03** (parent and child share reads with complementary positive shares) and
  **CT04** (parent writes while child retains a read share) — this profile has
  exactly one whole read share and no write statement. CT03 needs share
  arithmetic (T05); CT04 needs a write.
* **CT18** (worker error payload contains a scoped capture borrow) — errors here
  are payload-free, so the refusal would be "payloads do not exist", which is
  weaker than the case asks.
* **CT44** (stack capture expires or environment moves before join) — the
  measured refusal is about a lexical `thread scope`, not a stack frame, and
  this profile has no stack captures at all. Claiming it would overstate.

---

## 9. Gate results

Run at this card's tip, on Linux, after `git submodule update --init --recursive`.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1098 passed, 0 failed** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **51 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests` | **43 tests, OK** |
| `cd proof && lake build` | success, no warnings |
| `python3 -B scripts/check_lean_audits.py` | **Passed 22 Lean audits** |

1098 − 1080 = **18 new tests**, exactly the 4 + 10 + 4 added here.

### 9.1 Pinned artifacts

* **Not re-measured, and this is by construction rather than by measurement.**
  The four pinned 185-unit Cycle 19 artifacts and the configured CMetrics audit
  were not re-run: this card touches only the *source-language* compilation
  route inside `compile_files`. The C audit path
  (`qcore-audit` → `qcore-c-gateway`) does not call it, and no gateway,
  contract, profile, schema, policy or provider file was edited.
* The contract corpus stays at 59 modules, the `third_party/libevent` submodule
  pin is unchanged, and `docs/reevaluation/ct_manifest.json` is untouched at
  11/50 mapped.
* No raw Cycle 19 unit is promoted. `qcore-schemas` stays 4.0.0; prose revision
  0.3, plugin protocol v1, evidence v2 and program schema/profile v2 are
  unchanged.

### 9.2 Recovered work, recorded for the integrator

Mid-card, an unrelated worktree's `git stash pop` consumed this card's stash
entry, because the stash stack is shared across worktrees in this repository.
The uncommitted work was recovered from dangling commit `28bc95d2` and is
present in commit `9a56ae2`; the other agent's displaced files were removed
from this worktree and their content remains in dangling commit `470db817`.
Nothing was lost and nothing of theirs is committed here. No further `git
stash` was used.

---

## 10. What T05 and T07 can build on

* `ResourceState::Loaned { owner, borrower }` is the single custody state to
  generalize: T05's fractional split replaces the one borrower with a share
  map, and the `outstanding_loans()` bound in `check_function` is the exact
  line that stops being a hard "at most one".
* `read_resource` versus `use_resource` is already the shared/exclusive split a
  share algebra needs; only the *admission* rule changes.
* `ScalarOperand::Loan` and `ScalarType::ReadLoan` are the operand and type a
  graded loan would refine; `share FULL` is already parsed and refused for
  every other expression, so T05's work is typing, not syntax.
* The `QCore.ReadLoan` authority model is the obvious place to add a share
  parameter: `a_loan_admits_exactly_its_owner_and_its_borrower` becomes the
  degenerate case of a share-indexed statement.
* For T07, `fail_join` plus `a_retained_loan_refuses_every_reclamation` is the
  first statement in this project that a *failed* observation leaves duties
  intact permanently, rather than merely not discharging them.
