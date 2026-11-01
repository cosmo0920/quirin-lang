# MK-EVENT-DESCRIPTOR-QUANT

Card authority: ledger entry `MK-EVENT-DESCRIPTOR-CUSTODY`
(`docs/reviews/r0_3/acceptance_matrix.json` `named_followups`; DISCHARGE: "a
descriptor out slot or a member-held descriptor identity in v1, the four
declarations restated with it (bodies read at both revisions, pinned),
measured"). Base `eb49bb6ce`. Phase 1 is [PROPOSAL.md](PROPOSAL.md); the
prediction, committed before any code, is [PREDICTION.md](PREDICTION.md).

## User decisions (verbatim)

> **USER DIRECTION 2026-11-01:** "We must decompose the premise into quant
> like composed clauses."

> **USER DECISION 2026-11-01** (relayed by the integrator): "I approve the
> changes of D1-D5."

The integrator's reading, which this card implements:

- **D1 = 1A.** The member names the timer's new descriptor; the result is
  its witness. The forms are `#fresh(descriptor)` and
  `{#stale(descriptor), #kept}`.
- **D2 = 2A.** The destroy states `#stale(descriptor)`, with the rule premise
  `P-CELL-STATE-NOT-FORWARDED`; it does not refuse an untracked cell (not
  2D).
- **D3 = 3A.** The channel's slots are `#pointee` cells, and the member is
  `#same(r_fd)`.
- **D4 = 4A.** The channel destroy carries the member-equality guard.
- **D5.** The kind is carried on the value, and heap stays bare.

## What changed

- **Schema.** `qcore_schemas::descriptor_cell` holds the canonical
  `DescriptorCells`: kinds, per-class states, guarded releases and the
  member-equality guard. It is carried on `RetainingContract`, omitted when
  empty, so every other declaration serializes as before. `fresh`, `same` and
  `pointee` join `RESERVED_KEYWORDS`.
- **Loader.** `crates/qcore-contract/src/contract_descriptor_cell.rs` reads
  both levels, total-or-reject (`quant-live-cell-*`). It reads the shorthand
  cell states; the member-cell words `resource_member_kind`,
  `resource_member_out`, `resource_member_released`, `resource_member_alias`
  and `resource_member_stale_or_kept`; and the slot's lower level (the C04.13
  dispositions `resource_out` / `resource_borrowed` with
  `resource_kind(S, descriptor)`). It also reads the guarded release
  `resource_released(X, valid) when #P: TAG.FIELD == #X` and the guarded
  `#holds` removal. Every spelling elaborates to one canonical set; an
  agreeing overlap merges and a disagreeing one rejects.
- **Rule 38.** The package
  `crates/gateway-rules/rule-38-descriptor-cells` holds the cell table, its
  two registered arms (`cells::release`, `cells::decide_equality`) and the
  decision vectors shared with Lean. The host is
  `crates/qcore-c-gateway/src/legacy_descriptor_cells.rs`, wired through
  thin hooks:
  - rule 12t's retaining call and transfer;
  - the witness (recorded after the call's own write; refined in
    `refine_edge`; a validity test of a `-1`-guarded witness lowered as
    `BranchOnValidity` through a separate witness-name set that never feeds
    the custody predicates);
  - the join;
  - the exit and owner-release refusals.

  The rule document is `docs/agents/rules/38.md`. 38 was confirmed free:
  every numeric and lettered id was checked, and the one mention is "reserved
  rule 38 unused" in `w48-bu-external-callee-contracts/integration-amendments.json`.
- **Declarations.** The four declarations are restated in
  `providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`, and
  `mk_event_descriptors` is gone. The bodies were read at 66910c10, 046accf6
  and the slice export (PROPOSAL.md §1). `providers/contracts.py sync` wrote
  the lock diff: `event_loop.qv` goes from 56 to 62 clauses, and the four
  premise entries are removed. `quirin-audit pin` ran at the three trees
  ([scripts/pin.py](scripts/pin.py)).
- **Audit.** Every declaration carrying a cell state publishes
  `P-CELL-STATE-NOT-FORWARDED` (origin `rule-38`) in its row premises. In
  `outer_selection.rs`, `NOT_CLOSED_WORLD` replaces `mk_event_descriptors`
  with the new tracking-limit premise.
- **Specification.** The contract spec gains C04.6.2 (the decisions
  verbatim). C04.6's out-slot row and "not in the vocabulary" line are
  amended, and so is C04.20's tracking-limit list. Grammar G05d gains
  `cell_clause`, `guarded_release`, `guarded_removal` and
  `member_cell_predicate`. Rule 12t's text is amended.
- **Lean.** `QCore.DescriptorCells` holds 16 audited theorems, using only
  `propext` and `Quot.sound`, with no `sorry` and no project axiom. The POSIX
  uniqueness of open descriptor numbers is a hypothesis of
  `distinct_live_descriptors_decide_ne`.
