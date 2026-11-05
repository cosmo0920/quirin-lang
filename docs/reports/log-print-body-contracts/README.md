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

That last expectation did not hold, and step 2 found why (§2.2): the
reading classified this `void *` formal as a function pointer.

## 2. Step 2: what changed

**Answer.** Both effects can be stated in resource-v1 except the variadic
tail, so each callee gets a pinned declaration of its five fixed parameters
and its callback. The fixture run of those declarations exposed a fail-closed
defect in rule 12n's reading, which this card repairs. Neither declaration
decides a call on any route today, because every invoked member is open
there for another reason: 36 unread units (§3).

### 2.1 The declarations (step 2b)

| Callee | Module | Callback clause | Body evidence | Pins |
|---|---|---|---|---|
| `cmt_log_print` | `providers/qcore-provider-cmetrics/contracts/cmetrics/logging.qv` | `resource_callback_member_invoked(ctx, cmt.log_cb, ctx, level, file, line, _)` | none: declared assumption | `046accf6`, `66910c10` |
| `cio_log_print` | `providers/qcore-provider-chunkio/contracts/chunkio/logging.qv` | `resource_callback_member_invoked(ctx, cio_options.log_cb, ctx, level, file, line, _)` | none: declared assumption | `046accf6`, `66910c10` |

Both state `ctx`, `level`, `file`, `line` and `fmt` `resource_borrowed`,
and `resource_return_none()`. Placement: each library's own provider crate.
The brief's `quirin-provider-cmetrics`/`-chunkio` are the existing
`qcore-provider-*` crates, and no libc name is declared. ABI: `void *` is
`ptr<mut, u8>`, `const char *` is `ptr<const, u8>`, `int` is `i32`.
`providers/contracts.py sync` regenerated the views, inventories and
`contracts.json`; `quirin-audit pin` added the two entries to the crates'
`contracts.lock` at both revisions (sites: the `cmt_log.h`/`cio_log.h`
declarations and the bodies). The route manifest
`cycle19-resource-contracts.json` lists both modules, so it again equals the
union of the resource views. Pinned counts moved deliberately, each with its
comment: selected resource modules 66 -> 68 and v1 declarations 499 -> 501
(`compiler/tests/contract_loader.rs`, `crates/qcore-audit/tests/provider_selection.rs`,
`crates/qcore-c-gateway/tests/{prod_contracts,neighbours}.rs`), withheld
(retaining) declarations 109 -> 111, corpus modules 147 -> 149 (qcore-provider
modules 89 -> 91, `scripts/tests/test_contract_corpus.py`), and the
`cmetrics-counter` profile's pinned digest of the crate's `contracts.json`
(`contract/profiles/cmetrics-counter/source-contracts.json`).

- **Body-checked or declared?** Declared. Both bodies are in the reviewed
  compile database, but rule 13i lets no body evidence cover a member
  invocation, and both home rows are Unknown (§1.3). Every row resting on
  either declaration names it in its contract dependencies.
- **No libc dependency.** The bodies' `vsnprintf` is already declared and in
  force on this route, and `va_start`/`va_end` raise no refusal. Nothing
  waits on LIBC-GAP-DECLARATIONS.
- **The variadic tail is not declared.** The truthful statement adds
  `resource_variadic_borrowed_tail()`: `vsnprintf` renders the tail into the
  callee's own buffer, and no tail pointer reaches the callback. The loader
  refuses that clause beside a callback clause with "a retention clause
  stands only beside plain parameter and result predicates" (the retaining
  branch of `contract_loader.rs`; the control
  `the_loader_refuses_a_variadic_tail_beside_a_callback_clause` pins it).
  Leaving the tail out is the refusing direction: in
  `legacy_declared_abi.rs`, a variadic C function whose declaration has no
  tail clause is no ABI disagreement. A call with an actual after `fmt`
  keeps the refusal "arguments beyond the declared parameters of
  `cmt_log_print` are outside the resource projection" (measured in the
  fixture). By source reading, 18 of the 61 `cmt_log_print` sites and 38 of
  the chunkio calls pass a tail actual. The missing form is
  VARIADIC-TAIL-BESIDE-CALLBACK-CLAUSES.
- **The callback is never read as benign.** The consumer is rule 12n over
  the run's closed set, with rule 13u's loop-safe composition. An open set
  refuses (`callback-member-open`), an effectful candidate refuses
  (`callback-effectful`), and only a closed empty set invokes nothing.

### 2.2 The reading repair (step 2a, rule 12n)

