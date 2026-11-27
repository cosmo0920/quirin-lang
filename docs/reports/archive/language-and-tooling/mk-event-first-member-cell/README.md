# MK-EVENT-FIRST-MEMBER-CELL

Card authority: ledger entry `MK-EVENT-FIRST-MEMBER-CELL`
(`docs/reviews/r0_3/acceptance_matrix.json` `named_followups`, registered by
MK-EVENT-DESCRIPTOR-QUANT; DISCHARGE: "read the first member (C11
6.7.2.1p15: a pointer to a structure, suitably converted, points to its
initial member) as the event's cell, with a test that a record whose first
member is not the event still refuses, measured"). Base `e589ee5a7`. The
prediction, committed before any code, is [PREDICTION.md](PREDICTION.md).

The vocabulary is the one the user approved for descriptor cells (USER
DECISION 2026-11-01, verbatim: "I approve the changes of D1-D5.";
specification C04.6.2). This card adds no clause spelling, and none was
needed: the change is in how rule 38 (1) keys an actual.

## What changed

- **The reading** (rule 38 (1)). A member cell `#data: mk_event.fd` used to
  be read only where Clang's type of every actual of the callee in the
  calling function, before the conversion to `void *`, was
  `struct mk_event *`. The type must still be one type in every call. It may
  now also be `struct R *`, where the unit's one complete definition of `R`
  declares its first member `m` with type `struct mk_event`. A place `X` of
  that kind (a name followed by member steps) is read as `&X->m`. The words
  are those the direct reading of `&X->m` is given, so the key is the same:
  a creation through `X` and a release through `&X->m` (or the reverse) name
  one cell. Everything else refuses with `descriptor-cell-undecided`, and
  the description says why:
  - a first member of another type: a pointer to or an array of the event,
    a `const` event, an integer, or another record, so a nested
    first-member chain refuses at its first level;
  - a later member, or a record with no member;
  - a union, `void *` (an explicit cast hides the record), or a pointer to
    a pointer;
  - an incomplete record, or one with more than one complete definition;
  - a mix of types at one callee in the function, even two individually
    readable ones (which call is which is not read), and an unknown type;
  - an actual that is not a plain place (`p + 0`).

  With `38` in `QCORE_GATEWAY_RULES_OFF`, every cell call still refuses,
  first-member actuals included.
- **Package.** `crates/gateway-rules/rule-38-descriptor-cells/src/initial_member.rs`
  holds the decision (`member_view`, a new classification arm
  `descriptor-cell-initial-member` in `registry.toml`, regenerated into
  `gateway_rule_registry.rs`) and `INITIAL_MEMBER_VECTORS`, shared with Lean.
- **Host.** In `crates/qcore-c-gateway/src/legacy_descriptor_cells.rs`,
  `operands` asks the view, rewrites the actual to its first member, and
  reads Clang's facts through `record_first_member`. Two small additions
  outside the card's files carry those facts:
  `union_bitfield::RecordFacts::first_members` (each named struct's first
  `FieldDecl`, name and desugared type, one entry per complete definition)
  and `IndirectCallUnit::records()` in `typed_access_path.rs`.
- **Lean.** `QCore.DescriptorCells` gains 11 audited theorems (27 in all,
  `propext` and `Quot.sound` only, no `sorry`, no project axiom):
  - `the_initial_member_is_the_event_cell`: the record pointer resolves to
    the place `&p->m` resolves to directly;
  - `the_cell_table_is_unchanged_under_the_initial_member`: for every key
    encoding, a creation, a release and the guard's lookup through the record
    pointer give what they give through `&p->m`;
  - `the_initial_member_is_at_the_record_address`: the first member's offset
    is 0, C11 6.7.2.1p15 modelled;
  - the refusals: `every_other_record_refuses`, `a_nested_first_member_refuses`,
    `a_non_event_first_member_refuses`, `an_empty_or_incomplete_record_refuses`,
    `an_undecided_or_other_type_refuses`, `a_mixed_call_set_has_no_type`,
    `a_refused_view_creates_nothing`;
  - `initial_member_vectors_hold`: the Rust vectors, entry for entry.
