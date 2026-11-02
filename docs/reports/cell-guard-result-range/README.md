# CELL-GUARD-RESULT-RANGE

Card brief: `private/integrator-briefs/cell-guard-result-range.md` and
`r7-common.md` (round 7, 2026-11-03). Base and baseline `e3050fe46`.

> **USER DIRECTION 2026-11-03** (verbatim): "We have to clarify why the
> difference is caused on our contracts." then "Queue the card".
>
> **USER DIRECTION 2026-11-03** (verbatim, added before launch): "The Fluent
> Bit's caller tests sometimes check the incorrect assertions so we need to
> pay attention for the assertions and actual returning values of Fluent Bit
> functions."

**Status: stopped at a user decision (§4), after the census.** No contract,
gateway, Lean or test code has changed. The census answers the user's
question (§1–§2) and found the decision the brief's rules leave open: a
range of Monkey's channel creator can be body-checked for the configuration
Fluent Bit builds only with the C library's result sets, and no route that
holds a Fluent Bit caller analyses that body.

## 1. Why the contract and the callers differ

`mk_event_channel_create` forwards to its backend's `_mk_event_channel_create`
(`mk_event.c:189`, the backend file `#include`d at `:31-52`). Which backend
is compiled is a build fact:

- **Fluent Bit's builds** (all three trees) define no `MK_EVENT_LOOP_*` and
  are Linux builds, so `mk_event.c:44` includes **epoll**
  (`mk_core_info.h` defines only `MK_HAVE_TIMERFD_CREATE`).
- **The corpus's Monkey build** defines `MK_EVENT_LOOP_LIBEVENT`
  (`monkey-build/.../mk_core_info.h`): the `mk_core/mk_event.c` the corpus
  analyses is the **libevent** backend. (The provider comment in
  `event_loop.qv` calls epoll "the analysed configuration"; it is the
  production configuration, not the analysed one. §6.)

Every backend's body, read at the three pins, through every helper its
result comes from (`census/bodies.json`):

| Backend | Result set from the bodies alone | With the C library's result sets |
|---|---|---|
| epoll (Fluent Bit) | every `int` | {-1, 0} (pipe, epoll_ctl) |
| kqueue | [MIN, 0] | {-1, 0} (pipe, kevent) |
| select, poll | [MIN, 0] | {-1, 0} (pipe) |
| libevent (corpus) | {-1, 0} | {-1, 0} |

In **every** backend, from the bodies alone, the call succeeds exactly when it
returns 0: each failure arm returns a nonzero value and the success arm
returns the constant 0. That is the contract's split, `#result == 0` /
`!= 0` (D3): it is exact without any assumption.

The epoll body alone allows every `int` because its failure arms return
other functions' results unchanged: `return ret` after `pipe` failed
(`ret < 0`, any negative), and `return ret` after `_mk_event_add` failed, where
that `ret` is `-1` or `epoll_ctl`'s result after `ret < 0` was refuted (any
non-negative value). Only POSIX's `pipe` (0 or -1) and Linux's `epoll_ctl`
(0 or -1) make every failure exactly -1.

So Fluent Bit's `ret == -1` (nine sites) is **correct but redundant**: it
equals the contract's `!= 0` on {-1, 0}, and {-1, 0} holds only with the C
library's result sets. Rule 38 decides the cells from the contract and the
body alone, so on `== -1`'s false edge it knows only `ret != -1`, which does
not decide `ret == 0`. The `!= 0` callers (five sites) are exact and decide
today.

## 2. The census

`census/census.py` is a lexical reader, bound to every file it reads (one
SHA-256 tree digest per tree, and the digest of every file holding a body or
a site). The readings are `census/bodies.json` (each body's returns, keyed
by file, function and body digest) and `census/sites.json` (each site's
failure edge, keyed by revision-independent text). The tool computes every
class from the readings, the test's predicate and the body-derived result
set, and refuses an unread body, a reading whose returns differ from the
body's, an unannotated site, an unused annotation or a changed digest:

```sh
python3 -I docs/reports/cell-guard-result-range/census/census.py \
  --root 66910c10=<corpus fluent-bit-clang-source> \
  --root 046accf6=<system fluent-bit-clang-source> \
  --root 34155955=<slice fluent-bit-clang-source> \
  --root monkey-66910c10=<corpus monkey/monkey-source> \
  --check docs/reports/cell-guard-result-range/census/census.json
```

It exits 0 with 0 problems (`census.json` sha256
`4c22ec16ea4699095deba8ebd0c16235e71026b9531073bb10ecc7f327dcb44f`). Trees:
66910c10 (4,927 files, digest `97197d3e…`), 046accf6 (4,936, `07e7d39d…`),
34155955 (4,940, `d9214d14…`), Monkey 66910c10 (492, `f23d3edb…`).

**Classes.** A test is `exact` when its failure set equals the contract's on
every `int`, `redundant` when they differ only outside the result set,
`wrong` when a value of the result set is classified otherwise (a failure
read as success, a success read as failure) or the test compares against a
value the body never returns, and `untested` when nothing tests the result.
Each is computed with the C library's result sets (the actual set) and from
the bodies alone.

