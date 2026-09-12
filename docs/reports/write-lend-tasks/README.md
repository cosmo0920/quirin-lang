# WRITE-LEND-TASKS — write lends crossing the spawn boundary (wave 31)

Card brief: the residual `SLICE-PERMISSIONS-WRITE-LEND` named for the half it
could not reach — *"both parties are one sequential entry's bindings — the F1
task-crossing remainder"* — and the direction CT10's rejection shape needs:
**two workers each holding write authority over disjoint byte ranges of one
buffer across the spawn boundary**.

Base: `origin/main` at `469f6509` (post-wave-30), isolated worktree, branch
`worktree-agent-a0f14b595495fba8a`.

**What landed, in one sentence.** A spawn capture `lend PLACE LOW..HIGH`
delivering an ungraded `lend<Buffer>` worker parameter — a **range write
lease**: write authority over the bytes `[LOW, HIGH)` of an owned Buffer's
payload, transferred to one worker for exactly spawn..join, decided disjoint
at check time by the wave-29 overlap rule (`ownership::axes_overlap`, called
rather than restated), enforced by the scoped-task ledger's new
`WriteLeased` custody state (the owner has neither write, read, release,
move nor further transfer while any lease is out), admitted by the
transferability judgment as its fourth row, and proved exclusive in Lean:
**no byte has two write authorities at any point of the interleaving**
(`QCore.WriteLendTasks.no_byte_has_two_write_authorities`, audited).

Zero new diagnostic codes (the Q-space icebox is untouched; two condition
widenings stated in spec §L21 and §4 below), zero new reserved words, zero
new tokens. Lean audits 121 → **122** (`AuditWriteLendTasks.lean`).

---

## 1. Scope: the bounded slice, stated before the evidence

This is the checked half of the card's goal, landed whole; the exercise half
is a named residual. Concretely:

**Landed.**

* Two workers *hold* write authority over disjoint byte ranges of one
  buffer's payload, across the spawn boundary, simultaneously — the
  authority genuinely transferred (the owner's exclusion is total and
  refusal-tested in every direction), decided disjoint at check time, and
  returned exactly at each creator join.
* All four refusals the card names: overlapping lends to two workers
  (`OverlappingLeases`, naming both ranges), the owner writing while a
  worker is live (`LeaseOutstanding` — the whole-payload write overlaps any
  nonempty lease), a release before the join (`LeaseOutstanding` through the
  unedited drop rule), and a varying bound
  (`WRITE-LEND-TASKS-VARYING-BOUND`, refused at the capture by its own
  spelling).
* The exclusivity theorem, over the interleaving: reachability over the
  lease ledger's admitted opens and closes models every point of every
  spawn/join schedule, and the write-authority count over any byte is at
  most one at every reachable state (§7).
* A native sanitized run of the two-lease program (ASan/UBSan, LSan on
  Linux): two threads created, both holding a lease of one heap object,
  both joined, the object allocated once and released once.

**Bounded, deliberately (each a named residual, §9).**

* **The worker exercises nothing** (`WRITE-LEND-TASKS-EXERCISE`): no
  operation in a worker's body admits a `lend<Buffer>` binding — not read,
  not write, not return, not drop. The authority is held and enforced
  against the owner, not yet exercised, so **no claim of two threads
  natively mutating disjoint bytes is made**. The native test's honest
  content is the crossing, the custody and the trace, not a mutation.
* **The byte axis is the payload's real four bytes**
  (`WRITE-LEND-TASKS-EXTENT`): the task route's Buffer payload is one
  `int32_t`, so `BUFFER_PAYLOAD_BYTES = 4` and a lease's bounds are decided
  against that extent — real memory, never a declared axis with nothing
  behind it. A byte-extent buffer (`bytes(N)` crossing a spawn) does not
  exist on this route.
