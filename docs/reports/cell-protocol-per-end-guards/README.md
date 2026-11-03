# CELL-PROTOCOL-PER-END-GUARDS

Card brief: `private/integrator-briefs/r9-common.md` (round 9, 2026-11-03).
Base `1b859bdc9` (origin/main), baseline `baseline:1b859bdc9`. It discharges
the `named_followups` entry CELL-PROTOCOL-PER-END-GUARDS, registered by
[CELL-PROTOCOL-CLOSE-ACTUAL](../cell-protocol-close-actual/README.md) and
amended by [CELL-PROTOCOL-FB-DECLARATIONS](../cell-protocol-fb-declarations/README.md)
and [CELL-GUARD-RESULT-RANGE](../cell-guard-result-range/README.md).

> **USER DECISION 2026-11-03** (verbatim, on the choice between "a sentinel
> on each channel end, or several protocols per record"): "Per-end
> sentinels".
>
> **USER DIRECTION 2026-11-03** (standing, verbatim): "The Fluent Bit's caller
> tests sometimes check the incorrect assertions so we need to pay attention
> for the assertions and actual returning values of Fluent Bit functions."

The [prediction](PREDICTION.md) was committed at `54e9d0397`, before any code.

## 1. What changed

**The grammar** (G05i, amended in place; G05k and rule ids 38f+ were free).
A record's `quant cell` clauses are grouped by guard. Besides the one record
guard G05i had (a flag, or a sentinel over several `#fresh` cells), each
`#fresh` cell may carry its own sentinel: a **per-end sentinel** is a `!= -1`
or `> 0` guard over exactly one `#fresh` cell, that cell itself, with its
`#same` names under the same guard. A record states at most one guard that is
not per-end. The groups load to one `CellProtocol` each
(`LoopLifetimeContract::cells`, sorted; a one-guard record serializes as
before), and `ResourceContracts::cell_protocols` keeps a record's groups only
when every declaration states the same set. New malformed vectors: two record
guards, two flags, a `#same` under another end's guard, two binders.

```qcore
quant cell #o: flb_output_instance.ch_events[0] |-> #fresh(descriptor) when #o.ch_events[0] > 0;
quant cell #o: flb_output_instance.ch_events[1] |-> #fresh(descriptor) when #o.ch_events[1] > 0;
quant cell #o: flb_output_instance.event.fd |-> #same(ch_events[0]) when #o.ch_events[0] > 0;
```

**Rule [38f](../../agents/rules/38f.md)** (new; rule 38c's text and C04.6.3
point to it). In the package (`protocol.rs`, `cells.rs`) and the host
(`legacy_descriptor_cells.rs`, `legacy_transfer.rs`):

- **One fact per guard.** A path keys a fact by (record object, guard)
  (`FactKey`). A test, store or close reads the group whose guard or cell it
  names. At a cell-bearing call a per-end group gains a fact only through a
  cell of its own the call spells; a record guard keeps rule 38c's reading
  (every actual rooted at the record, CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD).
- **The `> 0` dead edge.** A sentinel's dead edge decides "no cell is live"
  only when no possible descriptor is on it (rule 38's premise: a live
  descriptor is never `-1`). `!= -1`'s does. `> 0`'s holds descriptor 0, a
  success value of every creator and calloc's absent value at once, so it
  never decides: its cells are introduced `Maybe` and the fact is marked
  `zero`. Its live edge decides. This is the user's reading: the meaning comes
  from the creators' result set (descriptors, 0 included) and the absent
  value, never from the destroyer's test. It also applies to rule 38c's
  record-level `> 0` guard (fixture-only; no production record guard is
  `> 0`).
- **Joins.** Each path's pending names whose cell either path recorded go
  into its own table first (`Fact::normalize`), a pending name live or
  undecided on one path is undecided in the join (`Fact::join`), and a
  protocol cell both paths left stale stays stale whatever identities they
  name (`Cells::join_keeping_stale`). Before, a join kept only the pending
  names both paths shared, so a name live on one path could vanish.
- **The holder's release ends the protocol.** A call releasing the record (an
  actual naming it at a `resource_released` position) ends every fact of it
  and forgets its cells. Where a cell may still be live it refuses:
  `descriptor-cell-zero-undecided` when a `> 0` dead edge left the end
  undecided (new reason key; a candidate defect, `Unknown`, never `Unsafe`),
  else `descriptor-cell-owner-released`. A tracked holder refuses as before.

**What changed for existing fixtures** (every change adds a refusal or names
the release; none removes a refusal from a row that could become `Safe`):

