# `WRITE-LEND-TASKS-EXERCISE` — the worker exercises its range write lease

**Wave 36. Language side. Card closed: all three legs landed together, the
fourth (the F02 alphabet question) decided.**

## Headline

Card `WRITE-LEND-TASKS` (wave 31) transferred a range write lease across the
spawn boundary and enforced it, and said so in the sentence this card is named
after:

> **No operation admits a lease in a worker body** — the authority is held and
> enforced, not exercised, which is a measured boundary and not a claim about
> two threads mutating (`WRITE-LEND-TASKS-EXERCISE`); do not add one without
> landing its checker rule, emission and sanitized native comparison together.

All three are landed here, in one card. A worker body now writes and reads
single bytes through its lease; the emitted C writes real bytes of the shared
payload; and two threads writing distinct bytes of one `int32_t` run under
AddressSanitizer, UndefinedBehaviorSanitizer and LeakSanitizer with the native
stdout compared **byte for byte** against the reference execution's own
rendering. The exclusivity statement wave 31 proved about *authorities* is
re-proved about the **writes themselves**.

`TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` is **decided** — the F02 observable
alphabet does **not** grow — on a ground the exercise itself supplies, and the
decision is stated with its ground in §6.

Two things this card measured that it did not set out to measure, both
recorded rather than absorbed: **ThreadSanitizer cannot run any program this
route emits**, lease or not (§5.4), and **a stray byte one past a lease lands
in the buffer struct's own tail padding**, where no memory tool can see it
(§5.5) — which is the measured reason the emitted helper carries its own extent
guard instead of delegating the decision to a sanitizer.

**+0 reserved words, +0 tokens, +0 `Q` codes.**

## 1. Charter and scope

Land, together or not at all: (1) a checker rule for a worker-body write
through its lease, decided in the ledger/IR rather than by a route rule; (2)
emission of that write into the shared payload; (3) a sanitized native
comparison of two workers writing disjoint ranges; and (4) an explicit decision
on the forward F02 alphabet question. Withdraw and publish the measurement if
the three cannot be landed soundly together.

They could. Nothing is withdrawn.

**Out of scope, and stated so rather than left silent:** the gateway; any
Windows or Apple run (`WRITE-LEND-EXERCISE-WINDOWS-RUN`, on the
`WRITE-LEND-TASKS-WINDOWS-RUN` precedent — the `windows_c` path is carried
unchanged and no such run has happened); and re-mapping CT07, which this card
deliberately does not touch (§10).

## 2. What existed before the card

* `scoped_tasks::LeaseRange`, `ResourceState::WriteLeased`, disjointness by
  `ownership::axes_overlap` **called** by `LeaseRange::overlaps`. Untouched.
* `ScalarType::WriteLease`, whose own doc said it "admits **no operation in the
  worker's body**"; `AbstractValue::Lease(ResourceId)`, which was not even
  *readable*; `State::buffer(slot)`, which accepted only `AbstractValue::Owned`,
  so `WriteBuffer` was structurally unreachable from a lease slot.
* `TaskEvent::BufferLeased` / `BufferLeaseReturned`, refused by
  `reference_trace` (constraint 15n, wave 32).
* `BUFFER_PAYLOAD_BYTES = 4`: the real bytes of the emitted
  `struct qcore_buffer`'s `int32_t payload`.

The lease was therefore a permission nobody could use. Wave 31's exclusivity
theorem was true and about nothing that happened.

## 3. Leg 1 — the checker rule

### 3.1 The shape of the decision, and why it is split

A holder names a byte **relative to its own lease**; the byte it reaches is
`low + index`. Which relative indices may it name?

The two halves of that question are known in two different places, and the rule
is split accordingly:

* **The worker's body** states the exclusive bound it touches through its
  lease. A body is checked once, independently of any call site
  (`check_function` binds parameters from declared types alone), so it cannot
  know where its lease sits. It reports `FunctionSummary.lease_footprint`.
* **The creator's spawn site** is the only place the lease's real
  `[low, high)` exists. It decides `bound ≤ high - low` and refuses otherwise.

