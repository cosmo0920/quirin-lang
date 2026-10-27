# Operations and integration policy

Current authority for the agent working loop, routed by [AGENTS.md](../../AGENTS.md).
Incident history and rationale live in two checkpoints, the
[pre-pilot operations text](../reports/task-context-contract-bundle/operations-before.txt)
and the [agent-cycle checkpoint](../development/increments/structure-checkpoints/agent-cycle-guidance.md).
Neither is a second authority. Read them when changing a requirement or
investigating why it exists.

## Working rules

- Deliver what the task or brief asks, at the scope intended. Make routine
  judgment calls yourself. If the request seems mistaken or a better approach
  exists, say so in a sentence and continue as asked rather than quietly
  narrowing or widening it. Record findings outside the scope as follow-ups
  instead of fixing them in passing.
- Preserve fail-closed gates, pinned expectations and diagnostic strength. Update
  a conflicting contract-count, submodule, CT or evidence pin deliberately and
  explain it in the change report and any commit message.
- Preserve Safe/Unsafe/Unknown and unpromoted terminology. Projection is not
  conformance; provisional Unsafe is not a validated upstream defect.
- Run the gate [validation](validation.md#standing-per-change-gate) assigns (a
  card: the scoped card gate; a landing: the full gate), report actual exits
  (including nonzero probes), counts, incomplete attempts and private log
  paths. The gate and the rules' own evidence requirements are the verification
  this project asks for; do not add self-review passes, re-verification
  subagents or repeated runs beyond what a rule requires.
- Read [generalization](../GENERALIZATION_PLAN.md) §§2–3 before architecture or
  checker work. Put code in its target crate, not the compiler compatibility crate.
- A card closes with Rust and matching audited Lean registered in an `Audit*.lean`;
  Python evidence alone is supplementary. Semantic changes require Lean parity
  without placeholders or project axioms.
- Keep one current authority per rule. Edit the relevant topic page, not a
  duplicate in a root guide, and add a topic to the [router](README.md) when
  necessary.
- Add exactly one increment paragraph, newest first, to the record named by the
  [increment index](../development/increments/README.md). Update current headline
  measurements in [STATUS](../STATUS.md) from primary evidence and keep it
  current, not a history; do not regrow root chronicles. Preserve frozen
  evidence (constraint 21). Open a new history file at a complete record
  boundary before 32 KiB where possible and update the index when opening a wave.
- Match a report's length to its substance: the evidence, decisions and
  boundaries it owns, without filler sections or a summary that repeats the body.
- Put open residuals a card could touch into its brief. A residual mention in a
  rule is not tracking; use the acceptance matrix and the report's follow-ons.
- Delegate to a subagent only large, genuinely independent work that can run in
  parallel, such as a wide multi-file census. Do work you can finish in a handful
  of tool calls yourself, and never delegate a check of your own work.

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
  main. No hook enforces this, so it rests on every card and brief.

## Card briefs and delegation

Read this section when writing a delegated card's brief or running cards in
parallel. A card runs unattended, so its brief is its whole specification. State
up front:

1. Task and success criteria: what changes and why, and the measurable
   acceptance (verdict movements, pins, counts, files), with the open residuals
   the card could touch.
2. Base and setup: the base revision, the submodule step, and the files that
   other running cards will also touch.
