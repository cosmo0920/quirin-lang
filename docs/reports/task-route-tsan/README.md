# TASK-ROUTE-TSAN — ThreadSanitizer on the checked task route (wave 37)

**Headline.** The fault is **not QCore's**, and the measurement that settles it
is a fifteen-line C file containing no QCore code at all. ThreadSanitizer's
runtime ships an interceptor for `pthread_create` and **none for
`thrd_create`**; on glibc 2.34+ `thrd_create` reaches `start_thread` without
passing through that interceptor, so the new thread is never registered with
TSan and the first instrumented event in it faults at
`__tsan_func_entry+19` — `mov 0x18(%rdi),%rax`, a read of offset `0x18` of a
null `ThreadState *`. That is the entire content of `TASK-ROUTE-TSAN-SEGV`, and
it explains the address wave 36 recorded without having to attribute anything
to QCore's emission, allocator or mapping.

**The route now has TSan evidence.** Binding the two C11 thread primitives to
pthread through `-include` — changing **no emitted byte** — makes every subject
run clean, and the row is non-vacuous: mutating the emitted C so the two
workers write the *same* byte makes TSan name the race inside
`qcore_buffer_byte_write`. The write-lease card's C11 3.14 claim, which wave 36
could carry only by argument and by ASan/UBSan/LSan, is now backed by a
thread-race detector.

`TASK-ROUTE-TSAN-SEGV` is **retired**; four narrower residuals replace it, one
of them a process finding this card did not go looking for (§7.1).

## 1. The finding as inherited

`WRITE-LEND-TASKS-EXERCISE` (wave 36, `docs/reports/write-lend-tasks-exercise/README.md`
§5.4) measured that TSan killed its flagship, wave 31's held-only lease program
and the published `examples/threads/two_workers` at the same address (`0x18`)
and, in its runs, the same pc, and concluded — correctly — that the failure was
common to the route rather than particular to any one program. It recorded a fourth row, "a minimal
`thrd_create`/`thrd_join` program — **runs clean**", which is why the diagnosis
did not follow immediately: that row is a **false negative**, and §2.2 says
why.

That card declined to set the residual's priority. This card is that priority
being taken.

## 2. Diagnosis by measurement

### 2.1 The charter's hypothesis, refuted

The charter's stated first hypothesis was a fixed-address mapping colliding
with TSan's reserved shadow/app ranges — the shape a crash at one fixed address
across unrelated programs usually has. **Refuted, and cheaply.** A collision
with a reserved range faults at a *large* address; `0x18` is 24 bytes past
null, which the sanitizer itself names (`Hint: address points to the zero
page`). No `mmap` is reached: neither `os_endpoint` nor `MAP_FIXED_NOREPLACE`
appears anywhere in a program the task route emits — the emitted runtime's only
allocation primitives are `calloc`/`malloc`/`free` behind
`QCORE_TASK_CALLOC` / `QCORE_BUFFER_MALLOC`. The Linux mapping observations in
`CLAUDE.md`'s platform records belong to other subsystems and are untouched by
this card.

### 2.2 What it actually is

`artifacts/probes/min.c` — no QCore code, a `struct`, `thrd_create`, `thrd_join`,
and a thread body holding exactly one instrumented memory access:

```
ThreadSanitizer:DEADLYSIGNAL
==439589==ERROR: ThreadSanitizer: SEGV on unknown address 0x000000000018 (pc 0x598496bd4243 ...)
==439589==The signal is caused by a READ memory access.
==439589==Hint: address points to the zero page.
```

The identical address. Under `gdb` with `handle_segv=0`:

```
Thread 2 "min" received signal SIGSEGV, Segmentation fault.
0x0000555555630243 in __tsan_func_entry ()
#0  0x0000555555630243 in __tsan_func_entry ()
#1  0x000055555563e455 in body (p=...) at min.c:5
#2  0x00007ffff7ca3c7b in start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:451
#3  0x00007ffff7d375cc in __GI___clone3 () at .../clone3.S:78
=> 0x555555630243 <__tsan_func_entry+19>:	mov    0x18(%rdi),%rax
```

