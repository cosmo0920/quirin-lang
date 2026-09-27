# W43-LIBC-PRINTF-FAMILY: frozen prediction

Frozen **before** any printf-family declaration was drafted into
`providers/quirin-provider-libc/contracts/libc/stdio.qcore` or run through the
arbiter. It is committed on its own so the outcome can be compared with it
([README §7](README.md#7-prediction-against-outcome)). Nothing below is
edited after the first staged run. It rests only on these inputs:

* the BEFORE run (`measure/before/`, base `f8b41209`), which reproduces the
  W43-LIBC-RESULT-ALIAS diagnostics byte for byte;
* the declared-ABI census of the nine brief-named members over the 185
  captured units ([measurement/family9.qcore](measurement/family9.qv));
* two lexical forecast scripts over the analysed `-E -P` texts,
  [scripts/call_shapes.py](scripts/call_shapes.py) and
  [scripts/line_bound.py](scripts/line_bound.py);
* a reading of the gateway's tail rule
  (`variadic_nested::tail_arguments_are_data_with_nested`) and of the C02-P
  closure (`borrowed_actual::untracked_bases`, `seed_call_arguments`).

## P1. Which members are declared

| Member | Called in the corpus (leaves / functions) | Census (units agreeing / withdrawn) | Forecast |
|---|---|---|---|
| `printf` | 22 / 16 | 168 / 0 | declared |
| `fprintf` | 1,138 / 239 | 168 / 0 | declared |
| `snprintf` | 1,044 / 234 | 168 / 0 | declared |
| `sprintf` | 8 / 4 | 168 / 0 | declared |
| `vsnprintf` | 25 / 25 | 0 / 168 (`abi-unreadable-declaration`) | **refused by name**: the reader has no spelling for `__gnuc_va_list` (`__builtin_va_list`), so a declaration would be withdrawn at every call site and only rename leaves |
| `vprintf` | 1 / 1 (`mk_print`) | 0 / 168 | refused, same ground |
| `vfprintf`, `vsprintf` | 0 | 0 / 168 | not called; refused on the same ground in any case |
| `dprintf` | 0 | 168 / 0 | not called, so not declared |

## P2. Leaves removed

* **Declared-callee: exactly 2,212**, every `call to uncontracted function`
  leaf of the four: `printf` 22, `fprintf` 1,138, `snprintf` 1,044,
  `sprintf` 8. Every call site of the four is in a unit whose declaration the
  census reads as agreeing, and every leaf is a discarded or assigned call, so
  none can survive as an uncontracted-callee leaf.
* **Withdrawn-contract leaves: 0** for all four names.
* **Same-line and same-function removals: 0 to 200.** Two mechanisms, neither
  measured lexically with confidence:
  * condition-position calls (`if (snprintf(…) >= n)`) report today as
    `unmodeled call … in branch condition`, not as a callee leaf, and the
    condition route may now model them;
  * `seed_call_arguments` refuses as an access base every name passed to an
    uncontracted call. Once the four are declared, a name passed only to
    borrowed or tail positions of them leaves that set, which can admit a chain
    or a borrowed actual elsewhere in the same body.

## P3. Leaves added (the exchange)

**Point forecast 90, range 40 to 150.** The call-site census
([call-shapes.json.gz](call-shapes.json.gz), from `call_shapes.py`) finds, over
the 2,212 sites:

| Tail shape (worst actual) | Sites | Forecast |
|---|---:|---|
| literals only, or no tail | 939 | admitted: no leaf |
| a bare name (local, global or `__FUNCTION__`) | 1,204 (`fprintf` 1,106) | mostly admitted (never bound); a name the closure keeps risky refuses with `arguments beyond the declared parameters of X` |
| a call | 39 | mostly `nested call` refusals, since most inner callees are uncontracted |
| a bare pointer formal | 25 | admitted unless the formal is risky or protected for another reason |
| an expression, dereference, subscript or address | 23 | mostly refused: tail premise T2, or the lexical indirect-access gate |

The added families, forecast: `nested call` about 35, tail refusals about 30,
`indirect memory access` and `unmodeled formal address or borrowed expression`
about 20, and a few `unknown_alias` rebindings. **No added
`call to X whose declared contract was withdrawn` leaf.**

## P4. Functions moved

**Five Unknown → Safe**, the five whose only blocker is one of the four
(`sole_callees.py` over the baseline, and `line_bound.py`, which finds no other
function whose every leaf is on a family line except
`rd_kafka_buf_read_CurrentLeader`, which carries 21 other leaves and does not
move):

| Function | Unit | Blocker | Forecast and the exchange risk the ledger names |
|---|---|---|---|
| `cmt_errno_print` | `cmt_log.c` | `fprintf(stderr, "…", file, line, errnum, buf)` | **Safe.** `file` is a `const char *` formal, the `MK-PRINT-TAIL-ACTUALS` shape. It is not protected (never dereferenced with `*`, never at a custody position) and after the declaration it is passed only to a tail position, so the C02-P closure no longer refuses it and T4 admits it. `buf` is passed only to `strerror_r`'s borrowed position. `line`, `errnum` are declared scalars |
| `ctr_errno_print` | `ctr_log.c` | same body | **Safe**, same reading |
| `cio_errno_print` | `cio_log.c` | same body | **Safe**, same reading |
| `rd_kafka_mk_nodename` | `rdkafka_broker.c` | `snprintf(dest, dsize, "%s:%hu", name, port)` | **Safe.** The ledger forecast an exchange (`name` is a `const char *` formal). Forecast here: no exchange, by the same closure argument as `file` |
| `cJSON_Version` | `cJSON.c` | `sprintf(version, "%i.%i.%i", 1, 7, 18)` | **Safe.** The tail is three literals. `version` is a `static` local array, but `return version;` carries no leaf at the baseline, and nothing in the declaration changes the return route. The ledger's independent exchange risk is forecast **not** to fire |

**Zero to five further moves** through same-function effects and through
newly validated summaries (`rd_kafka_mk_nodename`'s caller
`rd_kafka_broker_find` loses one callee leaf but keeps six other reasons).
**Zero** moves out of provisional `Unsafe`, **zero** moves into it, and **zero**
Safe functions move.

## P5. Units, pins and candidates

* Unit statuses: **0** move.
* `leak`: **272, unchanged.** Every admitted tail actual is by construction a
  name the body can never bind to a tracked identity, so no binding the old
  refusal poisoned becomes a live obligation.
* `double_release`, `invalid_release`, `use_after_release`: **0 / 0 / 0**, no
  violation candidate.
* `summary_conflict`: 1, unchanged (no family body is defined in any unit, so
  the mode gate has nothing to compare).
* `unknown_alias`: within ±20 of 4,657.
* `unsupported`: about −2,100 (−2,212 + about 90 added, less the same-line
  removals).