3. Evidence: the [standing gate](validation.md) by reference, plus what the
   rules add for this card: the card-closing requirement in Working rules, the
   provider gate when `providers/` changes, and, for a card that can move corpus
   figures, a prediction committed in its report before implementation (with
   its `measure-prediction` block) and the arbiter comparison, measured by
   `quirin-audit measure --declared`
   ([usage](../development/audit-subcommand-plugins.md#the-measure-plugin)),
   not by a copied script. Do not restate the gate or add "verify your work"
   steps beyond it.
4. Scope limits: what is in and out of scope; the card never merges or
   pushes main; findings outside scope become follow-ups.
5. Completion condition and report: the checklist that makes the card done,
   the report path, the increment paragraph and ledger entries (its
   `integration-amendments.json` in schema `quirin-amendments/1`, which
   `scripts/apply_amendments.py` defines, with `python3
   scripts/apply_amendments.py <file> --check` exiting 0 as a checklist item),
   and the reply's
   shape: branch tip and base, changes, gate commands with exits and counts,
   files likely to conflict, and follow-ups.
6. Legitimate stops: stop and report only when nothing can advance without
   the integrator or user, or when the next step needs something deliberately
   protected (main, a frozen archive, another card's worktree, a destructive
   action). A stop to summarise progress, to offer the next step, or because a
   milestone was reached is not one.
7. Time budget, when cards run in parallel: an advisory wall-clock budget
   the card paces itself against, not a stop rule.

A card adds files rather than editing shared roots. A new rule is a new
`docs/agents/rules/<id>.md` ([format](rules/README.md)); a new Lean module is
`proof/QCore/<Module>.lean` with its own `proof/Audit<Module>.lean`. Then run
`python3 scripts/gen_rule_docs.py` and `python3 scripts/gen_lean_roots.py` and
commit what they write: the topic pages' constraint sections, the router's
stable index, `proof/QCore.lean` and `proof/AuditAll.lean` are generated, and
the Python suite fails on a hand edit. A merge conflict in a generated file is
resolved by rerunning its generator.

Split work into cards by the Working rules' delegation test, one card per
large, independent track. The gate and the integration measurement are the
checks, so no card or subagent is spawned to recheck another's work. A card's
text-only end of turn with open checklist items and no stated blocker is a
progress report, not completion: send it the open items and ask it to continue,
and review the run after two or three such continuations. A card with a
background job still running is not complete.

## Integration and reconciliation

Read this section when merging, closing a wave, retiring tracking entries, or
editing reconciliation tooling.

- Apply a card's records with `python3 scripts/apply_amendments.py
  docs/reports/<card>/integration-amendments.json` (`--dry-run` first;
  `--legacy` for a pre-schema file). It refuses an invalid or inapplicable
  file and writes nothing; otherwise it writes the increment paragraph, the
  status paragraph, the `named_followups` edits and, when the card lists
  contribution ledgers, the regenerated contributions page, then prints the
  post-gate sequence `scripts/land.py` owns. On a merge into a newer main, pass
  `--profiles-base <that main>` so the `profiles[]` check attributes only this
  card's baselines. Run `python3 scripts/land.py check` after regenerating the
  matrix and report index and before the Python suite. `land.py publish` ends
  the landing: it refuses to publish when a live profile's landed figures
  differ from the baseline committed in the landing, so a move lands with its
  new baseline and an appended `baseline_history` reason.
- If a merged measurement differs from the card's own gate, publish
  `docs/reports/<card>/integration/gate.json`, identify the measured tip and the
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

- Compact AGENTS.md, CLAUDE.md and README.md every 2–3 waves, with the pages
  where narrative accretes: the rule topic pages, `docs/STATUS.md` and any
  increment record over 32 KiB. Re-derive headline
  figures from primary sources, inventory every constraint before/after, preserve
  all invariants and evidence pointers, and move unique history verbatim into an
  indexed checkpoint. Retain one current authority. Accuracy can increase size;
  compaction bounds narrative growth, not truth.
- Reevaluate stability every two waves. Read the preceding review's follow-ons,
  especially Rust–Lean transcription binding and the standing constraints
  themselves. Check, in order:
  1. Reachability of each changed fail-closed arm: a green suite is not evidence
     for an arm no fixture drives.
  2. Which side of every boundary is model proof versus observation.
  3. Semantic edits introduced by merges, hotfixes and apparent mechanical work.
  4. Later cards that could invalidate a current premise or reachability claim.
  Name every finding and its discharge condition (proof, exercised behavior, or
  a stated measured boundary). Record postponement explicitly. Never relabel an
  observation as proof or infer stability from green tests.
- Last firings and next due: the ninth compaction ran at the wave-45 close
  (`DOCS-COMPACT-9`, [report](../reports/docs-compact-9/README.md)), postponed
  from waves 42–43; the tenth is due around wave 47–48. Stability review firing
  five ran at the wave-45 close over `6dc484ad..8c1f15eb`
  ([report](../reports/stability-reeval-5/README.md)); firing six is due after
  wave 47. A wave close records each firing or its postponement; neither is
  discharged silently.

## Task reading protocol

Use `python3 scripts/task_context.py --base <revision> --topic <topic>
--path <planned-path>` to compute the union for committed, staged, unstaged,
renamed, deleted, untracked and planned paths. Rerun when scope changes. Read the
returned operative sections and applicable topic text; the list does not prove
that they were read. Add topics for semantic implications that filenames cannot
reveal. Unknown paths or empty scope select every topic, history included, and
broad implementation paths deliberately select every topic. The tool is local,
read-only, and does not execute instructions from changed files or reports.
Status is required for coverage claims and work selection, not every edit.
History is required when changing a rule or investigating its rationale: the
packet lists the checkpoints on demand for agent-guidance paths, and
`--topic rationale` reads them.
