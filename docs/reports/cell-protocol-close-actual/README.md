# CELL-PROTOCOL-CLOSE-ACTUAL

Card brief: `private/integrator-briefs/cell-protocol-close-actual.md` and
`r6-common.md` (round 6, 2026-11-02). It discharges `CELL-PROTOCOL-CLOSE-ACTUAL`,
registered by [MK-EVENT-CELL-CROSS-FUNCTION](../mk-event-cell-cross-function/README.md).
Base `aa55f2235`, that card's tip. Main then landed that card's records
(`86b26e5b2`, docs only), which was merged into this branch (`455a1639a`); the
card gate ran with `--base 86b26e5b2`. It ran beside
CELL-PROTOCOL-FB-DECLARATIONS, which owns the Fluent Bit declarations.

> **USER DECISION 2026-11-02** (verbatim, relayed by the coordinator): "B as
> recommended". Its Q3: `close` / `mk_event_closesocket` of a declared
> protocol cell member is that cell's `#stale`.

The [prediction](PREDICTION.md) was committed at `fc36ceeda`, before any code.

## The problem

Rule 38c reads `close(r->CELL)` of a declared protocol cell as that cell's
`#stale`. Its close step runs after the statement's own lowering. But that
lowering was the access gate's refusal: the gate (W44-INDIRECT-CALL-ARGUMENT-ACCESS
premise 1) refuses an integer member actual at `close`'s custody position.
So every such close left `indirect memory access is outside the resource
projection` on its row, whatever rule 38c read. That covers the census's C3,
C5, C6, C8, C9 and C10 ends.

## What changed: rule 38d

[Rule 38d](../../agents/rules/38d.md) adds one admission and nothing else.

**The admission.** It is asked only where the statement's whole lowering is
exactly the gate's refusal, and the statement reads no tracked descriptor
(W44 premise 2). All of these must hold:

- rule 38 is in force;
- the statement is rule 38c's close step: `close(r->CELL)`, with `r`
  declared `struct TAG *` and `CELL` a one-segment cell of `TAG`'s declared
  protocol;
- the callee's contract in force is a function-map contract, whose one
  actual sits at a `resource_released` position of kind `descriptor`;