`%rdi` is the `ThreadState *` `__tsan_func_entry` was handed; it is null,
because the thread was never registered. Two further measurements close it:

* `nm` on the built binary finds `___interceptor_pthread_create`,
  `__interceptor_pthread_create` and `__interceptor_trampoline_pthread_create`,
  and **no `thrd_create` interceptor of any kind**, while the binary's dynamic
  imports include `thrd_create@GLIBC_2.34`.
* The same program written against `pthread_create` (`artifacts/probes/minpthread.c`) runs
  **clean**, printing its result.

Toolchain: Ubuntu clang 21.1.8, Ubuntu GLIBC 2.43-2ubuntu2.4, x86-64.

**Why wave 36's minimal program ran clean.** `artifacts/probes/minempty.c` — a
`thrd_create` program whose body is `{ (void)p; return 0; }` — exits 0 under
TSan. Its body contains no memory access and no call, so clang instruments it
not at all: there is no `__tsan_func_entry` to fault. The minimal program was
too minimal, and the row it produced was a false negative about the toolchain
rather than a fact about QCore. Recorded here because the same trap is easy to
fall into again.

### 2.3 The fault reproduces on the route, at one instruction

Each of the five subjects below, built `-fsanitize=thread` with no binding,
dies at fault address **`0x18`** with exit status 66
(`artifacts/tsan-runs.json`).

One accuracy note, because it is the kind of thing that is easy to overstate
and this card is partly about an overstatement. The raw **pc is not literally
identical across every run**: four subjects reported `0x555555630243` and
`core_context` reported `0x56b4f08d2243` in the same sweep. That is not a
second failure mode — it is where ThreadSanitizer's runtime happened to load.
Both pcs have page offset `0x243`, and the symbolized location is the same
instruction in both, `__tsan_func_entry+19`. What is identical across all five
is the **fault address and the faulting instruction**; the pc is identical only
when the runtime lands at the same base, which it usually but not always does.

## 3. Fix or bound

There is **nothing in QCore to fix**. Emitting C11 `thrd_create` is correct and
deliberate: the route must also emit for MSVC, and `c11_threads_compat.h`
exists precisely to carry that name onto Windows and Apple. Changing the
emitted primitive to `pthread_create` would weaken portability and move every
published program's bytes to work around a gap in a sanitizer runtime. So the
answer is the other one the charter allows: **work around it at the harness,
and bound what the resulting row can say.**

`tests/support/thread_sanitizer.rs` carries `PTHREAD_BINDING`, a header
prepended through `-include` that replaces `thrd_create`/`thrd_join` at their
**call sites** with pthread-backed equivalents. `<threads.h>` is included
*before* the macros are defined, so the real declarations are never rewritten.
One binding covers both emitters, which reach the same two names from different
directions: the task runtime through its own documented
`#ifndef QCORE_TASK_CREATE` hook — whose default *is* `thrd_create` — and the
mutex-invariant backend through a literal call.

**Emitted C changed by zero bytes.** No file under `crates/qcore-core/src/`,
`compiler/` or `examples/` that contributes to emission is touched by this
card; the C the compiler produces is compiled verbatim and the binding arrives
entirely through a compiler flag, the same mechanism the existing
allocation-tracking harness uses for `QCORE_BUFFER_MALLOC`. There is therefore
no emitted-program byte diff to state, and §6 records the check that confirms
it rather than an argument that it must be so.

### 3.1 What the binding costs, stated rather than elided

The program TSan observes differs from the published program in **which symbol
spawns the thread**. It is not a different concurrency structure:

