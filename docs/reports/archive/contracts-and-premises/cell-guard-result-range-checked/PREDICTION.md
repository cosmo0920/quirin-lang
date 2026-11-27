# CELL-GUARD-RESULT-RANGE-CHECKED prediction

Committed after the result-set form and the C library declarations
(`86d93cf98`, the coordinator's step 2) and before the range derivation, the
null property's reading of a result set and rule 38's reading (step 3), on
base `1d94f0995`. The rows were relisted at
`baseline:e3050fe4691a6b55103e1cd51314837213e9f0e2` (the main checkout's
`private/baselines/`, every route's `resource-diagnostics.json`).

> **USER DECISION 2026-11-03** (verbatim): "C: a new result-set form that
> allows unions, such as result_set(-1 | [0, MAX]). It is the only option that
> can also state open's and timerfd_create's results (needed by
> RULE-38-VALIDITY-READING-RESULT-SET). It is also the largest grammar change.
> I choose this because we must not loosen the custody of the contracts if we
> cannot select other options."

## What can move

Three readings, all fail-closed:

1. **Rule 38's result guard** reads a caller's comparison of a cell-creating
   call's status witness (`#result == 0` creators) through the callee's
   pin-derived `nonnull_result_range` while it is in force. Only
   `mk_event_channel_create` declares one. A test decides a class exactly when,
   within the range, it classifies the class exactly (`== -1`, `< 0` under
   {-1, 0}); a test already exact on every `int` (`!= 0`) is unchanged.
2. **The null property's bound** of a call result reads a C library result set
   (`pipe`, `epoll_ctl`, `open`, `timerfd_create`) and an in-unit helper's
   derived set. It only decides a comparison the set excludes.
3. **Nothing else.** No resource custody statement, pin or lowering changes.

## The rows the range would read

Every named creation keeps the access gate's refusal before rule 38 reads its
result at the baseline (relisted per route):

| Route | Row (×units) | Leaves at the creation sites (baseline) |
|---|---|---|
| system | `flb_engine_start` (×2) | flb_engine.c:1168, :1237, :1246 `indirect memory access is outside the resource projection` |
| system | `flb_sched_create` (×2) | flb_scheduler.c:794 the same refusal; :783 (timer, `fd == -1`, exact) keeps its two rule-12t/38 leaves |
| slice | `input_thread_instance_create` (×2) | flb_input_thread.c:262, :275, :287 the same refusal |
| corpus | `mk_sched_launch_worker_loop` | mk_scheduler.c:365 the same refusal |
| corpus | `mk_server_create` | monkey.c:115, :136 the same refusal |
| corpus | `mk_server_worker_loop` | mk_server.c:502 (timer, untested) `descriptor-cell-undecided`; no range is declared on the timer |

So rule 38 reads none of these results, and the rows stay `Unknown` with the
same leaves. The `!= 0` channel callers (flb_downstream_worker.c:248,
flb_engine.c:1237, flb_input.c:1380, flb_lib.c:209, flb_output.c:1545, Monkey
monkey.c:115/:136) are exact on every `int`, so a range in force reads them
the same way (Lean `narrowing_keeps_holds`).

The null property's new reading decides only a comparison its set excludes.
For `pipe` and `epoll_ctl` that is a test separating values outside {-1, 0}
(`> 0`, `== 1`, …); for `open` and `timerfd_create` a test below -1. A census
of the analysed Fluent Bit tree's `pipe`/`epoll_ctl` calls
(`rg '\b(pipe|epoll_ctl)\s*\('` at 046accf6) finds `== -1`, `< 0`, `!= 0`
and returned results only, none of which the sets decide.

- **No `Safe` move**, **no `Unsafe` move**, **no move away from `Safe` or
  `Pending`** on any route, resource or null property.
- **Expected environment-only changes.** With the null property selected (as
  on every live route) the bounds environment now holds libc's bounds view
  (four flexible declarations) and the pinned `mk_event_channel_create` range,
  so the bounds environment digest changes; the resource environment carries
  the derived range table where a range is in force. These are environment
  changes, not moves.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
