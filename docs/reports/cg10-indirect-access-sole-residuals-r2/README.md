# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: four readings of the access residual

Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, round 2 (wave 48, round 21). Base:
local `main` at `84ec54d1d`; after the measurements the card was rebased onto
`6f64d545e` (CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS landed) and the
three routes measured again on the rebased tip (section 3). Gateway rules
[14a](../../agents/gateway.md#constraint-14a)–[14d](../../agents/gateway.md#constraint-14d),
Lean `QCore.ResidualAccessReadings` (18 audited theorems, `proof/AuditResidualAccessReadings.lean`;
303 → 304 audits on the base, 305 → 306 on the rebased tree); two neutral declarations in `qcore-provider-chunkio` and
`qcore-provider-ctraces`, pinned at both audited revisions.
[PREDICTION.md](PREDICTION.md) was committed at `ea9b35e3c`, before any
implementation code was written.

## 1. BEFORE and the population (task 1)

**Reproduced.** The base binary (`3f385e08…`) measured corpus `66910c10`
**881 / 9 / 3,944** (report `ab8f5f14…`, verdict rows `321a8913…`), system
87 / 4 / 357 and the tail-pause slice 45 / 1 / 354 with 4 Pending, the
integrator's figures. The census run (`scripts/census-run.sh`, the
`indirect-census` binary) wrote the same report byte for byte.

**Re-measured.** Unknown functions blocked by `indirect memory access is
outside the resource projection` alone: **228 functions, 380 leaves** (205 /
347 after round 1; 203 carry over, 25 are new, exposed by later cards'
contracts). The first round's `classify.py` and `residual_table.py`, unchanged,
classify them ([classify-before.json](artifacts/classify-before.json),
[residual-before.json](artifacts/residual-before.json)).

**Ranking, checked for other blockers.** `scripts/leaf_detail.py` joins every
leaf with what the readers recorded at its line; `scripts/rank.py` turns each
leaf into the fixes it needs and counts the functions one fix frees **alone**
([rank-before.json](artifacts/rank-before.json)). An uncontracted-callee leaf
needs the callee's contract **and** a reader fix for every unread actual, so
no reader lift frees any of the 41 + 24 functions of the two largest classes:
a lift there re-attributes. That corrects the brief's expected heads (43 and 24
"alone" were functions freed alone by *contract plus reading*). Read by hand,
the classes freed alone by one fix were mixtures; PREDICTION.md lists the
sub-shapes readable under existing premises (lifts A–D) and the rest by name.

## 2. The lifts (task 2)