* glibc implements `thrd_create` by calling `pthread_create`'s implementation.
  This is not read from documentation — it is visible in the faulting backtrace
  in §2.2, where the frame below the unbound `thrd_create`'d worker is
  `start_thread (arg=<optimized out>) at ./nptl/pthread_create.c:451`. Both
  builds reach the same `start_thread` on the same `clone3`.
* Every happens-before edge TSan reasons about here — thread creation and
  thread join — is therefore the same edge in both builds.
* The replacement bodies are not invented for the test. They are the
  pthread-backed `thrd_create`/`thrd_join` that the shipped
  `crates/qcore-core/src/c11_threads_compat.h` already compiles on Apple hosts,
  so every task program in this repository is already built this way on one
  supported platform.
* The binding's one representation assumption — that `thrd_t` and `pthread_t`
  are one representation — is a `_Static_assert` in the header, not a comment.

What the row therefore **cannot** speak for: the `thrd_create` call itself, the
Windows thread arm, and the Apple arm. Registered as
`TASK-ROUTE-TSAN-PTHREAD-BINDING`.

## 4. The TSan row and its non-vacuity

`compiler/tests/task_route_tsan.rs` (8 tests) and one added test in
`crates/qcore-core/tests/unit/mutex_c_emit.rs`. Every one is
`#[cfg(target_os = "linux")]`, and on any other platform the file contributes
**no rows at all** rather than passing vacuously — the
`CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS` shape this card was told not to
repeat. That is stated in the file's own header, not only here.

| Test | What it establishes |
|---|---|
| `a_program_with_no_qcore_in_it_dies_the_same_way` | The attribution. A C program this repository does not emit shows the `TASK-ROUTE-TSAN-SEGV` fault |
| `the_same_program_through_pthread_runs_clean` | The other half: the same program spawned by `pthread_create` runs clean |
| `an_uninstrumented_body_is_why_the_minimal_probe_ran_clean` | Why wave 36's control was a false negative |
| `every_subject_dies_unbound_at_the_one_fault` | The inherited symmetry, over all five task-route subjects |
| `every_subject_runs_clean_under_thread_sanitizer_when_bound` | **The row.** All five run clean, zero races |
| `overlapping_writers_are_reported_…` | **The row's non-vacuity**, at the byte write |
| `removing_the_join_edge_is_reported_so_the_clean_rows_are_not_blind` | **The row's non-vacuity, at the route's own synchronization** |
| `the_binding_changes_no_emitted_byte` | The emitted C never mentions the binding and still defaults its hook to `thrd_create` |
| `the_fork_join_crossing_runs_clean_under_thread_sanitizer` | CT46's emitted crossing, bound and unbound |

### 4.1 Non-vacuity, twice over

**The row's own.** The checker refuses two workers writing the same byte, so
reaching that program means breaking the emitted C after the fact: both
workers' writes are rebased onto absolute byte `0` instead of onto their own
lease bases — a one-line substitution the test asserts is present before making
it. ThreadSanitizer then names the race, and names it *where it is*:

```
WARNING: ThreadSanitizer: data race (pid=478805)
  Write of size 1 at 0x720400000008 by thread T2:
    #0 qcore_buffer_byte_write  overlap.c:499:48
    #1 qcore_task_fn_0          overlap.c:595:5
    #2 qcore_task_trampoline    overlap.c:534:22
  Previous write of size 1 at 0x720400000008 by thread T1:
    #0 qcore_buffer_byte_write  overlap.c:499:48
  Location is heap block of size 16 at 0x720400000000 allocated by main thread:
    #1 qcore_buffer_new         overlap.c:465:35
```

Both sides are worker threads, the location is the buffer `qcore_buffer_new`
allocated, and the frames are the emitted helper's own. Without this row, the
clean row would be satisfied by a build in which TSan saw nothing at all.

