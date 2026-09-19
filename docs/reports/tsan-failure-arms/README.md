# `TSAN-FAILURE-ARMS` — a ThreadSanitizer release trigger the join does not own, the task programs' failure arms, and an enforced invisibility property (wave 41)

Base `463fa6e8` (= `origin/main` at start). Linux/x86-64/glibc only; **no Windows
or Apple claim is made anywhere in this card** (`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN`
stands unchanged). Every row is a pthread-bound build
(`TASK-ROUTE-TSAN-PTHREAD-BINDING` stands unchanged).

**Headline.** The forcing header gains a second release trigger — a per-program
cohort count the row supplies — and the gate-closed reproduction wave 39
measured at **0 of 20** forced is now caught **20 of 20** under both forced
schedules, with wave 39's own 0-of-20 kept in the same test as the control arm.
Four failure-arm rows run the two subjects the card names over a refusing
`QCORE_TASK_CREATE` and a NULL-returning `QCORE_BUFFER_MALLOC`, clean, with the
arm that executed asserted from the program's own stdout and the stdout compared
byte for byte against the reference run scripted for the same outcomes —
**20/20 on every one of the twelve arm × schedule cells**. The forcing header's
invisibility to ThreadSanitizer is enforced by two committed tests rather than
documented. Twelve mutations, every one `RED-AS-REQUIRED`. **No emitted byte
moves**: the diff over every emitting source against the base is empty.

## 1. What is true today (re-derived at the base, not inherited)

* `TSAN-ROWS-2` (wave 39) landed forced-schedule rows over six task-route
  subjects, clean under `unbound-SEGV / bound-unforced / together / reverse`,
  stdout byte-identical to the reference execution. Re-derived here: at the base
  commit `cargo test -p qcore-compiler --test task_route_tsan` is 17 passed /
  1 ignored, and the wave-39 catch-rate cells reproduce exactly (§5).
* Its registered boundaries that this card is the subject of:
  **`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`** (no row takes a task program's failure
  arms, and schedule forcing cannot reach them),
  **`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`** (the gate opened at the creator's
  join, so deleting the join hid every race — 0/20 forced, 20/20 unforced),
  **`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`** (the shipped header is
  `no_sanitize("thread")` with an uninstrumented spinlock, because a
  visible-synchronization header made the reverse schedule *miss* an overlap),
  plus `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED` and
  `TSAN-FORCED-CATCH-POWER-EQUAL`, which this card does not engage.
* `STABILITY-REEVAL-4` (wave 40) measured the order of attack — **the harness
  breaks first**, because a failed spawn may never reach the join at which the
  gate opens — and registered `TSAN-HEADER-INVISIBILITY-UNENFORCED`: nothing in
  the workspace read the header's text, so the invisibility property had no
  mechanism.

## 2. The release trigger the join does not own

### 2.1 What it is

`tests/support/thread_sanitizer.rs`'s `SCHEDULE_FORCING` gains
`QCORE_FORCED_COHORT`, the row's own count of the workers a program parks in one
cohort. Inside the create hook, under the header's existing uninstrumented
spinlock and immediately after the new worker is registered:

```c
if (QCORE_FORCED_COHORT > 0 && qcore_forced_parked() >= (size_t)QCORE_FORCED_COHORT)
    qcore_forced_open_gate();
```

`qcore_forced_parked()` counts created-but-unreleased workers. The gate opening
is the **same function** the join trigger calls, so both release policies
(together with its exit barrier; reverse, last-created first, serialized) are
untouched — only *when* the gate opens changes. The counter formerly named
`qcore_forced_joins` is renamed `qcore_forced_gate`, because it now counts gate
openings rather than joins.

Three properties are deliberate:

1. **The join trigger is retained, not replaced.** A cohort count that is never
   reached — which is exactly what the spawn-refusal arm does — still releases at
   the first join. `0` selects wave 39's join-only behaviour and stays reachable,
   which is what lets the old measurement live on as a control arm rather than as
   a memory.
2. **The count is the row's**, supplied per program; `SUBJECTS` already carried
   it (the third column, the workers a creator spawns before its first join).
   That it is read off the program's text rather than derived from the checked
   task IR is registered as `TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND`.
