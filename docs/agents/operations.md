# Operations and integration policy

Current authority, routed by [AGENTS.md](../../AGENTS.md). These are the operative
requirements. The [pre-pilot text](../reports/task-context-contract-bundle/operations-before.txt)
preserves the complete incident history, measurements and evidence pointers as a
historical checkpoint; it is not a second current authority. Read that rationale
when investigating or changing a requirement, rather than on every startup.

## Working rules

- Preserve fail-closed gates, pinned expectations and diagnostic strength. Update
  a conflicting contract-count, submodule, CT or evidence pin deliberately and
  explain it in the change report and any commit message.
- Preserve Safe/Unsafe/Unknown and unpromoted terminology. Projection is not
  conformance; provisional Unsafe is not a validated upstream defect.
- Run the full [standing gate](validation.md), report actual exits (including
  nonzero probes), counts, incomplete attempts and private log paths.
- Read [generalization](../GENERALIZATION_PLAN.md) §§2–3 before architecture or
  checker work. Put code in its target crate, not the compiler compatibility crate.
- A card closes with Rust and matching audited Lean registered in an `Audit*.lean`;
  Python evidence alone is supplementary. Semantic changes require Lean parity
  without placeholders or project axioms.
- Add exactly one increment paragraph, newest first, to the record named by the
  [increment index](../development/increments/README.md). Update current headline
  measurements in [STATUS](../STATUS.md) from primary evidence; do not regrow root
  chronicles. Open a new history file at a complete record boundary before 32 KiB
  where possible and update the index when opening a wave.
- Put open residuals a card could touch into its brief. A residual mention in a
  rule is not tracking; use the acceptance matrix and the report's follow-ons.

## Worktree and measurement hygiene

- Prefer temporary WIP commits to the shared `git stash` stack. Delegated cards
  use isolated worktrees, remain at least one commit ahead of their base once
  work exists, and never end with an unchanged tree while jobs are in flight:
  the isolation harness can remove it. Integrate cards one at a time under the
  failure-gated chain discipline, with a report under `docs/reports/<card>/`.
- Store measurement outputs in ignored, worktree-private `measure/<run>/` or
  `private/<run>/`, never under `target/` or a shared scratchpad. Every run gets
  fresh paths, including confirmation runs. A disk-exhausted run measures disk,
  not the program: retain the failed attempt, discard it as validation evidence,
  free only your own safe build output, and remeasure.
- Reclaim a delegated card's build directory only after its final report, not
  after an artifact appears. Ask its agent rather than inferring completion.
- Restore mutation inputs by rewriting bytes and assert the round trip. Never
  use `copy2`, `cp -p`, or an mtime-preserving restore. If moving a backup or using
  `cp`, `touch` after restoration. Run a green baseline after the last mutation.
- Publish a mutation count only with its harness and per-mutation logs in the
  report directory. Otherwise label it *measured once, harness not shipped*;
  later corrections belong beside the frozen report, not inside it.
- Kill only captured PIDs or a known task handle; never `pkill`/`pgrep` patterns
  such as cargo, lake or python. The process table is shared across worktrees.
- A delegated card never advances main; only the integrator merges or pushes
  main. This is policy, not mechanism: the versioned main-ref guard hooks were
  removed, so no hook enforces it. Historical guard evidence remains in the
  checkpoint.

## Integration and reconciliation

Read this section when merging, closing a wave, retiring tracking entries, or
editing reconciliation tooling.

- If a merged measurement differs from the card's own gate, publish
  `docs/reports/<card>/integration-gate.json`, identify the measured tip and the
  merge SHA, and state it in the merge message. Equal figures need no duplicate
  gate. A close sweeps every integration gate and remeasures its merged-gate row.
  The recorded Clang source must follow [validation policy](validation.md).
- Work every card's integrator list at wave close. Run:

  ```sh
  python3 scripts/reconciliation_sweep.py . --since <previous-close> --check docs/reports/wave<N>-close/reconciliation.json
  python3 scripts/reconciliation_sweep.py . --cards <cards> --residuals
  ```

  Record each item as applied/declined/superseded/deferred/noted; every non-applied
  verdict needs its ground, and a card without a section needs a checked
  `no_section` reason. Apply amendments in their proposed words or decline with
  a reason; never silently soften them. `applied` needs a `verify` list of
  `{file, contains}`/`{file, absent}`, or an `unverifiable` ground. The check
  validates recorded strings, not whether a decision was right.
- Prefer durable targets where the amendment actually belongs. Each close checks
  its own record against its own tree. A later legitimate supersession is not a
  duty to rewrite a frozen predecessor or make its old check pass again.
- Confirm a retired name was actually registered; removing a never-registered
  name is a no-op and must be described as such. Rehome every residual whose sole
  tracker is retired. Compare `--residuals` before and after removal; distinguish
  discharged historical names by naming their discharging cards.

## Maintenance cadence

Read this section for documentation compaction, stability review and wave close.

- Compact AGENTS.md, CLAUDE.md and README.md every 2–3 waves. Re-read headline
  figures from primary sources, inventory every constraint before/after, preserve
  all invariants and evidence pointers, and move unique history verbatim into an
  indexed checkpoint. Retain one current authority. Accuracy can increase size;
  compaction bounds narrative growth, not truth. The historical eighth firing
  was wave 40; the ninth was due around wave 42–43, not silently discharged here.
- Reevaluate stability every two waves. The wave-40 close recorded firing five
  due after wave 41; this routing pilot does not claim to have run it. Read the
  preceding review's follow-ons, especially Rust–Lean transcription binding and
  the standing constraints themselves. Check, in order:
  1. Reachability of each changed fail-closed arm: a green suite is not evidence
     for an arm no fixture drives.
  2. Which side of every boundary is model proof versus observation.
  3. Semantic edits introduced by merges, hotfixes and apparent mechanical work.
  4. Later cards that could invalidate a current premise or reachability claim.
  Name every finding and its discharge condition (proof, exercised behavior, or
  a stated measured boundary). Record postponement explicitly. Never relabel an
  observation as proof or infer stability from green tests.

## Task reading protocol

Use `python3 scripts/task_context.py --base <revision> --topic <topic>
--path <planned-path>` to compute the union for committed, staged, unstaged,
renamed, deleted, untracked and planned paths. Rerun when scope changes. Read the
returned operative sections and applicable topic text; the list does not prove
that they were read. Add topics for semantic implications that filenames cannot
reveal. Unknown paths or empty scope select every topic. The tool is local,
read-only, and does not execute instructions from changed files or reports.
Status is required for coverage claims and work selection, not every edit.
History is required when changing a rule or investigating its rationale.