| rule | shape | premise | freed (corpus) |
| --- | --- | --- | ---: |
| [14a](../../agents/gateway.md#constraint-14a) | a scalar statement whose value holds exactly one call: the call alone (W44's reads first), then the statement's typed reads and store, the call an opaque scalar | the callee's contract in force has a `None` result and every position `Borrowed`: it changes no binding any read is decided on, so C's unspecified operand order decides nothing | **8** |
| [14b](../../agents/gateway.md#constraint-14b) | a pointer variable given `&PATH` or `PATH ± n` | every root never bound; the alias is exactly the pointer read's (target-free, unknown, possibly null) | **1** |
| [14c](../../agents/gateway.md#constraint-14c) | rule 10n(1)'s address at an actual may be a scalar element `&PATH[I]` | the array holds no pointer; 10n(1)'s type test and never-bound roots stand | **7** (4 direct, 3 through validated summaries) |
| [14d](../../agents/gateway.md#constraint-14d) | an actual `P ± n` (`P` a path or a never-bound pointer variable) | the address premise: every root never bound; W44's second obligation unchanged | **2** |
| contracts | `cio_errno_print`, `ctr_errno_print`: neutral (every formal borrowed, `resource_return_none()`) | their `errno` actual is read by CONSTRUCTOR-RESIDUALS' nested rule over libc's `__errno_location` | **6** |

**Lift C of the prediction needed no rule.** The prediction planned an `errno`
reader (`(*__errno_location())`). On reading the classifier, a contracted
outer call already reads that actual (CONSTRUCTOR-RESIDUALS' nested rule over
the libc declaration of `__errno_location`); only the two callees lacked
contracts. The 14a–14d identifiers were therefore assigned as above (the
prediction's lift D, the element address, is rule 14c), and rule 14d, the
interior offset at an actual, is a further lift measured after the second
AFTER run, down the ranking (it was not predicted).

**Contracts** (`chunkio/chunk_residency.qv`, `ctraces/context_lifetime.qv`):
each body read at both revisions (`cio_log.c` and `ctr_log.c` byte-identical
at `66910c10` and `046accf6`; `ctr_log.h` differs only in its logging macros'
file-name spelling, the prototype unchanged), pinned with `scripts/pin.sh`
(`pin` 0 at both trees for both crates; `pin --check --crate` 0 at both;
`pin --verify-contracts` 0; the whole-tree `--check` exits 1 at both trees, as
on the base, only for the libevent crate's tip-mode `mk_vhost_set`). The first
spelling, `file: ptr<mut, u8>`, was withdrawn in every unit by the declared-ABI
gate (`abi-parameter-type-disagreement`, measured in the first AFTER run, no
row freed); `const char *` is `ptr<const, u8>` (commit `f56b28509`). Both
bodies validate under W4 in the AFTER run (`body_standing: validated`), so
these are checked declarations, not assumptions resting unchecked.

**Controls** (`tests/indirect_access_sole_residuals_r2.rs`, 8 tests; unit tests
in `residual_access_reading.rs` and `scalar_element_address.rs` bind the Rust
decisions to the Lean tables): each rule admits its shape and is `Safe` on a
never-bound formal; two calls, a releasing position, an owned result and an
uncontracted callee keep the refusal (14a); a bound root, a slot address and a
release through the alias stay non-`Safe` (14b); a pointer or record element
stays refused (14c); a bound root, a releasing position and a cast operand stay
refused (14d). The alias of rule 14b dereferenced afterwards is decided by its
own rule (an unresolved identity, possibly null), never by the gate.

## 3. Measurement (task 5)

Binaries: BEFORE `3f385e08…`, AFTER `aa878c14…` (`run.json`'s `binary_sha256`
checked on every corpus run; system and slice ran the saved binary). **After
the rebase** onto `6f64d545e` the rebased tip's binary (`b3e23340…`) wrote all
three reports byte for byte as the pre-rebase AFTER (corpus `7a70c7b7…`, system
`01df2f02…`, slice `71812a5f…`; verdict rows `f08d6ce0…`, `c9c9bd07…`,
`badf903f…`), as CG16's custody identity predicts, so every figure below holds
on the rebased tip; the census tables were not run again. `cargo check
--workspace --all-targets` 0, the conflict-marker grep empty; the card-lines
audit (`audit_card_lines.sh 84ec54d1d 848e100eb`) listed only the STATUS rows
and the increment paragraph, which conflicted with CG16's records and were
re-applied on main's text (`1d4ed0bc7`); `contract_loader.rs` merged to
`(61, 365, 221)`; `pin --check` per crate 0 at both trees again.

| route | BEFORE | AFTER | report / verdict rows |
| --- | --- | --- | --- |
| corpus (`66910c10`) | 881 / 9 / 3,944 | **905 / 9 / 3,920** | `ab8f5f14…` / `321a8913…` → `7a70c7b7…` / `f08d6ce0…` |
| system | 87 / 4 / 357 | **89 / 4 / 355** | `98066723…` → `01df2f02…` |
| tail-pause slice | 45 / 1 / 354, 4 Pending | unchanged statuses | `8c37c395…` → `71812a5f…` |

**Corpus: +24 Unknown → Safe, no other status move** ([compare-corpus.json](artifacts/compare-corpus.json),
`scripts/compare.py` with both parity streams): 14a 8
(`rd_kafka_broker_outbufs_space`, `calculate_share_acknowledgements_send_rate`,
`_bytes_consumed_rate`, `_fetch_rate`, `_heartbeat_rate`,
`rd_kafka_cgrp_set_last_err`, `rd_kafka_cgrp_update_session_timeout`,
`rd_kafka_msgbatch_set_first_msg`); 14b 1 (`update_offset`); 14c 7
(`metric_hist_exchange` in `cmt_metric.c` and `cmt_metric_histogram.c`,
`cmt_metric_hist_get_value`, `summary_quantile_exchange`; through the summaries
they validate `cmt_metric_hist_inc`, `cmt_metric_hist_set`,
`cmt_metric_hist_bucket_inc`); 14d 2 (`finalize_checksum`,
`rd_segment_write_update`); the two declarations 6
(`cio_file_native_compose_path`, `cio_file_native_delete_by_path`,
`initialize_resource_spans`, `initialize_scope_spans`, `initialize_spans`,
`otlp_kvpair_list_initialize`). **No Safe → non-Safe move, no correction; no
leak or violation added** (leak leaves 40 → 40, `Unsafe` 9 → 9,
`SummaryConflict` 2 → 2).

**Leaves** (266 moving rows): access −302 / +8; uncontracted call −21 / +3;
unresolved identity −3 / **+131**; interior-borrow-ended −2; branch condition
−1; `nullable resource used by a non-null contract` +2; publication of an
object not held live +2. The +131 are rule 14b's aliases (and roots the newly
read actuals expose) dereferenced later: target-free and possibly null, as a
pointer read binds them, so `append`, `peek`, `sink_peek`,
`rd_kafka_share_ack_type_from_msg_op` and `mk_sched_broadcast_signal` move
from the gate's refusal at the declaration to unresolved identities at the
uses (**re-attribution**, 74 functions where 14b read a statement stay
Unknown); `header_lookup`'s publication of the alias by `mk_list_add` is
refused (not one live object). The two `nullable resource` leaves are in
`ctr_kvlist_to_otlp_kvpair_list` and `set_resource_spans`, whose callees
(`otlp_kvpair_list_initialize`, `initialize_resource_spans`) now validate and
whose summaries require a non-null formal the callers hand possibly null: an
honest new refusal, still Unknown. In `rd_kafka_topic_partition_list_update`
the two interior-borrow-ended leaves at the loop's `d = &dst->elems[i]` go:
the refused statement marked every name it might write, and `d_priv` /
`s_priv` are re-derived from the current `d` / `s` before every use
(`rdkafka_partition.c:4544-4548`), so no use follows a parent change; the
function stays Unknown. Three moving rows carry no label: rule 12k now reads
an actual 14c/14d reads and names the uncontracted callee
(`cmt_mpack_consume_double_tag` twice, `rd_hexdump`).

**Readings** ([readings-corpus.json](artifacts/readings-corpus.json), deduplicated
by function, line and tokens): 14a 131 admitted / 1,135 declined (a matched
shape whose call is not a `Borrowed`-only `None`-result contract or does not
lower), 14b 89 admitted, 19 element-address and 9 interior-offset actuals the
base never read.

**System +2** (`flb_engine_adaptive_flush_init` in two configurations, 14a:
`config->flush_adaptive_current_interval = flb_engine_adaptive_flush_interval(config, config->flush_adaptive_level)`);
14 moving rows. **Slice**: no status move, 12 moving rows (leaves).

**Parity stream, in full** ([parity-corpus.json](artifacts/parity-corpus.json),
`scripts/parity_diff.py`, multisets with node addresses erased): 683,320 →
691,720 rows, 681,100 identical. Every function with a row only in BEFORE is a
function that moved or in which a rule of this card was consulted, except 21
unit-level `summary-premises-graph` rows (the graph changes where summaries
newly validate). The rows only in AFTER are the rules' own (`residual-access`
5,828, `call-argument` 992, `typed-place` 1,216) and the readings the
newly read statements reach. System and slice: [parity-system.json](artifacts/parity-system.json),
[parity-slice.json](artifacts/parity-slice.json).

**Safe roll-up** (the sidecar's `contract_dependencies`): corpus **418 none /
94 → 101 only validated / 23 partly / 346 → 363 declared assumption**;
declarations consulted 330 → 332, validated 54 → 57 (`cio_errno_print`,
`ctr_errno_print`, and `cmt_metric_hist_get_value`, whose body now validates
through 14c), declared assumptions 244 → 243. Of the 24 new Safe rows, 7 rest
only on validated declarations (the cmetrics atomics) and **17 on at least one
declared assumption, none introduced by this card** (`__errno_location`,
`calloc`, `strlen`, `memcpy`, `__assert_fail`, `htonl`, `clock_gettime`,
`rd_atomic32_get`, `rd_atomic64_get`, `free`, `malloc`, `snprintf`,
`unlink`). System 47 / 2 / 4 / 34 → 49 / 2 / 4 / 34; slice unchanged.

**Prediction against outcome.** Predicted +16 to +26, measured **+24**: lift A
(14a) 8 of 9 (`rd_kafka_broker_weight_usable` holds a `?:`, which the reader
does not read); lift B (14b) 1 of 7 predicted 4–7, below the range: the alias a
pointer read binds is possibly null, so the dereferences after it refuse
(re-attribution, above); C 6 of 6 (contracts only); D (14c) 4 direct of 4,
plus 3 summaries (predicted 0–4 in all); 14d +2, not predicted. Access-alone
population predicted 200–212, measured **201**. System +2 (0–2 predicted),
slice 0. New Safe rows on declared assumptions this card introduced: 0
(predicted 4–6: both declarations validate).

## 4. Residual (amends `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS`)

On the AFTER census run ([residual-after3.json](artifacts/residual-after3.json),
[rank-after3.json](artifacts/rank-after3.json)): **201 functions, 333 leaves**.

| class | alone | leaves | owner / reason left |
| --- | ---: | ---: | --- |
| uncontracted callee, an actual neither reader recognises | 37 | 87 | needs a contract **and** a reading; mostly decision-class callees (`rd_kafka_log0`, `rd_kafka_timer_start0`, `rd_kafka_interceptor_method_add`, `LZ4F_free`) |
| whole-call return, the same | 24 | 25 | as above |
| `custody-position:released` | 13 | 52 | by design (W44) |
| `pointer-store:other` | 12 | 15 | rule 10v's identity-carrying values (Q36 owner premise) |
| `pointer-store:formal-pointer` | 11 | 16 | a formal stored into another object: class-1 `retained_by` |
| `probe-declined` | 6 | 11 | owned results and a descriptor stored into members (field-link model), two calls in one value, a `?:` actual |
| `unclassified:CallArgument` | 6 | 6 | `destroy(arr[i])`: a release of a caller's array element (rule 5k excludes it by design) |
| whole-call return, no rule-12k reading | 5 | 5 | |
| `call-result-store`, elaboration no value form, `pointer-store:arithmetic` | 4 each | | one shape each (a `?:` operand, a `void *` bit-cast, a multi-declarator, a union function-pointer store, a sibling-member offset) |
| 26 smaller classes | 35 | | each at most 3 alone |
| two or more classes | 40 functions | | |

Registered: **`CG10-INTERIOR-ALIAS-NULLABILITY`** (follow-ups 740 → 741): 7
Unknown functions are blocked only by dereferences of a rule-14b alias, which
is possibly null as a pointer read is (`append`, `peek`, `sink_peek`,
`rd_kafka_share_ack_type_from_msg_op`,
`rd_kafka_topic_partition_list_has_duplicates`,
`rd_kafka_topic_partition_list_regex_cnt`, `mk_vhost_fdt_chain_lookup`;
`scripts/alias_uses.py`, [alias-uses.json](artifacts/alias-uses.json)); a
non-null premise for `&B->F` / `&B->F[I]` under a dereferenced `B` is the
question, not taken here ("handle as safe as possible").

Every remaining fix frees at most 3 functions alone, and every class above 3 is
a design refusal, a decision class or a contract question: the card's stopping
rule (yield about 1 per lift) holds. Fixed point in rank order 37, 61, 86, 99,
110, 117, ….

**`W45-CALLEE-CASCADE-RESIDUAL`** ([callee-rank-before.json](artifacts/callee-rank-before.json),
[callee-rank-after3.json](artifacts/callee-rank-after3.json), its `rank.py`
unchanged): Unknown functions blocked by uncontracted calls alone **523 → 537**,
by one callee alone 366 → 379; the 14 functions the lifts left behind a callee
are re-attributions: `rd_kafka_log0` 3 (a configured logging callback),
`cfl_kvlist_create`, `ctr_id_set`, `msync`, six
`opentelemetry__proto__*__init`, `rd_kafka_topic_partition_ensure_toppar` 1
each. No contract was written for them: none is trivial (a callback, an owned
constructor, protobuf-c initialisers outside any provider crate).

## 5. Gate

Standing gate (`docs/agents/validation.md`) on the rebased tip,
`scripts/gate.sh g3` then `scripts/write_gate_json.py g3 pg2` and
`scripts/finish-gate.sh g3 pg2`; logs `private/cg10r2/gate-g3/`, exits copied
to [gate/](gate/), summary [gate.json](gate.json) (code steps at `1d4ed0bc7`; later commits change records only):

| step | exit | result |
| --- | ---: | --- |
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 467 units, 480 summaries; 7,785 passed, 0 failed, 12 ignored (`private/cg10r2/tests-gate-g3`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 306 audits passed |
| `python3 -m unittest discover -s scripts/tests` (before the matrix) | 1 | the expected stale `lean_audits` (305 committed, 306 produced) |
| `build_docs_index.py --check` (before the records' index) | 0 | |
| `build_matrix.py . --gate gate.json` | 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK |
| `build_docs_index.py`, `--check` | 0, 0 | | A second
pre-rebase gate, `gate-g2`, was stopped during its test step when main moved
(steps 00–03 exit 0) and is not evidence.

**Provider gate** `pg2` on the rebased tip (main's chunkio contracts beside
this card's; `scripts/provider-gate.sh`): `cargo check --locked` 0, `cargo test` 129 passed, fmt 0, clippy 0, `providers/contracts.py check` 0, package build 0 and `verify` 0 for all five packages (chunkio, cmetrics, ctraces, libevent, librdkafka), providers Python suite 25 tests OK. The pre-rebase
`pg1` (at `80f91076a`) passed every step too: 129 tests, five packages
verified, 25 providers Python tests ([gate/provider/](gate/provider/) holds
`pg2`'s exits). Targeted runs before the gates: the new
controls (`t-int4`), the contract pins (`t-pins3`), all under
`private/cg10r2/`.

**Pins flipped deliberately.** The first gate, `gate-g1` (code at
`80f91076a`), failed exactly two gateway pins (7,767 passed, 2 failed; every
other step 0, 304 Lean audits): `sub_object_stores`'s
`r->count = x_len(s)` (a `None`-result, all-`Borrowed` call stored into a
scalar member) is read by rule 14a and is now `Safe`, with a new control that
two calls keep the refusal; `neighbours`' control that a pointer-arithmetic
tail actual keeps the constructor's refusal (`storage->stream->name + 0`) is
read by rule 14d, which the test now asserts, and the refused control is `*&`
over the same path (`f26c53cd7`). Contract pins, before any gate:
`compiler/tests/contract_loader.rs` 363 → 365 v1 declarations and two
previous-mode rows, `compiler/tests/qcore_contract_corpus.rs` two clause rows.

## Files

* `crates/qcore-c-gateway/src/legacy_residual_access.rs` (rules 14a, 14b;
  hook in `legacy_c.rs::expression_statement`), `residual_access_reading.rs`
  (the Clang readings of 14a, 14b, 14d), `scalar_element_address.rs` (14c; hook
  in `typed_position_shapes.rs::address`), the 14d hook in
  `typed_access_positions.rs::argument`; shared items `Reader::opaque_call`
  and `StatementIndex::statement_node` in `typed_access_path.rs`.
* `proof/QCore/ResidualAccessReadings.lean`, its audit, `proof/QCore.lean`.
* Contracts: `providers/qcore-provider-chunkio/contracts/chunkio/chunk_residency.qv`,
  `providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv`, their
  `contracts.json` and `contracts.lock`.
* Tests: `crates/qcore-c-gateway/tests/indirect_access_sole_residuals_r2.rs`,
  fixture `tests/fixtures/contracts/residual_r2.qv`; pins in
  `compiler/tests/contract_loader.rs` (363 → 365 v1 declarations, two
  previous-mode rows) and `compiler/tests/qcore_contract_corpus.rs` (two clause
  rows).
* Scripts: `build.sh`, `measure.sh`, `measure-binary.sh`, `census-run.sh`,
  `heavy.sh`, `classify.py`, `residual_table.py`, `leaf_detail.py`, `rank.py`,
  `compare.py`, `readings.py`, `parity_diff.py`, `callee_rank.py`,
  `alias_uses.py`, `update_ledger.py`, `totals.py`,
  `identity.py`, `verdict_projection.py`, `attribution.py`, `decides.py`,
  `class_leaves.py`, `sync.sh`, `pin.sh`, `gate.sh`, `provider-gate.sh`,
  `finish-gate.sh`, `write_gate_json.py`.
