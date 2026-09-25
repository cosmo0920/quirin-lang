# Agent-cycle guidance checkpoint (W44-AGENT-GUIDANCE-OPUS55)

> Historical checkpoint, not current instructions. Current rules start at
> [AGENTS.md](../../../../AGENTS.md) and the [rule router](../../../agents/README.md).

Every paragraph of the base guidance that card W44-AGENT-GUIDANCE-OPUS55 removed
or reworded, verbatim, at base `94502987472c25567d87fc873615ea5de1667603`. Each block names its source file and
base line range. The card's [report](../../../reports/w44-agent-guidance-opus55/README.md)
maps each block to its current location; `checkpoint.py --check` there verifies
the blocks against the base byte for byte. Relative links are as in the source
file and may not resolve from here.

### `AGENTS.md` lines 3–5

<!-- preserved:agent-cycle-01:start -->
This file is the startup router and authority for which rules to read. Detailed
rules live in the linked topic files; their existing numbers and force are
unchanged. This structure deliberately replaces the former monolithic briefing.
<!-- preserved:agent-cycle-01:end -->

### `AGENTS.md` lines 9–22

<!-- preserved:agent-cycle-02:start -->
1. Read [operations](docs/agents/operations.md)'s Working rules, Worktree and
   measurement hygiene, and Task reading protocol. Read its integration and
   maintenance sections when that work is in scope.
2. Generate the reading union with `python3 scripts/task_context.py --base
   <revision> --topic <topic> --path <planned-path>`; `--render` emits exact
   current text. It includes committed, staged, unstaged, renamed, deleted and
   untracked paths. Rerun as scope changes; add topics for semantic implications.
3. Read the returned sections. The [rule router](docs/agents/README.md) remains
   the manual fallback and stable rule index. Unknown relevance requires broader
   reading, never omission. A generated list is not evidence that it was read.
4. Read [current status](docs/STATUS.md) when selecting work or making coverage
   claims. Mapping, model proof and native observations are separate evidence.
5. Run the [standing gate](docs/agents/validation.md) and report commands,
   exits, counts and private log paths. No optimistic fallback or hidden failure.
<!-- preserved:agent-cycle-02:end -->

### `AGENTS.md` lines 33–47

<!-- preserved:agent-cycle-03:start -->
[Documentation hub](docs/README.md) is the common entry point for users and
contributors. [DEVELOPMENT.md](DEVELOPMENT.md) retains legacy heading links.
[Architecture](docs/development/architecture.md) maps crates and entry points.
[Fluent Bit queue](docs/plans/fluent-bit-system-coverage/README.md) holds
prospective production cards. Open work is the acceptance matrix's
`named_followups` plus each report's follow-ons; a rule mention is not tracking.

One current authority per rule. Edit the relevant topic page, not a duplicate
in a root guide. Add exactly one increment paragraph, newest first, to the
current record named by [the increment index](docs/development/increments/README.md).
Update [status](docs/STATUS.md) from primary evidence when figures change; do
not turn it into history. Keep new history pages below 32 KiB where a complete
record boundary permits. Preserve frozen evidence. Add a topic to the router
when necessary; do not grow root chronicles. Compaction cadence and preservation
requirements remain in [operations](docs/agents/operations.md).
<!-- preserved:agent-cycle-03:end -->

### `CLAUDE.md` lines 5–9

<!-- preserved:agent-cycle-04:start -->
Follow the startup router in `AGENTS.md`. Read
[operations](docs/agents/operations.md), the applicable
[numbered rules](docs/agents/README.md), and the
[validation gate](docs/agents/validation.md). These are the canonical policies;
this file deliberately carries no duplicate ledger or working rules.
<!-- preserved:agent-cycle-04:end -->

### `docs/agents/operations.md` lines 3–7

<!-- preserved:agent-cycle-05:start -->
Current authority, routed by [AGENTS.md](../../AGENTS.md). These are the operative
requirements. The [pre-pilot text](../reports/task-context-contract-bundle/operations-before.txt)
preserves the complete incident history, measurements and evidence pointers as a
historical checkpoint; it is not a second current authority. Read that rationale
when investigating or changing a requirement, rather than on every startup.
<!-- preserved:agent-cycle-05:end -->

### `docs/agents/operations.md` lines 11–29

<!-- preserved:agent-cycle-06:start -->
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
<!-- preserved:agent-cycle-06:end -->

### `docs/agents/operations.md` lines 33–56

<!-- preserved:agent-cycle-07:start -->
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
<!-- preserved:agent-cycle-07:end -->

### `docs/agents/operations.md` lines 93–110

<!-- preserved:agent-cycle-08:start -->
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
<!-- preserved:agent-cycle-08:end -->

### `docs/agents/operations.md` lines 114–122

<!-- preserved:agent-cycle-09:start -->
Use `python3 scripts/task_context.py --base <revision> --topic <topic>
--path <planned-path>` to compute the union for committed, staged, unstaged,
renamed, deleted, untracked and planned paths. Rerun when scope changes. Read the
returned operative sections and applicable topic text; the list does not prove
that they were read. Add topics for semantic implications that filenames cannot
reveal. Unknown paths or empty scope select every topic. The tool is local,
read-only, and does not execute instructions from changed files or reports.
Status is required for coverage claims and work selection, not every edit.
History is required when changing a rule or investigating its rationale.
<!-- preserved:agent-cycle-09:end -->

