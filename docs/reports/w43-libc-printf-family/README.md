# W43-LIBC-PRINTF-FAMILY: the printf family, declared and attributed leaf by leaf

Card W43-LIBC-PRINTF-FAMILY, 2026-09-25, on base `f8b41209` (main after the
CONTRACT-INHERIT-ADOPTION integration). It discharges `LIBC-PRINTF-FAMILY`:
`printf`, `fprintf`, `snprintf` and `sprintf` are declared in
`providers/quirin-provider-libc/contracts/libc/stdio.qcore` and measured in a
four-stage arbiter, one member per stage, so each member's movement reads on
its own. The `va_list` members are refused by name on a measured reader ground.

This is **pure contract data**. No loader, checker, gateway or backend code
changes, so there is no semantic change and no Lean obligation. The tail-actual
widening the brief allowed was measured and **not** taken
([§5](#5-the-exchange-and-why-the-tail-rule-was-not-widened)). Every
declaration is a trusted assumption about libc (trust boundary 28).

## Headline

* **Declared: 4.** `printf`, `fprintf`, `snprintf`, `sprintf`: every declared
  parameter `resource_borrowed`, a `resource_variadic_data_tail()`, and
  `resource_return_none()` for the `int` result. The census reads each as
  agreeing in all **168** units that declare it, withdrawn in none. The arbiter
  confirms it: no `whose declared contract was withdrawn` leaf names any of the
  four.
* **Refused by name: 4.** `vsnprintf`, `vprintf`, `vfprintf`, `vsprintf` are
  withdrawn in all 168 units (`abi-unreadable-declaration`): the reader cannot
  read `__gnuc_va_list`. `dprintf` agrees everywhere but is never called, so it
  is not declared.
* **Arbiter.** BEFORE reproduces the W43-LIBC-RESULT-ALIAS diagnostics **byte
  for byte**. AFTER removes **2,229** leaves and adds **1,283**, and all 3,512
  are attributed. **5 functions move Unknown → Safe**; nothing else moves.
  No unit status moves. The leak pin holds at **272**. There are 0
  `double_release`, `invalid_release` or `use_after_release` candidates.
* **The exchange is the finding.** 2,212 uncontracted-callee leaves go, but
  1,179 come back as declared-tail refusals and 100 as nested-call refusals.
  1,081 of the tail refusals sit on librdkafka's `RD_UT_*` unit-test macro
  lines. No function is blocked only by them, so widening the tail rule would
  move no function. They are registered, not widened.
* **Prediction.** The frozen [PREDICTION.md](PREDICTION.md) got the function
  movement exactly right: the five sole-blocked functions move, and neither
  ledger-forecast exchange fires. It got the leaf exchange wrong by an order of
  magnitude: 1,283 added against a forecast of 90 (range 40–150). See
  [§7](#7-prediction-against-outcome).
* **Follow-ups.** `LIBC-PRINTF-FAMILY` is retired. `LIBC-PRINTF-VA-LIST`,
  `LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES` are
  registered.

| Measure | BEFORE (W43-LIBC-RESULT-ALIAS) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 257 / 84 / 4,493 | **262 / 84 / 4,488** | +5 / 0 / −5 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 48,930 | **47,984** | −946 |
| `unknown_alias` | 4,657 | 4,657 | 0 |
| `leak` / `summary_conflict` / `double_release` | 272 / 1 / 0 | 272 / 1 / 0 | 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole | 11,652 / 2,688 / 384 | 9,636 / 2,617 / 378 | −2,016 / −71 / −6 |
| Unknown functions blocked only by one uncontracted callee | 295 (230 callees) | 296 (228 callees) | +1 |
| declared-tail refusals (leaves / functions) | — | 1,018 / 247 on Unknown functions | new at this scale |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 108 / 45 / 130 | 108 / 45 / **134** | 0 / 0 / +4 |

The census rows count Unknown functions, as
[blocker-census.txt](blocker-census.txt) does. The uncontracted-call row's
leaves fall by 2,016, not 2,212: the census counts Unknown functions only.
197 family leaves were in provisional-`Unsafe` functions (`fprintf` 177,
`snprintf` 20), and one newly-safe-callee leaf also went (2,212 − 197 + 1).

## 1. The declarations

Appended at the end of `libc/stdio.qcore`. The only other edit to the module
is a bracketed pointer in its header's "not declared" paragraph. The prototypes
come from the analysed units' own `/usr/include/stdio.h` (glibc, x86-64, LP64):

| Member | Prototype (after preprocessing) | Declaration |
|---|---|---|
| `printf` | `int printf (const char *__restrict __format, ...)` | `format` borrowed; tail; none |
| `fprintf` | `int fprintf (FILE *__restrict __stream, const char *__restrict __format, ...)` | `stream`, `format` borrowed; tail; none |
| `snprintf` | `int snprintf (char *__restrict __s, size_t __maxlen, const char *__restrict __format, ...)` | `s`, `maxlen`, `format` borrowed; tail; none |
| `sprintf` | `int sprintf (char *__restrict __s, const char *__restrict __format, ...)` | `s`, `format` borrowed; tail; none |

**The assumptions, each trusted rather than verified:**

* The result is an `int`, a scalar, so `resource_return_none()` is the
  faithful mode, not an approximation.
* The stream (`FILE *`) is the caller's, and the call neither opens, closes
  nor retains it. That is `fflush`'s reading.
* The destination buffer is written through and stays the caller's. That is
  `msgpack_packer_init`'s `pk` reading.
* The tail is rendered into text. No tail argument is freed and no pointer to
  one is retained. That is `mk_print`'s reading.

**What the declarations do not say** is listed in the module's comment. It
covers buffer bounds, format/argument agreement, `%n`, `%s` through a released
pointer, stream errors, `errno`, locale and threads.

**Declared-ABI gate** (constraint 8). The census tool
(`declared_abi_census`, the production decision) ran over the 185 units
captured from this card's own run
([census/family9.txt](census/family9.txt), from the measurement-only module
[measurement/family9.qcore](measurement/family9.qcore)). It was then rerun on
the whole shipped manifest:

| Member | Units agreeing | Withdrawn | Leaves / functions at the baseline |
|---|---:|---:|---|
| `printf` | 168 | 0 | 22 / 16 |
| `fprintf` | 168 | 0 | 1,138 / 239 |
| `snprintf` | 168 | 0 | 1,044 / 234 |
| `sprintf` | 168 | 0 | 8 / 4 |
| `dprintf` | 168 | 0 | not called; not declared |
| `vsnprintf`, `vprintf`, `vfprintf`, `vsprintf` | 0 | 168 each | 25 / 25, 1 / 1, 0, 0; refused |

On the whole manifest, the other 130 declarations' census totals are
unchanged: 352 undeclared, 113 return-type withdrawals (the GNU `strerror_r`)
and 64 unreadable. **Nothing is withdrawn fail-closed for the four**, because
no unit disagrees.

**Refused by name.** The four `va_list` members end in
`__gnuc_va_list __arg`, a typedef of the builtin `__builtin_va_list`. The
reader cannot resolve that typedef, and v1 has no spelling for it. On x86-64
it decays to an object pointer; on AArch64 it is a struct passed by value.
All 26 functions calling `vsnprintf` or `vprintf` carry other reasons, 24 of
them a `__builtin_va_start`/`__builtin_va_end` leaf, so none is sole-blocked and
a reader repair alone moves no function. Registered as `LIBC-PRINTF-VA-LIST`.

## 2. Placement and pins

* **Placement.** The declarations are in the libc crate only. The four names
  are in `providers/quirin-provider-libc/libc-names.txt`, no other contract
  module declares them, and the placement test passes unchanged.
* **Unchanged, checked:**
  * `contract/corpus.json`, `cycle19-resource-contracts.json` (`libc/stdio.qcore`
    is already indexed and selected), the libc `CONTRACTS` index and
    `scripts/contract_corpus.py`;
  * `providers/contract-load-path.json`: `python3 providers/contracts.py sync`
    exits 0 and changes nothing, because `libc/stdio.qcore` is not a published
    (digest-pinned) module.
* **Moved, deliberately:**
  * `compiler/tests/qcore_contract_corpus.rs`: four clause pins after `remove`.
    Each has 3 clauses plus 1 per declared parameter: `printf` 4, `fprintf` 5,
    `snprintf` 6, `sprintf` 5.
  * `compiler/tests/contract_loader.rs`: (selected, v1 declarations, rejected)
    goes (45, 130, 207) → **(45, 134, 207)**. The old-table all-borrowed loop
    gains the four, with their declared arities, as `mk_print` is listed.

No assertion is weakened.

## 3. Arbiter

One release binary serves every run, because the card is declarative and the
audit reads the contract modules at run time:
`cycle19_fluent_bit_libraries_audit`, sha256 `f57394c0…a1ae`, built from base
`f8b41209`. Every run uses
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py).
The stages were written by [`scripts/apply_stage.py`](scripts/apply_stage.py)
and driven by [`scripts/run_stages.py`](scripts/run_stages.py). The driver
restores the committed module by rewriting its bytes, and asserts the round
trip.

| Run | Declared | Exit | Window (UTC, 2026-09-24) | Record |
|---|---|---:|---|---|
| BEFORE | base (`f8b41209`) | 0 | 16:01:30–16:10:49 | [before-run.json](before-run.json) |
| stage 1 | + `printf` | 0 | 16:18:15–16:26:06 | [stage-printf-run.json](stage-printf-run.json) |
| stage 2 | + `fprintf` | 0 | 16:26:06–16:33:17 | [stage-fprintf-run.json](stage-fprintf-run.json) |
| stage 3 | + `snprintf` | 0 | 16:33:17–16:42:25 | [stage-snprintf-run.json](stage-snprintf-run.json) |
| AFTER | + `sprintf` = the committed module (`84c5d217`) | 0 | 16:42:37–16:50:04 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

A private capture run with the transparent Clang wrapper
(`../w43-libc-result-alias/scripts/clang-tee.sh`, driven by
[`scripts/capture_run.py`](scripts/capture_run.py)) supplied the 185 units for
the census and the `-E -P` texts for attribution: 180 paired, and the same 5
unpaired units as before, which project no function. Its diagnostics are byte
identical to BEFORE.

**BEFORE vs the W43-LIBC-RESULT-ALIAS baseline**
([before-identity.json](before-identity.json)):

* diagnostics, chunk-contracts and source are **byte-identical**;
* `contracts` differs from that baseline by the checkout-path keys and by
  CONTRACT-INHERIT-ADOPTION's inclusion record, `crate:module` keys and
  `resource_string.qcore`, which the brief lists as confirmed verdict-neutral;
* `source-effects` differs by the checkout-path digests the brief lists.

**AFTER vs BEFORE:** chunk-contracts, source-effects and source are
byte-identical. `contracts` differs in exactly one leaf: the source text of
`libc/stdio.qcore` ([contracts-diff.json](contracts-diff.json)).

## 4. Attribution, member by member

Each stage is attributed with W43-LIBC-CONTRACTS'
[`attribute.py`](../w43-libc-contracts/scripts/attribute.py), declaring only
that stage's member. [`scripts/reattribute.py`](scripts/reattribute.py) then
adds one class, `same-body-closure`. It assigns a changed leaf in a function
whose own body **calls** the stage's member in a position that never reported
an uncontracted-callee leaf. The mechanism is C02-P's
`seed_call_arguments`: it refuses, as an access base, every name passed to an
uncontracted call. That class fired **once** (stage 3, below). Every stage
exits 0 with `attribution_complete: true`. So does the whole BEFORE → AFTER
delta ([attribution-summary.json](attribution-summary.json)). Rows:
[attribution-printf.json.gz](attribution-printf.json.gz),
[…-fprintf](attribution-fprintf.json.gz),
[…-snprintf](attribution-snprintf.json.gz),
[…-sprintf](attribution-sprintf.json.gz),
[…-full](attribution-full.json.gz). The per-member digest is
[stage-summary.json](stage-summary.json).

| Stage | Member leaves removed | … with no leaf left on their line | … on a line that gained a leaf | Other removed | Added | Moves |
|---|---:|---:|---:|---:|---:|---|
| `printf` | 22 | 13 | 9 | 0 | 9 | none |
| `fprintf` | 1,138 | 41 | 1,097 | 4 | 1,159 | 3 Unknown → Safe |
| `snprintf` | 1,044 | 934 | 110 | 14 | 113 | 1 Unknown → Safe |
| `sprintf` | 8 | 4 | 4 | 1 | 4 | 1 Unknown → Safe |
| **sum of stages** | **2,212** | 992 | 1,220 | 19 | 1,285 | **5** |
| BEFORE → AFTER | 2,212 | | | 17 | 1,283 | 5 |

The stage sums exceed the end-to-end figure by two leaves. Each was added in
one stage and removed in a later one:

* `rd_hexdump`'s `fprintf` tail refusal (`rdlog.c`), added at stage 2 and
  removed at stage 3 by `same-body-closure`. Its tail names `hexen` and
  `charen` were refused as bases only because they were also passed to
  `snprintf`, in `hof += snprintf(…)`, which reports an assignment refusal
  rather than a callee leaf.
* one `snprintf` tail refusal in `create_jws_compact_serialization`
  (`rdkafka_sasl_oauthbearer.c`), added at stage 3 and removed at stage 4
  (`same-function`), where the same body's `sprintf` is declared.

**What replaced each member's leaves**, on the same analysed line:

* **`printf`**:
  * 8 declared-tail refusals: `cio_file_scan_dump` (2), `cio_memfs_scan_dump`,
    `rd_msghdr_print`, `mk_print` (2), `cb_debug_chunk_complete`,
    `mk_server_info`;
  * 1 nested call: `getpid` in `mk_server_info`.
* **`fprintf`**:
  * 1,094 declared-tail refusals, 1,081 of them on `RDUT:` macro lines;
  * 65 nested calls, led by `rd_kafka_err2str` and `rd_kafka_msgq_len`.
  * Removed besides: 2 same-line and 2 same-function
    `indirect memory access` refusals.
* **`snprintf`**:
  * 78 declared-tail refusals;
  * 32 nested calls;
  * 1 `resource call with a formal embedded in an unmodeled expression`;
  * 1 `call has an unmodeled formal address or borrowed expression`;
  * 1 same-function tail refusal (`rd_kafka_broker_set_error`).
  * Removed besides:
    * 12 same-line `indirect memory access` refusals on `snprintf` lines, such
      as `snprintf(hostname, …, context->host)`, where a member-access tail now
      leaves the lexical gate as an admitted T5 chain;
    * 1 newly-safe-callee leaf, `rd_kafka_mk_nodename` in
      `rd_kafka_broker_find`;
    * the `rd_hexdump` closure leaf.
* **`sprintf`**:
  * 2 `resource call with a formal embedded in an unmodeled expression`;
  * 2 nested calls: `rd_list_elem` in `create_jws_compact_serialization`
    and `getpid` in `mk_utils_register_pid`.

Across the delta, **62 units** change. Each moves only its
`diagnostic_counts` and its count-summary `reason`, both by exactly the leaf
delta of its own functions. No unit status or other unit field moves, and no
top-level field other than the counts moves.

**The five functions that move Unknown → Safe.** Each is Safe **relative to
the declared premises**, which is invariant 2's reading and not a proof of the
C:

| Function | Unit | Its only BEFORE blocker | Stage |
|---|---|---|---|
| `cmt_errno_print` | `cmt_log.c` | `fprintf(stderr, "[%s:%i errno=%i] %s\n", file, line, errnum, buf)` | 2 |
| `ctr_errno_print` | `ctr_log.c` | the same body | 2 |
| `cio_errno_print` | `cio_log.c` | the same body | 2 |
| `rd_kafka_mk_nodename` | `rdkafka_broker.c` | `snprintf(dest, dsize, "%s:%hu", name, port)` | 3 |
| `cJSON_Version` | `cJSON.c` | `sprintf(version, "%i.%i.%i", 1, 7, 18)` | 4 |

**Both forecast exchange risks were measured, and neither fired:**

* **The `const char *` formal in a tail** (`file`, `name`: the
  `MK-PRINT-TAIL-ACTUALS` shape). Neither formal is protected: neither is
  dereferenced with `*` nor passed at a custody position. After the
  declaration, each is passed only to a declared tail position, so
  `seed_call_arguments` no longer refuses it, and T4 admits it as never bound.
  The 26 `mk_print` refusals were not re-examined here. The two premises §5
  separates are the candidate grounds for them.
* **`cJSON_Version`'s `static` local array.** `return version;` carried no leaf
  at the baseline, and the declaration does not touch the return route. The
  tail is three literals.

Newly exposed sole blockers ([sole-moves.json](sole-moves.json)): six functions
enter the "one uncontracted callee" set. Three ctraces formatters are now
blocked only by `sds_cat_safe`, `format_metric_type` only by
`cfl_sds_cat_safe`, and two ACL constructors only by `rd_kafka_AclBinding_new0`.
The five movers leave it. `CTRACES-FORMATTER-CHAIN`'s `snprintf` half is
therefore measured done, and its chain now waits only on `cfl_sds_cat_safe`
(for the integrator; that entry is not edited here).

**Leak pin (constraint 7).** The pin is unmoved at **272**, and no leak leaf is
added or withdrawn. This is the fifth consecutive declarative arbiter to leave
it alone. Every admitted tail actual is, by the rule's construction, a name the
body can never bind to a tracked identity, so no binding that the old refusal
poisoned becomes a live obligation. The new baseline is
[artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json)).
`docs/reports/w43-libc-result-alias/artifacts/` is its frozen predecessor and
is not edited.

**Violation candidates: none.** `double_release`, `invalid_release` and
`use_after_release` are all 0 before and after. No provisional `Unsafe` is
added, and none is presented as an upstream defect.

## 5. The exchange, and why the tail rule was not widened

[`scripts/exchange.py`](scripts/exchange.py) ([exchange.json](exchange.json))
reads every added refusal against the baseline call-site shapes
([call-shapes.json.gz](call-shapes.json.gz)).

**1,179 declared-tail refusals** in 253 functions (`fprintf` 1,093,
`snprintf` 78, `printf` 8). `resource_variadic_data_tail()` admits nothing on
its own. `variadic_nested::tail_arguments_are_data_with_nested` refuses a
non-empty tail on either of two premises:

* **(i)** the function's typed declaration facts are unreadable
  (`FunctionTypes::is_readable`, a total-or-reject local reader), which refuses
  even a literal;
* **(ii)** a tail name that the C02-P closure may bind.

What decides between them:

* **1,081** of the 1,179 are on librdkafka `RDUT:` macro lines:
  `fprintf(stderr, "RDUT: …", __FILE__, __LINE__, __FUNCTION__)`, followed by
  the user's own `fprintf(stderr, fmt, …)`.
* **10** have literal-only tails, so only premise (i) can refuse them.
* **A gateway probe** ([probe/](probe/)) replays `unittest_rd_crc32c` from its
  captured unit text under the shipped manifest. It keeps all five tail
  refusals when `__FUNCTION__` is rewritten to a string literal
  ([probe/crc32c-function-name-as-literal.txt](probe/crc32c-function-name-as-literal.txt)).
  That function is therefore refused on readability, not on a name. An
  earlier synthetic version of the probe, whose text was not kept, admitted
  the same `fprintf` shape with `__FUNCTION__` in a small readable body. It is
  measured once, and the harness is not shipped.
* The remaining leaves are **not decided** between (i) and (ii). A wider probe
  over the ten largest functions was started and stopped by hand: the debug
  build took more than ten minutes on the first 1.1 MB unit. It is not
  evidence.

**100 nested-call refusals** in 69 functions, where a tail passes another
call's result:

* `rd_kafka_err2str` 42, `rd_kafka_msgq_len` 13, `rd_list_cnt` 9,
  `rd_kafka_error_string` 7, and 17 more inner callees with 1–3 each.
* O04-TRACE's nested admission needs the inner callee's contract in force (N6,
  N7); these are uncontracted.

**The bound.** Over AFTER, **no** non-Safe function's remaining leaves are all
family tail refusals or nested-call refusals added by this card. No tail-actual
admission and no nested-call admission could move a function by itself. The
brief allows the widening only soundly, with Lean parity and its own
attribution class. The measured yield is leaves only, and a sound candidate
exists: literal and C11 6.4.2.2 predefined-identifier tail actuals are data
whatever the declarations say. It is registered as `LIBC-PRINTF-TAIL-ACTUALS`
rather than done here. The nested half is `LIBC-PRINTF-NESTED-TAIL-CALLEES`.

## 6. Validation

Measured in this worktree (`agent-ad6e8e3163534f46e`) by
[`../w43-libc-result-alias/scripts/run_gate.py`](../w43-libc-result-alias/scripts/run_gate.py),
unchanged. It sets `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`, turns debug
info off, and captures every exit to a file, never through a pipe. The core
and provider gates ran on `8a1a6117`, the commit carrying the declarations
(`84c5d217`) and the re-pins. Later commits change documents, the ledger,
report scripts and evidence only. Logs are worktree-private under
`measure/gate/`. The generator input is [gate.json](gate.json), with the
[Clang recording](clang-version.txt) taken in this checkout beside it.

GATE-TABLE

## 7. Prediction against outcome

[PREDICTION.md](PREDICTION.md) was committed (`dcb642f5`) before any
declaration was drafted into the manifest.

| Item | Predicted | Measured | |
|---|---|---|---|
| Members declared | 4, `va_list` members refused, `dprintf` not declared | same | ✔ |
| Declared-callee leaves removed | exactly 2,212 | 2,212 | ✔ |
| Withdrawn-contract leaves | 0 | 0 | ✔ |
| Other removals (same-line, same-function) | 0–200 | 17 (+1 newly-safe callee) | ✔ |
| **Leaves added** | **90 (40–150)** | **1,283** | ✘ |
| … nested calls | ~35 | 100 | ✘ |
| … tail refusals | ~30 | 1,179 | ✘ |
| Functions Unknown → Safe | the five named | the five named | ✔ |
| The two ledger-forecast exchanges | neither fires | neither fires | ✔ |
| Further moves | 0–5 | 0 | ✔ |
| Unit moves, leak, violation candidates, `summary_conflict` | 0, 272, 0, 1 | 0, 272, 0, 1 | ✔ |
| `unknown_alias` | within ±20 | ±0 | ✔ |
| `unsupported` | about −2,100 | −946 | ✘ |

**Where the forecast failed.** It assumed that a bare-name tail (1,204 sites,
1,106 of them `fprintf`) is mostly admitted because the name is never bound. It
did not account for premise (i). Tail admission first requires the whole
function's typed declaration facts to be readable, and librdkafka's `RD_UT_*`
test bodies mostly are not. The lexical forecast could not see that premise.
The function-level forecast held because the five movers are small bodies that
the reader reads in full.

## 8. Ledger and status

* **Retired**: `LIBC-PRINTF-FAMILY`. It is preserved verbatim with its ground
  in [retired-followups.json](retired-followups.json).
* **Registered** ([followups.json](followups.json)): `LIBC-PRINTF-VA-LIST`,
  `LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES`.
* Follow-ups go from **299 to 301**. `corpus.cycle19_raw_units` points at this
  report's artifacts, and its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* `docs/STATUS.md`'s corpus, contract-corpus and follow-up rows are updated
  from this arbiter, and `docs/reports/INDEX.md` is regenerated.

## 9. For the integrator

* **Constraint 7:** W43-LIBC-PRINTF-FAMILY is the fifth consecutive
  declarative arbiter at 272. It made 4 declarations and 5 function moves.
* **Constraint 8:** the census adds a measured reader boundary,
  `__builtin_va_list` (`__gnuc_va_list`), which is unreadable in every unit.
* **`CTRACES-FORMATTER-CHAIN`:** its `snprintf` half is measured done, and it
  now waits only on `cfl_sds_cat_safe` (§4).
* **Files likely to conflict:**
  * with W43-LIBC-RESTRICT-PROTOTYPES: `compiler/tests/qcore_contract_corpus.rs`
    (clause pins in module order; this card's four follow `remove`) and
    `compiler/tests/contract_loader.rs` (the v1 count, 130 → 134 here, and the
    all-borrowed loop). The two cards' counts add up. If that card also appends
    to `libc/stdio.qcore`, keep both append blocks;
  * with every concurrent card: `docs/reviews/r0_3/acceptance_matrix.json`
    (follow-ups and `corpus.cycle19_raw_units`), `docs/STATUS.md`,
    `docs/reports/INDEX.md` and the increment index;
  * the verdict-moving cards (W43-INDIRECT-ACCESS-GENERAL,
    W43-RESULT-ALIAS-RETURNED) move the same baseline. This card's attribution
    is against `w43-libc-result-alias/artifacts`, and a composed tree needs a
    re-measured integration arbiter.
* **Increment record:** `wave-41-continuation.md` is 36,333 bytes, over the
  cap. This card opens `wave-43.md` at a record boundary and points the index
  at it. A concurrent card may open the same file.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/before f8b41209
python3 docs/reports/w43-libc-printf-family/scripts/identity.py measure/before docs/reports/w43-libc-result-alias/artifacts measure/before-identity.json
python3 docs/reports/w43-libc-printf-family/scripts/capture_run.py measure/capture-run
python3 docs/reports/w43-libc-result-alias/scripts/strip.py
cargo build --release -p qcore-c-gateway --example declared_abi_census
target/release/examples/declared_abi_census docs/reports/w43-libc-printf-family/measurement/family9.qcore -- measure/capture/plain/*.i
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py measure/before/resource-diagnostics.json measure/capture/ppP measure/unit-texts
python3 docs/reports/w43-libc-printf-family/scripts/call_shapes.py measure/before/resource-diagnostics.json measure/unit-texts <out> printf fprintf snprintf sprintf
python3 docs/reports/w43-libc-printf-family/scripts/line_bound.py measure/before/resource-diagnostics.json measure/unit-texts <out> printf fprintf snprintf sprintf
# (PREDICTION.md frozen and committed here)
python3 docs/reports/w43-libc-printf-family/scripts/run_stages.py <copy of the committed stdio.qcore> measure/stages 84c5d217
python3 docs/reports/w43-return-expression/scripts/run_audit.py measure/after 84c5d217
python3 docs/reports/w43-libc-contracts/scripts/attribute.py <previous stage> <stage> measure/unit-texts <attr.json> <file naming the stage's member>
python3 docs/reports/w43-libc-printf-family/scripts/reattribute.py <attr.json> measure/unit-texts <closed.json> <member>
python3 docs/reports/w43-libc-printf-family/scripts/stage_summary.py <out> printf=… fprintf=… snprintf=… sprintf=…
python3 docs/reports/w43-libc-printf-family/scripts/exchange.py <full closed.json> docs/reports/w43-libc-printf-family/call-shapes.json.gz measure/after <out>
python3 docs/reports/w43-libc-printf-family/scripts/contracts_diff.py measure/before measure/after <out>
python3 docs/reports/w43-libc-contracts/scripts/archive.py measure/after docs/reports/w43-libc-printf-family/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-libc-printf-family/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w43-libc-printf-family/artifacts 30
python3 docs/reports/w43-libc-printf-family/scripts/newly_sole.py docs/reports/w43-libc-result-alias/artifacts docs/reports/w43-libc-printf-family/artifacts <out>
python3 docs/reports/w43-libc-printf-family/scripts/publish_evidence.py
python3 docs/reports/w43-libc-printf-family/scripts/update_ledger.py
```

The probe in [probe/](probe/) is a temporary gateway integration test, run
with `PROBE_UNIT` and `PROBE_ENTRY` set, and removed from the test tree before
the gate. The private raw outputs are in this worktree's `measure/`.
