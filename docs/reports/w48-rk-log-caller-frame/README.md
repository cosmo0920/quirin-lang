# W48-RK-LOG-CALLER-FRAME: librdkafka's logger read from its callers' frame

Gateway rules [32](../../agents/gateway.md#constraint-32) and
[32a](../../agents/gateway.md#constraint-32a). On the user's decision of
2026-10-22: a caller-facing contract with named premises, challenged for
whether callers can be Safe outright. The card was halted on 2026-10-23
([halt record](../w48-impl-round1-halt/README.md)) with one blocking review
item; this report closes the resumed card.

## Outcome

- **Callers are conditionally Safe, never Safe outright.** The challenge's
  answer is in rule 32's checked projection (`caller-frame.json`,
  `qcore_contract::caller_frame`): the filtered, `rk == NULL`, `!log_queue`,
  `!rk->rk_logq` and no-callback paths, the disabled-queue reply of the fresh
  op, the copied serve callback and the balanced sequential forward keep/drop
  are proved without a premise; three effects stay caller-visible and are
  named premises of both declarations
  (`providers/qcore-provider-librdkafka/contracts/librdkafka/log_caller_frame.qv`):
  - `rk_plugin_callback_closure` (P-RK-PLUGIN-CALLBACK-CLOSURE, existing):
    the `conf->log_cb` arm;
  - `rk_log_enqueue_no_event_callback` (P-RK-LOG-ENQUEUE-NO-EVENT-CALLBACK,
    new): no queue event callback runs synchronously in `rd_kafka_q_io_event`
    during the enqueue;
  - `rk_log_enqueue_forward_not_last` (P-RK-LOG-ENQUEUE-FORWARD-NOT-LAST,
    new): no concurrent unforward-and-release makes the forward drop the last.
  Lean shows each premise is needed (`each_premise_is_needed`,
  `an_interleaved_release_makes_the_forward_drop_last`,
  `an_event_callback_can_release_a_tracked_object`).
- **The blocking review item is fixed fail-closed (rule 32a).** At the halt,
  `rd_kafka_log0` itself moved Unknown→Safe: its body's call to
  `rd_kafka_log_buf` read the sibling's caller-frame declaration, whose body
  standing is a declared assumption (`unvalidated_units 1`), so the logger's
  own row rested on the frame's own declarations while the
  `logger-effects-v1` model's 15 obligations stay Open. Now every call from a
  frame member's body to a member of the same reviewed projection (the
  recursive call included) is held `Unknown` (`caller-frame-self-reliance`,
  category `incomplete-obligation-coverage`). Both members' rows are decided
  by their bodies: `rd_kafka_log0` is Unknown on the held call, and
  `rd_kafka_log_buf` keeps its own 8 leaves. Callers outside the frame read
  the declarations unchanged.
- **All 7 other moves survive** (eight-home route, below).

## Changes

| Part | Where |
|---|---|
| Caller-frame declarations (rule 32, before the halt) | `librdkafka/log_caller_frame.qv`, lock entries pinned at 66910c10 |
| Projection check (rule 32) | `crates/qcore-contract/src/caller_frame.rs`, `tests/caller_frame.rs`, `caller-frame.json` |
| Frame registry (rule 32a) | `caller_frame::FRAMES` (held equal to the projection's `declarations` by `the_frame_registry_is_the_reviewed_projection_declarations`), `within_one_frame`, `is_frame_member` |
| Self-reliance hold (rule 32a) | `crates/qcore-c-gateway/src/legacy_caller_frame_hold.rs`; wired after the pin hold in the three token pipelines of `legacy_c.rs`; prefix reserved in `legacy_reserved_spelling.rs`; statement refusal in `legacy_classify.rs`; held-name readers in `legacy_residual_access.rs`, `legacy_generic_selection.rs`, `contract_dependencies.rs`; reason key in `qcore-audit/src/revision_compare.rs` |
| Reason mapping | `crates/qcore-schemas/src/reason.rs` ("inside its own caller frame" → `incomplete-obligation-coverage`), `tests/reason_codes.rs` |
| Regression test | `crates/qcore-c-gateway/tests/caller_frame_hold.rs` (7 tests over the production declarations) |
| Lean | `QCore.RkLogCallerFrame` §Rule 32a: `a_frame_hold_is_unknown`, `a_member_never_reads_a_member_declaration`, `a_held_member_row_is_never_safe`, `an_outside_caller_reads_the_declaration`, `the_hold_never_certifies`, `the_logger_rows_are_decided_by_their_bodies`; registered in `AuditRkLogCallerFrame.lean` (standard axioms only) |

Disabling the hold once (`hold_call_sites` returning `None`) turned three of
the regression tests' rows `Safe` (`rd_kafka_log0` with a blocked sibling, with
a trivial sibling, and recursive): *measured once, harness not shipped*
(`private/rkcf2/mut-1.log`). The hold is coarser than "decided by the body":
even a sibling whose own body would validate is not read; that only refuses.

## Measurement

