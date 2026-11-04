# LOG-PRINT-BODY-CONTRACTS

Card brief: `private/integrator-briefs/r15-log-print.md` (round 15,
2026-11-05), card 4 of the user's contract-gap queue, with the common brief
`private/integrator-briefs/r13-common.md`. Base `945c0db1b`. The card is new;
it is registered and retired through its
[amendments](integration-amendments.json).

> **USER REQUEST 2026-11-05** (verbatim): "Now, we're still needed to fill the
> gap of the missing contracts."

**Question.** The user's `quirin-audit current` run at Fluent Bit `046accf6`
reports `cmt_log_print` (cmetrics) and `cio_log_print` (chunkio) as "call to
uncontracted function" although both bodies are in the run. Why does neither
get a summary, and what can state their effect?

## 1. Step 1: diagnosis (committed before any change)

### 1.1 Inputs

- The user's run report, read-only:
  `build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json`
  in the main checkout (186 configurations: 8 Safe, 7 provisional Unsafe,
  167 Unknown, 4 data-only).
- The published baseline `baseline:c8c7edf21` (the main checkout's
  `private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`; no baseline
  at this card's base `945c0db1b` is published yet): corpus `66910c10`,
  system `046accf6`, slice `34155955`.
- The bodies at the three pins: `lib/cmetrics/src/cmt_log.c` and
  `lib/chunkio/src/cio_log.c` are byte-identical at `046accf6`, `66910c10`
  and `34155955` (SHA-256 `24b55983…ead7b` and `bb1cbeb7…adb4`), and so is
  `lib/chunkio/src/chunkio.c` (`70e82e18…89bd`).

### 1.2 The callers

Recomputed from the run report (pairs are distinct (function, callee)):

| Callee | Sites | Functions | Calling units | Only-uncontracted functions it appears in | Functions where it is the only uncontracted callee |
|---|---|---|---|---|---|
| `cmt_log_print` | 61 | 24 (all Unknown) | 7 cmetrics units | 13 | **0** |
| `cio_log_print` | 55 | 23 (all Unknown) | 6 chunkio units | 4 | **0** |

The integrator's census counts "sole blocker of 13" and "of 4" as the
functions whose only diagnostics are uncontracted calls and which call the
callee. In each of those 17 functions at least one other uncontracted callee
remains: the 13 cmetrics functions all call `cmt_map_metric_get` or
`cmt_map_metric_get_val` (and most a `cmt_metric_*` setter), which
CMT-MAP-OPTS-CONTRACT-ADOPTION tracks; the 4 chunkio functions call
`cio_os_isdir`/`cio_os_mkpath`, `cio_file_delete`,
`cio_file_native_open`/`_close`/`mmap_file` and `adjust_layout`/`cio_file_resize`.
**A contract for either log printer frees no function by itself.**

The corpus baseline at `66910c10` has the same counts (61/24/13/0 and
55/23/4/0). The system and slice selections at `046accf6` and `34155955`
analyse no cmetrics or chunkio caller of either printer (0 sites).

### 1.3 Why there is no summary

Two independent reasons, the same at every pin:

1. **No cross-unit summary route.** W4 summaries are derived and applied
   inside one unit. Every caller is in another unit (`cmt_counter.c`,
   `cmt_gauge.c`, … / `cio_file.c`, `cio_stream.c`, …); `cmt_log.c` and
   `cio_log.c` call neither printer. That is W44-CALLEE-CROSS-UNIT-SUMMARY,
   and it would not help here anyway, because of reason 2.
2. **The home bodies are Unknown.** The callee's own row, identical at
   `046accf6` (the user's run) and `66910c10` (corpus baseline):

| Callee | Origin | Category | Refusal |
|---|---|---|---|
| `cmt_log_print` | `cmt_log.c:38` | unknown_alias | unresolved resource identity `ctx` (`struct cmt *cmt = ctx;`, a `void *` formal bound to a record pointer) |
| | `cmt_log.c:40` | unsupported | unmodeled call, aggregate or indirect access in branch condition (`if (!cmt->log_cb)`) |
| | `cmt_log.c:56` | unsupported | indirect memory access is outside the resource projection (`cmt->log_cb(ctx, level, file, line, buf)`) |
| `cio_log_print` | `cio_log.c:34` | unknown_alias | unresolved resource identity `ctx` (`struct cio_ctx *cio = ctx;`) |
| | `cio_log.c:36` | unsupported | branch condition (`if (!cio->options.log_cb)`) |
| | `cio_log.c:40` | unknown_alias | unresolved resource identity `cio` (`cio->options.log_level`) |
| | `cio_log.c:52` | unsupported | indirect memory access (`cio->options.log_cb(ctx, level, file, line, buf)`) |

Both rows depend on the declaration `vsnprintf` (in force) and carry the
null premise on the context (`null-deref:cmt_log_print:5285:cmt`,
`null-deref:cio_log_print:2112:cio`). No refusal is a `va_list`, `va_start`,
`va_end`, `vsnprintf` or global: the `va_list` protocol and `vsnprintf` read
cleanly. The decisive leaf is the **indirect call through the context's log
callback member**; the other three are the context's `void *`-to-record
binding and member reads around it.

**Is any refusal mechanical (step 2a)?** No. The summary route reads no
indirect call: rule 11p decides a call through a function pointer only over
a candidate set the unit's own AST closes, and `cmt->log_cb` is a member
read through a formal (CG22-OPEN-INDIRECT-CALLS). Admitting it in a body
would be the run-wide member closure of rule 12n applied to an inferred
summary, a new reading, and the callers still could not use a home-unit
summary (reason 1). So step 2 is a contract (2b).

### 1.4 What the bodies do (the effect a contract must state)

`cmt_log_print(void *ctx, int level, const char *file, int line, const char *fmt, ...)`
(`lib/cmetrics/src/cmt_log.c:32-57`) and `cio_log_print` (same signature,
`lib/chunkio/src/cio_log.c:28-53`):

- read `ctx` as `struct cmt *` / `struct cio_ctx *`: its `log_cb` member
  (`cio->options.log_cb`) and `log_level`; return when the callback is null
  or the level is above the context's;
- render `fmt` and the variadic tail with `vsnprintf` into a local
  `char buf[CMT_LOG_BUF_SIZE]` (`va_start`/`va_end` around it);
- call the context's callback as `log_cb(ctx, level, file, line, buf)`.

Nothing is freed, stored or returned. Every fixed formal and every tail
actual is only read. **The one effect beyond borrowing is the callback**,
which the context's member holds. In the contract vocabulary that is
`resource_callback_member_invoked(ctx, TAG.FIELD, ctx, level, file, line, _)`
(C04.19.3, rule 12n), the last position being the callee's own buffer.

The member's stores in the three pinned trees (every spelling of `log_cb`
outside tests, librdkafka, Monkey and LuaJIT, identical at all three pins):

- `cmt.log_cb` (`struct cmt`, `cmetrics.h:54`): **no store anywhere**.
  `cmt_create` allocates the context with `calloc` and never sets it; no
  setter exists. Over the run's facts the member's set is expected to be
  closed and empty: the callback is never called.
- `cio_options.log_cb` (the innermost named record of `cio->options.log_cb`,
  `chunkio.h:79`): Fluent Bit's `opts.log_cb = log_cb;` in
  `src/flb_storage.c:709` and `src/flb_fstore.c:492` (two different static
  functions named `log_cb`), the chunkio tool `lib/chunkio/tools/cio.c:598`
  (its own static `log_cb`; the unit is in the run's compile database),
  `options->log_cb = NULL` (`chunkio.c:78`), and the setter
  `cio_set_log_callback(struct cio_ctx *ctx, void (*log_cb))`
  (`chunkio.c:316-319`), whose formal is a `void *`, not a function
  pointer. Rule 12n's reading opens a member stored from a formal that is
  not a function pointer ("a formal that is not a function pointer",
  `callback_member_closure.rs`), so the set is expected to be **open**.
