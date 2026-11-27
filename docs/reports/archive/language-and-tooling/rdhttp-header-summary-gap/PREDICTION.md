# RDHTTP-HEADER-SUMMARY-GAP: prediction (frozen before implementation code)

Base: local `main` at `b21f0c9a2`. BEFORE binary `private/rhsg/bin/before`
(`18cf8371…`, built at the base, clean tree), every route exit 0, reproducing
the base exactly:

| route | totals | report sha256 | verdict rows | roll-up (none / only validated / partly / declared) |
| --- | --- | --- | --- | --- |
| corpus | 881 / 9 / 3,944 | `fafb1160…` (projection `cd4187bb…`) | `97cbdcf3…` | 418 / 91 / 23 / 349; declarations 51 / 31 / 247 / 1 held |
| system | 87 / 4 / 357 | `b438aea0…` | | |
| slice | 45 / 1 / 354 / 4 Pending | `883d4d3c…` | | |

## Cause (isolated before this prediction)

* The unit's frontend is the same: `rdhttp.c` and `rdbuf.c` share every flag
  of the pinned compile database (`-std=gnu17`, the same `-D`/`-I` lists).
* Clang's JSON AST of `rdhttp.c` nests **224** levels deep
  ([`scripts/depth.py`](scripts/depth.py)): four definitions,
  `rd_http_ssl_configure` 224, `rd_http_get` 214, `rd_http_req_init` 212,
  `rd_http_post_expect_json` 212, each calling `curl_easy_setopt`, whose GCC
  type-check macro (`curl/typecheck-gcc.h`, active under `__GNUC__`) expands
  into nested conditionals. `rdkafka_sasl_oauthbearer_oidc.c` (also a
  libcurl user) nests 57.
* `typed_access_path::unit_ast` parses the whole-unit JSON with
  `serde_json::from_slice` (default recursion limit 128). The parse fails, the
  unit index is `None`, and `StatementIndex::read` answers `None` for **every**
  definition of the unit, the `rd.h` header bodies included.
* `rd_malloc`, `rd_calloc` and `rd_strdup` each spell `rd_assert(p)`, glibc's
  `assert`, a GNU statement expression under `gnu17` (rule 11g), read only
  from the unit index; without it W4 publishes no summary, so rule 13m has no
  conflict to check and the unit records the body as examined with no evidence
  (`unvalidated`), which keeps each declaration a declared assumption (rule
  13g). `rd_free` (no statement expression) needs no typed reading and
  validates there.
* Minimal reproduction (`crates/qcore-c-gateway/tests/deep_unit_ast.rs`, run
  on the base, `private/rhsg/controls-before3.log`): an `rd_malloc`-shaped
  header wrapper beside an `else if` chain of 4 arms is `13m`-validated; beside
  80 arms (JSON depth 173) it is `Unvalidated` with the same record shape as
  `rdhttp.c`; a release wrapper validates at every depth.

## The fix (rules 13o, 13p)

A new module reads a unit's JSON AST whose nesting exceeds `serde_json`'s
default limit but stays within an admitted bound (256, twice the default)
with the limit disabled, on a dedicated thread with a large stack, as two
readers already do (`typed_dispatch_census`, `index_extent`). The parsed
document is the same; only the stack guard changes. A unit beyond the bound
keeps no index (as now). Nothing changes for a unit that parses today (the
deep path runs only after the default parse fails). The rule-13g fold is not
touched.

## Predictions

1. **Corpus verdict rows.** Every unit but `rdhttp.c` byte-identical in its
   rows. In `rdhttp.c` the four `typed AST is unavailable (CG30)` leaves go;
   each of those rows keeps other leaves, so no status moves there; other
   `rdhttp.c` rows may gain or lose leaves from typed readings, with no status
   move predicted. Totals 881 / 9 / 3,944 unchanged. No Safe → non-Safe.
   Uncertain: `unittest_http_get_params_append` (Unsafe on a leak) could
   change leaves; any move is attributed by reading.
2. **Unit evidence.** `rdhttp.c` records `13m` for `rd_malloc`, `rd_calloc`
   and `rd_strdup` (W4 conflicts with the nullable result, as in the 42, 45 and
   31 other units); nothing else changes in any other unit.
3. **Corpus roll-up.** Declarations validated 51 → **54**, declared
   assumptions 247 → **244**, partly 31, held 1. Safe roll-up 418 / 91 → 91–95
   / 23 / 349 → 345–349: `rd_malloc`'s 3 and `rd_calloc`'s 1 Safe rows move
   to "only validated" only where they rest on no other declared assumption.
4. **Parity multisets.** Changes only in rows from `rdhttp.c` (typed readings
   newly available there); every other unit's rows identical.
5. **System and slice.** No librdkafka unit on these routes is deep (no
   `rdhttp.c`): verdict rows, parity multisets and roll-ups identical.
6. **The other 21 declarations of the residual row.** Not covered by this
   fix: their unvalidated units are not deep (the atomic builtins, `rd_realloc`,
   `rd_file_read`, `mk_list_add`/`mk_list_del`, `mk_utils_libc_error`,
   `flb_log_check`). Correction to the ranking: `cb_queue_chunks` and the 11
   `sb_*` declarations examine no body at all on the corpus (`withdrawn_units`
   180, `C06.3-no-body`, no unvalidated unit); the ranking's fallthrough put
   them in this class.
7. **Wall time.** Corpus within noise (one extra parse of `rdhttp.c`'s ~750 MB
   JSON, about a second).
