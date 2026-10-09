# DECLARED-ASSUMPTION-BODY-CHECKS: refinement conflicts settled by the declaration's own entry check

Card DECLARED-ASSUMPTION-BODY-CHECKS (wave 48, card-high), on the user's
direction of 2026-10-09: "make the Safe count more trustworthy rather than
larger"; "as safe as possible". Base: local `main` at `84967949c`, rebased
onto `d8a1a2a4e` (VERDICT-CONTRACT-PREMISE-ANNOTATIONS landed) before the
final measurement. [PREDICTION.md](PREDICTION.md) was committed at
`8efa7811e` (`117e372d2` before the rebase), before any implementation code.

**Published property and scope.** No new property: custody (`c.custody`) on
the pinned corpus, the system route and the tail-pause slice, unchanged. The
card moves Safe verdicts from resting on declared assumptions to resting on
validated declarations by checking declarations against bodies the units
already hold. Gateway rules **13m** and **13n**; Lean `QCore.DeclaredEntryCheck`
(and `QCore.BodyClauseStanding`'s evidence table); contract specification
C06.5.

## 1. BEFORE and the ranking

Two BEFORE runs, every route exit 0, both reproducing their base's figures:

| run | base | binary | corpus | system | slice |
| --- | --- | --- | --- | --- | --- |
| `before` | `84967949c` | `c616df60…` | 881 / 9 / 3,944, report `c1e6e7d1…` (byte-identical to the previous card's AFTER), 531.4 s | 72.5 s | 33.3 s |
| `before2` | `d8a1a2a4e` | `f4b6b753…` | 881 / 9 / 3,944, verdict rows `97cbdcf3…`, 449.5 s | 87 / 4 / 357, 62.5 s | 45 / 1 / 354 / 4 Pending, 27.3 s |

`before2`'s roll-up is the coordinator's: corpus 418 none / **63** only
validated / 23 partly / 377 declared assumption; declarations 46 validated /
31 partly / 252 declared assumptions / 1 held.

**Ranking** ([`scripts/ranking.py`](scripts/ranking.py); full lists
[`artifacts/ranking-before2.txt`](artifacts/ranking-before2.txt) and, for the
prediction's base, [`ranking-before.txt`](artifacts/ranking-before.txt)).
Non-libc declared-assumption and partly-validated declarations by corpus Safe
rows, each with why no body validates it. `mtx_lock`/`mtx_unlock` are
`libc/threads.qv` declarations with no analysed body (the C library's C11
threads, not a project wrapper): out of scope. Of the 377 Safe rows on a
declared assumption, 213 rest on a libc declaration; 164 on non-libc
declared assumptions alone (158 on exactly one).

| reason (`before2`) | declarations | Safe-row mentions | heads |
| --- | ---: | ---: | --- |
| row `Unknown` (the body is a reported row whose analysis refuses) | 60 | 94 | `LZ4_decompress_generic` 12 (116 refusals), `mk_print` 6, `LZ4_decompress_unsafe_generic` 5, `cio_chunk_is_up` 4, `cmt_mpack_consume_uint_tag` 4, `rd_kafka_AclBinding_new0` 4 |
| no body in any analysed unit | 31 | 37 | `cfl_sds_destroy` 8, `cfl_sds_len` 5, `cfl_sds_cat_safe` 3 |
| **W4 conflict** (a refinement: the mode gate withdraws nothing) | 9 | 33 | **`rd_free` 23**, `rd_kafka_mem_free` 4, `rd_malloc` 3, `mk_mem_free` 2, `rd_calloc` 1; `flb_free`, `rd_strdup`, `rd_strndup`, `input_chunk_has_magic_bytes` 0 |
| uncovered clause kinds (partly validated) | 26 | 29 | `sds_cat_safe` 5, `rd_kafka_admin_result_ret_groups` 4, `rd_kafka_topic_partition_get_private` 4 |
| header body in scope, no W4 summary | 21 | 20 | `rd_atomic32_get` 8, `rd_atomic64_get` 5 (an `__atomic` builtin with no owning crate), `mk_utils_libc_error` 3 |
| **named premise** (C04.18; never fully validated, reported apart) | 9 | 10 | `cJSON_New_Item` 7 (`cjson_allocator_hooks`), `add_item_to_array` 1, `flb_input_chunk_get_name` 1, `rd_kafka_q_purge0` 1; five at 0 |
| failed otherwise | 2 | 0 | `cJSON_ParseWithLengthOpts` (13a), `rd_kafka_error_new_v` (ABI) |
| row `Safe`, no evidence | 1 | 1 | `rd_kafka_crash` (a no-return body publishes no summary) |

The nine conflicts are the card's target. W4 seeds every bound formal
non-null and never weakens it, and reads a result checked by `rd_assert`
(glibc's `assert` ending at the no-return `__assert_fail`) as non-null, so
every nullable wrapper disagrees with its declaration: `rd_kafka_mem_free`'s
row carries ``inferred … [Borrowed, Released { nullable: false }] contradicts
the declared … [Borrowed, Released { nullable: true }]``. Rule 13g recorded
each disagreement as a body failure. The mode gate (LOADER-ABI-MODES) already
classifies them as refinements within one duty
(`LOADER-ABI-MODES-REFINEMENT`), withdrawing nothing.

## 2. What changed (rules 13m, 13n)

`crates/qcore-c-gateway/src/legacy_declared_entry_check.rs` (a thin hook in
`analyze_translation_unit_under_holds`, before `unit_evidence`):

* **Rule 13m.** For a function-map declaration that is its own custody
  projection, in force in the derivation exactly as declared, not refuted
  (C06.4), not mode-withdrawn and not a withdrawn checked specification
  (C06.3), whose W4 summary conflicts with it, the body is analysed under the
  **declaration's** entry custody on the CFG the reported run builds (the
  declaration is the contract in force, so its formals are protected as
  there; a derived result on the derived-return CFG, as the row route does).
  It passes only when `observations_match_contract` holds at every exit, the
  result is `Safe` with no diagnostic, no callee double release is found, and
  rule 11y's dialect and extension refusals name nothing: the row route's
  `entry` check with every later row refusal added. Passing, the unit records
  `13m` evidence and no `conflict` failure; failing, the failure stands. `13m`
  covers the custody projection, as `W4` and `entry` do (rule 13i).
* **Rule 13n.** The check runs only while evidence is collected
  (`contract_dependencies::collecting`), with parity rows muted
  (`without_parity_rows`) and the recorder suspended
  (`contract_dependencies::unrecorded`, a fresh discarded consultation log).
  No verdict reads it; a row whose summary conflicts keeps its
  `SummaryConflict` and its `Unknown`.

Shared items (named): `contract_dependencies::{collecting, unrecorded}` (new);
`contract_dependencies::unit_evidence` takes the check's result and records
through `declared_entry_check::recorded`; `contract_clause_standing::{EVIDENCE,
covers}` gain `13m`; the hook moves `dialect_hook::Extensions::of` before
`unit_evidence` (same tokens, read once); the sidecar's `MEANING` names rule
13m.

**Why this does not weaken a check.** W4 publishes a summary only after the
same `observations_match_contract` check on the body under that summary's
entry. An equal W4 summary therefore implies the declaration's own entry
check; 13m performs that check on the declaration directly, adds the row
route's later refusals, and is consulted only where the mode gate found no
duty contradiction. A refinement in the caller's favour cannot pass it: the
control `a_nullable_formal_handed_to_a_non_null_release_is_not_validated`
(a nullable formal passed to a non-null release) keeps its `conflict`
failure. A forwarding wrapper validated this way rests on its callee's
declaration exactly as every W4-validated body does
(`LOADER-ABI-MODES-RELATIVE`): `rd_free`'s `free(ptr)` on `libc::free` under
the standing `libc-c-standard-abi` assumption.

**Withdrawn before the record: a scope-only route.** The prediction also
named a check of a header body W4 published nothing for. It was implemented
and measured (`a1`, binary `0bb3a795…`, base `84967949c`): it established
nothing on any route, and no fixture reached it (every shape tried, nullable
guards, aliases, interior and borrowed results, member reads, gave W4 a
summary or failed the entry check too). It was removed rather than kept
untested; `a1`'s outputs are otherwise those of the final binary (the same
eight declarations moved, roll-up 418 / 92 / 22 / 349 on the old base).

## 3. AFTER and the identity proof

Binary `private/dabc/bin/a2` `7b8460cc…` (= `run.json`), built at
`22980136a`, every route exit 0: corpus 446.8 s, system 61.6 s, slice 28.7 s
(BEFORE 449.5 / 62.5 / 27.3 s). [`scripts/identity.sh`](scripts/identity.sh)
runs [`verdict_projection.py`](scripts/verdict_projection.py) (removes
exactly the unit key `contract_body_evidence` and the sidecar's
`contract_dependencies` section), `parity.py` and
[`clause_rollup.py`](scripts/clause_rollup.py) on each route
([`artifacts/`](artifacts/)):

| route | verdict rows (BEFORE = AFTER) | report projection | sidecar projection | parity multiset | unit-field moves |
| --- | --- | --- | --- | --- | --- |
| corpus | `97cbdcf3…` | `cd4187bb…` | `a29d90f4…` | identical (662,667 rows) | 8: `flb_free`, `mk_mem_free`, `rd_calloc`, `rd_free`, `rd_kafka_mem_free`, `rd_malloc`, `rd_strdup`, `rd_strndup` |
| system | `09b7a59e…` | `65ab8ab8…` | `4574217a…` | identical (139,962) | 1: `flb_free` |
| slice | `ce1be80e…` | `a48fe10c…` | `4b6f8183…` | identical (50,407) | 1: `flb_free` |

No differing path after projection; every unit-field move is a `conflict`
failure replaced by `13m` evidence. Totals unchanged: 881 / 9 / 3,944,
87 / 4 / 357, 45 / 1 / 354 / 4 Pending. Report bytes added: corpus 47,540
(+0.4%), system and slice 2,928. **No verdict moved; no Safe → non-Safe.**
The parity streams are not identical in order between two `--jobs 4` runs
(as before); as multisets they are identical, so the muted check added no
row and no inference changed.

`mk_vhost_set` tip probe ([`scripts/tip-probe.sh`](scripts/tip-probe.sh),
[`tip_compare.py`](scripts/tip_compare.py), on the first base): verdict rows
identical on the tip (`a35c40cf…`) and the master copy (`7bf0efa2…`);
`mk_vhost_set` Unknown (tip 1 leaf, master 6), `mk_vhost_set_property`
Unknown (2 leaves), dependencies unchanged.

## 4. The new roll-up

| route | Safe | none | only validated | partly | declared assumption | declarations (validated / partly / declared / held) |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| corpus BEFORE (`before2`) | 881 | 418 | 63 | 23 | 377 | 330 (46 / 31 / 252 / 1) |
| corpus AFTER | 881 | 418 | **91** | 23 | **349** | 330 (**51** / 31 / **247** / 1) |
| system BEFORE → AFTER | 87 | 47 | 1 → **2** | 4 | 35 → **34** | 99 (7 → 8 / 6 / 84 → 83 / 2) |
| slice BEFORE → AFTER | 45 | 29 | 0 → **2** | 2 | 14 → **12** | 83 (4 → 5 / 1 / 69 → 68 / 9) |

**Moving declarations** (`declared-assumption` → `validated`, `13m` in every
examining unit): corpus `rd_free` (66 units, 23 Safe rows), `mk_mem_free`
(26, 2), `rd_strndup` (13, 0), `rd_kafka_mem_free` (1, the row; 4),
`flb_free` (1, 0); system and slice `flb_free` (15 units; 1 and 3 Safe rows).
**Every moving Safe row, attributed:** corpus +28 into "only validated"
(`rd_free` 23, `rd_kafka_mem_free` 4, `mk_mem_free` 1; `mk_mem_free`'s other
row rests on another declared assumption); system +1 (`sb_destroy_chunk`,
`flb_free`); slice +2 (`flb_lib_free` in two units, `flb_free`). Every
"only validated" row of BEFORE is one AFTER on all routes; none fell.

**Covered but not moved.** `rd_malloc` (42 of 43 units `13m`), `rd_calloc`
(45 of 46) and `rd_strdup` (31 of 32) stay declared assumptions: in
`rdhttp.c` W4 publishes no summary of the same header text and the rule
reads only conflicts, so that unit examined the body with no evidence. The
unit's own rows carry four ``C extension types could not be excluded: the
definition's typed AST is unavailable (CG30)`` refusals; all three bodies
use `rd_assert` (glibc's `assert` statement expression under `gnu17`) and
`rd_free`, which does not, validates there. The cause is consistent with a
typed-AST reading the statement expression needs and was not isolated
further (4 Safe rows: `rd_malloc` 3, `rd_calloc` 1). `input_chunk_has_magic_bytes`
(W4 `Owned` against a declared `Borrowed` with `resource_user_neutral()`)
carries a counted-user clause, so it is never checked and keeps its failure.

## 5. Prediction against outcome

| prediction | outcome |
| --- | --- |
| no verdict moves; report projection identical; parity multisets identical | held on all three routes |
| corpus validated 48 → 53–58 | 48 → 53 on the first base (`a1`); 46 → 51 on the rebased base |
| `rd_free`, `rd_kafka_mem_free`, `mk_mem_free`, `flb_free`, `rd_strdup`, `rd_strndup` validated; `rd_malloc`/`rd_calloc` only if their evidence-less unit validates | `rd_strdup` wrong: it has the same evidence-less unit (`rdhttp.c`) as `rd_malloc` and `rd_calloc`, none of which validated |
| declared assumptions 252 → 244–247 | 247 |
| only validated 64 → 87–95; declared assumption 377 → 346–354 | 64 → 92 and 377 → 349 (first base); 63 → 91 and 377 → 349 (rebased) |
| partly 22 / none 418 unchanged | held (23 on the rebased base, unchanged) |
| system 3–8 / slice 0–2 only validated | system 1 → 2 (the premise card moved the base from 3 to 1), slice 0 → 2 |
| the scope-only route (then "13m") validates `rd_malloc`/`rd_calloc` in their one unit | wrong: it established nothing anywhere; withdrawn (§2) |
| no structural declaration closes | held |

## 6. Lean

`proof/QCore/DeclaredEntryCheck.lean`, audited by
`proof/AuditDeclaredEntryCheck.lean` (11 theorems, no placeholder, no project
axiom): `new_evidence_needs_the_entry_check`, `new_evidence_needs_a_conflict`,
`a_container_or_counted_user_declaration_is_never_checked`,
`a_withdrawn_or_refuted_declaration_is_never_checked`,
`a_failed_check_keeps_the_conflict_a_failure`, `a_contradiction_still_fails`,
`without_a_passing_check_nothing_changes`, `only_a_conflict_changes`,
`the_new_evidence_covers_only_custody`,
`the_new_evidence_never_covers_a_clause_entry_does_not`, and the shared
vectors `recording_vectors_hold`, bound to `checked` and `recorded` by the
module's unit test. `QCore.BodyClauseStanding` gains the evidence `r13m`
(its `coverage_vectors_hold` row bound by
`crates/qcore-c-gateway/tests/contract_clause_standing.rs`; four
`clause_standing_vectors_hold` rows using `13m`, bound by
`crates/qcore-audit/tests/contract_dependencies_report.rs`, 19 rows).

Controls (`crates/qcore-c-gateway/tests/declared_entry_check.rs`, the
provider's libc texts byte for byte and a fixture module): the nullable
release wrapper, its two guarded forms, the same wrapper as a row (keeps its
`SummaryConflict`, non-`Safe`) and a result refinement after a no-return
check are `13m`; a nullable formal handed to a non-null release keeps its
`conflict` failure; keeping the formal, an uncontracted call, a release on
one path, a declared alias the entry check does not establish and a
counted-user declaration are never `13m`; recording changes no row and the
check consults nothing for another function. `contract_dependencies`'s unit
test checks `unrecorded` leaves no trace.

## 7. Gate

See [gate.json](gate.json) and [gate/](gate/) (section filled by the gate run).

## 8. Ledger

* `VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES` **amended**: rule 13m reads
  only W4 conflicts and none of its declarations has one; re-measured on
  `d8a1a2a4e` (sizes in the entry).
* `VERDICT-COUNTED-USER-CLAUSE-EVIDENCE` **amended**: no exact reading was
  found. The counted-user pass establishes a body's neutrality only relative
  to premise UC-FRAME (every declaration without a counted-user clause is
  read as neutral) and records no per-clause `held` for
  `resource_user_neutral()`; `input_chunk_has_magic_bytes`'s refinement
  conflict is added to it.
* Registered **`DECLARED-ASSUMPTION-BODY-CHECK-RESIDUALS`**: the ranking's
  remaining reasons with sizes (§1, AFTER), the `rdhttp.c` evidence-less unit
  of `rd_malloc`/`rd_calloc`/`rd_strdup`, and the named-premise declarations
  reported apart.

## Files

* Code: `crates/qcore-c-gateway/src/legacy_declared_entry_check.rs` (new),
  `crates/qcore-c-gateway/src/legacy_c.rs` (module and hook),
  `crates/qcore-c-gateway/src/contract_dependencies.rs`,
  `crates/qcore-c-gateway/src/contract_clause_standing.rs`,
  `crates/qcore-audit/src/contract_dependencies.rs` (`MEANING`).
* Tests: `crates/qcore-c-gateway/tests/declared_entry_check.rs` (new),
  `crates/qcore-audit/tests/contract_dependencies_report.rs`.
* Lean: `proof/QCore/DeclaredEntryCheck.lean`,
  `proof/AuditDeclaredEntryCheck.lean`, `proof/QCore/BodyClauseStanding.lean`,
  `proof/QCore.lean`.
* Specification: contract C06.5. Rules: gateway 13m, 13n, router rows.
* Scripts: [`scripts/`](scripts/). `providers/` unchanged: no contract edited,
  no re-pin.