**Immediate tests of the result** (part 3; 046accf6, equal to 34155955;
66910c10 lacks the Monkey emscripten test):

| Callee | Test | Sites | Class (C library) | Class (bodies alone) | Decides today |
|---|---|---|---|---|---|
| channel | `ret != 0` | flb_downstream_worker.c:248, flb_engine.c:1237, flb_input.c:1380, flb_lib.c:209, flb_output.c:1545; Monkey monkey.c:115, :136 | exact | exact | yes |
| channel | `ret == -1` | flb_engine.c:1168, :1246; flb_input_thread.c:262, :275, :287; flb_output_thread.c:238, :537, :553; flb_scheduler.c:794 | redundant | wrong ([MIN,-2], [1,MAX] read as success) | no |
| channel | `ret < 0` | Monkey mk_scheduler.c:365 | redundant | wrong under epoll; redundant under libevent | no |
| timer | `fd == -1` (and `config->flush_fd`, `config->shutdown_fd` `== -1`) | flb_engine.c:325, :337, :1498; flb_input.c:2351, :3248; flb_metrics_exporter.c:234; flb_scheduler.c:133, :672, :783; flb_sp.c:519, :535, :2194 | exact | exact | yes (plain names) |
| timer | `timeout_fd > 0` (first test, :585) | Monkey mk_server.c:502 | **wrong** (0 read as failure) | wrong | no |
| both | Monkey test programs | test/event_timeout.c:47 (`fd >= 0`), test/wasm/events.c (6 `CHECK`s), test/lib_server.c:160 | exact / redundant; lib_server **untested** | | |

Totals with the C library's result sets: 66910c10 19 exact, 11 redundant, 1
wrong, 1 untested; 046accf6 and 34155955 23 exact, 13 redundant, 1 wrong, 1
untested; the corpus's Monkey 2 exact, 2 redundant, 1 wrong, 1 untested.
**No Fluent Bit (`src/`) test of a creator's immediate result is wrong**: 26
sites, 17 exact and 9 redundant.

**Tests of the stored result** (part 4): every comparison of a member (or the
Monkey local) that holds a creator's descriptor. 29 per Fluent Bit tree:
5 exact, 4 outside (`in_exec`'s and `in_exec_wasi`'s `ch_manager`, which
`flb_pipe_create` writes), **20 wrong**; 3 in Monkey's tree, all wrong. Every
wrong one reads descriptor 0, a success value under the C library's result
sets, as absent.

### Candidate defects (source readings only; nothing filed upstream)

Each is reachable only when the creator returns descriptor 0, which needs
descriptor 0 free in the process (stdin closed at start; `flb_utils_set_daemon`
closes only stdout and stderr).

- **`flb_input_collector_destroy` (flb_input.c:3145 at 046accf6, :3125 at
  66910c10), `coll->fd_timer > 0`.** The strongest: the holder's own absent
  value is -1 (`coll->fd_timer = -1` at :2242 and :3181), so the guard is not
  the sentinel's. A COLLECT_TIME timer on descriptor 0 is never destroyed,
  and the collector holding its registered event is then freed while the
  loop still holds the registration.
- **The channel destroyers' `> 0` ends**: flb_input.c:1311/:1315,
  flb_output.c:615/:619, flb_input_thread.c:210/:213/:218/:221,
  flb_config.c:617/:625/:627. The absent value is calloc's 0, so these
  records' protocols cannot tell an absent end from descriptor 0 (the round-6
  census reads these destroyers as breaking any protocol). An end on
  descriptor 0 is never closed.
- **`flb_engine_started` / `flb_engine_failed`** (flb_engine.c:1034, :1048),
  `config->ch_notif[1] <= 0`: a write end on 0 is reported as no channel.
- **`flb_engine_start`'s shutdown timer** (flb_engine.c:1497 `<= 0`, :1563
  `> 0`): a timer on descriptor 0 is created again on the next pass and
  never destroyed.
- **`flb_sp_window_destroy`** (flb_sp.c:670), `task->window.fd > 0`.
- **Monkey** (Fluent Bit's `lib/monkey` and the corpus's Monkey):
  `mk_server_worker_loop` never tests the timer at :502 and destroys it only
  under `timeout_fd > 0` (:585); `mk_server_lib_notify_event_loop_break`
  (:630) and `mk_server_lib_notify_started` (:648) read a write end on 0 as
  no channel; the test program `test/lib_server.c:160` overwrites `ret`
  (:167) before testing it.
- **Monkey backends that ignore a failed registration**: the timer arms of
  epoll without timerfd (:319), select (:253), poll at 66910c10 (:252; fixed
  at 046accf6) and **libevent** (:369, :375, the corpus's analysed
  configuration) return the descriptor even when `_mk_event_add` (or
  `event_add`) failed.

Two tests are exact: `flb_sched_destroy`'s `sched->ch_events[0] != -1` and
`flb_sched_timer_cb_disable`'s `timer->timer_fd != -1`, whose holders store
-1 as their absent value.