**The second non-vacuity, and it is about the route rather than one write.**
The clean rows could in principle be clean because TSan sees the
happens-before edges and is satisfied, or because it sees nothing at all in
these programs. `overlapping_writers_…` rules the second out for one byte
write; `removing_the_join_edge_…` rules it out for the route's own
synchronization. `shared_readers` is the subject because it is the strongest
one: two workers *and the parent* read one buffer while the children hold their
shares, and the parent **writes** it after both joins — a write that is safe
exactly because the join orders it after the readers. Replacing the emitted
`QCORE_TASK_JOIN` with a sleep, keeping every other statement, removes that
edge and nothing else, and TSan reports **two** races, the first naming
`qcore_task_collect` reading completion data.

That mutation also demonstrates the instrument's character, which is why the
test says so in its own doc comment: the 50 ms sleep means the worker has
certainly *finished* before the parent reads, so the accesses never physically
overlap — and TSan reports them anyway, because it detects a **missing
happens-before edge** rather than an observed interleaving. That is what makes
the clean rows worth more than a timing test, and it is shown rather than
argued.

**Every arm's.** All nine new arms were driven red by mutation and restored
with mtime advanced (`artifacts/mutate.sh` and `artifacts/mutate2.sh`, verdicts
in `artifacts/mutations.json`, all nine `RED-AS-REQUIRED`):

| Arm | Mutation | Verdict |
|---|---|---|
| `a_program_with_no_qcore_in_it_dies_the_same_way` | bind its threads | red |
| `the_same_program_through_pthread_runs_clean` | spawn it with `thrd_create` | red |
| `an_uninstrumented_body_…` | give the body one memory access | red |
| `every_subject_dies_unbound_at_the_one_fault` | bind them | red |
| `every_subject_runs_clean_…_when_bound` | unbind them | red |
| `overlapping_writers_are_reported_…` | do not force the overlap | red |
| `the_binding_changes_no_emitted_byte` | expect a hook default the route does not emit | red |
| `removing_the_join_edge_…` | do not remove the join | red |
| `the_fork_join_crossing_…` | unbind the bound run | red |

## 5. What TSan says about each subject

All measurements Ubuntu clang 21.1.8 / GLIBC 2.43 / x86-64. Per-subject exits, fault addresses and race counts are archived in
`artifacts/tsan-runs.json`, derived from the run files rather than hand-copied;
every unbound subject exits 66 at fault address `0x18`, every bound subject
exits 0 with zero race reports.

| Subject | Threads live at once | Unbound | Bound | What the clean run is worth |
|---|---|---|---|---|
| `examples/threads/two_workers` | **2 workers** (both started before either join, verified in the emitted C) | SEGV `0x18` | clean | Two real OS threads overlapping, with a detector watching. First TSan evidence this published example has had |
| `examples/threads/shared_readers` | **2 workers plus the parent**, all three reading one buffer at once | SEGV `0x18` | clean | The strongest row here, and by more than the others. Three threads read one heap object concurrently — the parent's own `qcore_buffer_read` runs *while both children hold their 512-unit shares* — and the buffer is **written** (`qcore_buffer_write(buffer, 7)`) only after both joins. So this program contains a real write/read pair that is unsafe if and only if the join edge is missing, and TSan is silent: the counted read-share discipline's restoration of exclusive write access is backed by a detector, not only by the ledger that decided it |
| `examples/threads/owned_buffer` | 1 worker; parent works on a separate buffer meanwhile | SEGV `0x18` | clean | The transfer really is exclusive — the parent touches no byte of the transferred object while the worker holds it |
| `examples/threads/core_context` | 1 worker | SEGV `0x18` | clean | Owned handoff and explicit join |
| The write-lease disjoint exercise | **2 workers**, each writing one byte of one `int32_t` through its own lease | SEGV `0x18` | clean | **The claim wave 36 withheld.** Two threads writing distinct bytes of one scalar object are distinct memory locations under C11 6.2.6.1/4 and 3.14, so not a data race — and a race detector agrees, rather than the point resting on the emitted helper's comment |
| The same, mutated onto one byte | 2 | — | **race reported** | §4.1 |
| CT46's emitted crossing (`fork_join_trace`) | **1 at a time** — the checked trace serialises the child | SEGV `0x18` | clean | See below |