- **Tests.**
  - `crates/qcore-contract/tests/descriptor_cells.rs`: the two levels read
    identically for the timer and the channel; the teardown guard; twelve
    rejections; the restated provider module.
  - The loader's unit tests.
  - The package's vectors.
  - `crates/qcore-c-gateway/tests/descriptor_cells.rs`, on route G under the
    provider's own declarations:
    - a destroy of a caller's cell is `Safe`;
    - a witnessed timer created into a tracked object and destroyed is
      `Safe`;
    - a live cell refuses its owner's release (`descriptor-cell-owner-released`);
    - a second destroy refuses (`descriptor-cell-double-release`);
    - an untested witness refuses;
    - a record whose first member is the event refuses;
    - a channel created into a caller's object and destroyed through its read
      end is `Safe`; through the other end, its cells escape;
    - an undecided guard refuses;
    - with `QCORE_GATEWAY_RULES_OFF=38`, every cell call refuses.

## Deviations and limits

- **Failure-class cells.** A creation's failure class reads every cell the
  creation named as stale, slots included. The proposal said a slot is
  untouched there. The stale reading refuses a later release where the
  untouched one might not, so it is the refusing direction.
- **Array-element slots.** A slot actual `&X->f[K]` (Fluent Bit's
  `&sched->ch_events[0]`) and its by-value read `X->f[K]` are keyed by rule
  38, but the access gate upstream refuses the array element's member read at
  the call first. Those calls keep a gate refusal.
- **Cross-function reading.** The witness, `#same` and the guard decide only
  within one body. Cross-function lifetime remains
  `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`.

## Measurement

**Not run: blocked in this session.** The isolation guard of this worktree
agent refuses every `quirin-audit measure` command line that names the
prepared inputs in the main checkout
(`--inputs-base /media/Data/GitHub/quirin-lang`), with "names git in a form
too complex to verify that it stays inside the worktree". The tool's own
default inputs base (this worktree) holds no prepared inputs: a probe exited 1
at `build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json:
No such file or directory`. The measurement was neither bypassed nor
delegated. The command to run, from this worktree at its tip (base side
`baseline:eb49bb6ce`, published 2026-11-01):

```sh
cargo build --release -p qcore-audit -p quirin-audit-plugin-measure
target/release/quirin-audit --plugin-path target/release --output private/measure-1 \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:eb49bb6ceb52c5a441dc26e68332065119993e99 --side tip=rev:<tip> \
  --declared docs/reports/mk-event-descriptor-quant/PREDICTION.md
```

`--inputs` (not `--profile`) is used because of
`MEASURE-BASELINE-ROUTE-SPELLING`.

**Prediction** (PREDICTION.md): no status moves on corpus, system or slice.
The premise-carrying rows were relisted at `baseline:eb49bb6ce`, and the set
is identical to the one at `98b1bb785` (PROPOSAL.md §4): 8 corpus rows, 18
system row instances and 16 slice row instances. The one `Safe` row,
`flb_sched_timer_cb_disable` (×2), is the shape of the fixture
`a_destroy_of_a_callers_cell_is_the_identity`, which stays `Safe`.

## Card gate

The gate ran as `python3 scripts/gate_scope.py --base eb49bb6ce --out
private/gate-2` on `3d3e3db93`, the last code commit; only this section
changed after it. Scope: effectively the full gate, with every Lean audit and
the provider gate.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 590 units; 8,993 passed, 0 failed, 15 ignored, 90 filtered |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| Lean audits | 0 | 399 |
| Python suite | **1** | 568 tests, 1 failure |
| provider package build | 0 | |
| provider package verify-all | 0 | |
| provider package verify | 0 | |
| provider tests | 0 | 133 passed, 0 failed, 1 ignored |
| provider fmt | 0 | |
| provider clippy | 0 | |

**The Python failure.** The one failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`:
the acceptance matrix's derived `lean_audits` is 398, and the tree now gives
399, the new `AuditDescriptorCells`. The matrix is the integrator's to
regenerate at landing, as for every card that adds a Lean audit.

**The first gate run.** `private/gate` on `3a0defb78` exited 1 on two pins
this card moves, now fixed: the withheld-declaration count (107 → 108) and
the parser-field classification of `cell_witness_names`.

**Lock fix.** Commit `b2b10e54d`, the providers-lock fix, is cherry-picked as
`f3afdedc5`.

**Logs.** Gate logs are under `private/gate-2/` and the console log is
`private/logs/gate-2-console.log`. The console log is in the worktree because
the isolation guard refuses a redirect to the scratchpad path.

## Observations and follow-ups

- **`MK-EVENT-FIRST-MEMBER-CELL`.** A `data` actual whose pointee is a record
  with `struct mk_event` as its first member (`ins` in
  `input_instance_channel_events_init` and `flb_output_init_all`, Monkey's
  `server_timeout`) refuses (rule 38 (1)). Reading the first member as the
  event would admit them.
- **`MK-EVENT-CELL-ARRAY-SLOTS`.** The access gate refuses the member read of
  an array element at a cell-bearing call (`&X->f[K]`, `X->f[K]`), so Fluent
  Bit's channel slots never reach rule 38.
- **Removal poisoning at a channel destroy.** The gap PROPOSAL.md §5 named is
  closed by D4's guarded removal: an undecided guard poisons.
- **The pipe arm of `_mk_event_timeout_create`.** It is not compiled, and is
  unchanged (PROPOSAL.md §5).
