# `CT-LANE-5` — wave 37's CT mapping lane

**Wave 37. Base `0f6eb0a6`. One case re-mapped, three re-measured and left
unmapped, one not attempted. No count moves.**

## Headline

CT07's scenario is *two disjoint array halves mutated independently*. For two
waves the word **independently** was carried by a program with one thread in it.

Wave 28 mapped CT07 onto two disjoint **read** loans. Wave 30
(`CT07-WRITE-REMAP`) remapped it onto two **write** authorities over disjoint
ranges — and said, in its own semantic profile, exactly what it was not:

> both write authorities are bindings of **ONE SEQUENTIAL** entry — disjoint
> write authority with interleaved statements, **NOT two threads**; the
> task-crossing half is residual `WRITE-LEND-TASKS`.

That residual's successor landed in wave 31, and wave 36's
`WRITE-LEND-TASKS-EXERCISE` made it exercisable: a worker body writes through
its lease, and two threads write distinct bytes of one `int32_t` under
ASan/UBSan/LSan. That card deliberately did **not** remap the row, and named the
work as its own follow-on item 7, `CT07-TWO-THREAD-REMAP`, with a discharge
condition. This card is that discharge.

CT07 now rests on the two-thread artifact: route `sequential` → `task`, verdict
mode `sequential_sanitized` → `native_heap_scenarios`, executor
`run_sequential_sanitized` → `run_native_heap_scenarios`. It becomes **CT03's
write twin** — where CT03 is two readers over one allocation, CT07 is two
writers over disjoint byte ranges of one allocation.

**No count moves.** CT07 was mapped before this card and is mapped after it.
Mapped stays **46 of 50**, sanitized-native **10 of 46**, proof **3 of 46**.
This is evidence quality, not coverage (constraint 23), and the card's own
report says so before anyone has to ask.

CT42, CT10 and CT41 were re-measured and are **left unmapped**, each with the
measurement published in §3–§5 rather than inherited. CT24 was not attempted
(§5.2). **No C gateway source file is touched.**

## 1. What discharges the follow-on, in its own words

Wave 36 §10 item 7 set the condition:

> *Discharge:* a CT-wire lane re-examines CT07 with this card's flagship as the
> candidate artifact and records, in its own stage-evidence record, whether the
> evidence **improves** and whether **any count moves**.

Both halves are recorded in `ct_stage_evidence.json` and answered here:

* **The evidence improves.** §2 takes the scenario apart word by word.
* **No count moves.** §6.

## 2. CT07 — the scenario's words, and the fact carrying each

The case artifact is `compiler/tests/fixtures/ct/CT07/case.qcore`: one
allocation whose payload is a single `int32_t`; two children holding write
leases over `[0, 2)` and `[2, 4)`; each child writing a byte through its own
lease; the owner reading both bytes back after both joins.

| Scenario word | What carries it now | What carried it at wave 30 |
|---|---|---|
| **two** | Two `thrd_create`'d OS threads, spawned before either is joined | Two *bindings* in one entry |
| **disjoint** | `ownership::axes_overlap`, called by `scoped_tasks::LeaseRange::overlaps` in the ledger, deciding at check time | the same primitive, on the sequential lend's borrow node |
| **halves** | `[0, 2)` and `[2, 4)` of the four real payload bytes | `[0, 4)` and `[4, 8)` of an eight-byte buffer |
| **mutated** | A real byte write in each **worker body**, reaching memory as `((unsigned char *)&buffer->payload)[index] = byte;` | Four stores in the parent's own statement list |
| **independently** | **Two threads.** Both leases are outstanding simultaneously — the observation shows `QCORE_BUFFER_LEASE 1` and `QCORE_BUFFER_LEASE 2` both taken before either byte is written | Interleaved statements of one thread |
| **Accept** | The module checks, emits, and runs clean under ASan/UBSan/LSan | same |
| **bounds** | Two controls, one per layer (§2.2) | two controls |
| **disjointness proof** | *Not claimed as a proof.* The **decision** is the ledger's; the theorem exists but is not bound to this artifact — see `CT07-TWO-THREAD-PROOF-LEG` | not claimed either |

### 2.1 The measured observation

The native run's standard output, compared **line for line** by
`run_native_heap_scenarios` against `render_task_observation` of the checked
task IR's reference execution:

```
QCORE_BUFFER_ALLOC 0 1 0
QCORE_TASK_SPAWN 1 0
QCORE_BUFFER_LEASE 1 1 0 2
QCORE_TASK_SPAWN 2 0
QCORE_BUFFER_LEASE 2 1 2 4
QCORE_BUFFER_BYTE_WRITE 1 1 0 9
QCORE_TASK_RESULT 1 I32 1
QCORE_TASK_JOIN 1
QCORE_BUFFER_LEASE_RETURN 1 1
QCORE_BUFFER_BYTE_WRITE 2 1 2 9
QCORE_TASK_RESULT 2 I32 1
QCORE_TASK_JOIN 2
QCORE_BUFFER_LEASE_RETURN 2 1
QCORE_BUFFER_BYTE_READ 0 1 0 9
QCORE_BUFFER_BYTE_READ 0 1 2 9
QCORE_BUFFER_DROP 0 1
QCORE_TASK_RESULT 0 I32 9
```

Two things are visible rather than asserted. **Both leases are live at once** —
lines 3 and 5, before any write — which is the scenario's simultaneity. And the
**rebasing law**: each worker writes its own *relative* byte `0`, and the events
name owner bytes `0` and **`2`**.

Beside it, the heap witness (`track.h`, this case's own): **one** allocation,
**one** release, matched **by address**, so "two threads, one object" is an
identity claim and not a trace claim.

### 2.2 The controls — a strict superset of wave 30's

Eight controls, of which four are wave 30's and wave 28's, retained:

| Control | Role | Measured |
|---|---|---|
| `overlapping_leases.qcore` | disjointness | `[Q600]`, the ledger's own `OverlappingLeases { requested: [2,4), held: [0,3) }`, naming both ranges |
| `past_range_byte.qcore` | bounds — the **lease's** range | `[Q600]`, "worker exercises bytes [0, 3) of its lease, which does not fit the leased range [0, 2) of extent 2" |
| `past_extent_read.qcore` | bounds — the **payload's** extent | `[Q600]`, "byte 4 is outside the payload's [0, 4) extent" |
| `overlapping_lends.qcore` | disjointness, sequential twin *(wave 30)* | `[Q054]` |
| `out_of_extent_lend.qcore` | bounds, sequential twin *(wave 30)* | `[Q053]` |
| `loan_index_out_of_extent.qcore` | bounds, two-layer *(wave 30)* | `[Q054, Q053]` — the **ordered** vector |
| `sequential_write_halves.qcore` | **accept** — wave 30's own case program | checks, under `StandardBufferV1` |
| `read_loan.qcore` | **accept** — wave 28's own case program *(wave 30)* | checks, under `StandardStringV1` |

The two bounds controls are separate on purpose, because neither implies the
other. `past_range_byte.qcore`'s index names a **real payload byte** — in fact
the byte leased to the *other* worker — so it refuses for the **lease**;
`past_extent_read.qcore` refuses for the **payload**. And the payload bound is a
checker refusal rather than a sanitizer observation for a measured reason: wave
36 §5.5 found that with the emitted guard removed, one byte past the payload
lands in the buffer struct's own **tail padding**, where AddressSanitizer is
**silent**.

### 2.3 Two harness extensions, and why each was necessary

Moving CT07 onto the heap-scenario mode would have **dropped evidence**, because
that mode carried no accept controls (CT03 and CT17 have none) and pinned only a
head diagnostic. A remap that quietly stopped admitting the configuration it
used to rest on is a narrowing wearing an upgrade's clothes. So:

* **`accept_controls` on `NativeHeapScenarios`** — a program that must still
  check, with the execution profile it must still be published under.
* **`diagnostic_sequence` on `TaskRejectionControl`** — because
  `loan_index_out_of_extent.qcore` refuses in **two** layers, and pinned on its
  head code alone, either layer could have fallen silent without failing.

Both are `#[serde(default)]`, so CT03's and CT17's fixtures are **byte-untouched**.

### 2.4 Non-vacuity — seven mutations, seven caught

Green is an observation, not a proof, so each claim the new row makes was broken
on purpose (`measure/ct-lane-5/mutate.py`; results in `mutations.json`):

| Mutation | Result | What it proves is really checked |
|---|---|---|
| read-back byte `9` → `8` | CAUGHT | the native run really happens and is really compared |
| second worker's owner byte `2` → `1` | CAUGHT | the rebasing law is **observed**, not assumed |
| `heap_allocations` `1` → `2` | CAUGHT | the heap witness is read; "one object" is by address |
| accept control's profile → `StandardStringV1` | CAUGHT | an accept control cannot silently change route |
| accept control → a **refused** program | CAUGHT | the accept arm really requires admission |
| two-layer vector → `[Q053, Q054]` | CAUGHT | the **order** is checked |
| one-layer vector → `[Q600, Q600]` | CAUGHT | a vector cannot claim a layer that is not there |

### 2.5 Boundaries — published, not papered over

* **There is no ThreadSanitizer evidence, and none is claimed.** TSan cannot run
  *any* program this route emits — lease or not — measured in wave 36 on three
  programs including a published example holding no lease, all faulting at the
  identical address and pc (`TASK-ROUTE-TSAN-SEGV`). So "two threads write
  distinct memory locations, and this is not a data race" is carried by C11 3.14
  plus the sanitizer and allocation-identity evidence, **not** by a race
  detector. A sibling card `TASK-ROUTE-TSAN` is working that residual in
  parallel; this card neither waits for it nor borrows its result. **What its
  landing would add to CT07**, exactly: a TSan row asserting *silence* on this
  flagship — the direct observation that two disjoint byte writers are not a
  race, which today is an inference from the standard rather than a measurement.
  It would also be the only instrument that could observe the two writes
  overlapping in time (see the next bullet).
* **The byte-identical trace does not witness interleaving.** Measured, and
  registered as `CT07-TWO-THREAD-REPLAY-ORDER`. Every worker writes its
  diagnostic events into its **own private event array**, which the parent
  replays at collect (constraint 15a; the array is named in
  `diagnostic_log_c_emit.rs`'s own comment). The 17-line observation is therefore
  deterministic **by construction**, and its agreement with the reference is
  evidence about custody, values and rebasing — not about real-time order. The
  concurrency evidence is real but lives elsewhere: two `thrd_create`'d threads,
  the disjointness decided in the ledger before emission, one allocation by
  address, a clean sanitizer run.
* **The extent.** The payload is one `int32_t`, so the two "halves" are two bytes
  each. Four bytes is a thin array. `WRITE-LEND-TASKS-EXTENT`, inherited from
  wave 31 unchanged.
* **The proof stage stays `unassessed`.** `CT07-TWO-THREAD-PROOF-LEG`.
* **Platform.** Linux arm executed; the runner compiles on macOS and the
  manifest's `host_gate` says so; no Windows or Apple run is claimed.
* **One store per party.** The ledger admits at most one write lease per worker
  and the route admits a check-time-constant index only, so "mutated" is one
  store each, not a loop.

## 3. CT42 — both arms re-measured, still unmapped

Scenario: *two descriptors share an underlying logical state*; expected
*preserve shared protocol-cell identity*. Both arms were re-measured on this
tree, and each fails for its own reason.

**Arm A — the C twin (`CT-CTWIN-DESCRIPTOR`).** The contract loader still
refuses an int-typed resource identity, at
`crates/qcore-contract/src/contract_loader.rs:757-759`:

```rust
if result != ResourceReturnMode::None && !return_type.starts_with("ptr<") {
    return Err("owned result requires a pointer return type".into());
}
```

which produces the verbatim refusal waves 9, 17 and 35 all recorded, and which
is pinned by `compiler/tests/contract_loader.rs::every_abi_scalar_is_a_borrowed_only_projection`
— so it is load-bearing, not incidental. Discharging it **starts** at one line
and **cascades**: `qcore-schemas`' `ResourceReturnMode` records only
nullability, and a descriptor's failure value is `-1`, not null, so the failure
arm is not expressible at all; `crates/qcore-core/src/extern_call.rs:790-801`
refuses a non-pointer owned result independently; and
`crates/qcore-c-gateway/src/legacy_declared_abi.rs` restates the rule as a
premise it enforces, beside a projection that tracks identities as **pointer
values**. This lane's charter forbids C gateway source changes and says to map
only where every word is carried without one. By this measurement, Arm A needs
one. **Not attempted; named.**

**Arm B — the socket protocol cell (`CT42-CELL-UNROUTED`).** Wave 31's
`CT42-PROTOCOL-CELL` landed the model half: `derive_socket_cells` and
`footprint_program` in `crates/qcore-core/src/socket_footprint.rs`, composing
with the unedited `Q664` `SharedProtocolCell` refusal, with Lean. Re-measured
here: `derive_socket_cells` and `footprint_program` have callers in
`crates/qcore-core/tests/unit/` **and nowhere else** — not the frontend, not the
checker, not `compile_files`. A `.qcore` program can already *spell* the two
names (`examples/sockets/tcp_echo.qcore` binds `client` and `server` over one
completed connection), but what the pipeline produces for it is a
`SocketCertificate`, which has no `protocol_cells`; no source statement asks the
disjointness question at all.

A CT fixture is a program the harness compiles through the real pipeline, so a
CT42 row on Arm B would name stages the harness **cannot execute** — precisely
what constraint 23 forbids. Wiring the deriver in needs new source syntax and a
checker change: **a route card, not a mapping lane.** `CT42-CELL-UNROUTED`
holds unchanged.

## 4. CT10 — the sub-byte half measured, still unmapped

Scenario: *adjacent bit-fields in one memory location*; expected *reject
independent concurrent writes*. The concurrency half landed in wave 31; the
sub-byte half is what remains, and this card measured exactly where the floor is.

**The concurrency half is present, byte-granular and source-reachable.**
`LeaseRange` / `WriteLeased` / `OverlappingLeases` in `scoped_tasks.rs`, deciding
through `ownership::axes_overlap` (`ownership.rs:145`), reachable from source by
the `lend NAME LOW..HIGH` capture. This lane's own CT07 remap **is** that
mechanism executing.

**The sub-byte half is representable in exactly one model-level module and
unreachable from any program.** `FieldStorage::BitfieldUnit { unit, bit_offset,
bit_width }` (`crates/qcore-core/src/disjoint_footprint.rs:99-103`) is the only
bit-addressed type in the workspace — and its own documentation says the
declared bit range *"never participates in the disjointness decision"*
(`:214-216`). `FootprintExport::SharedMemoryLocation` (`:229-231`) exists
precisely so a bit-field unit **cannot** be exported to a concurrency checker as
an interval. The module has **zero** callers in the frontend, the checker or
`compile_files`. There are no bit-field tokens anywhere in
`crates/qcore-frontend/`, and the source record model carries no offsets or
widths at all. The C gateway reads `isBitfield` at four sites purely to
**refuse**, its token reader refusing on the `:` character and discarding the
whole enclosing record.

**The shortcut is unsound in both directions — a new measurement, registered as
`CT10-BYTE-ENCODING-UNSOUND`.** Wave 31 offered and wave 35 declined the obvious
encoding: represent two adjacent bit-fields as two leases of the byte they
share, and let the byte-granular exclusivity refuse them. That is not merely
"the right answer for the wrong reason". C11 makes two bit-fields one memory
location exactly when they share a storage **unit**, so the byte rule would
**admit** a program whose bit-fields lie in different bytes of one unit (C11
makes them one location) and **refuse** one whose adjacent members are
addressable (they are genuinely independent). Mapping CT10 on that encoding
would put a wrong rule behind a right verdict.

Closing it needs either a bit-field declaration in `.qcore` so a `LayoutContract`
is derived rather than hand-built, or `DisjointFootprintsV1` wired to a source
route at all. Both are route cards. **Left unmapped rather than mapped on half a
scenario.**

## 5. CT41 and CT24

### 5.1 CT41 — not re-attempted, and why that is a decision

Wave 35 declined CT41 on a ground that is structural rather than a gap: across
five shapes through the real gateway, the producer with `volatile int g_done;`
and the producer with plain `int g_done;` are **byte-identical** in verdict,
reason and diagnostic, and the consumer shape is `safe` with no diagnostic.
Identical verdicts across the volatile twins are the **definition** of failing
clause (b) of the wave-9 attribution rule — removing the named construct must
move the verdict, and here it moves nothing.

This lane's charter says not to re-attempt without new capability, and there is
none: the resource projection still has no synchronization model. What the
briefing names as new is a name for the premise on the gateway side —
**`GATEWAY-VOLATILE-PREMISE`**, from wave 37's first arbiter — which is where any
discharge would have to begin. **Stated with its provenance rather than as a
fact of this tree:** that card had not merged at this card's base, and a grep of
`docs/` at the tip finds the name in *this card's own files only*. So it is
recorded here as a forward pointer, not as a registration this lane verified;
the integrator should confirm it lands with the arbiter before treating CT41's
decline as resting on it. The decline itself does not depend on it — it rests on
wave 35's measurement, which stands on its own.

### 5.2 CT24 — not attempted

Withdrawn twice (wave 26's lane, and wave 30's `CT24-CLEANUP-PREEMPTED`), and
constraint 25 makes a third attempt actively hazardous: the CT
`newly_decidable_not_mapped_reasons` map is **derived** from the judgment's own
key set, so publishing a labelled row for CT24 would move that map's keys
against a matrix this lane may not regenerate — putting Python tests red for a
case nothing about which moved. Its card T00 `DECIDABLE-NOW` bucket stands
untouched.

## 6. Counts — every one unmoved

| Measure | Before | After |
|---|---|---|
| CT mapped | 46 / 50 | **46 / 50** |
| CT unmapped | 4 (CT10, CT24, CT41, CT42) | **4 (unchanged)** |
| sanitized-native | 10 of 46 | **10 of 46** |
| proof | 3 of 46 | **3 of 46** |
| `named_followups` | 187 | **191** (four registered, §8) |
| CT evidence files | 22 | **23** |

CT07 was already in `SANITIZED_GATED_CASES` and remains there — one gated case
leaving the sequential runner and arriving at the heap runner, so the table is
unchanged. Nothing was promoted and no unmapped case became mapped. The only
numbers that move are this card's own residual registrations.

`mapped 46 / unmapped 4` is re-derived by `scripts/ct_stage_summary.py . --check`
with **no drift**.

## 7. Gate

Run at this card's own tip in an isolated worktree; outputs are worktree-private
under `measure/ct-lane-5/`, exit codes captured to files rather than through a
pipe. `gate.json` beside this report carries the numbers.

| Step | Exit |
|---|---|
| `cargo test --workspace --no-fail-fast` | **0** — 4,611 passed / **0 failed** / 8 ignored, 231 result summaries |
| `cargo fmt --all -- --check` | **0** |
| `cargo clippy --all-targets -- -D warnings` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **0** |
| `python3 scripts/check_lean_audits.py` | **0** — 135 audits |
| `python3 scripts/ct_stage_summary.py . --check` | **0** — no drift |

Three things are recorded rather than smoothed over.

**The run above is the second.** The first was launched before this card's last
source edit and was **killed rather than reported**, because it would have been a
measurement of a source state this card does not ship —
`compiler/tests/write_lend.rs` `include_str!`'d CT07's `case.qcore`, which this
card replaces (§9). Killed by PID after checking `/proc/<pid>/cwd`, never by
pattern, and never touching the sibling worktree's or the main checkout's
processes.

**`cargo fmt --check` failed on its first invocation here**, on one line of this
card's own comment block, and is recorded as measured-again-after-fixing rather
than as having passed the first time.

**The workspace test count is unmoved** from the wave-36 close's 4,611 / 0 / 8,
and that is expected rather than surprising: this card adds no test *function*.
It re-points an existing case at a new artifact, extends two existing runners and
adds fixture data — all exercised by tests that already existed. The card's own
new evidence is the seven mutations of §2.4, which are a falsification harness
rather than committed tests.

**Not run, and stated rather than omitted:** provider tests (this card changes no
manifest template and no `qcore-schemas` major version, so constraint 18's
trigger does not fire) and the Python evidence-validator suite as a whole. The
acceptance matrix is therefore regenerated **without** `--gate`, so its standing
gate block is *carried and labelled as carried* by the generator's own mechanism
rather than being restated here as freshly measured.

## 8. Residuals and retirements

### 8.1 Registered by this card (`named_followups` 187 → 191)

1. **`CT07-TWO-THREAD-PROOF-LEG`** — CT07's `proof` stage stays `unassessed`,
   though *two* Lean theorems are about the mechanism it now rests on
   (`no_byte_has_two_write_authorities`, and wave 36's
   `no_byte_is_written_by_two_parties`, which is the scenario's "disjointness
   proof" in the model). Neither is bound to CT07's artifact by a named test, so
   citing either would be the overstatement constraint 23 forbids. This is an
   unusually cheap proof leg *because the theorem already exists* — the missing
   half is only the binding. *Discharge:* a proof-leg card on the CT01/CT37/CT03
   pattern, moving the proof numerator 3 → 4.
2. **`CT07-TWO-THREAD-REPLAY-ORDER`** — the byte-identical trace does not witness
   interleaving, because worker events are buffered privately and replayed by the
   parent (§2.5). *Discharge:* accept it as a stated boundary of the trace
   mechanism, or land the TSan row.
3. **`CT10-BYTE-ENCODING-UNSOUND`** — the byte encoding of bit-fields is wrong in
   both directions, not merely coarse (§4). *Discharge:* a bit-field declaration,
   or `DisjointFootprintsV1` wired to a source route. Both route cards.
4. **`CT-FIXTURE-EXTERNAL-READERS`** — eight CT fixture directories are read by
   `include_str!` from outside `ct_corpus.rs`, three of them the proof-gated
   cases, so a mapping lane can silently re-point another card's tests. This card
   hit exactly that and caught it by noticing a stale doc comment, which is luck
   rather than a mechanism (item 10 of §11 carries the measured list).
   *Discharge:* a check that every such `include_str!` is declared in one place or
   refused.

### 8.2 Retirement this card earns — for the integrator

**`CT07-TWO-THREAD-REMAP`** (wave 36's `WRITE-LEND-TASKS-EXERCISE` §10 item 7).
Its discharge condition was to re-examine CT07 with that card's flagship as the
candidate artifact and record, in this lane's own stage-evidence record, whether
the evidence improves and whether any count moves. Both are recorded: the
evidence improves (§2), no count moves (§6). **Verified registered:** the name is
present in `docs/reviews/r0_3/acceptance_matrix.json` `named_followups`, so this
retirement has something to retire.

### 8.3 Retirements this card does **not** claim

`retired_unmapped_reasons` is **empty, deliberately**. No unmapped row is
retired, because no case became mapped. CT42, CT10, CT41 and CT24 keep exactly
the buckets and label sources they carried at base — re-verification reaching the
same label is the record saying the label still holds, and inventing a new bucket
would be label churn for cases nothing about which moved. The measurements are
published in §3–§5 and in the stage-evidence record, which is where they belong;
they narrow the *reasons* without moving the *labels*.

Neither `CT-CTWIN-DESCRIPTOR`, `CT42-CELL-UNROUTED`, `TASK-ROUTE-TSAN-SEGV`,
`WRITE-LEND-TASKS-EXTENT` nor `CT10-REASON-ROW` is retired here; each is
re-measured or inherited and each stands.

### 8.4 Constraint 21 — the supersession, stated

`docs/reports/ct-lane-5/ct_stage_evidence.json` declares
`"supersedes": "docs/reports/ct07-write-remap/ct_stage_evidence.json"` on its
CT07 entry. Wave 30's record **keeps its bytes**; nothing in it is edited, and
its history stays visible. The supersession is checked in both directions by
`scripts/ct_stage_summary.py`: a declaration naming no earlier record, or naming
the wrong one, is refused, so it can go neither stale nor vacuous.

## 9. Fixture disclosure (`CT-FIXTURE-CROSS-CASE-DUPLICATES`)

**Run, not asserted** (`measure/ct-lane-5/fixture_sweep.py`). Every `.qcore`,
`.c` and `.h` file under `compiler/tests/fixtures/ct/` was hashed — **165
files**. The result is exactly the five groups already on record:

```
CT01/case.qcore = CT12/case.qcore = CT14/case.qcore
CT03/case.qcore = CT04/write_after_joins.qcore
CT01/owned_transfer.h = CT12/owned_transfer.h
CT14/completion_before_join.h = CT37/completion_before_join.h   (crosses a proof case)
CT02/case.qcore = CT14/read_before_join.qcore
```

**This card contributes to none of them.** Two relationships it *does* create are
disclosed, because a reader comparing the files should know which differences are
deliberate:

* `CT07/case.qcore` is the plain **scoped-task** form of the `DISJOINT_EXERCISE`
  constant in `compiler/tests/write_lend_tasks_exercise.rs` — a Rust string
  literal, not a fixture file, so it collides with nothing the sweep ranges over.
  The deliberate differences: no `import std.logger;` and no `log_level trace;`,
  so the checked artifact is a `ScopedTasks` module the heap runner consumes
  rather than a `ComposedTasks` one, and both of the owner's byte reads are kept.
* `CT07/sequential_write_halves.qcore` carries wave 30's case program with a
  different leading comment. Its **code** is byte-identical to what
  `CT07/case.qcore` held at base `0f6eb0a6`; the file is not, because the comment
  explains why the program is now a control. That is the same disclosure wave 30
  itself made when it moved the program in from
  `compiler/tests/fixtures/write_lend/disjoint_write_halves.qcore`.
* `CT07/track.h` is deliberately **not** byte-identical to CT03's or CT17's: the
  marker prefixes differ (`LEASE_ALLOC`/`LEASE_FREE`), which is what lets each
  case's heap witness be attributed to that case when a run fails.

## 10. What this card does not claim

* No new mapping, no promotion, no count moved except its own residual
  registrations.
* No proof stage, for CT07 or any case; the proof numerator is unmoved at 3.
* No TSan evidence, for CT07 or for the route.
* No Windows or Apple run.
* No new checker rule, diagnostic code, lowering, model, Lean theorem, reserved
  word or token. The only Rust is harness wiring in `compiler/tests/ct_corpus.rs`.
* **No C gateway source change of any kind** — two arbiters own
  `crates/qcore-c-gateway/` this wave, and the one mapping that would have needed
  it (CT42's Arm A) is named as a residual instead.
* No edit to any frozen archive (constraint 21).

## 11. Integrator reconciliation list

Every item is either an amendment already made on this branch (**applied here**)
or a decision only the integrator can take.

1. **RETIREMENT — `CT07-TWO-THREAD-REMAP`.** Retire it. Wave 36's discharge
   condition is met in both its halves (§8.2), and the name is registered in the
   matrix, so there is something to retire. If you decline, please record the
   ground; the work it asked for is done either way.
2. **Ledger headline — CT row, applied here in the derived records, yours in
   `AGENTS.md`.** The CT row's numbers do **not** move: 46/50 mapped,
   sanitized-native 10 of 46, proof 3 of 46. What wants a sentence is that
   **CT07's evidence moved onto a two-thread artifact with no count moving** —
   the `CT07-WRITE-REMAP` precedent from wave 30, which the ledger already
   records the same way ("evidence quality, not coverage"). Suggested text, to
   land or decline as written: *"wave 37 (`CT-LANE-5`) re-mapped **CT07** off the
   sequential write lend onto the two-thread range write lease — the scenario's
   word `independently` resting on two threads for the first time, with a
   sanitized native run and an allocation-identity witness behind it — superseding
   the wave-30 record from its own record with **no count moved**, and published
   the measurement for CT42, CT10 and CT41, all three still unmapped."*
3. **Constraint 23 — no amendment proposed.** This card is an instance of the
   rule, not a change to it. Noted so the absence is deliberate rather than an
   oversight.
4. **Constraint 14 — this card does not engage it.** No reserved word, no token,
   no lexer change. Please add it to the wave's "cards that do not engage this
   constraint at all" list.
5. **Q-namespace — this card allocates nothing and widens no condition.** The
   three new controls all refuse at the **existing** `Q600` conditions the
   scoped-task band already owns (`WRITE-LEND-TASKS`' and
   `WRITE-LEND-TASKS-EXERCISE`' own widenings, wave 31 and wave 36). Noted for
   the same reason as item 4.
6. **Three residuals to carry** (§8.1). `CT07-TWO-THREAD-PROOF-LEG` is the
   cheapest of the three and the only one that would move a numerator.
7. **A decision you may want to take: `CT07-TWO-THREAD-REPLAY-ORDER`.** It can be
   *closed by statement* rather than by work — the private-buffer-and-replay
   design is deliberate and makes traces comparable at all — but it should be
   closed by someone willing to say so in the ledger, because the alternative is
   that a future reader takes CT07's byte-identical comparison for evidence of
   interleaving. I have recorded it as open rather than pre-empting that call.
8. **Sequencing note, no action.** A sibling card `TASK-ROUTE-TSAN` is working
   `TASK-ROUTE-TSAN-SEGV` this wave. If it lands, §2.5 names exactly what it adds
   to CT07 (a TSan row asserting silence on this flagship). This card makes no
   claim on its outcome and does not depend on it.
9. **`validation.measurement_provenance` was relabelled by the regeneration, and
   you will want to set it back at the close.** Regenerating without `--gate`
   moved it from `integrator_supplied (docs/reports/wave36-close/gate.json)` to
   the generator's `carried_from_previous_matrix` string. **The numbers
   themselves did not change** — all twelve other `validation` keys are
   byte-identical — and this card's own measured run *agrees with them exactly*
   (4,611 / 0 / 8; 135 Lean audits), so the block is relabelled rather than
   stale. I left it carried rather than passing `--gate`, because two of the nine
   required gate keys (`provider_tests`, `python_evidence_validator_tests`) are
   measurements this card did not take, and supplying them under an
   `integrator_supplied` label would have asserted numbers I did not measure.
   *Action:* regenerate with `--gate` at the close, which restores the label
   correctly. Flagged because a relabel is easy to miss in a 45-line diff.
10. **A structural hazard this card hit, measured and registered:
    `CT-FIXTURE-EXTERNAL-READERS`.** `compiler/tests/write_lend.rs`
    `include_str!`'d `fixtures/ct/CT07/case.qcore`, so re-pointing CT07's case
    silently handed another card's tests a task-route program. I caught it, fixed
    it (§9), and then **measured how general it is** rather than assuming it was
    a one-off. It is not: **eight** CT fixture directories are read by
    `include_str!` from outside `ct_corpus.rs` —

    ```
    CT01  ct01_proof_leg.rs, ct12_ct14_proof_reach.rs, f02_trace_relation.rs,
          owned_source_tasks.rs, windows_threads.rs
    CT03  ct03_proof_leg.rs, windows_threads.rs
    CT07  write_lend.rs                       <- this card's own collision
    CT12  ct12_ct14_proof_reach.rs
    CT13  source_tasks.rs
    CT14  ct12_ct14_proof_reach.rs
    CT17  windows_threads.rs (case.qcore AND track.h)
    CT37  ct37_proof_leg.rs
    ```

    so **three of the four proof-gated or proof-adjacent cases** (CT01, CT03,
    CT37) are in it, which is the worst place for it: a mapping lane re-pointing
    one of those would move a proof leg's artifact with nothing in the CT harness
    saying so. Registered as a residual rather than left as advice, because the
    thing that saved this card was noticing a stale doc comment, which is not a
    mechanism. *Discharge:* a check that every `include_str!` of
    `compiler/tests/fixtures/ct/` outside `ct_corpus.rs` is either listed in one
    place or refused — small, and a close-rule or test-level check rather than a
    card.

## Published beside by `WAVE-38-CLOSE` (2026-09-18) — `mutations.json`, and the §2.4 harness

Appended, not edited (constraint 21): every line above the rule is the frozen
card text. §2.4 cites "`measure/ct-lane-5/mutate.py`; results in
`mutations.json`". This directory holds no `mutations.json`: the seven results
are the `mutations` key of `ct_stage_evidence.json` (line 237 at `4f663934`),
so the citation names the wrong file; and `measure/` is a gitignored,
worktree-private path, so the harness is not in the tree (`ls` at `4f663934`:
`README.md`, `ct_stage_evidence.json`, `gate.json`). The seven-of-seven count
is therefore **measured once by the card, harness not shipped** — a later
reader can re-derive each mutation from the table's description but cannot
re-run the card's script. Recorded under
`W36-W37-MUTATION-HARNESSES-UNSHIPPED` (card `STABILITY-REEVAL-3`, wave 38);
the card-report convention in `CLAUDE.md` now says a mutation count is cited
only with its script and its per-mutation log in the report directory. This
note changes no code and re-runs nothing.
