# RDHTTP-HEADER-SUMMARY-GAP: the unit index of a unit nesting beyond the default JSON limit

Card RDHTTP-HEADER-SUMMARY-GAP (wave 48, card-high), on the user's direction
of 2026-10-09: "handle as safe as possible"; raise the checked count without
weakening any check. Base: local `main` at `b21f0c9a2`.
[PREDICTION.md](PREDICTION.md) was committed at `d63ef67b3`, before any
implementation code.

**Published property and scope.** No new property: custody (`c.custody`) on
the pinned corpus, the system route and the tail-pause slice. The card finds
why librdkafka's `rdhttp.c` held no body evidence for three `rd.h` wrappers
every other unit validates, and repairs the transport defect behind it.
Gateway rules **13o** and **13p**; Lean `QCore.DeepUnitAst`.

## 1. BEFORE

Binary `private/rhsg/bin/before` (`18cf8371…`, built at the base, clean tree),
every route exit 0, reproducing the base exactly: corpus 881 / 9 / 3,944
(report `fafb1160…`, projection `cd4187bb…`, verdict rows `97cbdcf3…`, Safe
roll-up 418 / 91 / 23 / 349, declarations 51 validated / 31 partly / 247
declared assumptions / 1 held), 459.7 s; system 87 / 4 / 357 (`b438aea0…`),
64.4 s; slice 45 / 1 / 354 / 4 Pending (`883d4d3c…`), 28.6 s. A second BEFORE
corpus run (`before2`) is byte-identical, 459.6 s.

## 2. The cause, isolated

| candidate (brief) | finding |
| --- | --- |
| different preprocessing context | **no**: `rdhttp.c` and `rdbuf.c` share every flag of the pinned compile database (`-std=gnu17`, the same `-D` and `-I` lists) |
| a unit-level refusal before those bodies | **no**: the unit is analysed; its rows carry row-level leaves only |
| an inference budget or cut-off | **no** |
| the 13m eligibility conditions | **no**: there is no conflict to check, because W4 publishes no summary |
| a different declaration identity | **no**: the same header text, the same `in force` declaration |
| **(found)** the unit's typed AST index is absent | **yes** |

* Clang's JSON AST of `rdhttp.c` nests **224** levels
  ([`scripts/depth.py`](scripts/depth.py), counted as `serde_json` counts:
  each `[` or `{` outside a string). Four definitions are deeper than 128:
  `rd_http_ssl_configure` 224, `rd_http_get` 214, `rd_http_req_init` 212,
  `rd_http_post_expect_json` 212. Each calls `curl_easy_setopt`, whose GCC
  type-check macro (`curl/typecheck-gcc.h`, active under `__GNUC__`) expands
  into a statement expression of nested conditionals, one per option kind.
* `typed_access_path::unit_ast` parsed the whole-unit JSON with
  `serde_json::from_slice`, whose default recursion limit (a stack guard)
  refuses 128 levels or more. The parse failed, the unit had **no index**,
  and `StatementIndex::read` answered `None` for every definition of the
  unit, the `rd.h` header bodies included.