Route: the conditional eight-home pinned corpus (batch4 link map
`w48-sched-cleanup-production/link-map.corpus.json`), the scheduler card's
driver as adapted by W48-BU-SINGLE-LEAF-BODIES (`scripts/measure_mirror.py`),
over the re-spelled input mirror (`scripts/mirror_inputs.py`, manifest sha256
`4edb2e25…`): after the checkout moved to `/media/Data/GitHub/quirin-lang`,
the pinned compile databases spell the old, now symlinked, path and the audit
selects units by path string, so the mirror re-spells only that prefix in the
two corpus databases (every original and rewritten digest in its manifest;
source trees byte-identical by tree digest). Job `scripts/pair8.sh`, both
sides built from `git archive` snapshots with Libevent at `d82464a2`, under the
shared lock, `--jobs 4`.

| Side | Revision | Binary sha256 | Safe | Unknown | Unsafe |
|---|---|---|---|---|---|
| BASE | `4dd8c8013` | `90d222cc…` | 977 | 3848 | 9 |
| TIP | `f53a127e1` | `3dae1137…` | 998 | 3827 | 9 |
| TIP35OFF | `f53a127e1` with rule 35's `DECAY_ARM`/`LITERAL_ARM` false | `6e9b6216…` | 986 | 3839 | 9 |

BASE equals the scheduler and single-leaf cards' BASE on this route (977).
TIP35OFF uses the switch W48-BU-SINGLE-LEAF-BODIES used for its own TIPOFF
(`scripts/off35.sh`; `off.diff` is exactly the two constants), so
BASE→TIP35OFF holds rules 32 and 34 only and TIP35OFF→TIP rule 35 only. The
merged tip `b4be13d47` (main `3353e1c63`) differs from `f53a127e1` only under
`docs/`, so these figures are the merged tip's.
Rows: 4,834 on both sides, identical keys; no Safe regression; no new Unsafe;
no Pending.

### Status changes (21), by rule

**Rule 32, 7 rows Unknown→Safe**, each conditionally Safe on the three named
premises, read from its row's hypothesis dependencies on `rd_kafka_log0`
(`scripts/premises.py`): `rk_plugin_callback_closure`,
`rk_log_enqueue_no_event_callback`, `rk_log_enqueue_forward_not_last`.

| Row | File | Remark |
|---|---|---|
| `rd_kafka_mock_cgrp_classic_member_active` | rdkafka_mock_cgrp.c | planner-predicted; BASE leaf: `rd_kafka_log0` uncontracted |
| `rd_kafka_mock_cgrp_consumer_member_active` | rdkafka_mock_cgrp.c | planner-predicted; same |
| `rd_kafka_mock_sharegroup_member_active` | rdkafka_mock_sharegrp.c | planner-predicted; same |
| `rd_kafka_mock_cgrp_classic_set_state` | rdkafka_mock_cgrp.c | BASE leaf: one indirect access (an actual of the then-uncontracted log call) |
| `rd_kafka_topic_set_state` | rdkafka_topic.c | same |
| `rd_kafka_share_log_fetch_stall` | rdkafka.c | BASE leaves: one indirect access and one unresolved identity `rkcg` |
| `rd_kafka_share_ack_commit_cb_register_op` | rdkafka.c | BASE leaf: one indirect access; also rests on its own existing declaration's `rk_plugin_callback_closure` |

The planner predicted 3 freed callers; all 3 moved, and 4 more callers whose
only blocker sat in the log call itself moved too. Of the 35 functions whose
BASE row names `rd_kafka_log0` as an uncontracted callee, 3 move and 32 keep
other leaves (457→423 leaves in total; their remaining families are in
`attribution/callers-8-3.json`: unresolved identities, indirect accesses, uncontracted
inner calls in the variadic tail such as `rd_kafka_err2str(...)`).

`rd_kafka_log0` and `rd_kafka_log_buf` stay **Unknown**. Of the 8 rule-32
moves the sibling cards measured with the unfixed rule 32 (their BASE→TIPOFF,
977→987 with rules 32 and 34), **7 remain**; the eighth, `rd_kafka_log0`
itself, is the reviewed defect and no longer moves (977→986 here). Each of
the 7 rests on exactly the three named premises; none rests on a frame
member's own declaration.

**Sibling effects, 14 rows**, reported only: rule 34 (W48-SCHED-CLEANUP-PRODUCTION)
`sched_thread_cleanup`, `mk_sched_threads_destroy_conn` (on the scheduler
premises); rule 35 (W48-BU-SINGLE-LEAF-BODIES) `rd_kafka_log_print`,
`rd_kafka_log_syslog`, `rd_kafka_set_logger`, `rd_kafka_anyconf_set_modified`,
`rd_kafka_mock_cluster_io_clear_events`, `rd_kafka_mock_cluster_io_set_event`,
`rd_kafka_mock_cluster_io_set_events`, `rd_kafka_mock_connection_set_blocking`,
`rd_kafka_oidc_assertion_get_algo_label`, `unit_test_telemetry_decode_error`,
`rd_kafka_txn_state_transition_is_valid`, `mk_config_signature`: none consults
either logger declaration, and the twelve are exactly rule 35's own TIPOFF→TIP
moves.