* Exactly two simultaneous holders (`MAX_WRITERS = 2`, the write-side
  `MAX_READERS`); whole lifetime = spawn..join (only the creator's join
  reclaims); no re-lease (a lease source must be an owned buffer; workers
  are leaf functions, so a lease binding structurally cannot reach a spawn);
  no nested lease; at most one lease per creation, one write-leased resource
  at a time, and no mixing with read shares — each a represented refusal.
* Integer-literal bounds only (`WRITE-LEND-TASKS-VARYING-BOUND`): narrower
  than the loop-bound tier on purpose, because the disjointness of two
  workers' ranges must be a check-time decision, never a premise.
* The composed epoch route refuses the capture by name
  (`WRITE-LEND-TASKS-EPOCH`).

## 2. The construct

```qcore
import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {      // write authority over [0, 2)
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 2..4) {  // [2, 4) — both leases live
          Result::Ok { value: second } => {
            let a = join move first?;             // [0, 2) returns here
            let b = join move second?;            // the owner's authority is back
            let after = qcore_buffer_read(buffer);
            drop buffer;
            return after;
          }
          ...
```

The range spelling is the loop's (`LOW..HIGH`, the `..` being two existing
`Dot` tokens — no new token); the parameter type `lend<Buffer>` and the
capture word `lend` are both **contextual** (§10). The reference trace names
the crossing:

```
QCORE_BUFFER_LEASE 1 1 0 2        …        QCORE_BUFFER_LEASE_RETURN 1 1
QCORE_BUFFER_LEASE 2 1 2 4        …        QCORE_BUFFER_LEASE_RETURN 2 1
```

