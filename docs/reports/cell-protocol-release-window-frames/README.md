# CELL-PROTOCOL-RELEASE-WINDOW-FRAMES

Card brief: `private/integrator-briefs/r12-common.md` (round 12, 2026-11-04).
Base `c8c7edf21` (origin/main), baseline `baseline:c8c7edf21`. It discharges
the `named_followups` entry CELL-PROTOCOL-RELEASE-WINDOW-FRAMES, registered by
[CELL-PROTOCOL-PER-END-GUARDS](../cell-protocol-per-end-guards/README.md) and
amended by [CELL-RESULT-SET-ALIGNMENT](../cell-result-set-alignment/README.md).

> **USER DECISION 2026-11-04** (verbatim, on the entry's options (a) callee
> frame declarations, (b) a member-actual narrowing, (c) keep the refusal):
> "(a) Callee frames".

The brief's reading of it: a call in the window between an end's close and
the holder's release admits only where a declared or body-checked clause
states that the callee reads no end member of the record; (b) is not chosen
(unsound under `container_of`); calls without such a clause keep refusing.

The [prediction](PREDICTION.md) was committed at `990805b63`, before any code;
its amendment (the same `measure-prediction` block, `none` on every route)
records the two findings of §2.

## 1. What changed

**The clause** (grammar [G05k](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05k-callee-frames-over-a-declared-cell-protocol),
contract specification [C04.6.4](../../QCORE_CONTRACT_SPEC_r0_3.md#c0464-callee-frames-over-a-declared-cell-protocol);
G05k, C04.6.4 and rule id 38g were free, checked against every grammar id,
contract section and numeric and lettered rule id):

```qcore
quant cell mk_list_del: flb_output_instance |-> #untouched;
```

The callee, with everything it calls, neither reads nor writes any member a
declared protocol of `struct TAG` names (the first member of each cell and
guard), on every object of the type, however it reaches it. Two semantic
points are settled by "(a) Callee frames" plus fail-closed, so the card did
not stop:

- **Over the record's type, not the actuals.** A frame over the call's actuals
  would be option (b); `&ins->_head` reaches `ins` by `container_of`.
- **Writes included.** "Reads no end member" is read as the frame's usual
  footprint (no read, no write), the stronger claim: a callee that wrote an
  end would leave the caller's fact stale.

A `quant cell` clause that opens with a name instead of a `#` binder was
refused before (`quant-cell-shape`), so no accepted text changes meaning. The
loader refuses a frame whose record has no protocol in the same declaration
(`quant-frame-record`), a duplicate (`quant-frame-duplicate`) and any other
shape (`quant-frame-shape`), and reserves `#untouched` as a binder. Frames
load to `LoopLifetimeContract::frames` and `ResourceContracts::cell_frames`
(a record keeps its frames only where it keeps its protocol and every
declaration stating it states the same callees).

**Rule [38g](../../agents/rules/38g.md)** (new; rule 38f (5) points to it). The
package model is `rule-38-descriptor-cells/src/frame.rs`; the host is
`legacy_cell_frames.rs` and rule 38c's `touch`.

- **Standing in a unit.** After the parser builds a body's CFG,
  `settle_frames` checks each declared frame of a contracted callee the body
  calls against Clang's AST of the unit (`read_body`, `frame::body_check`):
  **checked** where the unit defines the callee's body and every body it
  calls and none names a protocol member or uses an unread construct (a call
  by pointer or out of the unit, recursion, a pointer reinterpreted as a
  scalar or byte pointer, an integer read as a pointer, a whole-record read or
  store, a scalar read through a pointer, `offsetof`, inline assembly,
  `__builtin_bit_cast`, atomic builtins); **refuted** where a body names a
  protocol member; **declared** otherwise. The standing travels on the CFG
  (`LegacyCfg::cell_frames`), so certificate rederivation reproduces it.
- **The reading.** Only where a contracted call would refuse a record that
  does not keep its protocol, and does not release it: checked, the fact
  passes the call unchanged; declared, it passes and the call is held,
  ``call to `CALLEE` reaching a `struct TAG` that does not keep its cell
  protocol rests on its declared frame, which awaits human review
  (awaiting-human-review)``, read by `verdict::is_held_call` as `Pending`;
  refuted, `descriptor-cell-frame-refuted` (new reason key, `Unknown`); none,
  `descriptor-cell-protocol-broken` as before. The analysis with every held
  declaration in force (`with_human_holds_in_force`, which now also sets
  `cell_frames_accepted`) reads a declared frame accepted, so a `Pending` row
  with another reason is `Unknown` (W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL).
  A refutation is never lifted by acceptance.

## 2. Findings that changed the prediction's content (no status move)

- **Option (b) was already in the host, and is removed for the window.**
  Rule 38c's host read a call as reaching a record only through an actual
  naming it (`ins`, or an unread actual such as `&ins->_head`): a member value
  (`ins->processor`, `thi->tp`) or another pointer (`collector`, which reaches
  the instance through `collector->instance`) was read as reaching nothing.
  That is the narrowing the user did not choose. In the window (a record that
  does not keep its protocol) every contracted call that does not release the
  record now reaches it, except a call handed only the record's own cell
  numbers (rule 38c's `close(r->CELL)`, which carries no pointer; the CFG
  carries each record's cell paths for this test). The holding case keeps
  rule 38c's reach (follow-up CELL-PROTOCOL-HOLDING-REACH-NARROWING).
- **A second contracted window callee.** `flb_input_collector_destroy(collector)`
  (flb_input.c:1331) is contracted and, under the first finding, in the window.
  Its frame is declared in production (§3). `flb_task_queue_destroy(ins->singleplex_queue)`
  (flb_output.c:628) is refused at the access gate first (the baseline's :628
  leaf), so no frame would be read and none is declared.

## 3. The declarations

On the existing host `flb_config_set_program_name`
(`providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`),
beside the protocols, with the bodies read at 66910c10, 046accf6 and
34155955 (byte-identical at the three; SHA-256 prefixes in the census run):

```qcore
quant cell flb_input_collector_destroy: flb_input_instance |-> #untouched;
quant cell mk_list_del: flb_input_instance |-> #untouched;
quant cell mk_list_del: flb_output_instance |-> #untouched;
```

- `mk_list_del`: `lib/monkey/include/monkey/mk_core/mk_list.h:139-150`
  (`__mk_list_del` and `mk_list_del`, static inline) names only `struct
  mk_list`'s `prev` and `next` and is in every unit that calls it: **checked**.
- `flb_input_collector_destroy`: `src/flb_input.c:3140-3156` (:3120-3136 at
  66910c10) reads `coll->instance->config` (not a protocol member), removes
  the collector's own event from `config->evl` and frees the collector; it
  calls Monkey out of the unit: **declared** (held for review).

**Pins.** `quirin-audit pin --source-root <tree> --crate
quirin-provider-fluent-bit --function flb_config_set_program_name` at
046accf6, 34155955 and 66910c10: the contract digest is new (`c002910c…`),
the sites are unchanged, one identity covers the three revisions.
`providers/contracts.py sync` regenerated `contract-inventory.lock`:
`teardown_destroy.qv` 161 → 164 clauses, the host 21 → 24; declarations
unchanged (15).

## 4. The window-call census

[`census/census.py`](census/census.py) reads each destroyer of a record with
declared per-end sentinels at the three trees, takes its window (the first
close of a declared end to `flb_free` of the record), lists every call in it
and joins each callee with one annotation ([annotations.json](census/annotations.json)).
It binds itself to the destroyers' file digests and refuses an unannotated
callee, an unused annotation or a changed census:

```sh
python3 -I docs/reports/cell-protocol-release-window-frames/census/census.py \
  --root 66910c10=<corpus fluent-bit-clang-source> \
  --root 046accf6=<system fluent-bit-clang-source> \
  --root 34155955=<slice fluent-bit-clang-source> \
  --check docs/reports/cell-protocol-release-window-frames/census/census.json
```

It exits 0 with 0 problems: 3 trees, 9 windows, 81 calls (`census.json`
sha256 `be8c324083c8208d7a606071faedba92ae472b0a0c6fbb8091d9003f3f4fae60`).
Lines at 046accf6 = 34155955 (66910c10 in brackets; no corpus row analyses
these units):

| Window | Line | Call | Contracted here | Frame | Rule 38g |
|---|---|---|---|---|---|
| `flb_output_instance_destroy` | :616, :620 [538, 542] | `mk_event_closesocket(ins->ch_events[k])` | yes (`close`) | n/a | the closes themselves (cell numbers only) |
| | :624 [546] | `flb_output_free_properties(ins)` | no | none | refuses as uncontracted |
| | :628 [550] | `flb_task_queue_destroy(ins->singleplex_queue)` | yes | none | the access gate refuses first |
| | :631 [553] | `mk_list_del(&ins->_head)` | yes | **checked** | passes |
| | :635 [557] | `flb_processor_destroy(ins->processor)` | no | none | refuses as uncontracted |
| | :638 [560] | `flb_free(ins)` | yes | n/a | the holder's release |
| `flb_input_instance_destroy` | :1312-:1324 [1304-1316] | four closes | yes (`close`) | n/a | the closes themselves |
| | :1330 [1322] | `mk_list_del(&collector->_head)` | yes | **checked** | passes |
| | :1331 [1323] | `flb_input_collector_destroy(collector)` | yes | **declared** | held (`awaiting-human-review`) |
| | :1334 [1326] | `pthread_mutex_destroy(&ins->ingress_queue_lock)` | no | none | refuses as uncontracted |
| | :1335 [1327] | `pthread_cond_destroy(...)` | held by its thread-synchronization obligation | none | refuses there first |
| | :1338 [1330] | `flb_storage_input_destroy(ins)` | no | none | refuses as uncontracted |
| | :1340 [1332] | `mk_list_del(&ins->_head)` | yes | **checked** | passes |
| | :1344, :1345 [1336, 1337] | ring-buffer cleanup and destroy | no | none | refuses as uncontracted |
| | :1350 [1342] | `flb_processor_destroy(ins->processor)` | no | none | refuses as uncontracted |
| | :1353 [1345] | `flb_free(ins)` | yes | n/a | the holder's release |
| `input_thread_instance_destroy` | :211-:222 | four closes | yes (`close`) | n/a | the closes themselves |
| | :225 | `flb_tp_destroy(thi->tp)` | no | none | refuses as uncontracted |
| | :226 | `flb_free(thi)` | yes | n/a | the holder's release |

An uncontracted callee's call refuses whatever a frame says, so no frame is
declared for one; the fixtures declare frames for `flb_output_free_properties`,
`flb_storage_input_destroy` and `flb_tp_destroy` to show what each would read
(all three declared: their bodies call out of the unit, or are not in it).

## 5. Tests

`crates/qcore-c-gateway/tests/cell_protocol_release_window_frames.rs`, 13
tests under the production module (its frames, and three fixture frames added
to the host's own text), each asserting the complete vector with body lines:

| Fixture | Window call | Outcome |
|---|---|---|
| `the_production_module_states_mk_list_del_frames` | the inventory | `flb_input_instance`: `flb_input_collector_destroy`, `mk_list_del`; `flb_output_instance`: `mk_list_del` |
| `mk_list_del_in_the_output_window_is_body_checked` | `mk_list_del(&ins->_head)` (:631, mk_list.h's bodies in the unit) | framed: the unlink refusal and the release's `unknown_alias` only; unframed: `protocol-broken` too |
| `flb_output_free_properties_is_declared_only_and_held` | `flb_output_free_properties(ins)` (:624; body in the unit, calls `flb_kv_release` out of it) | `Pending`, the hold alone |
| `a_checked_frame_passes_the_window_call` | a helper whose whole body is in the unit | `Safe`, its certificate rechecks |
| `a_frame_whose_body_reads_an_end_is_refuted` | a notifier reading `ch_events[1]` | `descriptor-cell-frame-refuted` |
| `a_callee_without_a_frame_keeps_refusing` | `flb_processor_destroy(ins->processor)` (:635), a member value | `protocol-broken` |
| `without_frames_the_window_refuses_as_before` | the declared and checked shapes, no frame | `protocol-broken` |
| `the_input_window_holds_storage_and_checks_the_unlink` | `flb_storage_input_destroy(ins)` (:1338), `mk_list_del(&ins->_head)` (:1340) | held, the unlink refusal, the release's `unknown_alias` |
| `flb_storage_input_destroy_is_declared_only_and_held` | `flb_storage_input_destroy(ins)`, then the `-1` sentinel | `Pending` |
| `flb_tp_destroy_is_declared_only_and_held` | `flb_tp_destroy(thi->tp)` (:225), a member value | `Pending` |
| `a_hold_never_masks_another_reason` | `flb_tp_destroy(thi->tp)` then `flb_free(thi)` | `Unknown` (the second analysis keeps the release's reason) |
| `flb_task_queue_destroy_refuses_at_the_gate_first` | `flb_task_queue_destroy(ins->singleplex_queue)` (:628) | the gate's refusal, as at the baseline |
| `flb_input_collector_destroy_is_declared_only_and_held` | `flb_input_collector_destroy(collector)` (:1331), another pointer | held, and the collector's own `unknown_alias` |

No earlier fixture changes (`cell_protocol_per_end_guards.rs`,
`cell_protocol_close_actual.rs`, `cell_protocol_fb_declarations.rs`,
`cell_result_set_alignment.rs`, `descriptor_cell_protocols.rs`, the member-path
and production-contract suites): their window uses are `uses_record(ins)`
(named, no frame) and their closes are handed cell numbers only.

Package: `the_body_vectors_hold` (11), `the_window_vectors_hold` (16),
`the_vectors_are_the_lean_ones` (both tables, read from the Lean text),
`only_a_checked_or_accepted_frame_passes`,
`the_protocol_members_are_every_cell_and_guard_head`. Loader:
`callee_frames_load_beside_their_protocol`,
`every_other_frame_spelling_rejects_the_module`,
`two_declarations_that_disagree_leave_the_record_without_frames`. Host unit
tests: the hold's spelling round trip and the AST type readers.

## 6. Lean

`QCore.CellProtocolReleaseWindowFrames`
(`proof/QCore/CellProtocolReleaseWindowFrames.lean`, audited by
`AuditCellProtocolReleaseWindowFrames.lean`; standard axioms only, no `sorry`,
no project axiom), 13 theorems: `a_framed_call_keeps_every_end` and
`a_framed_call_reads_no_end` (a callee whose footprint excludes the protocol
members keeps every end of every object and its effect reads none);
`an_established_body_names_no_member`,
`an_established_body_calls_only_established_bodies`,
`a_missing_body_is_never_established`, `a_recursive_body_is_never_established`
(the body check); `no_declaration_no_frame`,
`only_a_checked_or_accepted_frame_passes_unheld`, `a_declared_frame_is_held`,
`acceptance_never_lifts_a_refutation`,
`without_a_frame_the_window_refuses_as_before` (the standing and the reading);
`window_vectors_hold` and `body_vectors_hold` (Rust `WINDOW_VECTORS` and
`BODY_VECTORS`, entry for entry). The registry lists the module under rule 38.

## 7. The declared bound

No reading of this card is decided by the declared live set `[1, MAX]`
(CELL-RESULT-SET-DESCRIPTOR-ZERO-POLICY): a frame passes or holds a call
whatever the sets, and it never discharges an end. The vector
`pos/L0: test live; close c; join; call checked; free` (POSIX's `[0, MAX]`)
still refuses `zero-undecided` at the release.

## 8. Prediction and measurement

[PREDICTION.md](PREDICTION.md): `none` on corpus, system and slice. Expected
content changes, none a status move:

- system `flb_output_instance_destroy` (units 0, 1): `protocol-broken` at :631
  removed;
- slice `flb_input_instance_destroy` (units 0, 1): `protocol-broken` at :1330
  and :1340 removed, the hold at :1331 added;
- slice `input_instance_channel_events_init` (units 0, 1): possibly
  `protocol-broken` at a later contracted call on the success edge;
- rows depending on the host may show a changed contract digest.

The worktree's isolation guard refuses the live measurement, so the
integrator runs it from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/cell-protocol-release-window-frames-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:c8c7edf2165a2065c234f3a63b4eed23e937ced6 \
  --side tip=rev:731599d8072cfa138a31b8332945e078fecb6841 \
  --declared docs/reports/cell-protocol-release-window-frames/PREDICTION.md
```

**Measurement (integrator, 2026-11-05, combined).** This card was merged by
the user together with CELL-PROTOCOL-RELEASE-WINDOW-FRAMES,
CFL-MPACK-EXTERNAL-DECLARATIONS, LIBC-GAP-DECLARATIONS,
LOG-PRINT-BODY-CONTRACTS, LIBRDKAFKA-HEADER-INLINES,
MEMBER-ACCESS-PROJECTION and CURRENT-RUN-DEFECTS (main `25508ccaf`, records
`8531b005b`, full gate passed), so one undeclared measurement covers them:
tip `rev:8531b005b` against `baseline:c8c7edf21`, output
`private/measure/integrate-r4-20261105T111129Z`, exit 0, 145 min. System
and slice: 0 moves. Corpus: 38 moves (Safe +37, Unsafe +1), 2 unit moves
(`ctr_attributes.c` -> Safe, `cmt_decode_statsd.c` -> provisional Unsafe), 0
unmatched. Every move declared by a merged card's PREDICTION.md happened. 29
moves are declared there; `_rd_strcasestr` is LIBC-GAP's recorded extra; 8
are cascades no single-card prediction counted, each with no leaf left at
the tip:
- `crc32c_init_hw` (crc32c.c): its only base leaves were the two calls to
  `crc32c_zeros`, which MEMBER-ACCESS-PROJECTION makes Safe (in-unit summary).
- `rd_kafka_mock_Produce_reply_tags_partition_write`,
  `rd_kafka_msgset_writer_write_MessageSet_v2_header`,
  `rd_kafka_msgset_writer_write_Produce_header`: base leaves were calls to
  the `rd_kafka_buf_write_*` writers this card declares, plus `unresolved
  resource identity` and member-store `indirect memory access` leaves on
  their results (`msetw->msetw_of_start = rd_kafka_buf_write_i64(rkbuf, 0)`,
  a scalar offset) that were artefacts of the undeclared call.
- `rd_kafka_msg_partitioner_consistent` (`rd_crc32_init`/`rd_crc32_update` in
  its return expression), `rd_kafka_outq_len` and `rd_kafka_queue_length`
  (`rd_kafka_q_len(rkqu->rkqu_q)` in a return): the return-expression and
  argument-member leaves were on calls this card declares borrowing.
- `rd_kafka_partition_leader_destroy_free(void *ptr)`: passes its formal (as
  `leader = ptr`) to `rd_kafka_partition_leader_destroy`, declared here
  `resource_released(leader, nonnull)`: a release of its own formal, as the
  base's `unresolved resource identity ptr` leaf could not read.
[integrate-r4-native-comparison.json](../librdkafka-header-inlines/integrate-r4-native-comparison.json)
(sha256 `01ab43b871fb84afdc0710b3326a00c5dc67ea785daad61d2a22f0d7519eb9f5`).

## 9. Card gate

`python3 scripts/gate_scope.py --base c8c7edf21 --out private/gate` ran on
`731599d80`, the last code commit, with a clean tree ([gate.json](gate.json),
`uncommitted: []`). Only this report's README, its amendments and gate.json
were added after it. The scope was full: the diff reaches `qcore-c-gateway`,
`qcore-schemas` and `qcore-contract`, a proof root changed (all Lean), and
`providers/` changed (provider gate).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 |
| fmt, clippy | 0, 0 | |
| Rust suite | 0 | 603 units, 644 summaries; 9,184 passed, 0 failed, 16 ignored, 135 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean | 0 | 407 audits |
| Python suite | **1** | 577 tests, 1 failure (below) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider `cargo test` | 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The one Python failure is the expected stale record:
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`,
`['24.0.0', 406] != ['24.0.0', 407]`: the committed matrix's Lean-audit count;
this card adds `AuditCellProtocolReleaseWindowFrames.lean`. The integrator
regenerates the records.

An earlier run on `c6680f3db` was stopped by me during its build to fold in
`731599d80` (the frame table carried only on a CFG that can hold a fact). The
logs are under `private/gate/` in the worktree; the console log is outside
it, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/gate-console.log`.

## 10. Follow-ups

- **Retired: CELL-PROTOCOL-RELEASE-WINDOW-FRAMES** (this card).
- **Registered: CELL-PROTOCOL-HOLDING-REACH-NARROWING.** Outside the window,
  rule 38c's host still reads a call as reaching a record only through an
  actual naming it; a holding record not reached keeps its fact across a call
  handed a member value or another pointer. Under the declared protocol the
  record still keeps its protocol after such a call, but the caller's
  identities could be stale if the callee closed and reset an end.
- **Registered: CELL-FRAME-REVIEW-REPORTING.** A frame hold names its act in
  its refusal only: the audit's `pending_human_review` section and the rows'
  `review_holds` member list C06.1 markers, not declared frames. No
  production frame is held where a row could be `Pending` today.
