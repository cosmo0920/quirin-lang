# `VERIFIER-HOST-CONTROL-BUDGET` — the contested host row, explained and repaired

Wave 35; base `a300a39d`. Scope: `crates/qcore-verifier-host/` only.
Raw logs are worktree-private under `measure/control-budget/` and are named
individually below; every figure in this report comes from one of them.

## Headline

The row is **not** a fixed throughput shortfall, and it is **not** deterministic
in either direction. The contested control fails because the transport loop's
backoff charges a **fixed 1 ms quantum for an event that takes ~5 µs**, and it
takes that quantum most often when the host is *least* loaded — which is the
inversion nobody had a mechanism for. On this host the pipe capacity is **8,192
bytes**, so one quantum per bounded chunk is a ceiling of **8 KiB/ms**; an
8 MiB request needs ≥1,024 quanta and eats a 1,000 ms budget whole. That is the
**same arithmetic the wave-33 card repaired, surviving on the write side**: that
card removed the *unconditional* sleep, leaving the conditional one to charge a
full quantum whenever the peer had not yet been scheduled.

Disposition taken: **(c), a repair to the transport loop.** The backoff now
waits for pipe *readiness* (`poll(2)`), bounded by the same 1 ms quantum and the
same deadline. No assertion, deadline or refusal arm was touched. Paired,
interleaved, under one ambient load: the old backoff **4/8 red**, phase 4 taking
350–1,000 ms; the new one **8/8 green**, phase 4 taking **42–112 ms** inside the
unchanged 1,000 ms budget.

## 1. What each prior measurement got right, and what does not reproduce

| claim | source | status here |
|---|---|---|
| the control returns `Err(Timeout)` in isolation | `stability-reeval-1` §2.1 | **reproduced** |
| a wall-clock shortfall, not a hang (8,000 ms passes) | `stability-reeval-1` §2.1 | **reproduced** and now explained |
| failing **deterministically 3/3** in isolation | `stability-reeval-1` §2.1 | **does not reproduce** |
| green inside the full workspace suite | `docs-compact-6` | **reproduced**, and now explained |
| the repair is present and the assertion not weakened | `stability-reeval-1` §2.1 | **confirmed**; still true after this card |

