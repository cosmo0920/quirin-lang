# MK-EVENT-CELL-CROSS-FUNCTION

Card brief: `private/integrator-briefs/mk-event-cell-cross-function.md` (round
5, 2026-11-02). This card narrows `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`; it
does not discharge it. The base is `0ac169180`, which is origin/main after
RETAINING-CALL-MINTED-NAME-ACTUALS. Phase 1 started on `a9f7bebbe`, and the
branch was rebased before Phase 2.

- **Phase 1** ([PROPOSAL.md](PROPOSAL.md) §1–6) holds the census of every
  create/destroy pair across functions at 66910c10, 046accf6 and 34155955. It
  found that the decided forms do not forward a cell effect, and it stopped
  for a decision.
- **The decision** (§7):

  > **USER DECISION 2026-11-02** (verbatim, relayed by the coordinator): "B
  > as recommended".

  The answers were:
  - Q1: B, declared record-type cell protocols, plus exact transport;
  - Q2: the `quant cell` spelling;
  - Q3: a close of a cell member is its `#stale`;
  - Q4: copied numbers are not names.
- **The prediction**, [PREDICTION.md](PREDICTION.md), was committed at
  `63bbb3068`, before any code.

## What changed

**The clause** (C04.6.3, grammar G05i). The grammar is G05i because G05e to
G05h were already taken.

```qcore
quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;
quant cell #s: flb_sched.event.fd |-> #same(ch_events[0]) when #s.ch_events[0] != -1;
```

- **Guards.** The guards are `!= -1` and `> 0` (sentinel cells, which must be
  `#fresh`) and `== 1` (a flag that is no cell).
- **Reading.** The frontend reads `quant cell` as a new clause kind
  (`QuantCell`). Before this card the spelling was a parse error.
- **Loading.** `local_condition::cell` / `cell_protocols` load it total or
  reject, into `qcore_schemas::descriptor_cell::CellProtocol`, inside
  `LoopLifetimeContract::cells`. That field is additive and omitted when
  empty.
- **Hosting.** The host becomes a withheld lifetime declaration, as a `quant
  registered` host does.
- **The table.** `ResourceContracts::cell_protocols` keeps a record's
  protocol only when every declaration agrees.
- **Its standing.** The protocol is a declared assumption: premise
  `P-CELL-PROTOCOL-DECLARED`, published by `qcore-audit` with the host. Every
  row that reads it names the host (`contract_dependencies::relied`).

**Rule 38c** ([rule text](../../agents/rules/38c.md)).