3. **A creation that fails after its cohort released it gives its seat back.**
   The rollback path now undoes the `cohort_size` increment and clears
   `released`, or a sibling's exit barrier would wait for a body that never runs.

### 2.2 The gate-closed reproduction, now caught

`a_join_deleted_is_caught_by_the_cohort_trigger` (replacing wave 39's
`a_join_deleted_without_opening_the_gate_hides_the_race_under_forcing`, whose
measurement it keeps) applies wave 39's mutation to CT07's own case: the emitted
`QCORE_TASK_JOIN` is replaced by the **unbound** 50 ms no-edge stand-in, named by
its function so the forcing header's gate-opening macro cannot reach it. The test
runs it at both triggers, under both forced schedules, in one place:

| trigger | schedule | workers released | race reported |
|---|---|---|---|
| `cohort 0` (wave 39's join-only gate) | together | **0 / 20** | **0 / 20** |
| `cohort 0` | reverse | **0 / 20** | **0 / 20** |
| `cohort 2` (this card) | together | **20 / 20** | **20 / 20** |
| `cohort 2` | reverse | **20 / 20** | **20 / 20** |
| — (unforced, either trigger) | unforced | 0 (no forcing) | **20 / 20** |

`artifacts/catch-rates.json`, keys `join-deleted-gate-closed/ct07-case @ cohort N
@ …`, 20 runs per cell. The `cohort 0` rows reproduce
`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`'s published 0/20 exactly.

### 2.3 The four existing schedules, unchanged

Every wave-39 cell was re-measured at **both** triggers so the table says what
the second trigger changed rather than leaving it to be inferred. At `cohort 0`
every wave-39 number reproduces: overlap on CT07's case and on the flagship
20/20 under all three schedules; join-edge-removal on `shared_readers` and on
CT07's case 20/20 under all three; and the `halt_on_error=0` race counts
reproduce wave 39's published **16 / 6 / 6** exactly
(`race_count_halt_on_error_0`). The clean rows
(`every_subject_runs_clean_under_each_forced_schedule`) run at the row's own
cohort and keep every wave-39 verdict — zero races, clean, `compiled == c`,
stdout byte-identical to the reference, and the asserted schedule shape.

**A cost, measured rather than elided.** At `cohort 2` the *join-edge-removal*
mutation loses catches: 0/20 on both subjects under `forced-together`, 18/20 on
`shared_readers` under `forced-reverse` (CT07's case stays 20/20 under reverse).
The workers are released at creation, run and finish long before the creator's
post-join access, and what ThreadSanitizer reports becomes a **thread leak**
rather than a data race. So the second trigger is an **addition for the arms that
need it, not a replacement**: the two committed wave-39 mutation rows stay at
wave 39's trigger and the other cohort is measured beside them rather than
asserted. Registered as `TSAN-COHORT-TRIGGER-CATCH-COST`.

### 2.4 Byte identity — the seam is `-include` only

Checked the four ways wave 39 checked it, plus a published hash table:

1. `git diff --stat 463fa6e8 -- crates/qcore-core/src/ crates/qcore-frontend/src/
   compiler/src/ examples/` is **empty** (`artifacts/emission-diff.txt`). The
   whole card's `git diff --name-only` against the base is three files:
   `compiler/tests/task_route_tsan.rs`, `tests/support/thread_sanitizer.rs`,
   `docs/reports/tsan-failure-arms/README.md` (plus this directory's new
   artifacts).
2. `the_binding_changes_no_emitted_byte`: for all six task-route subjects the
   emitted C is deterministic, mentions none of `qcore_tsan_`, `qcore_forced_`,
   `QCORE_TSAN_DROP_JOIN` and — added by this card — **`QCORE_TSAN_FAILURE`** and
   **`QCORE_FORCED_COHORT`**, and still carries both hook defaults
   `#define QCORE_TASK_CREATE thrd_create` and `#define QCORE_TASK_JOIN thrd_join`.
   The new entries are shown live by mutation `no_byte_change_new_symbols`.
3. All three headers reach the compiler only through `-include` flags, and the
   two new compile-time selectors only through `-D`; a build without them cannot
   see them.
4. Every forced **and every failure-arm** row asserts `run.compiled == c` — the
   translation unit read back from disk after writing is the emitted C.
5. `artifacts/emitted.sha256`: the sha256 of the emitted C for all seven
   subjects at the card's tip, produced by `measure/emit-subjects.sh` through the
   real `quirin emit`. Since (1) is empty these are the base's bytes by
   construction; the table is published so a reader can re-derive rather than
   infer.

## 3. The failure arms

### 3.1 How they are injected

A third `-include` header, `FAILURE_INJECTION`, placed **after** the forcing
header so it wraps whatever `QCORE_TASK_CREATE` then means (the binding's
pthread-backed `thrd_create`, or `qcore_forced_create`):

* `Failure::RefuseSecondCreate` — CT07's own `refuse_second_spawn.h` rule: only
  the **second** creation refuses, so the first worker is really created, really
  runs its body and is really joined on the recovery arm.
* `Failure::NullBufferMalloc` — `QCORE_BUFFER_MALLOC` returns `NULL`, so the
  entry's `qcore_buffer_new(0)?` takes its allocation-failure error edge. The
  define lands before the emitted prelude, whose own `#ifndef` is then untaken.

Neither is invented here; both are shapes the CT heap runner already executes
under ASan/UBSan. What is new is that they **compose with the pthread binding and
the forced schedules**, which is what makes a failure arm runnable under
ThreadSanitizer at all.

### 3.2 The rows, and which arm each executed

`every_failure_arm_runs_clean_and_says_which_arm_it_took`, over
{write-lend flagship, CT07's `case.qcore`} × {both arms} × {unforced, together,
reverse} = **twelve cells**, each asserting: zero races; `clean` (exit 0, no
`ThreadSanitizer:` line of any kind); `compiled == c`; the arm's markers present
and its counter-markers absent **in the program's own stdout**; stdout
byte-identical to the reference execution driven by the matching
`TaskExecutionScript`; and, when forced, the realised schedule shape for the
cohort that arm creates.

| arm | cohort | the program's own stdout says | reference script |
|---|---|---|---|
| refusing second `QCORE_TASK_CREATE` | 1 | **present** `QCORE_BUFFER_LEASE 1 1 0 2`, `QCORE_TASK_CREATE_FAILED 0`; **absent** `QCORE_BUFFER_LEASE 2 1 2 4`, `QCORE_BUFFER_BYTE_WRITE 2 1 2 9`, `QCORE_BUFFER_ALLOC_FAILED` | `spawn_outcomes: [true, false]` |
| NULL-returning `QCORE_BUFFER_MALLOC` | 0 | **present** `QCORE_BUFFER_ALLOC_FAILED 0`; **absent** every `QCORE_BUFFER_LEASE`, `QCORE_TASK_CREATE_FAILED`, `QCORE_BUFFER_BYTE_WRITE` | `allocation_outcomes: [false]` |

So the refusal arm is the one where the emitted program takes `qcore_block_6` —
the first worker's lease really taken out, the second creation refused, the
first task joined and the buffer dropped on the recovery path — and the
allocation arm is `qcore_block_2`, the entry's `?` edge, where no worker exists.
The assertion is on the program's output, never on the harness's intent; the
control row `without_the_injection_the_same_subject_takes_the_success_arm`
shows the same subject under the same schedule taking the success arm when
nothing is injected (both failure markers absent, the second lease present).

Measured over 20 runs per cell (`artifacts/catch-rates.json`, `failure_arms`):
every one of the twelve cells is **20/20 clean, 20/20 arm-named, 20/20
stdout-matches-reference**, on both subjects.

### 3.3 The arm that is not thread-live, named rather than dressed up

On this route the NULL-malloc arm creates **no thread**: both subjects allocate
only at entry, before any spawn, so the allocation-failure edge is a
single-threaded recovery path and ThreadSanitizer's clean verdict there is about
a single-threaded path. That is a real row (it is the arm executing, under TSan,
clean, byte-compared) and a weak one, and it is said in those words.
`TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED` records it; what would reach a
thread-live allocation failure is a program that allocates a buffer **after** a
spawn, which no admitted program on this route spells today — the nearest
existing shape is CT16's worker-allocation fixture, which is not a TSan subject.
The refusal arm **is** thread-live: worker 1 is created, released, runs its body
and is joined inside the arm.

## 4. The invisibility property, enforced

`TSAN-HEADER-INVISIBILITY-UNENFORCED` asked for either a textual tripwire plus a
doc line naming the behavioural guard, **or** the `hook_visible_to_tsan`
reproduction promoted to a workspace test. This card lands **both**, and neither
is `#[ignore]`d — both run in the standing gate.

**Half one, the shipped header's shape.**
`the_forcing_header_stays_invisible_to_thread_sanitizer` reads
`SCHEDULE_FORCING`'s text and asserts: the `QCORE_FORCED_UNINSTRUMENTED` macro
still expands to `__attribute__((no_sanitize("thread")))`; the header contains
none of `pthread_mutex`, `pthread_cond`, `pthread_barrier`, `pthread_rwlock`,
`sem_wait`, `mtx_lock`, `mtx_init`, `cnd_wait`, `cnd_signal`, `cnd_broadcast`;
and **every** `static` function of the `qcore_forced_` namespace carries the
attribute macro on its own line or the line above. The check is shown non-vacuous
by asserting it found at least eight definitions, and by two mutations
(`hook_visible_to_tsan`, `hook_mutex_reintroduced`).

**Half two, the property half one protects.**
`a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not`
builds the `hook_visible_to_tsan` mutation as a function over the shipped text —
`thread_sanitizer::with_visible_synchronization` — putting back exactly what
wave 39 removed: a pthread mutex for the bookkeeping lock, instrumentation on
every function, and a parked worker that learns of its release **by acquiring
that mutex**. Under `ForcedReverse` the releaser is the previous worker's own
`finish`, so ThreadSanitizer sees a release/acquire edge between the two workers'
byte writes and the overlap mutation goes **unreported**; the same text under the
shipped header is reported, in the next assertion of the same test.

Measured, 20 runs per cell (`artifacts/catch-rates.json`, `invisibility`):

| header | schedule | overlap reported |
|---|---|---|
| shipped (invisible) | together | **20 / 20** |
| shipped (invisible) | reverse | **20 / 20** |
| `hook_visible_to_tsan` | together | **0 / 20** |
| `hook_visible_to_tsan` | reverse | **0 / 20** |

Wave 39 measured the miss under reverse only; this card measures it under
`forced-together` as well, so the false negative is broader than wave 39
recorded. The committed test asserts the reverse cell (wave 39's own named
schedule) and the table publishes both.

**What makes this enforcement and not decoration:** every substitution in
`with_visible_synchronization` asserts its anchor appears **exactly once** in the
shipped header. A header edit that moves the lock, the park loop or the attribute
out of the shapes wave 39 measured makes the test panic rather than silently pass,
and mutation `visible_mutation_without_the_lock` shows the park-through-the-lock
is the mechanism: take the lock back out of the mutated park loop and the visible
header *reports* the overlap again, so the test goes red.

**What it still does not catch,** said plainly: a future forcing design that
synchronises visibly through some primitive not on the forbidden list and not in
the `qcore_forced_` namespace would pass half one. Half two is a measurement of
one mutation, not a quantifier over all visible designs. The residual
`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY` therefore stays registered, amended with
what now enforces it (§7).

## 5. Non-vacuity in both directions

### 5.1 The catch-rate table (`artifacts/catch-rates.json`, 20 runs per cell)

Produced by `measure_catch_rates_for_the_report` (`#[ignore]`d measurement
harness, extended by this card), run at the card's tip:

```
QCORE_TSAN2_MEASURE_DIR=measure/catch QCORE_TSAN2_REPEATS=20 \
  cargo test -p qcore-compiler --test task_route_tsan \
  measure_catch_rates_for_the_report -- --ignored
```
exit 0, 222.54 s (`artifacts/catch-rates.run.log`).

| mutation | trigger | unforced | together | reverse |
|---|---|---|---|---|
| overlap / ct07-case | cohort 0 | 20/20 | 20/20 | 20/20 |
| overlap / ct07-case | cohort 2 | 20/20 | 20/20 | 20/20 |
| overlap / disjoint-exercise | cohort 0 | 20/20 | 20/20 | 20/20 |
| overlap / disjoint-exercise | cohort 2 | 20/20 | 20/20 | 20/20 |
| join-edge-removed / shared_readers | cohort 0 | 20/20 | 20/20 | 20/20 |
| join-edge-removed / shared_readers | cohort 2 | 20/20 | **0/20** | **18/20** |
| join-edge-removed / ct07-case | cohort 0 | 20/20 | 20/20 | 20/20 |
| join-edge-removed / ct07-case | cohort 2 | 20/20 | **0/20** | 20/20 |
| join-deleted-gate-closed / ct07-case | cohort 0 | 20/20 | **0/20** | **0/20** |
| join-deleted-gate-closed / ct07-case | cohort 2 | 20/20 | **20/20** | **20/20** |

Every `cohort 0` row is wave 39's published number. The two bold families are
this card's findings: the repair (last row) and its cost (§2.3).

The `workers_released` column is in the JSON beside every cell: 20/20 for every
forced cell except `join-deleted-gate-closed @ cohort 0`, where it is 0/20 —
which is the mechanism of the wave-39 miss, recorded rather than described.

### 5.2 The mutation sweep (`artifacts/mutate.sh`, `artifacts/mutations.json`)

Twelve mutations, one per arm this card added or changed; **eight of them move
the rule** (the forcing header, the failure-injection header, the invisibility
mutation, the run plumbing) rather than the test that reads it. Each is applied
by `sed`, the named test is run alone, and the file is restored **by rewriting it
from the saved copy and advancing its mtime** — never mtime-preserving
(`MUTATION-HARNESS-STALE-ARTIFACTS`). The baseline is proved green before the
first mutation and after the last, and every file is checked sha256-equal to its
pre-sweep copy.

```
baseline_before_rc 0   baseline_after_rc 0   restore_sha256_rc 0
```
(`artifacts/baseline.sha256`, `artifacts/restore.check`: both files `OK`;
`baseline-after` 18 passed / 0 failed / 1 ignored.)

| # | mutation | moves | named test | verdict |
|---|---|---|---|---|
| 1 | `cohort_trigger_removed` — neutralise the cohort condition | rule | `a_join_deleted_is_caught_by_the_cohort_trigger` | RED-AS-REQUIRED |
| 2 | `cohort_trigger_always_on` — fire whatever the row asked | rule | `a_join_deleted_is_caught_by_the_cohort_trigger` (control arm) | RED-AS-REQUIRED |
| 3 | `rows_cohort_is_the_row_s` — hand every row cohort 1 | test | `every_subject_runs_clean_under_each_forced_schedule` | RED-AS-REQUIRED |
| 4 | `rows_unbound` — drop the pthread binding | test | `every_subject_runs_clean_under_each_forced_schedule` | RED-AS-REQUIRED |
| 5 | `arm_create_not_refused` — let the second creation through | rule | `every_failure_arm_runs_clean_and_says_which_arm_it_took` | RED-AS-REQUIRED |
| 6 | `arm_malloc_not_null` — let the allocator allocate | rule | same | RED-AS-REQUIRED |
| 7 | `arm_header_not_included` — stop passing the injection header | rule | same | RED-AS-REQUIRED |
| 8 | `arm_control_injected` — inject into the success-arm control | test | `without_the_injection_the_same_subject_takes_the_success_arm` | RED-AS-REQUIRED |
| 9 | `hook_visible_to_tsan` — re-instrument the header | rule | `the_forcing_header_stays_invisible_to_thread_sanitizer` | RED-AS-REQUIRED |
| 10 | `hook_mutex_reintroduced` — add a visible primitive | rule | same | RED-AS-REQUIRED |
| 11 | `visible_mutation_without_the_lock` — take the lock out of the mutated park | rule | `a_visible_forcing_header_would_hide_the_overlap_…` | RED-AS-REQUIRED |
| 12 | `no_byte_change_new_symbols` — ask for a symbol the emitted C has | test | `the_binding_changes_no_emitted_byte` | RED-AS-REQUIRED |

Mutations 3 and 4 re-establish, on this card's own call shapes, the non-vacuity
wave 39 established for `every_subject_runs_clean_under_each_forced_schedule`:
that card's archived `mutate.sh` anchors on a `run_scheduled(...)` call this card
replaced with a `Row { … }` literal, so two of its seventeen entries would now
read `MUTATION-DID-NOT-APPLY` against the current tree. The wave-39 archive is
**not edited** (constraint 21); the coverage is re-established here and the
overlap is disclosed.

Mutation 3 is the sharpest of the twelve: giving every row a cohort of 1 instead
of its own worker count makes the reverse schedule realise *creation* order
(worker 1 released at its own creation, worker 2 only at the join), so the trace
assertion fails — which is the measurement that the cohort count, and not
something else, drives the release.

## 6. Gate

`docs/reports/tsan-failure-arms/gate.json`, with `gate/` beside it. The
`clang` recording was made in this worktree (`clang-version.txt`,
`clang-version.exit`), not borrowed.

## 7. Residuals and retirements

### Retired — each verified registered first, then met word for word

* **`TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`.** Discharge: *"a release trigger
  independent of the join (a per-program cohort count supplied by the row, or a
  creator-side rendezvous the runtime already emits), with the gate-closed row
  re-measured under it."* Met: `QCORE_FORCED_COHORT` is the per-program cohort
  count supplied by the row, opening the gate inside the create hook; the
  gate-closed row is re-measured under it at 20/20 forced (§2.2), with wave 39's
  0/20 kept as the control arm of the same test. The pinning test is kept, under
  its new name.
* **`TSAN-FORCED-SCHEDULE-FAILURE-ARMS`.** Discharge: *"TSan rows composing
  tests/support/thread_sanitizer.rs's binding with a refusing QCORE_TASK_CREATE
  and a NULL-returning QCORE_BUFFER_MALLOC on the flagship and CT07's case,
  stdout compared with the reference run scripted for the same outcomes, each arm
  shown non-vacuous by mutation."* Met clause by clause (§3.2, §5.2 mutations
  5–8). The one boundary the new rows carry —
  `TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED` — is registered rather than folded in.
* **`TSAN-HEADER-INVISIBILITY-UNENFORCED`.** Discharge: *"a cheap textual
  tripwire (the header must carry no_sanitize("thread") on every qcore_forced_
  function) plus a doc line naming … the behavioural guard; OR promote
  hook_visible_to_tsan to an #[ignore]d workspace mutation test."* Both branches
  landed, and the second is stronger than asked — the mutation test is **not**
  `#[ignore]`d, so it runs in the standing gate (§4).

### Amended, not retired

* **`TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`** — stays registered: it is a rule
  about any future forcing harness, and half two is a measurement of one mutation
  rather than a quantifier over all visible designs. Amendment text in §9.
* **`TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`** — the brief asks whether the remaining
  half is now met. **The named remaining half is met**: that residual's own text
  says *"What remains is the failure arms, carried as
  TSAN-FORCED-SCHEDULE-FAILURE-ARMS"*, and that sibling is retired above; the
  other two narrownesses (repeated runs under named schedules; CT46's serialised
  crossing) were already met and decided in wave 39. **The residual itself is
  not retired**, because what is left of it is the permanent epistemic statement
  in its title — a clean ThreadSanitizer run is evidence about the paths that
  executed, not a proof of race freedom — and retiring it would let a reader
  conclude the opposite. Its disposition becomes its first branch: *stated
  permanently as a measured boundary*. Amendment text in §9.

### Registered (four, `recorded_by: TSAN-FAILURE-ARMS`)

* **`TSAN-COHORT-TRIGGER-CATCH-COST`** — the cohort trigger costs catches on the
  join-edge-removal mutation (§2.3): 0/20 on both subjects under
  `forced-together`, 18/20 on `shared_readers` under `forced-reverse`, against
  20/20 at wave 39's join-only trigger. Discharge: a trigger that releases before
  the join *and* leaves the creator's post-join access unordered relative to the
  worker bodies TSan still holds shadow for; or state permanently, keeping both
  triggers measured in the table and the two wave-39 rows at `cohort 0`.
* **`TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED`** — the NULL-malloc arm creates no
  thread on either subject, so its clean TSan verdict is about a single-threaded
  recovery path (§3.3). Discharge: a TSan row over an allocation failure that
  occurs while a worker is live — which needs an admitted program on this route
  that acquires a buffer after a spawn; or state permanently that the
  allocation-failure arm's TSan evidence is single-threaded by construction.
* **`TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND`** — the cohort count is read off
  each program's text by the row's author (`SUBJECTS`' third column, and the
  per-arm counts in `failure_arms()`), not derived. A subject whose text changes
  its spawn count silently gets a wrong cohort, and the only thing that catches it
  is the schedule-shape assertion. Discharge: derive the count from the checked
  task IR (the `TrySpawn` spans preceding the first `Join`) and assert the row's
  literal against it; or state permanently.
* **`TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER`** — §4's half two shows
  one visible-synchronization design hides the overlap; it does not show that
  every such design does, and half one's forbidden-primitive list is a list.
  Discharge: a property-shaped check (for instance, that no symbol the header
  references is one TSan intercepts) or an argument that the namespace check is
  exhaustive; or state permanently as the measured boundary of the enforcement.

### Standing, untouched by this card

`TASK-ROUTE-TSAN-PTHREAD-BINDING` (every row is a pthread-bound build),
`TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` (Linux arm only — this card adds no Windows
or Apple row and makes no such claim), `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`,
`TSAN-FORCED-CATCH-POWER-EQUAL` (unchanged: at `cohort 0` every mutation is still
caught equally by the unforced and the forced harness; at `cohort 2` the unforced
harness is *strictly better* on the join-edge family, which strengthens rather
than weakens that residual's statement).

## 8. What this card does not engage

* **Diagnostics.** No `Q` code is allocated, and no existing band's condition is
  widened: nothing in this card is a checker rule. `docs/QCORE_SPEC_r0_3.md` §L21
  is untouched.
* **Constraint 14 (the keyword census), stated even though it is +0.** This card
  adds **no source word and no token**: it adds two C preprocessor selectors
  (`QCORE_FORCED_COHORT`, `QCORE_TSAN_FAILURE`) that live only in test-support
  headers reaching the compiler through `-include`/`-D`, and two Rust items
  (`Failure`, `Row`). Run, not assumed: `is_reserved_source_word`
  (`crates/qcore-frontend/src/frontend.rs:2946`) holds **113** literals, 113 of
  them distinct (case-sensitive extraction, comment lines excluded), and
  `git diff 463fa6e8 -- crates/qcore-frontend/src/frontend.rs` is empty — the
  count is the wave-40 corrected 113 and is unchanged.
* **Lean.** No semantic surface moves: the harness is test support and the
  emitted programs are byte-identical, so there is no definition or theorem to
  mirror. The audit count is unchanged and recorded in `gate.json`.
* **Counts.** No CT stage, no corpus count, no contract count and no verdict
  moves. CT07's `expected.json` is **not** edited: this card's rows cite the
  fixture through `compiler/tests/task_route_tsan.rs` exactly as wave 39's
  citation does, and adding a citation to a frozen expectation file is not
  something this card's evidence requires.

## 9. For the integrator

Every item below is an item.

1. **Ledger — no headline number moves.** The merged-gate row in `AGENTS.md` is
   the integrator's own tip measurement; this card moves no corpus, CT, contract
   or verdict count. The workspace test count grows by the rows this card adds
   (three new test functions, one renamed) — the integrator's tip run is the
   authority for the number.
2. **Retire `TSAN-FORCED-GATE-OPENED-BY-THE-JOIN`** from
   `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups`, verified
   present by id first. Ground: §2.2, met word for word.
3. **Retire `TSAN-FORCED-SCHEDULE-FAILURE-ARMS`.** Ground: §3.2 and §5.2.
4. **Retire `TSAN-HEADER-INVISIBILITY-UNENFORCED`.** Ground: §4, both branches
   of its discharge landed, the second branch stronger than asked (the mutation
   test is not `#[ignore]`d).
5. **Amend `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`** by appending, in these
   words: *"ENFORCED since card TSAN-FAILURE-ARMS (wave 41), which discharged
   TSAN-HEADER-INVISIBILITY-UNENFORCED: two committed, non-ignored workspace
   tests in compiler/tests/task_route_tsan.rs.
   the_forcing_header_stays_invisible_to_thread_sanitizer reads SCHEDULE_FORCING's
   text — the QCORE_FORCED_UNINSTRUMENTED macro must still expand to
   no_sanitize("thread"), the header must name no pthread/C11 synchronization
   primitive, and every static qcore_forced_ function must carry the attribute.
   a_visible_forcing_header_would_hide_the_overlap_which_is_why_the_shipped_one_is_not
   builds the hook_visible_to_tsan mutation as thread_sanitizer::with_visible_synchronization
   — a pthread mutex for the bookkeeping lock, instrumentation restored, and a
   parked worker that learns of its release by acquiring that mutex — and asserts
   that the overlap mutation goes UNREPORTED under ForcedReverse while the
   shipped header reports it. Measured 0 of 20 visible against 20 of 20 shipped
   under BOTH forced schedules, so the false negative is broader than wave 39
   recorded (docs/reports/tsan-failure-arms/artifacts/catch-rates.json,
   'invisibility'). Every substitution asserts its anchor appears exactly once,
   so a header edit out of the measured shape panics rather than passes. The rule
   itself stays registered: half two is one mutation, not a quantifier over all
   visible designs — TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER."*
6. **Amend `TASK-ROUTE-TSAN-EXECUTED-SCHEDULES`** by appending, in these words:
   *"AMENDED by TSAN-FAILURE-ARMS (wave 41): the last concrete half is met. The
   failure arms this residual named first — a failed qcore_buffer_new, a failed
   spawn — are now executed under ThreadSanitizer on the write-lend flagship and
   on CT07's own case, under all three named schedules, clean, with the arm that
   ran asserted from the program's own stdout and the stdout compared byte for
   byte against the reference run scripted for the same outcomes (20/20 on every
   one of twelve cells); its carrier TSAN-FORCED-SCHEDULE-FAILURE-ARMS is
   retired. The residual is NOT retired: what remains is the permanent
   epistemic statement in its title, and its disposition is now the first branch
   of its own discharge — stated permanently as a measured boundary. Boundaries
   carried: TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED (the allocation-failure arm
   creates no thread on either subject) and TSAN-COHORT-TRIGGER-CATCH-COST."*
7. **Register four residuals** with `recorded_by: TSAN-FAILURE-ARMS` and
   `source: docs/reports/tsan-failure-arms/README.md`, texts as in §7:
   `TSAN-COHORT-TRIGGER-CATCH-COST`, `TSAN-ALLOC-FAILURE-ARM-SINGLE-THREADED`,
   `TSAN-FAILURE-ARM-COHORT-SUPPLIED-BY-HAND`,
   `TSAN-VISIBLE-HEADER-ONE-MUTATION-NOT-A-QUANTIFIER`.
8. **`named_followups` arithmetic:** 257 at the base, −3 retired, +4 registered
   = **258**. `AGENTS.md`'s stated count wraps as
   ``(`named_followups`, **257**`` … ``entries)``; keep it in step whatever its
   shape. The matrix in this card's tree was regenerated **with `--gate`**
   pointing at this card's `gate.json`.
9. **No standing constraint is landed or amended by this card**, and no §L21 row
   moves. If the integrator wants the enforcement indexed in `AGENTS.md`, it
   belongs beside `TASK-ROUTE-TSAN-PTHREAD-BINDING`'s existing treatment rather
   than as a new numbered constraint — this card does not propose one.
10. **File ownership.** This card owns `tests/support/thread_sanitizer.rs`,
    `compiler/tests/task_route_tsan.rs` and
    `compiler/tests/write_lend_tasks_exercise.rs`. It touched the first two;
    `write_lend_tasks_exercise.rs` is **unmodified** (its only use of the support
    module is `PTHREAD_BINDING` and `run`, whose signatures are unchanged). No
    file owned by a sibling card was touched; `git diff --name-only` against the
    base is three files plus this report directory.
11. **Wave-39 archive overlap, disclosed:** two of `docs/reports/tsan-rows-2/
    artifacts/mutate.sh`'s seventeen entries (`rows_unbound`,
    `gate_opened_after_all`) no longer apply verbatim to the current tree,
    because this card replaced the call shape one anchors on and renamed the test
    the other names. The frozen archive is **not edited** (constraint 21); the
    coverage is re-established by this card's own sweep entries 3, 4 and 1–2.
