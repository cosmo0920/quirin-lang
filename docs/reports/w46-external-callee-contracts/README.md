# W46-EXTERNAL-CALLEE-CONTRACTS: contracts for the external callees that block the most functions

Card W46-EXTERNAL-CALLEE-CONTRACTS, 2026-09-27, base local `main` `85a35431`
("Record the w45-sr5-soundness integration gate"). The external arm of P4
(CALLEE-CLOSURE) in the [unsupported-coverage plan](../../plans/unsupported-coverage/README.md).
The [prediction](PREDICTION.md) was frozen and committed (`9f94581f`) before any
provider file was edited. Gateway rule [10j](../../agents/gateway.md#constraint-10j).

## Headline

- **Ranking.** 385 external callees (no definition in any corpus unit) carry
  2,277 uncontracted call sites. Only **33** Unknown functions are blocked by
  one external callee alone, and **no callee blocks more than 2**, so the
  brief's "at least 3 alone" tier is empty. Every libc/POSIX callee in the top
  30 had already been declined by an earlier card for a named vocabulary or
  reader gap, except `fcntl`.
- **Nine contracts**, each read from its body at both audited revisions
  (`66910c10` and `046accf6`, identical there) and each the neutral element
  (every argument borrowed, result none), in existing effect kinds:

  | Callee | Crate, module | Mode |
  |---|---|---|
  | `fcntl` at `F_SETFD` only | `quirin-provider-libc`, `libc/unistd.qcore` | flexible, premise `cmd == 2` |
  | `mk_utils_libc_error` | `qcore-provider-libevent`, `monkey/logging.qcore` | pinned |
  | `rd_atomic32_get` | `qcore-provider-librdkafka`, `librdkafka/memory.qcore` | pinned |
  | `cfl_sds_len` | `quirin-provider-cfl`, `cfl/sds_lifetime.qcore` | pinned |
  | `mpack_finish_map`, `mpack_finish_array`, `mpack_write_true`, `mpack_write_false`, `mpack_write_bin` | `quirin-provider-mpack`, `mpack/writer.qcore` | pinned |

- **Corpus arbiter: +7 Safe.** Functions **398 / 9 / 4,427 → 405 / 9 / 4,420**
  (Safe / provisional Unsafe / Unknown); units unchanged (5 / 8 / 168 / 4);
  leak 34 unchanged; unknown_alias 3,571 → 3,572; unsupported 41,314 → 41,239.
  Every leaf and every move is attributed; **no correction** (no Safe →
  non-Safe move); **no new leak or violation**.
- **Fluent Bit slice at `046accf6`:** no verdict moves (units 0 / 2 / 13,
  functions 28 / 2 / 204); one site of `flb_engine_start` (two configurations)
  exchanges an indirect-access leaf for `call to `fcntl` outside its declared
  literal-argument premise`.
- **Lean:** none. Every clause is in an existing effect kind (borrowed, return
  none, literal premise, variadic data tail), so no new theorem is needed.
- **Follow-ups 505 → 509:** four registered with their sizes, four amended with
  this card's measured part, none retired.

## 1. Ranking (measured first)

[`scripts/rank_external.py`](scripts/rank_external.py) reads the baseline
`w45-container-root-reach/integration/artifacts/diagnostics.json.gz`. A callee is
*external* when no unit lists a function of that name among its analysed
functions or `source_functions`; the plan's 390 / 2,284 used a different
"defined" test, and this card's 385 / 2,277 are its own. For each callee it
counts the functions it blocks **alone** (every diagnostic of an Unknown
function, in every category, is an uncontracted call to it), the functions it
**co-blocks** (every diagnostic is an uncontracted call, it among several
callees), those co-blocked only by external callees, and its sites.
[`scripts/classify_owners.py`](scripts/classify_owners.py) classifies owners
from `libc-names.txt` and a textual declaration search of the pinned source
([owners.json](artifacts/attribution/owners.json)): libc/POSIX 35, bundled
libraries 216 (librdkafka 103, cfl 33, cmetrics' vendored mpack 33, Monkey 32,
zstd 7, others 8), compiler builtins 15, Fluent Bit functions outside the units
14, and 105 without a declaration in the tree (OpenSSL, libcurl, generated
protobuf-c, Cyrus SASL, and function-pointer formals called by name).
Full tables: [before](artifacts/attribution/ranking-before.txt),
[after](artifacts/attribution/ranking-after.txt),
[JSON](artifacts/attribution/ranking-before.json.gz).

The top 30 by functions alone, with the decision for each:

| Callee | Alone / co | Decision |
|---|---:|---|
| `cfl_sds_cat_safe` | 2 / 6 | declined: in-place reallocation through a `cfl_sds_t *` slot (`W46-SDS-CAT-SAFE-IN-PLACE-REALLOC`) |
| `mk_utils_libc_error` | 2 / 0 | **declared** |
| `_mk_event_wait_2` | 2 / 0 | declined: backend-selected `static inline` bodies included into `mk_event.c` (select, epoll, kqueue, libevent, poll), five different bodies that are not analysed functions |
| `pthread_setspecific`, `pthread_getspecific` | 1 / 1, 1 / 0 | declined earlier: retained pointer and its alias (`W43-LIBC-CALLBACK-RETAIN`, `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`) |
| `pthread_key_create`, `pthread_rwlock_init`, `pthread_rwlock_destroy` | 1 each | declined earlier: in-place lifecycle (`W43-LIBC-IN-PLACE-LIFECYCLE`) |
| `pthread_once` | 1 | declined earlier: caller-supplied function |
| `dlopen`, `dlclose` | 1 each | declined earlier: no library-handle kind (`W44-DLOPEN-DECLARATION-REMEASURE`) |
| `bind` | 1 | declined earlier: `__CONST_SOCKADDR_ARG` transparent union, withdrawn by the reader |
| `fcntl` | 1 | **declared at `F_SETFD` only** (§2) |
| `rd_kafka_q_destroy_owner`, `rd_kafka_toppar_keep0` | 1 each | declined: a reference-count drop and add (`rdkafka_queue.h:253`, `rdkafka_partition.h:533`), which the heap kind would misstate |
| `ERR_clear_error`, `curl_global_init` | 1 each | declined: OpenSSL and libcurl are outside the audited source, so a pinned contract has no site to review; a flexible mode for a non-libc library is a product decision |
| `__atomic_*` (3), `__builtin_ctz`, `__builtin_ctzll`, `__builtin_nanf` | 1 each | declined: compiler builtins, no prototype, no owning crate |
| `_mk_event_add`, `_mk_event_inject`, `_mk_event_timeout_create`, `_mk_event_timeout_destroy`, `mk_http_thread_resume` | 1 each | declined: Monkey backend internals (the epoll `_mk_event_add` retains `data` in the registration) |
| `rd_atomic32_get` | 1 | **declared** |
| `rd_qsort_r_cmp` | 1 | declined: a function-pointer formal called by name |

Beyond the ranking's alone column, the functions whose every blocker is an
uncontracted **external** callee (two or more) were read too
([ranking-before.txt](artifacts/attribution/ranking-before.txt), last block).
Three of them close with exact contracts: `pack_basic_header_finish`
(`mpack_finish_map`, `mpack_finish_array`), `pack_bool` (`mpack_write_true`,
`mpack_write_false`) and `pack_bytes` (`cfl_sds_len`, `mpack_write_bin`).
The others were declined: `getrlimit`/`setrlimit` (the enumeration typedef
`__rlimit_resource_t`, measured withdrawn by W43-LIBC-CONTRACTS),
`regcomp`/`regfree` (in-place lifecycle), `cfl_arena_create_with_options` (an
existing non-v1 declaration of the same name in `cfl/arena_lifetime.qcore`,
and a heap-kind owned result would let `free` discharge an arena), OpenSSL and
libcurl, `abort` (no no-return mode), the va builtins and anything beside
`cfl_sds_cat_safe` or `pthread_setspecific`.

## 2. The contracts

Each module section quotes the body it rests on, states each parameter's
custody, and lists what is not claimed. In brief:

- **`fcntl`** (`libc/unistd.qcore`). The earlier decline stands for `fcntl` as
  a whole (a creator at `F_DUPFD`). This declaration carries
  `requires resource_literal_argument(cmd, 2);`, the `prctl` precedent, so it
  applies only where the command is literally `2` after preprocessing
  (`F_SETFD`, `bits/fcntl-linux.h:169`). There it sets the descriptor flags and
  creates, closes and transfers nothing: `fd` borrowed (a use; a use after
  `close` is a violation candidate), `cmd` a scalar, the tail
  `resource_variadic_data_tail()`, result none. Flexible, as every libc
  contract.
- **`mk_utils_libc_error`** (`monkey/logging.qcore`). `static inline` in
  `mk_utils.h:92-96`, reached through `mk_libc_error(c)`; it reads `errno`,
  formats into a local buffer and calls `mk_print`. `caller`, `file` borrowed,
  `line` a scalar, result none.
- **`rd_atomic32_get`** (`librdkafka/memory.qcore`, the crate has no atomic
  module). `static RD_INLINE` in `rdatomic.h:100-112`; every configuration arm
  reads the counter. `ra` borrowed, `int32_t` none.
- **`cfl_sds_len`** (`cfl/sds_lifetime.qcore`). `cfl_sds.c:193-200` reads the
  header length (zero for NULL). Borrowed, `size_t` none.
- **Five mpack primitives** (`mpack/writer.qcore`). The three library copies
  are identical; the amalgamation's finishers add a builder pop and its
  `mpack_write_bin` a different assert. No copy frees, stores or transfers an
  argument, so the custody effect agrees though the bodies differ, and each
  copy's body digest is pinned.

**Pinning.** `quirin-audit pin --source-root … --crate C --function F` at
`66910c10` (`build/cycle19/…/fluent-bit-clang-source`, read only) and at
`046accf6` (a fresh private clone, `private/w45-fb/w46-external-callee-contracts/attempt-1`),
then `pin --check` at both: every crate "recorded", 0 not in force, 0 stale,
exit 0 at both. The corpus run reports 82 pinned in force (74 before) and 101
libc contracts under the standing assumption (100 before).

**Where the bodies live.** `mk_utils_libc_error` and `rd_atomic32_get` are
header bodies every including unit carries, but neither is an analysed
function and the BEFORE run published no W4 summary for either, so each
declaration is an unchecked premise pinned to the header text, not a check
(`W46-HEADER-INLINE-DECLARED-UNCHECKED`). `mpack_finish_map`/`_array` are
external-linkage `inline` header definitions, which W4 refuses on linkage
(`W44-CALLEE-EXTERNAL-INLINE-DEFINITION`, whose second discharge arm this is
for its two largest callees).

## 3. Controls

[`crates/qcore-c-gateway/tests/w46_external_callee_contracts.rs`](../../../crates/qcore-c-gateway/tests/w46_external_callee_contracts.rs)
loads the production modules byte for byte (the librdkafka section from its
marker, because the module's libc includes are not resolved by the standalone
loader) and runs six tests:

- **accepting:** one body per callee family, each `Safe`;
- **withdrawal:** the same bodies without the card's sections are not `Safe`
  and carry an uncontracted call;
- **refusing, leak:** an owned actual handed only to any of the borrowed
  positions (writer, data, sds, caller string, counter) is a leak candidate;
- **refusing, released heap actual:** refused, never `Safe`;
- **refusing, premise:** `fcntl` at `F_GETFL`, `F_DUPFD`, `4` and `1 + 1` is
  outside the premise (`Unknown`);
- **refusing, descriptor:** `fcntl(F_SETFD)` without `close` leaks, and after
  `close` is a use-after-release candidate; with `close` it is `Safe`.

No declaration has an owned or nullable result, so there is no nullable-result
control to write. The candidates in these fixtures are fixture findings, not
claims about any upstream body.

## 4. Corpus arbiter (185 units)

| Measure | BEFORE (= baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 398 / 9 / 4,427 | **405 / 9 / 4,420** | +7 / 0 / −7 |
| Units Safe / provisional Unsafe / Unknown / data-only | 5 / 8 / 168 / 4 | 5 / 8 / 168 / 4 | 0 |
| `unsupported` / `unknown_alias` / `leak` / `summary_conflict` | 41,314 / 3,571 / 34 / 1 | 41,239 / 3,572 / 34 / 1 | −75 / +1 / 0 / 0 |
| Uncontracted-call leaves / Unknown functions / blocked only by them | 9,157 / 2,619 / 436 | 9,084 / 2,609 / 429 | −73 / −10 / −7 |
| Functions blocked alone by one external callee | 33 | 30 | −3 |
| Contract revision: pinned in force / libc standing | 74 / 100 | 82 / 101 | +8 / +1 |

**BEFORE** (`run_audit.py`, `--jobs 12`) reproduces the baseline
([before-identity.json](artifacts/arbiter/before-identity.json)): diagnostics,
chunk-contracts and source byte-identical; the contracts sidecar differs only by
the empty `refuted_contracts` section the brief names, and source-effects only
by the digests that bind it, as W45-SR5-SOUNDNESS recorded;
`verdict_bearing_identical: true`.

**AFTER** was run twice (with the tree before and after the commit `e8c4a93b`);
the diagnostics are byte-identical, and the second run is archived under
[artifacts/arbiter/](artifacts/arbiter/) (manifest with raw and gzip digests).
[`scripts/attribute.py`](scripts/attribute.py) attributes every difference
([attribution/arbiter.json](artifacts/attribution/arbiter.json), exit 0,
**0 unattributed**):

- 71 `call to uncontracted function` leaves of the nine callees removed
  (`mpack_finish_map` 27, `mpack_finish_array` 23, `mk_utils_libc_error` 10,
  `cfl_sds_len` 5, `fcntl` 2, one each of the other four);
- 1 `nested call `cfl_sds_len`` leaf removed (`pack_string`);
- 2 condition leaves removed on lines that call `cfl_sds_len` (`unpack_opts_unit`,
  `cmt_encode_opentelemetry_create_batches`), read from the source;
- 2 cascade leaves removed in callers of functions that moved to Safe
  (`pack_metric` → `pack_basic_header_finish`, `mk_sched_broadcast_signal` →
  `mk_sched_send_signal`), through the existing summary rules;
- 2 leaves added at contracted sites: `arguments beyond the declared
  parameters of `fcntl`` (`rd_kafka_open_cb_generic`, the `&on` tail,
  `W46-FCNTL-SETFD-ADDRESS-TAIL`) and `unresolved resource identity `value``
  (`unpack_opts_unit`, where the modelled condition exposes the next refusal).

**Moves (7, all Unknown → Safe, no correction):**

| Function | Moved by |
|---|---|
| `cmt_encode_cloudwatch_emf.c::pack_basic_header_finish` | `mpack_finish_map`, `mpack_finish_array` |
| `ctr_encode_msgpack.c::pack_bool` | `mpack_write_true`, `mpack_write_false` |
| `ctr_encode_msgpack.c::pack_bytes` | `cfl_sds_len`, `mpack_write_bin` |
| `ctr_encode_msgpack.c::pack_string` | `cfl_sds_len` (the nested call) |
| `mk_server/mk_scheduler.c::mk_sched_send_signal` | `mk_utils_libc_error` |
| `mk_server/mk_socket.c::mk_socket_create` | `mk_utils_libc_error` |
| `rdkafka.c::rd_kafka_thread_cnt` | `rd_atomic32_get` |

Each `Safe` is relative to the declared premise it names. `mk_scheduler.c`
stays a provisional Unsafe unit for its pre-existing leak; its diagnostic count
changes only by the removed leaves.

**Leaks and violations.** No leak, `double_release`, `invalid_release` or
`use_after_release` leaf is added or removed; leak stays 34. Nothing is
classified because nothing new appeared, and no provisional Unsafe is added.

## 5. Prediction against outcome

| Predicted | Outcome |
|---|---|
| +5 to +7 Safe, the same fewer Unknown | **+7**, but not the predicted seven: `rd_kafka_open_cb_generic` stays Unknown and `pack_string` (not predicted) moves |
| the five high-confidence moves | all five |
| `rd_kafka_thread_cnt` (medium: `&global` actual) | moved |
| `rd_kafka_open_cb_generic` (medium: premise, `fcntl64` redirect, `&on` tail) | premise and declaration read; **the `&on` tail is refused** |
| units unchanged; no Safe → non-Safe; no new leak or violation | as predicted |
| about 60 `UNC` leaves removed | 71, plus 1 nested, 2 condition and 2 cascade leaves |
| slice off route | **wrong**: at `046accf6` `flb_engine_start` calls `fcntl(…, F_GETPIPE_SZ)` (not in the `66910c10` file), so the slice was run; no verdict moves |

The miss on `pack_string` came from reading only the uncontracted-call leaves:
its blocker was a `nested call` leaf naming `cfl_sds_len`, which the ranking's
"alone" test does not count as an uncontracted call.

## 6. Fluent Bit system slice at `046accf6`

`quirin-audit current --selection contract/profiles/fluent-bit/system-selection.json --jobs 8`
([scripts/run-current.sh](scripts/run-current.sh)) on the fresh private build,
BEFORE with the base provider files and AFTER with this card's: both exit 0,
units **0 / 2 / 13**, functions **28 / 2 / 204** (3 not projected), 82 pinned
contracts in force in AFTER. Two leaves removed and two added, one per
configuration of `src/flb_engine.c:1183`
([before-vs-after.json](artifacts/slice/before-vs-after.json)): the
indirect-access leaf becomes `call to `fcntl` outside its declared
literal-argument premise`. No status moves. Reports:
[before](artifacts/slice/before-85a35431.json.gz), [after](artifacts/slice/after.json.gz).

## 7. Gate

[`scripts/gate.sh`](scripts/gate.sh) at `a787a9b5`, every step exit 0
([gate.json](gate.json), [steps](gate/steps.txt)): 322 units / 334 summaries,
**6,021 passed, 0 failed**, 11 ignored; fmt and clippy clean; both
reevaluation examples 0; 210 Lean audits; Python 364; docs index; provider
tests 129, fmt, clippy, `contracts.py check`, all five packages built and
verified, `providers/tests` 15. Logs are private under
`/tmp/claude-1000/w46ext/gate2/`.

A first run at `72470631` failed 3 tests and fmt, and both causes were this
card's:

- `contract_inherit_qcore_provider` compares `librdkafka/memory.qcore` with a
  hand-written fixture, which now carries `rd_atomic32_get` too;
- the native descriptor binding (`descriptor_contract_binding.rs`) bound every
  recorded `fcntl` execution to the new declaration, although it is declared
  only at `F_SETFD`. The binding now rederives the literal premises from the
  environment's trusted texts and profiles an execution outside them as an
  undeclared call, never binds it (C04.5); a new unit test pins `F_SETFD`
  bound, `F_DUPFD` and an unrecorded command profiled. This is off the
  arbiter's route (only the `bind_descriptor_contracts` example uses it).

## 8. Follow-ups

Registered ([followups.json](followups.json), applied by
[scripts/update_ledger.py](scripts/update_ledger.py)):
`W46-SDS-CAT-SAFE-IN-PLACE-REALLOC` (170 sites, 3 alone, 5 co-blocked after),
`W46-FCNTL-SETFD-ADDRESS-TAIL` (1 leaf, 1 function),
`W46-HEADER-INLINE-DECLARED-UNCHECKED` (2 declarations, 11 sites, 3 moved
functions rest on them) and `W46-EXTERNAL-RESIDUAL-RANKING` (the 30 remaining
alone-blocked functions by class). Amended with this card's part:
`W44-CALLEE-EXTERNAL-INLINE-DEFINITION`, `NESTED-CALL-IN-ARGUMENT-POSITION`,
`W43-DESCRIPTOR-REFUSED-SHAPES`, `CTRACES-FORMATTER-CHAIN`. None retired.

## 9. Reproduction

```sh
python3 docs/reports/w46-external-callee-contracts/scripts/rank_external.py ARTIFACTS --owners OWNERS --json OUT
python3 docs/reports/w46-external-callee-contracts/scripts/classify_owners.py RANK_JSON SOURCE_ROOT OUT
python3 docs/reports/w43-return-expression/scripts/run_audit.py OUT_DIR COMMIT --jobs 12
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py RUN BASELINE --root-alias WORKTREE=/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/hungry-thompson-1740da
python3 docs/reports/w46-external-callee-contracts/scripts/attribute.py BEFORE AFTER --source SOURCE_ROOT --json OUT
python3 docs/reports/w46-external-callee-contracts/scripts/tallies.py BEFORE AFTER
quirin-audit pin --source-root CHECKOUT --crate CRATE --function NAME…   # then pin --source-root CHECKOUT --check
```

The contracts sidecar embeds this worktree's path in its source keys, as every
isolated card's does.