- **The package model.** It is in
  `rule-38-descriptor-cells/src/protocol.rs`: `Fact` (what the guard says,
  whether the body touched the record, and the caller's pending identities),
  `holds`, `store`, `join`, and `PROTOCOL_VECTORS`. The cell table gains
  `adopt`, `forget`, `entries` and the refusal
  `descriptor-cell-protocol-broken`.
- **The host.** It is `legacy_descriptor_cells.rs`:
  - **Guard test.** A guard test is an exact `r->GUARD OP V` or its
    complement. It is held in `LegacyCfg::cell_tests`, keyed by branch node,
    and the instruction itself is unchanged. On a desugared plan it is read on
    the plan's one deciding node. Its live edge introduces the caller's
    identities, which are adopted into rule 38's table when an operation
    first reads the cell. Its other edge records that no cell is live.
  - **Stores and closes.** A store to a protocol member, and `close(r->CELL)`,
    become a `CellProtocolStep` node after the statement's own lowering.
  - **Creation.** A creation into a protocol cell is admitted only where the
    guard was read dead or the cell is recorded.
  - **Exits.** At an exit, a record that keeps its protocol exempts its cells
    from `descriptor-cell-escapes`, and a record that does not keep it
    refuses.
  - **Calls.** At a call that may reach the record (other than a cell-bearing
    one), a record that keeps its protocol is forgotten, so it is read again
    from the protocol (the transport). A record that does not keep it
    refuses.
  - **Joins.** A join keeps the protocol where both paths kept it.
  - **Releases.** The release of a tracked record refuses while a pending
    name may be live.
- **The identity case.** In an environment that declares no protocol, the
  parser emits no step, no cell operand carries a protocol, and no path holds
  a fact. Every hook is then the identity.

**`RETAINING-CALL-MINTED-NAME-LIFETIME-SUMMARIES`** (registered by the
concurrent card in this card's area) is not touched. B changes no lifetime
summary, and no inferred summary carries a cell.

**Lean.** `QCore.DescriptorCellProtocol`, audited by
`AuditDescriptorCellProtocol.lean`, holds 13 theorems. It uses only standard
axioms (`propext`, `Classical.choice`, `Quot.sound`), with no `sorry` and no
project axiom. The theorems are these:

- **A forwarded cell equals the in-body reading.**
  - `a_forwarded_cell_equals_the_in_body_reading`: adopting the three names a
    live test introduced gives exactly the in-body creation's table.
  - `the_adopted_names_leave_the_entry`.
  - `the_forwarded_guard_decides_equal`.
- **An unresolved forward never decides.**
  - `an_untested_record_decides_no_guard`.
  - `a_test_never_overrides_a_fact`.
  - `a_conflicting_guard_never_decides_dead`.
- **A forwarded cell ends only by its close or its holder's release.**
  - `a_dead_protocol_names_no_live_cell`.
  - `a_store_never_overwrites_a_live_cell`.
  - `a_release_stales_the_pending_names`.
  - `a_live_cell_holds_the_record`.
- **Guards.**
  - `a_creation_establishes_only_the_sentinel`.
  - `a_live_looking_sentinel_breaks`.
- **Vectors.** `protocol_vectors_hold` checks the Rust `PROTOCOL_VECTORS`
  step for step.

## Tests

`crates/qcore-c-gateway/tests/descriptor_cell_protocols.rs` holds 16 tests,
one or more per census shape. Each asserts the complete diagnostic vector,
and every `Safe` certificate is rechecked. The hosts are synthetic, and the
Monkey declarations are the provider's own.

- **C1, the `!= -1` sentinel, destroyed in another function.**
  - `flb_sched_destroy`'s teardown over a caller's record is `Safe`, as is
    its complement test (`== -1`).
  - These refuse with `descriptor-cell-protocol-broken`:
    - a teardown that leaves the sentinel;
    - a teardown on one arm only;
    - a call that reaches a broken record.
  - These keep rule 38's `descriptor-cell-guard-undecided`:
    - a second destroy (two stale identities);
    - an untested teardown;
    - a record type without a protocol;
    - a destroy after a call that forwarded the record.
- **C1's creator.**
  - A creation into a caller's record that the test read dead forwards its
    cells: no `descriptor-cell-escapes`. Only rule 12t's
    `retention-undecided` stays, which is the registration side and out of
    scope.
  - `flb_sched_create`'s shape over a record this body allocates keeps the
    existing member-store refusal (`unknown_alias:c`) before the creation.
  - Without the dead stores, the creation refuses
    (`descriptor-cell-undecided`), and the failure arm and the exit refuse
    too.
- **C4 and C7, the `== 1` flag.**
  - `input_thread_instance_destroy`'s teardown is `Safe`.
  - A flag stored dead over live cells refuses.
  - A creator that tests the flag dead, creates, and stores the flag live
    keeps the protocol; only `retention-undecided` stays.
- **C5, C8 and C9, `> 0` with `close`.** The access gate refuses `close(s->e[0])`
  first: an integer member at a custody position (W44 premise 1). That test
  rests on the gate's refusal, not on rule 38c (follow-up
  `CELL-PROTOCOL-CLOSE-ACTUAL`). The Q3 reading is covered by the package
  vectors (`release`) and the Lean vectors.

Other tests:

- **Package.** The package's unit tests (10, three of them new) check the
  shared vectors, that no creation establishes `> 0` or a flag, and that an
  untouched record keeps its protocol.
- **Loader.** `qcore-contract`'s `local_condition::cell_protocol_tests` (3)
  cover these cases:
  - the decided candidate loads to one protocol, and its host is withheld;
  - thirteen malformed spellings each reject the module;
  - two declarations that disagree leave the record without a protocol.

## Prediction and measurement

No production declaration carries a `quant cell` clause, so no row moves on
any route, and the prediction is `none` on corpus, system and slice. Adopting
protocols for Fluent Bit's records needs host declarations in
`quirin-provider-fluent-bit` and a store census, so it is the follow-up
`CELL-PROTOCOL-FB-DECLARATIONS`. That follow-up's expected yield is leaf trades
at `flb_sched_destroy` :856, `input_thread_instance_destroy` :197 and
`flb_engine_shutdown` :1718, with no status move.

The PREDICTION.md table was relisted at `baseline:a9f7bebbe`, because the new
baseline was not yet published. It has been relisted since at the published
`baseline:0ac169180`. The same eight rows (×units) carry descriptor-cell leaves,
with the same leaf counts.

The worktree's isolation guard refuses the live measurement, so the integrator
runs it, from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/mk-event-cell-cross-function-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:0ac169180f549406ec2f4aef9effb7ad2eb63df0 \
  --side tip=rev:3bb265c64b70ba9ee50d20ff3d8e018dc107ac14 \
  --declared docs/reports/mk-event-cell-cross-function/PREDICTION.md
```

## Card gate

The gate ran as `python3 scripts/gate_scope.py --base 0ac169180 --out
private/gate-p4` on `3bb265c64`, the last code commit, with a clean tree. Only
this report, its amendments and [gate.json](gate.json) were added after it.
The scope was full: the diff reaches `qcore-c-gateway`, every Lean audit ran,
and the provider gate did not apply.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 593 units; 9,059 passed, 0 failed, 15 ignored, 109 filtered |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| Lean audits | 0 | 402 |
| Python suite | **1** | 577 tests, 1 failure |

**The Python failure.** The one failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`.
The matrix's derived `lean_audits` is 401, and the tree now gives 402: the new
`AuditDescriptorCellProtocol`. The integrator regenerates the matrix at
landing.

**Earlier attempts.**

- **`private/gate-p2`** (on `4201a6074`). It stopped in the Rust suite build:
  `compiler/examples/alias_aggregate_probe.rs` names every instruction
  variant. Fixed in `1cfa66fc0`.
- **`private/gate-p3`** (on `1cfa66fc0`). It exited 1 on two steps:
  - clippy: `compile_planned_if` was unused;
  - the Rust suite: 4 failures in `libc_reading_site_evidence` and
    `reading_site_domain`, "artifact reference mismatch:
    legacy_condition_desugar.rs". `docs/reports/libc-reading-site-evidence/bindings.json`
    pins that file whole.

  `3bb265c64` restores the file byte for byte and reads the plan's deciding
  node in `legacy_cfg_builder.rs`.

The logs are under `private/gate-p4/` in the worktree. The console logs are
outside it, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/cell-cross-function-gate-p4-console.log`
(and `-p3`, `-p2` for the earlier runs).

## Observations and follow-ups

- **Registered.**
  - `CELL-PROTOCOL-FB-DECLARATIONS`: the production declarations and their
    store census.
  - `CELL-PROTOCOL-CLOSE-ACTUAL`: the access gate's refusal at `close`'s
    custody position, which ends the path before Q3 is read.
- **Amended** into `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (LEFT for
  descriptor cells):
  - no run-wide store census yet, so every protocol is a declared assumption;
  - member stores into a record the body allocates refuse first;
  - creators that rely on calloc's zero have no dead reading;
  - records reached through lists or threads stay refused;
  - numeric copies stay refused (Q4);
  - C2's `if (config->evl)` guard is no cell sentinel;
  - Monkey's process-lifetime channels are never closed.
- **The proposal's C2 observation** (a close of descriptor 0 after a failed
  start) is unchanged. It is not a finding.