- the actual is exactly `r->F`, with `F` an integer member, or `r->F[K]`,
  with `F` an integer array member and `K` a canonical constant below its
  length (rule 38a's element read). `TAG` must have a single complete
  definition with no nested definition.

The statement is then classified again with that actual admitted. The result
must be the plain contracted call, with no destination and no nested call.

The decision is the package function `rule_38_descriptor_cells::close_actual::admits`,
a conjunction whose truth set is exactly `ADMITTED`. Its registry arm is
`descriptor-cell-close-actual`. The host is `Parser::cell_close_actual` in
`legacy_call_argument_access.rs`, asked from `expression_statement` in
`legacy_c.rs`.

**The lowering.** The admitted statement lowers to four steps:

1. the actual's typed read of `r` (W44 premise 3);
2. its second obligation (W44 premise 4);
3. the call, with the released position read as `Borrowed`, because the
   number read is no identity the projection tracks (as rule 1 reads a
   member release);
4. rule 38c's close step, marked `admitted`.

That close is the statement's whole custody effect.

**The reading** (`close_actual::reading`, and the close step's arm in
`legacy_descriptor_cells.rs`):

- **On a path with a fact of `r`'s caller object:**
  - a cell that names a live single identity is released once through rule
    38's table, and the record's pending names of it are staled (Q3; a
    `#same` name with it);
  - a stale cell refuses (`descriptor-cell-double-release`);
  - a cell that is maybe live, live under a join's identity, or not
    recorded (the guard's dead edge) refuses (`descriptor-cell-undecided`).
- **On a path with no such fact**, rule 38c reads nothing, and the gate's
  refusal stands, as the same diagnostic. This covers a close with no guard
  test, a close after a call that forwarded the record, a close on a
  tracked record and a close on a released formal.
- **Never a silent release.** An admitted close is the cell's `#stale` or a
  refusal. With `38` in `QCORE_GATEWAY_RULES_OFF` nothing is admitted, and
  the step refuses at the gate as well.

**The identity case.** With no declared protocol, rule 38c's parser emits no
step, so `cell_close_actual` is never asked. The new `admitted` field is
omitted from the serialized step when false.

**Shapes that keep the gate's refusal byte for byte.** An undeclared record,
a member that is not a cell, a multi-segment cell (`event.fd`), a copied
number (Q4), a used result (`r = close(…)`, `return close(…)`), a computed
actual (`s->e[0] + 0`), a non-release custody position (`resource_consumed`),
an uncontracted callee, Windows `closesocket` (no provider declares it) and
rule 38 off. A heap `close` cannot be declared over an `int` parameter: the
contract is withdrawn (`abi-parameter-type-disagreement`), as before. The
non-descriptor arm is covered by the decision vectors.

## Tests

`crates/qcore-c-gateway/tests/cell_protocol_close_actual.rs` has 27 tests.
Each asserts the complete diagnostic vector with lines, and every `Safe` is
rechecked. Before the code, the file was run on the base to record the
refused shapes' vectors. Every refused shape's vector below is the base's,
unchanged.

- **Admitted and decided (`Safe`):**
  - `close` of both ends of a `> 0` protocol under one guard test, then the
    sentinel stored dead;
  - the same through `mk_event_closesocket` (Monkey's Linux
    `#define mk_event_closesocket(s) close(s)`);
  - a `!= -1` protocol;
  - a `== 1` flag protocol;
  - a scalar (non-array) cell;
  - a `flb_pipefd_t` (typedef) array cell.
- **Admitted, then refused by rule 38c:**
  - Fluent Bit's spelling, each end under its own `> 0`
    (`descriptor-cell-undecided` at the second end and
    `descriptor-cell-protocol-broken` at the exit; the base also had the
    gate's refusal);
  - a close on the guard's dead edge;
  - a second close (`descriptor-cell-double-release`);
  - closes without the sentinel store (`descriptor-cell-protocol-broken`).
- **Admitted, but rule 38c reads nothing, so the gate's refusal stands:**
  - no guard test;
  - after `ev_note(s)` forwarded the record;
  - a tracked record;
  - a released formal (`unknown_alias:s` beside it).
- **Refused at the gate, as at the base:**
  - an undeclared record;
  - a non-cell member;
  - `event.fd`;
  - a copied number;
  - a computed actual;
  - a used result;
  - `resource_consumed`;
  - a heap `close` (withdrawn);
  - no `close` contract;
  - `closesocket`.
- **The switch.** Rule 38 off (a child process) keeps the gate's refusal.

Other test changes:

- `descriptor_cell_protocols.rs`: `a_close_of_a_protocol_cell_keeps_the_access_gate_refusal`
  is now `a_close_of_a_protocol_cell_leaves_the_access_gate`. The same C8
  fixture is `Safe`.
- The package's unit tests (4 new):
  - the admission vectors;
  - the admission set over all 192 sites;
  - the reading vectors;
  - that only a live single identity is staled.

**Lean.** `QCore.CellProtocolCloseActual`, audited by
`AuditCellProtocolCloseActual.lean`, has 14 theorems. It reuses
`QCore.DescriptorCellProtocol`'s `Fact`, `adopt`, `staleIdentity`, `store`
and `holds`, and `QCore.DescriptorCells`'s `release`. The theorems:

- **The admission:**
  - `the_admission_set_is_exactly_the_stated_one`;
  - `every_other_site_keeps_the_refusal`;
  - `rule_38_off_admits_nothing`.
- **Feeding rule 38c's `#stale`:**
  - `an_admitted_close_is_the_cells_stale` (the table's release plus the
    record's `staleIdentity`, the protocol vectors' `release` step);
  - `the_closed_identity_has_no_live_name`.
- **Never silent:**
  - `an_admitted_close_without_a_fact_refuses`;
  - `an_admitted_close_never_releases_silently`;
  - `a_second_admitted_close_refuses`;
  - `a_close_of_the_same_name_after_its_target_refuses`;
  - `a_close_on_the_dead_edge_refuses`;
  - `a_close_of_the_joined_identity_refuses`.
- **The protocol:** `admitted_closes_and_the_sentinel_restore_the_protocol`,
  for the `> 0` and `!= -1` guards.
- **Vectors:** `admission_vectors_hold` and `reading_vectors_hold`, the
  Rust `ADMISSION_VECTORS` and `READING_VECTORS` entry for entry.

## Prediction and measurement

The prediction is `none` on corpus, system and slice. No production
declaration carries a `quant cell` protocol at this tip, so no step is
emitted and nothing changes. PREDICTION.md relists every row whose leaves
include a gate refusal at a close of a member, at `baseline:0ac169180`:

- system: `flb_engine_failed` and `flb_output_instance_destroy`, ×2 each;
- slice: `flb_input_instance_destroy`, `input_thread_instance_destroy` and
  `collector_start`, ×2 each, plus `flb_tail_file_remove` and
  `flb_tail_fs_inotify_exit`;
- corpus: seven rows, three of them indirect `->close(…)` calls.

**What the merged tree should do** with CELL-PROTOCOL-FB-DECLARATIONS. That
card's declarations were not visible here, so each end is stated by the
protocol it would need. No status move is expected at any end.

- **`flb_output_instance_destroy`** (flb_output.c:615–621), if
  `flb_output_instance` declares a `ch_events[0] > 0` protocol:
  - the close at :616 is admitted and staled;
  - the close at :620 is admitted but refuses (`descriptor-cell-undecided`),
    because its own `ch_events[1] > 0` test is no protocol guard and the
    first test's dead edge reaches it;
  - the record is `descriptor-cell-protocol-broken` at the exit;
  - the gate's two refusals are gone.

  The fixture is `fluent_bits_output_instance_ends_refuse_after_admission`.
  Without that declaration, both lines are unchanged.
- **`flb_input_instance_destroy`** (flb_input.c:1311–1325):
  - `ch_events` behaves like the output end, if `flb_input_instance`
    declares it;
  - `ingress_queue_channels` keeps the gate's refusal, because a record
    has one protocol (G05i: one guard per record).
- **`input_thread_instance_destroy`** (flb_input_thread.c:194–224):
  - if `flb_input_thread_instance` declares the `notification_channels_initialized
    == 1` flag (C7, the only decidable one), `ch_parent_events` and
    `ch_thread_events` are not cells, so :211–222 keep the gate's refusal;
  - if it declares one of those pairs instead, that pair reads as the
    output end does.
- **`flb_engine_failed`** (flb_engine.c:1063–1065): its guard
  `ch_notif[1] != ch_notif[0]` is no protocol test, so the path holds no
  fact and the gate's refusal stands, byte for byte, whatever
  `flb_config` declares.
- **`collector_start`, the tail rows and the corpus rows**: unchanged.
  Each is a non-cell member or not declared, has a used result, or is an
  indirect call.

Every Fluent Bit row keeps other leaves too, so a move to `Safe` would be a
soundness finding.

The worktree's isolation guard refuses the live measurement, so the
integrator runs it from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/cell-protocol-close-actual-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:0ac169180f549406ec2f4aef9effb7ad2eb63df0 \
  --side tip=rev:a86bfdd4d \
  --declared docs/reports/cell-protocol-close-actual/PREDICTION.md
```

## Card gate

GATE-PLACEHOLDER

## Observations and follow-ups

- **Registered: `CELL-PROTOCOL-PER-END-GUARDS`.** Fluent Bit's destroyers
  close each channel end under its own `> 0` test and store no sentinel. A
  G05i protocol has one guard per record and says nothing of a non-guard
  cell's value where the guard is dead. So the admitted second close stays
  undecided and the record broken. A record holding two or three channel
  pairs can declare only one. That needs a user decision: per-end
  sentinels, or several protocols per record.
- **Amended: `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`.** The close residual is
  discharged, and the above stays.
- **`--check`.** `apply_amendments.py --check` exits 0 on the merged tree
  (retire, amend, register). Before the merge it refused the retire, because
  `CELL-PROTOCOL-CLOSE-ACTUAL` was registered only by the records that
  landed in `86b26e5b2`.
