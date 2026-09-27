# W44-LIBC-RESIDUALS: frozen prediction

Frozen **before** any reader, tail-rule or contract edit, and before any staged
arbiter run. It is committed on its own so the outcome can be compared with it
in the report. Nothing below is edited after the first staged run. It rests on:

* the BEFORE run (`measure/before/`, base `c1e69bb5`), whose diagnostics are
  **byte-identical** to the combined baseline
  `docs/reports/w43-indexed-resource-collections/integration/artifacts/`
  (`chunk-contracts` and `source` identical too; `contracts` identical once the
  checkout root is aliased; `source-effects` differs only in the
  `resource_contracts:<digest>` key and the digests that bind it);
* a private capture run (tee wrapper, base binary) for 185 units: 180 paired
  `-E -P` texts, the same 5 unpaired units as every wave-43 card;
* the census scripts in [scripts/](scripts/):
  [leaf_census.py](scripts/leaf_census.py) →
  [census-leaves-before.json.gz](census-leaves-before.json.gz),
  [header_inline_census.py](scripts/header_inline_census.py) →
  [census-header-inline.json](census-header-inline.json),
  [tail_nested_lines.py](scripts/tail_nested_lines.py) →
  [census-tail-nested-before.json.gz](census-tail-nested-before.json.gz),
  [callee_lines.py](scripts/callee_lines.py);
* the base declared-ABI census over the 185 captured units of the manifest
  (13,101 rows) and of the measurement module
  [measurement/residuals8.qv](measurement/residuals8.qv)
  ([measurement/residuals8-base.txt](measurement/residuals8-base.txt)).

## Baseline figures the prediction starts from

| Family | Leaves (all / Unknown fns) | Functions | Bound: non-Safe functions whose every leaf is in the family |
|---|---:|---:|---:|
| header-inline uncontracted (`__bswap_16/32/64`, `__uint64_identity`) | 599 / 586 | 99 | **3** (`ut_put_be16/32/64`) |
| `vsnprintf` + `vprintf` uncontracted (the `va_list` members) | 26 / 26 | 26 | 0 |
| tail refusals (`fprintf` 1,093, `snprintf` 78, `mk_print` 21, `printf` 8) | 1,200 / 1,014 | 263 (246 Unknown) | 1 (`cb_debug_chunk_complete`, whose tail is the tracked allocation `buf`) |
| … of which a literal-or-predefined-identifier tail | ≤ 1,091 (1,081 on `RDUT:` lines) | 212 touched | **0** |
| nested refusals with a printf-family outer call | 89 | 62 | 0 |
| every nested refusal | 143 | 97 | — |

The brief's 279 + 247 are the Unknown-function `__bswap_32`/`__bswap_16`
leaves (286 / 252 over all functions), and its 1,014 / 246 is the Unknown share
of the 1,200 / 263 tail family. The printf-nested family is **89** here, not
the 100 W43-LIBC-PRINTF-FAMILY measured: later wave-43 cards moved 11.

## P0. What is built and what is refused

