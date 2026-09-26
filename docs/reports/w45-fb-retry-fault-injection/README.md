# W45-FB-RETRY-FAULT-INJECTION — native fault-injection observations (C1-C3)

This card carries out the prepared run guide
(`docs/reports/fb-system-audit-cards/retry/README.md`'s C1-C3, and the
follow-on plan in that card's `followups.json`) on a real Fluent Bit build,
with a control for every fault run. It is **native observation only**: no
analyzer, contract, provider or Lean change, and **no Quirin verdict moves**.
A reproduced behaviour here is a *local observation of the candidate under an
injected premise*, never an upstream defect claim, a confirmed production bug,
or a production failure-rate statement. Nothing was sent upstream. Evidence
binding, hashes and commands are in [`evidence.json`](evidence.json); resume
steps in [`RESUME.md`](RESUME.md). C4 (stale completion identity) is **out of
scope** for this card, unchanged from the run guide.

## Build and binding

- Quirin base (worktree/main at time of run): `5e9630fe`.
- Fluent Bit source: `864c3d97f3589515285a19396ed8dfc85cda08ae`, confirmed on
  the read-only user checkout `/home/cosmo/Documents/GitHub/fluent-bit`
  (`rev-parse HEAD`) and via the binary's own `--version` output
  (`Fluent Bit v5.1.3, Git commit 864c3d97f3589515285a19396ed8dfc85cda08ae`).
  No source change; the checkout was never written to and never rebuilt.
- Binary (prepared by a prior step, not by this card):
  `private/w45-fb/w45-fb-retry-fault-injection/attempt-1/build/bin/fluent-bit`
  (outside the worktree, under the repository's `private/`), Clang 21.1.8,
  Debug, `-O0 -g3 -fno-omit-frame-pointer`, `-DFLB_OUT_RETRY=On`. SHA-256
  verified before every run batch:
  `fc5ab4b5ac970ac3f22b833c4b9d89be00ff2fa7bc280995b4abf783b2b87d33`
  (matches the run guide's pinned hash; build logs in that tree's `logs/`,
  unchanged by this card).
- Every run used only the in-tree `dummy` input and `retry` output
  (`-i dummy -p rate=1 -o retry -p retry=3 -m '*' -f 1 -v`), no storage path,
  no network destination.

## Method

gdb ran in batch mode (`gdb -q -batch -x SCRIPT --args ...`) with
`set confirm off` and `handle SIGTERM nostop pass`, launched under
`timeout -k 10 N` with its PID captured. Fluent Bit was stopped with
`kill -TERM <fluent-bit-pid>`, where the inferior's PID was read from the
process tree (`pgrep -P <gdb-pid>`, itself found as the child of `timeout`),
never by pattern. Every run's full gdb+program output, exact command, PIDs,
signal-send timestamp and exit code are under `runs/`; the gdb scripts are
under `runs/gdb/`.

**Harness bug found and fixed before any candidate run counted**: the first
driver captured `timeout`'s own PID as "the gdb PID" and searched for its
child, which is gdb itself, not the inferior — so the first attempts sent
SIGTERM to gdb (killing the whole ptraced tree immediately, before Fluent
Bit's own signal handler or teardown ran at all). Diagnosed with a minimal
breakpoint-only script (`flb_sched_destroy` / `flb_config_exit`) and a
plain-process control run showing normal ~50ms graceful shutdown; fixed by
resolving gdb's real PID as the child of `timeout` and the inferior's PID as
the child of gdb. All candidate runs below post-date the fix and show the
expected `[engine] caught signal (SIGTERM)` / grace-period / teardown
sequence.

## C1 — timer left behind when the retry request fails to allocate

**Fault point.** `flb_sched_request_create` (`src/flb_scheduler.c`): the
timer is created and linked into `sched->timers` at :275-280/889 before the
request is allocated at :282; if that allocation fails, the null check at
:283 returns -1 without destroying the timer. **Mechanism**: a breakpoint at
`flb_scheduler.c:283` (the request null-check, reached once per process via
an `$c1_done` guard) sets `request = 0` for the fault run only, forcing the
same branch a real `flb_malloc` failure would take. A second breakpoint at
`flb_sched_destroy` walks `sched->timers` at teardown.

- **Control** (`runs/c1-control.log`): `C1: timer 0x7ffff0031060 linked,
  request 0x7ffff00349a0`. At `flb_sched_destroy`, 4 timers remain — types 2
  (frame) and 4 (permanent callback) only; no `type=0`/`data=(nil)` entry, and
  no leftover `type=1` (`FLB_SCHED_TIMER_REQUEST`) entry either, because the
  real request this breakpoint observed had already completed its retry
  cycle and been invalidated well before shutdown.
- **Fault** (`runs/c1-fault.log`): `C1: timer 0x7ffff0031060 linked, request
  (nil)`, followed immediately by `[ warn] [engine] retry for chunk
  '...' could not be scheduled: input=dummy.0 > output=retry.0` (the
  `flb_engine.c:792-803` handling the guide names). At teardown, 5 timers
  remain: the same 4 as control, **plus** `C1: timer 0x7ffff0031060 type=0
  data=(nil)` — the exact address printed at the fault point, with the
  zeroed type/data the guide predicted, confirmed still linked until
  `flb_sched_destroy` freed it (no crash, no use-after-free).
- **Classification: observed-local.** The retained-timer branch reproduces
  exactly as named; the Codex report's contrary guard — "not a permanent
  leak, because teardown reclaims the timer" — **held**: the timer was
  present and freed inside `flb_sched_destroy`, not before, and not after
  process exit.

## C2 — temporary task user left when a synchronous output fails to enqueue

**Simulation caveat.** No in-tree output plugin sets `FLB_OUTPUT_SYNCHRONOUS`
(2048) at this revision (confirmed by the Codex report; only an external
proxy plugin can). Every C2 (and C3) run therefore forces the flag on the
`retry` output instance via a breakpoint at `flb_output.c:892` right before
the singleplex queue is built, and is recorded here as a **simulation**, not
an observation of any shipped configuration.

**Fault point.** `flb_output_task_singleplex_enqueue` adds a temporary task
user at `flb_output.c:368`, then calls `flb_output_task_queue_enqueue`; if
the node allocation there (`:288`) fails, that callee destroys the retry (if
present) and returns -1 (`:289-295`), and the caller returns -1 at `:372-374`
without decrementing the user it just added. Mechanism: a breakpoint at
`flb_output.c:289` (guarded to fire once, or once with `retry != 0` for the
retry variant) sets `queued_task = 0` for the fault run, forcing the same
null-check branch a real `flb_malloc` failure would take.

**First dispatch** (retry == NULL at the fault point):
- **Control** (`runs/c2-control-first.log`): `C2: task 0x7ffff0030b30
  users=1 queued_task=0x7ffff00369f0 retry=(nil)`; `flb_task_destroy` for
  that exact task fires at `12:51:17.140`, well before the SIGTERM sent at
  `12:51:18` — reaped by the ordinary flow.
- **Fault** (`runs/c2-fault-first.log`): `C2: task 0x7ffff0030b30 users=1
  queued_task=(nil) retry=(nil)`. That task is never destroyed during normal
  operation; shutdown logs `[engine] service has stopped (1 pending tasks)`
  (control's equivalent line reads `0 pending tasks`), and only then, at the
  5-second grace-period expiry, `C2: task 0x7ffff0030b30 destroyed, users=1`
  — the temporary user was never decremented.

**Retry** (breakpoint condition `retry != 0`, landing on a scheduled retry's
re-enqueue rather than the first attempt):
- **Control** (`runs/c2-control-retry.log`): `C2(retry): task 0x7ffff0030b30
  users=1 queued_task=0x7ffff0033fb0 retry=0x7ffff0030cd0`; destroyed at
  `users=0` via `flb_task_users_release -> flb_task_destroy` (backtrace
  captured) when the task hits its own retry-attempts limit, before SIGTERM.
- **Fault** (`runs/c2-fault-retry.log`): `C2(retry): task 0x7ffff0030b30
  users=1 queued_task=(nil) retry=0x7ffff0030cd0`; retained with `users=1`
  through shutdown (`(1 pending tasks)`), destroyed only at grace-period
  expiry (`C2(retry): task 0x7ffff0030b30 destroyed, users=1`).
- **Classification: observed-local (simulated synchronous output).** Both
  variants reproduce the named candidate identically in shape: the failed
  enqueue's temporary user is never released by the ordinary path, and the
  task/chunk is retained until forced input teardown, not permanently
  leaked. The Codex report's contrary guard — "input teardown ultimately
  destroys tasks despite the count, so the bounded consequence is retention
  until teardown, not a leak" — **held** in both variants.

## C3 — task left with zero users after a queued retry's flush fails

**Simulation caveat**: same forced `FLB_OUTPUT_SYNCHRONOUS` as C2.

**Fault point.** `flb_output_task_queue_flush_one` drops the temporary user
at `flb_output.c:329` without checking whether the task can now be released,
then calls `flb_output_task_flush` at `:330`; if that returns -1, the retry
is destroyed and the next queued task is flushed (`:335-341`). The candidate:
a task whose only remaining claim was this retry keeps its task-map entry at
`users == 0`, because nothing calls `flb_task_users_release` for it.

**Setup.** With the output forced synchronous, `flb_output_task_flush`'s
non-threaded arm is asynchronous by construction (it writes an
`out_flush` pointer through `config->ch_self_events` and returns 0
immediately; the plugin callback runs on a later event-loop tick) — so a
second task's scheduled retry naturally lands in the singleplex queue's
*pending* list behind the one still marked in-progress, giving a genuine
`queued_task->retry != 0` case at `flb_output.c:330` without needing an
extra input or a raised rate.

**Script fix (recorded, fault point unchanged).** The guide's literal fault
recipe — "type `step` to enter `flb_output_task_flush`, then `return -1`" —
does not work as a scripted breakpoint `commands` block here: the call at
`flb_output.c:330-332` spans multiple source lines for argument evaluation,
and a single `step` from the line-330 breakpoint only advances within that
same call's argument lines rather than entering the callee, so the
following `return -1` would pop the wrong frame. The first attempt
(`runs/c3-fault-attempt1-discarded.log`) did exactly this, printed the
intermediate line, and stalled (SIGTERM was sent but the process never
resumed cleanly). Fixed by adding a **second** breakpoint directly at the
entry of `flb_output_task_flush`, conditioned on the queued task's pointer,
and issuing `return -1` there instead — same named function returning -1 to
the same caller at the same call site, only the gdb technique for landing
inside that frame changed.

- **Control** (`runs/c3-control.log`): `C3: queued retry flush of task
  0x7ffff0030b30, users=0` (the temporary user already dropped at :329, as
  the guide describes); the deferred flush then actually runs and succeeds
  (`[ info] [engine] flush chunk '...' succeeded at retry 1: task_id=0`), and
  the task is destroyed through the ordinary path
  (`flb_task_users_release -> flb_task_destroy`, backtrace captured,
  `del=1`) about 14 seconds before the SIGTERM sent at `12:55:20`.
- **Fault** (`runs/c3-fault.log`): `C3: queued retry flush of task
  0x7ffff0030b30, users=0 (before forced flush failure)`, then `C3: forcing
  flb_output_task_flush(task=0x7ffff0030b30) to return -1 at entry`,
  immediately followed by `[debug] [retry] task retry=0x7ffff0030cd0,
  invalidated from the scheduler` (the guide's ":335-341" branch). That
  task's id slot is never reused for a new task afterward (unlike every
  other task in the same run). At shutdown the engine's own accounting
  names it directly: `[task] dummy/dummy.0 has 2 pending task(s): task_id=0
  still running on route(s): retry/retry.0`, and it is destroyed only at
  grace-period expiry, five seconds after SIGTERM, through
  `flb_engine_destroy_tasks <- flb_input_instance_destroy` (input teardown,
  backtrace captured, `del=0`) — a different, forced path from the control's
  ordinary `flb_task_users_release` (`del=1`).
- **Classification: observed-local (simulated synchronous output).** The
  candidate reproduces: a zero-user task with no other route or retry is not
  reaped by the ordinary release path and survives only because input
  teardown destroys every mapped task unconditionally. The Codex report
  names "other active routes or retries" as a contrary guard; this run
  deliberately used a single-route setup so that guard does **not** apply,
  isolating the candidate. The teardown-reaps-everything guard **did**
  hold — the task was freed at input-instance destruction, not permanently
  lost, and no crash or use-after-free occurred.

## What none of this shows

Every fault here is a gdb-forced null pointer or forced return value on a
debug build under a debugger, standing in for a real allocation failure or a
real synchronous-output plugin that does not exist in this tree. These are
local observations of the three named candidates under those injected
premises — not upstream defect claims, not confirmed production bugs, and
not statements about production failure rates. No Quirin `Unknown` becomes
`Safe` or `Unsafe` from any of this. `FB-STALE-COMPLETION-REACHABILITY` (C4)
stays fully out of scope, and the separate semantic implementation card
`FB-RETRY-LIFETIME` stays open — a native observation does not discharge a
Rust/Lean implementation card, and no such change was made here.