This is not an approximation of checking each write. It is the same statement,
and that is proved rather than argued:
`QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law`
says the one arithmetic comparison holds **iff** every relative index below the
bound is contained in the lease. That theorem is what licenses the summary at
all. Do not move the fit into the worker (it cannot be decided there) or the
footprint into the creator (it is a property of the body).

### 3.2 The IR

Two instructions, in `crates/qcore-core/src/task_execution.rs`:

* `WriteBufferByte { authority, index, value }` — lease only.
* `ReadBufferByte { authority, index, value_slot }` — lease (relative to its
  `low`) or owner (relative to 0, so absolute).

`index` is a `ScalarOperand`, so the **constant-index rule lives in the IR**
and is testable there: `constant_index` admits `ScalarOperand::I32(k)` with
`k >= 0` and refuses everything else, after resolving the slot so that a stale
or mistyped operand still reports what it always reported. An index the route
cannot resolve is **not provably inside** its authority — the sequential lend's
`WRITE-LEND-COMPUTED-INDEX` arm, crossing the spawn boundary.

`State::authority` classifies a slot as `Owner`, `Lease` or `Loan`, keeping the
original owned-buffer refusal verbatim for everything else.

### 3.3 The four rules that must survive refactors

1. **The owner gains no byte write.** The whole-payload write stays its only
   write footprint. This is deliberate and load-bearing: wave 31's rule that
   *the owner holds no whole-payload authority, read included, while any lease
   is out* rests on the owner's only spellable footprint being the whole
   payload. Admitting an owner byte write would introduce a second owner-side
   footprint that rule was never measured against.
   `WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`.
2. **A read share admits no byte view.** `WRITE-LEND-EXERCISE-LOAN-BYTE-READ`.
3. **The owner's byte read takes the same gate its whole-payload read takes** —
   `State::readable`, which calls `ledger.read_resource` — so a lease
   outstanding refuses it by the existing rule rather than by a second one
   written here.
4. **A worker takes at most one write lease.** A creation supplies at most one,
   so a second lease parameter could never be filled and the footprint summary
   could not say which lease it belongs to. Refusing makes the summary's
   single-authority reading a fact rather than a convention.

### 3.4 The spelling: +0 words, +0 tokens, censused

The **arity** selects the form, on card `CT07-SLICE-PERMISSIONS`' `borrow_utf8`
precedent (arity one or three):

* `qcore_buffer_write(LEASE, INDEX, VALUE)` — arity 3, a byte write.
* `qcore_buffer_read(NAME, INDEX)` — arity 2, a byte read.

Both names are already reserved route selectors; both new arities were parse or
type refusals before this card. Census, run rather than assumed, over **514**
`.qcore`/`.qc` files:

| Spelling | occurrences | at the new arity |
|---|---|---|
| `qcore_buffer_write` | 23 | **0** |
| `qcore_buffer_read` | 139 | **0** |

The census is a **pre-card** measurement, taken at base `0a5c2ecc`. Before this
card the only arity-3 `qcore_buffer_write` and arity-2 `qcore_buffer_read`
anywhere in the tree were the **emitted C helpers'** own signatures — three and
two hits in Rust string literals, a different language and a different
function, and the byte helpers take distinct names
(`qcore_buffer_byte_write`, `qcore_buffer_byte_read`), so there is no collision
even there. The card's own fixtures add QCore-source occurrences afterwards,
which is the point of taking the census first. **No shipped program's meaning
changes**: every occurrence in the 514 fixtures is at an arity this card does
not touch.

Reproduction:

```sh
find . -path ./third_party -prune -o \( -name '*.qcore' -o -name '*.qc' \) -print | wc -l
grep -rn 'qcore_buffer_write([^)]*,[^)]*,' --include='*.qcore' --include='*.qc' .
grep -rn 'qcore_buffer_read([^)]*,'        --include='*.qcore' --include='*.qc' .
```

## 4. Leg 2 — the emission

The emitted worker writes

```c
((unsigned char *)&buffer->payload)[index] = byte;
```

— one object, not a read-modify-write of the payload. That distinction is the
whole soundness argument: two threads writing distinct bytes of one `int32_t`
are writing **distinct memory locations** under C11 6.2.6.1/4 and 3.14 and are
not in a data race, whereas a value-level `payload = (payload & ~mask) | v`
from two threads plainly would be. A byte-order-normalising emission was
considered and **rejected for exactly that reason**, not for cost.

