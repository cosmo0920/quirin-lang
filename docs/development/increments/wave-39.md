# Wave 39 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`LOOPS-ON-COMPOSED-ROUTES` (2026-09-18).** A `for` beside a borrow or a lend
is admitted on both routes that hold a loan, with the loan's **window computed
over iterations**: on the buffer route `std_buffer::last_uses` takes the last
top-level statement naming the loan *at any depth*, so a loan a loop body
names is live across every iteration, and the lend-structured projection
unrolls the body inside the borrow node with the index bound; on the composed
string route every body store, at every iteration, is judged against every
loan still read after the loop before the body runs (`loop_body_conflict`),
the body then running through the buffer route's own `BufferRun::repeat`
(published for this, so there is one iteration rule) and the composed custody
list unrolling it. Proved sound relative to `LOOP-V1`'s unrolling in
`QCore.LoopWindows` (`AuditLoopWindows.lean`, 140 on disk after the rebase onto
`e80b58d6`: `every_use_lies_inside_the_window`,
`the_unrolled_window_lies_inside_the_iteration_window`,
`the_unrolling_is_loop_v1s`, `the_window_is_exact_over_a_flat_program`, and
the non-vacuity `a_top_level_only_window_misses_a_body_use`; the IR's
decisions shown monotone in the live loans, which is what makes the
over-approximation refusing). **Re-derived rather than inherited**: on the
composed route the measured premise ("a view used only inside a body") is
unwritable — a body holds buffer events, which name no view — and the real
admitting arm was the `Repeat` arm judging and running nothing; both repaired.
`SLICE-PERMISSIONS-LOOP-WINDOW` lifted (it had no tracker entry, by the wave-38
close's decision) and **`STRING-CONCAT-LOOP-WINDOW` retired**, its discharge
met word for word; `WRITE-LEND-RELEND` left open, orthogonal. Three residuals
registered, each a refusal **by name** that sits before the window is
computed: `LOOP-PER-ITERATION-LOAN`, `LOOP-WINDOW-BODY-CUSTODY`,
`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY` (`named_followups` 232 → 234 on the
rebased tree). Rebased onto `e80b58d6` behind `AGGREGATE-OWNED-FIELD` and
`CT07-TWO-THREAD-PROOF-LEG`, with that card's two by-name refusals
(`OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY`) pinned against the
iteration windows — they sit before this card's block and no window is
computed for them.
Constraint 15h amended deliberately, its two "one flat statement sequence"
sentences quoted as superseded in place and the theorem appended; §L21 rows
`Q051`, `Q068` and `Q077`–`Q079` carry the narrowings with the exact admitted
shape, `Q078` unwidened, `Q097`–`Q099` reached from the composed route
unwidened (`StringError::Loop`), zero codes allocated; constraint 14 +0, the
census run. Six mutations shipped and run, six red, three files restored
sha256-equal (M1 is the `STABILITY-REEVAL-3` §4 scenario admitted under the
top-level-only window). Emission census over 527 programs: 139 admitted by
both byte-identical, 0 moved, the 2 new admissions this card's own examples
(`examples/loops/lend_fill.qcore`, `examples/strings/looped_fill.qcore`, each
run sanitized on the Linux arm; no Windows or Apple run claimed). Gate at
`4cc580bb` on the rebased tree: **4,701 / 0 / 9** over 237 summaries (+24,
the card's own file), fmt/clippy/probes clean, **140** Lean audits, Python
303 OK after the regeneration (red only on the two tripwires before it),
Clang present; the pre-rebase gate measured 4,662 / 0 / 9 and 138, and a
first run at `ac2a6b84` exited 101 against stale artifacts the first
mutation harness left (restore by `mv` without a `touch`; three rows of the
card's own file) plus one load-red constraint-16 row, both recorded and the
harness corrected (`docs/reports/loops-on-composed-routes/gate.json`, §10).
Evidence: `docs/reports/loops-on-composed-routes/README.md`.

**`CT42-CELL-ROUTE` (2026-09-18).** The socket protocol cell is routed from
source and CT42 is mapped — **46/50 → 47/50**, a grown denominator, which is
mapping and not conformance (constraint 23); sanitized-native 10 of 47 and
proof 3 of 47 unmoved. In the order the brief set: **(1) direction first** —
the deriver's two "unreachable for an admitted program" arms, which card
`STABILITY-REEVAL-3` measured assigning a singleton cell the unedited checker
judges *disjoint* from the binding's real connector, are refusals
(`SocketCellError::UnpairedAccept`/`UnpairedConnect`, `Q085` by condition
extension), each reached by a fixture through the new
`derive_cells_of_admitted` and shown red by mutation, the Lean `step` partial
to match with `an_admitted_accept_takes_an_existing_cell` stating the
direction, the corrected sentence published beside the frozen wave-31 report.
**(2) The T09 edit taken**: `FootprintEvent::DeriveDescriptorAlias`, handled
in the same arm as the declared alias and stamping no
`DeclaredProtocolCellIdentity`, audited as
`QCore.SocketCellRoute.a_derived_projection_stamps_no_declared_identity`
beside `derived_and_declared_aliases_decide_alike`. **(3) The route**:
`disjoint NAME NAME;` on `source.socket.v1` — one reserved word, census-clean
at zero QCore source occurrences across 525 files — the transport checker
deciding only liveness (`Q081`, `Q082`, `Q083` through a `Read` of each
custodian), the question deriving nothing and **erasing from emission**
(byte-identical C and observation with and without it), answered by the
unedited `DisjointFootprintsV1` checker at **`Q664`** under the `Concurrency`
pass, allocating no code and widening `Q080`/`Q081`/`Q085` (and `Q095`, see
below). **(4) CT42 mapped** on the `sequential` route under `reject`: the
case asks about the two ends of one accepted connection and is refused; five
controls (listener admitted and byte-identical to the twin, two connections
admitted, a datagram pair admitted — `CT42-CELL-UDP-QUEUE` re-affirmed with
its ground — a closed custodian refused `Q083`,`Q082`, a scalar refused
`Q081`); stage record `docs/reports/ct42-cell-route/ct_stage_evidence.json`
retiring the wave-35 bucket row by declaration; native and proof stages
`unassessed`, the C-twin arm still `CT-CTWIN-DESCRIPTOR`'s. **The finding**:
the HTTP composition hands unknown statements to the socket reader, so before
the card composed the cell check there `quirin check` **admitted** an HTTP
exchange asking about its own two ends with no judgement (measured, exit 0);
the driver now composes `check_socket_cells` over its re-lowered transport
and the same program refuses at `Q664`, `Q095`'s condition widened and stated.
Mutations 9 of 9 red as required and shipped. Retired: `CT42-CELL-UNROUTED`,
`CT42-MAP`, `CT42-CELL-FALLBACK-ARM-DIRECTION`, `CT42-CELL-DERIVED-PREMISE`,
`CT42-CELL-UDP-QUEUE`; registered: `SOCKET-CELL-COUPLING-UNPROVED`,
`CT42-CONTROLS-NATIVE-UNRUN`; amended: `CT42-CELL-PENDING-PHASE`; one entry
of the card's own withdrawn before merge on the superseded HTTP measurement.
Standing constraint **15p** landed. Gate and counts in
`docs/reports/ct42-cell-route/README.md` and its `gate.json`.

**`TSAN-ROWS-2` (2026-09-18).** Contention-forcing ThreadSanitizer schedules
on the checked task route, on the CT fixtures themselves, and the CT46
decision — `docs/reports/tsan-rows-2/README.md`. A second `-include` header
(`tests/support/thread_sanitizer.rs::SCHEDULE_FORCING`) rides the task
runtime's own `#ifndef QCORE_TASK_CREATE`/`QCORE_TASK_JOIN` seam — the seam
CT07's `refuse_second_spawn.h` already rides — parking every worker at
creation and releasing the cohort at the creator's next join, **together**
(every thread live until every body has run, an exit barrier) or
**last-created first, each body to completion**; emission is untouched
(`git diff --stat` over `crates/qcore-core/src/`, `compiler/src/` and
`examples/` empty; every subject's emitted C mentions neither harness and
still carries both hook defaults; the bytes the compiler was handed are read
back and asserted equal to the emitted C under every schedule). The subjects
are no longer twins: the write-lend flagship is read by both
`task_route_tsan.rs` and `write_lend_tasks_exercise.rs` from **one fixture
file** (`compiler/tests/fixtures/write_lend_tasks_exercise/disjoint_exercise.qcore`),
CT07's own case is lowered through `compile_files` and pinned as that flagship
minus its logger, and CT46's own case is a subject by its own path — clean
bound, stdout byte-identical to `interpret_mutex_program`'s observation, and
**unforceable by construction**: the mutex backend calls `thrd_create` bare,
so the header attaches to nothing there (no worker released, no schedule
line, measured), and the checked order keeps the creator in `thrd_join` for
the whole of the child's run. Rows: six task-route subjects × {unforced,
together, reverse} all clean with zero races and stdout equal to the
reference execution (the wave-37 rows read no stdout; these do), the realised
schedule read back from the harness's stderr trace and asserted. **The
finding runs the other way from the brief's expectation.** Every
emitted-program mutation — the overlap on CT07's case and on the flagship,
the join edge removed on `shared_readers` and on CT07's case — is caught
**20 of 20 under all three schedules** (`artifacts/catch-rates.json`), so no
mutation the unforced harness misses was found (`TSAN-FORCED-CATCH-POWER-EQUAL`);
what forcing changes is the report set (16 races unforced, 6 forced, under
`halt_on_error=0`, correcting wave 37's "two races" sentence with numbers) and
what it adds is executed-schedule coverage. Two asymmetries were found in the
forcing direction instead: the **first** header, built on a pthread mutex and
condition variable, made the reverse schedule **miss** the overlap — worker 1
woke by acquiring what worker 2 released after its write, an edge TSan could
see (`artifacts/catch-point-reverse-mutex-edge.log`,
`schedule_forcing.v1-pthread.h`) — so the shipped header is
`no_sanitize("thread")` throughout with an uninstrumented spinlock, and the
reproduction is a shipped mutation (`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`);
and a join deleted **without** opening the gate hides every race (0 of 20
forced, 20 of 20 unforced), pinned by test, which is why the join-edge
stand-in opens the gate under forcing (`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`).
Wave 36's `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not`
now asserts both halves of its boundary: unbound, both programs die at the
`0x18` shape (not any SEGV), with every asymmetry and the "both run" case a
`panic!` naming `TASK-ROUTE-TSAN-PTHREAD-BINDING`'s discharge; bound, both
run race-free with stdout equal to the reference. **CT46 decision** (the
second discharge arm of `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`, taken
and recorded, with proposed `why` text for the integrator): this route's
native evidence stays serial by construction. Mutations: 17 arms, every one
`RED-AS-REQUIRED` on its intended assertion, two of them moving the header
itself, harness and logs shipped (`artifacts/mutate.sh`, `mutations.json`).
No semantic surface moved, so no Lean; constraint 14 not engaged (no word, no
token); no new `Q` code; CT07's row gains a **citation**, not a stage flip;
no Windows or Apple claim (`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` stays).
Retired, each met word for word: `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`,
`WLT-EXERCISE-TSAN-TEST-SILENT-ARMS`, `CT07-TWO-THREAD-REPLAY-ORDER` (its
second arm, the TSan row landed on CT07's own fixture, with the boundary
restated as `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`). Registered:
`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`, `TSAN-FORCED-CATCH-POWER-EQUAL`,
`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`, `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`,
`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`, `CT46-NOTES-PREDATE-TSAN-ROW`;
`named_followups` 228 → **231**. Gate figures in the report's §7 and
`gate.json`.

**`AGGREGATE-OWNED-FIELD` (2026-09-18).** An IR card before a route card, on
base `e878b362`. The cycle-3 ownership IR now addresses an owner by a **path**
— `ownership::Place::{Name, Field}` with `FieldPath { base, field }`, one level
— and lands the four rules constraint 15k had recorded as missing, each in the
IR and none in a route: `Compose` moves each named buffer into its field (the
source is a use after move from then on); `MoveOut` is the partial move,
leaving a hole a second read refuses; `Drop` of an aggregate releases every
live owned field **in declaration order and nothing twice**, tolerating a
hole; `NormalPathLeak` names an unreleased field by its path; and `Copy` of a
record declaring an owned field refuses whatever its fields' state. Every
existing `OwnershipOp` variant is unchanged and a `String` owner is always
`Place::Name`, so a program that composes no aggregate builds the operation
list it always built — measured over the whole tree by a two-tree census
(`docs/reports/aggregate-owned-field/README.md` §3, §6) rather than argued.
Lean parity is `QCore.AggregateOwnedField` (25 theorems, registered in
`AuditAggregateOwnedField.lean`; headline
`drop_through_releases_exactly_the_live_fields` and `…nothing_twice`), with
`QCore.SourceRecord`'s scalar-record theorems unchanged and their prose amended
to say which half they are. On the route, `source.record.v1` admits a field
declared **`bytes`** — the acquisition's own reserved word; the task route's
`Buffer` names a different resource and was measured and rejected — moved in
at construction, standing at every custody position a buffer name does
(`RECORD.FIELD[I] = V;`, `let X = RECORD.FIELD[I];`, `extent(RECORD.FIELD)`,
`drop RECORD.FIELD;`), moved out by the field read, and released by
`drop RECORD;`; a record of scalars is exactly what `AGGREGATES-V1` measured.
**Zero words reserved and no code allocated**: `Q055` is **narrowed** (a buffer
at a field refuses only at an `i32` field — the deliberate lift), `Q051`–`Q054`
widen by condition extension stated in §L21, `Q103` gains vocabulary no
sequential program reaches (`OWNED-FIELD-SEQUENTIAL-UNREACHED`), `Q100` is
untouched. Two shapes are refused by name rather than modelled:
`OWNED-FIELD-LEND-WINDOW` and `OWNED-FIELD-LOOP-BODY`. Emission holds the
buffer pointer as a struct member, the designated initializer *is* the move,
and the whole-record release frees the members still held in declaration
order; constraint 15k's "no acquisition, no prelude" argument is re-derived
with the owned field enumerated (a construction moves, it does not acquire).
The published example `examples/records/owned_field.qcore` runs clean under
ASan/UBSan/LSan and matches its reference execution byte for byte
(`compiler/tests/aggregate_owned_field.rs`, 22 tests; `examples_suite`
extended). `AGGREGATE-OWNED-FIELD` retired by exact name;
`AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`, `AGGREGATE-TASK-CAPTURE`,
`OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY` and
`OWNED-FIELD-SEQUENTIAL-UNREACHED` registered with discharge conditions.
Constraint 15k amended deliberately. Report:
`docs/reports/aggregate-owned-field/README.md`.

**`CT07-TWO-THREAD-PROOF-LEG` (2026-09-18).** CT07's `proof` stage moves
`unassessed` → **executed** on a named host-unconditional test, and the proof
numerator moves **3 of 46 → 4 of 46** — evidence about one case's artifact,
never conformance (constraint 23); no other count moves (mapped 46, sanitized
10, no unit or function status, no corpus number). Base `e878b362`. **The
theorem bound** is wave 31's
`QCore.WriteLendTasks.no_byte_has_two_write_authorities` — for every ledger
state a spawn/join interleaving can *reach* and every byte, at most one write
authority — instantiated on CT07's own two-thread ledger state as
`QCore.Ct07LeaseLedger.ct07_no_byte_has_two_write_authorities`; wave 36's
`no_byte_is_written_by_two_parties` is instantiated on the same state, with
the artifact's two byte writes (writer 1 byte 0, writer 2 byte 2) shown
admitted against it — a consequence checked on top of the lease binding, not a
second binding route. **The binding is a new pattern, through the ledger**:
the CT01/CT37/CT03 route is closed by decision, `reference_trace` refusing
this run's `BufferLeased` event under constraint 15n as amended
(`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`), and it was not touched and the
alphabet did not grow. Instead the new Lean module adds one general theorem,
`replay_reaches` — whatever state a successful replay of admitted opens and
closes produces from the empty ledger is `Reachable` — and
`compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape`
lowers `fixtures/ct/CT07/case.qcore` through the real pipeline, reads the two
lease captures out of the checked task IR, runs the reference execution,
transcribes its lease opens and closes into the Lean `Step` vocabulary
(`[lease 1 [0,2), lease 2 [2,4), reclaim 1, reclaim 2]`, holders the
children's own identities, exactly `ct07Steps`), replays them with the
clause-for-clause transcription of `replay`, requires `Some` (the `Reachable`
witness), recomputes the kernel-decided pin `ct07_ledger_digest =
8876998389998880787` from the artifact with wrapping `u64` arithmetic, checks
the scenario shape `dischargesDisjointHalves` (two leases, two distinct
holders, every payload byte under **exactly one** write authority — the
theorem's own `writeAuthorities`, met with equality where the theorem proves
`≤ 1`), and drives the production `ScopedTaskLedger` through the same steps,
requiring its `(holder, range)` set to agree after every step and
`LeaseRange::overlaps` (`ownership::axes_overlap`) to agree with the
transcribed `overlaps` on the artifact's ranges — the `Reachable` term ↔
ledger-state correspondence the residual's discharge named. **Non-vacuity,
shipped** (`docs/reports/ct07-two-thread-proof-leg/scripts/mutate_ct07_proof_leg.py`,
`mutations.log`): the three fixture mutations are each measured at **two**
catch points — overlapping halves `[0,3)`/`[2,4)` and the empty half `[2,2)`
are caught by the **checker** (`Q600`, the ledger's `OverlappingLeases`; the
capture's "must cover at least one byte") before any artifact exists, and the
procedure fed their would-be steps directly refuses each at the clause the
theorem names (disjointness; nonemptiness); the single writer is **admitted**
by the checker — a reachable state the theorem holds of — and refused by the
**binding** at the steps assertion, the shape (bytes 2 and 3 under no
authority) and the pin; six rule mutations of the transcription each redden
the named test (one shows the shape's `== 1` strictly stronger than the
theorem's `≤ 1`); two Lean mutations fail `lake build` by `decide`. Lean audits
**137 → 138** (`AuditCt07LeaseLedger.lean`, 24 declarations, `propext`/
`Quot.sound` only). `PROOF_GATED_CASES` and `RUNNER_REGISTRY` gain the CT07
row; the manifest's CT07 proof row, the fixture's `stages_executed`, the
acceptance record's generated fields and `derived_from`, and both
`EVIDENCE_FILES` copies move in lockstep, the wave-37 `ct-lane-5` record
superseded from this card's own record (constraint 21). No `Q` code, no
keyword, no lexer change (census: +0, nothing to census). Retired, each
verified registered and met word for word: `CT07-TWO-THREAD-PROOF-LEG`,
`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`; registered:
`CT07-PROOF-LEG-F02-PREMISE-PIN` (the leg's last assertion re-measures that
`AdmittedCausalOrder::derive` refuses CT07's run, so a later alphabet growth
turns the leg red by design and must remove the pin deliberately);
`named_followups` **228 → 227**. Gate at tip `81f783ce`: **4,644 passed / 0
failed / 9 ignored** over 235 summaries and 233 targets (+5 on the wave-38
close's 4,639), fmt/clippy clean, both probes 0, **138** Lean audits, Python
303 OK after the `--gate` regeneration (the detached run's own Python step red
on the two regeneration tripwires, `228 != 227` and `137` against `138`,
recorded in `gate.json`), Clang present, provider 122 carried.
Report: `docs/reports/ct07-two-thread-proof-leg/README.md`.
