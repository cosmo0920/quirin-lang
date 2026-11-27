# W43-LIBC-RESTRICT-PROTOTYPES: prediction, frozen before any declaration

Written on 2026-09-25, on base `f8b41209`, before any contract module was
edited and before the card's arbiter ran. The only code already written is the
declared-ABI reader's array-bound step, which has not yet been run on the
corpus. Its inputs are the baseline
`docs/reports/w43-libc-result-alias/artifacts/diagnostics.json.gz`, the pinned
source and the census counts in `docs/reports/w43-libc-result-alias/abi-evidence.json`
(`restrict14`). This file is not edited after the arbiter runs. The report
compares the outcome with it.

## Baseline leaves for the ten names

The baseline has 26 `call to uncontracted function NAME` leaves for the ten
names, in 20 functions:

| Name | Leaves | Functions | Sole blocker of |
|---|---:|---:|---|
| `stat` | 3 | `cio_os_isdir`, `cio_utils_recursive_delete`, `mk_rconf_read` (provisional Unsafe) | none |
| `strftime` | 3 | `format_metric`, `mk_clock_headers_preset`, `mk_clock_log_set_time` | none |
| `gettimeofday` | 3 | `rd_kafka_log_print`, `rd_seed`, `cnd_timedwait_ms` | none |
| `getsockopt` | 1 | `mk_net_conn_create` (provisional Unsafe) | none |
| `getpeername` | 1 | `mk_socket_ip_str` | none |
| `dlsym` | 2 | `rd_dl_sym`, `mk_plugin_load_symbol` | `mk_plugin_load_symbol` |
| `cnd_timedwait` | 1 | `cnd_timedwait_ms` | none |
| `pthread_sigmask` | 10 | `rd_kafka_new` (3), `rd_kafka_background_thread_create` (3), `rd_kafka_broker_add` (3), `mk_signal_thread_sigpipe_safe` (1) | `mk_signal_thread_sigpipe_safe` |
| `regexec` | 1 | `rd_regex_match` | none |
| `regerror` | 1 | `rd_regex_match` | none |

## Predictions

**P1. Class (a), the reader change alone, moves nothing.** All five artifacts
are byte-identical to BEFORE. No selected declaration's census row changes
outcome, because no selected declaration's C prototype has an array parameter
whose bound carries a qualifier or an identifier, nor an array of qualified
pointers.

**P2. `regexec` becomes readable.** After preprocessing, glibc's
`regmatch_t __pmatch[_Restrict_arr_ _REGEX_NELTS (__nmatch)]` is
`regmatch_t __pmatch[__restrict __nmatch]` under Clang's default C17 (checked
on this host with `clang -E`). The new reader reads it, so `regexec` agrees in
its one unit, `rdregex.c`. The brief's shorter spelling `[__restrict_arr]`
omits the variable size.

**P3. `getpeername` stays withdrawn in `mk_socket.c`.** That unit defines
`_GNU_SOURCE`, so its prototype spells `__SOCKADDR_ARG`, the transparent union.
The card does not read the union (see P7). Declaring `getpeername` therefore
**renames** its one leaf: `call to uncontracted function` becomes
`whose declared contract was withdrawn (abi-unreadable-declaration)`. It
decides nothing.

**P4. Declared-callee leaves.** 25 of the 26 leaves are removed, which is every
name except `getpeername`. None of those units withdraws the declaration.

**P5. Function moves: exactly two Unknown → Safe, and nothing else.**
* `mk_plugin_load_symbol` (dlsym): `dlerror(); s = dlsym(handler, symbol); if (dlerror() != NULL) return NULL; return s;`.
  `dlerror` is already declared, and the result is `resource_return_none()`,
  so `s` carries no identity.
* `mk_signal_thread_sigpipe_safe` (pthread_sigmask): three calls on
  address-of-local actuals, the first two already declared.

The two new Safe functions supply validated W4 summaries. Each of their callers
loses one `newly-safe-callee` leaf, and both callers stay Unknown:
`mk_plugin_load` has 55 leaves and `mk_sched_launch_worker_loop` has 39.
Every other function that names a declared callee keeps at least one other
blocker and stays in its status. That includes the two provisional Unsafe
functions (`mk_rconf_read`, `mk_net_conn_create`) and `rd_regex_match`, which
keeps `regcomp`/`regfree`. No unit status moves.

**P6. Same-line and same-function movement is small and in both directions.**
A few `unmodeled call ... in branch condition` leaves on lines whose condition
calls `stat` on a plain path and `&local` may be removed:
`mk_file_get_info` (`if (stat(path, &target) == -1)`),
`mk_utils_register_pid` (`if (stat(path, &sb) == 0)`) and
`cio_os_mkpath` (`if (!stat(dir, &st))`). The lines whose actuals are member
reads (`cf->path`, `server->path_conf_root`, `host->documentroot.data`,
`rktrans->rktrans_s`, `&re->re`, `channel->fd`) keep a refusal, possibly
re-worded. A modelled call can also meet an existing gate and add a leaf, as
W43-LIBC-RESULT-ALIAS saw: the `call has an unmodeled formal address or
borrowed expression` gate at `strftime(buffer + len1, …)`, or `sizeof` at
`strftime(tmp, sizeof(tmp) - 1, …)`. Forecast: net −25 to −32 leaves, almost
all `unsupported`, with `unknown_alias` within ±3.

**P7. The transparent union is left out of scope.** Reading `__SOCKADDR_ARG`
needs a new typedef class (an untagged union of object pointers carrying
`__transparent_union__`), its own Lean model and its own attribution class.
Its only measured yield is `getpeername`'s single leaf, in a function with
seven other blockers. Its natural home is `W43-LIBC-ABI-READER-SHAPES` beside
`bind`/`connect`. So `getpeername` stays withdrawn in 93 units, and the entry
stays open.

**P8. Leak pin, candidates and families.** The leak count stays **272**.
There are 0 `invalid_release`, 0 `use_after_release` and 0 `double_release`
leaves. Every declaration is the neutral element and releases nothing. The
`summary_conflict` count stays 1.

**P9. `dlsym` is declared, not refused.** Its result is a symbol address in the
loaded object. The caller acquires no obligation, must not free it, and it is
neither fresh custody nor derived from an argument. That is
`resource_return_none()`, the `dlerror`/`gai_strerror` precedent. The
dependency on `dlclose` is recorded as unstated, and it is unobservable here
because `dlopen`/`dlclose` stay undeclared.
