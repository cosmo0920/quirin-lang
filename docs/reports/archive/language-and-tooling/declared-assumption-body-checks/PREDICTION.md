# DECLARED-ASSUMPTION-BODY-CHECKS: prediction (frozen before implementation code)

Base: local `main` at `84967949c` (VERDICT-BODY-STANDING-CLAUSE-SCOPE landed).
The ranking below is read from that card's AFTER run (`a1`, binary `65d7c0d8…`,
built at `8e9529095`; every later commit of the base changes formatting, tests
and records only), so this card's own BEFORE must reproduce it: corpus
881 / 9 / 3,944, verdict rows `97cbdcf3…`, declarations 48 validated / 29
partly / 252 declared assumptions / 1 held, Safe roll-up 418 none / 64 only
validated / 22 partly / 377 declared assumption. If BEFORE differs, this file
is amended (with the difference named) before any implementation code.

## 1. Ranking (corpus, non-libc, by Safe rows resting on the declaration)

Scope: every declaration whose body standing is `declared-assumption` or
`partly-validated` and whose module is not `quirin-provider-libc`. `mtx_lock`
and `mtx_unlock` are `libc/threads.qv` declarations with no analysed body (the
C11 threads of the C library, not a project wrapper): out of scope.

Of the 377 Safe rows on a declared assumption, 213 rest on a libc declaration
(never movable here); 164 rest on non-libc declared assumptions alone (158 on
exactly one).

| Safe rows | declaration | why not validated |
| ---: | --- | --- |
| 23 | `rd_free` | **conflict** in all 66 examining units: W4's summary `Released { nullable: false }` against the declared `resource_released(ptr, nullable)` (inherited from `libc::free`); W4 never proposes a nullable formal, so every nullable wrapper conflicts |
| 12 | `LZ4_decompress_generic` | row `Unknown` (116 diagnostics: unresolved `src`/`dst`, unsupported assignments) |
| 8 | `cfl_sds_destroy` | no body in any analysed unit |
| 8 | `rd_atomic32_get` | header body in 29 units' scope, no W4 summary (an `__atomic` builtin, no owning crate) |
| 7 | `cJSON_New_Item` | row `Unknown` (one refusal: the indirect call `hooks->allocate(…)`) |
| 6 | `mk_print` | row `Unknown` (17: variadic literal identities) |
| 5 | `LZ4_decompress_unsafe_generic`, `cfl_sds_len`, `rd_atomic64_get`; `sds_cat_safe` (partly) | row `Unknown`; no body; builtin; uncovered formals (13a alone) |
| 4 | `cio_chunk_is_up`, `cmt_mpack_consume_uint_tag`, `rd_kafka_AclBinding_new0`; `rd_kafka_mem_free`; `rd_kafka_admin_result_ret_groups`, `rd_kafka_topic_partition_get_private` (partly) | rows `Unknown`; **conflict** (row, the same nullable release); uncovered formals |
| 3 | `rd_malloc` | **conflict** in 42 units (owned result: W4 `nonnull` after `rd_assert`, declared `nullable`) and no evidence in 1 |
| 2 | `mk_mem_free` | **conflict** in 26 units (as `rd_free`) |
| 1 | `rd_calloc` | **conflict** in 45 units and no evidence in 1 |
| 0 | `flb_free`, `rd_strdup`, `rd_strndup`, `input_chunk_has_magic_bytes` | **conflict** (the last carries a counted-user clause) |

Reasons sized (non-libc declared-assumption and partly-validated declarations;
Safe-row mentions in brackets): row `Unknown` 64 (103); no body in any analysed
unit 31 (37); uncovered clause kinds (partly validated) 29 (29); header body in
scope with no W4 summary 21 (20); **conflict** 9 (33); failed otherwise 2 (0:
`cJSON_ParseWithLengthOpts` 13a, `rd_kafka_error_new_v` ABI); row `Safe` with
no evidence 1 (1: `rd_kafka_crash`, a no-return body publishes no summary).

## 2. What the card does (preference order of the brief)

* **(b) thin wrappers, via the declaration's own entry check.** The nine
  conflicts are nullability refinements (LOADER-ABI-MODES-REFINEMENT; the mode
  gate withdraws nothing), not contradictions. The exact reading of a
  declaration is the check W4 already uses to validate a summary: the body
  analysed under the declaration's entry custody, every exit matching the
  declaration (`observations_match_contract`), `Safe`, no diagnostic. Today it
  runs only on reported rows, and not when W4's summary conflicts.
  * **13m (new):** the declared-entry check runs, evidence-only, on a body the
    derivation scope holds but the unit does not report (a header `static
    inline` body), when W4 published no summary of it.
  * **13n (new):** a W4 conflict that the mode gate does not withdraw is
    recorded as a failure only when the declared-entry check fails; when it
    passes, the unit records `13n` evidence instead.
  * Both cover exactly the custody projection (as `W4` and `entry`), only for a
    function-map declaration that is its own custody projection (no container
    or counted-user clause), in force in the derivation, not refuted,
    mode-withdrawn or a withdrawn checked specification. They run with parity
    rows muted and the dependency recorder suspended: no parity row, no
    consultation, no verdict-bearing output. A forwarding wrapper validated
    this way rests on its callee's declaration exactly as every W4-validated
    body does (LOADER-ABI-MODES-RELATIVE; for `rd_free`, `libc::free` under
    the standing `libc-c-standard-abi` assumption).
* **(a) small refusals in bodies:** none of the row-`Unknown` heads is a small
  fix under existing rules (`LZ4_decompress_generic`: 116 refusals;
  `cJSON_New_Item`: an indirect call through a hooks member, rule 11p's
  domain; `rd_kafka_error_new`: the `va_list` ABI of `rd_kafka_error_new_v`).
  Registered with sizes, not attempted.
* **(c) uncovered clause kinds:** `VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES`
  amended with this card's measurement (13m applies to the function-map
  members whose body is a scope-only header body; predicted to close none);
  `VERDICT-COUNTED-USER-CLAUSE-EVIDENCE` stays registered unless an exact
  reading is found after the main work.

## 3. Predicted outcome

| quantity | BEFORE | predicted AFTER |
| --- | --- | --- |
| verdicts (all routes) | 881 / 9 / 3,944; system 87 / 4 / 357; slice 45 / 1 / 354 / 4 Pending | **identical**; verdict rows identical |
| report projection without `contract_body_evidence` | — | byte-identical to BEFORE's on all routes |
| parity multisets | — | identical (the checks are muted) |
| corpus validated | 48 | **53–58**: `rd_free`, `rd_kafka_mem_free`, `mk_mem_free`, `flb_free`, `rd_strdup`, `rd_strndup` (13n); `rd_malloc`, `rd_calloc` only if their one evidence-less unit validates too (13m) |
| corpus partly validated | 29 | 29 (no structural declaration closes) |
| corpus declared assumptions | 252 | 244–247 |
| Safe: only validated | 64 | **87–95** (`rd_free` alone +23; all eight +31) |
| Safe: on a declared assumption | 377 | 346–354 |
| Safe: partly / none | 22 / 418 | 22 / 418 |
| not validated | — | `rd_atomic*` (builtin), `rd_realloc` (no libc `realloc`), `mk_list_add`/`mk_list_del` (container clauses), `input_chunk_has_magic_bytes` (counted-user clause), `rd_kafka_topic_partition_get_private` (an uncontracted call in its body) |
| system / slice only validated | 3 / 0 | 3–8 / 0–2 |
| Safe → non-Safe | — | none (no verdict reads the label) |
