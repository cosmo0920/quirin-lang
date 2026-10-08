# VERDICT-CONTRACT-DEPENDENCIES: the declarations each verdict rests on

Card VERDICT-CONTRACT-DEPENDENCIES (wave 48, card-high), registered by the
integrator with the served-queue premise's review evidence
([REFCOUNT-FAMILY-BLOCKING-CALLEES §9](../refcount-family-blocking-callees/README.md#9-review-evidence-for-the-served-queue-premise-integrator-2026-10-09)),
on the user's direction of 2026-10-09: "handle as safe as possible". Base:
local `main` at `cdc422ad1`, which it stayed at.
[PREDICTION.md](PREDICTION.md) was committed at `042fa6696`, before any
implementation code.

**Published property and scope.** No new property: custody (`c.custody`) on
the pinned corpus, the system route and the tail-pause slice, unchanged. What
the card adds is the per-row **assumptions** component of the verdict
contract (language specification L20; contract specification C06.5): every
published row now names the contract declarations its derivation consulted,
and the run says, per declaration, whether a body it analysed validated it.
Gateway rules **13f–13h**; Lean `QCore.ContractDependencies`.

## 1. Design

**The set (rule 13f).** `qcore_schemas::contract_consultation` is a
per-thread log: a lookup by name in a resource environment
(`contract`, `relational_contract`, `retaining_contract`) notes the name and
whether it found a declaration, only while a log is open. The audit opens a
`Recording` around one unit's analysis
(`qcore_audit::projection::project_{prepared_,}unit_into`; the certificate
recheck runs after it closes). Inside, `contract_dependencies::within`
attributes lookups to the function whose work is running (every CFG build,
W4 inference, contextual CFG and per-function analysis). A row's set is:

* every lookup of its own work **whose name its body spells** in the stream
  the build read (unit rewrites, indirect-call candidates and held spellings
  included; a minted spelling is read back to its declaration). The filter is
  needed: the first build (`a1`) attributed 179–197 declarations to every
  system row, because passes such as `literal_premise::restrict_to_declared_literals`
  scan the whole unit inside one function's build (found with a backtrace on
  the lookup, run `dbg`);
* every callee its solved CFG applies a contract of (`ContractCall` and its
  nested calls, element stores and releases): the solver reads contracts only
  through those instructions, so every contract that affected the abstract
  interpretation is in the set whatever path looked it up;
* its own name (its entry contract);
* closed over every **in-unit summary** it consulted (a W4 inference, an
  inferred disposition, ranked or neutral recursion): the summary's own set is
  added, transitively.

Each name is classified against the unit's input environment: `held` (pending
review, pin mismatch or a held spelling), `withdrawn` (a withdrawal spelling,
withdrawn in the unit, or refuted outside its own body), `declarations`
otherwise; names nothing declares are dropped. The loader's duplicate check
was the one non-consultation lookup and now reads the maps directly.

**Body standing (rule 13g).** A unit records positive evidence and failures
for the declared functions whose bodies its final derivation scope holds:
`W4` (the final derivation's summary equals a declaration with no container
or counted-user clause), `entry` (the body under its declaration's entry
custody matches every exit, `Safe`, no diagnostic), `11r`, `12w`, `13a`,
`13c`, `13d`; failures are a summary conflict, a refutation, an ABI or mode
withdrawal, a checked specification not established, a contradicted
disposition, a failed structural check, a dropped guard. A failure wins in
its unit. Nothing is recorded under hypothetical holds (rule 10i's second
analysis) or from intermediate derivations. For the run a declaration is
`validated` only when every unit that examined one of its bodies validated
it and one did; `held` when pending review or not in force; every other a
`declared-assumption` (C06). Under "as safe as possible" the rule is
deliberately conservative: two `static` bodies of one name, one validating
and one not, make the declaration a declared assumption.

**Evidence only (rule 13h).** The row field `contract_dependencies`
(`declarations` always; `held`, `withdrawn`, `summaries` when nonempty), the
unit field `contract_body_evidence`, and the sidecar section
`contract_dependencies` (per declaration digest, mode, status, body standing,
bodies, premises, rows and Safe rows; the Safe roll-up and the top 20) are
written after the verdict from a record no analysis reads. Every route
writes the section (pinned, current, contracts-only, configured; the composer
folds it from the rows like the refutation section, under the
`contract_premises` the current route records before its units).