Pre-repair, on the unmodified loop, **22 isolated repetitions at 1-minute load
5–12** gave **7 pass / 15 fail** — a biased coin, not a determinism
(`baseline-isolated.log` 6/10 pass at load 9.7–11.8; `load-ambient.log` 0/6 at
load 9.9–12.3; `traced-phases.log` 1/6 at load 5.6–6.0). The earlier card's
three runs are consistent with this; what does not survive is the word
*deterministic*, and that word was load-bearing there — it was the reason given
for excluding a load-sensitivity explanation. The right conclusion was still
reached (this is not constraint 16's shape), but the premise was three samples
of a coin.

## 2. Mechanism

### 2.1 The instrumentation

The loop was temporarily instrumented behind `QCORE_HOST_TRANSPORT_TRACE` with
per-phase counters (loop iterations, progress-making writes and reads, backoff
calls split by phase, the time at which input finished, the time of first
output, total). The instrumented source is archived at
`measure/control-budget/lib.rs.instrumented` and the patch at
`measure/control-budget/instrumented.diff`; **both were removed before
publication** and the shipped diff carries neither counters nor an env switch.

### 2.2 The measured ceiling

Measured on this host (`python3`, `F_GETPIPE_SZ`, five consecutive samples):

```
default pipe capacity = 8192 bytes   (8192 8192 8192 8192 8192)
fs.pipe-max-size = 1048576   fs.pipe-user-pages-soft = 16384   hard = 0
kernel 7.0.0-31-generic, AMD Ryzen AI 7 350, 16 CPUs
```

8,192 bytes is **two pages**, where `PIPE_DEF_BUFFERS` would give 65,536. That
reduction is measured, not explained — see the follow-on
`HOST-PIPE-CAPACITY-UNEXPLAINED`.

The control's request frame is **8,389,095 bytes** and the trace shows exactly
**1,025 progress-making writes** — ~8,184 bytes each, i.e. one pipe capacity per
write. The loop's own comment already named this shape on the read side
("one bounded chunk per iteration"); the write side inherits it from the pipe.

### 2.3 The sleeps *are* the time

From `traced-phases.log`, phase 4 (the control), pre-repair:

| `write_sleeps` | `input_done_us` | ratio |
|---|---|---|
| 448 | 477,296 | 1.065 ms/sleep |
| 560 | 611,466 | 1.092 |
| 758 | 803,928 | 1.061 |
| 905 | 957,494 | 1.058 |

`input_done_us ≈ 1.06 ms × write_sleeps`, across the whole range. The input half
of the control consumes **477–992 ms of the 1,000 ms budget in backoff alone**,
leaving the 8 MiB output half whatever is left — usually nothing. In the worst
pre-repair runs the input never finished at all inside the deadline
(`input_done_us=0` with `write_sleeps=947`).

### 2.4 Why being *slower* makes it pass — the inversion

The backoff is taken only when no pipe moved. During the input half that means:
the host wrote 8 KiB, filling the pipe, and came back around **before the peer
`cat` had been scheduled to drain it**. On a quiet machine the loop returns in
about a microsecond, long before the peer runs, so nearly every write is
followed by a full 1 ms charge. Under contention the host is preempted between
iterations; by the time it runs again the peer has already drained the pipe, the
write succeeds, and **no quantum is charged at all**.

Measured load sweep, pre-repair, phase 4:

| condition | backoff calls | `input_done` | outcome |
|---|---|---|---|
| ambient, load 9.9–12.3 | 657–947 | 694–992 ms (or never) | **0/6 pass** |
| +24 spinners, load 18.8–22.5 | 22–90 | 38–115 ms | 3/3 pass |
| +24 spinners settled, load 79–82 | 18–91 | 37–170 ms | 3/3 pass |

**The leading hypothesis in the brief is falsified.** Mean core clock in the
*passing* heavy-load arm was 2,954–3,082 MHz; in the *failing* ambient arm it
reached 3,654 MHz. The arm that passes runs at a **lower** clock. CPU frequency
scaling and idle-state exit latency do not explain the inversion; scheduling
slack does.

### 2.5 The paired experiment

To compare without waiting for an idle host, the two backoff strategies were
made temporarily env-selectable and run **interleaved**, so both arms see the
same ambient load (`ab-interleaved.log`, 8 pairs, 1-minute load 17.8–21.8):

| arm | result | phase-4 total | `input_done` | backoff calls |
|---|---|---|---|---|
| **A** fixed 1 ms quantum | **4 pass / 4 fail** | 350, 440, 645, 939, **999, 999, 1000, 1000** ms | 132–839 ms | 231–941 |
| **B** readiness wait | **8 pass / 0 fail** | 42–112 ms | 4–30 ms | 86–**1441** |

Two things to read off this table. Every arm-A failure sits **exactly on the
1,000 ms wall** (999,720 / 999,772 / 1,000,125 / 1,000,311 µs) — the deadline is
reached, not overshot by a hang. And arm B's backoff **count is often higher**
than arm A's (1,441 against a maximum of 941) while its time is ~20× lower: the
cost was never the number of waits, it was the price of one. Directly:

* pre-repair, 905 waits in 957 ms → **1,057 µs per wait**
* post-repair, 1,019 waits in 5.6 ms → **5.5 µs per wait**

a factor of **190**.

## 3. Disposition

**Taken: (c), a repair to the transport loop**, because the mechanism revealed a
real defect rather than a host property. `backoff_until_ready` replaces
`thread::sleep(Duration::from_millis(1))` with a `poll(2)` on exactly the pipes
still live — `POLLOUT` on stdin while unsent, `POLLIN` on each output pipe that
has not finished — with timeout `min(remaining deadline, 1 ms)`.

What is preserved, deliberately and by construction:

* **The 1 ms quantum is unchanged as a bound.** No wait is longer than the wait
  it replaced; `remaining.is_zero()` returns without waiting at all, so the
  deadline is now reached slightly *sooner*, never later.
* **An idle provider still backs off.** With no readiness, `poll` blocks the
  full quantum — the same CPU behaviour as the sleep. It cannot become a spin:
  a `poll` failure falls back to the timed sleep, and an empty descriptor set
  (every pipe finished, direct child not yet reaped) sleeps exactly as before.
* **Every refusal arm, deadline, ordering and cap is untouched.** The diff adds
  one helper and changes one call site. No `return` in the loop moved, no
  `ProviderFailure` variant's condition changed, `read_available` is unchanged,
  and all three of the row's deadlines (40 ms, 100 ms, 1,000 ms) stand.
* **The contested assertion stands.** `assert_eq!(provider.verify(&request),
  Ok(response))` at `standalone_host.rs:212`, both `Err(Io(_))` arms and the
  descendant/live marker checks are byte-for-byte unedited. **The test file was
  not touched by this card at all.**

**Why not (a), a measured throughput calibration in the test.** With the ceiling
removed the control completes in 42–112 ms against 1,000 ms — a ~9× margin — so
a calibration would add machinery to a row that no longer races. Worse, it would
have *encoded the defect as a property*: a budget derived from measured
throughput would have measured 8 KiB/ms, enshrined the ceiling, and left every
real provider returning a few MiB throttled to 8 MB/s on a link that does
gigabytes per second. Calibrating around it was the option most likely to look
like a discharge while making the defect permanent.

**Why not (b) alone, recording the row beside constraint 16.** The mechanism is
not host-dependent in the way that framing assumes — it is a defect in the loop,
and constraint 16's five rows are rows that are *correct* but load-sensitive.
Recording this one there would have filed a bug as an environment note. **But
(b) is half-right and I am not dropping that half**: the row carries a *second*,
opposite load sensitivity which this card does not repair (§5), and that one
genuinely is constraint 16's shape. Proposed amendment text is in §9.

**Not done, per the residual:** the response was not shrunk, no deadline was
raised, nothing was deleted.

## 4. Invariant 6 — the Lean surface, checked rather than inherited

The wave-33 card asserted "no source semantics, verdict admission rules, or Lean
definitions change". I did not inherit that. Surveyed `proof/` (303 `.lean`
files) for anything modelling this code. Exactly two modules name
`ProcessProvider`:

* `proof/QCore/PackageIdentity.lean` — the **construction discipline**: that a
  `ProcessProvider` exists only through `verify_executable_package`, and that
  `UnverifiedProvider` is a distinct type with no owning accessor.
* `proof/QCore/ProcessQuotaPlan.lean` — the **declared-limit-kind plan**: given
  a target, profile and declared kinds, which mechanism installs each kind and
  what happens to one no mechanism installs. Its own preamble states what stays
  a Rust and OS boundary.

Neither mentions the transport loop, the backoff, `read_available`, pipe
readiness, or the mechanics by which a wall-clock deadline is enforced — the
wall clock appears there only as the *name of a declared limit kind*, whose
value and placement this card does not touch. **The Lean surface is not
engaged**, and the check is recorded so the next card need not take it on faith.

The honest form of the soundness argument is directional, not a theorem: each
wait is bounded by the wait it replaced, so with identical scheduling the loop
reaches every subsequent check no later than before, and an execution can only
move from `Timeout` toward completion — never the reverse. Real scheduling is
not identical between runs, so this is an argument about direction backed by 30
post-repair executions, not a proof about OS wall time.

## 5. Two load sensitivities, in opposite directions

While driving the load sweep I found the row's **first** arm breaking at the
other extreme. At 1-minute load ≈100 (`overload-probe-ramp.log`, 6 repetitions,
3 failures):

```
assertion failed: start.elapsed() < Duration::from_secs(1)
```

That is `standalone_host.rs`'s guard after the 40 ms timeout arm — a fixed 1 s
wall-clock assertion about a 40 ms budget. It is **correct but load-sensitive**,
which is precisely constraint 16's shape, and it is **not repaired by this
card** and not claimed to be. It is named `VERIFIER-HOST-EARLY-ARM-OVERLOAD`
below. So the row was never simply "red alone, green under load": it had a
**band**, failing below it for the reason this card repairs and above it for a
reason this card does not.

## 6. Outcomes — isolation and full suite

Every figure carries the 1-minute load average at which it was taken, as the
brief requires. Five sibling cards were building on this machine throughout, so
"isolation" here means *this test alone in its process*, never an idle host.

| measurement | reps | load | result | log |
|---|---|---|---|---|
| **pre**-repair, isolated | 10 | 9.7–11.8 | 6 pass / 4 fail | `baseline-isolated.log` |
| **pre**-repair, isolated | 6 | 9.9–12.3 | 0 pass / 6 fail | `load-ambient.log` |
| **pre**-repair, isolated, traced | 6 | 5.6–6.0 | 1 pass / 5 fail | `traced-phases.log` |
| **pre**-repair, +24 spinners | 3 | 79–82 | 3 pass / 0 fail | `overload-probe.log` |
| paired A/B, interleaved | 8+8 | 17.8–21.8 | A 4/8, **B 8/8** | `ab-interleaved.log` |
| **post**-repair, isolated, traced | 10 | 43–57 | **10 pass / 0 fail** | `repaired-traced.log` |
| **post**-repair, isolated | 12 | 10.8–11.3 | **12 pass / 0 fail** | `repaired-isolated-lowload.log` |
| **post**-repair, isolated, final shipped source | 8 | 34.1–36.3 | **8 pass / 0 fail** | `final-source-isolated.log` |
| **post**-repair, whole crate | 112 tests | 7.9 | **112 pass / 0 fail** | §7 |
| **post**-repair, whole workspace | 1 | 21.2→36.4 | contested row **ok** | §7 |

