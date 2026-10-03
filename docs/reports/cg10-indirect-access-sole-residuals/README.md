# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: the uncontracted callees behind the access refusal

Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (wave 48, round 18). Base: local
`main` at `2ecd39531`, which it stayed at. Gateway
[rule 12k](../../agents/gateway.md#constraint-12k) (the gate's refusal of a
call whose uncontracted callee decides names that callee), Lean
`QCore.UncontractedAccessAttribution` (15 theorems), audited by
`proof/AuditUncontractedAccessAttribution.lean` (286 → 287 audits); seven
declared, pinned contracts in `qcore-provider-cmetrics` and
`qcore-provider-librdkafka`. [PREDICTION.md](PREDICTION.md) part 1 was
committed at `9aea0f16f`, before any implementation code was committed or
built (the rule-12k draft existed in the working tree and was set aside while
the BEFORE binaries were built from the base sources); part 2 at `bc9710be2`,
after rule 12k's own measurement and before any contract.

## 1. The true blocker (task 1, rule 12k)

**Reproduced.** The BEFORE corpus run is `d2f5a3f7…`, 695 / 9 / 4,130, byte for
byte the integrator's baseline; the census run (`scripts/census-run.sh
census-before`, same digest) and `scripts/classify.py` reproduce the
registered table exactly: 481 functions, 916 leaves, `uncontracted-callee` 227
alone (578 leaves), `return-uncontracted-callee` 102 (107), every leaf's class
identical to CG10-INDIRECT-ACCESS-SOLE-BLOCKERS' `classify-after.json`.

**The rule** (`legacy_uncontracted_access.rs`, asked last in
`expression_statement` and in `return_statement` after rule 12i; shared item
`reclassified_with_borrowed_actuals` in `legacy_call_argument_access.rs`). For
exactly the gate's refusal of a statement whose first call names a callee with
no contract in force and no review or pin hold (a held call's refusal is
`Pending`, so naming it would move a verdict), with no braces and no tracked
descriptor read, the statement is classified again with every memory-reading
actual of the call read as rule W44 reads one at a `Borrowed` position (the
lexical recogniser, else rule 10k's typed actual, with no parity recorded). If
that reading is refused with exactly ``call to uncontracted function `F` ``
for the same `F`, the refusal keeps the gate's writes and takes that
description; otherwise it is unchanged byte for byte. A whole-call `return`
is read through its synthetic `PLACE = f(args)`. The reading's counters are
restored and its instruction discarded: it never lowers anything.

Lean (`QCore.UncontractedAccessAttribution`): `only_the_gate_refusal_is_read`,
`the_writes_are_kept`, `the_result_is_the_gate_refusal_or_the_callees`,
`a_refusal_stays_a_refusal`, `the_verdict_of_every_node_is_unchanged`,
`the_status_of_a_function_is_unchanged` (the join over any list of nodes),
`a_contracted_callee_keeps_the_gate_refusal`,
`a_held_callee_keeps_the_gate_refusal`,
`a_descriptor_read_keeps_the_gate_refusal`,
`an_unrecognised_actual_keeps_the_gate_refusal`,
`an_admitting_reading_keeps_the_gate_refusal`,
`another_refusal_keeps_the_gate_refusal`,
`the_attributed_callee_is_the_statements`,
`an_attributed_leaf_is_never_pending`, `the_attribution_is_idempotent`, and
two `decide` examples. Tests (`tests/indirect_access_sole_residuals.rs`, 6): a
call statement and a whole-call return name their uncontracted callee and stay
`Unknown`; the same call with a `Borrowed` contract is `Safe`; a nested call in
an actual, a member destination and a contracted custody position keep the
gate's text.

**Measured on its own** (base + `e6414f3ca`; `scripts/attribution.py`, 0
problems: every changed leaf is one access leaf replaced at its line):

| route | BEFORE | rule 12k | access leaves | uncontracted-call leaves |
| --- | --- | --- | --- | --- |
| corpus | 695 / 9 / 4,130 `d2f5a3f7…` | **no status move** `00462adf…` | 13,083 → 10,121 | 8,433 → 11,442 |
| system | 85 / 4 / 359 | no status move | 1,212 → 738 | +474 |
| slice | 45 / 1 / 354, 4 Pending | no status move | 1,238 → 708 | +530 |

1,457 corpus functions change leaves. 47 more leaves are gained than removed:
the diagnostics are a set keyed by line and description, so a line holding two
refused statements (a macro expanding to several) carried one access leaf for
both, and naming one callee shows both (`rd_kafka_set_fatal_error0`,
`rdkafka.c:949`). Of the 685 leaves of the two classes, 463 now name their
callee; of the 331 functions, **215** are blocked by uncontracted calls alone
and **116 keep an access leaf** (an actual neither reader recognises: `errno`
spelled `(*__errno_location())`, a nested call, a cast such as
`(void *)on_conf_set`). Corpus-wide the functions the access refusal blocks
alone fall 481 → 200, and those blocked by uncontracted calls alone rise 477 →
758. The parity stream records every reading
([artifacts/attribution-corpus.json](artifacts/attribution-corpus.json)
`readings`: 3,122 statement and 126 return attributions, 7,591 unrecognised
actuals, 171 refused elsewhere).

**Corrections.** The registered ranking's premise that the 329 functions of
the two classes are blocked by their callees holds for 215: 116 also need an
actual read that neither reader makes, so a contract alone frees none of them.
`rd_kafka_interceptor_method_add` frees 4 alone, not 14: its callers' third
actual is the cast `(void *)on_…`, which W44 does not read.

## 2. The callees (task 2)

[artifacts/ranking-before.json](artifacts/ranking-before.json) (BEFORE, by the
first call of each refused statement), [kind-before.json](artifacts/kind-before.json)
(where each body is) and [decides-r12k.json](artifacts/decides-r12k.json)
(with rule 12k: only the functions each callee decides). Top of the rule-12k
ranking, with each callee's body:

| callee | alone | touched | body | its own blocker |
| --- | ---: | ---: | --- | --- |
| `rd_avg_add` | 12 | 14 | header inline (`rdavg.h:77`) | its one call, `rd_hdr_histogram_record` (cross-unit, home `Safe`) |
| `cmt_atomic_store` | 10 | 14 | cross-unit, home `Safe` | no cross-unit route |
| `cmt_atomic_load` | 8 | 8 | cross-unit, home `Safe` | no cross-unit route |
| `rd_kafka_confval_set_type` | 5 | 5 | cross-unit, Unknown | access, unresolved `valuep`, branch condition |
| `rd_atomic64_get` | 4 | 4 | header inline | `return __atomic_fetch_add(…)`: a builtin (class 6) |
| `rd_kafka_interceptor_method_add` | 4 | 4 | same unit, Unknown | retains `ic_opaque` and `func` (decision class 1) |
| `rd_atomic64_add` | 3 | 3 | header inline | builtin |
| `rd_kafka_queue_new0`, `rd_list_add`, `rd_list_clear` | 3 each | 3 | Unknown bodies | `rd_list_add` retains its element (class 1) |
| `rd_list_destroy` | 2 | 5 | cross-unit, Unknown | a destructor body (`W44-CALLEE-DESTRUCTOR-SUMMARY`) |
| `ctr_attributes_set_*` (6) | 2 each | 2–3 | cross-unit, Unknown | `cfl_kvlist_insert_*` (cfl, external); the kvlist and array forms take custody |
| `rd_atomic32_add` | 2 | 3 | header inline | builtin; **backs `rd_refcnt_add0` (class 3)** |
| `cnd_timedwait_ms` | 2 | 2 | cross-unit, home `Safe` | no cross-unit route |
| `cmt_atomic_compare_exchange` | 1 | 5 | cross-unit, home `Safe` | no cross-unit route |
| `rd_atomic32_set` | 1 | 3 | header inline | builtin; **backs `rd_refcnt_set` (class 3)** |

Fixed point in rank order over the 215: 12, 22, 30, 35, 39, 43, 46, 49, 52, 55,
… Corpus-wide (every Unknown function blocked by uncontracted calls alone,
758): the same callees lead, beside the W48 bottom-up bodies
(`cJSON_New_Item` 10, `LZ4_decompress_generic` 9).

## 3. The lift (task 3)

Each declaration is the neutral element (every formal `resource_borrowed`, the
scalar result `resource_return_none()`), read off every definition at both
audited revisions (byte-identical at `66910c10` and `046accf6`,
[`scripts/compare_revisions.py`](scripts/compare_revisions.py)) and pinned
there with `quirin-audit pin` ([`scripts/pin.sh`](scripts/pin.sh)):

* `qcore-provider-cmetrics`, `cmetrics/metric_values.qv`: `cmt_atomic_load`,
  `cmt_atomic_store`, `cmt_atomic_compare_exchange`. W37 left them undeclared
  for their five backend definitions (`CMT-ATOMIC-BACKEND-COPIES`); every copy
  (`_clang.c`/`_gcc.c` builtins, `_generic.c` under a pthread mutex, `_msvc.c`
  under a critical section or `_Interlocked*64`) reads or writes the
  `uint64_t` at `storage` and returns a scalar, so they agree (W37's
  refinement of `ADD-NAME-COLLISION`). The analysed copy's rows are `Safe` in
  `cmt_atomic_clang.c`, and constraint 8's duty gate compares the
  declarations with its summaries there (no `SummaryConflict` added).
* `qcore-provider-librdkafka`, `librdkafka/memory.qv` beside `rd_atomic32_get`:
  `cnd_timedwait_ms` and `rd_hdr_histogram_record` (home rows `Safe`, callers
  in other units), and the header bodies `rd_atomic64_get` and
  `rd_atomic64_add` (as `rd_atomic32_get`; their bodies are a builtin call).
  `rd_avg_add` is **not** declared: with `rd_hdr_histogram_record` declared its
  header body validates in every including unit under W4 (rule 4a), the
  checked route.

**Withdrawn before any record: `rd_atomic32_add`, `rd_atomic32_set`.** The
first AFTER measurement (`after1`, [artifacts/attribute-after1.json](artifacts/attribute-after1.json))
declared them too and moved 65 functions, among them `rd_kafka_cert_dup`
(`rd_refcnt_add(&src->refcnt); return src;`) and `rd_kafka_toppar_list_copy`
(through the header body `rd_kafka_toppar_keep0`): librdkafka's reference
counts are these two functions (`rd.h:338,351`, `#define rd_refcnt_add0(R)
rd_atomic32_add(R, 1)`), so a name-keyed declaration reads every reference
`keep` as custody-neutral, which is `W48-CALLEE-DECISION-TABLE` class 3, a
decision this card does not make. They were removed, the librdkafka lock was
restored to its base bytes and the four remaining declarations pinned again
by the tool; a control asserts `rd_atomic32_add` stays uncontracted.

**Pins.** `pin` at both trees exits 0; `pin --check --crate` for each changed
crate exits 0 at both (cmetrics 26, librdkafka 17 recorded, 0 not in force, 0
stale); `pin --verify-contracts` exits 0. The whole-tree `pin --check` exits 1
at both trees, as on the base: its only row not in force is the libevent
crate's tip-mode `mk_vhost_set` contract, which by design is in force only at
Monkey's tip (MONKEY-VHOST-PROPERTY-CUSTODY). Deliberate pin updates:
`compiler/tests/contract_loader.rs` 265 → 272 v1 declarations and seven
previous-mode rows; `compiler/tests/qcore_contract_corpus.rs` seven clause
rows; the hand-written form of the converted `librdkafka/memory.qv`
(`compiler/tests/fixtures/contract_inherit_qcore_provider/`, four
declarations that inherit nothing); and the cmetrics inventory digest the
source-contract consumer pins (`contract/profiles/cmetrics-counter/source-contracts.json`,
`5ef81f3f…` → `93dcbddb…`, the README's "update that consumer's manifest pin
deliberately"). Controls (`tests/cg10_sole_residual_contracts.rs`, 5, the production
sections byte for byte): each declared helper reads its member actuals and is
`Safe`; without the card each statement names its uncontracted callee; an
owned buffer handed to a borrowed position and dropped still leaks; the
`rd_avg_add` shape validates with its histogram call declared and is Unknown
without it; `rd_atomic32_add` stays uncontracted.

Not attempted, each by name: callees whose actuals neither reader recognises
(116 functions, above); decision-class callees (`rd_kafka_interceptor_method_add`,
`rd_list_add`, `rd_kafka_timer_start0`: retained pointers and callbacks, class
1; `rd_atomic32_add`/`_set`, `rd_kafka_toppar_keep0`: reference counts, class
3); Unknown bodies (`rd_kafka_confval_set_type`, `rd_kafka_queue_new0`,
`rd_list_clear`, `rd_list_destroy`, `mk_header_prepare`); and the
`ctr_attributes_set_*` chain, which needs cfl's `cfl_kvlist_insert_*` (two of
which take custody of their value) and then ctraces declarations. The smaller
access classes were not lifted (section 5).

## 4. Measurement

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus (`66910c10`) | 695 / 9 / 4,130 | **756 / 9 / 4,069** | `d2f5a3f7…` → `ce57d194…` |
| system | 85 / 4 / 359 | 85 / 4 / 359, no status move | `e395d8a9…` → `0d793dad…` |
| tail-pause slice | 45 / 1 / 354, 4 Pending | unchanged statuses | `a69bc558…` → `b4c6b390…` |

The system and slice reports embed this worktree's paths, so their digests are
this worktree's; their totals equal the integrator's baseline. The contracts
move no system or slice row (`compare.py` rule 12k → AFTER: 0 moving rows);
those routes move by rule 12k's leaves only.

**Corpus: +61 Unknown → Safe, no other status move** ([compare-corpus.json](artifacts/compare-corpus.json);
attributed by [`scripts/attribute_contracts.py`](scripts/attribute_contracts.py)
over rule 12k → AFTER, [attribute-after2.json](artifacts/attribute-after2.json)):

* `cmt_atomic_load` 13 (the cmetrics value getters and `*_count_set`/`*_inc`),
  `cmt_atomic_store` 11 (the setters, `map_unlock`, `add`),
  `cmt_atomic_compare_exchange` with `_store` 5 (the four `*_exchange` loops
  and `metric_exchange`);
* `rd_avg_add` (its header body validated through `rd_hdr_histogram_record`) 12
  (`unit_test_telemetry_set_*`), `rd_atomic64_get` 4
  (`calculate_share_*_total`), `rd_atomic64_add` 3, `cnd_timedwait_ms` 2
  (`rd_kafka_brokers_wait_state_change`, `rd_kafka_metadata_cache_wait_change`);
* callee summaries newly validated, 6: `cmt_metric_add`, `_inc`, `_sub` (over
  `add`), `cmt_metric_hist_sum_add`, `_set`, `cmt_summary_sum_set` (over the
  `*_sum_exchange` loops);
* read by hand, 5: `map_lock` and `cmt_metric_exp_hist_lock` (a held
  `cmt_atomic_compare_exchange` in a loop condition), `cmt_metric_get_value_type`
  (`return (int) cmt_atomic_load(…)`), `cmt_metric_has_start_timestamp`
  (`cmt_atomic_load(…) != 0` in a return), and
  `rd_kafka_cgrp_consumer_heartbeat_preconditions_met` (the header body
  `rd_kafka_max_poll_exceeded`, which reads `rd_atomic64_get`, in a branch
  condition).

47 of the 61 are among the 331 functions; 14 lie outside them (cascades and
held calls). **No Safe → non-Safe move, no correction; no leak or violation
added** (leak leaves 40 → 40, `Unsafe` 9 → 9, `SummaryConflict` 2 → 2). Leaves
BEFORE → AFTER: access −2,966, uncontracted call −28 / +2,873 (rule 12k's net),
branch condition −11, loop condition −2, return expression −1, unresolved
identity −3 / +18 (roots the newly read actuals expose, in functions that stay
Unknown).

**Prediction against outcome.** Rule 12k: no status move on any route
(held); 463 of 685 class leaves re-attributed (400–600), 215 of 331 functions
decided by callees (170–270), 2,962 corpus leaves (800–3,000). Contracts:
predicted +30 to +50, measured **+61**, above the range by 11: the cmetrics
getters and setters cascade further than counted (the `add`/`*_exchange`
summaries, 6) and five functions held their calls inside a condition or a
return, which the ceiling of 47 did not read. No Safe → non-Safe and no
provisional `Unsafe`, as predicted.

**`mk_vhost_set` at the tip** ([scripts/tip-probe.sh](scripts/tip-probe.sh),
the BEFORE and AFTER binaries; [before](artifacts/tip-rows-before.txt),
[after](artifacts/tip-rows-after.txt)): unchanged. The tip's one refusal is
the uncontracted `mk_vhost_set_property`, whose two leaves are rule 12k's
non-cases: `vh->documentroot.data = mk_string_dup(v)` (an owned result stored
into a member, the DocumentRoot custody question, real behaviour, refused)
and `vh->documentroot.len = strlen(v)` (a contracted call's scalar result
stored through a pointer root, `call-result-store`, residual).

## 5. Residual (amends `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS`)

On the AFTER corpus ([residual-after2.json](artifacts/residual-after2.json),
each leaf classed by the BEFORE census at its line, and the two uncontracted
classes split by rule 12k's answer): **205** functions blocked by the access
refusal alone, 347 leaves.

| class | alone | leaves | touched |
| --- | ---: | ---: | ---: |
| uncontracted callee, an actual neither reader recognises | 43 | 99 | 69 |
| whole-call return, the same | 24 | 25 | 24 |
| `custody-position:released` (by design) | 12 | 40 | 23 |
| `pointer-store:other` (rule 10v) | 12 | 14 | 14 |
| `elaboration`: no value form | 10 | 15 | 14 |
| `pointer-store:formal-pointer` (rule 10v) | 9 | 14 | 12 |
| uncontracted callee, no rule-12k reading at the line (a macro line or a descriptor read) | 8 + 5 | 23 | 18 |
| `probe-declined` | 6 | 11 | 10 |
| `call-result-store` (incl. `vh->documentroot.len = strlen(v)`) | 5 | 15 | 12 |
| `unrecognised-actual:other` | 5 | 6 | 6 |
| 25 smaller classes | 24 | — | — |
| two or more classes | 42 functions | | |

Fixed point in rank order: 43, 67, 88, 101, 113, 122, 131, 140, 151, …. Beside
it, the callee-decided population: 704 Unknown functions are blocked by
uncontracted calls alone ([decides-after2.json](artifacts/decides-after2.json);
`rd_kafka_confval_set_type` 5, `rd_kafka_interceptor_method_add` 4,
`rd_kafka_queue_new0` 3, `rd_list_add` 3, `rd_list_clear` 3, …), owned by the
P4 callee closure.

## 6. Gate

Standing gate (`docs/agents/validation.md`), `scripts/gate.sh g3` then
`scripts/finish-gate.sh g3`, code steps at `abba7ce22` on base `2ecd39531`;
logs `private/cg10res/gate-g3/`, exits copied to [gate/](gate/), summary
[gate.json](gate.json):

| step | exit | result |
| --- | ---: | --- |
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 441 units, 454 summaries; 7,561 passed, 0 failed, 11 ignored (`private/test-binaries/20261003T213057Z-3295650`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 287 audits passed |
| `python3 -m unittest discover -s scripts/tests` (before the matrix) | 1 | the expected stale `lean_audits` (286 committed, 287 produced) |
| `build_matrix.py . --gate gate.json` | 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK |
| `build_docs_index.py`, `--check` | 0, 0 | |

**Provider gate** (`scripts/provider-gate.sh g2`, at `468be8322`; `providers/`
unchanged since): `cargo check --locked`, fmt, clippy 0; 129 provider tests;
24 provider Python tests OK; `providers/contracts.py check` 0; package build 0
and `verify` 0 for all five packages. Targeted runs before the gate:
`scripts/targeted.sh t1`, `scripts/pins.sh p1` to `p7` (logs under
`private/cg10res/`).

**Pins flipped deliberately** (rule 12k; the gate's second run, `gate-g2`,
failed exactly these and the two contract pins above, 7,548 passed / 13
failed): each asserted the gate's text on a statement whose uncontracted
callee rule 12k now names, and each still asserts `Unknown` and never `Safe`:
`call_argument_access` (`y_unknown(r->count)`), `address_of_member`
(`a_untracked(&hold->lock)`), `borrowed_field_actuals` (`b_untracked(storage->cio)`),
`neutral_frame_recursion` (`purge(n->next)`, its own uncontracted recursive
call), `return_position_pointer_leaf` (`return z_unknown(view->kind)`),
`o04_b09b10` (the constructor's gate refusals 3 → 2: `mk_list_del(&ch->_head)`
at L177 is named), and `o04_arms`, `o04_b08`, `o04_read_bound`, whose
real-unit residual lists gain the fixture-uncontracted
`msgpack_packer_init`, `mk_list_add`, `flb_hash_table_add`,
`pthread_mutex_lock`/`_unlock` (B09/B10's eight gate refusals on C6's path now
name these callees: 8 → 0 under the gate's text, the same 8 statements named;
the three tests now report every unexpected refusal at once). The first run
(`gate-g1`) stopped at clippy (`question_mark` in rule 12k's return form,
fixed at `468be8322` with no behaviour change). The measured binaries were
built before that fix and before `cargo fmt` (`e6414f3ca`'s code, whitespace
and that one `?` aside); both are behaviour-neutral.

## Files

* `crates/qcore-c-gateway/src/legacy_uncontracted_access.rs` (new); hooks in
  `legacy_c.rs` (`expression_statement`, `return_statement`, the module
  declaration); shared item in `legacy_call_argument_access.rs`.
* `proof/QCore/UncontractedAccessAttribution.lean`, its audit, `proof/QCore.lean`.
* Contracts: `providers/qcore-provider-cmetrics/contracts/cmetrics/metric_values.qv`,
  `providers/qcore-provider-librdkafka/contracts/librdkafka/memory.qv`, their
  `contracts.json` and `contracts.lock`.
* Tests: `tests/indirect_access_sole_residuals.rs`,
  `tests/cg10_sole_residual_contracts.rs`; pins in
  `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`,
  the hand-written conversion fixture, `contract/profiles/cmetrics-counter/source-contracts.json`
  and the nine gateway tests of section 6.
* Scripts: `build.sh`, `measure.sh`, `census-run.sh`, `classify.py` (the
  BEFORE census), `callee_census.py`, `callee_kind.py`, `attribution.py`,
  `decides.py`, `class_leaves.py`, `compare.py`, `attribute_contracts.py`,
  `residual_table.py`, `totals.py`, `compare_revisions.py`, `pin.sh`,
  `lock_delta.py`, `sync.sh`, `targeted.sh`, `pins.sh`, `tip-probe.sh`,
  `tip_rows.py`, `gate.sh`, `provider-gate.sh`, `finish-gate.sh`.