### 5.1 CT46: what TSan adds, and what it still cannot say

**Adds.** The crossing really does run a second OS thread that locks the mutex,
opens the invariant, writes the protected allocation, closes and unlocks, while
the creator waits — and a detector watching both threads is satisfied by the
happens-before edges the emission writes (spawn, the child's lock/unlock pair,
join). Before this row the child's write and the creator's post-join read were
known to agree with the reference execution and to be clean under
ASan/UBSan/LSan, none of which says anything about synchronization.

**Cannot say.** `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION` already owns the
sentence and this card restates it rather than quietly improving on it: the
checked trace is a total order whose child segment is contiguous between the
spawn and the join, so the creator's own critical section begins *after* the
join and the two **never contend** for the mutex. A clean TSan run over a
schedule with no contention is evidence about that schedule. It is not a claim
that the emitted locking would keep two contending threads race-free, and none
is made. The task-route subjects are different in exactly this respect — three
of the five have two workers genuinely live at once — which is why their rows
are worth more and are described separately above.

### 5.2 A note on strength, because a race detector is easy to over-read

TSan is a happens-before detector, so a clean run generalizes past the exact
interleaving observed: it reports a race between two accesses it saw with no
ordering edge between them, whatever order they happened to occur in. It still
only sees **code paths that executed**. Each program here is run once per test,
and the emitted programs' failure arms — a failed `qcore_buffer_new`, a failed
spawn — are never taken. Recorded as `TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`
rather than left for a reader to discover.

## 6. Gate

Standing per-change gate from `AGENTS.md`, run at **`dd07c480`** from
`artifacts/gate.sh` into a run-scoped worktree-private directory, with **every
step's exit captured to its own file and no pipe in the command**. The
submodule init ran first (exit 0, `third_party/libevent` at `d82464a2`).
`df -h .` before the run: **81 GB free** — above the 60 GB threshold, so the
full workspace gate rather than crate-scoped checks. Machine figures in
`gate.json`.

| Step | Exit | Measured |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **0** | **4,620 passed / 0 failed / 8 ignored**, 232 result summaries |
| `cargo fmt --all -- --check` | **0** | clean |
| `cargo clippy --all-targets -- -D warnings` | **0** | clean |
| `cargo run … --example reevaluation_probe` | **0** | |
| `cargo run … --example reevaluation_followup` | **0** | |
| `python3 scripts/check_lean_audits.py` | **0** | **135** audits passed |

### 6.0 Two runs, and the first one caught something

The gate was run **twice**, and the first run is reported rather than
discarded. At `7892d8dc` every step was 0 **except `cargo fmt --all -- --check`,
which exited 1** — on two rustfmt violations in this card's own new
`tests/support/thread_sanitizer.rs`. That is the gate doing its job on this
card, so it is recorded here and in `gate.json`'s `earlier_run` rather than
quietly fixed. The repair (`dd07c480`) is whitespace only, inside a
`#[cfg(target_os = "linux")]` helper, and the **whole gate was re-run** rather
than the one step — so the table above is a single measurement of a single
tree, not a splice of two.

The test figure was **identical across both runs** (4,620 / 0 / 8), which is
itself worth recording: none of constraint 16's five load-sensitive rows flaked
in either run, *despite* another session running its own full gate on this
machine for part of both (load average peaked near 33). No row needed isolated
re-verification, and none is claimed to have had one.

### 6.1 The zero-byte-change check, run rather than argued

