# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `83e6e497`. It
is merged onto main `1db9a569`, after all wave-45 cards through
W45-CONTRACT-DEFERRED-REVIEW. The merge is a clean two-parent merge (50 files
changed, 4,160 insertions, 21 deletions); nothing conflicted.

No function or unit moved: **0 corrections**. The card's own delta (measured
against `docs/reports/w45-sub-object-stores/integration/artifacts`, which its
own BEFORE reproduced byte for byte) composed onto the combined tree exactly.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## The core test step's flake

The core test step (`02-test`) first exited 101 on one failure:
`qcore-compiler::test::task_route_tsan::removing_the_join_edge_is_reported_under_every_schedule`
(`compiler/tests/task_route_tsan.rs:746`). ThreadSanitizer reported a thread
leak instead of the planted join-edge race while another card was building
elsewhere on the host. This row is not a registered constraint-16 (EXCLUSIVE)
table entry; registering it is a follow-up for the integrator, not done here.

It passed 5 of 5 isolated reruns
([task-route-tsan-reruns.txt](task-route-tsan-reruns.txt)), and a full rerun
of the test step on the same tree was clean: **5,781 passed / 0 failed / 11
ignored over 307 summaries**, exit 0
([task-route-tsan-reruns.txt](task-route-tsan-reruns.txt)). Both attempts are
recorded in [gate.json](gate.json); the first exit is kept as measured, not
replaced, and the core counts in the gate table below and in `gate.json` are
taken from the clean rerun.

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` (first attempt) | 101 | 1 failure: `task_route_tsan::removing_the_join_edge_is_reported_under_every_schedule` (load-sensitive TSan flake, see above) |
| core `cargo test --workspace --no-fail-fast` (rerun, same tree) | 0 | **5,781 passed / 0 failed / 11 ignored** over 307 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 195 audits |
| Python before matrix regeneration | 0 | — |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 15 Python tests |
| release corpus run (arbiter) | 0 | — |
| `baseline_identity.py` against `w45-sub-object-stores/integration/artifacts` | 1 | by design; this card moves leaves (`verdict_bearing_identical: false`) |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **COMPOSES** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 358 tests |

## Composition

**Card delta.** Against `docs/reports/w45-sub-object-stores/integration/artifacts`
(the card's own BEFORE baseline, reproduced byte for byte), the card removes 1
leaf and adds 5. It moves 0 functions and 0 units.

**Combined delta.** On the combined tree the delta is exactly the same:
removed 1, added 5. No overlap with earlier integrations, no transitive
callers cleared, no complementary Unknown → Safe clearing, no functions moved.
The composition check ([composition-check.txt](composition-check.txt)) prints
`COMPOSES`.

**The five added for-clause leaves**, all in functions already `Unknown`
(from [added-leaves.tsv](../added-leaves.tsv)):

| Function | Site | Original clause |
|---|---|---|
| `balance` | `rdkafka_sticky_assignor.c:1331` | `RD_MAP_FOREACH_ELEM(elem, &fixedAssignments.rmap) {` |
| `rd_kafka_sticky_assignor_assign_cb` | `rdkafka_sticky_assignor.c:2032` | `RD_MAP_FOREACH_ELEM(elem, &currentAssignment.rmap)` |
| `rd_map_copy` | `rdmap.c:143` | `RD_MAP_FOREACH_ELEM(elem, src) {` |
| `unittest_untyped_map` | `rdmap.c:481` | `RD_MAP_FOREACH_ELEM(elem, &rmap) {` |
| `ut_tq_find_prev_pos` | `rdunittest.c:144` | `TAILQ_FOREACH(e, head, link) {` |

The one removed leaf is the `unresolved resource identity` report on the
`TAILQ_FOREACH` increment at `rdunittest.c:144`, exchanged for that site's new
for-clause refusal (net `unknown_alias` 3,568 → 3,567, `unsupported` +5 =
42,699).

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | `w45-sub-object-stores` baseline | combined |
|---|---|---|
| functions | 390 / 12 / 4,432 | 390 / 12 / 4,432 |
| units | 4 / 10 / 167 / 4 | 4 / 10 / 167 / 4 |
| leak | 37 | 37 |
| summary_conflict | 1 | 1 |
| unknown_alias | 3,568 | **3,567** |
| unsupported | 42,694 | **42,699** |

No function or unit changed status. The combined run is archived as the new
baseline under [artifacts/](artifacts/).
