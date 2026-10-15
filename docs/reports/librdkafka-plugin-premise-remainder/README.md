# LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the plugin premise on member facts, the crash path and a model contract

Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (wave 48, round 28, card-high).
Base: local `main` `711287771`. The member census and the frozen
[prediction](PREDICTION.md) were committed (`7472690aa`, `5f75f69c0` after the rebase onto `635669fdb`) before any analyser,
loader, contract or Lean change.

## 1. Authority

**User decisions (2026-10-15, verbatim):**

1. **Null views: "Only nonnull_member* need it".**
   * A `quant null-deref` fact ("this function dereferences B") does not depend on which callback runs, so it needs no premise.
   * Only `nonnull_member_stable` / `nonnull_member_scope` declarations must carry it, through a premise clause added to just those forms.
   * A stray callback store could falsify them.
2. **Crash path: "Yes, list it".**
   * `rd_kafka_crash` (`memory.qv`) and `rd_kafka_dump0` (`kafka_mem.qv`) name the premise.
   * `rd_kafka_toppar_get_avail` keeps it.
   * No verdict should change; the dependent Safe rows gain the premise.
3. **Model contract: "Add premise support".** Extend the Quirin model-contract format so that `queue_destructor.qv`'s `rd_kafka_q_destroy_final` can state the premise. State it there.

**Ledger entries:** `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED` (amended) and `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED`, with their DISCHARGEs.

**Rules and invariant:** rules 14x and 14z, the premise clause of C04.18 and hard invariant 2.

No new surface word: the existing `resource_premise(NAME, "TEXT")` spelling is reused everywhere.

## 2. The member census and its rule

[PREDICTION.md](PREDICTION.md) section 2 states the rule and
[`member_reach.py`](scripts/member_reach.py) applies it
([census](artifacts/member-census.json)). The script imports
LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED's `reach.py` graph unchanged and runs over
the 94 preprocessed librdkafka units at `66910c10` and `046accf6`
([`census.sh`](scripts/census.sh); 0 nonzero exits at both). All 72 member
clauses give identical results at both revisions.

**Rule.** A member declaration names `rk_no_plugin_library` when a callback
outside the run could falsify its fact. That holds in either of two cases:

* **(a)** A function that may store into `TAG.FIELD` can run inside a
  plugin-registered callback. That is, it is reachable from a function with
  external linkage, which such a callback may call.
* **(b)** The declared body itself reaches an rk_conf or interceptor call, or
  an unresolved indirect call.

The run's fold checks every analysed store whatever context runs it. What no
check covers is a store made by code outside the run. librdkafka runs its
callbacks on its own threads, and a stable fact persists run-wide (rule
14y), so (a) decides.

**Result.** `rd_kafka_broker_s.rkb_rk` has 14 functions that may store into it
(2 by assignment, 12 by a possible whole-object write).
`rd_kafka_broker_add` is reached from the public `rd_kafka_new`, so (a) holds
for the member.

| Form | Declarations | Premise | (b) `rk-conf` / `indirect` / `none` |
| --- | ---: | --- | --- |
| `nonnull_member_stable` (`rd_kafka_broker_add`) | 1 | yes | 1 / 0 / 0 |
| `nonnull_member_scope` (`rkb_rk` of `rkb`) | 66 | yes | 62 / 2 / 2 |
| rule 14t `nonnull_member` (for the record) | 4 (5 clauses) | not admitted (decision 1) | 5 / 0 / 0 |

So **67 declarations** carry the premise. No other crate declares a scoped
member form on this base.

## 3. Changes

**Format: the premise in every module.** `crates/qcore-frontend/src/frontend.rs` (`premise_clause`) checks the clause's shape in every contract block as it parses it: resource, bounds or model. A clause headed `resource_premise` must be exactly a `requires resource_premise(NAME, "STATEMENT")`, with:

* `NAME` matching `[a-z][a-z0-9_]*`;
* one plain string;
* no name repeated in a declaration.

Anything else rejects the module. This is the model-contract extension (total-or-reject). `contract_loader::module_premises` reads a model contract's premises from the format alone. Placement stays each loader's rule.

**Bounds v1.** `BoundsClause::Premise` is admitted only in a declaration that also states `nonnull_member_stable` or `nonnull_member_scope`. Anywhere else it is refused with `premise-beside-member`: alone, or beside `quant null-deref`, rule 14t's `nonnull_member`, or a bounds clause. Further details:

* it is part of the declaration's text, digest and pin;
* `for_bounds` removes it;
* `contract_views::check_null_deref_only` admits it beside a resource view, which `rd_kafka_toppar_fetch_backoff` needs.

**Null property.**

* `null_deref::member_fact_premises` keys each premised fact by the dependency a row lists it under: `stable:TAG.FIELD`, or the scope declaration's function name.
* `attach_premises` gives every row whose dependency set names one a `premises` field (`name`, `statement`, `through`) and the dependency `premise:NAME`.
* The fold's `add_dependencies` re-lists them after travel grows a set.
* The fold's `stable_declarations` / `scope_declarations` records gain `premises`.
* No decision reads any of it.