| Item | Forecast |
|---|---|
| 1 header-inline | **declared** as the neutral element in a new `libc/byteswap.qcore`, all six helpers the census finds called from main source (`__bswap_16/32/64`, `__uint16/32/64_identity`), stated as header-inline bodies. Evidence: every one of the 174 units that defines them carries the **same** (signature, body), and every body is pure integer arithmetic over its one scalar parameter (no call, no pointer, no object other than the parameter). Not projected: header definitions are outside the enumerated denominator (E01-B2). |
| 1a reader | `__bswap_64` is **withdrawn in all 174 units** at the base (`__extension__ static __inline __uint64_t __bswap_64 (…)`: `__extension__` is not a discarded specifier). The reader is taught to discard `__extension__`, a GNU keyword that carries no type; Lean parity. The other five agree in 174/174. |
| 2 `va_list` | the reader resolves `__builtin_va_list` (reached through `__gnuc_va_list`/`va_list`) at a **parameter** to a new loader spelling `va_list`, parameter-only, not a `ptr` spelling, so only `resource_borrowed` can be written on it; a pointer to it, a result of it, and a unit that `typedef`s the builtin name itself still refuse. Lean parity. `vsnprintf` and `vprintf` are declared (all borrowed, `resource_return_none()`); `vfprintf`/`vsprintf` are faithful but never called and are not declared (the `dprintf` precedent). |
| 3 tail | the narrow widening **is built**: when a function's typed declarations are unreadable, a tail whose every actual is only literals and C11 6.4.2.2 predefined identifiers (`__func__`, `__FUNCTION__`, `__PRETTY_FUNCTION__`, keywords in Clang) is admitted; every other unreadable tail keeps the refusal, and readable functions are unchanged. Lean parity and its own attribution class. **Measured function yield before building: 0.** |
| 4 nested | only **`strerror`** is declared (`libc/string.qcore`, static or thread-local message, `resource_return_none()`, the `gai_strerror` precedent): its 2 printf-nested sites are whole-argument calls. `getpid` agrees in 137 units but both of its sites are `(long) getpid()`, a cast that premise N4 refuses, so a declaration would decide nothing and it is **not** declared. The remaining 85 leaves are corpus-defined inner callees (W44-CORPUS-CALLEE-SUMMARIES' bodies: `rd_kafka_err2str` 42, …), inner calls inside larger expressions (`!(rd_kafka_msgq_len(…) == …)`, `(double)(rd_clock() …)`, `path[strlen(path) - 1]`), or Cyrus SASL. No nested premise is widened. |

## P1. Stage 0, the reader and vocabulary change alone (base contracts)

* All five artifacts **byte-identical** to BEFORE except `contracts`'
  checkout-path keys (none expected): the base manifest declares no name whose
  C declaration carries `__extension__` or a `va_list` parameter.
* Manifest census: **0 of 13,101** rows move.

## P2. Stage 1, the literal-tail widening (base contracts)

* Tail leaves removed: **point 1,050, range 850–1,091**, nearly all on
  `RDUT:` lines.
* Leaves added: **point 40, range 0–200**: statements that now pass the tail
  rule and refuse at a later gate, and `unknown_alias`/leak leaves where a
  removed refusal no longer poisons a binding.
* Functions: **0 moves** (range 0–2). Units: 0.
* Leak: **271**, range 271–274; any added leak leaf is attributed as exposed by
  an un-poisoned binding, never as an upstream defect.

## P3. Stage 2, `libc/byteswap.qcore`

* ABI census: all six agree in all 174 defining units (with P0's reader
  change), withdrawn in none.
* Removed: the **599** header-inline callee leaves and the one
  `nested call `__uint16_identity`` leaf (600).
* Added: **point 520, range 400–600**. 331 sites are `*_vp = __bswap_32 (_v);`
  and 81 `*_vp = (int16_t)__bswap_16 (_v);` (librdkafka's buffer-read macros:
  a store through a pointer), 159 + 7 member stores
  (`(&Group)->len = (int16_t)__bswap_16 (_v);`, `md->…`), so most exchange for
  an indirect-access or unmodelled-store refusal. Scalar-destination sites
  (`_klen = (int32_t)…`, `i64val = __bswap_64 (0);`, `int16_t be = …`) are
  forecast admitted.
* Functions: **`ut_put_be16`, `ut_put_be32`, `ut_put_be64` Unknown → Safe**
  (their only leaf; `memcpy` is declared), range 2–6. Units 0.
* Leak 271 (range 271–273).

## P4. Stage 3, `vsnprintf` and `vprintf`

* ABI census: **168 / 168** agree for both, withdrawn in none.
* Removed: the **26** callee leaves. Added: **point 12, range 4–26**:
  `buf + of`, `errstr + of`, `((void*)0)`, `vsnprintf(…) + 1` and
  `(size_t)vsnprintf(…)` shapes exchange for an expression-position or
  borrowed-expression refusal.
* Functions: **0** (every caller keeps a `__builtin_va_start`/`va_end` or
  other leaf). Leak 271.

## P5. Stage 4 (= AFTER), `strerror`

* ABI census: agrees wherever declared (181 units), withdrawn in none.
* Removed: **2** nested leaves. Added: 0–2. Functions: 0 (one caller is
  provisional `Unsafe` and keeps it).

## P6. Totals

* Functions: **294 / 85 / 4,455 → 297 / 85 / 4,452** (range +2 to +8 Safe).
* Units: 3 / 43 / 135 / 4 unchanged.
* `unsupported` 45,848 → about **44,700** (range 44,300–45,200).
* `unknown_alias` 4,358 within ±30. Leak 271 (range 271–275).
* Violation candidates (`double_release`, `invalid_release`,
  `use_after_release`): **0**.

## Addendum, frozen before the route (b) stage runs were read (2026-09-25)

Item 1 moved from declaring the header-inline helpers to admitting their
validated bodies, after W44-CORPUS-CALLEE-SUMMARIES measured that glibc's
`__bswap_*` bodies validate under W4 as `Borrowed -> None` and are held back
only by its reserved-name filter. The arbiter is re-run on main `5355998d`
against `docs/reports/w44-corpus-callee-summaries/integration/artifacts/`
(functions 296 / 85 / 4,453). Committed before any route (b) stage output was
read; the runs were already in flight.

* **BEFORE** reproduces that baseline's diagnostics byte for byte.
* **Stage 0** (the `va_list` reader and spelling only; the `__extension__`
  widening is reverted): all five artifacts byte-identical to BEFORE.
* **Stage 1** (T6): removes 950–990 tail refusals, adds 0; no function moves.
* **Stage 2** (reserved-name scope lift): the 599 header-inline callee leaves
  go (range 590–599) with about 27 newly-safe-callee leaves and 0–10 added;
  exactly `ut_put_be16/32/64` move Unknown → Safe. No other reserved-name
  body is admitted with a summary that decides a leaf.
* **Stage 3** (`vsnprintf`, `vprintf`): −26 / +1, no move. **Stage 4**
  (`strerror`): −2, no move.
* **Totals**: functions 296 / 85 / 4,453 → **299 / 85 / 4,450**; units,
  leak 271 and `unknown_alias` unchanged; no violation candidate.