The 12 post-repair isolated repetitions at load ~11 are the decisive ones: that
is the same regime in which the pre-repair loop returned 6 failures in 16 runs.

## 7. Gate

Disk was checked before the gate as the brief directs: **210 GB free**, above the
60 GB threshold, so the **full standing gate was run** rather than the
crate-only subset. Every step's exit is `measure/control-budget/gate-summary.txt`.

| step | exit | result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a2`, before anything expensive |
| `cargo test --workspace --no-fail-fast` | **101** | **4,558 passed / 1 failed / 8 ignored**, 230 test binaries. The one failure is constraint 16's first row — §7.1 |
| — the contested row inside that suite | — | **ok** |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **129** Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | **229** Python tests |

The gate ran at 1-minute load 21.2 rising to 36.4, with five sibling cards
building throughout. `cargo fmt` was re-run on the final shipped source after
the §7.1 A/B and is clean; the whole `qcore-verifier-host` crate
(**112 tests**) was re-run on that same final source, 0 failed.

### 7.1 The one red row — measured, not assumed, and not this card's

`qcore-verifier-host::enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
failed with `assertion left == right failed: Broken pipe (os error 32); left:
Io, right: Terminated` at `enforced_process_limits.rs:389`.

This is **constraint 16's first named row** — `docs/reports/examples-suite-wire/README.md:307`
names it in those words ("the `RLIMIT_NPROC` conformance row, constraint 16's
first"), and `docs/development/increments/wave-32.md:105` already records this
exact `Io(Broken pipe)` signature for it.

I did not stop at that identification, because this card makes the transport
loop **faster**, which could plausibly reorder which failure a racing shell
reports. So the same interleaved A/B was run on this row alone
(`nproc-ab.log`, 8 pairs, load 36.5, alternating arms):

| arm | result |
|---|---|
| **A** pre-repair fixed quantum | **7 fail / 1 pass** |
| **B** repaired readiness wait | **6 fail / 2 pass** |

The row fails at the same rate with and without the repair, so **this card does
not cause it**. That is consistent with the code: the row's race is decided by
the *first* write attempt, which precedes any backoff in either arm.

One sharper point than "it is a known flake". The row's own comment says
`RLIMIT_NPROC` "counts every process of the **real UID**, not of this process
tree". Five sibling cards were running: the account held **616–639 processes**
throughout (recorded per repetition in `nproc-isolated-postrepair.log` and
`nproc-ab.log`). For this row the sibling cards are not ambient load — they are
*inside its measurement*, and they make its premise hold far more extremely than
intended. Accordingly it was **red 5 of 6 even in isolation** here, where
"isolation" means this test alone in its process on a machine still carrying
load 30. **It therefore cannot be cleanly verified on this host under this
wave's conditions**, and I am not claiming it green. Constraint 16's instruction
is to verify in isolation and never weaken; I verified, could not reach a quiet
account, and record both facts (`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`).

### 7.2 What I did not run, and why

The brief asks for several repetitions of the contested row *inside the full
suite*. **I ran one** full-workspace repetition (the gate above, in which the
row passed) and **skipped the further ones**. Reason, stated rather than left
implicit: free disk fell from 210 GB at gate start to **42 GB** by the time the
gate and the A/B work finished — six cards on one disk, my own `target/` alone
at 43 GB — which is **below the brief's 60 GB threshold**. Starting another
workspace build at that point risked the sibling cards, and the disk has already
filled once this wave. The isolated evidence is correspondingly deeper: **38
post-repair repetitions** of the contested row (§6), against one full-suite
observation.

## 8. Named follow-ons

* **`VERIFIER-HOST-EARLY-ARM-OVERLOAD`** — `standalone_host.rs`'s
  `assert!(start.elapsed() < Duration::from_secs(1))`, guarding the 40 ms
  timeout arm, fails at 1-minute load ≈100 (3 of 6 repetitions measured, §5).
  Correct but load-sensitive: constraint 16's shape, at the opposite end of the
  band from the defect this card repaired. **Discharge**: record it beside
  constraint 16's five with the measured load condition (§9 carries proposed
  text), or derive the guard from the declared budget instead of a fixed second.
  Not a discharge: raising the second, or deleting the guard.
* **`HOST-PIPE-CAPACITY-UNEXPLAINED`** — the measured default pipe capacity on
  this host is **8,192 bytes (two pages)** where `PIPE_DEF_BUFFERS` would give
  65,536, stable across five samples, with `pipe-user-pages-soft` at its default
  16,384 and no relevant kernel command-line argument. The capacity is one of
  the two factors in the ceiling arithmetic (capacity ÷ quantum), so the size of
  the defect this card repaired is a function of an unexplained number.
  **Discharge**: identify the kernel or distribution setting that produces it,
  or measure the capacity on a second Linux host and record whether the ceiling
  arithmetic differs there.