and the emitted C announces the same events behind the same typed logger the
read-loan lines use (`QCORE_DIAG_TASK_LEASE`, the byte range packed into the
event's value field; `QCORE_DIAG_TASK_LEASE_RETURN` at the join).

## 3. The custody shape: the rules live in the ledger, and every layer inherits them

The wave-29 card put the sequential rules in the ownership IR; the crossing's
rules live in the scoped-task ledger (`crates/qcore-core/src/scoped_tasks.rs`),
the one model where every other task custody rule already lives, and the
checker, the reference interpreter and the frontend all inherit them:

| Rule | Ledger decision | Refusal value |
|---|---|---|
| **Exclusive overlap between two lends** | a second lease of the same resource is admitted **exactly when its range is disjoint** from every outstanding lease — `LeaseRange::overlaps`, which **calls** `ownership::axes_overlap` (made `pub` by this card; the rule itself stays in the IR per amended constraint 15h) | `OverlappingLeases`, naming both ranges |
| **Owner authority while a lease is out** | `use_resource` and `read_resource` refuse for everyone — write, read, release, move, further lend/lease — because every such footprint is the whole payload and any nonempty lease overlaps it | `LeaseOutstanding`, naming ranges and holders |
| **Release before the join** | the **existing** drop rule through `use_resource`, no new rule beside it | `LeaseOutstanding` |
| **Whole lifetime = spawn..join** | a pending lease grants nobody anything (reservation window), rollback returns it exactly, commit rebinds it to the child, and **only the creator's successful join** reclaims it — the same transaction that collects the capsule | `LeaseNotOutstanding` on an unaccountable lease |
| **A third holder** | refused by count before any range is compared | `TooManyWriters` |
| **An empty range** | not a lease: it would carry no authority across | `InvalidLease` |
| **Read shares and write leases never coexist** | a share's whole-payload read footprint overlaps every nonempty lease, in both orders | `LoanOutstanding` / `LeaseOutstanding`, naming what is out |

**A deliberate divergence from wave 29, in the refusing direction:** the
sequential lend admits an empty range (it holds no byte and conflicts with
nothing); the crossing **refuses** one (`InvalidLease`) — a spawn capture
that transfers no authority is a vacuous crossing, and this route fails
closed rather than admitting it.

**The owner's read is excluded, unlike the read-share route and unlike the
sequential lend's out-of-range reads.** On the read-share route the owner
keeps its read because nothing can mutate; under the sequential lend the
owner reads *outside* the lent ranges because its footprint is per-byte.
Here the borrowers hold **write** authority and the owner's only read is the
whole payload — which overlaps every nonempty lease — so `qcore_buffer_read`
refuses while anything is out. Range-wise owner access is unspellable on
this route today; if a per-byte owner read ever exists, the wave-29 IR rule
(admitted outside every lend) is the rule to inherit.

## 4. Diagnostics: zero codes, two widened conditions

The `Q-NAMESPACE-DECISION` icebox forbids allocating a code; both landings
are condition widenings stated in spec §L21 (the `Q6xx` and `Q020` rows) on
the `CT47-COPY-CAPTURE` precedent.

| Refusal | Code | Widening |
|---|---|---|
| the judgment's refusals over the grown vocabulary: a write lend of `i32` (names the contract `i32` does carry), a lend-write delivered to `borrow<Buffer>`, a read share delivered to `lend<Buffer>` (both `ReceivedKindMismatch`) | `Q600` | **yes** — the CT47 boundary condition, over a 14 x 5 x 14 matrix |
| the boundary's check-time lease decisions: a varying bound, an empty range, a range past `BUFFER_PAYLOAD_BYTES` | `Q600` | **yes** — new check-time decisions at the crossing construct |
| every ledger lease refusal of §3's table, through the unedited `check_task_module` | `Q600` | **yes** — the scoped-task condition covers the lease-extended ledger |
| the `lend<Buffer>` parameter admission; lease not a result; not an entry parameter; place/parameter agreement over the new pair | `Q020` | **yes** — the task-parameter condition |
| a lease capture on the composed epoch route | `Q600` (composed reconciliation) | refused by name (`WRITE-LEND-TASKS-EPOCH`) |
| a malformed `lend` capture (no bounds, no `..`) | `Q000` | grammar, like every capture-shape failure |

**Deliberate pin updates, disclosed (constraint 5 is not engaged — nothing
weakened, every update strengthens or extends a message):**

* `crates/qcore-frontend/tests/transferability.rs`: the pinned
  `admitted_contracts` texts now name the third contract
  (`lend PLACE LOW..HIGH (a disjoint range write lease)`), and the pinned
  grammar-gate message reads `capture requires copy, move, read ... share,
  lend ... LOW..HIGH, or write`. The Q000/Q600/admitted **verdicts** of
  CT47's three measured shapes are unchanged.
* `crates/qcore-frontend/tests/read_loans.rs`: the task-parameter message
  gains `or an ungraded lend<Buffer>`.
* `crates/qcore-core/tests/unit/transfer.rs`: the matrices re-pinned whole
  at 14 x 5 (four admitting cells of 70) and 14 x 5 x 14 (four of 980);
  `write` still refused for every kind, verbatim.
* The `Q500` executable-subset catch-all stays the unreachable fail-closed
  backstop; its message text was widened to name the fourth admitted form.
  CT47's evidence about it ("kept as backstop") is intact; the text change
  is this card's and is disclosed here.

## 5. The judgment: a fourth row, with `write` untouched

`qcore_core::transfer` grows by one kind (`WriteLease`, never itself
transferable — the product of a crossing, like `ReadLoan`), one mode
(`LendWrite`, spelled `lend PLACE LOW..HIGH`), one protocol
(`DisjointWriteLeases` — sharing made safe by **partitioning the bytes**,
where read shares remove mutation and a mutex would order it), and one
admitted transfer (`LendWriteRange`, delivering `WriteLease`).
`TransferClass::MovedOnly` gains `mutably_shareable_under` beside
`shareable_under`, so the read-sharing and write-sharing disciplines are
separate questions a consumer must ask separately.

`TransferMode::Write` is **deliberately not** the new mode: the plain
`write PLACE` capture still parses, still has no lowering, and is still
refused by name for every kind — `aWriteCaptureIsRefusedByName` is
re-decided over the grown matrices, so CT47's pinned refusal survives
byte-for-byte in verdict. The frontend's CT46 agreement check (judgment vs
enumeration, disagreement refuses) covers the new row unchanged.