* `rd_malloc`, `rd_calloc` and `rd_strdup` each spell `rd_assert(p)`: glibc's
  `assert`, which under `gnu17` is the statement expression
  `__extension__ ({ if (p) ; else __assert_fail (…); })` (rule 11g), read
  only from the unit index. Without it W4 publishes no summary, rule 13m has
  no conflict to settle, and `unit_evidence` records the body as examined
  with nothing (`unvalidated`), which keeps each declaration a declared
  assumption (rule 13g's fold needs every examining unit). `rd_free` has no
  statement expression and validates there by `13m` without a typed reading.
* CG30's rule 11y had measured the same depth (its "nests 223 deep" note in
  `definition_ast`; this card's count of the compile-database unit is 224)
  and added the `typed AST is unavailable (CG30)` refusal to the four deep
  definitions; the effect on every other definition of the unit was not
  recorded.
* **Minimal reproduction** ([`tests/deep_unit_ast.rs`](../../../crates/qcore-c-gateway/tests/deep_unit_ast.rs),
  run on the base, [`artifacts/controls-before3.log`](artifacts/controls-before3.log)): an `rd_malloc`-shaped
  wrapper (`malloc`, `assert(p)`, `return p`) in an included header, declared
  `resource_return_owned(nullable)`, beside an `else if` chain whose Clang
  JSON nests `2n + 13` levels: 4 arms (21) `13m`-validated; **80 arms (173)
  `Unvalidated`**, the record shape of `rdhttp.c` (`examined: {w}`, no
  evidence, no failure); a release wrapper with no statement expression is
  `13m`-validated at 4, 80 and 130 arms.

**Depth census** ([`scripts/census.py`](scripts/census.py), the pinned
route's own compile databases, [`artifacts/census-before.txt`](artifacts/census-before.txt)):
every unit in which any of the residual row's declarations is `unvalidated`
on BEFORE (61 units, 62 compile entries, over librdkafka, chunkio, Fluent Bit
and Monkey). Only `rdhttp.c` exceeds 127
(224); the others nest 45 to 82, so no other declaration of the row is held
by this cause.

## 3. The fix (rules 13o, 13p)

`crates/qcore-c-gateway/src/deep_unit_ast.rs`, one call from
`typed_access_path::AstIndex::from_json`:

* **Rule 13o.** The document is parsed as before. Only when that parse fails
  is its nesting measured (`depth`); a document of at most `ADMITTED_DEPTH`
  (256, twice the default) is parsed again with `disable_recursion_limit` on a
  dedicated 256 MiB stack, exactly as the typed dispatch reader
  (`typed_dispatch_census`) and the index-extent reader already parse Clang's
  JSON (the crate already enables `serde_json`'s `unbounded_depth`; no crate
  added). The text and the parser are the same, so the index is the document
  the default parse would have read without its guard; every reader reads the
  unit as any other, and no reader's admission changes.
* **Rule 13p.** Beyond the bound, on a failed reparse (malformed JSON, trailing
  bytes) or when the thread cannot start, the default parse's error is
  returned: the unit keeps no index, as before, and a body it examined without
  evidence stays `unvalidated`. The rule-13g fold is untouched; the brief's
  prohibition (never let the fold ignore a unit that examined a body without
  validating it) holds by construction.

**Why this does not weaken a check.** The repair supplies the same typed
reading every other unit has; the evidence it enables in `rdhttp.c` is the
`13m` check of rules 13m/13n, performed in the 42, 45 and 31 other units. The
readers now reach the four deep definitions too, and refuse what they do not
model (§4). Nothing else changes for a unit that parses today: the deep path
runs only after the default parse fails.

**Bound and stack.** The reparse runs on its own stack; the readers then
recurse over the index on the analysis worker. The deep fixture (80 arms,
depth 173) runs the readers on a default 2 MiB debug test thread, and the
corpus run reads `rdhttp.c` (224) on the arbiter's workers; both completed.
The beyond-bound fixtures (130 arms) run on a 256 MiB thread because the
token route's own `else if` recursion overflows a debug test thread there
**at the base as after the card** (the base's test binary aborts with a stack
overflow at 130 and 200 arms; measured, not changed by this card).

## 4. AFTER and attribution

Binary `private/rhsg/bin/a1` `b51d4b20…` (= `run.json`), built at
`e6d5bdf8a` (clean tree), every route exit 0. Comparison:
[`scripts/identity.sh`](scripts/identity.sh) (projection, parity, clause
roll-up), [`scripts/attribute.py`](scripts/attribute.py) (every report
difference by unit and row), [`scripts/parity_units.py`](scripts/parity_units.py)
(every parity difference by analysed source).

| route | totals | verdict rows | report projection | parity multiset | roll-up |
| --- | --- | --- | --- | --- | --- |
| corpus | 881 / 9 / 3,944 (unchanged) | `97cbdcf3…` → `321a8913…`: **no status moves**; leaves of 13 `rdhttp.c` rows | differs only in `rdhttp.c` (33 paths) | 662,667 → 683,320 rows; all 21,075 differing rows (211 only before, 20,864 only after) from `rdhttp.c`'s source | 418 / 91 / 23 / 349 → 418 / **94** / 23 / **346** |
| system | 87 / 4 / 357 | identical | identical | identical (139,962) | unchanged (47 / 2 / 4 / 34) |
| slice | 45 / 1 / 354 / 4 Pending | identical | identical | identical (50,407) | unchanged (29 / 2 / 2 / 12) |

A confirmation corpus run (`a1b`) is byte-identical to `a1` (report
`ab8f5f14…`) and its parity multiset identical.

**`rdhttp.c` rows** ([`artifacts/attribution-a1.json`](artifacts/attribution-a1.json)). Every moved leaf is
a refusal; no leak or violation is added or removed; every status holds
(`unittest_http_get_params_append` stays `Unsafe` on its leak at
`rdhttp.c:802`, untouched).

| leaves removed (60) | leaves added (1,028) |
| --- | --- |
| 34 braced initializer not modeled; 19 indirect access outside the projection; 16 unmodeled access in a branch condition; 6 variadic tail beyond `fprintf`'s parameters; 4 `typed AST is unavailable (CG30)`; 4 uncontracted calls (`__builtin_va_*`); 4 unsupported assignment; 1 unrecognized destination; 1 nested call; 1 loop condition | 939 uncontracted calls: 908 of the type-check macro's 44 `Wcurl_easy_setopt_err_*` warning functions, now read inside its statement expressions, and 31 others the rows now reach (`curl_easy_setopt` 18, `rd_strcmp` 2, `rd_http_error_new` 2, nine more once each); 52 unmodeled access in a branch condition; 25 unresolved identities; 7 indirect access; 4 va_list protocol refusals (`ap2` in an inner block); 1 nullable resource used by a non-null contract |

Rows: `rd_http_ssl_configure` (−17 / +434), `rd_http_req_init` (−11 / +314),
`rd_http_post_expect_json` (−7 / +158), `rd_http_get` (−9 / +84),
`rd_http_req_perform_sync` (−4 / +13), `rd_http_error_new` (−8 / +9),
`unittest_http_get` (−9 / 0), `unittest_http_get_params_append` (−7 / +1),
`rd_http_ssl_ctx_function` (−5 / +5), `rd_http_req_destroy` (−4 / +2),
`rd_http_parse_json` (−4 / +3), `rd_http_req_write_cb` (−3 / +3),
`rd_http_error_new_from_buf` (−2 / +2). The unit's own
`contract_body_evidence` and `diagnostic_counts` are its only other moved
keys.

**Parity** by position (only after): `cast-semantics-census` 10,366,
`elaboration-statement` 3,475, `condition-desugar` 3,124, `call-exit-census`
3,090, then 124 `statement-expression-statement` and smaller classes; by
function, the four deep definitions hold 19,870 of the 21,075; the header
bodies the unit's derivation reads (`rd_malloc`, `rd_calloc`, `rd_strdup` 8
each, `rd_file_stat`, `rd_usleep`, `rd_getenv`, `rd_buf_len`,
`rd_atomic32_get`, `rd_free`), OpenSSL's `lh_*_thunk` and one
`rd_kafka_q_io_event` row are all from `rdhttp.c`'s source.

**Roll-up** ([`artifacts/clauses-a1-corpus.json`](artifacts/clauses-a1-corpus.json)). Moved declarations, each
`declared-assumption` → `validated` with `13m` in every examining unit:
`rd_malloc` (43 units, 3 Safe rows), `rd_calloc` (46, 1), `rd_strdup` (32, 0).
Validated 51 → **54**, declared assumptions 247 → **244**, partly 31, held 1.
Every moving Safe row, attributed: `rd_kafka_mem_calloc` (`rd_calloc`),
`rd_kafka_mem_malloc` and `rd_list_alloc` (`rd_malloc`) move to "only
validated"; `rd_malloc`'s third row rests on another declared assumption.
Every "only validated" row of BEFORE is one AFTER; none fell.

**Wall time.** Corpus 459.7 / 459.6 s BEFORE → **527.7 / 528.5 s** AFTER
(+68 s, +15%, two runs each, `--jobs 4`, lock time excluded); system 64.4 →
64.1 s; slice 28.6 → 29.6 s. The prediction ("within noise") was wrong. Every
report and parity difference is in `rdhttp.c`, so the cost is that unit's:
its JSON (about 750 MB for the compile-database unit) is now indexed and read by every typed reader (20,864 more
parity rows), and on four `--jobs` it is on the critical path.

**`mk_vhost_set` tip probe** ([`scripts/tip-probe.sh`](scripts/tip-probe.sh),
[`tip_compare.py`](scripts/tip_compare.py)): verdict rows identical on the tip
(`a35c40cf…`) and the master copy (`7bf0efa2…`); `mk_vhost_set` Unknown (tip 1
leaf, master 6), `mk_vhost_set_property` Unknown (2); `mk_lib.c` nests 47.

## 5. The other declarations of the residual row

The row (DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS class 3) named 24
declarations. Re-measured AFTER (ranking
[`scripts/ranking.py`](scripts/ranking.py), `artifacts/ranking-a1.txt`):

* **Closed by this card (3):** `rd_malloc`, `rd_calloc`, `rd_strdup`.
* **Correction: no examined body (12).** `cb_queue_chunks` and the 11
  `sb_*` storage-backlog declarations record no examining unit on the corpus
  (`bodies`: 0 validated, 0 unvalidated, `withdrawn_units` 180, all
  `C06.3-no-body`); the previous ranking fell through to this class for
  them. They are reclassified `no-examined-body` (the ranking script now says
  so). 0 Safe rows.
* **Their own causes, not this one (9; every unvalidated unit nests 45–82):**
  `rd_atomic32_get` (29 units, 8 Safe rows), `rd_atomic64_get` (5 units, 5),
  `rd_atomic64_add` (9, 3): an `__atomic` builtin with no owning crate
  (W48-CALLEE-DECISION-TABLE class 6; `rdhttp.c` stays unvalidated for
  `rd_atomic32_get` after the fix); `mk_utils_libc_error` (5, 3);
  `rd_file_read` (1, `rdkafka_sasl_oauthbearer_oidc.c`, 1; depth 57);
  `flb_log_check` (1, `flb_input_chunk.c`, 0); `mk_list_add` (18, 0),
  `mk_list_del` (17, 0); `rd_realloc` (8, 0). W4 publishes no summary of
  these bodies in units that hold their typed index; the causes were not
  isolated here. 20 Safe-row mentions.

## 6. Prediction against outcome

| prediction | outcome |
| --- | --- |
| no status move; leaves move only in `rdhttp.c`; the four CG30 leaves go | held (13 rows; no status move; no leak or violation added) |
| `rdhttp.c` records `13m` for the three wrappers; nothing else in any other unit | held |
| validated 51 → 54; declared 247 → 244; only validated 91 → 91–95; declared-assumption rows 349 → 345–349 | 54, 244, **94**, **346** |
| parity changes only in `rdhttp.c` rows | held (21,075 rows, one source) |
| system and slice identical | held |
| the other 21 declarations not covered; the 12 no-body ones a ranking fallthrough | held (census: none of their units nests beyond 82) |
| corpus wall time within noise | **wrong**: +68 s (+15%), §4 |

## 7. Lean

`proof/QCore/DeepUnitAst.lean`, audited by `proof/AuditDeepUnitAst.lean` (8
theorems, no placeholder, no project axiom): `an_index_read_before_is_read_after`,
`the_index_is_the_document`, `beyond_the_bound_there_is_no_index`,
`only_the_deep_band_changes`,
`a_unit_without_an_index_keeps_the_declaration_unvalidated`,
`the_fold_never_skips_a_unit` (both from
`QCore.ContractDependencies.an_unvalidated_body_is_never_validated`),
`depth_vectors_hold` and `band_edges`. The depth scan's vectors are bound to
the Rust (`deep_unit_ast::tests::lean_depth_vectors_are_the_rust_vectors`,
`depth_vectors_hold`, `band_edges_match_lean`). Not proved: that
`serde_json` reads the same value with and without its limit (the library's
contract, checked on nested documents by
`a_deep_document_within_the_bound_is_the_same_value`), and Clang's fidelity
(trust boundary 28).

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/).

