# T05 — `ScopedReadSharesV1`: complementary fractional read shares

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Date: 2026-09-08. Base checkpoint: `c60fd91` (the second delegated-increment
tip). Direct predecessor: [T04, the bounded scoped read
loan](../remaining-t04/README.md), whose §10 named exactly the three places this
card generalizes. Share-algebra precedent: [T02's pointwise share conservation
over interval atoms](../remaining-t02/README.md).

**Scope.** Two readers of one generation-indexed allocation under a fixed flat
scope. This card adds a *count* to the authority T04 lent, and adds the Buffer
write whose exclusive-authority requirement makes that count load-bearing. It
does not implement disjoint slicing, write shares, more than two readers,
several lent resources at once, nested workers or general points-to analysis;
each of those is a measured refusal below. It promotes no raw Cycle 19 unit and
maps no CT case: mappings are *proposed* in §9 and
`docs/reevaluation/ct_manifest.json` is untouched at 15/50.

---

## 1. What landed

| Artifact | Change | Role |
|---|---|---|
| `crates/qcore-core/src/scoped_tasks.rs` | +684/−188 (whole-file net) | the share-indexed ledger, `lend`/`reclaim`, four new refusals, six new tests |
| `crates/qcore-core/src/task_execution.rs` | +98/−… | `Loan { slot, share }`, `WriteBuffer`, the relaxed one-resource bound |
| `crates/qcore-core/src/task_execution/reference.rs` | +67/−… | share-carrying reservations, the write and its event |
| `crates/qcore-core/src/task_c_emit.rs` | +37/−… | `qcore_buffer_write` runtime and emission |
| `crates/qcore-frontend/src/source_tasks.rs` | +126/−… | share evaluation, the `qcore_buffer_write` form |
| `crates/qcore-frontend/src/frontend.rs` | +1 | `qcore_buffer_write` as a resolvable intrinsic name |
| `crates/qcore-verification/src/driver.rs` | +36/−… | `SourceExecutionProfile::ScopedReadSharesV1` and its selection |
| `crates/qcorec/src/main.rs` | +3 | the `check` label |
| `crates/qcore-frontend/tests/read_shares.rs` | +385 (new) | 9 source-level acceptance tests |
| `compiler/tests/scoped_read_shares.rs` | +328 (new) | 4 tests, 3 of them sanitized native runs |
| `crates/qcore-frontend/tests/read_loans.rs` | +62/−… | two deliberate re-pins (§3) |
| `proof/QCore/ReadShare.lean` | +513 (new) | 19 public theorems, 6 private lemmas, 6 decidable scenarios |
| `proof/AuditReadShare.lean` | +21 (new) | axiom registration for all 19 |
| `proof/QCore.lean` | +1 | the import |
| `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02bc | +~91 | the normative admitted grammar |

Nothing under `contract/`, `crates/qcore-schemas/`, `crates/qcore-audit/`,
`crates/qcore-c-gateway/`, `compiler/src/`, `docs/reevaluation/` or
`docs/reviews/` was touched. **`qcore-schemas` stays 5.0.0**: no wire version,
plugin protocol, evidence version or manifest field changed, and the integrator
has no schema-version decision to make from this card.

### 1.1 Why this extends the read-loan profile instead of adding a parallel one

T04's report §10 said the generalization points were `ResourceState::Loaned`,
the `read_resource`/`use_resource` split, and the `outstanding_loans()` bound in
`check_function`. All three are exactly where this card works. Building a second
task checker beside the first would have duplicated the `match` lowering, the
all-paths explorer, the C11 runtime and the reference execution, and would have
left two checkers to keep in agreement about one custody state. The cost is two
deliberately re-pinned expectations (§3); the mitigation is that every other T04
test, every T03 mutex test and every mapped CT case passes unedited.

---

## 2. The model, exactly

### 2.1 A lending budget, not a separation-logic fraction

This is the one modelling decision a reader should check before anything else.

Each resource generation has a budget of `FULL_READ_AUTHORITY` = 1024, the same
unit as the concurrency model's `FULL_ACCESS`, so a share here and a share in
T02's pointwise ledger are the same quantity. **What is budgeted is the
authority handed *out*.** Lending transfers a positive share to a borrower; the
outstanding total may never exceed the budget; exclusive authority exists
exactly when the outstanding total is zero.

The owner's own read is **not** budgeted. It is retained across committed
lending, exactly as T04 retained it, and it is sound only because a write
demands the reconstructed exclusive authority this card is about. In the
classical separation-logic reading, lending `p` would leave the owner `1 − p`,
so `share FULL` would leave the owner unable to read its own Buffer. That
reading is **not** implemented here, and the difference is deliberate:
implementing it would have silently changed what T04's canonical fixture means
— its parent reads while lending `share FULL` — rather than extending it.

The consequence, stated plainly so nobody has to infer it: this profile bounds
*how much authority can be simultaneously outstanding* and *when full authority
is reconstructed*. It does not model the owner's read as a consumed resource.
The fractional-permission variant in which it is consumed is a named open
boundary (§8), not a claim made here.

### 2.2 The custody state

```rust
ResourceState::Loaned { owner: TaskId, shares: Vec<(ShareHolder, u32)> }
ShareHolder::Borrower(TaskId) | ShareHolder::Pending(ReservationId)
```

Ordered, no repeated holder, every share positive, total at most
`FULL_READ_AUTHORITY`. `shares` is never empty in a reachable state: the last
reclamation restores `Live(Custodian::Task(owner))`, which is byte-identical to
the state before the first lend.

| Authority | Owned | Loaned, share committed | Loaned, creation in flight | Dropped |
|---|---|---|---|---|
| exclusive (write, release, move, re-lend) | owner | **nobody** | **nobody** | nobody |
| read | owner | owner **and** every positive-share holder | every positive-share holder | nobody |

`Pending` is why the owner loses its read during a creation window: the share
has left the owner and reached nobody. That preserves T04's pinned "the
reservation window grants nobody anything, read included" verbatim, while a
*committed* sibling keeps reading throughout.

### 2.3 Structural bounds, enforced rather than assumed

* at most one loan argument per creation (frontend *and* core);
* at most one lent **resource** at a time (core) — the card's deliverable is two
  readers of *one* allocation;
* at most `MAX_READERS` = 2 simultaneous holders per resource (ledger), refused
  even when authority remains;
* a resource is never both moved and lent by one creation;
* every share is positive and at most `FULL`, checked in the frontend *and*
  again in the ledger — two gates, neither relying on the other.

### 2.4 The write

`qcore_buffer_write(BUFFER, VALUE)` evaluates to `unit`, takes one exact
`own<Buffer>` name and one i32, and demands exclusive authority. T04's report
named this as the missing piece that would "turn acceptance 1 into a directly
executed case"; it is now that. A borrower cannot reach it at all — a read loan
is not an `own<Buffer>` binding — and an owner cannot reach it while any share
is out.

---

## 3. Two pinned expectations were re-pinned, deliberately

Both are recorded here and in the tests themselves, per the project rule that a
conflicting pin must be updated deliberately and said so.

| Pin | T04 | T05 | Direction |
|---|---|---|---|
| a second whole loan of one resource | `Q600 LoanOutstanding { resource: 1, borrower: 3 }` | `Q600 ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1024 }` | still refused; the reason is now quantitative |
| `share 1`, `share (1 + 1)` | refused: "fractional read shares are outside this profile" | **admitted** | this is the capability the card adds |

The first is a change of *refusal reason*, not of direction: the program is
still refused, and the new message names the outstanding total and the request.
The second is the card's deliverable, so the T04 test
`only_a_full_read_share_is_admitted` was rewritten as
`a_read_share_must_be_a_positive_fraction_of_the_full_authority`, which pins
both the newly admitted forms and the forms that are still refused (zero,
over-full, a resolvable name with no declared constant, and — one gate earlier —
an unresolvable name). `LoanOutstanding`'s payload changed shape from one
`borrower` to `outstanding` plus `borrowers`, because with two holders naming
one of them would be a false statement; that changed three assertion strings.

No diagnostic **code** was added or changed. `Q600` remains the scoped-task
refusal and `Q013`/`Q020`/`Q500` remain the frontend codes the task route
already emitted at those sites, per `docs/QCORE_SPEC_r0_3.md` §L21.

---

## 4. Acceptance

### 4.1 Positive complementary shares with total ≤ full; excess and unauthorised writes refused

| Claim | Evidence |
|---|---|
| two positive complementary shares admit two readers | ledger: `Loaned { owner: 0, shares: [(Borrower(1), 512), (Borrower(2), 512)] }`, both read, owner reads, a third task gets `NotReadable`; source: `two_complementary_halves_admit_two_readers_of_one_allocation`; native: run 1 |
| the outstanding total is exactly the sum granted | ledger `lent_share` assertions; Lean `lending_adds_exactly_the_granted_share` |
| excess is refused **as a whole** | `ShareExceedsAuthority { resource: 7, outstanding: 513, requested: 512 }` with `lent_share` unchanged afterwards; source: `share 512` + `share 513` → `outstanding: 512, requested: 513`; `share FULL` twice → `outstanding: 1024, requested: 1024`; `share 1024` + `share 1` → `outstanding: 1024, requested: 1` |
| the boundary is a boundary, not a blanket refusal | `share 511` + `share 513` is **admitted**; the exact remainder always fits |
| a zero or over-full share is not a share | `InvalidShare { resource: 9, requested: 0 }` and `{ requested: 1025 }` in the ledger; `"a read share must be positive and at most FULL (1024); this capture requests 0"` / `1025` / `2048` in the frontend |
| a write without exclusive authority is refused | between the creations: `LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2, 3] }`; after one join: `{ outstanding: 512, borrowers: [3] }`; a borrower writing: `"buffer write requires own<Buffer>; a read loan grants no write authority"`; after release: `"slot 4 is not an initialized owned buffer"` |
| a third simultaneous reader is refused even with authority to spare | `TooManyReaders { resource: 7, limit: 2 }` at 1 + 1 of 1024 |
| only the owner lends | a borrower gets `NotCustodian`, a non-task gets `UnknownTask`; Lean `only_the_owner_may_lend` |

### 4.2 Reconstruction to full is rejected while any share is retained

| Claim | Evidence |
|---|---|
| one join of two reconstructs nothing | ledger: after `join(0, 30)`, `lent_share == 512`, `loan_borrowers == [2]`, and `drop_resource` still refuses with `outstanding: 512`; source: a release or a write placed between the two joins is refused with `borrowers: [3]` |
| the returned share is nobody's | `held_share(1, 7) == 0` after the joined child's share comes back; Lean `a_returned_share_is_held_by_nobody` |
| completion is not the return of a share | both children `complete`, `lent_share` still `FULL` |
| the last share restores exclusive authority to the owner and nobody else | Lean `returning_the_last_share_restores_exclusive_authority`, an iff |
| a path that never joins a borrower cannot leave the scope | `Q600 ScopeBusy` |
| a failed join returns no share, forever | native run 3: exit 77 at the trapped abort, no `QCORE_BUFFER_RETURN`, no `QCORE_TASK_JOIN`, zero releases; Lean `a_retained_share_refuses_every_reclamation` |
| a failed creation returns only its own pending share | ledger: the sibling's committed share is untouched and the post-rollback state is *equal* to the pre-reservation state; native run 2 |

### 4.3 Two readers count the allocation's live bytes once; pointer copies preserve permission identity

| Claim | Evidence |
|---|---|
| one allocation, not two | native run 1: exactly one `SHARE_ALLOC` address and exactly one `SHARE_FREE` of the same address, with two `QCORE_BUFFER_LOAN` lines and two borrower reads of generation 1 |
| the ledger counts it once | one entry in `outstanding_loans()`, one owner, one outstanding total across both holders |
| both borrowers receive the same pointer | the emitted C contains two `captures[0] = s_N.value;` lines that are textually identical, for one lender slot |
| lending mints no binding the caller loses | every clearing of the lender's slot in the emitted C is on a `qcore_buffer_drop(` line; a *moved* capture is still nulled at creation |
| a borrower cannot turn its share into a value it keeps | refused as a `drop` operand, a copy source, a move source and a re-loan source — all four T04 refusals survive unchanged |

The card's "count its live bytes once" is discharged as *one allocation counted
once*, measured by address. This profile has no byte-size accounting: the Buffer
is a fixed-size record, so there is no 4096 to count. That gap is why CT35 is
**not** proposed (§9).

---

## 5. Refusal taxonomy

* **Ledger refusals** (surface as `Q600`): `LoanOutstanding`,
  `ShareExceedsAuthority`, `InvalidShare`, `TooManyReaders`, `NotCustodian`,
  `NotReadable`, `LoanNotOutstanding`, `JoinFailed`, `ScopeBusy`, and the
  checker's own `"this profile admits at most one outstanding read loan at a
  time"` / `"…at most one read loan per task creation"`.
* **Typing refusals** (the construct never becomes a share or a write operand):
  `"a read share must be positive and at most FULL (1024); this capture requests
  N"`, `"named read shares require a declared share constant, which is outside
  this profile"`, `"a read share literal must be a decimal fraction of the full
  authority"`, `"read share sum is outside the representable authority range"`,
  `"buffer write requires own<Buffer>; a read loan grants no write authority"`,
  `"buffer write requires one owned-buffer name and one i32 value"`,
  `"buffer write requires an exact buffer name"`, `"buffer write requires an i32
  value"`, plus every T04 typing refusal unchanged.

An unresolvable name in a share position is refused one gate earlier still, by
ordinary name resolution (`Q013 unknown name`), which is pinned so no share
expression can smuggle in an unbound identifier.

---

## 6. Native evidence

Three fixed scenarios plus one emission check, each compiled from the real
emitted C with `-std=c11 -pthread -Wall -Wextra -Werror -g -O0
-fsanitize=address,undefined` and run with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, then compared line for line
with the independently computed reference execution.

| Run | Fixture | What it witnesses |
|---|---|---|
| 1 | write / lend 512 / lend 512 / both read / both join / write / read / release | full agreement with the reference trace; exactly one heap object allocated and released exactly once; `WRITE 42 < LOAN 1 < LOAN 2`, `RETURN 1 < WRITE 7`, `RETURN 2 < WRITE 7`, `WRITE 7 < READ 7 < DROP` |
| 2 | forced failure of the **second** `thrd_create` only | one loan opened, one returned, `QCORE_TASK_CREATE_FAILED`, one allocation released; the failure arm never regains write authority, so exactly one write ran |
| 3 | forced `thrd_join` failure | exit 77 at the trapped abort, one allocation outstanding, zero releases, no `RETURN`, no `JOIN`, exactly one write |
| — | emission check | both children's capture expressions are textually identical; every clearing of the lender's slot belongs to a release |

LeakSanitizer is enabled for runs 1 and 2 and deliberately **disabled** for run
3, where the retained allocations are the observation rather than a defect;
`forbidden_free` would exit 91 if anything were reclaimed.

### 6.1 No race detector covers this backend, and that is inherited, not chosen

T04 measured that ThreadSanitizer does not intercept C11 `thrd_create`: a
`-fsanitize=thread` build of *any* emitted task program dies with `SEGV on
unknown address 0x18` in the child thread before the worker body runs. That is
unchanged here, and this card would benefit from a detector more than T04 did,
because two live borrowers now touch one object. **Data-race freedom for the
lending window rests on the checker's rule that a write demands reconstructed
exclusive authority, not on a dynamic detector.** No TSan evidence is claimed.

---

## 7. Lean

New `QCore.ReadShare` namespace over a share-indexed custody model
(`owned` / `loaned owner shares` / `dropped`, with
`Holder = borrower | pending`), `lentTotal`, `shareOf`, `canRead`,
`canUseExclusively` (of which `canWrite` is a definitional alias), `lend`,
`reclaim` and `release`.

| Theorem | Obligation |
|---|---|
| `lending_adds_exactly_the_granted_share` | acceptance 1: nothing minted, nothing rounded |
| `the_outstanding_total_never_exceeds_full` | acceptance 1: the budget holds after any admitted lend |
| `excess_allocation_of_authority_is_refused` | acceptance 1: over-budget requests are `none` |
| `a_zero_share_is_not_a_share` | positivity, in every state |
| `an_over_full_share_is_refused` | a single over-full share, against an untouched resource |
| `only_the_owner_may_lend` | no onward lending, no lending by strangers |
| `a_third_simultaneous_holder_is_refused` | the reader bound is enforced, not assumed |
| `an_outstanding_share_suspends_exclusive_authority` | acceptance 1: write and release refused for everyone |
| `reconstruction_waits_for_the_last_share` | acceptance 2: one join of two reconstructs nothing |
| `a_returned_share_is_held_by_nobody` | the returned share is not somebody else's |
| `returning_the_last_share_restores_exclusive_authority` | to the owner and nobody else (an iff) |
| `returning_requires_the_recorded_share` | a join returns only its own recorded share |
| `a_retained_share_refuses_every_reclamation` | the failed-join state, forever |
| `two_positive_shares_admit_exactly_two_borrowers` | acceptance 3: two readers, one identity, strangers excluded |
| `the_owner_keeps_its_read_across_committed_lending` | §2.1's decision, stated as a theorem |
| `a_pending_share_is_readable_by_nobody` | the creation window grants nothing |
| `lending_changes_only_the_lent_resource` | frame |
| `returning_changes_only_the_lent_resource` | frame |
| `the_admitted_share_round_trip_is_authority_neutral` | lend ∘ reclaim = identity, at any admitted share |

Six private lemmas support them and six decidable `example`s mirror the Rust and
source fixtures. `AuditReadShare.lean` registers all nineteen and
`check_lean_audits.py` passes **24** audits (was 23) with warnings as errors.
Every theorem depends only on `propext` and `Quot.sound`: **no new axiom, no
`sorry`, no placeholder.**

### 7.1 Named Lean boundaries

1. **Source and native extraction are premises.** No theorem says the Rust
   lowering produces this model for the source it reads, or that the emitted C
   implements it. The Rust ledger and the Lean model were written to agree and
   are tested separately; they are not mechanically related (contrast F01, which
   does that for `check_concurrent`).
2. **The model is custody only.** It has no threads, no schedule, no memory
   model and no time; "two borrowers read concurrently" is a Rust/native
   observation, not a Lean one.
3. **`MAX_READERS` is a constant, not an induction.** The two-reader theorems
   are stated over explicit two-element share lists; there is no theorem about
   an aggregate of *n* holders, because the profile admits two.
4. **The budget is the lending budget of §2.1.** No theorem says the owner's own
   read is a consumed fraction, because in this model it is not.

---

## 8. Boundaries, stated as limits

* **The lending budget is not a separation-logic fraction** (§2.1). A profile
  in which the owner's read is itself consumed — so `share FULL` leaves the
  lender unable to read — is unimplemented and would change T04's canonical
  fixture's meaning. It is the most important open item this card leaves.
* **No disjoint slicing.** A share is a fraction of one whole resource
  generation's authority, never a region of it. There is no offset, no length,
  no struct field and no bit-field, and no points-to analysis of any kind.
* **No write shares.** Write authority is all-or-nothing: `canWrite` is
  definitionally `canUseExclusively`. A borrower can never write.
* **Two readers, one resource.** A third holder and a second simultaneously
  lent resource are both represented refusals. Neither bound is an approximation
  of a larger one that exists.
* **No byte accounting.** "Count its live bytes once" is discharged as "one
  allocation, counted once, measured by address". The Buffer is a fixed-size
  record; there is no size to sum. CT35 is not proposed for this reason.
* **No nested workers.** A spawned worker is still a first-order leaf, so no
  borrower can lend, sub-lend or spawn.
* **One target.** Linux, C11 threads, one compiler invocation. Non-Linux
  emission is refused at `Q500` as before.
* **No race detector** (§6.1).
* **The share is static.** It is a literal in the capture, evaluated at
  lowering. There is no share variable, no share arithmetic over runtime values
  and no declared share constant; `share NAME` is refused.
* **`fail_join` is a ledger operation, not a checker branch**, exactly as in
  T04: the native behaviour is `abort()` and there is no continuation to check.

---

## 9. Proposed CT mappings — for the integrator, not applied

`docs/reevaluation/ct_manifest.json` is **not edited** (15/50 mapped), and the
CT harness fails closed on a fixture directory whose case still reads
`unmapped`, so the candidate fixtures live in [`fixtures/`](fixtures/) here.
Each measured result below is from `qcorec check` at this card's tip.

| Case | Scenario | Fixture | Measured result | Proposed stages |
|---|---|---|---|---|
| **CT03** | parent and child share reads with complementary positive shares → Accept | `fixtures/CT03/case.qcore` | `checked 2 function(s); entry main; scoped owned-buffer tasks with counted read shares (Linux C11)`, exit 0 | parser, type_checker, reference_execution, native_backend **executed**; cfg, proof **unassessed** |
| **CT04** | parent writes while child retains a read share → Reject | `fixtures/CT04/case.qcore` | `case.qcore:14:13: Q600: LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2, 3] }`, exit 1 | parser, type_checker **executed** |
| **CT05** | two children each request FULL for the same memory → Reject | `fixtures/CT05/case.qcore` | `case.qcore:12:9: Q600: ShareExceedsAuthority { resource: 1, outstanding: 1024, requested: 1024 }`, exit 1 | parser, type_checker **executed** |
| **CT11** | full permission reconstructed while shared holders survive → Reject | `fixtures/CT11/case.qcore` | `case.qcore:16:13: Q600: LoanOutstanding { resource: 1, outstanding: 512, borrowers: [3] }`, exit 1 | parser, type_checker **executed** |

`cfg` is proposed `unassessed` for CT03 on purpose: this route emits from the
checked task IR and has no sequential CFG stage, so claiming `executed` would be
false. `native_backend` is `executed` for CT03 because the same program is one
of this card's sanitized native runs.

**CT11 carries a caveat the integrator should weigh.** The manifest wording is
"full permission reconstructed while local shared **aliases** survive". This
profile has no local aliases: the surviving share is held by a spawned child
that has not been joined. The refusal is the one the case is asking for — full
authority is not reconstructed while a share is retained — but the retaining
party is a task, not a local alias. If the integrator reads the case as
specifically about intra-frame aliasing, it should stay unmapped.

**Deliberately not proposed**, with the reason:

* **CT06** (same pointer copied into differently named variables → still same
  permission identity). This profile has no owned-pointer copy form at all:
  `let copy = buffer;` is refused with "owned buffers require explicit move or a
  checked buffer read". The closest measured fact — both children receive the
  same pointer for one allocation — is evidence for acceptance 3, not for the
  case as written, which needs a copy the grammar does not admit.
* **CT35** (one shared 4096-byte allocation read by two tasks → count 4096
  once). No byte-size accounting exists (§8). Mapping it would claim a size
  computation this profile does not perform.
* **CT24/CT25** (T04's own proposals, still in
  [`../remaining-t04/fixtures/`](../remaining-t04/fixtures/)). Unchanged by this
  card and still unmapped; T04's proposal stands as written.

---

## 10. Gate results

Run at this card's tip, on Linux, after `git submodule update --init --recursive`.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1160 passed, 0 failed** (exit 0) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **56 passed, 0 failed** |
| `python3 -B -m unittest discover -s scripts/tests` | **43 tests, OK** |
| `cd proof && lake build` | success, no warnings |
| `python3 -B scripts/check_lean_audits.py` | **Passed 24 Lean audits** |

Baseline at `c60fd91` was **1141** workspace tests, measured in this worktree
before any change. 1160 − 1141 = **19 new tests**, exactly the 6 + 9 + 4 added
here. Providers, Python tests and the two probes are unchanged in count.

**One pre-existing flake, reported rather than hidden.** On the first baseline
run, `qcore-verifier-host`'s
`a_confined_fixture_cannot_write_outside_the_admitted_paths` failed with
`Io`/`Terminated` (`Broken pipe`); it passed on every subsequent run, including
all post-change runs. This is P03's known load-flaky family
(`docs/reports/remaining-p03/README.md` §9.1) and no assertion was weakened.

### 10.1 Pinned artifacts

* **Not re-measured, and this is by construction rather than by measurement.**
  The four pinned 185-unit Cycle 19 artifacts and the configured CMetrics audit
  were not re-run: this card touches only the *source-language* compilation
  route inside `compile_files`. The C audit path (`qcore-audit` →
  `qcore-c-gateway`) does not call it, and no gateway, contract, profile,
  schema, policy or provider file was edited — the whole diff is nine
  source-language files, three test files, three Lean files and one grammar
  document.
* The contract corpus stays at **59 modules**, the `third_party/libevent`
  submodule pin is unchanged, and `docs/reevaluation/ct_manifest.json` is
  untouched at **15/50 mapped**.
* **No raw Cycle 19 unit is promoted**: 2 Safe / 42 provisional Unsafe / 137
  Unknown plus four data-only units, unchanged. `qcore-schemas` stays **5.0.0**;
  prose revision 0.3, plugin protocol v1, evidence v2 and program
  schema/profile v2 are unchanged.

---

## 11. Suggested status-ledger text — for the integrator

> Complementary read shares (T05): `ScopedReadSharesV1` counts the authority
> T04 lent. Each `read PLACE share S` capture transfers a positive share out of
> a per-resource budget of `FULL_READ_AUTHORITY` (1024, the concurrency model's
> unit), so one allocation may be lent to two borrowers whose shares sum to no
> more than the full authority; excess is refused as a whole with the
> outstanding total and the request named, a third simultaneous holder is
> refused by a declared bound, and exclusive authority — with it `drop`, `move`
> and the new checked `qcore_buffer_write` — returns only when the *last* share
> comes back. The budget is over the authority handed out: the owner's own read
> is retained across committed lending, and the separation-logic reading in
> which it is itself consumed is recorded open. Two T04 expectations were
> re-pinned deliberately, both keeping the refusal direction. Nineteen public
> Lean theorems in a new `QCore.ReadShare` namespace, no new axiom; 19 new
> tests; three fixed ASan/UBSan/LSan runs measure one allocation counted once
> across two live borrowers, a failed second creation returning only its own
> share, and a failed join returning none. ThreadSanitizer still does not
> intercept C11 `thrd_create`, so data-race freedom rests on the no-write-while-
> shared rule, not on measurement. CT03/CT04/CT05/CT11 mappings are proposed in
> the report only; CT stays 15/50. Disjoint slicing, write shares, more than two
> readers, several lent resources at once, byte accounting and nested workers
> remain refused. `qcore-schemas` stays 5.0.0 and no raw Cycle 19 unit is
> promoted. Evidence: `docs/reports/remaining-t05/README.md`.

---

## 12. What a follow-on can build on

* `ShareHolder` is the single place a third holder class would go; `MAX_READERS`
  is the single constant an *n*-reader profile would relax, and `lend`'s
  admission list is already the complete rule set.
* `ResourceState::Loaned { owner, shares }` would become
  `{ owner, retained, shares }` for the separation-logic reading of §2.1; the
  only rule that changes is `read_resource`'s owner arm, and the only fixture
  that changes meaning is T04's canonical `share FULL` program.
* `TaskInstructionKind::WriteBuffer` is the first mutation in this profile. A
  *write share* would split its gate the way `read_resource`/`use_resource`
  already split reads, rather than adding a new instruction.
* The Lean model's `lentTotal` and `shareOf` are total functions over a list, so
  an *n*-holder generalization is an induction rather than a rewrite; the
  two-reader theorems are the base cases.