## 6. Compatibility, measured

* Every neighbouring task suite runs unmoved: `source_tasks` 6,
  `scoped_read_loans` 14, `scoped_read_shares` 4, `owned_source_tasks` 4,
  `error_payload_tasks` 11, `read_loans` 10, `transferability` 9 (three
  message pins updated deliberately, §4), `task_recovery`,
  `epoch_tasks_source`, `obs_reordering`, `task_io_compose`,
  `thread_examples` — all green in the workspace run (§11).
* A module with no lease travels identical code paths: the lowering's new
  arms are behind the new capture/type spellings, the ledger's new state is
  reachable only through `reserve_with_transfers`'s lease list
  (`reserve_with_loans` delegates with an empty one), and the emitted C of
  every pre-existing program is byte-identical **except** for the shared
  diagnostic prelude, which gains two enum members and two adapter cases
  (`QCORE_DIAG_TASK_LEASE`, `QCORE_DIAG_TASK_LEASE_RETURN`). No pre-existing
  program's **output** changes by a byte: the new kinds are logged by no old
  program, and the examples suite (byte-comparing every published program's
  execution) runs green.
* The reference interpreter and the native runtime announce the same lease
  events at the same points (creation commit; successful join), so the
  trace vocabulary stays one vocabulary. The F02 trace relation refuses the
  new events exactly as it refuses the T05 write — outside its pre-T05
  alphabet, represented, never silently dropped.
* A new `SourceExecutionProfile::ScopedWriteLeasesV1` names the profile —
  a strictly stronger claim than owned transfer, on the read-share
  precedent. `qcore-schemas` is untouched (the profile enum lives in
  `qcore-verification`), so constraint 18's provider rebuild is not owed.

## 7. Lean (invariant 6)

`proof/QCore/WriteLendTasks.lean` (registered in `proof/QCore.lean`; in the
whole-project axiom closure) and `proof/AuditWriteLendTasks.lean` — **17
declarations audited by name**, placeholder-free, no project axiom, every
closure within `{propext, Classical.choice, Quot.sound}`. Lean audits
121 → **122**.

* **The exclusivity theorem the card names**:
  `no_byte_has_two_write_authorities` — for every ledger state reachable by
  admitted opens and closes (**any point of any spawn/join interleaving**,
  by construction of the `Reachable` relation) and every byte,
  `writeAuthorities s b ≤ 1`, where the count is the outstanding leases
  holding the byte **plus the owner exactly when nothing is outstanding**
  — so both directions (holder vs holder, owner vs holder) are one
  statement. Proved from `reachable_states_are_pairwise_disjoint` (the
  invariant survives every interleaving) and
  `at_most_one_lease_holds_a_byte` (pairwise disjointness forbids a shared
  byte, via the wave-29 `overlaps_iff_a_shared_byte`).
* **The admission is the disjointness question** —
  `a_second_lease_is_admitted_iff_disjoint` (an equation with `!overlaps`,
  the wave-29 decision imported), `two_admitted_leases_share_no_byte`,
  `an_empty_range_is_not_a_lease`, `a_third_simultaneous_lease_is_refused`.
* **The return at the join, exactly** —
  `the_owner_is_suspended_exactly_while_a_lease_is_out` (an iff),
  `closing_the_last_lease_restores_the_owner`,
  `a_join_never_disturbs_a_siblings_lease`,
  `a_closed_holder_retains_nothing`.