| Fixture | Change |
|---|---|
| `cell_protocol_close_actual.rs`: `fluent_bits_output_instance_ends_refuse_after_admission`, `a_close_on_the_dead_edge_refuses`, `closes_that_leave_the_sentinel_break_the_protocol`, `a_second_close_refuses`, `a_non_release_position_keeps_the_gate_refusal`, `an_uncontracted_close_keeps_the_gate_refusal` | the `> 0` dead edge's ends are undecided: `descriptor-cell-escapes` added (and `protocol-broken` where it was not reported) |
| `cell_protocol_close_actual.rs`: `a_close_under_the_dead_guard_refuses` | the `<= 0` edge's close still refuses; `protocol-broken` and an escape added |
| `cell_protocol_close_actual.rs`: `a_close_through_a_released_record_refuses` | `ev_free(s)` with a live cell now refuses `descriptor-cell-owner-released` |
| `descriptor_cell_protocols.rs`: `a_conditional_teardown_that_breaks_the_protocol_refuses` | the untouched arm's live names survive the join: three `descriptor-cell-escapes` added |
| `descriptor_cell_protocols.rs`: `a_creator_that_does_not_kill_the_sentinel_first_refuses` | `ev_free(c)` ends the protocol, so the call no longer also reports it broken; the row keeps `owner-released` and both `undecided` |
| `cell_protocol_fb_declarations.rs`: the inventory test | the module now states three records' protocols; renamed `the_production_module_declares_the_thread_instance_flag_protocol` |

## 2. The holder's release and the window before it (no stop)

The DISCHARGE names the destroyers that store no sentinel after the close and
use the record before releasing it. "Per-end sentinels" plus fail-closed
settle that window, so the card did not stop:

- **The window.** After a live edge's close with no sentinel store, the end
  still looks live (`> 0`) while its descriptor is closed. A call that may
  reach the record there could read the end as live and close it again, so it
  refuses (`descriptor-cell-protocol-broken`), as rule 38c already did.
- **The release.** The holder's release ends the protocol: nobody can read
  the record after it, so a closed end left looking live is no longer a
  claim. It refuses only where an end may still be live.
- **Why no choice is needed now.** Every Fluent Bit destroyer of these ends
  tests `> 0`, so each release refuses `descriptor-cell-zero-undecided`
  whatever the window reads. Deciding the window (for example a frame
  declaration stating that `flb_output_free_properties` reads no end) could
  move nothing until an end test reads the holder's own sentinel. The options
  are recorded in follow-up CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (§9).

## 3. The declarations

Hosted on the existing vehicle `flb_config_set_program_name`
(`providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`),
beside the thread instance's flag group, so a record's groups stay in one
declaration:

- `flb_output_instance`: `ch_events[0] > 0` (with `event.fd`
  `#same(ch_events[0])`), `ch_events[1] > 0`;
- `flb_input_instance`: the same, and `ingress_queue_channels[0..1] > 0`;
- `flb_input_thread_instance`: `ch_parent_events[0] > 0` (with `event.fd`),
  `ch_parent_events[1] > 0`, `ch_thread_events[0] > 0` (with
  `event_local.fd`), `ch_thread_events[1] > 0`.

`> 0` states the absent set "every value at or below 0": calloc's 0 for the
`ch_*` ends and the stored -1 of `ingress_queue_channels` (§4). It claims less
than `!= -1` would, and the dead edge's descriptor 0 stays undecided either
way.

**Withheld: `flb_config`** (`ch_manager`, `ch_notif`, `ch_event.fd`). Declared
alone, its per-end text would put the `Safe` row `flb_engine_started` (its
`ch_notif[1] <= 0` test) on the declaration while reading nothing there, and
the record's guard is its flag protocol, withheld for
CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD. Its text, for that follow-up (the
fixture `the_config_exit_ends_under_the_withheld_text` reads it):

```qcore
quant cell #c: flb_config.ch_manager[0] |-> #fresh(descriptor) when #c.ch_manager[0] > 0;
quant cell #c: flb_config.ch_manager[1] |-> #fresh(descriptor) when #c.ch_manager[1] > 0;
quant cell #c: flb_config.ch_event.fd |-> #same(ch_manager[0]) when #c.ch_manager[0] > 0;
quant cell #c: flb_config.ch_notif[0] |-> #fresh(descriptor) when #c.ch_notif[0] > 0;
quant cell #c: flb_config.ch_notif[1] |-> #fresh(descriptor) when #c.ch_notif[1] > 0;
```