**Contracts** ([`write_contracts.py`](scripts/write_contracts.py), `--check` 0):

* The premise clause is copied byte for byte from `client.qv`'s 89 declarations to:
  * the 67 `null_deref.qv` declarations, after their member clause;
  * `rd_kafka_crash` and `rd_kafka_dump0`, after the selector;
  * `rd_kafka_q_destroy_final`, the model contract, as its first clause.
* Each module gets one comment block.
* **Pins** ([`pin.sh`](scripts/pin.sh), logs `private/lppr/pin-d1/`):
  * the 67 are re-pinned in `bounds-contracts.lock`, and the two in `contracts.lock`, at both revisions;
  * the crate's resource and bounds checks are 0 at both, and `--verify-contracts` is 0;
  * the whole-tree check is 1 at both, only for libevent's tip-mode `mk_vhost_set`, by design.
* `contracts.py sync` and `check` are 0.
* **Clause-count pins.** In `compiler/tests/qcore_contract_corpus.rs` ([`edit_pins.py`](scripts/edit_pins.py)), 70 rows rise by one (`rd_kafka_q_destroy_final` 6 → 7). A new test reads every corpus module's premises and pins the model contract's.

**Tests:**

* `crates/qcore-c-gateway/tests/null_member_premise.rs`, on a real Clang AST:
  * rows resting through `stable:`, a scope body and a caller list the premise;
  * a `quant null-deref` row does not;
  * the rows are equal with and without the premise once its fields are removed;
  * the placement and malformation rejections.
* `crates/qcore-contract/tests/contract_premise.rs`: a model contract's premise read, and eight malformations rejected at parse.
* `null_member_scope_fold`: the fold's record and a travelled row list it.

## 4. Lean

`QCore.NullMemberPremise` is audited in `proof/AuditNullMemberPremise.lean`; Lean audits go 324 → 325 on the base, 325 → 326 on the rebased tip (after MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS). Its theorems:

* `a_resting_row_lists_the_premise`;
* `a_listed_premise_is_a_dependency`;
* `a_listed_premise_comes_from_a_named_fact`;
* `a_row_on_no_premised_fact_lists_none`;
* `listed_premises_put_every_fact_in_force`: a member fact a row rests on is in force under exactly the premises the row lists;
* `listing_changes_no_verdict`;
* `more_dependencies_never_drop_a_premise`;
* `premise_vectors_hold`.

## 5. Measurement

**BEFORE.** Binary `bin/before` (`352276f2…`), built from an exported copy of `711287771` (`private/lppr/base-src`, [`before.sh`](scripts/before.sh)). Its contracts, profile and scripts are the base's (`REPO` set to that tree on every route).

**AFTER.** Binary `a1` (`b11509e5…`), built from `459fb3083` (the contracts commit; `530c8d18a` after the rebase).

Wall times were measured inside the shared lock and are not a performance claim.

A first BEFORE attempt (`before`) is retained but discarded:

* the corpus route stopped at the libevent source pin (the export had an empty submodule directory);
* the system route read this worktree's changed contracts.

Summary: [summary.json](artifacts/summary.json).

| Route | Custody before → after | Report | Null property | Wall before → after |
| --- | --- | --- | --- | ---: |
| corpus | 936 / 9 / 3,889 → same | `82214412…` → **`82214412…`** (byte-identical) | 1,113 / 1,435 / 2,355 / 18 Unsafe → same | 670 → 674 s |
| system | 93 / 4 / 351 → same | `c6db4f64…` → `f67a8069…` | 84 / 125 / 245 → same | 80 → 80 s |
| slice | 52 / 1 / 347 / 4 Pending → same | `2c74f61f…` → `5170f458…` | 71 / 103 / 232 → same | 40 → 40 s |

`run.json`'s `binary_sha256` is the measured binary on both corpus runs.

**Verdicts.** No verdict changed and none is corrected ([`premise_rows.py`](scripts/premise_rows.py), [rows](artifacts/rows.json)).

* **Null rows.** Every AFTER null row on all three routes equals its BEFORE row once three things are removed:
  * its `premises` field;
  * its `premise:` dependencies;
  * its identity's bounds-environment digest, which every row carries and which `null_deref.qv`'s new text moves.

  That covers every verdict, reason, check, scope and other dependency. The null summaries are equal once the fold records' `premises` are removed.
* **Custody report.** The corpus report is byte-identical. The system and slice reports differ only in each unit's contract-source identities (`resource_contract_sources`, `inventory_sha256`) ([system](artifacts/report-diff-system.txt), [slice](artifacts/report-diff-slice.txt)).
* **Custody dependencies.** Every row's `contract_dependencies` field is identical, and so are the sidecar's counts and Safe roll-up.
* **Corpus sidecar.** Its records differ only in the two re-pinned declarations' digests and premises, the module and lock digests, and the module sources ([diff](artifacts/sidecar-diff-corpus.txt), BEFORE's tree root normalised).

**Null rows that list the premise (corpus).** 339 rows rest on a premised member fact: by BEFORE dependency, `stable:rd_kafka_broker_s.rkb_rk` or one of the 66 scope functions.

