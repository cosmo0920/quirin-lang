# RETAINING-CALL-MINTED-NAME-ACTUALS

Card authority: ledger entry `RETAINING-CALL-MINTED-NAME-ACTUALS`
(`docs/reviews/r0_3/acceptance_matrix.json` `named_followups`, registered by
[MK-EVENT-CELL-ARRAY-SLOTS](../mk-event-cell-array-slots/README.md)). Its
DISCHARGE reads: "match the minted name at every withheld declaration (or
classify under the plain name), with fixtures per declaration family and a
declared measurement of the rows it touches". Base `a9f7bebbe`. User
direction (2026-11-02, verbatim): "We need to launch these cards."

The prediction was committed before the implementation (`08e187cfd`):
[PREDICTION.md](PREDICTION.md). No contract, lock or pin changes.

## The choice: match the minted name

Rule 12t reaches its route as the classifier's uncontracted fallback: a
withheld declaration is not in the function map. The route then classifies
the call again with the callee renamed to `qcore_retained__<callee>`, under
which `retaining_environment` installed the declaration's custody contract.

Classifying under the plain name instead would find no contract. That is the
fallback that called the route, so the minted name is the only spelling the
inner classification can use. It would also need changes in
`legacy_retained_by.rs`, which belongs to the concurrent card. The custody
contract differs from the declaration only in its result
(`custody_contract`: a keep's result is `None`), so the positions the probe
read are the positions in force. So the card matches the minted name, as rule
38a did, but at every withheld declaration.

## What changed

Gateway rule 38b ([rule text](../../agents/rules/38b.md)) lives entirely in
`crates/qcore-c-gateway/src/legacy_call_argument_access.rs`. Rule 38a's text
records the amendment.

1. **The flag** (`minted_admission`). An admitted entry is also found under
   the minted name when one of these holds:
   - its call is cell-bearing, so every admitted actual qualifies (rule 38a,
     unchanged);
   - it sits at a declared `Borrowed` position of any other withheld
     declaration and is not a rule-1 release.

   A function-map callee's entries are never minted.
2. **The match** (`AdmittedArgument::matched`). The minted name matches only
   the entry's own callee, at the same index and with the same tokens. Hook
   (a) records when an actual left the gate under the minted name
   (`minted_used`).
3. **The position in force** (`minted_positions_borrowed`). For rule 13u,
   the route reads a callback's requirement as `resource_owned` on the
   call's own position after the inner classification. That is not a read.
   So every actual that left the gate under the minted name must sit at a
   `Borrowed` position of the classified call, or the probe restores the
   gate's refusal.

The probe's other premises are unchanged. In particular, the probe applies
only when the re-classification is a contracted call. So where the route
itself refuses, the gate's refusal is restored byte for byte. That covers an
undecided retention spelling, a registry holder the body does not name, and
an open callback set. Rule 38a's element admission stays limited to
cell-bearing declarations. A by-value element at `mk_event_add`'s `fd` is
admitted only because W44 already reads it as a typed scalar.

**Lean.** `QCore.RetainingCallMintedName`, audited by
`AuditRetainingCallMintedName.lean`, holds 26 theorems. It uses only
`propext`, `Quot.sound` and `Classical.choice`, with no `sorry` and no
project axiom. It reuses `QCore.CellArraySlots.matchesName`, whose flag is
now the entry's `mintedAdmission`. That flag equals rule 38a's cell-bearing
flag wherever cells exist (`rule_38a_is_the_cell_bearing_case`,
`off_a_withheld_declaration_the_flag_is_rule_38a_s`). Proved:

- **the minted name admits exactly the plain-name set:**
  - a minted entry matches its minted name as its plain name;
  - the minted name matches only its own callee;
  - over any entries it never admits more, and admits exactly as much when
    every entry is minted;
- **fail-closed:**
  - a function-map callee, an uncontracted callee, a custody position, a data
    tail and a release are never minted without cells;
  - a refused actual is admitted under no name;
  - a minted actual at an `owned` position in force keeps the refusal, and an
    admitted probe reads every minted actual `Borrowed`;
- **the shared table:** `minted_admission_vectors_hold`, entry for entry with
  `MINTED_ADMISSION_VECTORS`, also checked by a Rust unit test;
- **non-vacuity:**
  - the brief's `mk_event_add` actual is admitted under the minted name;
  - `flb_task_destroy`'s release is admitted under its plain name only;
  - another callee's minted name admits nothing.

**Tests.** `crates/qcore-c-gateway/tests/retaining_call_minted_name.rs` holds
17 tests. Each asserts the complete diagnostic vector, and every `Safe`
certificate is rechecked. None calls a cell-bearing declaration, so rule 38
and the lifetime summaries (card MK-EVENT-CELL-CROSS-FUNCTION) do not reach
them. In the table, *before* is the same fixture with the minted flag
limited to cell-bearing calls, measured in the worktree before the switch
was removed.

| Family | Fixture | Before | Now |
|---|---|---|---|
| Monkey (`monkey/event_loop.qv`) | `mk_event_add(loop, c->ch[0], …, &c->event)` over a tracked record | gate, `c` | `retention-undecided`, `c` |
| | `mk_event_del(config->evl, &coll->event); ev_free(coll)` (`flb_input_collector_destroy`) | gate, `coll` | `coll` |
| | `mk_event_add(s->evl, …, &s->event); mk_event_del(s->evl, &s->event)` over a caller's record | `Safe` (an older route) | `Safe` |
| | `mk_event_add(loop, s->fd, …, &s->p)` (W44 refuses `&s->p`) | gate, `s` | gate, `s` |
| Collector setters | `flb_input_set_collector_event(in, cb, ctx->ch_manager[0], config)` (`in_tail_init`) | gate | **`Safe`** |
| | `flb_input_set_collector_time(in, cb, ctx->refresh_sec, ctx->refresh_nsec, config)` | `Safe` (an older route) | `Safe` |
| msgpack | `msgpack_packer_init(&ic->mp_pck, ic, cb)`, tracked `ic` / caller's `ic` | gate | gate (rule 12t refuses, so the gate is restored) |
| librdkafka | `rd_kafka_queue_new0(rk, rk->rk_rep)` (`rd_kafka_queue_get_main`) | gate | `retention-undecided` at the exit |
| | `rd_kafka_q_destroy_owner(rkqu->rkqu_q)` (`rd_kafka_queue_destroy`) | gate | `reference-drop-unheld` |
| Refusals | a consumed position, a rule-1 release, `&s->ch[0]` at a declaration without cells, a callback's `resource_owned` on an admitted position | gate | gate |
| | an uncontracted callee | uncontracted | uncontracted |
| Controls | a function-map callee | `Safe` | `Safe` |
| | `mk_event_del(c->evl, …)` after `ev_free(c)` | `Unsafe` | `Unsafe` |

The callback fixture passes over a tracked `s`. Over a caller's formal, an
older route (C02-P) already exempts `s->ctx`, before and after this card. In
`tests/cell_array_slots.rs`, `a_withheld_declaration_without_cells_keeps_the_refusal`
is now `a_withheld_declaration_without_cells_reads_the_element_value_as_a_scalar`.
That is the same fixture, which now reaches rule 12t's `retention-undecided`.

The first card gate also failed one test outside the card's files:
`function_pointer_declarators`'s
`an_array_of_function_pointers_still_refuses_the_record`. It asserted the
gate's refusal at the withheld `ev_add` / `ev_del` over a record the lexical
reader refuses whole (`int (*cbs[2])(void)`). Rule 10k's Clang-typed actual
admits `coll->evl` there under the plain name, and that admission now leaves
the inner classification too. So the cycle is `Safe`, as over the readable
record, and the registration alone reaches `retention-undecided`. The test
is now `an_array_of_function_pointers_is_read_through_the_typed_actual`
(`e8b507b90`, expectations only). The reader's own refusal is still pinned by
`typed_field_access`'s unit tests.
The unit tests `minted_admission_vectors_hold` and
`the_minted_name_matches_exactly_the_plain_admission` cover the flag and the
match.

## The rows

The relisting and the per-family reading are in [PREDICTION.md](PREDICTION.md).
332 corpus, 4 system and 10 slice rows carry a gate leaf at a withheld call
without cells that has a memory-reading actual at a `Borrowed` position.

The prediction is **no status move on any route**:

- 278 / 4 / 8 of those rows keep a leaf on another statement;
- every one of the other 56 was read, and each family was run on fixtures
  under the baseline route's own contract sources. No such row reaches
  `Safe`:
  - the route refuses `rd_list_*` registry holders, `(void *)` interceptor
    callbacks, `&local` confval pointers and the open `rko_serve` callback
    set, so the gate's refusal is restored;
  - the admitted calls answer `retention-undecided`, `reference-drop-unheld`,
    or "arguments do not match resource contract for `bsearch`";
  - the unresolved identities that stay are `coll`, `rkqu`, `ret` and `cmp`.

`rd_kafka_broker_request_supported` is the least certain row. A fixture with
a hand-written `bsearch` prototype was `Safe`. The near-verbatim fixture
under glibc's `<stdlib.h>` is `Unknown`, because the comparator's requirement
makes `&skel` an `Owned` position.

## RETAINING-CALL-MEMBER-ACTUAL

Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION discharged
`RETAINING-CALL-MEMBER-ACTUAL` by reading a withheld declaration's borrowed
positions as the function map's. Until this card, that held in the outer
classification only, except at the four cell-bearing declarations.

It now holds **in full for every withheld declaration's `Borrowed`
positions**, in both classifications. Two things are not covered, and
neither is a borrowed position of a declaration:

- **Inferred lifetime summaries.** A same-unit helper or a replayed import
  is classified under a minted name too (`retaining_environment`), and the
  probe reads its contract (`lifetime_summary`). Its entries are not minted,
  because it is not a retaining declaration. Its admitted actuals still
  leave only the outer gate. This is follow-up
  `RETAINING-CALL-MINTED-NAME-LIFETIME-SUMMARIES`, in
  MK-EVENT-CELL-CROSS-FUNCTION's area.
- **Rule-1 member releases** (W45-CALLEE-SUMMARY-CASCADE) at a withheld
  declaration's nullable release position are deliberately not minted: the
  position takes custody.

A declaration under a scoped (conditional) lifetime is outside
`retaining_contract`. The probe reads it as uncontracted and keeps the
refusal, as before.

## Measurement

The integrator ran the measurement from the main checkout, because the
worktree's isolation guard refuses command lines naming the prepared inputs
there. One run was made on 2026-11-02:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/retaining-call-minted-name-actuals-20261102T032109Z \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:a9f7bebbe506ab9d1b8657b990660e99c69f69ea \
  --side tip=rev:e8b507b90 \
  --declared docs/reports/retaining-call-minted-name-actuals/PREDICTION.md
```

It exited 0 with status `complete` and no failures. The declared comparison
is `match` on corpus, slice and system, with no delta mismatches, no missing
rows and no unpredicted rows. There are 0 status moves, 0 unit moves and 0
unmatched rows on every route, so `rd_kafka_broker_request_supported` (the
least certain prediction) stays where it was. Only leaves and contract
dependencies change: on 40 corpus rows and 6 slice rows, each keeping its
status; no system row changes. The record is
[native-comparison.json](native-comparison.json) (sha256
`eab4afcf986611ce098e63cc03f4b64ad49e2182a51f579a2df9dd18a2570a06`;
PREDICTION.md sha256
`33257a2947a8f14600257f711f1845dfa29fc8c3b8024314c9e3afeac565855a`).
`e8b507b90` is the card's last code commit; `ac406b81e` adds only this
report, its gate record and its amendments.

## Card gate

The gate ran as `python3 scripts/gate_scope.py --base a9f7bebbe --out
private/gate-2` on `e8b507b90`, the last code commit, with a clean tree.
Only this report, its amendments and [gate.json](gate.json) were added after
it. The scope was the full gate, because the diff reaches `qcore-c-gateway`.
Lean ran in full, because `proof/QCore.lean` changed. The provider gate does
not apply.

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 checked |
| fmt | 0 | |
| clippy | 0 | |
| Rust suite | 0 | 592 units, 632 summaries; 9,037 passed, 0 failed, 15 ignored, 109 filtered |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| Lean audits | 0 | 401 |
| Python suite | **1** | 577 tests, 1 failure |

**The Python failure.** The one failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`.
The matrix's derived `lean_audits` is 400, and the tree gives 401 with the
new `AuditRetainingCallMintedName`. The integrator regenerates the matrix at
landing.

**The first gate run** (`private/gate` on `b0fd45039`) exited 1. The Rust
suite had 1 failure (9,036 passed), the same Python failure, and every other
step passed. The Rust failure was
`function_pointer_declarators::an_array_of_function_pointers_still_refuses_the_record`
(see Tests). `e8b507b90` changes that test's expectation only.

**Logs.** Logs are under `private/gate-2/` (and `private/gate/` for the first
run) in the card's worktree. The console log is outside it, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/rcm-gate-console.log`.
The second run overwrote the first run's console log.

## Observations and follow-ups

- **`RETAINING-CALL-MINTED-NAME-LIFETIME-SUMMARIES`** (registered). See
  above.
- **`QCore.CellArraySlots`'s wording.** `matchesName`'s first argument is
  named `cellBearing`, and `without_cells_the_minted_name_never_matches`
  reads as rule 38a's rule. Both are still true of the definition. The
  argument is now the entry's `mintedAdmission` flag, which this card's
  module proves equal to the cell-bearing flag at a cell-bearing
  declaration. The module is not this card's, so its text is left to the
  integrator.
- **Fixtures, not measurement.** The per-family outcomes above come from
  fixtures under the route's contract sources, with the unit's other
  functions reduced to declarations. They do not have the run's callback
  census or the link map. The integrator's measurement is the evidence.