The fixture run of the chunkio declaration closed `cio_options.log_cb`
**empty** (`Ok({})`) over a unit holding only the setter
`void cio_set_log_callback(struct cio_ctx *ctx, void (*log_cb)) { ctx->options.log_cb = log_cb; }`.
`MemberFacts::function_pointer` read every type spelling containing `(*)`
as a function pointer. Clang keeps declarator parentheses as sugar, so it
spells the `void *` formal `void (*)`. The formal was recorded as a
function-pointer formal (`Formal { cio_set_log_callback, 1 }`), whose values
are its callers' function-pointer actuals. A `void *` actual records none,
so a member stored from such a formal could close, empty or partial. Over a
run whose other units were all read, a call of `cio_log_print` would then
have been read as invoking no callback, or only the recorded ones.

The repair (`callback_member_closure.rs`): a spelling is a function pointer
only with a parameter list after `(*)` (`spells_function_pointer`), and a
typedef is a function typedef only when it spells a function type
(`spells_function_type`). The `void (*)` formal now opens the member ("a
formal that is not a function pointer"), as a formal spelled `void *`
always did, and an array pointer `T (*)[N]` is no longer a callback member.
The repair only opens sets: the stores and actuals it stops recording are
of non-function type, which premise P-CG22-MEMBER-CLOSED-WORLD already
excludes for the plain `void *` spelling. Rule 12n is amended in
`docs/agents/rules/12n.md`. Lean: `QCore.MemberFormalType`
(`a_non_function_formal_store_is_open`, `an_open_unit_keeps_the_member_open`,
`a_non_function_formal_store_never_decides`,
`the_defect_would_close_the_member_empty`), audited in
`proof/AuditMemberFormalType.lean`. Which C spelling is a function pointer
stays the reading's rule-28 extraction premise.

### 2.3 Controls

`crates/qcore-c-gateway/tests/log_print_member_contracts.rs` runs over the
shipped modules (`include_str!`) and records shaped as the pinned headers:

| Test | Shows |
|---|---|
| `a_member_no_unit_stores_invokes_nothing_over_the_run` | `cmt.log_cb` closes empty over the run, so a call without a tail actual is Safe; without the run's facts, or with one unread unit, it is refused (`callback-member-open`) |
| `a_tail_actual_stays_refused` | a call with a tail actual keeps the arity refusal |
| `an_effectful_candidate_is_refused` | a registered candidate that releases `ctx` is refused (`callback-effectful`) |
| `a_member_stored_from_a_void_pointer_formal_is_open` | the repair: the `void (*)` setter formal opens `cio_options.log_cb` (with a `void *` caller), and the call is refused; a real function-pointer formal still forwards its caller's designator |
| `the_loader_refuses_a_variadic_tail_beside_a_callback_clause` | the missing form |

Before the repair the fourth test failed with `Ok({})`.

## 3. Why nothing is decided today

Rule 12n decides over the run's closed set. On the corpus baseline at
`c8c7edf21` and in the user's `current` run, the member pre-pass leaves 36
units unread: 34 preprocessing failures of ungenerated sources
(`lib/luajit-cmake/lj_vm.S`, the bison/flex outputs of `src/record_accessor`,
`src/stream_processor/parser` and `plugins/processor_sql/parser`, and others)
and 2 C++ units. An unread unit opens every member, and both sidecars list
every invoked member as "36 unit(s) of the run may name it but were not
read". That is S3-PREPASS-UNREAD-GRAPH-ISOLATION (and
W47-PREDEFINED-FACTS-UNGENERATED-SOURCES for the sources). So every call of
either printer stays refused on every route. The leaf now reads
`callback-member-open`, or the arity refusal, instead of "call to
uncontracted function".

When that closes:

- `cmt_log_print`: no unit stores into `cmt.log_cb` (the spelling census of
  §1.4, over all three trees). The calls without a tail actual are then
  decided as the identity (the first control).
- `cio_log_print`: still open, from the setter's `void *` formal, now read
  correctly. Closing it needs MEMBER-CLOSURE-VOID-POINTER-SETTER-FORMAL: a
  sound forwarding of a `void *` formal's actuals. Here the only call,
  `cio_create`'s `cio_set_log_callback(ctx, options->log_cb)`, passes the
  member itself. It also needs contracts for the three candidates, the
  static `log_cb` functions of `src/flb_storage.c:508`, `src/flb_fstore.c:27`
  and `lib/chunkio/tools/cio.c` (the first two only log `str` through
  `flb_error`/`flb_warn`/`flb_info`/`flb_debug` and return 0).

Even then the yield is small: each of the 17 only-uncontracted functions
has another uncontracted callee (§1.2).

## 4. Prediction and the user's run