### Leaf trades

Rows whose leaves changed: rule 32, 93 (7 status, 86 leaf-only) in
BASE→TIP35OFF, being the frame members and rows consulting either logger
declaration; rule 34, 3 (2 status, 1 leaf-only), the rest of that segment;
rule 35, all 115 of TIP35OFF→TIP (12 status, 103 leaf-only). Rule 32's
trades (`attribution/split-base-off35.json`): removed 66 uncontracted calls, 64 indirect
accesses, 62 unresolved identities, 4 branch-condition and 1 for-loop
refusals; added 18 unresolved identities, 11 uncontracted calls (inner calls
of the log arguments the uncontracted log call had covered, e.g.
`rd_kafka_err2str`), 11 indirect accesses and the 1 hold.

Logger call leaves: BASE 47 `rd_kafka_log0` and 1 `rd_kafka_log_buf`
uncontracted → TIP 0 uncontracted, 1 held (`rd_kafka_log0`'s call to
`rd_kafka_log_buf`, `caller-frame-self-reliance`).

`W47-ALIAS-CALLEE-BORROWED-CLOSURE` (`unknown_alias` leaves): 11,047 → 11,002
over the corpus: rule 32 −44 (433→389 within its rows), rule 34 +1, rule 35 −2. `CG06-VALUE-CONNECTIVE-RESIDUALS`: the shared
driver emits no typed-path parity, so this route has no consultation-level
figure; no value-connective leaf changes. The retained pre-halt pair route
(below), which did emit parity, admits 15 value-connective and 21
variadic-tail statements at `rd_kafka_log0` calls and removes 207 attributed
uncontracted accesses; its leaf counts are not comparable with this route's.

System and tail-pause slice: no librdkafka unit is selected
(`contract/profiles/fluent-bit/{system,tail-pause}-selection.json` name none),
and the hold acts only inside the two members' bodies, so neither route can
move; neither was run.

The committed attribution is under [`attribution/`](attribution/):
`native-comparison.json` (sides, invocations, figures, row attribution),
the per-pair `compare-*.json`, `split-*.json`, `premises-8-3.json`,
`callers-8-3.json` and the mirror manifest.

## Attempts and retained evidence

All under `private/` in the card worktree; failed attempts are not evidence.

| Run | Outcome |
|---|---|
| `private/rkcf/base-2`, `tip-4` (pre-halt own pair route, `49a946c8a`/`04a917472`, no link map) | retained: 970→978 Safe, 8 moves including `rd_kafka_log0` itself (the reviewed defect) |
| `private/rkcf/failed-attempts/*` | gates 1–5 and TIP 1–3, described in its README (disk exhaustion, missing submodule link, stale pins, interrupted) |
| `private/rkcf2/pair8-1` | failed: both routes exit 1, "compile database contains no selected cmetrics sources" (inputs rooted at the real path; the databases spell the old one) |
| `private/rkcf2/pair8-2` | cancelled while waiting for the lock (it would have read the inputs through the symlink); no output |
| `private/rkcf2/pair8-3` | **evidence**: the table above; lock 12:13:37Z, both routes exit 0, done 14:10:00Z |
| `private/rkcf2/off35-1` | **evidence**: TIP35OFF, build and route exit 0; lock 16:32:28Z, done 18:00:46Z |

Two pre-halt receipts the halt record left unexplained:

- `private/rkcf/pyunit-pre.exit` = 1 (2026-10-22 21:13): the Python suite run
  on the uncommitted tree after `AuditRkLogCallerFrame.lean` was added; the one
  failure is `test_the_derived_facts_are_recomputed_rather_than_carried`, the
  committed matrix still recording 365 Lean audits against 366 files. The
  matrix count was moved 365→366 in the same working tree and
  `pyunit-pre2.log` (21:14) passed all 374; both landed in `e76b048f6`.
- `private/rkcf/sync-sync.exit` = 1 (20:59:58): `providers/contracts.py sync`
  in the TIP copy refused a stale librdkafka lock, because the two new pinned
  declarations were not yet in it; sync does not pin. `pin.sh` then pinned
  them at 66910c10 (`pin.exit` 0, 21:00:13) and `sync-check.exit` was 0
  (21:00:52).

## Gate

GATE_SECTION

## Boundaries

- No obligation of `logger-effects-v1` is discharged; the 15 stay Open, and
  an `internal` classification only says an obligation is not the caller's.
- The three premises are stated, not proved; the two new ones are registered
  as follow-ups with DISCHARGE conditions (`integration-amendments.json`).
- A held call's row still lists the consulted declarations' hypothesis atoms
  in `contract_dependencies` (the dependency reader maps the held spelling to
  its declaration); that over-reports the dependency of an Unknown row and
  supports nothing.
- `FRAMES` is a code registry of one reviewed projection; a second caller-frame
  projection must add its own slice and test.