## 3. What a range would move today: nothing

Every creation the brief names is refused before rule 38 reads its result
(baseline `86b26e5b2`, which `e3050fe46` does not change for these rows):

- `flb_engine_start` (system): the creations at :1168, :1237, :1246 each keep
  `indirect memory access is outside the resource projection`.
- `input_thread_instance_create` (slice, ×2 units): :262, :275, :287, the
  same.
- `flb_sched_create` (system, ×2 units): :794, the same refusal. Its timer
  at :783 is tested `fd == -1`, which is exact and already decides.
- The corpus's Monkey callers: `mk_sched_launch_worker_loop` :365 and
  `mk_server_create` :115, :136, the same refusal. `mk_server_worker_loop`
  :502 tests nothing, so it keeps `descriptor-cell-undecided`.

So under every option below no status moves, and no leaf at these creators
changes, until the access gate admits these creations. The integrator's
`baseline:e3050fe46` relisting (PREDICTION.md) is not done: it was not yet
published when this stop was reached, and the prediction depends on the
decision.

## 4. The decision (the user's)

The brief's rules are: state the range "only as a body-checked range", and
"a range out of force (withdrawn, unchecked, refuted) changes nothing". Two
facts make those rules undecidable for Fluent Bit's callers as the tool
stands.

**Q1. The C library's result sets.** For epoll, the configuration Fluent Bit
builds, {-1, 0} follows from the body only with `pipe` ∈ {-1, 0} (POSIX) and
`epoll_ctl` ∈ {-1, 0} (Linux). Today a call's result is its C type's range in
every body check (C06.5, `null_deref/range.rs`), and bounds contract v1 has
no flexible mode, so a libc result range cannot be declared (a pinned bounds
declaration needs sites in the audited source).

- **(a) Recommended.** Declare those result sets as libc contracts in
  `quirin-provider-libc`, flexible, under the standing assumption
  `libc-c-standard-abi`, like every other libc contract. This needs a
  flexible mode for libc result ranges, and the range check must read a call
  result through a declared range and through an in-unit helper's summary.
- **(b)** Use only what the bodies alone prove. That is {-1, 0} for libevent
  (the corpus) and nothing for epoll, so no Fluent Bit `== -1` test ever
  decides.

**Q2. Which body checks the range at Fluent Bit's callers.** System and slice
analyse no Monkey unit. Only the corpus analyses `mk_event.c`, and it does so
in the libevent configuration, whose standing says nothing about epoll.

- **(i)** Add Fluent Bit's `lib/monkey/mk_core/mk_event.c` (epoll) to the
  system and slice selections. Their figures then change (one new unit
  each, whose rows are new), measured separately. A run-wide standing must then be known before the
  callers are analysed, which is a phase like C06.4's.
- **(ii) Recommended.** A pin-time derivation. `quirin-audit pin --bounds`
  derives the range from the configured backend's body chain at each
  reviewed revision. The lock records the sites of every helper the
  derivation read (`_mk_event_channel_create`, `_mk_event_add` in
  `mk_event_epoll.c`), so the range is in force exactly where the audited
  source matches. This is known before any unit is analysed, as every pin
  is.
- **(iii)** The brief's rule as written. Only a same-run analysed body
  checks the range, and Fluent Bit's callers never decide. Without a phase,
  the corpus's Monkey callers (other units) do not decide either.

**Q3. Scope.** By §3, no row moves under any choice today. So either:

- implement the chosen Q1/Q2 in this card (the gateway reading, the rule-38
  package arm, the Lean module and the fixtures follow the brief unchanged);
  or
- land this census and answer now, and queue the implementation behind the
  access-gate refusal at the creators.

**Recommended: land the census now, and queue the implementation with (a)
and (ii)** as one follow-up. That follow-up is
`CELL-GUARD-RESULT-RANGE-CHECKED` or this card resumed.

## 5. Not changed

The census changes no source, contract, lock, gateway path or Lean module.
It reads the trees only. The census's own fail-closed checks are its tests.

## 6. Observations (not findings)

- **The provider comment names the wrong configuration.**
  `providers/qcore-provider-libevent/contracts/monkey/event_loop.qv` (header
  comment) calls epoll "the analysed configuration". The corpus analyses the
  libevent build; epoll is Fluent Bit's production configuration, which no
  route analyses. The declarations' states hold for both (the census's
  part 1 covers all five backends).
- **Rule 38's validity reading already assumes a result set.** For a
  descriptor witness (`!= -1` / `== -1`), W43's `facts` table reads `fd < 0`
  as `fd == -1` and `fd >= 0` as `fd != -1`, "for a descriptor whose values
  are -1 or >= 0". From the bodies alone, the timer's result set is every
  `int` in every backend (§2: `return timer_fd`, `return fd`, `return fd[0]`
  after a test of `-1` only). It is {-1} ∪ [0, MAX] only with the C
  library's result sets. No Fluent Bit site tests the timer that way today:
  every `src/` test is `== -1`. Only Monkey's test programs (`fd >= 0`) do.
  Any decision on Q1 applies here too.