* **`HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN`** — two arms of `backoff_until_ready`
  are reached by **no test**: the empty-descriptor-set sleep (every pipe
  finished while the direct child is not yet reaped — transient, not
  deterministically drivable) and the `poll` failure fallback (arguably
  unreachable at all, since the descriptors are owned by live pipes for the
  duration of the call). Per `CLAUDE.md`'s `STABILITY-REEVAL` discipline these
  are recorded as **defensive, not verified**; the green suite is evidence about
  the paths it took. Both fail in the conservative direction (they degrade to
  the exact backoff they replaced), so the risk is bounded by direction.
  **Discharge**: drive them with an injected descriptor set, or state them as a
  measured boundary.
* **`VERIFIER-HOST-READINESS-APPLE-UNRUN`** — `backoff_until_ready` is
  `#[cfg(unix)]` and `poll(2)` is POSIX, so Apple compiles and would run it, but
  **no macOS execution is claimed**. Linux only, as with every other
  observation here. There is **no Windows sibling to this risk**: verified in
  source that the non-Unix arms of `verify`/`verify_v2` refuse by name
  ("bounded process providers require Unix process groups") and reach no
  transport loop at all.
* **`VERIFIER-HOST-CONTROL-MARGIN-UNPINNED`** — the control now completes in
  42–112 ms against a 1,000 ms budget, but **nothing pins that margin**: a
  future change reintroducing a per-chunk quantum would be caught only when the
  margin was fully spent again, which is how this defect survived wave 33.
  **Discharge**: a test asserting the large-response control's elapsed time
  against a budget derived from the declared timeout, or an explicit decision
  that a wall-clock margin assertion is not wanted in this suite.

## 9. For the integrator

1. **Two matrix entries close with this card**, and both are yours to flip, not
   mine — I regenerated nothing. `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED`:
   discharged by option (c), an evidenced host investigation *plus* the repair
   the investigation justified; the control now passes **38/38** across every
   post-repair repetition here. `FB-PLAN-GATE-HOST-TIMEOUT`, whose own text says
   it "closes when that one does". The generator is
   `docs/reports/remaining-d02/build_matrix.py` (per your correction mid-card;
   the brief's `scripts/build_matrix.py` does not exist and I did not invent a
   path), invoked in the `--gate` form the last regenerations used.
2. **One correction to a published residual.** The
   `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` entry states the row failed
   "deterministically 3/3 in isolation". Twenty-two pre-repair isolated
   repetitions here gave 7 pass / 15 fail (§1). The entry's *conclusion* held;
   its determinism claim was three samples of a biased coin, and it was the
   stated reason for excluding a load explanation. Worth carrying into whatever
   text replaces it.
3. **Proposed amendment text for standing constraint 16** — verbatim, to be
   landed as written or recorded `declined` with its ground:

   > `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
   > (`crates/qcore-verifier-host/tests/standalone_host.rs`) is a **sixth**
   > load-sensitive row, and the only one sensitive at the *top* of the range:
   > its first arm's `assert!(start.elapsed() < Duration::from_secs(1))`, a
   > fixed one-second guard over a 40 ms deadline, was measured failing 3 of 6
   > repetitions at 1-minute load ≈100 on a 16-CPU host
   > (`docs/reports/verifier-host-control-budget/README.md` §5). Its *other*
   > historical sensitivity — the large-response admitting control failing when
   > the host was **least** loaded — was a transport defect, not a host
   > condition, and was repaired by card `VERIFIER-HOST-CONTROL-BUDGET`; do not
   > re-file that half here.

4. **AGENTS.md headline numbers were not edited**, per the brief. The workspace
   test count in §7 is measured at this card's tip and is yours to reconcile.
5. **The test file is untouched.** `git diff --stat` for this card is one file,
   `crates/qcore-verifier-host/src/lib.rs`. If the shape of this card surprises
   you — a card that was scoped to disposition a control's *budget* and instead
   changed the loop — §3 states why each of the other two landings was refused,
   and the paired A/B in §2.5 is the evidence that distinguishes them.
6. **`measure/` is worktree-private and gitignored**; nothing under it is
   proposed for commit. The instrumented source and its patch are archived there
   so §2's numbers can be reproduced, and both were stripped from the shipped
   diff — verified by grepping the final source for the instrumentation and
   switch markers, which returns nothing.
