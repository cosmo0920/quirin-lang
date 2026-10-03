# CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: the 513 functions the access refusal blocks alone

Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (wave 48, round 17; the top-priority CG
residual). Base: local `main` at `cdc2fa878`, which it stayed at. Gateway
[rule 12i](../../agents/gateway.md#constraint-12i) (a refused `return`'s one
call read with rule W44) and [rule 12j](../../agents/gateway.md#constraint-12j)
(`.`-only scalar places on automatic record objects), amending rule 10t's
record-object root, C-GATEWAY-RETURN-CALL-W4 and B01. Lean
`QCore.ReturnCallActuals` (13 theorems) and `QCore.RecordObjectPlace` (12),
audited by `proof/AuditReturnCallActuals.lean` and
`proof/AuditRecordObjectPlace.lean` (284 → 286 audits).
[PREDICTION.md](PREDICTION.md) was frozen at `c6613c89b`, after the
classification and before any implementation code (`a90825c3c`).

## 1. The classification (task 1)

The integrator's ranking reproduced exactly on the BEFORE corpus run
(`d13ed4c1…`, 654 / 9 / 4,171): **513** Unknown functions whose only refusal
family is `indirect memory access is outside the resource projection` (977
leaves); the family touches 2,971 functions and 13,283 leaves.

The leaves were classified by the readers' own records, not by spelling
(`scripts/classify.py`): the measurement-only `indirect-census` feature's
streams (the lexical gate's record of each refusal, rule W44's per-actual
record of each call it was consulted for — the contract position
`Position::of`, the recogniser's answer, the probe's outcome — and the return
census), produced by `scripts/census-run.sh`, whose corpus report is
byte-identical to the measured binary's (`d13ed4c1…`), and the typed readers'
parity rows (`elaboration-statement`, `typed-place`, `pointer-store`,
`pointer-store-flow`). The table is PREDICTION.md's; its
[artifact](artifacts/classify-before.json) lists every leaf with its class.

The ranking's premise does not hold for 321 of the 513 (**a correction**): 218
call statements and 103 whole-call returns name a callee with no contract or
summary in force, and the lexical gate refuses the statement before the
call's contract is looked up, so the access refusal hides an uncontracted call.
Lifting the access frees none of them (each would read `call to uncontracted
function`); they belong to the P4 callee closure. Among the callees: header
inlines the corpus does not list as functions (`rd_avg_add`, `rd_atomic64_get`,
`cfl_sds_create`), Unknown librdkafka bodies (`rd_kafka_log0`, `rd_list_destroy`)
and cross-unit Safe functions whose summaries are not imported
(`cmt_atomic_store`, `cmt_atomic_load`). The largest sub-classes this card
could lift soundly were therefore the 15 `return-contracted-call` functions
(W44 is never consulted at the return position) and the 19 record-object
functions (13 the gate's token view parses, 6 with a parenthesised root read by
hand). The CG10 root-premise residuals (`CG10-BOUND-ROOT-PATH-REFUSED`,
`CG10-BOUND-ROOT-NULLABLE-OR-TRACKED`) are not among the sole blockers: their
leaves are `unresolved resource identity` refusals, a different family, so no
function in the 513 carries one. CG11's unresolved-owner stores
(`CG11-UNRESOLVED-OWNER-POINTER-STORES`, whose failing premise restores the
gate's refusal byte for byte) appear in 3 of them (8 leaves), never alone; rule
10v's refused values (`pointer-store:*`) block 24 alone.

Classification limit: leaves are keyed by (function, internal line); a macro
line holding several statements is classified by its last census row. The six
parenthesised-root telemetry functions counted by hand as record-object places
(`calculate_produce_latency_max`, `calculate_throttle_max`, …) expand
`brokers_max` on that line, whose loop over the brokers holds another refused
statement the census row did not name; they stay Unknown (§3).

## 2. The lifts

**Rule 12i** (`legacy_return_call_access.rs`, hook in `return_statement`,
asked last). `return f(args);` is decided by classifying `PLACE = f(args)`
(C-GATEWAY-RETURN-CALL-W4) and B01's `return E(f(args))` by classifying the
call; the gate refuses both when an actual reads memory, and the return
position never asked rule W44. The rule asks it with exactly the statement the
return position classified: W44's whole decision is reused (an actual at a
custody-taking position, an uncontracted callee, a descriptor read or a
declined probe keeps the refusal byte for byte; a declined probe restores the
site and element counters), and so is the return position's acceptance
(`escapes_only_fresh_custody`; B01's form only a `None` result). The lowering
is W44's block — the typed reads, their second obligation, the call — then the
existing exit. Lean: `the_lowering_is_the_statement_lowering_then_the_exit`,
`the_reads_precede_the_call_which_precedes_the_exit`,
`a_statement_w44_refuses_keeps_the_refusal` (and the uncontracted, descriptor
and declined-probe instances), `a_custody_position_is_never_used`,
`a_reallocated_result_keeps_the_refusal`, `the_scalar_form_returns_no_custody`,
`the_exit_discharges_nothing_prior` (composing
`QCore.ReturnCallCustody.admitted_modes_discharge_nothing_prior`), and three
examples (`strcmp(a->name, b->name)` admitted, a `Released` actual refused).

**Rule 12j** (`record_object_place.rs`, hook in `Reader::path`). The typed
reader admitted a `.`-rooted place only on a root the custody closure proves
never bound; a record object whose address reaches `memset`, whose member name
is spelled in an uncontracted call's actual (the lexical closure reads
spellings: `heartbeat_total` is also `rk->rk_telemetry.heartbeat_total`), or
whose pointer member receives a store was refused at every scalar member
access. Where the never-bound premise refused, a place is admitted when every
step is a `.` member selection (rule 10w's union and bit-field steps
included, with their decision and frame), the root's Clang declaration is a
`ParmVarDecl` or a block-scope `VarDecl` of this function with no
storage-class specifier and no thread-local storage, and the access is a
scalar read or store. It lowers to no base and no chain: no pointer is read,
so there is no dereference obligation, and the store writes only the scalar
member's storage, which no binding names. Lean: `decision_vectors_hold`
(bound to `DECISION_VECTORS`),
`admitted_exactly_on_a_dot_only_scalar_place_of_automatic_storage`,
`a_hop_is_never_admitted`, `a_root_outside_automatic_storage_is_refused`,
`a_non_scalar_access_is_refused`, `the_never_bound_answer_is_unchanged`,
`the_rule_only_adds`, `an_admitted_place_has_no_obligation`,
`a_scalar_read_changes_no_binding`, `a_scalar_store_changes_no_binding`,
`a_union_store_keeps_the_frame`, `the_frame_adds_no_target`.

Tests (`tests/indirect_access_sole_blockers.rs`, 13, each admission with a
refusal control; `tests/unit/record_object_place.rs`, 4): rule 12i admits
`return is_cmp(a->name, b->name)`, an owned `return is_dup(a->name)` and B01's
`return 1 + is_count(a->name)`, and keeps the refusal for a `Released` actual,
an uncontracted callee and a released root read at the return; rule 12j admits
scalar stores into an addressed automatic record, through a custody-free union
member and into a by-value parameter (each refused with the rule disabled,
checked once by hand), and keeps the refusal for a `static` and a file-scope
record, a hop (`o.next->a`) and a pointer value read (`is_count(o.name)`).

**Pins flipped deliberately** (rule 12i; the gate's first run, `gate-g1`,
failed exactly these four rows, 7,546 passed / 4 failed):
`neutral_expression_positions`'s `return digits10(*slot);` and
`return digits10(table[n]);` rows asserted `Unknown` because the return
position never asked rule W44; they now assert `Safe` beside their statement
forms (`unsigned int t = digits10(*slot); return t;`), which neither of the
card's hooks can move — the ground card C-GATEWAY-RETURN-CALL-W4 used for the
pointer-arithmetic row beside them (renamed
`a_dereference_in_a_call_actual_agrees_with_the_statement_position` and
`a_subscript_in_a_call_actual_agrees_with_the_statement_position`).
`summary_validation_premises`' `get(const void *_p) { const struct c_tp *p =
_p; return c_same(p->topic, p->topic); }` row and its cascade asserted
`get` Unknown and its callers withdrawn; `get`'s own row now reads those
actuals as its statement form does (`int r = c_same(…); return r;`, Safe with
the same summaries, asserted beside it), so rule 4b validates the summary on
the premises its row reads, the ground CG06 used for the comparator row of the
same file. The cascade keeps a negative control: with a `?:` actual, which
neither W44 nor rule 12i reads, `get` stays Unknown and `mid` and `w` keep
their uncontracted-call refusals.

## 3. Measurement

| route | BEFORE | AFTER | digest |
| --- | --- | --- | --- |
| corpus (`66910c10`) | 654 / 9 / 4,171 | **695 / 9 / 4,130** | `d13ed4c1…` → `d2f5a3f7…` |
| system | 83 / 4 / 361 | **85 / 4 / 359** | `68cca540…` → `60bac16e…` |
| tail-pause slice | 45 / 1 / 354, 4 Pending | unchanged statuses | `2222e03d…` → `62ba8c6d…` |

**Corpus: +41 Unknown → Safe, no other status move** (200 rows move; every
move in [artifacts/compare-corpus.json](artifacts/compare-corpus.json)):

* rule 12i directly, 14: `cmt_opts_compare`, `ctr_id_get_len`,
  `mk_server_lib_notify_started`, `rd_kafka_consumer_closed`,
  `rd_kafka_admin_timeout_remains`, `rd_kafka_admin_timeout_remains_us`,
  `rd_kafka_broker_filter_never_connected`, `rd_kafka_interceptor_method_cmp`,
  `rd_kafka_metadata_cache_entry_cmp`, `rd_kafka_plugin_cmp`,
  `rd_kafka_txn_current_timeout`, `rd_strcmp3`, `XXH_readLE32_align`,
  `XXH_readLE64_align`;
* rule 12j directly, 15: twelve librdkafka telemetry value builders
  (`calculate_broker_avg_rtt`, `calculate_broker_max_rtt`,
  `calculate_consumer_assigned_partitions`, `…_commit_latency_avg/_max`,
  `…_poll_idle_ratio_avg`, `…_rebalance_latency_avg/_max/_total`,
  `calculate_share_consumer_poll_idle_ratio_avg`,
  `calculate_share_time_between_poll_avg/_max`), `XXH32_reset`, `XXH64_reset`,
  `cnd_timedwait_ms`;
* callee summaries the rules validate, 12: `XXH_readLE32`, `XXH_readLE64`
  (`XXH_readLE*_align`), `cnd_timedwait_abs` and `cnd_timedwait_msp`
  (`cnd_timedwait_ms`), the six cmetrics `*_lookup` (`cmt_opts_compare` in
  their branch condition), `rd_kafka_share_consumer_closed`
  (`rd_kafka_consumer_closed`) and `rd_kafka_broker_needs_connection`
  (the header inline `rd_kafka_fatal_error_code`, whose
  `return rd_atomic32_get(&rk->rk_fatal.err)` rule 12i reads). The script's
  `callee-summary` label names 3; the other 9 lost leaves (six branch conditions,
  two returns, one statement) name no callee and were attributed by reading
  their source lines.

**No Safe → non-Safe move, no correction, no leak or violation added**
(`leak` 40 → 40). Leaves: indirect-access −231, branch-condition −105, nested
call −40, uncontracted call −13, unresolved identity −10 / +80 (the newly read
places' roots, in functions that stay Unknown), return-expression −9, and +3
``nullable resource `__qcore_returned_value` used by a non-null contract`` in
`mk_vhost_fdt_open` and `cmt_metric_name_from_labels`: `return
open(sr->real_path.data, …)` and `return strdup(ts->labels[i]->value)` now
reach the return position's existing exit, which refuses a nullable owned
result exactly as it does for the same call without a member actual; both
functions were and stay Unknown. Rule 12i was consulted 317 times (307
whole-call returns, 10 B01 returns; 41 admitted), rule 12j admitted 544 place
decisions ([artifacts/place-decisions.json](artifacts/place-decisions.json)).

The sole-blocker population moves 513 → 481 (27 of the 513 freed: 13
record-object, 12 return-call, 1 mixed, and `rd_kafka_share_consumer_closed`
through its callee's summary; 14 freed functions were outside it); the family
touches 2,918 functions and 13,057 leaves (−53 / −226).

**Prediction against outcome.** Predicted +23 to +38; measured **+41**, above
the range by 3. Rule 12i's direct gains (14, predicted 9–15) and rule 12j's (15,
predicted 14–19) are inside their ranges; the callers freed by validated
summaries (12) exceeded the 0–6 estimate (six cmetrics lookups share one
comparator; header inlines carry rule 12i into every including unit). Six of
the 19 predicted record-object functions stay Unknown on a macro line's other
statement (§1). No Safe → non-Safe, as predicted. System +2 (predicted 0 to
+3), slice no status move (8 rows change leaves, 7 by rule 12j).

**`mk_vhost_set` at the tip** (`scripts/tip-probe.sh`, contracts-only, this
card's binary and the BEFORE binary): unchanged. On this base the tip's only
refusal is ``call to uncontracted function `mk_vhost_set_property` ``, and
`mk_vhost_set_property` keeps its two leaves at `mk_lib.c:502`–`503`:
`vh->documentroot.data = mk_string_dup(v)` (an owned result stored into a
member, the DocumentRoot custody question — real behaviour, refused) and
`vh->documentroot.len = strlen(v)` (a scalar call result stored through a
pointer root: the `call-result-store` class, residual). Master keeps its
`va_end` protocol refusals.

## 4. Residual (registered as `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS`)

On the AFTER census run (`scripts/census-run.sh census-after1`, byte-identical to the measured AFTER, `d2f5a3f7…`): **481** functions, 916 leaves ([artifact](artifacts/classify-after.json)).

| sub-class | freed alone | leaves | touched | owner |
| --- | ---: | ---: | ---: | --- |
| `uncontracted-callee` | 227 | 578 | 285 | P4 callee closure (`W45-CALLEE-CASCADE-RESIDUAL`, `W44-CALLEE-*`) |
| `return-uncontracted-callee` | 102 | 107 | 104 | P4 callee closure |
| `pointer-store:other` | 12 | 16 | 16 | rule 10v value classes (CG11): a cast local, an interior address, … |
| `custody-position:released` | 12 | 43 | 26 | `W44-CALL-ARGUMENT-CUSTODY-POSITION` (by design); rule 5k's member release |
| `elaboration:unlisted form: no value form of this card` | 10 | 17 | 16 | mixed: pointer arithmetic values, element addresses `&s->iov[i]`, multi-declarator aliases |
| `pointer-store:formal-pointer` | 9 | 23 | 20 | rule 10v (Q36: an owner identity for `*s`, or a Stored summary mode) |
| `probe-declined` | 6 | 13 | 12 | owned results stored into members (the field-link model) |
| `unrecognised-actual:other` | 5 | 6 | 6 | W44 recogniser |
| `call-result-store` | 4 | 20 | 17 | scalar call results stored through a pointer root (unread) |
| 15 smaller classes | 23 | — | — | `return-contracted-call` 2 (a by-value record actual, a `?:` actual, a cast actual), `place:storage` 1 (a file-scope record, refused by rule 12j), … |

Fixed point in rank order: 227, 331, 345, 369, 383, 398, 408, 413, 430, 433, …. The uncontracted-callee class grew 218 → 227 alone: nine functions whose other blocker the card read (telemetry builders calling the header inline `rd_atomic64_get` among them) are now blocked by their callee only. Not lifted, each by name: the callee closure (not an access question), custody positions (by design), rule 10v's refused values and owned results stored into members (custody questions with a missing owner premise), and the record-object pointer stores of `pb_ostream_from_buffer` (`stream.callback = &buf_write`, `stream.errmsg = NULL`: identity-free values into a record object, which rule 10v's owner premise reads only when never bound; the function also stores a formal's pointer, so that extension frees nothing).

## 5. Gate

Standing gate (`docs/agents/validation.md`), `scripts/gate.sh g2` then
`scripts/finish-gate.sh g2`, code steps at `799dacf1a` on base `cdc2fa878`;
logs `private/cg10sole/gate-g2/`, exits copied to [gate/](gate/), summary
[gate.json](gate.json):

| step | exit | result |
| --- | ---: | --- |
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 439 units, 452 summaries; 7,550 passed, 0 failed, 11 ignored (`private/test-binaries/20261003T103104Z-3855597`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 286 audits passed |
| `python3 -m unittest discover -s scripts/tests` (before the matrix) | 1 | the expected stale `lean_audits` (284 committed, 286 produced) |
| `build_matrix.py . --gate gate.json` | 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK |
| `build_docs_index.py`, `--check` | 0, 0 | |

The first run (`gate-g1`, code at `e1e15b64d`) failed exactly the four pins
§2 flips (7,546 passed, 4 failed); every other step exited 0. Provider gate
not run: `providers/` is unchanged by the card (and since CAST-FORMALIZATION's
provider gate over the same tree). Targeted runs: `scripts/targeted.sh t1`
(the card's unit test and nine neighbouring suites, all 0 but the card's
first draft) and `scripts/pins.sh p2` (the flipped suites and the card's,
all 0).

## Files

* `crates/qcore-c-gateway/src/legacy_return_call_access.rs`,
  `crates/qcore-c-gateway/src/record_object_place.rs` (new); hooks in
  `legacy_c.rs` (`return_statement`, the module declaration),
  `typed_access_path.rs` (`Reader::path`), `typed_place.rs`
  (`record_place_row`), `lib.rs`.
* `proof/QCore/ReturnCallActuals.lean`, `proof/QCore/RecordObjectPlace.lean`,
  their audits, `proof/QCore.lean`.
* Scripts: `classify.py` (task 1), `census-run.sh`, `measure.sh`,
  `compare.py`, `place_decisions.py`, `tip-probe.sh`/`tip_rows.py`,
  `targeted.sh`, `pins.sh`, `build.sh`, `gate.sh`, `finish-gate.sh`,
  `update_ledger.py`, `update_records.py`, `provider-gate.sh` (not run: no
  `providers/` change).