**Pins.** `quirin-audit pin --source-root <tree> --crate
quirin-provider-fluent-bit --function flb_config_set_program_name` at 046accf6
(system), 34155955 (slice) and 66910c10 (corpus): the contract digest is new
(`7f2109e9…`), the sites are unchanged, one identity covers the three
revisions. `providers/contracts.py sync` regenerated `contract-inventory.lock`:
`teardown_destroy.qv` 147 → 161 clauses, the host 7 → 21; declarations
unchanged (15). `providers/contracts.py check` exits 0.

## 4. The census

[`census/census.py`](census/census.py) is CELL-PROTOCOL-FB-DECLARATIONS'
lexical store reader with this card's records, readings and an end-test
classifier. It binds itself to every file it reads (one SHA-256 tree digest
per tree) and refuses an unannotated site, an unused annotation or a changed
digest:

```sh
python3 -I docs/reports/cell-protocol-per-end-guards/census/census.py \
  --root 66910c10=<corpus fluent-bit-clang-source> \
  --root 046accf6=<system fluent-bit-clang-source> \
  --root 34155955=<slice fluent-bit-clang-source> \
  --check docs/reports/cell-protocol-per-end-guards/census/census.json
```

It exits 0 with 0 problems (`census.json` sha256
`e9763068867be60b33132d60b70e1cf7afc24ec5d5f9312d600986110f5744ad`): 1,175 /
1,182 / 1,184 files read, 93 / 94 / 94 sites.

**Stores** ([annotations.json](census/annotations.json), 33 non-read sites,
the same at the three trees up to line numbers). Every one keeps the per-end
claim (`> 0`: live; `<= 0`: absent or descriptor 0): the allocations
(calloc's 0), the creations (`mk_event_channel_create`, `flb_pipe_create`,
the record handed as its first-member event), the `-1` stores of
`ingress_queue_channels` (flb_input.c:549-550, flb_input_ingest.c:493-494),
`flb_engine_failed`'s `-1` into `ch_notif[1]`, and `MK_EVENT_ZERO`'s -1 in
`ch_event.fd`; `event.type` stores are outside. The destroyers' closes are
reads: they break the claim until the holder's release (§2).

**End tests** (USER DIRECTION 2026-11-03). Each comparison of a declared (or
withheld) end with a constant is classified against the end's value set: the
absent values its stores write (`ABSENT`: calloc's 0; `ingress_queue_channels`'
-1; `ch_notif`'s -1 and 0; `ch_event.fd`'s -1) and the descriptor values the
creators return ([0, MAX]). `exact`: present exactly on the descriptors;
`redundant`: right on the value set, different outside it; `wrong`: a value of
the set misread. Where the absent value is itself a descriptor value (calloc's
0), no test of that end can be exact.

| Tree | Exact | Redundant | Wrong |
|---|---|---|---|
| 66910c10 | 0 | 0 | 16 |
| 046accf6 | 0 | 0 | 16 |
| 34155955 | 0 | 0 | 16 |

Every wrong site (046accf6 = 34155955; 66910c10 in brackets):

| Site | Test | Why |
|---|---|---|
| flb_output.c:615 [537], :619 [541] `flb_output_instance_destroy` | `ch_events[0..1] > 0` | descriptor 0 read as absent; calloc's 0 is also a descriptor value |
| flb_input.c:1311 [1303], :1315 [1307] `flb_input_instance_destroy` | `ch_events[0..1] > 0` | the same |
| flb_input.c:1319 [1311], :1323 [1315] `flb_input_instance_destroy` | `ingress_queue_channels[0..1] > 0` | descriptor 0, a success value, read as absent (absent is -1) |
| flb_input_thread.c:210, :213, :218, :221 `input_thread_instance_destroy` | `ch_parent_events[0..1] > 0`, `ch_thread_events[0..1] > 0` | descriptor 0 read as absent |
| flb_config.c:617 [616] `flb_config_exit` | `ch_manager[0] > 0` | the same |
| flb_config.c:625 [624], :627 [626] `flb_config_exit` | `ch_notif[0] > 0`, `ch_notif[1] > 0 &&` | the same |
| flb_config.c:607 [606] `flb_config_exit` | `if (config->ch_event.fd)` | the absent -1 (`MK_EVENT_ZERO`) read as live, and 0 as absent |
| flb_engine.c:1034 [1018] `flb_engine_started`, :1048 [1032] `flb_engine_failed` | `ch_notif[1] <= 0` | descriptor 0 read as no channel |

These are candidate defects, source readings only; nothing is filed upstream.
Each is reachable only when a creator returns descriptor 0 (standard input
closed at start). Rule 38f reads none of these tests as an end's absence: a
`> 0` live edge decides its close, its dead edge leaves the end undecided, and
the holder's release names it (`descriptor-cell-zero-undecided`).