**Premises (contract specification C04.18).** A new contract-level clause
`requires resource_premise(NAME, "STATEMENT");`, total-or-reject (lowercase
identifier, one plain non-blank string, each name once), rederived from the
retained texts, inherited and compared like the other markers, changing no
mode. It is part of the declaration's text, so the pin covers it: the control
run (`pin --check` with `a4` before the re-pin, `private/vcd/pin-control/`)
reported both declarations `contract-changed`, not in force (exit 1). The
section also reports each literal-argument premise (`literal_argument`,
`do_free = 1`) and rule 13e's two premises for a result holder.

Attached: the served-queue premise to `rd_kafka_q_wait_result` and
`rd_kafka_q_purge0` (`librdkafka/client.qv`), re-pinned with
`quirin-audit pin` at `66910c10` and `046accf6` (`scripts/pin.sh`, logs
`private/vcd/pin-a4/`: both pins 0, the crate's `--check` 0 at both, the
whole-tree check 1 at both as on the base (the libevent crate's tip-mode
`mk_vhost_set`), `--verify-contracts` 0).

Premises still stated only in comments (registered,
`VERDICT-CONTRACT-PREMISE-ANNOTATIONS`): the reachability premise of
`fluent_bit/output_singleplex.qv`; `AOM-DESTROY-CORRUPTED-UNPAID-PREMISE` in
`fluent_bit/input_chunk_destroy.qv` and `input_chunk_destroy_direct_routes.qv`;
"C3's ingestion premise" in `fluent_bit/loop_owner.qv`; O04-TRACE's premises
N1–N10 in `libc/errno.qv`; the libc/C-standard ABI premise of every libc
module (already the run's named standing assumption `libc-c-standard-abi`).

## 2. BEFORE

Binary `private/vcd/bin/before` `be16e3ba…` (= each `run.json`):
corpus **881 / 9 / 3,944** `36fd1649…` (432.5 s), system 87 / 4 / 357
`c000abd8…` (58.7 s), slice 45 / 1 / 354 with 4 Pending `0cd51920…`
(26.2 s): the brief's figures.

## 3. AFTER and the identity proof

Binary `private/vcd/bin/a5` `377d3462…` (= `run.json`; the card's final
code), every route exit 0: corpus 433.0 s, system 58.7 s, slice 27.2 s (no
measurable cost against BEFORE's 432.5 / 58.7 / 26.2 s). The first AFTER,
`a4` (`851bf3dd…`), wrote byte-identical reports on every route; between the
two the gate found two defects, both fixed before `a5` (§8), neither on a path
the routes take.
[`scripts/identity.sh`](scripts/identity.sh) runs
[`verdict_projection.py`](scripts/verdict_projection.py) and `parity.py` on
each route. The projection removes exactly the card's keys and re-serialises
in serde's form; on every BEFORE report it reproduces the bytes
(`projector_reproduces_before: true`).

| route | BEFORE report | AFTER report | projection of AFTER | verdict rows (before = after) | parity multiset |
| --- | --- | --- | --- | --- | --- |
| corpus | `36fd1649…` | `96852f4e…` | **`36fd1649…`** (byte-identical) | `97cbdcf3…` | identical (662,667 rows) |
| system | `c000abd8…` | `2c0c8f6b…` | `0a555b1e…` | `09b7a59e…` | identical (139,962) |
| slice | `0cd51920…` | `f38e0ccf…` | `278f0c26…` | `ce1be80e…` | identical (50,407) |

The system and slice projections differ from BEFORE only at
`units[*]/compile_configuration/{dependencies,compile_task_identity}/resource_contract_sources`
and `inventory_sha256`: the current route names the contract sources'
digests, and the premise changed `client.qv`. The sidecars differ (after
projection) only in that module's text and digest, the two declarations'
`contract_sha256` and the crate lock's digest; the script lists every
differing path and fails on any other (none). The system run before the
premise (`a3`, `59530ef3…`) projects to `c000abd8…` byte for byte. The
parity streams are never identical in order between two runs with
`--jobs 4` (BEFORE against itself would differ the same way); as multisets
they are identical on all three routes. Totals unchanged: 881 / 9 / 3,944,
87 / 4 / 357, 45 / 1 / 354 / 4 Pending.

`mk_vhost_set` tip probe ([`scripts/tip-probe-base.sh`](scripts/tip-probe-base.sh)
for BEFORE, which needs the base contracts because the base loader rejects
the new clause; restored and digest-checked): verdict rows identical on the
tip and the master copy. `mk_vhost_set` Unknown (tip 1 leaf, master 6),
dependencies `{"declarations": ["mk_vhost_set"], "summaries": ["mk_vhost_lookup"]}`
at the tip (its own declaration, tip-in-force) and `held: ["mk_vhost_set"]`
on master; `mk_vhost_set_property` Unknown (2 leaves) on both, resting on
`mk_list_add`, `mk_mem_alloc`, `mk_string_dup`, `strcasecmp`, `strlen`.

## 4. The reviewer's work list

From each AFTER sidecar ([`scripts/rollup.py`](scripts/rollup.py)):

| route | Safe rows | on no declaration | only on validated | on a declared assumption | consulting held/withdrawn | declarations consulted (validated / declared assumption / held) |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| corpus | 881 | 418 | 85 | **378** | 0 | 330 (73 / 256 / 1) |
| system | 87 | 47 | 3 | **37** | 0 | 99 (8 / 89 / 2) |
| slice | 45 | 29 | 0 | **16** | 0 | 83 (4 / 70 / 9) |

Top 20 declared assumptions by corpus Safe rows: `strcmp` 35, `memcpy` 32,
`mtx_lock` 27, `mtx_unlock` 27, `rd_free` 23, `snprintf` 16,
`LZ4_decompress_generic` 12, `__assert_fail` 12, `memset` 10, `calloc` 9,
`free` 9, `cfl_sds_destroy` 8, `clock_gettime` 8, `rd_atomic32_get` 8,
`strlen` 8, `cJSON_New_Item` 7, `fprintf` 6, `mk_print` 6, `setsockopt` 6,
`strcasecmp` 6. Thirteen of them are libc under the standing assumption; the
pinned ones (`rd_free`, `LZ4_decompress_generic`, `cfl_sds_destroy`,
`rd_atomic32_get`, `cJSON_New_Item`, `mk_print`) are where a review buys the
most.

Validated (corpus, 73) by evidence: W4 45, entry 38, 13c 10, 13d 9, 13a 7,
11r 2 (a declaration can carry several); 28 rest on a structural check alone
(the rule 13a/13c/13d declarations of `librdkafka/client.qv` and five 13a
borrowed results), which validates the clause that check reads, not every
mode of the declaration (`VERDICT-BODY-STANDING-CLAUSE-SCOPE`). Header
`static inline` bodies analysed in every unit that includes them
(`flb_malloc`, `flb_calloc`, `mk_mem_alloc`) validate in each.

`rd_kafka_toppar_purge_and_disable_queues` (the ledger's example) is Safe
resting on `mtx_lock`, `mtx_unlock` (libc, standing) and `rd_kafka_q_purge0`
(pinned in force, body standing **declared assumption**: its one analysed body
is Unknown, premise `served_queue`), through the in-unit summary of
`rd_kafka_q_disable0`. `rd_kafka_q_purge0` is consulted by 11 corpus rows (1
Safe), `rd_kafka_q_wait_result` by 6 (0 Safe).

## 5. Size

Names, not indices: the field adds 1,164,559 bytes to the corpus report
(10,194,083 → 11,358,642, +11.4%), 124,712 to the system report (+8.0%) and
100,166 to the slice's (+6.4%); the sidecar section adds 219,899 / 70,916 /
56,266 bytes. Below the 25% the prediction set for a compact encoding, so
none was built. Mean declarations per row: corpus 1.59 (Safe 0.77, at most
22).

## 6. Prediction against outcome

| prediction | outcome |
| --- | --- |
| no verdict moves; projections byte-identical to BEFORE | held for the corpus report; system and slice reports differ in the contract-source digests the current route records (not foreseen), verdict rows identical |
| parity identical in order after address erasure | wrong: never in order across `--jobs 4` runs; multisets identical |
| premise covered by the pin digest; re-pinned, no movement | held (control `contract-changed`) |
| fewer than 15% of corpus Safe rows rest only on validated declarations or none | wrong: 503 of 881 (57%) rest on none (418) or only validated ones (85) |
| top assumptions allocator and string primitives | partly: string and lock primitives lead (`strcmp`, `memcpy`, `mtx_lock`); `free`/`calloc` 9 each |
| `rd_kafka_q_purge0` on the example row with the premise, declared assumption | held |
| +10–20% size | +11.4% corpus |

## 7. Lean

`proof/QCore/ContractDependencies.lean`, audited by
`proof/AuditContractDependencies.lean` (18 theorems, no placeholder, no
project axiom): `an_applied_call_is_a_dependency`,
`a_consulted_declaration_is_a_dependency`,
`the_entry_contract_is_a_dependency` (completeness);
`a_summary_dependency_propagates`, `a_summarys_applied_call_is_a_dependency`,
`reaches_trans` (propagation); `publishing_the_set_changes_no_verdict`,
`two_sets_publish_the_same_verdict_row`; `a_declared_assumption_is_never_validated`,
`no_examined_body_is_a_declared_assumption`,
`an_unvalidated_body_is_never_validated`, `a_failed_body_is_never_validated`,
`validated_needs_evidence_in_every_unit`, `a_failure_wins_in_its_unit`,
`a_held_declaration_is_never_validated`; shared vectors
`outcome_vectors_hold`, `standing_vectors_hold`, `closure_vectors_hold`,
bound to the Rust by `crates/qcore-c-gateway/tests/contract_dependencies.rs`
and `crates/qcore-audit/tests/contract_dependencies_report.rs`.

## 8. Gate

Gate `g2` (at `8b200b77d`) failed two test binaries, both the card's defects,
fixed at `58bc2b290`: `qcore-contract::contract_inherit` (7): the
inherited-declaration agreement compared the rendered literal-argument
premise, which names each declaration's own parameter, so a child inheriting
a literal premise (`flb_table_put` from `cfl_table_put`) was refused; it now
compares the named premises only (the literal premise is compared by index
as before). `qcore-audit::checkpoint_cli` (3): the composer printed the
dependency tally after the refutation tally, the whole run before it. An
earlier attempt (`g1`) stopped at clippy (`collapsible_match`, fixed at
`8b200b77d`). Final gate: [gate.json](gate.json) and [gate/](gate/).

## 9. Ledger

* `VERDICT-CONTRACT-DEPENDENCIES` **retired**: every published row carries the
  consulted declarations with mode, status and body standing (validated,
  declared assumption, held; withdrawn per row), from the route's own
  consultations, verdicts identical, Lean parity, standing gate.
* Registered `VERDICT-CONTRACT-PREMISE-ANNOTATIONS` (the comment-only
  premises of §1, five modules, each a pinned-text change and re-pin) and
  `VERDICT-BODY-STANDING-CLAUSE-SCOPE` (28 corpus declarations validated by a
  structural check of one clause kind alone).

## Files

* Code: `crates/qcore-schemas/src/contract_consultation.rs`,
  `crates/qcore-c-gateway/src/contract_dependencies.rs` and its hooks
  (`legacy_c.rs`, `legacy_summary.rs`, `legacy_recursion.rs`,
  `legacy_review_standing.rs`, `legacy_slot_move_borrow.rs`,
  `legacy_retention_guard.rs`, `legacy_reference_count.rs`,
  `legacy_reference_result.rs`, `legacy_declared_modes.rs`,
  `legacy_literal_premise.rs`), `crates/qcore-audit/src/contract_dependencies.rs`
  and the route hooks (`projection.rs`, `pinned_route.rs`, `current.rs`,
  `no_build.rs`, `compose.rs`, `lib.rs`), `crates/qcore-contract/src/contract_loader.rs`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`. Pins: `compiler/tests/qcore_contract_corpus.rs` (two
  clause rows 4 → 5).
* Tests: `crates/qcore-c-gateway/tests/contract_dependencies.rs`,
  `crates/qcore-contract/tests/contract_premise.rs`,
  `crates/qcore-audit/tests/contract_dependencies_report.rs`.
* Specifications: contract C04, C04.18, C06.5; language L20; grammar G05.
* Scripts: [`scripts/`](scripts/).
