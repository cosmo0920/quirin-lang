# NONNULL-MEMBER-UNIFIED-QUALIFIER: one member clause, three kinds

Card NONNULL-MEMBER-UNIFIED-QUALIFIER (wave 48, round 31, card-high). Base:
local `main` `3f8f1a617` (FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE landed), rebased
onto `a423fca5d` (FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME's records-only landing)
and then onto `b157c9cba` (RETENTION-UNMATCHED-REMOVAL-POISONS: rule 12t,
`retained_by.rs`, libevent's `event_loop.qv` and pins in
`compiler/tests/{qcore_contract_corpus,contract_loader}.rs`); only the
increment record, the docs index and the ledger conflicted. The
[prediction](PREDICTION.md) was committed (`24d919b36` before the rebases)
before any loader, analyser, contract or Lean change. Hashes in sections 4
and 5 are pre-rebase: the binary `a1` was built at `6f0675d3f` and measured on
the contracts of `d62be618c`; the rebases moved none of this card's code or
contract bytes. Section 5's combined arbiter measures main `b157c9cba` against
the rebased tip.

## 1. Authority

**User decision 3 (2026-10-16, verbatim):** "As shown above", "Migrate, one
spelling", "Yes, all kinds".

* plain: `requires nonnull_member(TAG, FIELD);`
* stable: `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];`
* scope: `requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];`

`P` is required with `[[scope]]` and rejected otherwise. The old words
`nonnull_member_stable` / `_scope` are migrated and then rejected, with a
diagnostic naming the new form. No aliases. The plugin premise applies to all
three kinds by the same reachability rule.

**Ledger:** `LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED`. **Rules:** 14t, 14x,
14z; spec C04.18 and C09.3; grammar G05, G05e, G05f. The premise is librdkafka's
`rk_no_plugin_library` only: no Fluent Bit declaration gains one (out-of-tree
Fluent Bit plugins are OUTER-CONTRACT-DESIGN's).

## 2. Grammar and loader

`crates/qcore-contract/src/bounds_contract.rs` reads a clause headed
`nonnull_member` in `member_clause` before any other bounds clause; the
qualifier after a top-level `where` is exactly `quant #scope [[WORD]]`
(`member_qualifier`). The three kinds load to the three facts the retired
spellings loaded to (`BoundsClause::NonnullMember`, `NonnullMemberStable`,
`NonnullMemberScope`); every consumer downstream is unchanged.

| Input | Diagnostic |
| --- | --- |
| `nonnull_member_stable(…)`, in any position (a `where`, an `ensures`, a review marker) | `nonnull-member-retired-spelling: … write \`requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];\`` |
| `nonnull_member_scope(…)`, likewise | `nonnull-member-retired-spelling: … write \`requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];\`` |
| a third argument without `[[scope]]` (none, or `[[stable]]`) | `nonnull-member-parameter-needs-scope` |
| `[[scope]]` with two arguments | `nonnull-member-scope-needs-parameter` |
| `P` not a pointer parameter | `nonnull-member-scope-parameter` |
| `[[WORD]]`, `WORD` neither `stable` nor `scope` | `nonnull-member-qualifier-unknown` |
| any other qualifier shape (`where;`, `[stable]`, `#other`, a missing `quant` or `#`, two qualifiers) | `nonnull-member-qualifier-shape` |
| a parameter as TAG, a dotted FIELD, a wrong argument count | `nonnull-member-shape` |
| under `ensures` | `nonnull-member-requires` |
| a premise beside no member clause | `premise-beside-member` (now: beside a member clause of any kind) |

The canonical text renders `#`, `[` and `]` (`render`), so the digest covers
`nonnull_member(S, next) where quant #scope [[stable]]` and spacing does not
move it. Both retired words stay in `RESERVED_SPELLINGS`, so no other
vocabulary may take them. `contract_views::check_null_deref_only` admits every
kind through the `nonnull_member(` prefix; its old-word branch is gone.

**Premise for every kind.** The placement check admits `resource_premise`
beside a plain member too. `null_deref::member_fact_premises` keys a plain
member's premises by `member:TAG.FIELD`, the dependency its rows already name
(rule 14t), so `attach_premises` and the fold's travel (`add_dependencies`)
list it unchanged; the fold's `member_declarations` record gains `premises`.

**Evidence labels kept.** The null property's labels that name the fact kinds
(`nonnull_member_stable; …` in a kept fact's assumption, `nonnull_member_scope(TAG,
FIELD, P)` in a scope entry's precondition, scope and check subject) are kept
byte for byte, so no row moves; they are labels, not surface spellings
(commented at both sites). The fold's `scope_declarations` records carry each
entry as written, so they show the new spelling (section 5).

**Tests:** `crates/qcore-contract/tests/nonnull_member_qualifier.rs` (7: each
kind's fact, the one-spelling text and its digest, every rejection above, the
premise beside every kind); `null_member_premise.rs` gains a plain member read
on a real Clang AST, listing the premise through `member:R.link`, and its
placement test now admits the plain kind; `null_member_scope_fold` gains the
fold record and a travelled row through `member:T.owner`; every old-spelling
fixture is migrated.

## 3. Lean

`QCore.NullMemberQualifier` (`proof/QCore/NullMemberQualifier.lean`), audited
in `proof/AuditNullMemberQualifier.lean`; Lean audits 332 → **333**.

* `the_merged_surface_loads_the_old_fact`: each retired clause's migration loads to its fact;
* `every_admitted_clause_is_a_migrated_one`, `migration_is_injective`;
* `the_merged_surface_is_equivalent_to_the_old_forms`: a merged clause loads to a fact exactly when it is the migration of an old clause with that fact;
* `a_retired_word_is_rejected_naming_the_new_form`, `a_parameter_needs_the_scope_qualifier`, `the_scope_qualifier_needs_a_parameter`, `an_unknown_qualifier_is_rejected`;
* `a_premise_stands_beside_every_kind`, `a_premise_needs_a_member_fact`;
* `qualifier_vectors_hold` (the Rust test's examples).

`QCore.NullMemberPremise` already proves listing over abstract dependency keys,
so `member:TAG.FIELD` needs no new theorem there.

## 4. Migration and premises

[`write_contracts.py`](scripts/write_contracts.py) (`--check` 0):

| Crate | Declarations migrated | Premise added |
| --- | --- | --- |
| `qcore-provider-librdkafka` (`null_deref.qv`) | 67 (1 `[[stable]]`, 66 `[[scope]]`) | 4 declarations, 5 plain clauses |
| `quirin-provider-fluent-bit` (`null_deref.qv`) | 14 (4 `[[stable]]`, 10 `[[scope]]`) | none |

No other tracked contract module used either word (the check scans every
tracked `.qv` under `providers/`). Comments in both modules use the new
spelling.

**The reachability rule, reproduced.** [`census.sh`](scripts/census.sh)
re-ran LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's `member_reach.py` (unchanged but
for the view it reads, the base's `null_deref.qv`) over the 94 preprocessed
librdkafka units at 66910c10 and 046accf6 (0 nonzero exits).
[Its census](artifacts/member-census-reproduced.json) equals the committed one
in every row but `premised_before` (the 67 were premised on this base): all 72
clauses identical at both revisions; the five plain clauses have rule (a) and
rule (b) `rk-conf`. So the four declarations name the premise:
`rd_kafka_toppar_new0` (`rktp_rkt`), `rd_kafka_topic_new0` (`rkt_rk`,
`rkt_topic`), `rd_kafka_cgrp_new` (`rkcg_rk`), `rd_kafka_transport_new`
(`rktrans_rkb`).

**Pins** ([`pin.sh`](scripts/pin.sh) with binary `a1`, logs
`private/nmuq/pin-1/`): the 71 librdkafka declarations and the 14 Fluent Bit
declarations were re-pinned with `pin --bounds` at both revisions (exit 0
each); each crate's bounds check and the whole-tree bounds check are 0 at both;
`--verify-contracts` 0. The whole-tree resource check is 1 at both, for the
same entries the base binary reports on the base's resource locks
([`whole_tree_check.sh`](scripts/whole_tree_check.sh), `private/nmuq/pin-base/`):
libevent's tip-mode `mk_vhost_set` and the Fluent Bit teardown declarations
reviewed at the tail-pause export (`flb_tail_config_destroy`,
`flb_tail_file_remove`; `flb_input_instance_destroy` at 66910c10). This card
changes no resource lock. `contracts.py sync` updated librdkafka's
`contracts.json`; `check` 0.

**Clause-count pins** (`compiler/tests/qcore_contract_corpus.rs`): the four
plain declarations rise by one (2 → 3, 3 → 4, 2 → 3, 2 → 3); the migrated
declarations keep their counts; the premise census test now counts 71 premised
`null_deref.qv` declarations, names the four, and finds none in Fluent Bit's.

## 5. Measurement

BEFORE: binary `before` (`6449fae6…`) built from the unchanged base tree and
measured on the base's contracts. AFTER: binary `a1` (`1394b4c9…`), built at
`6f0675d3f`, measured on the migrated contracts (`d62be618c`). Each route
under the shared lock with the null property; `run.json`'s `binary_sha256` is
the measured binary on both corpus runs. Wall times (inside the lock, not a
performance claim): corpus 936 → 970 s, system 223 → 228 s, slice 173 → 173 s.
[Totals](artifacts/), [comparison](artifacts/compare.json),
[breakdown](artifacts/breakdown.json) ([`summarise.sh`](scripts/summarise.sh),
[`compare.py`](scripts/compare.py), [`breakdown.py`](scripts/breakdown.py)).

| Route | Custody | Custody report | Null property |
| --- | --- | --- | --- |
| corpus | 939 / 9 / 3,886 → same | `f2eb590a…` → **byte-identical** | 1,114 / 1,435 / 2,354 / 18 Unsafe → same |
| system | 95 / 4 / 349 → same | `210b42f1…` → **byte-identical** | 86 / 125 / 243 → same |
| slice | 52 / 1 / 347 / 4 Pending → same | `a89fa7e0…` → **byte-identical** | 69 / 121 / 216 → same |

**No verdict changed; no correction.** Every AFTER null row on all three routes
(4,921, 454, 406) equals its BEFORE row once its `premises` field, its
`premise:` dependencies and its identity's bounds-environment digest are
removed. The digest moves on every row, since `null_deref.qv`'s text changed.

**Rows that list the premise through a plain member (corpus).** 404 rows'
dependency sets name one of the five members (`rktp_rkt` 210, `rkt_topic` 199,
`rkt_rk` 171, `rkcg_rk` 106, `rktrans_rkb` 51), in 25 units (`rdkafka_cgrp.c`
99, `rdkafka_partition.c` 65, `rdkafka_topic.c` 28, `rdkafka_offset.c` 27).
Every one lists `rk_no_plugin_library` with those members in `through`, and
the dependency `premise:rk_no_plugin_library`:

* **272 gain it:** 87 Safe and 185 Unknown (null verdict);
* **132 already listed it** through `stable:rd_kafka_broker_s.rkb_rk` or a
  scope function: 30 Safe, 100 Unknown and the 2 provisional Unsafe
  (`rd_kafka_share_fetch_reply_op`, `rd_kafka_toppar_op_serve`, findings
  already counted, never upstream defects); their `through` lists grow.

No other row's premises changed. System and slice analyse no librdkafka unit:
0 rows there.

**Fold.** The five members' `member_declarations` records list the premise.
The null summaries are otherwise equal, except that the 76 `scope_declarations`
records (librdkafka 66, Fluent Bit 10) carry each entry as written, now in the
new spelling: a deviation from the prediction, which said every other fold
record would be unchanged. It is the declaration text moving, not a verdict
or a check; compared with the entries read back in the retired spelling, the
summaries are equal on all three routes.

**Prediction against outcome.** As frozen except the respelled fold entries
above; the custody reports did not move at all.

### Combined arbiter after the rebase onto `b157c9cba`

RETENTION-UNMATCHED-REMOVAL-POISONS landed meanwhile (rule 12t,
`retained_by.rs`, libevent's `event_loop.qv` and its pins, the compiler's pin
tests), so main `b157c9cba` (binary `main`, `e35cfc0c…`, built from an
exported tree that reads main's own contracts) was measured against the
rebased tip (binary `a2`, `8edc97de…`, built at `4bdfc1353`) on every route
with the null property, under the shared lock
([`combined.sh`](scripts/combined.sh); [totals](artifacts/),
[comparison](artifacts/compare-combined.json)).

| Route | Custody main → tip | Custody report | Null property |
| --- | --- | --- | --- |
| corpus | 939 / 9 / 3,886 → same | `33e139c0…` → **byte-identical** | 1,114 / 1,435 / 2,354 / 18 Unsafe → same |
| system | 95 / 4 / 349 → same | `9dd0fb3e…` → `704f00a2…`: the export's tree root and the contract-source identities only | 86 / 125 / 243 → same |
| slice | 52 / 1 / 347 / 4 Pending → same | `d77c87fb…` → `73a94bce…`: likewise | 69 / 121 / 216 → same |

* With the export's root (`private/nmuq/main-src`) replaced by this
  worktree's and `resource_contract_sources` / `inventory_sha256` removed
  (they hash each contract's path and text), the system and slice reports are
  equal.
* **Null rows** are identical apart from the premise fields and the bounds
  digest on all three routes; the same 404 corpus rows rest on the five plain
  members and all list the premise (272 gain it); no other row's premises
  change; the 76 scope records show the new spelling.
* **No verdict changed.**

The first main corpus run stopped at the libevent source pin: the export's
`third_party/libevent` was a plain copy, so `git rev-parse` there answered this
worktree's HEAD. With that directory made a local clone at the pinned
`d82464a2`, the corpus route was rerun
([`combined_corpus_retry.sh`](scripts/combined_corpus_retry.sh)); the first
attempt is kept in `private/nmuq/attempt1/` and discarded. At the tip, `pin
--bounds --check` is 0 for both crates and the whole tree at both revisions,
`--verify-contracts` 0, and the whole-tree resource check reports the same
not-in-force set as before (`private/nmuq/pin-check-a2/`).

## 6. Ledger

[`update_ledger.py`](scripts/update_ledger.py): 790 → 789 on the base; on
main `b157c9cba` (two cards' records landed since, 794) 794 → **793**
(`dimensions.named_followups` equal). Retired:
`LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED`. Its DISCHARGE holds: the user
decided the plain form admits the premise ("Yes, all kinds"); the five clauses'
four declarations name it, re-pinned, measured, and every null row resting on
`member:TAG.FIELD` lists it.

## 7. Rules

Amended in place, no new id: **14t** (the plain kind; may name a premise,
listed under `member:TAG.FIELD` and on the fold record; librdkafka's four),
**14x** (the `[[stable]]` spelling; the premised-facts paragraph now covers
every kind), **14z** (the `[[scope]]` spelling; `P` only with it; labels kept),
and their router rows. Spec C04.18 ("Bounds contract v1": any kind) and C09.3
("Premised member facts", and a new "One clause, three kinds" paragraph with
the decision verbatim); grammar G05 (placement), G05e (pointer) and G05f
(rewritten: `member_clause`, `member_qualifier`, every diagnostic);
`contract/README.md`'s clause-ownership row.
