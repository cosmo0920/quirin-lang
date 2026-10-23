# W48 implementation round 1: halt record

Human halt, 2026-10-23. The user halted the three running implementation cards
and asked for their progress to be recorded. [halt.json](halt.json) holds the
exact commits, figures and evidence paths. This page states the boundaries.

## Integration branch at the halt

Local `main` is `0217006f2`. It is **unpushed**; origin stays at `49a946c8a`.
The user merged four branches into it:

- `integrate/audit-ir-retained-products` (complete card);
- each card branch below, at the tip it had when merged.

Evaluation of `0217006f2`:

- `cargo check --workspace --all-targets` exits 0.
- Python passes 373 of 374 tests. The one failure is the committed matrix's
  stale Lean count: it records 367, the tree has 369.
- No full gate was run.

The user's decision: keep this branch unpushed. When the cards finish, merge
their final commits, regenerate the matrix, run one combined full gate and push
everything together.

Before the halt, each card committed its work and fast-forwarded to
`0217006f2`. All three trees are clean. No process remained, and no card held
the shared validation lock.

## Cards

| Card | Rule | Branch tip | Prediction | Measured | Missing |
|---|---|---|---|---|---|
| W48-RK-LOG-CALLER-FRAME | 32 | `422e64b3e` | before implementation | its own pair route: 970→978 Safe | final gate (waiting on the lock), re-measure on the new base, report |
| W48-SCHED-CLEANUP-PRODUCTION | 34 | `2fe7708fa` | before implementation | eight-home corpus 977→979 Safe; provider gates pass | full gate (interrupted, then queued), re-measure on the new base, report |
| W48-BU-SINGLE-LEAF-BODIES | 35 | `1482e0267` | **after** the first semantic commit `b41a36512`, before any measurement | none: the first locked run was building binaries when halted | measurement, gate, report |

**Logger: eight rows move Unknown→Safe**, including the three callers the
planner predicted. Every logger call leaf is gone (48→0). The figures come from
the card's own pair route; at the same revision, the eight-home route used by the
other cards reads 977 Safe, not 970. The figures are not comparable across cards
until the logger is re-measured on that route.

**Concern, to review before landing.** `rd_kafka_log0` itself becomes Safe. It
rests on its own declared contract, whose body standing is a declared assumption
and is unvalidated. The brief required the logger's own verdict to stay governed
by its body, with its 15 internal effect obligations Open.

**Scheduler.** `sched_thread_cleanup` and `mk_sched_threads_destroy_conn` become
custody Safe. Both stay Unknown on the null property. Neither predicted caller
moved: `mk_sched_threads_destroy_all` and `mk_sched_threads_purge` remain
Unknown.

**Single-leaf bodies.** Rule 35's Rust and Lean exist, and rule 35 is documented.
It has run only on synthetic fixtures. The prediction's late chronology is stated
in its own file.

No verdict figure in this record has been gated or measured on the integration
branch. None is a status claim. `docs/STATUS.md` is unchanged.

## Resume

1. Resume each card from its branch tip.
2. For each card: measure BASE `4dd8c8013` against its final commit, attribute
   every move by rule, run its own full gate and write its report.
3. Merge the final commits into local `main`, regenerate the matrix, run one
   combined full gate and push.
4. Then apply the approved CFG release patch, with its own gate and report parity.
