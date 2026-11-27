# W45-REFUTED-CONTRACT-WITHDRAWAL: prediction (frozen before implementation)

Written on the base `12553fb3` after the BEFORE measurements and before any
code change. It is not edited afterwards; the report compares against it.

## BEFORE, measured

* Corpus arbiter (`run_audit.py … --jobs 8`, base binary): diagnostics and
  `chunk-contracts` byte-identical to
  `docs/reports/w45-fb-backlog-custody/integration/artifacts/`;
  `verdict_bearing_identical: true`. The contracts sidecar differs by the
  worktree path and by the two `meaning` strings W45-USER-CONTRACT-PLUGINS
  rewrote (its integrator's `contracts-raw-diff.log`), nothing else.
  Functions 398 / 9 / 4,427, units 5 / 8 / 168 / 4, leak 34,
  unknown_alias 3,570, unsupported 41,325, summary_conflict 1, Pending 0.
* System slice at `046accf6` (fresh private prepare, base binary): units
  0 / 2 / 13, functions 28 / 2 / 204 / 3 not projected; unsupported 3,338,
  unknown_alias 717, counted_user 4; **zero** function rows differ from the
  published W45-FB-BACKLOG-CUSTODY slice.

## The design being predicted

A declared contract of a function whose body a unit analyses is **refuted**
when that body contradicts it: a counted-user clause of its own row is
`refuted` (C04.7), or W4's validated summary contradicts a declared custody
*duty* (constraint 8's mode arm). A nullability or owner-index refinement
(`SummaryConflict` with no duty inequality) is **not** a refutation. A refuted
contract is withdrawn from every caller: in its own unit directly, and in
every other unit through a per-run table computed by a first pass. The held
call refuses with reason key `contract-refuted-by-body` and is `Unknown`.

## Predictions

1. **Corpus: no movement.** No corpus row carries a refuted counted-user clause
   (no row has `user_count_clauses`), no duty contradiction is recorded (no
   `mode-*-contradiction` leaf), and the one `summary_conflict`
   (`rd_kafka_mem_free`, `Released{nullable}` declared, `Released{nonnull}`
   derived) is a refinement. The table is empty, no call is held, and
   `resource-diagnostics.json` and `chunk-contracts.json` are byte-identical.
   The contracts sidecar gains one additive section (the empty table and its
   digest). **No Safe → non-Safe correction** in the corpus.
2. **Slice: the table has exactly two entries**, both counted-user
   refutations in `src/flb_output.c` (both variants):
   `flb_output_task_flush` (OR-7, `resource_user_exit(task, 0, added)` at
   `src/flb_output.c:541`) and `flb_output_task_singleplex_enqueue` (OR-1,
   `resource_user_exit(task, -1, restored)` at `src/flb_output.c:376`).
   Every other declaration with an analysed body is established or
   unchecked, not refuted.
3. **Slice: held call sites** — `flb_engine_dispatch_retry` (`:179`, `:186`)
   and `tasks_start` (`:319`, `:327`) in `src/flb_engine_dispatch.c`, cross
   unit, through the table; `flb_output_task_queue_flush_one` (`:333`) in
   `src/flb_output.c`, in unit. Each in both variants: 10 held call rows.
4. **Slice: no status movement.** All three callers are already `Unknown`
   (unattributed counted-user operations, unresolved identities), so units
   stay 0 / 2 / 13 and functions 28 / 2 / 204 / 3. The two OR functions stay
   provisional `Unsafe`: their own refutation is on their own row and does not
   pass through the held calls. Leaf exchanges are expected only in the three
   caller rows (the unattributed-operation leaf at a held site becomes the
   hold leaf; a consumed `retry` actual handed on may add `unknown_alias`
   leaves after `:179`), and possibly the blocker text of OR-1's undecided
   success clause (`flb_output_task_queue_flush_one` now reaches a held call).
5. **No leak or violation leaf is added** anywhere.
6. **Ledger:** `W45-FB-SINGLEPLEX-CUSTODY-DECLARED` is discharged by the
   withdrawal (no caller applies either declaration: 0 applying call sites).
   `W45-FB-CROSS-UNIT-USER-PREMISES` is re-scoped, not discharged: the two
   premises (`flb_output_thread_pool_flush` with no analysed body,
   `flb_task_retry_destroy` undecided in `src/flb_task.c`) are `unchecked`,
   not refuted, and stay applied; they support the two refutations, never a
   `Safe`.