- **Tests** (`crates/qcore-c-gateway/tests/descriptor_cells.rs`, route G under
  the provider's own declarations). The previous card's
  `a_record_whose_first_member_is_the_event_refuses` is now
  `a_record_whose_first_member_is_the_event_is_read_as_it` (`Safe`: a
  destroy of a caller's record is the identity, as through `&w->event`). New:
  - `the_record_and_its_first_member_name_one_cell`: the four
    create/destroy spellings of a tracked wrapper are `Safe`. After a
    create through one spelling, a second destroy through the other is a
    double release, never undecided (the result had the keys differed).
    With no destroy, the owner's release refuses.
  - `a_record_whose_first_member_is_not_the_event_still_refuses`: nine
    shapes (a later event, a pointer, an array, a `const` event, a nested
    record, a union, an incomplete record, `(void *) p`, `p + 0`).
  - `a_mix_of_actual_types_refuses`: a wrapper with a later-event record,
    and a wrapper with `&q->event`.
  - `a_channel_through_its_first_member_record_is_safe`: the member-equality
    guard reads the first member too.
  - The rule-38-off child: a first-member actual refuses there as well.
- **Text.** Rule 38 (1) (`docs/agents/rules/38.md`, regenerated into
  `gateway.md` and the router) and C04.6.2's "Meaning at a call".

## The callers, read at the pinned revisions

These are the bodies at 66910c10 (corpus: `build/cycle19/c-gateway-wave-pin`,
Monkey `monkey/monkey-source`) and 046accf6 (system,
`private/w45-fb/w46-external-callee-contracts/attempt-1`). The slice export
is byte-identical to 046accf6 for `flb_input.c` (`92a0917b89dd…`) and
`flb_output.c` (`2d31e29a14f4…`). Each record's first member is
`struct mk_event event`. Every call of the four cell-bearing declarations
whose `data` actual is such a record:

| Caller | Call | Record | Now |
|---|---|---|---|
| `mk_server/mk_server.c::mk_server_worker_loop` (corpus; `mk_server.c` `07a15b43f175…`, identical in Fluent Bit's `lib/monkey`) | `mk_event_timeout_destroy(evl, server_timeout)` :586 | `struct mk_server_timeout` | **reads the cell**. The create at :502 is refused by the access gate first (`server->timeout`), so the table is empty. The release is the identity on custody (`P-CELL-STATE-NOT-FORWARDED`), and the row keeps 31 other leaves |
| same | `mk_event_timeout_create(evl, server->timeout, 0, server_timeout)` :502 | same | access gate first; unchanged |
| `mk_server/mk_scheduler.c::mk_sched_launch_worker_loop` (corpus) | `mk_event_channel_create(sched->loop, &sched->signal_channel_r, &sched->signal_channel_w, notif)` :365 | `struct mk_sched_notif` | access gate first; unchanged |
| `src/flb_output.c::flb_output_init_all` (system; :1463 at 66910c10, not a corpus unit) | `mk_event_channel_create(config->evl, &ins->ch_events[0], &ins->ch_events[1], ins)` :1545 | `struct flb_output_instance` | access gate first (array-element slots); unchanged, `MK-EVENT-CELL-ARRAY-SLOTS` |
| `src/flb_input.c::input_instance_channel_events_init` (slice; :1372 at 66910c10, not a corpus unit) | `mk_event_channel_create(evl, &ins->ch_events[0], &ins->ch_events[1], ins)` :1380 | `struct flb_input_instance` | same |

`flb_output_thread.c`'s `th_ins` calls (:418, :537, :560) have the same
shape, but no audited unit holds that file. Every other cell call passes
`&X->member` or a `struct mk_event *` (`event`, `ctx->event_channel`), or
is refused before rule 38.

## Prediction

No status moves on corpus, system or slice. `mk_server_worker_loop` (corpus)
loses its `descriptor-cell-undecided` leaf and stays `Unknown`. The rows of
the four channel creators above stay as they are, held by the access gate,
whose array-slot reading is `MK-EVENT-CELL-ARRAY-SLOTS`'s. No declaration,
lock or pin changes. A move to `Safe` would be a soundness finding.

## Measurement (the integrator's)

The worktree's isolation guard refuses command lines naming the main
checkout's prepared inputs, so this card did not run the measurement (brief,
"Measurement"). The command for the integrator, from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/mk-event-first-member-cell-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:e589ee5a7248dfd4afbcefd5b542e0de339c31dd \
  --side tip=rev:cf149f861 \
  --declared docs/reports/mk-event-first-member-cell/PREDICTION.md
```

**Result.** The integrator ran it on 2026-11-01, as
`private/measure/mk-event-first-member-cell-20261101T115209Z`. It exited 0
with status `complete` and no failures. The declared comparison `base->tip` is
`match` on corpus, slice and system, with no delta mismatches, no missing rows
and no unpredicted rows. The record is
[native-comparison.json](native-comparison.json)
(sha256 `fceb4c65a7b025dbb6f37d3416ffe57d06737f9e1d201572d3df18da34a5351c`;
PREDICTION.md sha256
`e9ed1171a954731609fccc94b11e00c5f87bfad1b12280d78fb07e7d6434e2fa`).

## Card gate

The gate ran as `python3 scripts/gate_scope.py --base e589ee5a7 --out
private/gate` on `cf149f861`, the last code commit, with a clean tree; only
this report, its amendments and [gate.json](gate.json) were added after it.
Scope: full, because the diff reaches `qcore-c-gateway`. Lean covers the
changed module and its audit (no `proof/` root changed). The provider gate
does not apply: no `providers/` path changed.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 590 units, 629 summaries; 9,000 passed, 0 failed, 15 ignored, 94 filtered |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| Lean (`QCore.DescriptorCells`, `AuditDescriptorCells.lean`) | 0 | 1 audit, 27 theorems |
| Python suite | 0 | 577 tests |

No audit file was added, so the matrix's derived Lean-audit count is
unchanged and the Python suite passes as committed. Logs are under
`private/gate/` in the card's worktree: `rust-suite/summary.json`, one log
per step, and the console log at `private/logs/gate-console.log`.

## Observations and follow-ups

- **`MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`** (registered). Take a function
  that passes a caller's record formal `c` whole to one cell call. A cell
  call there whose actual is a member address of that same formal
  (`&c->event` or `&c->r`) keeps the access gate's refusal. A member address
  of another formal is admitted. Rule 38 does not cause this: the refusal
  stays where rule 38 refuses the whole actual too (`(void *) c`). So a
  function that spells one record both ways stays `Unknown`. Fluent Bit's
  channel creators pass `ins` together with `&ins->ch_events[0]`, so the
  refusal may survive `MK-EVENT-CELL-ARRAY-SLOTS`.
- **A bare-local integer at the guard.** The guard's integer actual is keyed
  only as `X->f…` (`place_steps`; read from the code, not a fixture). A
  channel whose ends are locals (`&r`, `&w`), destroyed with `r`, leaves
  its guard undecided and refuses. That direction is fail-closed. No
  production caller does this: they all keep their ends in members.
- **One level only.** The reading does not follow a nested first-member
  chain (`struct A { struct B b; }`, `struct B { struct mk_event e; }`).
  Every such shape refuses. No production caller needs it.