## 5. Tests

`crates/qcore-c-gateway/tests/cell_protocol_per_end_guards.rs`, 15 tests under
the production `teardown_destroy.qv` and Monkey's `event_loop.qv`, each
asserting the complete vector with body lines:

| Fixture | Body | Outcome |
|---|---|---|
| `the_production_module_states_per_end_sentinels` | the inventory | 11 guards: 10 per-end `> 0`, the thread instance's flag |
| `the_output_instance_destroyer_names_descriptor_zero_at_the_release` | flb_output.c:615-638 ends, a use, `flb_free` | `protocol-broken` at the use, `zero-undecided` at the release |
| `the_output_ends_released_at_once_refuse_only_for_descriptor_zero` | the ends, `flb_free` | `zero-undecided` only (with the caller-record release's `unknown_alias`) |
| `the_output_ends_after_an_opaque_call_are_still_read` | an uncontracted call first (the production state) | the tests are read (rule 38c's premise covers stores by uncontracted callees), the closes decided; broken with two escapes at the exit |
| `closed_ends_with_their_sentinels_reset_keep_the_protocol` | close and store 0 per end | `Safe`: forwarded, the dead edge's undecided ends with it |
| `a_close_on_a_positive_dead_edge_refuses` | `<= 0` then close | `undecided`, broken, escape |
| `the_input_instance_destroyer_reads_four_ends` | flb_input.c:1311-1353 | `protocol-broken` at the use, `zero-undecided` at the release |
| `the_ingress_sentinel_stores_keep_the_protocol` | the `-1` stores | `Safe` |
| `the_thread_instance_destroyer_reads_its_flag_and_four_ends` | flb_input_thread.c:194-227 | flag decided; `protocol-broken` at the use, `zero-undecided` at the release |
| `the_thread_ends_after_the_gate_refusal_are_read` | after `mk_event_loop_destroy`'s gate refusal (production) | the close decided; `zero-undecided` at `flb_free` |
| `the_config_exit_ends_under_the_withheld_text` | flb_config.c:606-631 | `ch_manager[0]`, `ch_notif[0]` decided; `ch_event.fd`, `ch_manager[1]`, `ch_notif[1]` keep the gate's refusal (no own test); `zero-undecided` at the release |
| `the_engine_started_test_reads_nothing_it_touches` | flb_engine.c:1032-1040 | `Safe` (untouched on both edges) |
| `a_creator_never_establishes_a_positive_sentinel` | flb_input.c:1372-1399 | both creations `undecided`, broken and escapes at the exits |
| `a_minus_one_sentinel_destroyer_ends_its_protocol_at_the_release` | `!= -1` ends, `flb_free` | the protocol ends silently; only the caller-record release's `unknown_alias` |
| `a_use_between_the_closes_and_the_release_refuses` | `!= -1` end, a use, `flb_free` | `protocol-broken` at the use |

Package: `the_per_end_vectors_hold` (17 vectors), `the_per_end_vectors_are_the_lean_ones`
(read from the Lean text), `only_the_positive_dead_edge_leaves_its_cells_undecided`,
`a_join_never_drops_a_name_that_may_be_live`, `per_end_is_one_fresh_cell_guarding_itself`.
Loader: `per_end_sentinels_load_one_protocol_per_guard` and five new malformed
spellings.

## 6. Lean

`QCore.CellProtocolPerEnd` (`proof/QCore/CellProtocolPerEnd.lean`, audited by
`AuditCellProtocolPerEnd.lean`; standard axioms only, no `sorry`, no project
axiom), 14 theorems: `dead_edge_decides_iff_no_descriptor_on_it`,
`descriptor_zero_is_on_the_positive_dead_edge`,
`the_live_edge_is_disjoint_from_the_dead_set`,
`the_positive_dead_edge_never_reads_absent`,
`the_minus_one_dead_edge_reads_absent`,
`a_close_on_the_positive_dead_edge_refuses`,
`a_join_never_drops_a_possibly_live_name`,
`the_table_join_keeps_a_possibly_live_name`, `two_stale_names_join_stale`,
`a_holder_release_ends_the_protocol_only_without_a_live_cell`,
`a_zero_reading_names_the_release`,
`a_record_keeps_its_protocols_iff_every_end_does`,
`a_record_release_refuses_iff_some_end_may_hold_live`, and
`per_end_vectors_hold` (Rust `PER_END_VECTORS`, entry for entry, bound by the
package test that reads the Lean text). The registry lists the module under
rule 38.

## 7. Prediction and measurement

[PREDICTION.md](PREDICTION.md) predicts `none` on corpus, system and slice:
no status move. Its row table was written before the fixtures and expected
the destroyers' tests unread where `ins`/`thi` is unresolved; the
production-state fixtures show that rule 38c reads a declared protocol's test
there too (its premise covers stores by uncontracted callees), which is the
prediction's "where this could be wrong" case. The expected content changes,
none a status move:

- **system `flb_output_instance_destroy`** (×2): the gate refusals at :616 and
  :620 go (the closes are decided on their live edges); `zero-undecided` at
  `flb_free` (:638); possibly `protocol-broken` at a contract call reaching
  `ins` before it (:631); the host joins the contract dependencies.
- **slice `flb_input_instance_destroy`** (×2): the gate refusals at :1312,
  :1316, :1320, :1324 go; `zero-undecided` at :1353; possibly
  `protocol-broken` at :1335 or :1340; the host joins the dependencies.
- **slice `input_thread_instance_destroy`** (×2): the gate refusals at :211,
  :214, :219, :222 go; `zero-undecided` at :226.
- **slice `input_instance_channel_events_init`** (×2): if `ins` resolves at
  :1380, `descriptor-cell-undecided` at the creation of each end and
  `protocol-broken` / `escapes` at the exits; the host joins the dependencies.
- **system `flb_output_init_all`, slice `flb_input_new`** (×2 each): the host
  may join the dependencies; no leaf change expected.
- Rows already depending on the host may show a changed assumptions digest.

The worktree's isolation guard refuses the live measurement, so the
integrator runs it from the main checkout:

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/measure/cell-protocol-per-end-guards-<stamp> \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:1b859bdc9ad16759428d4a99628aa08d75f13101 \
  --side tip=rev:<last code commit> \
  --declared docs/reports/cell-protocol-per-end-guards/PREDICTION.md
```

## 8. Card gate

`python3 scripts/gate_scope.py --base 1b859bdc9 --out private/gate` ran on
`22b4c8fa5`, the last code commit, with a clean tree ([gate.json](gate.json),
`uncommitted: []`). Only this report's README, its amendments and gate.json
were added after it. The scope was full: the diff reaches `qcore-c-gateway`,
`qcore-schemas` and `qcore-contract`, a proof root changed (all Lean), and
`providers/` changed (provider gate).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 |
| fmt, clippy | 0, 0 | |
| Rust suite | 0 | 601 units, 642 summaries; 9,147 passed, 0 failed, 16 ignored, 135 filtered |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean | 0 | 405 audits |
| Python suite | **1** | 577 tests, 1 failure (below) |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider `cargo test` | 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The one Python failure is the expected stale record:
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`,
`['24.0.0', 404] != ['24.0.0', 405]`: the committed matrix's Lean-audit count;
this card adds `AuditCellProtocolPerEnd.lean`. The integrator regenerates the
records.

The logs are under `private/gate/` in the worktree; the console log is
outside it, at
`/tmp/claude-1000/-media-Data-GitHub-quirin-lang--claude-worktrees-launch-cards-clean-worktrees-5365ff/6fcd7fbd-1cae-4a40-a37d-5bce071c02c6/scratchpad/pereg/gate-console.log`
(the background task's output, with the gate's own summary, is the same
run's).

## 9. Follow-ups

- **Registered: CELL-PROTOCOL-RELEASE-WINDOW-FRAMES.** Between a close that
  stores no sentinel and the holder's release, a call that may reach the
  record refuses. Deciding it needs a choice: (a) callee frame declarations
  (a declared or body-checked "reads no end member" clause for
  `flb_output_free_properties`, `flb_storage_input_destroy`,
  `flb_tp_destroy`, `mk_list_del`); (b) a member-actual narrowing (an actual
  `ins->processor` or `&ins->_head` reaches no end), unsound under
  `container_of`; (c) keep the refusal. Recommended: (c) until an end test
  reads the holder's own sentinel, then (a). No row moves under any choice
  today, since every Fluent Bit release of these records refuses
  `zero-undecided` first.
- **Amended: CELL-GUARD-DESCRIPTOR-ZERO-READ-AS-ABSENT.** The channel
  destroyers' `> 0` ends (formerly tracked here) and the census's other
  wrong end tests (`ch_event.fd`, `ch_notif[1] <= 0`) are rehomed there.
- **Amended: CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD.** A per-end group
  already gains a fact only through its own cells; the record guard keeps
  the issue. `flb_config`'s per-end text (§3) belongs with its flag protocol.
- **Amended: FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME.** The `> 0` channel
  records are declared under per-end sentinels.