* **The judgment's fourth row is backed** — `QCore.Transferability` grows
  with the Rust judgment (kind, mode, protocol, delivered transfer; both
  matrices re-decided whole at 14 x 5 and 14 x 5 x 14), and the new
  admitting arm carries `LeaseObligation`, discharged by
  `WriteLendTasks.no_byte_has_two_write_authorities` — the same
  record-which-rule-answers discipline as the move and read-lend arms.
  `exactlyOneKindIsMutablyShareable` pins that the mutable-sharing
  direction separates exactly the Buffer, under exactly the
  byte-partitioning protocol; `noModeRequestsTheMutexProtocol` now covers
  both sharing directions and both lend families.

Not proved, carried as the Rust side's boundary: the token-to-bounds
extraction (tested), the reservation/commit/rollback plumbing (tested
against the ledger), any C-memory-model or native-schedule statement, and
the holder's exercise of its authority (nothing to prove: no operation
exists, `WRITE-LEND-TASKS-EXERCISE`).

## 8. CT07 / CT10 readiness — exactly what a lane can now claim

**This card maps nothing and touches no fixture under
`compiler/tests/fixtures/ct/`** (a parallel lane owns that tree);
`docs/reevaluation/ct_manifest.json` and the acceptance matrix are
untouched. What it supplies:

* **CT07** — mapped since `CT07-WRITE-REMAP` (wave 30) on the sequential
  write-lend program, whose recorded boundary is exactly this card's
  subject: *both parties are one sequential entry's bindings, not two
  threads*. That boundary sentence is now **dischargeable**: the same
  two-disjoint-write-authorities scenario exists across a spawn boundary —
  two workers, one buffer, `[0, 2)`/`[2, 4)`, both live at once, the owner
  excluded during and restored after, `Q600`-refused on one shared byte —
  with the exclusivity statement audited over the interleaving rather than
  over a statement sequence. The honest remaining caveat a lane must carry:
  the workers **hold** and do not yet **exercise** the authority
  (`WRITE-LEND-TASKS-EXERCISE`), so "mutated independently *by the
  threads*" is not yet executed evidence; the sequential program remains
  the mutation evidence, and this card adds the crossing.
* **CT10** — *"Adjacent bit-fields in one memory location → Reject
  independent concurrent writes."* The wave-29 readiness statement said the
  missing halves were **concurrency** and **sub-byte granularity**. The
  concurrency half now exists: two independent write authorities over one
  location, requested **concurrently** (two live workers), refuse at `Q600`
  when they share a byte — `overlapping_leases_to_two_workers_refuse_naming_both_ranges`
  is CT10's rejection shape at byte granularity across a real spawn
  boundary. The sub-byte half stands where wave 29 left it: two bit-fields
  of one byte are one memory location, this route's atom is the byte, so
  the conservative encoding (both bit-fields lease the same byte → refused,
  the right answer) is an encoding a lane would have to declare in its
  profile. A lane mapping CT10 on this card's shape should declare exactly
  that byte-granularity encoding, or wait on the disjoint-footprint
  checker's `SharedBitfieldUnit` (`Q661`) for the literal construct.

## 9. Named follow-ons

| Name | What is missing | Discharge condition |
|---|---|---|
| `WRITE-LEND-TASKS-EXERCISE` | the worker exercises its lease — no operation admits a `lend<Buffer>` binding in a worker body, so no two threads natively write disjoint bytes yet | a worker-side range write statement (the lease's own coordinates, rebased as the wave-29 IR rebases), its checker rule (inside the lease's own `[0, high-low)`), its emission (byte stores through `unsigned char *` into the payload — race-free under C11 because disjoint bytes are distinct memory locations), and a sanitized native run byte-compared against the reference |
| `WRITE-LEND-TASKS-EXTENT` | the byte axis is the payload's four bytes (`BUFFER_PAYLOAD_BYTES`) | a byte-extent task buffer (`bytes(N)` crossing a spawn), or the composed buffer-route worker body `DISJOINT_FOOTPRINT_PROFILE.md` §8 F1 names |
| `WRITE-LEND-TASKS-VARYING-BOUND` | a lease bound that is a name, even one bound to a check-time constant | the loop-bound tier (literal `let`s), or the relational domain `SLICE-PERMISSIONS-VARYING-RANGE` names |
| `WRITE-LEND-TASKS-EPOCH` | a lease capture on the composed epoch route, refused by name | a measured residency rule for a leased range against the epoch's allocation axis |
| `WRITE-LEND-TASKS-F1` | the interval lives in the scoped-task ledger, not yet in `DisjointFootprintsV1`'s derived footprints | the F1 wiring: the capture's range as a `DeriveSlice` onto that checker, discharging its `DeclaredFootprintShape` premise for this shape |
| `WRITE-LEND-TASKS-WINDOWS-RUN` | the native test ran the Linux arm in this worktree; macOS compiles the same test file | the `STRING-CONCAT-WINDOWS-RUN` precedent for the task emission under MSVC |

