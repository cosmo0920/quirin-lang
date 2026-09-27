# W47-ACCESS-PATH-POSITIONS

Plan P3 of the [unsupported-coverage queue](../../plans/unsupported-coverage/README.md),
discharging `W46-ACCESS-PATH-POSITIONS`. Base `2e44e963` (local `main`,
"Record passing final matrix and reconciliation checks"). The prediction was
frozen before any implementation in [PREDICTION.md](PREDICTION.md) (commit
`f9b20080`). Gateway rule [10k](../../agents/gateway.md#constraint-10k) states
the rule; `QCore.TypedAccessPathPositions` is its Lean model.

## Mechanism

P2's Clang-typed reader (`typed_access_path.rs`), with every refusal it has,
is applied in three more positions, each **after** every older route of the
position refused (hard invariant 3: no route is retired, none consulted later).
A sub-expression is matched to exactly one Clang node by its internal line and
exact tokens (`typed_access_positions.rs`); the hooks are in
`legacy_typed_positions.rs`, with thin call sites in `legacy_c.rs`,
`legacy_condition_desugar.rs` and `legacy_call_argument_access.rs`.

- **Conditions.** A call-free operand the older routes refuse is read as a
  scalar value, a pointer read's truthiness, or a comparison of pointer
  operands (path reads, names, null constants), and lowered to the typed read
  with no destination and no frame: the per-hop obligations are checked at the
  operand's node, before either edge. Operands are wired by the W44
  desugaring (rule 5h), so the operand of `&&`, `||` or `?:` C does not
  evaluate on a path contributes no obligation there, and `p && p->x` reaches
  `p->x` only on the edge the existing null branch refined. A refused
  call-free condition is desugared only when an operand needed the typed
  reader; call-bearing conditions gain typed call-free operands. One
  `NAME = PATH` operand (a declared, non-formal, non-descriptor local) is P2's
  statement followed by the existing null or neutral decision on `NAME`.
- **Returns.** One scalar value, or one pointer read whose every root the
  custody closure proves never bound **and** `may_bind` does not name (rule
  5b's premise), with a `NoOp` exit: nothing escapes or is handed back. An
  array member returned by decay is an interior address (rule 5j) and refuses.
- **Call actuals.** An actual the lexical recogniser refused joins the W44
  argument probe only at a `Borrowed` or declared data-tail position of a
  callee whose contract or validated summary is in force; a pointer-valued
  actual needs never-bound roots and keeps `ArgumentReads`; its chains keep
  their per-link obligations (`ArgumentRead::chains`, new).
- **Enum arm (step B, measured separately).** `enum TAG` for a declared tag,
  or a typedef whose own declaration is directly an `EnumType`, is a scalar.
  Anonymous aggregate, union and pointer-to-enum typedefs keep the refusal.
- **Parity.** `QCORE_TYPED_PATH_PARITY` rows now carry a `position` key
  (`condition`, `condition-assignment`, `return`, `call-argument`): every typed
  fallback answer, and the typed candidate beside every condition-access,
  return-family and lexical call-argument admission. Rows without the key are
  P2's statement rows. No parity is claimed.

A performance repair (`53fcf116`) finds a node's line by binary search over
the source's newlines; the answer is the same number. Before it the first
step-A corpus attempt ran 94 minutes; after it 4.3 minutes.

## Measurement

Commands: `run_audit.py OUT COMMIT --jobs 16` (corpus) and
`w46-contract-gap-expansion/scripts/run_system.py OUT --jobs 16` (system at
`046accf6`), each with `QCORE_TYPED_PATH_PARITY` inside its output directory,
through `private/w47/tools/{measure,build_and_measure,corpus_only}.sh`. Private
runs are under `private/w47/` of this worktree.

**BEFORE** (base binary, `private/w47/before/`): exit 0 in both scopes. Corpus
diagnostics, chunk contracts and Monkey source are byte-identical to the W46
combined archive; contracts are identical after the checkout-root alias;
source effects differ only in path-derived digests (resource-contract ids,
build/configuration/task digests), every structural field equal. The sorted
parity stream (25,432 rows) and its contexts are byte-identical. The system
diagnostics differ only in the same path-derived digests (comparison: 0
changed rows), chunk contracts, source effects and sorted parity are
byte-identical ([identity](artifacts/before-identity/)). The system run
records `qcore_revision` `1257520b` because it started after that commit; its
binary was built at the base before any source change.

**Failed attempts, retained and discarded as evidence.** Three corpus runs
ended with SIGTERM (`exit: -15`) from outside this card, on a host running other
sessions' audits with swap exhausted: BEFORE attempt 1 after 5 minutes, step-A
attempt 1 after 90 seconds, step-A attempt 2 (`1257520b`, before the line-index
repair) after 94 minutes. Their directories are
`private/w47/{before-attempt1-sigterm,after-a/corpus-attempt1-sigterm,after-a/corpus}`.
The first step-A system run (`private/w47/after-a/system`, binary `1257520b`)
completed; the step-A figures below come from the later checkpoint.

**Step A** (positions, enum arm off): measurement checkpoint `72852495`
(`53fcf116` with `ENUM_ARM = false`), [archive](artifacts/step-a/).
**AFTER** (step B, final code): `9b2ac682`, [archive](artifacts/after/)
(manifests give raw and sorted-row SHA-256s). Both exit 0 in both scopes.

| Measure | BEFORE | Step A | AFTER |
|---|---:|---:|---:|
| Corpus functions Safe / provisional Unsafe / Unknown | 447 / 9 / 4,378 | 494 / 9 / 4,331 | **507 / 9 / 4,318** |
| Corpus units Safe / Unsafe / Unknown / data-only | 6 / 8 / 167 / 4 | same | same |
| Corpus unsupported / unknown_alias / leak | 39,085 / 4,760 / 34 | 37,111 / 6,433 / 34 | **36,262 / 7,440 / 34** |
| COND / RET / IND / UNC / OTHER leaves | 8,607 / 305 / 15,536 / 8,957 / 5,680 | 6,788 / 292 / 15,414 / 8,931 / 5,686 | 6,369 / 284 / 15,037 / 8,902 / 5,670 |
| System configuration rows Safe / Unsafe / Unknown | 56 / 4 / 388 | 63 / 4 / 381 | **63 / 4 / 381** |
| System unique functions (+3 excluded) | 29 / 2 / 203 | 33 / 2 / 199 | **33 / 2 / 199** |
| System unsupported / unknown_alias / counted_user | 3,069 / 892 / 4 | 2,908 / 985 / 4 | 2,908 / 985 / 4 |

All 60 corpus and 7 system moves are Unknown → Safe. There is **no Safe →
non-Safe move** (no correction), no new provisional Unsafe, and no added
`leak`, `use_after_release`, `invalid_release`, `double_release` or
`counted_user` leaf in either scope, so there is nothing to classify as real,
artifact or premise gap. The enum arm moves no system row.

**Every move** ([corpus](artifacts/after/corpus-comparison/moves.json),
[system](artifacts/after/system-comparison/moves.json),
`scripts/move_reasons.py`): 55 corpus moves are direct (every removed leaf is
at an admitted W47 position row or a statement row the enum arm newly
admits), 3 direct plus a summary cascade, and 2 pure summary cascades
(`rd_hdr_countsIndexFor`, `rd_hdr_lowestEquivalentValue`: their callee
`rd_hdr_getBucketIndex` became Safe). By library: librdkafka 51, Monkey 6,
ctraces 2, cmetrics 1. In the system slice,
`sb_find_segregated_backlog_by_output_plugin_instance` (`output_plugin ==
backlog->ins`) and `flb_engine_exit` (`config->ch_manager[1]` actual of
`flb_pipe_w`) are direct; `flb_engine_exit_status` follows its now-Safe callee;
`flb_output_check` follows the header-inline `mk_list_is_empty`, whose
`head->next == head` condition is now typed, so its summary validates.

**Every changed leaf** ([corpus](artifacts/after/corpus-comparison/changed-leaves.jsonl.gz),
[system](artifacts/after/system-comparison/changed-leaves.jsonl.gz)) carries
its attribution. Corpus: 2,155 COND, 21 RET and 127 IND removals are at an
admitted position row; 66 COND and 62 IND are on a continuation line of the
same statement; 307 IND and 16 OTHER are at a statement row the enum arm newly
admits; 1,888 added `unknown_alias` leaves are at admitted rows and 349 at
enum-arm statements. Every remaining (non-direct) leaf is classified in
[nondirect.json](artifacts/after/corpus-comparison/nondirect.json)
(`scripts/classify_nondirect.py`, with a reviewed-by-reading table): 445 added
aliases are later obligations in functions with an admitted row (a borrowed or
unresolved root checked later); 55 removed uncontracted-call leaves follow a
callee that became Safe (48) or whose summary now validates (7); 4 condition
refusals disappear because a call-bearing condition now plans; 5 added leaves
exchange a removed refusal on the same line (three `nested call … inside a
modeled call`, one `arguments do not match resource contract for
flb_task_get_route_data`, one alias); 2 later exposed refusals (`nullable
resource buf/fd used by a non-null contract` in `mk_utils_url_decode`,
`mk_vhost_fdt_open`); and 23 leaves read by hand: 16 removals in a
condition or actual whose callee — `check_buckets`, `otel_span_set_kind`,
`rd_kafka_cgrp_rebalance_protocol`, `rd_kafka_Uuid_cmp`, `mk_fifo_queue_get`,
`mk_http_status_completed`, `mk_channel_is_empty`, `mk_list_is_empty` — is now
modelled, 2 later `reply` obligations, and 5 removed aliases no longer
poisoned by a refused condition's write frame (rule 10a). That is all 534
non-direct corpus leaves. The system's 32 non-direct leaves fall in the same
groups (4 callee-now-Safe, 8 cascade conditions, 6 exchanged refusals, 8
callee-now-modelled, 6 aliases no longer needed or poisoned). All stay non-Safe
where they were; none is a new violation. `unknown_alias` rises by 2,680
(corpus) and 93 (system) as expected, each by source.

**Parity rows** (de-duplicated, `scripts/position_rows.py`): in the corpus the
typed fallback admits 3,449 condition, 30 assignment, 56 return and 388
call-actual rows and refuses 4,456, 24, 275 and 4,456; beside older
admissions it agrees to admit 1,270 / 266 / 187 condition-access /
return-family / call-argument rows and refuses 219 / 142 / 463. These are
distinct vocabularies; `W46-ACCESS-PATH-PARITY` stays open.

## Prediction against outcome

Against the frozen per-function forecast ([comparison](artifacts/after/corpus-comparison/comparison.json), `forecast`):

- **Step A, 24 predicted Safe: 12 match, 12 miss.** Ten one-line pointer
  getters (`rd_kafka_messages_get`, `rd_kafka_opaque`, `rd_kafka_ConfigEntry_name`,
  `rd_kafka_Node_host`, `rd_kafka_Node_rack`, `rd_kafka_handle_mock_cluster`,
  `rd_kafka_mock_cluster_bootstraps`, `rd_kafka_mock_cluster_handle`,
  `rd_kafka_share_partition_offsets_list_get`,
  `rd_kafka_share_partition_offsets_offsets`) and both
  `rd_kafka_topic_partition_get_(current_)leader_epoch` miss for one cause:
  the never-bound, `may_bind` and declared-local premises read the lexical
  declaration reader, and the typed reader's return and assignment rows for
  these functions are refused there (`position_rows.py` lists them); the
  per-function cause inside the lexical reader is not individually
  established. The control `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal`
  reproduces one cause, a typedef redeclared as C11 permits (librdkafka
  declares `rd_kafka_Node_t` twice): the same getter certifies with one
  declaration and refuses with two. Registered as
  `W47-TYPED-ROOT-DECLARATION-READER`; the premise is not relaxed.
- **Step B, 11 predicted Safe: 10 match, 1 miss** —
  `rd_kafka_mock_cgrp_classic_check_state` switches on an anonymous inline
  enum member (`enum { ... } state;`), whose type has no tag or typedef
  evidence and stays refused.
- **26 predicted P1 misses and 142 P1 Unknown:** 140 + 26 stay Unknown as
  predicted, except **28 unforecast gains in P1's list**: 22
  `rd_kafka_event_*_result` guards `if (!rkev || rkev->rko_evtype != …)`,
  where P1's lexical guard missed the `||` null test and the desugared plan
  reads `rkev->rko_evtype` only on the non-null edge; `rd_hdr_getBucketIndex`
  and `ut_print_toppar_list` (typed call actuals and conditions over roots
  the custody closure proves never bound, which P1 took as tracked);
  `mk_fifo_queue_get`, `mk_http_keepalive_check`, `mk_plugin_cap` (the same,
  roots "never observed" by P1's streams); and
  `rd_kafka_cgrp_maybe_clear_heartbeat_failed_err` (step B's
  `rd_kafka_cgrp_rebalance_protocol` becomes Safe and serves it).
- **Outside the list, 10 unforecast gains:** `advance_iterator`,
  `rd_hdr_histogram_reset`, `rd_hdr_sizeOfEquivalentValueRange`,
  `rd_kafka_broker_reconnect_backoff`, `rd_kafka_mock_partition_leader_epoch_check`
  and `mk_ptr_to_buf` (typed condition operands and actuals), the two
  `rd_hdr` summary cascades above, and `rd_kafka_share_update_acknowledgement_type`
  and `ut_set_types_uniform` (enum values).

Against P1's own forecast for its P3 list: of 86 P1-Safe rows 47 are Safe
(25 from P2, 22 here) and 39 remain Unknown; 28 P1-Unknown rows became Safe.

The system forecast ("a small number of rows") holds: 7 rows, 4 functions.

## Lean

`proof/QCore/TypedAccessPathPositions.lean`, registered in
`proof/AuditTypedAccessPathPositions.lean` (33 theorems; audit total 214). It
imports P2's module and `QCore.ConditionCallDesugar` and proves: refusal of an
unlisted position, of every P2 refusal in every position, of a connective
inside one read, of a custody, uncontracted or beyond-declaration actual, of a
pointer return or actual from a tracked root, and of an assignment into a
non-local; that the fallback never replaces an older admission; short-circuit
scoping (`an_unevaluated_and_operand_contributes_nothing`, the `||` and `?:`
forms, `an_and_guard_discharges_every_checked_hop`,
`an_or_guard_discharges_no_checked_hop` with non-vacuity witnesses); that an
operand's reads precede either edge; frame preservation for read positions;
that a typed return hands nothing back; that the callee's contract applies to
the caller's unchanged frame; and that the enum arm only adds enumerations.
The Rust-to-Lean correspondence is a review premise (constraint 28).

## Tests

`crates/qcore-c-gateway/tests/access_path_positions.rs` (12 tests): every
controlling position; `p && p->x` Safe against `p || p->x` UnknownAlias with a
tracked `p`; `c ? p->x : q->x` with only `p` guarded; union, typedef-hidden
union, call-in-path, cast and tracked mixed paths refused in a condition; the
assignment operand keeping 10b's identity refusal; typed returns; a return
through a released or tracked root never Safe; an interior-address return
refused; borrowed actuals Safe; releasing, uncontracted and tracked-root
pointer actuals refused; the redeclared-typedef getter control; and the enum arm
(tracked `b && b->mode == C_ON` Safe, `||` UnknownAlias, union beside it
refused). Unit tests in `typed_access_positions.rs` and `typed_access_path.rs`
cover node matching and positive enum evidence (anonymous aggregate, union and
pointer-to-enum typedefs refused).

Historical pins of older routes' grammar limits were re-pinned deliberately,
each with its reason in the test; every older route still refuses the shape,
and the typed fallback asked after it now reads it under its per-hop
obligations, so none is a relaxed refusal:
`call_argument_access::a_chain_keeps_the_refusal` (the gate refusal becomes the
chain's `r->next` link obligation); `return_position_pointer_leaf::the_adversarial_family_keeps_its_return_refusal`
(subscript, `void *` member and long chain read; address-of and `*view->ring`
still refuse); `chained_tail::{every_non_conforming_tail_shape_keeps_its_refusal,the_depth_bound_is_inherited_unchanged}`
(an indexed byte, a bitfield's sibling, a typedef-spelled link, an embedded
record and a five-selection chain read in a declared tail; the laundered base
now refuses by its `UnknownAlias` obligation); `neighbours::the_constructor_keeps_exactly_one_refusal_under_production_declarations`
(its load-bearing substitution moves from `name[0]`, now read, to `name + 0`,
which every route refuses); `o04_arms::every_untyped_link_keeps_its_unknown_and_its_diagnostic`
(three actuals read, exposing the fixture's own unguarded `h`; still Unknown);
`o04_read_bound::every_refused_member_shape_keeps_its_unknown_and_its_diagnostic`
(the partial argument `b->item->weight + n` read through the live stored
child); `reach_gate_positions::the_adversarial_family_keeps_its_unknown_and_its_diagnostic`
(subscript, long chain and `void *` member read in a condition); and the unit
`member_release::a_member_null_test_reads_only_single_pointer_members`
(`n->next->buf` and `n->buf == n->next` read; `n->buf && n->n` keeps the
refusal because both operands are older routes' reads). The bound-base pins
of rule 5b, the member-release double-release pins and every union, cast,
address and custody-position refusal pass unchanged.

## Gate

See [gate.json](gate.json).

## Follow-ups

- Retired: `W46-ACCESS-PATH-POSITIONS` (all three positions implemented with
  its discharge conditions and measured).
- Registered ([followups.json](followups.json)):
  `W47-ACCESS-PATH-POSITION-RESIDUALS` (sized above: 4,456 condition, 275
  return, 4,456 call-actual and 24 assignment refusals; 6,369 COND, 284 RET,
  15,037 IND leaves) and `W47-TYPED-ROOT-DECLARATION-READER` (12 forecast
  misses).
- Amended: `W43-INDIRECT-UNTYPED-SINGLE-ACCESS` (enum arm landed; the four P2
  enum misses are Safe) and `W46-ACCESS-PATH-PARITY` (position rows).
- Ledger 522 → 523 (`scripts/update_ledger.py`, then `build_matrix.py`).
- Stays open: `W46-ACCESS-PATH-PARITY`, `W46-ACCESS-PATH-TRACKED-MIXED-PLACES`
  (a tracked mixed path still refuses in every position), P5 pointer stores and
  `W46-UNION-MEMBER-PATH`.

## Files likely to conflict

`crates/qcore-c-gateway/src/{legacy_c.rs,legacy_condition_desugar.rs,legacy_call_argument_access.rs,typed_access_path.rs,lib.rs}`,
the two re-pinned tests, `proof/QCore.lean`, `docs/agents/{gateway.md,README.md}`,
`docs/STATUS.md`, `docs/plans/unsupported-coverage/README.md`,
`docs/development/increments/{README.md,wave-47.md}`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