[PREDICTION.md](PREDICTION.md) (version 2, committed before the
implementation, with version 1 kept in it) predicts no status move on
corpus, system or slice. On the user's `current` run both callees leave the
uncontracted list (2,790 -> 2,788 callees, 7,264 -> 7,217 pairs). The 17
only-uncontracted functions leave that set, because the replacing refusal
is not an uncontracted call (508 -> 491), with no status change. Unit
statuses stay 8 / 7 / 167 / 4.

## 5. Follow-ups

- **VARIADIC-TAIL-BESIDE-CALLBACK-CLAUSES** (new). The resource loader
  refuses a variadic tail clause beside any callback clause, so a variadic
  callee that also invokes a callback cannot state its tail. Discharge: the
  loader admits the tail beside member and synchronous invocations, route G
  applies rule 11w's per-site tail reading to such a declaration, Lean
  parity (a borrowed tail composed with an identity callback is the
  identity), measured. Size: 18 `cmt_log_print` and 38 `cio_log_print`
  sites by source reading.
- **MEMBER-CLOSURE-VOID-POINTER-SETTER-FORMAL** (new). Rule 12n opens a
  member stored from a formal that is not a function pointer; chunkio's
  `cio_set_log_callback(struct cio_ctx *, void (*log_cb))` is the case.
  Discharge: forward a non-function formal's values soundly (every actual
  at that position of every call, under the address-taken rule), Lean
  parity, controls; and resource-v1 contracts for the three static `log_cb`
  candidates.
- **W46-METRIC-TRACE-CALLBACK-PREMISES** (amended). The callback effect of
  `cmt_log_print` is now stated (C04.19.3); the tail form and the run's
  open set remain.
- **W46-FLUENT-METADATA-CALLBACK-EFFECT** (amended). The chunkio log
  callback is now stated on `cio_log_print`; the open set remains.
- Outside this card: `ctr_log_print` (ctraces, `lib/ctraces/src/ctr_log.c`)
  has the same shape, and no unit stores into `ctr.log_cb` either. The
  user's run reports no uncontracted site of it, so it is left to the
  contract-gap queue.

**Measurement (integrator, 2026-11-05, combined).** This card was merged by
the user together with CELL-PROTOCOL-RELEASE-WINDOW-FRAMES,
CFL-MPACK-EXTERNAL-DECLARATIONS, LIBC-GAP-DECLARATIONS,
LOG-PRINT-BODY-CONTRACTS, LIBRDKAFKA-HEADER-INLINES,
MEMBER-ACCESS-PROJECTION and CURRENT-RUN-DEFECTS (main `25508ccaf`, records
`8531b005b`, full gate passed), so one undeclared measurement covers them:
tip `rev:8531b005b` against `baseline:c8c7edf21`, output
`private/measure/integrate-r4-20261105T111129Z`, exit 0, 145 min. System
and slice: 0 moves. Corpus: 38 moves (Safe +37, Unsafe +1), 2 unit moves
(`ctr_attributes.c` -> Safe, `cmt_decode_statsd.c` -> provisional Unsafe), 0
unmatched. Every move declared by a merged card's PREDICTION.md happened. 29
moves are declared there; `_rd_strcasestr` is LIBC-GAP's recorded extra; 8
are cascades no single-card prediction counted, each with no leaf left at
the tip:
- `crc32c_init_hw` (crc32c.c): its only base leaves were the two calls to
  `crc32c_zeros`, which MEMBER-ACCESS-PROJECTION makes Safe (in-unit summary).
- `rd_kafka_mock_Produce_reply_tags_partition_write`,
  `rd_kafka_msgset_writer_write_MessageSet_v2_header`,
  `rd_kafka_msgset_writer_write_Produce_header`: base leaves were calls to
  the `rd_kafka_buf_write_*` writers this card declares, plus `unresolved
  resource identity` and member-store `indirect memory access` leaves on
  their results (`msetw->msetw_of_start = rd_kafka_buf_write_i64(rkbuf, 0)`,
  a scalar offset) that were artefacts of the undeclared call.
- `rd_kafka_msg_partitioner_consistent` (`rd_crc32_init`/`rd_crc32_update` in
  its return expression), `rd_kafka_outq_len` and `rd_kafka_queue_length`
  (`rd_kafka_q_len(rkqu->rkqu_q)` in a return): the return-expression and
  argument-member leaves were on calls this card declares borrowing.
- `rd_kafka_partition_leader_destroy_free(void *ptr)`: passes its formal (as
  `leader = ptr`) to `rd_kafka_partition_leader_destroy`, declared here
  `resource_released(leader, nonnull)`: a release of its own formal, as the
  base's `unresolved resource identity ptr` leaf could not read.
[integrate-r4-native-comparison.json](../librdkafka-header-inlines/integrate-r4-native-comparison.json)
(sha256 `01ab43b871fb84afdc0710b3326a00c5dc67ea785daad61d2a22f0d7519eb9f5`).