## 10. Keyword census (standing constraint 14)

**+0 reserved words, +0 tokens, two new contextual positions for the
already-contextual `lend`.** Archived at `measure/keyword_census_lend.txt`
with the file list; reproduction: `grep -Hn lend` over every `.qcore`/`.qc`
file outside `third_party/` (487 files).

* The bare word `lend` occurs in this repository only as the buffer route's
  `lend(` call shape (wave 29's own claim, unchanged), in comments, and in
  this card's own fixtures. It occurs **zero** times as the first word of a
  spawn capture and **zero** times before `<` — both positions were parse
  failures before this card, so claiming them collides with nothing.
* The reference-collection exclusion is **positive**, on the
  `SOCKET-INET-ADDRESSES` discipline: `lend` is skipped only where the
  statement spells `spawn` before it, the word stands immediately after `(`
  or `,`, and a name follows — an unrecognised `lend` is still collected,
  and the place name after it is a genuine reference that still resolves.
  `let lend = 5; print dec lend;` remains a program (wave 29's pin,
  untouched).
* The range spelling reuses `LOOP-V1`'s `..` (two `Dot` tokens); no lexer
  rule changed.

## 11. Gate

Run in this worktree on `origin/main` at `469f6509` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row and the submodule initialised
before the workspace run.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | see the final report row — recorded verbatim at close |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **Passed 122 Lean audits** (121 → 122: `AuditWriteLendTasks.lean`), no warning, exit 0 |
| `python3 -m unittest discover -s scripts/tests` | 200 tests, **1 expected failure** — §11.1 |
| `compiler/tests/write_lend_tasks.rs` | **17 passed, 0 failed** (16 checked + 1 sanitized native run) |
| `crates/qcore-core --lib` (incl. 4 new ledger rows, re-pinned transfer matrices) | all passed |
| `crates/qcore-frontend` tests (incl. `transferability`, `read_loans`) | all passed |

### 11.1 One expected Python failure, left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` reports
`lean_audits: 121 != 122` — the derived-value tripwire working as designed:
this card adds one audited Lean module and may not edit the acceptance
matrix. The same one-line reconciliation every wave's new audit file
receives (constraint 25). The other 199 Python tests pass.

### 11.2 Measurement hygiene

Census and measurement outputs are worktree-private under `measure/`
(gitignored), never under `target/` and never in the shared scratchpad.

## 12. Integrator reconciliation list

This card may not edit `AGENTS.md`, `README.md` or the acceptance matrix.

1. **Proposed amendment to standing constraint 15h** — one sentence appended
   (the card supersedes nothing in 15h; it discharges the *sibling* residual
   the wave-29 card listed):

   > **15h (appended, card `WRITE-LEND-TASKS`).** The write lend now also
   > crosses the spawn boundary, as a **range write lease**:
   > `lend PLACE LOW..HIGH` in a spawn capture transfers write authority
   > over `[LOW, HIGH)` of an owned Buffer's payload
   > (`BUFFER_PAYLOAD_BYTES = 4`, the `int32_t`'s real bytes) to one worker
   > for exactly spawn..join, decided by the **lease-extended scoped-task
   > ledger** (`ResourceState::WriteLeased`) rather than by a route rule.
   > The disjointness decision is `ownership::axes_overlap` — made `pub`
   > and **called** by `scoped_tasks::LeaseRange::overlaps`; do not
   > re-implement it in the ledger or move it out of the IR. Three rules
   > must survive refactors: the owner holds **no** whole-payload authority
   > — read included — while any lease is out (stricter than both the
   > read-share route and the sequential lend's per-byte exclusion, because
   > the owner's only spellable footprint here is the whole payload); an
   > **empty** lease refuses (`InvalidLease`) where the sequential empty
   > lend admits — the crossing of no authority is vacuous and fails
   > closed; and only the creator's join reclaims a lease, in the same
   > transaction that collects the capsule. The judgment's mode is
   > `LendWrite`, deliberately distinct from the still-refused `write`
   > capture (`aWriteCaptureIsRefusedByName` re-decided over the grown
   > 14 x 5 matrices). No operation admits a lease in a worker body
   > (`WRITE-LEND-TASKS-EXERCISE`); do not add one without landing its
   > checker rule, emission and sanitized native comparison together.
   > `write-lend-tasks`.

2. **`AGENTS.md`**: the `Q020`/`Q021` and `Q600` namespace rows should
   record the two condition widenings (authority: spec §L21, updated by
   this card); the headline Lean-audit number moves 121 → 122; the ledger's
   standing-constraint inventory gains the 15h append above if adopted.

3. **The derived `lean_audits` count** moves by one — §11.1's tripwire, the
   usual one-line matrix reconciliation (constraint 25).

4. **Public API changes** (workspace-internal; `qcore-schemas` untouched):
   * `qcore_core::ownership::axes_overlap` is now `pub`.
   * `qcore_core::scoped_tasks`: `LeaseRange`, `MAX_WRITERS`,
     `ResourceState::WriteLeased`, `reserve_with_transfers`,
     `leased_ranges`, `lease_holders`, `held_lease`,
     `outstanding_write_leases`; `TaskLedgerError` gains `LeaseOutstanding`,
     `OverlappingLeases`, `InvalidLease`, `TooManyWriters`,
     `LeaseNotOutstanding`. A downstream exhaustive match on
     `ResourceState` or `TaskLedgerError` needs arms.
   * `qcore_core::task_execution`: `ScalarType::WriteLease`,
     `ScalarOperand::Lease`, `BUFFER_PAYLOAD_BYTES`; `TaskEvent` gains
     `BufferLeased`/`BufferLeaseReturned`; re-exports `LeaseRange`,
     `MAX_WRITERS`.
   * `qcore_core::transfer`: `TransferKind::WriteLease`,
     `TransferMode::LendWrite`, `ShareProtocol::DisjointWriteLeases`,
     `AdmittedTransfer::LendWriteRange`;
     `TransferClass::MovedOnly` gains `mutably_shareable_under` (a
     **breaking** pattern change for exhaustive matches) and the class
     gains `mutably_shareable_under()`.
   * `qcore_frontend::frontend`: `SourceCaptureMode::LendWrite`,
     `SourceLeaseBound`.
   * `qcore_verification`: `SourceExecutionProfile::ScopedWriteLeasesV1`.
   * The emitted diagnostic prelude gains two enum members and two adapter
     cases; `task_trace_relation` refuses the two new trace lines as
     outside its pre-T05 alphabet.

5. **CT07's mapping and CT10's reason row** may be updated per §8 by a
   lane, never by this card.

## 13. Artifacts

- `measure/keyword_census_lend.txt` (worktree-private) — the constraint-14
  census over 487 fixture files; reproduction command in §10.
- The flagship fixture and its refusal controls live inline in
  `compiler/tests/write_lend_tasks.rs` (17 tests), deliberately not under
  `compiler/tests/fixtures/ct/` — that tree is a parallel lane's.