* **Every one lists `rk_no_plugin_library`:**
  * 75 Safe (null verdict);
  * 262 Unknown;
  * 2 provisional Unsafe (`rd_kafka_share_fetch_reply_op`, `rd_kafka_toppar_op_serve`, findings already counted, never an upstream defect).
* No other row lists it.
* By file: `rdkafka_broker.c` 67, `rdkafka_request.c` 62, `rdkafka_partition.c` 41, `rdkafka_fetcher.c` 29, the rest in 21 more units (25 in all).
* The fold lists the premise on all 67 records.
* System and slice analyse no librdkafka unit: 0 rows rest on the facts there.

**Crash path (custody, corpus).** 69 rows rest on `rd_kafka_crash` or `rd_kafka_dump0`.

* BEFORE: 2 Safe lacking the premise, 43 Unknown lacking it, 24 Unknown listing it through another declaration.
* AFTER: **all 69 list it**, so **2 Safe and 43 Unknown rows gain it**. The Safe ones are the bodies `rd_kafka_crash` and `rd_kafka_dump`.
* No Safe or Pending row resting on them lacks it.
* `rd_kafka_toppar_get_avail` keeps its premise.

**Model contract.** No verdict consults `queue_destructor.qv`. Its premise is read by `module_premises` and pinned by the corpus tests.

**Parity streams.** The typed-path parity streams are equal as multisets on all three routes once Clang's AST node addresses (`0x…`, which change between any two runs) and BEFORE's tree root are normalised: corpus 684,906 lines, system 142,456, slice 51,389.

**Rebase.** The runs were taken on `711287771`. The card was then rebased onto `635669fdb`, where MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS landed. That card changes rule 30's member-path body check in `member_path_check.rs` and `member_path_reading.rs`, and its own measurement moved only two units' body evidence. It touches no null-property, contract-format, bounds or librdkafka contract file, so the runs stand for the rebased tip. The gate below ran on the rebased tip.

**Prediction against outcome.** As frozen: verdicts identical, and only the premise and dependency fields, the fold records, the two sidecar records and the contract-source and pin digests moved. The corpus custody report did not move at all; the prediction allowed contract-source digests there.

## 6. Ledger

[`update_ledger.py`](scripts/update_ledger.py): 784 → **783**.

* **Retired: `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`.** The user decided:
  * the null views are independent of the callback;
  * the scoped member forms carry the premise, and all 67 do;
  * the model contract states it.

  It was measured.
* **Retired: `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED`.** The user decided; both declarations name the premise, re-pinned and measured; `rd_kafka_toppar_get_avail` follows.
* **Registered: `LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED`.** Rule 14t's five plain `nonnull_member` clauses (4 declarations) meet the census rule. The decision as carried admits the premise only beside the scoped forms. The user decides.

## 7. Rules

Amended in place; no new id:

* **14x** gains a "Premised facts" paragraph;
* **14z** gains the scope premise;
* **13h** now covers the format check;
* the router rows for all three are amended;
* spec C04.18 gains "Every vocabulary", C09.3 gains "Premised member facts", and grammar G05 / G05f are updated;
* `contract/README.md`'s clause-ownership table is updated.

The bold rule-id duplicate check is empty.

## 8. Gate

See [gate.json](gate.json) and [gate/](gate/). Two earlier attempts are retained in `private/lppr/` and discarded:

* **g1** (`0f2f8c8e3`, before the rebase). The host disk filled while the test binaries were building ("No space left on device", then a linker bus error), and later steps could not write their exits. Per the measurement-hygiene rule, a disk-exhausted run measures disk, not the program.
* **g2** (same commit, disk recovered). One unit failed: `contract_inherit_qcore_provider`'s `converted_wrappers_elaborate_to_the_hand_written_contracts`. Its hand-written parity fixture `compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv` restates `memory.qv` token for token, and `rd_kafka_crash` had gained the premise. The fixture now carries the same clause (`936c664b5`); the test passes alone (8 passed). Every other g2 step was 0, except the Python suite before the matrix regeneration, which is expected. Its totals were 500 units, 8,103 passed, 1 failed, 15 ignored.

## Files

* **Code:**
  * `crates/qcore-frontend/src/frontend.rs`;
  * `crates/qcore-contract/src/{bounds_contract,contract_loader,contract_views}.rs`;
  * `crates/qcore-c-gateway/src/{null_deref.rs,null_deref/scope.rs,index_extent/body_evidence.rs}`;
  * `crates/qcore-audit/src/{null_fold,null_member_scope_fold}.rs`;
  * tests as above.
* **Lean:** `proof/QCore/NullMemberPremise.lean`, `proof/AuditNullMemberPremise.lean`, `proof/QCore.lean`.
* **Contracts:**
  * `providers/qcore-provider-librdkafka/contracts/librdkafka/{null_deref,memory,kafka_mem,queue_destructor}.qv`;
  * `contracts.lock`, `bounds-contracts.lock`, `contracts.json`;
  * pins in `compiler/tests/qcore_contract_corpus.rs`;
  * the parity fixture `compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv`.