No file contributing to emission is touched by this card. The claim is checked
three ways rather than asserted: `git diff --stat` over `crates/qcore-core/src/`,
`compiler/src/` and `examples/` across the whole card is **empty**;
`the_binding_changes_no_emitted_byte` asserts each published example's emitted
C never mentions the binding and still carries `QCORE_TASK_CREATE thrd_create`;
and the binding reaches the compiler only through an `-include` flag, so a
build without that flag cannot see it.

### 6.2 Constraint 14 and the diagnostics namespace: both vacuous here

**+0 reserved words, +0 tokens, +0 `Q` codes**, and vacuously so rather than by
a census: this card adds no source-language construct, touches no lexer, no
parser, no checker and no diagnostic. It is a diagnosis, a test harness and
test rows. Listed for the wave's ledger so the card appears in the "does not
engage this constraint at all" line rather than being absent from it.

## 7. Residuals

`named_followups` **171 → 174** (+4 −1).

### Retired

**`TASK-ROUTE-TSAN-SEGV`** — verified registered before being counted. Its
discharge condition, quoted from the entry: *"identify the faulting construct,
fix it or work around it, and then record what TSan says about two workers
writing disjoint bytes of one payload — either a clean run … or a reported
race, which would be a finding."* Met word for word: the construct is
`__tsan_func_entry` reading a null `ThreadState *`, reached because TSan has no
`thrd_create` interceptor (§2); it is worked around by the pthread binding
(§3); and what TSan says is **a clean run** (§5), with a race reported the
moment the two writers are forced onto one byte (§4.1).

Two of the retired entry's incidental claims were **wrong**, and are corrected
here rather than left to propagate: that the fault is *"in what this route
emits and not in the toolchain's C11 threads support"*, and that *"the offset
`0x18` is the arguments field of `struct qcore_task_start_record`, reached
through a near-null record pointer in `qcore_task_trampoline`"*. The offset is
in TSan's own `ThreadState`; the record pointer is fine. Both readings followed
from the false-negative control, not from carelessness.

### Registered

1. **`TASK-ROUTE-TSAN-PTHREAD-BINDING`** — every TSan row in this repository
   runs a pthread-bound build, not the published thread primitive. §3.1 has the
   argument for why the concurrency structure is unchanged and the list of what
   the rows cannot speak for. *Discharge:* re-run with `bind_threads=false` once
   a toolchain ships a `thrd_create` interceptor — which
   `a_program_with_no_qcore_in_it_dies_the_same_way` will announce by going red
   on the day it happens — or land a pthread emission mode and state its
   portability cost.
2. **`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`** — a clean detector run is evidence
   about the schedules that executed; the failure arms are never taken. §5.2.
3. **`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN`** — Linux arm only, gated honestly,
   contributing no rows elsewhere. No Windows or Apple claim is made. On the
   `MUTEX-SHARE-CROSS-EMISSION-WINDOWS-RUN` / `WRITE-LEND-EXERCISE-WINDOWS-RUN`
   precedent.
4. **`TSAN-DIAGNOSIS-WAS-LOST-THEN-CONTRADICTED`** — the process finding, §7.1.

### 7.1 The finding this card did not go looking for

`docs/reports/remaining-t04/README.md` §5.1 already said, waves earlier and in
these words, that *"ThreadSanitizer does not intercept C11 `thrd_create`"* and
that a `-fsanitize=thread` build of **any** emitted task program dies at
`0x18`. It also named the remedy this card took: *"adding a pthread emission
mode purely for race testing, is recorded as open."*

That open item was phrased as prose and **reached no tracker** — the
`--residuals` half of the reconciliation defect `CLAUDE.md`'s close rule
describes, in an instance the sweep's own arm could not have caught, because
the sweep finds residual *names* and this was a sentence.

The cost was not only delay. Wave 36 re-measured the symptom, added a control
that ran clean, and concluded the **opposite** attribution, which is how
`TASK-ROUTE-TSAN-SEGV` came to be written against the route. And
`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`'s own text cites `remaining-t04` for
the correct diagnosis — so both readings were live in the tracker
simultaneously and nothing compared them. This is a `STABILITY-REEVAL` item of
kind 3 (a semantic claim that moved with no residual indexing it) found inside
this card's own subject, and it is published rather than smoothed over.