The lease's **base** travels in the task value's otherwise-unused `scalar`
field — `(struct qcore_task_value){INT32_C(low), s_N.value.buffer}` — because
a lease-relative index means nothing without the base it counts from, and the
base is the one fact the holder must not be able to choose. An **owner**'s base
is emitted as the literal `UINT64_C(0)`: the emitter computes the function's
lease-parameter slot set, so the two authorities cannot be confused. (A lease
is only ever a parameter — no operand rebinds one and there is no transitive
leasing — so that set is exact.)

The emitted helpers re-decide the extent (`index >= sizeof buffer->payload`
aborts) and log through the worker-private event array, so a worker's byte
events are replayed by the parent at collect exactly as every other buffer
event is (constraint 15a).

**What moved in previously emitted programs.** Two new diagnostic kinds at the
end of `enum qcore_diagnostic_kind` with their two `case`s, the two new helpers,
and their two entries in the emitter's explicit "used" list — the
wave-31 precedent, which added the two lease kinds the same way. Non-task
routes emit a different preamble and are untouched. Programs that carry a lease
additionally change where the creator constructs the lease's task value, since
it now carries the base. The used-list entry was **not** foreseen: `-Wall
-Wextra -Werror` rejected the unused statics, and the card's own lease-free
companion test caught it (constraint 15k's recorded hazard, firing again).

## 5. Leg 3 — the sanitized native comparison

### 5.1 The flagship

One allocation with payload `0`; two children with disjoint write leases over
the payload's two-byte halves; **each writing a byte through its own lease**;
the owner reading those bytes back after both joins.

The reference trace, pinned in
`each_worker_writes_its_own_rebased_byte_and_the_owner_reads_them_back`, shows
the rebasing law rather than asserting it — both workers write their *relative*
byte `0`, and the events name owner bytes `0` and **`2`**:

```
QCORE_BUFFER_LEASE 1 1 0 2
QCORE_BUFFER_LEASE 2 1 2 4
QCORE_BUFFER_BYTE_WRITE 1 1 0 9
...
QCORE_BUFFER_BYTE_WRITE 2 1 2 9
QCORE_BUFFER_BYTE_READ 0 1 0 9
QCORE_BUFFER_BYTE_READ 0 1 2 9
```

### 5.2 The run

`two_threads_write_disjoint_bytes_of_one_object_sanitized`: clang, `-std=c11
-pthread -Wall -Wextra -Werror -g -O0 -fno-omit-frame-pointer
-fsanitize=address,undefined`, `detect_leaks=1:halt_on_error=1`. Native stdout
is **byte-identical** to the reference execution's own rendering, native stderr
likewise, and exactly one heap object is allocated and released once.

The comparison is the **strict** one, and that is checked rather than assumed:
every source line in the flagship is the parent's, so the certificate's
`admits_reordering` is `false` and constraint 15a's byte comparison applies
(asserted in the test). Linux arm executed; the same test file compiles on
macOS (the Apple threads shim include is wired), and **no Windows or Apple run
is claimed** — `WRITE-LEND-EXERCISE-WINDOWS-RUN`.

### 5.3 Representation independence, as a guarantee

The reference models the payload's own object representation
(`i32::to_ne_bytes`), so reference and emitted C agree by construction on any
host. The **published** program is additionally free of any dependence on that
representation, and that is a guarantee rather than luck: C11 7.20.1.1 makes
`int32_t` two's complement with no padding bits, so `0` has an all-zero
representation on every conforming host, and a byte written at index `i` and
read back at index `i` is the same byte whatever the order of the four. A
program that mixes whole-payload and byte access *is* host-dependent; that is
`WRITE-LEND-EXERCISE-BYTE-ORDER`, admitted with its boundary stated rather than
refused.

### 5.4 The ThreadSanitizer row — measured, and the measurement is that there is none

The charter asked for TSan "measured rather than assumed". Measured:

| program | lease | worker byte write | TSan |
|---|---|---|---|
| this card's flagship | yes | yes | SEGV at `0x18`, DEADLYSIGNAL |
| wave 31's `held_only` | yes | no | SEGV at `0x18`, same pc |
| `examples/threads/two_workers.qcore` | **no** | no | SEGV at `0x18`, same pc |
| a minimal `thrd_create`/`thrd_join` program | — | — | **runs clean** |

ThreadSanitizer cannot run **any** program this route emits on this toolchain
(Ubuntu clang 21.1.8), including a published example that holds no lease and
writes no byte, at the identical faulting address and pc. The failure is
therefore the **route's** and nothing here is attributable to the exercise. The
finding is registered as `TASK-ROUTE-TSAN-SEGV` and pinned by
`thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`, which asserts
the symmetry in both directions — and which would **fail loudly** if a future
toolchain ran the route but refused the exercised program, since that asymmetry
*would* be a finding about this card.

So the C11 3.14 reading is carried by the helper's own comment and by the
sanitizer/allocation evidence, and is **not** backed by a TSan row. Said in
those words rather than elided.

### 5.5 The past-range falsification, in three steps — and a finding

`a_write_one_byte_past_the_range_is_caught_and_we_say_by_what` mutates the
emitted C, since the checker refuses the program:

1. Move the worker's write to `base + 4`, guard intact → the process dies with
   **SIGABRT**: the emitted helper's own extent re-decision, before any byte
   reaches memory and before AddressSanitizer is needed.
2. Remove the guard as well, still `+4` → the program runs **clean** and ASan
   is **silent**. This is not a sanitizer failure. `struct qcore_buffer` is
   `uint64_t` then `int32_t`, so it is 16 bytes and byte 4 of the payload is
   inside the struct's own **tail padding**, which the allocation really does
   own.
3. Remove the guard and move to `base + 64` → **ASan names the
   heap-buffer-overflow**.

Step 2 is the finding, and it is why the emitted helper carries its own guard
rather than delegating to a sanitizer: **one byte past a lease is, in general,
one byte the allocation still owns, and no memory tool will ever see it.** Any
future change that removes that guard on the grounds that "ASan would catch it"
is wrong, and this test says so.

## 6. Leg 4 — the F02 alphabet decision

**`TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` is decided: the observable alphabet
does not grow.** The two byte events join the lease events on the **refusing**
side of `reference_trace`.

### 6.1 The ground

`AdmittedCausalOrder` is a **sequence**. To admit a run it must state one
causal order over that run's events. Two workers holding disjoint write leases
write their bytes with **no causal edge between them** and no synchronization
until their joins — that is precisely what the leases buy. Any order the
alphabet could carry would therefore assert an ordering the run does not have.

That is the situation constraint 15a answers with a **window-and-emitter**
relation (`qcore.obs-reordering.v1`), and it is the opposite of F02's premise.
So growing the alphabet is not blocked by missing work; it is blocked by the
shape of the thing being grown. Re-opening it needs CT03's method — a
conservativity theorem plus a rule shown non-vacuous in both directions —
**and** a causal-order semantics for concurrent byte writes, which is the half
the sequence shape denies.

### 6.2 Reachability is deliberately *not* the ground, and that is measured

There is a tempting cheaper argument: a byte write is only reachable through a
lease, so the lease event always refuses first and growing the alphabet by the
byte events alone would decide nothing. That is **true** — measured by
`a_real_exercised_program_refuses_at_its_lease_before_its_byte_write`, which
compiles a real exercised program and checks the lease event's index is lower —
but it is an argument about what is currently reachable, and a later card that
made a lease-free byte write expressible would silently destroy it. The
decision rests on §6.1 instead, and the reachability fact is published beside
it as a measurement rather than as the reason.

### 6.3 The arm is driven, not merely written

`the_byte_events_are_outside_the_f02_alphabet` drives the refusal on a
**synthetic** observation carrying a byte event and no lease event, so the arm
is exercised rather than shadowed by the lease refusal that would always
precede it in a real run — and it checks that the refusal's index is the
offending event's own position, wave 32's
`refusal_points_at_a_refused_event` holding for these constructors too. The
byte **read** is driven separately, on its own footing. Mutation M6 (encode the
byte events as `ObservedEvent::Written` instead of refusing) is **caught**.

### 6.4 The operative consequence

Unchanged from wave 32, but now a decision rather than a pending question:
write-lend programs sit outside F02 relational comparison **by construction**,
their native evidence being sanitizer, allocation-count and the byte-identical
stdout of a strict comparison whose windows are the joins.

## 7. Lean

`proof/QCore/WriteLendTasksExercise.lean`, registered in `proof/QCore.lean` and
audited by `proof/AuditWriteLendTasksExercise.lean`. **133 Lean audits pass**,
mine among them, with only `propext`, `Classical.choice` and `Quot.sound`.

| Theorem | What it settles |
|---|---|
| `admits_index_iff_below_extent` | A rebased index is inside its lease **iff** it is below the lease's extent — the lease's own `low` cancels, which is why a worker can be checked without knowing where its lease sits |
| `the_footprint_fit_is_exactly_the_rebasing_law` | The spawn site's one comparison is **equivalent** to the whole family of per-index containment facts; this is what licenses the summary |
| `some_index_is_always_outside` | A nonempty lease always has indices it does not admit, so refusing an unresolvable index is not vacuous |
| `an_admitted_write_lies_inside_its_writers_lease` | The Rust rule, as a property of the write rather than of the index |
| `no_byte_is_written_by_two_parties` | **Exclusivity about the writes themselves**: in any reachable ledger state, two admitted writes to one byte are the same writer's |
| `the_owner_writes_no_byte_while_any_lease_is_out` | The whole-payload suspension, restated about writes so the theorem above covers every party |
| `the_added_operations_are_exactly_two` | The added surface as one list, so a widening changes a theorem rather than slipping past |

**Deliberately not proved, and said so in the module's own header:** that the
emitted `((unsigned char *)&buffer->payload)[i]` is a distinct memory location
under C11 3.14 (a reading of the standard, carried by the helper's comment and
exhibited by §5.2), anything about a native schedule, and the reference run's
agreement with the emitted C (measured, not proved).

## 8. Mutations — every new arm shown non-vacuous

`private/mutate.py` (worktree-private), 11 mutations, each restoring the file
with its mtime advanced past the mutation's
(`MUTATION-HARNESS-MTIME-RESTORE`).

| # | Mutation | Result |
|---|---|---|
| M1 | the spawn-site footprint fit never refuses | CAUGHT |
| M2 | a non-constant byte index is admitted as `0` | CAUGHT |
| M3 | the worker's lease base is ignored (every holder writes byte 0) | CAUGHT (both the emission pin and the native comparison) |
| M4 | the frontend admits an owner byte write | CAUGHT |
| M5 | the **IR** arm refusing an owner byte write | CAUGHT |
| M5b | the **IR** arm refusing a read-share byte view | CAUGHT |
| M5c | the **IR** at-most-one-lease-parameter rule | CAUGHT |
| M5d | the **frontend** arm refusing a read share at a byte position | **SURVIVES, as recorded** |
| M6 | the F02 alphabet grows by the byte events instead of refusing | CAUGHT |
| M7 | the owner's byte read skips the whole-payload authority gate | CAUGHT |
| M8 | the emitted byte helper loses its own extent guard | CAUGHT |

**M5d's survival is the result, not a gap.** Breaking the frontend's read-share
refusal leaves the **IR** refusing with the same sentence, which is constraint
15h's standing instruction — *decided in the IR, not by a route rule* — made
visible. The harness records the expected survival explicitly and **fails** if
that mutation is ever caught, so the defence-in-depth claim cannot quietly
become a single point of failure.

**The campaign found a real defect in the test set.** M5/M5b/M5c did not exist
in the first run: the frontend refuses each of those shapes with its own
message *first*, so the IR arms were backstops **no test reached**, and a green
suite would have been evidence about the frontend's paths only. That is exactly
the `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` shape the `STABILITY-REEVAL`
cadence exists to catch. Five IR-level tests in
`crates/qcore-core/src/task_execution/tests.rs` now drive them directly.

## 9. Gate

Run at the card's tip. Commands and results:

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| `cargo test --workspace --no-fail-fast` | see §9.1 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | see §9.1 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | see §9.1 |
| `python3 scripts/check_lean_audits.py` | **133 audits, exit 0** |

Card-local suites: `compiler/tests/write_lend_tasks_exercise.rs` **16 tests**,
`crates/qcore-core` lib **964 tests** (five of them this card's IR backstops).
Disk at the start of the gate: 214 GB free, so the full gate ran rather than
the crate-scoped subset.

### 9.1 Full-gate numbers

See `gate.json` beside this report for the machine-readable record.

## 10. Named follow-ons

Each with its discharge condition.

1. **`WRITE-LEND-EXERCISE-WINDOWS-RUN`** — the card's native test ran the
   **Linux** arm only; the `windows_c` path is carried unchanged and no MSVC
   run has happened, so none is claimed. On the `WRITE-LEND-TASKS-WINDOWS-RUN`
   / `STRING-CONCAT-WINDOWS-RUN` precedent. *Discharge:* a Windows run of
   `two_threads_write_disjoint_bytes_of_one_object_sanitized` compiling the
   emitted C with `cl.exe` and comparing stdout byte for byte.
2. **`TASK-ROUTE-TSAN-SEGV`** — ThreadSanitizer cannot run any program this
   route emits, measured on three programs including a published example that
   holds no lease (§5.4). This is a *pre-existing* property of the emitted task
   runtime, surfaced here because this card is the first with a reason to want
   a TSan row. *Discharge:* identify the faulting construct (the fault is a
   read at offset `0x18` of a near-null pointer inside the worker trampoline,
   at an identical pc across all three programs), fix or work around it, and
   record whether TSan then reports a race between two disjoint byte writers.
   Until then the route has no TSan evidence and must not be described as
   having any.
3. **`WRITE-LEND-EXERCISE-BYTE-ORDER`** — a program mixing whole-payload and
   byte access has a host-dependent observation, because the payload's object
   representation is host-dependent. Reference and emitted C **agree** on every
   host (both use the host's representation), so this is a portability boundary
   on *published transcripts*, not an unsoundness; the flagship is
   representation-independent by construction (§5.3). *Discharge:* either
   commit the route to a byte order at the boundary, or refuse a whole-payload
   access on a buffer with byte-write history, whichever a later card measures
   to be cheaper — and say which programs each would cost.
4. **`WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`** — the owner has no byte write,
   deliberately (§3.3). *Discharge:* a card that wants one must re-measure wave
   31's "the owner holds no whole-payload authority while a lease is out"
   against a second owner-side footprint, and land the per-byte owner exclusion
   that rule currently does not need.
5. **`WRITE-LEND-EXERCISE-LOAN-BYTE-READ`** — a read share admits the
   whole-payload read and no byte view. *Discharge:* a cross-model rule
   relating read-share counts to byte ranges, on the sequential route's
   read-loan overlap precedent.
6. **`WRITE-LEND-TASKS-EXTENT`** — unchanged and inherited from wave 31: the
   payload is one `int32_t`, so a lease's bounds live in `[0, 4)`. A
   byte-extent buffer (`bytes(N)`) crossing a spawn is still future work, and
   this card makes it more valuable rather than less, since four bytes is a
   thin demonstration of disjoint concurrent writing. *Discharge:* as wave 31
   stated it.
7. **`CT07-TWO-THREAD-REMAP`** — CT07's current mapping (wave 30,
   `CT07-WRITE-REMAP`) rests on two write authorities held by **one sequential
   entry's bindings, not two threads**, and named `WRITE-LEND-TASKS` as its
   residual. That residual's successor has now landed, and a CT-wire lane
   could upgrade CT07's evidence to a genuinely two-threaded scenario with a
   sanitized native run behind it. **This card deliberately does not remap the
   row** (constraint 23: mapping is a lane's work, and moving a row here would
   move the derived reasons map against a matrix this card may not regenerate).
   *Discharge:* a CT-wire lane re-examines CT07 with this card's flagship as
   the candidate artifact and records, in its own stage-evidence record,
   whether the evidence improves and whether any count moves.

## 11. For the integrator

Please work each item; every one is either an amendment already made on this
branch (marked **applied here**) or a decision only you can take.

1. **RETIREMENT — `WRITE-LEND-TASKS-EXERCISE`.** Retire it. All three legs its
   charter demanded (checker rule, emission, sanitized native comparison)
   landed in this card, together, with Lean and with every new arm shown
   non-vacuous by mutation. Constraint 15h's "no operation admits a lease in a
   worker body" sentence is **superseded deliberately** by the appendix applied
   here.
2. **RETIREMENT — `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`.** Retire it. It is
   **decided**, not deferred: the F02 observable alphabet does **not** grow,
   on the causal-order ground in §6.1, with the arm driven (§6.3) and the
   cheaper reachability argument published beside it as a measurement rather
   than as the reason (§6.2). Constraint 15n's "what is not decided is the
   forward question" passage is amended accordingly, **applied here**.
3. **Standing constraint 15h — appendix, applied here** in the words §3 and §4
   use. It carries the split decision, the four rules, the emission facts and
   the three measured boundaries. Please land it as written or record
   `declined` with a ground; do not soften it while reconciling.
4. **Standing constraint 15n — amendment, applied here.** The forward question
   becomes a decision with its ground.
5. **§L21 — two condition widenings, applied here, no code allocated.** `Q020`
   (at-most-one-lease parameter; the byte spellings' kind refusals) and `Q600`
   (the footprint fit, the constant-index tier, the three authority rules).
   The `Q-NAMESPACE-DECISION` consequence count moves **thirteen → fourteen**,
   still on **nine** bands (this card reuses wave 31's two), **applied here**.
6. **Ledger headline numbers — `named_followups` 159 → 165, applied here**
   (+7, −1). The seven added are every residual §10 names:
   `WRITE-LEND-EXERCISE-WINDOWS-RUN`, `TASK-ROUTE-TSAN-SEGV`,
   `WRITE-LEND-EXERCISE-BYTE-ORDER`, `WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`,
   `WRITE-LEND-EXERCISE-LOAN-BYTE-READ`, `CT07-TWO-THREAD-REMAP`, and
   **`WRITE-LEND-TASKS-EXTENT`** — the last being wave 31's own residual, which
   this card does not discharge and which **no tracker held**, so it is
   registered rather than left to `--residuals` to find. The one retired is
   `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` (item 2).
   `WRITE-LEND-TASKS-EXERCISE` itself had no matrix entry — it lived only in
   constraint 15h's text — so its retirement is the constraint amendment in
   item 3 and moves no count. AGENTS.md's stated count is updated in step.
   Please re-derive from the file rather than trusting this arithmetic.
7. **A decision I did not take: `TASK-ROUTE-TSAN-SEGV`'s priority.** It is not
   this card's defect and I have not fixed it, but it means the **whole checked
   concurrency route has no thread-sanitizer evidence at all** — which is a
   larger fact about the project's native evidence than it is about this card.
   If you agree it is wave-37 material rather than a follow-on, please promote
   it.
8. **A decision I did not take: CT07.** §10 item 7 explains why this card does
   not remap the row. If you would rather a lane did it in the same wave, it is
   ready to be scheduled.
9. **The matrix was regenerated, not hand-counted, and the gate provenance is
   untouched.** `named_followups` is hand-maintained, so editing it left the
   **derived** `dimensions.named_followups` stale at 159 — precisely
   `D02-DIMENSIONS-FOLLOWUP-COUNT`, the residual that fired at the wave-32 and
   wave-33 closes, and `CommittedMatrixIsSelfConsistent` (wave 34) caught it
   here. Repaired by running `docs/reports/remaining-d02/build_matrix.py`, not
   by editing the count. The regeneration's **only** other change was derived
   too (`validation.lean_audits` 132 → 133, this card's audit file), and
   `validation.measurement_provenance` was **restored to the committed value**
   rather than let the generator relabel the wave-35 close's
   integrator-supplied gate numbers as carried (constraint 25). **The wave-36
   close should pass `--gate` with its own measured numbers**; this card
   deliberately did not, because that flip is the integrator's.
   274 Python tests pass.
10. **No `gate.json` claim beyond §9.** The card's own counts are in §9; the
   wave close should re-measure at the integration tip rather than carry mine.