## 9. Ledger

* `DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS` **amended**: the `rdhttp.c`
  reading isolated and repaired (its DISCHARGE clause "the rdhttp.c unit's
  missing reading isolated" holds); class 3 re-measured (3 closed, 12
  reclassified no-examined-body, 9 left with their own causes and sizes);
  the other classes unchanged. Not retired: classes 1–7 remain.
* Registered **`DEEP-AST-FILTERED-TRANSPORTS`**: the filtered launch
  (`typed_c_ast::ast_declarations`, used by `definition_ast`, `function_ast`
  and the appended-declaration probe) and the closed-world
  `translation_unit_ast` still parse under the default limit; a definition
  itself nesting 128 or more (the four `rdhttp.c` definitions) has no
  filtered AST. No corpus row is measured to depend on it after this card
  (rule 11y now reads the unit index).
* Registered **`RDHTTP-DEEP-UNIT-WALL-TIME`**: the corpus arbiter's +68 s
  (459.6 → 528.1 s mean, `--jobs 4`) from reading `rdhttp.c`'s index.

## Files

* Code: `crates/qcore-c-gateway/src/deep_unit_ast.rs` (new),
  `crates/qcore-c-gateway/src/typed_access_path.rs` (hook, comment),
  `crates/qcore-c-gateway/src/lib.rs` (module).
* Tests: `crates/qcore-c-gateway/tests/deep_unit_ast.rs` (new), the module's
  unit tests.
* Lean: `proof/QCore/DeepUnitAst.lean`, `proof/AuditDeepUnitAst.lean`,
  `proof/QCore.lean`.
* Rules: gateway 13o, 13p, router rows. `providers/` unchanged: no contract
  edited, no re-pin.
* Scripts: [`scripts/`](scripts/); artifacts: [`artifacts/`](artifacts/).