## 8. Lean

**None, and the ground is stated rather than assumed.** The card-completion
standard asks for audited Lean **where a semantic surface moves**. Nothing
moved: no checker rule, no model, no judgment, no diagnostic, no emitted byte
(§6.1). The Rust added is a test harness and test rows; the C added is a
compile-time binding that exists only under a flag no shipped build passes. The
one correspondence a reader might expect Lean for — that the pthread-bound
build and the published build have the same happens-before structure — is not a
statement about any QCore model; it is a statement about glibc, and §3.1
supports it by measurement (the faulting backtrace showing `thrd_create`'s
worker sitting directly on `start_thread`) rather than by proof.

## 9. Integrator reconciliation list

1. **Apply the `named_followups` delta as landed:** 171 → **174**, +4 −1.
   AGENTS.md's count line is already updated in this card's commit; the four
   registered ids and the one retirement are listed in §7. The retirement was
   verified registered before being counted.
2. **Proposed amendment text for `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`.**
   That entry's *substance* — serial schedule, no contention evidence — is
   **entirely unchanged and restated** by this card's CT46 test in its own
   words. Only one parenthetical is now false. Replace, in its `why`:

   > TSan is not run anywhere in this repository and is measured non-functional
   > on emitted task programs (docs/reports/remaining-t04/README.md: it does not
   > intercept C11 thrd_create and a -fsanitize=thread build dies with SEGV), so
   > the usual instrument is unavailable here

   with:

   > TSan now runs on this backend's own emitted crossing
   > (docs/reports/task-route-tsan/README.md; remaining-t04's diagnosis was
   > right and card TASK-ROUTE-TSAN worked around it with a pthread binding that
   > changes no emitted byte), and reports nothing — but over a schedule in
   > which the creator and child never contend, so the instrument is available
   > and the contention question is still open

   Land it in these words or record `declined` with the ground; do not soften
   it while reconciling.
3. **Do not treat this card's rows as a CT stage change.** No CT row, manifest
   entry or stage evidence is touched. `CT07-TWO-THREAD-REMAP`, when it runs,
   **gains TSan evidence it can cite** for the two-thread write-lease
   scenario — `compiler/tests/task_route_tsan.rs::every_subject_runs_clean_under_thread_sanitizer_when_bound`
   and its non-vacuity sibling — and that is a citation, not a stage flip;
   a grown proof or sanitized numerator is not this card's to move
   (constraint 23).
4. **Wave 36's pinned test is untouched in its assertions.**
   `thread_sanitizer_cannot_run_this_routes_programs_lease_or_not` still runs,
   still passes, and still measures a true thing (unbound, TSan cannot run
   these programs). Only its doc comment gained a pointer saying which of its
   sentences this card superseded. Nothing was deleted to make a test pass.
5. **Constraint 14's "does not engage" enumeration.** Add `TASK-ROUTE-TSAN` to
   it (§6.2: +0 words, +0 tokens, +0 `Q` codes, vacuously — no lexer, parser,
   checker or diagnostic is touched). Deliberately **not** edited here: that
   bullet's enumeration currently stops at wave 35, so wave 36's cards are
   already missing from it, and adding one wave-37 card in front of that gap
   would make the list read as current when it is not. The wave-35 close's own
   finding (`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`) is about exactly this
   failure mode, which is why this card declined to repeat it and is saying so
   instead.
6. **A candidate for the close's own agenda, not an amendment:** §7.1's finding
   suggests the reconciliation sweep's `--residuals` arm cannot see open items
   written as prose. Whether to widen it is a decision for the close; this card
   records the instance and the cost rather than proposing a tool change it did
   not measure.