### `docs/agents/validation.md` lines 3–5; `docs/agents/buffers-records.md` lines 3–5; `docs/agents/concurrency.md` lines 3–5; `docs/agents/diagnostics.md` lines 3–5; `docs/agents/effects.md` lines 3–5; `docs/agents/evidence.md` lines 3–5; `docs/agents/gateway.md` lines 3–5; `docs/agents/io.md` lines 3–5; `docs/agents/providers.md` lines 3–5; `docs/agents/source-identity.md` lines 3–5; `docs/agents/source-routes.md` lines 3–5

<!-- preserved:agent-cycle-10:start -->
Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.
<!-- preserved:agent-cycle-10:end -->

### `docs/agents/validation.md` lines 23–28

<!-- preserved:agent-cycle-11:start -->
**The Python suite is listed as of the wave-37 close** (`DOCS-COMPACT-7` §7.3
item 3): every close since wave 32 had run `scripts/tests` and published its
count in the merged-gate row while this block omitted it, so the block now
names what the gate measures. The suite reads the committed matrix
(`CommittedMatrixIsSelfConsistent`), so it runs **after** any matrix
regeneration, not before.
<!-- preserved:agent-cycle-11:end -->

### `docs/agents/validation.md` lines 30–37

<!-- preserved:agent-cycle-12:start -->
**`--no-fail-fast` is part of the gate as of the wave-26 close (`GATE-NO-FAIL-FAST`,
a deliberate strengthening):** cargo's default stops at the first failing binary,
so one of constraint 16's load-sensitive rows silently hides every target
after it in crate order — a wave-26 card's first run ended at
`qcore-verifier-host` and never reached the `qcorec` crate at all,
`examples_cli.rs` included, making the number that run printed a lower bound
rather than a measurement. A flake must not be able to shrink the suite it is
reported against.
<!-- preserved:agent-cycle-12:end -->

### `docs/agents/validation.md` lines 39–46

<!-- preserved:agent-cycle-13:start -->
A delegated card's worktree is a **fresh checkout**, so the submodule step above
is due every time and nothing forces it: wave 33 lost two runs to a missing
`third_party/libevent`, whose absence surfaces late and reads like a regression
(`WORKTREE-SUBMODULE-UNGATED`). Separately, a gate step's exit must be
**captured, not piped** — a wave-35 merge reached origin with the Python suite
red because a pipe returned `tail`'s exit status, twice in one wave
(`GATE-EXIT-SWALLOWED-BY-PIPE`). Both are registered residuals whose mechanical
discharge is open; the instruction here is the interim.
<!-- preserved:agent-cycle-13:end -->

### `docs/agents/validation.md` lines 48–63

<!-- preserved:agent-cycle-14:start -->
**The `clang` gate key is a disclosure convention, not a check** — recorded at
the wave-40 close on `STABILITY-REEVAL-4`'s measurement
(`CLANG-RECORDING-UNBOUND-TO-ITS-RUN`), so a green regeneration is never read as
evidence that Clang was present for the run being recorded.
`build_matrix.py`'s `clang_recording()` requires only that a file named
`clang-version.txt` sit in the gate file's own directory or a `gate/`
subdirectory beside it, that the claim parse as `present: <banner>` /
`absent: <banner>`, and that the banner equal the recording's **first line**;
the `clang-version.exit` arm fires only **if that file happens to exist**. There
is no re-invocation, no host identity, no timestamp and no commit binding, so a
recording copied from any other report directory passes every arm. **Run
`clang --version` in the worktree that took the measurement and put the
recording beside that gate file**; borrowing one is permitted only where the
gate file's own note says so in those words
(`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`). The honesty is the measurer's, and
both residuals stay open on the mechanical arm.
<!-- preserved:agent-cycle-14:end -->

### `docs/agents/README.md` lines 3–6

<!-- preserved:agent-cycle-15:start -->
These pages are the canonical full text of the standing constraints formerly
embedded in `AGENTS.md`; references to “AGENTS constraint N” retain their meaning.
Read all applicable rules, not just this table. The rule text is preserved
verbatim apart from Markdown link rebasing. Root guides are navigation only.
<!-- preserved:agent-cycle-15:end -->

### `docs/agents/README.md` lines 10–22

<!-- preserved:agent-cycle-16:start -->
`python3 scripts/task_context.py --base <revision> --topic <topic> --path
<planned-path>` selects operative sections and complete applicable topic pages.
Use `--render` for their canonical text. Path routes are additive; unknown paths
select every topic. Add topics for claims or semantics that filenames cannot
identify. Recompute on scope changes. The tool reports exact line ranges, hashes,
rule IDs, reasons and reading-byte estimates; it does not certify rule compliance
or measure token billing. Tests cover committed/working/untracked paths and both
sides of renames. The manual union below remains the fallback if generation fails.

Operations history is preserved in the [pilot checkpoint](../reports/task-context-contract-bundle/operations-before.txt).
Read rationale when changing a rule; routine edits need the operative text.
Status is selected for work planning and coverage claims. Broad implementation
paths intentionally keep conservative packets until narrower routing is justified.
<!-- preserved:agent-cycle-16:end -->

### `docs/agents/README.md` lines 38–42

<!-- preserved:agent-cycle-17:start -->
Shared-file changes require the union. Relevant paths include the committed
diff from the declared base and all staged, unstaged, renamed, deleted,
untracked and planned files. If relevance is uncertain, inspect the authoritative
rules. This table routes reading; it neither exempts a constraint nor proves it
was read. Backticked code paths throughout these pages are repository-relative.
<!-- preserved:agent-cycle-17:end -->
