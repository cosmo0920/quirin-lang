# SUP07-REFUSED-TEST-ACCUSATION: a leak claim does not rest on a refused test

Wave 48 supplemental card (round 13, High), 2026-10-02. Started on local
`main` at `cffe232d4`; rebased onto `f50fa2009` when `main` moved during the
card (below); branch `sup07-refused-test-accusation`. Prediction frozen
before any implementation code: [PREDICTION.md](PREDICTION.md) (`88d79e61c`,
after the census `11f690879`); after the rebase,
[PREDICTION-REBASE.md](PREDICTION-REBASE.md) (`2fe4878a7`) before the code it
predicts. Gateway rule [12e](../../agents/gateway.md#constraint-12e).

## The accusation, reproduced

MONKEY-VHOST-PROPERTY-CUSTODY's `mk_string_dup` contract made Monkey's
`mk_http_error_page` a provisional `Unsafe`: a leak at the `return -1` of
`mk_server/mk_http.c:399` of the copy made at :395. Reproduced on BEFORE (the
arbiter, and the control `monkey_mk_http_error_page_is_unknown_without_a_leak`
on the pinned body): the :392 refusal (`indirect memory access`, the
`*message` read) poisons `temp`; the else arm binds it to the nullable copy;
at the join `temp` is the copy plus an unknown value; rule 10b refuses
`if (!temp)` (`UnknownAlias temp`) and keeps the copy `LIVE` on the null
edge; `report_leaks` accuses it at :399. By reading the pinned source
(`mk_http.c:383-411`, `mk_core/mk_memory.c:51`) the accusation is false: on
that edge `mk_ptr_to_buf` returned NULL (no copy) or `mk_string_dup` did (no
obligation).

## Census

A leak-witness census (the off-by-default `indirect-census` build; its
diagnostics byte-identical to the plain BEFORE on all three routes) searches,
for every leak, a producer → exit path that avoids each null edge of a
refused test whose binding may denote the site, and one that avoids only the
edges where joined provenance also ties the site to the tested name
([classes](artifacts/census/witness-corpus-before.json)):

- **Corpus:** of 102 witnesses (42 leaves over W4 invocations), 9 rest on a
  refused test on every path: `mk_http_error_page` (3, tied) and
  `mk_sched_add_connection` (6, the :201 and :205 allocations at the :210 exit
  of `if (!conn)`, not tied); 86 avoid every such edge, 7 have no settled
  path. 133 refused-test null edges denote a site, in 46 functions; 21 are
  tied, in 7 functions (`mk_http_error_page` and six `Unknown` librdkafka
  functions whose tied sites carry no leaf).
- **System, slice:** no leak witness; no tied edge.
- **Other violations** (summary conflicts, counted-user refutations) sit in
  functions with no refused test that denotes a site.

`mk_sched_add_connection` is also false by reading
(`mk_server/mk_scheduler.c:197-210`): on the null edge of `if (!conn)` the
allocation `conn` holds is NULL or was never made, `conn` reaching the :201
one only through `conn = (struct mk_sched_conn *) data`. Its tie was lost
only because `frame_protected_locals` reads `&conn->event` as taking
`conn`'s address.

## Rule 12e

`crates/qcore-c-gateway/src/legacy_refused_test.rs` (new), one hook in the
custody work list's `PointerRefinement::Unknown` arm, mirrored in the census.
On the null (invalid) edge of a refused null (validity) test, each site the
tested binding may denote and joined provenance ties to the tested name is
**settled**: removed exactly as a decided test's null edge removes its site
(`remove_null_site`, factored out of `refine_pointer_null` byte for byte).
The tie says the name denotes the site on every path on which the site
exists, so on that edge it is NULL or absent. The `UnknownAlias` and the
edge's `unmodeled` stay: the rule never makes a function `Safe`.

The provenance is joined provenance over the **tie names**
(`LegacyCfg::tie_names`): the conditional names plus each unique local whose
only `&` uses are member addresses through it (`&NAME->F`, `&(NAME)->F`,
read as C11 6.5.2.3/6.5.3.2 read them: the member of `*NAME`, never `NAME`'s
own slot). It is derived a second time only where the tie names differ; rule
10b's conditional release and every frame keep the conditional names.

Two shapes are why the rule is this narrow, each a control:

- **Without the tie** the site may be live under no name on the null edge:
  the copy is made, a refused statement overwrites the tested name on one
  path, and the leak is established
  (`a_copy_lost_before_the_refused_test_stays_accused`).
- **A marker instead of the settlement** joins by union: a copy leaked on the
  test's non-null edge that meets the null edge before the exit would be
  withdrawn too (`a_leak_on_the_non_null_edge_stays_accused_across_the_join`).

## `main` moved: the interim withhold rule

During the card `main` moved from `cffe232d4` to `f50fa2009`, merging
`7493e76a2` (MONKEY-VHOST-PROPERTY-CUSTODY, also numbered 12e, no gateway
text, ledger or report entry): on the null edge of a refused null test every
nullable, possibly-live site the binding may denote was **withheld** (an
unresolved effect) in `refine_pointer_null`. On the rebased tree it fails both
controls above: it removes two established accusations, which the brief
forbids. It is right on the corpus only because no corpus function has those
shapes. This card **replaces** it: the hook, `refused_null_test.rs`, its unit
test and `QCore.RefusedNullTest` / `AuditRefusedNullTest.lean` are removed;
its three controls in `monkey_vhost_property_custody.rs` stay and pass.
Parity before deletion: the corpus, system and slice with rule 12e equal
`main` with the withhold rule byte for byte (`dde4837e…`, `09423cd7…`,
`86810e4d…`).

## Lean

`QCore.RefusedTestAccusation`, 18 theorems in the new
`proof/AuditRefusedTestAccusation.lean` (main's 281 audits − 1 + 1 = 281):
under the tie no execution on the null edge owes the site
(`tied_null_edge_owes_nothing`), and without it one may
(`without_the_tie_a_null_edge_may_owe`), so the refused test alone
establishes nothing; `mk_http_error_page`'s executions are tied; `settle_iff`
and each kept case; `edge_vectors_hold`, bound to the Rust `EDGE_VECTORS` by
`tests/unit/legacy_refused_test.rs`; no claim on a settled edge, matching the
executions (`the_rule_matches_the_executions`), and the old claim was made
while nothing was owed (`the_old_claim_was_false`); an established claim
survives the join (`an_established_claim_survives_the_join`) where a marker
would withdraw it (`a_marker_would_withdraw_it`); a refused test is never
`Safe` (`a_refused_test_is_never_safe`), the control is `Unknown`, an
established leak beside it `Unsafe`. No placeholders, no project axioms.

## Tests

`crates/qcore-c-gateway/tests/sup07_refused_test_accusation.rs`, 7 controls
on the production `monkey/string.qv`: the :392/:395/:398/:399 shape and the
pinned `mk_http_error_page` body are `Unknown` with no leak and the named
`UnknownAlias temp`; the `mk_sched_add_connection` shape likewise
(`UnknownAlias conn`); a decided test is `Safe`; a second copy leaked beside
the test, a copy lost before it and a copy leaked on the non-null edge across
the join each stay `Unsafe` with their leak. The three that fail on BEFORE
are the three `Unknown` shapes (the scheduler control was added after the
rebase). Unit tests: the edge vectors against the Lean table, the absent
edge, the tie lookup and the member-address reader (the tokenizer spells
`->` as `-` `>`).

## Measurements

Corpus arbiter (`66910c10`, `--jobs 4`), Fluent Bit system route and
tail-pause slice ([scripts/measure.sh](scripts/measure.sh); digests in
[artifacts/digests.json](artifacts/digests.json), change lists and censuses
in [artifacts/](artifacts/)):

| Run | Tree | Corpus | System | Slice |
|---|---|---|---|---|
| BEFORE | `cffe232d4` | 647 / 11 / 4,176, leak 42, `a7d68d49…` | 83 / 4 / 361, `09423cd7…` | 45 / 1 / 354 / 4 Pending, `86810e4d…` |
| AFTER (PREDICTION.md) | `0a7b99cf2` on `cffe232d4` | **647 / 10 / 4,177**, leak 41, `25c1820b…` | identical | identical |
| BEFORE2 (`main`, withhold rule) | `f50fa2009` | 647 / 9 / 4,178, leak 39, `dde4837e…` | identical | identical |
| **AFTER2** | `cf46c1af5` on `f50fa2009` | **647 / 9 / 4,178**, leak 39, `dde4837e…` | identical | identical |

The system file differs from MONKEY-VHOST-PROPERTY-CUSTODY's AFTER
(`21b08170…`) only in identity fields (worktree paths and contract-source
digests); every status and leaf is equal.

**Corpus attribution** (AFTER2 against BEFORE, the only two functions that
change; [list](artifacts/changed-before-after2.json)):

- **`mk_http_error_page` provisional Unsafe → Unknown**, leak −1 (:395 at
  :399): a correction of a false accusation (above). Its other leaves stand.
- **`mk_sched_add_connection` provisional Unsafe → Unknown**, leak −2 (:201
  and :205 at :210): a correction of a false accusation (above), reached by
  the tie names.

No Safe → non-Safe move, no leak or violation added, no other leaf moves.

**The probe** (`mk_vhost_set` at Monkey's fix tip `21a2c190` and at master,
`mk_vhost_set_property`; [scripts/probe.sh](scripts/probe.sh), rows in
[artifacts/](artifacts/)): unchanged at every run. At the tip `mk_vhost_set`
is `Unknown` on `call to uncontracted function mk_vhost_set_property`
(:533) only, and `mk_vhost_set_property` on the DocumentRoot store (:502,
:503); at master `mk_vhost_set` keeps its five `va_end` refusals and :532.
No refused test is involved, so the shared goal does not move here.

## Prediction against outcome

PREDICTION.md (base `cffe232d4`): 647 / 10 / 4,177, only
`mk_http_error_page`, `mk_sched_add_connection` stays a provisional `Unsafe`,
the six librdkafka functions unchanged, system and slice byte-identical, the
probe unchanged. **Held exactly** (`25c1820b…`).

PREDICTION-REBASE.md (base `f50fa2009`): 647 / 9 / 4,178 equal to `main` in
every status and leaf, `mk_sched_add_connection` Unknown through the tie
names, no other move, system and slice identical, probe unchanged, Lean 281.
**Held exactly** (`dde4837e…`, byte-identical to `main`). The departure from
the first prediction (`mk_sched_add_connection`) is the rebase's: the withhold
rule on `main` had already moved it, and replacing that rule without the tie
names would have re-published a false accusation.

## Follow-ups

Retired: `MONKEY-REFUSED-NULL-TEST-JOINED-LEAK` (its DISCHARGE holds: the
correlation of a nullable site with its binding across the join, measured,
every withdrawn leak attributed, Lean parity). Amended:
`LEAK-UNDECIDED-NULL-TEST` (its Monkey half discharged; its
`rd_kafka_snappy_java_uncompress` half stays open and is corrected: the
`snappy.c:1712` leaf is still published, on a test inside a refused
`__builtin_expect` call that rule 12e does not reach). 696 → 695. Edits by
[scripts/update_ledger.py](scripts/update_ledger.py).

## Gate

See [gate.json](gate.json) and [gate/](gate/).
