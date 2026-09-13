# Halted-worktree restart and agent context plan

Assessment date: 2026-09-13. Main checkpoint: `622f2a5d`.
This is an execution plan, not a completed wave, a green gate, or a new
conformance claim. Three independent assessments used **gpt-5.6-sol, medium**,
each with a fresh bounded brief and no inherited conversation history.

Follow-up sweep: six obsolete checkouts have now been removed, preserving
their branch references and archiving measurement/recovery material locally.
Main and the four pending implementation worktrees remain. The inventory
below records the **pre-sweep** assessment; [sweep.json](sweep.json) records
the removal decisions and recovery archive hashes. Recovery archives live in
ignored `private/worktree-sweep-2026-09-13/` and are not included in the push.

Post-sweep validation on the primary checkout is in
[validation.json](validation.json): 4,473 workspace tests passed (8 ignored),
122 Lean audits passed, fmt/clippy and both probes exit 0. Python ran 200
tests with the one pre-existing matrix tripwire still failing (121 audits
pinned versus 122 present). This validates the documentation/cleanup operation,
not the four pending implementation tips or an aggregate wave close.

Documentation follow-up: the retrieval strategy is now part of the main
guides, and `DEVELOPMENT.md` is the table of contents for the split
`docs/development/` tree. Preservation and size evidence is in
`docs/reports/development-docs-structure/`. This implements development
navigation; C1's automated packets and C3's always-loaded constraint routing
remain planned. The baseline JSON files below remain measurement-time
snapshots and are not rewritten to describe the new documents.

## 1. Recovery inventory

The reproducible Git snapshot is [worktrees.json](worktrees.json). Recheck
HEAD, index and working tree before acting; the snapshot is not a lock.
Names below are suffixes under `.claude/worktrees/`. Bounded Sol/medium
dispatch templates are in [dispatch.json](dispatch.json); route references
there supplement, rather than replace, applicable authoritative guidance.

| Worktree | HEAD | State and next action |
|---|---|---|
| `vibrant-curie-9f9380` | `b8a9311f` | Clean integration of CT-WIRE-LANE-3 over exact current main; 0 behind / 2 ahead. First validation candidate. |
| `agent-aa7f0452464002289` | `1382a3c5` | Clean CT-WIRE source commit, already represented by the preceding merge. Do not apply twice. |
| `agent-a56d2b9c03bea3365` | `d616512b` | W31-CORPUS-CONTRACTS-2 partial: five tracked modifications and 28 untracked files. Preserve and finish in place. |
| `agent-a86de3e1a59ff9d7f` | `56672678` | CT03-PROOF-ALPHABET implementation committed; full-suite failure unresolved and report close-out incomplete. |
| `agent-a0f14b595495fba8a` | `0e6043ce` | WRITE-LEND-TASKS implementation committed; full-suite report placeholder, supporting gate logs not found. Remeasure. |
| `agent-ae5dc083212e99d45` | `7e3265d7` | CT42 already merged into main. |
| `agent-a50095313616f5e4a` | `2670bfaa` | Already landed ancestor; no implementation to resume. |
| `agent-a7ff6154ee76e76bf` | `5f6f94ab` | Already landed ancestor; no implementation to resume. |
| `agent-a8c5f623cda3030bb` | `48ba8286` | Already landed ancestor; no implementation to resume. |
| `agent-aec1c5f7a9965434a` | `4cdbabfe` | Already landed ancestor; no implementation to resume. |

The four unique card commits are not patch-equivalent to main. Six worktree
locks name historical PIDs absent during inspection; this does not prove
ownership has been released. The read-only assessment removed no locks or
worktrees; the later user-authorized sweep is recorded separately above. The
`.sweep-backups` directory is an archive, not an eleventh worktree.

A recovery copy of the corpus tree is in the primary checkout's ignored
`private/resume-2026-09-13/corpus-recovery/`: HEAD-to-working-tree binary patch,
separate staged patch, all 28 untracked files in a tar archive, and SHA-256
manifest. The entire ignored `measure/` tree is also saved separately as
`ignored-measure.tar.gz`, preserving the final failing workspace run, passing
Lean run, and baseline/diff evidence. Originals are unchanged. Other ignored
files remain outside this copy; review them before any later destructive
operation. Prefer a reviewed WIP commit to the shared stash.

## 2. Resume order and evidence still owed

1. **R1 / CT-WIRE-LANE-3:** validate `b8a9311f` in its existing integration
   checkout. Its report's original 121-audit gate predates the merge of CT42;
   it is not an integration-tip gate. Rerun the fresh CT native failure
   outside the ptrace sandbox as described below, then run the standing gate
   and reconcile derived matrix fields via
   the generator. Preserve CT24's measured refusal. Integrate only after the
   failure-gated chain permits it. Expected mapping delta: 42 to 43 of 50,
   proof remains 2, sanitized-native remains 7; these are separate measures.
2. **R2 / W31-CORPUS-CONTRACTS-2:** recheck live ownership/session evidence and
   assign one worker before mutation; the historical PID's absence alone does
   not establish that a locked tree is unowned. Read-only verification can
   proceed meanwhile. Confirm the recovery copies, including ignored logs,
   then finish the dirty tree before rebasing it.
   Verify the artifact manifest and every attributed leaf against the pinned
   wave-30 BEFORE. Its provisional report claims unsupported 49,760 to 49,755,
   zero unit/function status moves, leak 266 and unknown_alias 4,344 unchanged;
   these are existing claims to verify, not newly accepted measurements.
   Rerun `enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
   in isolation after its recorded Broken pipe failure. Complete all six
   `GATE_*` report placeholders and missing gate evidence, including fmt.
   Commit the complete measured change, then integrate without overwriting
   R1's CT numbers. Keep all earlier corpus archives frozen.
3. **R3 / CT03-PROOF-ALPHABET:** rerun
   `confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
   in isolation and complete the full-suite report row. Review the proposed
   CT03 proof-row change against the report's exact host-unconditional test
   and sealed digest relation; a capability implementation is not an executed
   stage. Perform the integrator-owned stage/pin updates only after those
   checks. If admitted after R1, the proposed proof numerator is 3 of 43.
4. **R4 / WRITE-LEND-TASKS:** recover or rerun the entire missing gate; do not
   infer success from the committed implementation. Reconcile
   `crates/qcore-core/src/task_trace_relation.rs` with R3: preserve CT03's
   `BufferWritten` admission while lease trace events remain refused until
   modeled. Preserve both `proof/QCore.lean` imports. Apply the report's
   integrator amendments to constraint 15h and diagnostic conditions explicitly.
5. **R5 / integrated close:** one integrator updates shared status, matrix
   and pins from fresh measurements. Each of CT42, R3 and R4 adds an audit
   relative to wave 30; neither branch's local "121 to 122" may overwrite
   another addition. Count files and execute the audits; do not publish a
   computed forecast as a passing gate. No old report is edited to describe a
   later integration run.

All cards overlap `DEVELOPMENT.md`; the proof cards also overlap
`proof/QCore.lean`. Corpus close-out touches `AGENTS.md` and contract pins.
These are integrator-owned reconciliation points. The semantic R3/R4 overlap
requires review even if Git reports no textual conflict.

After the development-document split, each pending card still carries the old
monolithic `DEVELOPMENT.md`. Compare that file against the card's recorded base
and transplant its new paragraphs into the current record file named by
`docs/development/increments/README.md`. Preserve the compact root index during
merge reconciliation; do not restore the old monolith from a source branch.

### First resumed validation

Fresh runs at `b8a9311f` are in that checkout's
`measure/resume-2026-09-13/`; no tracked implementation files were changed.

| Check | Fresh result |
|---|---|
| `cargo test -p qcore-compiler --test ct_corpus` | Exit 101: 4 passed, 1 failed; quiet sanitized heap executable returned 1 instead of 0. Exact-test retry also failed. |
| Same mapped-case test on main `622f2a5d` | Same assertion failed. Establishes baseline reproduction, not the cause. |
| `cargo test -p qcore-compiler --test region_joins_loan` | Exit 0: 11 passed. |
| `python3 scripts/ct_stage_summary.py . --check` | Exit 0: 43 mapped / 7 unmapped, proof 2, sanitized-native 7. |
| `python3 -m unittest scripts.tests.test_ct_stage_summary -v` | Exit 0: 54 passed. |
| `python3 -m unittest scripts.tests.test_build_matrix_sections -v` | Exit 1: 14 passed / 1 failed; matrix pins 121 Lean audits, current tree has 122. |

The matrix tripwire is expected reconciliation work, but it is still a failed
test. Further diagnosis retained the first native heap executable in the
checkout's private measurement directory. Its direct run with the harness
sanitizer setting reported `LeakSanitizer has encountered a fatal error` and
`LeakSanitizer does not work under ptrace (strace, gdb, etc)`, exit 1. This
attributes the observed native failure to the sanitizer's runtime environment;
the early case order and `SHARE_ALLOC`/`SHARE_FREE` trace point to CT03, not
the newly mapped CT25. Diagnostic interposition also produced separate cleanup
EPERM failures; those are not the original uninstrumented assertion failure.

An outside-sandbox retry was requested but interrupted before returning a
result; no approval is pending and no passing result is claimed. The next
required command, in the integration checkout under a supported environment,
is:

```sh
cargo test -p qcore-compiler --test ct_corpus \
  ct_manifest_and_fixture_set_agree_and_mapped_cases_execute \
  -- --exact --nocapture
```

Keep sanitizers and assertions enabled. Do not merge or publish an all-green
gate on the evidence available here.

## 3. Context cost: measured baseline and limits

[context-baseline.json](context-baseline.json) records source hashes, exact
byte/word/line counts and a clearly labelled bytes/4 estimate.

| Source | Bytes | Why it costs context |
|---|---:|---|
| `AGENTS.md` | 98,715 | Always-loaded guidance; 61,698 bytes of subsystem constraints. |
| `CLAUDE.md` | 8,063 | Includes `@AGENTS.md`; runtime duplication is not measured. |
| `DEVELOPMENT.md` | 362,048 | Long history; usually only one card paragraph is relevant. |
| Acceptance matrix | 449,331 | Structured record; select a key or follow-up rather than dumping it. |

The two guidance files total **106,778 bytes**, approximately **26,695 tokens
by bytes/4 only**. Three workers would receive 320,334 bytes if each received
both once. Actual prompt injection, tokenizer counts, caching, deduplication
and billed usage are unknown. These figures must not be advertised as money
saved. Small packets alone do not reduce the automatically loaded root file.

The largest constraint block is concurrency (37,555 bytes); moving that entire
block into every task packet would preserve most of the problem. The entry
point section is also large because it embeds the diagnostic namespace table.
`docs/HANDOFF_2026-09-07.md` is historical and must not serve as current status.

## 4. Bounded context implementation cards

| Card | Deliverable and owned paths | Acceptance |
|---|---|---|
| C1 / CONTEXT-PACKET | `scripts/agent_context.py`, `docs/agent_context_routes.json`, focused tests. Generate task packets into worktree-private `private/` or `measure/`. | Declared base, fresh HEAD and working-tree identity; exact source hashes; global rules; route union of committed, staged, unstaged, renamed/deleted, untracked and planned paths plus explicit topic tags. Unknown paths/topics exit nonzero with `needs-review`; no silent truncation. |
| C2 / CONTEXT-MEASURE | Repeatable size report and representative packet fixtures in the new card's evidence directory. | Separately measure guidance source bytes assumed injected, cumulative retrieval bytes, unique authoritative bytes and identical task/code inputs. Failed/repeated searches count in cumulative transfer. No runtime usage claim without telemetry. |
| C3 / CONTEXT-ROUTE | Root `AGENTS.md`, `CLAUDE.md`, durable subsystem guidance and routing tests; integrator-only edit. Depends on C1/C2. | Root+CLAUDE target <=16 KiB; all existing hard invariants, 44 standing constraint IDs, clauses and evidence pointers preserved; every planned edit resolves applicable guidance before mutation. |
| C4 / HANDOFF-FRESH | Packet refresh/check commands and bounded worker return format. Depends on C1; pilot before C3. | Reject stale base/HEAD/source hashes and edits outside the pre-edit planned set until regenerated; include both sides of cross-subsystem edits; deterministic output for identical input state; packets remain advisory. |

C1 defaults to references and short task facts. Explicit excerpt mode supplies
the exact authoritative blocks needed for implementation. A reference is not
proof the agent read the rule: the execution workflow requires reading those
blocks before editing. Empty/unknown route selection must stop packet-based
editing and fall back to reading applicable authoritative guidance.
Clean worktrees can contain substantial committed work: diff the declared
base against HEAD as well as inspecting the index and working tree. Shared
files such as `proof/QCore.lean`, `DEVELOPMENT.md` and the acceptance matrix
require path **and topic** routing; absent a topic, require a conservative
superset or full guidance. Never infer relevance from file extension alone.

Routing tests must verify **coverage and correct applicability**, not merely
that each of 44 IDs occurs somewhere in JSON. Required scenarios:

- Gateway/corpus: `legacy_c.rs`, `legacy_summary.rs`, contract modules and pins.
- CT mapping: `ct_corpus.rs`, CT fixtures, stage summary and acceptance records.
- Proof alphabet: `task_trace_relation.rs`, CT03 proof modules and Audit imports.
- Task write-lend: ownership, task ledger, parser/frontend lowering and emission.
- Cross-cutting controls: diagnostics/spec edits, global status files, a new
  unmapped path, and a file spanning more than one subsystem.

Commit an inventory oracle with exactly IDs 1..28 plus 5a, 10a, 12a and
15a..15m: stable anchor, authoritative path, exact block hash, evidence-pointer
set, global/routed class and route tags. Check exact set equality, one current
authoritative home per ID, no duplicates/orphan routes, and the expected
constraint set for every frozen benchmark scenario. Reconcile changed hashes
through explicit review rather than silently refreshing the oracle.

C3 keeps universal safety, archive, verdict and measurement rules in the root,
including the full standing gate, the code-allocation freeze, and mandatory
cross-subsystem lookup. Split concurrency by actual responsibilities rather
than one enormous file. Before shortening any rule, compare its full clauses
and pointers, not just its number. The existing compaction rule requires any
unique facts moved out of root guidance to survive verbatim in DEVELOPMENT;
coordinate that preservation with routing to durable normative instructions.
The verbatim DEVELOPMENT copy is historical preservation; each current rule
has exactly one normative home. Amend the authority model in the root at the
same time, so neither copies nor generated packets become competing rules.
Update CLAUDE's source-of-truth wording and the documentation-layout description
in `docs/GENERALIZATION_PLAN.md` together. Do not edit old compaction reports.

C2 freezes four task manifests before comparison: same task prompt, base,
code/test/report inputs and expected constraint set. Its success target after
C3 is **at least 70% fewer cumulative guidance payload bytes** for each
scenario versus the current workflow, with equal coverage and no extra
retrieval failures. Report total transferred bytes separately, including
unchanged source/test/log inputs; do not promise a 70% reduction in that total.
Startup source bytes are an explicit injection assumption, not runtime
telemetry. The 16 KiB startup
target alone would be an 84.7% byte reduction; it is a target, not an achieved
result. Run paired tasks with the same model/settings/task input. Record cold
and warm runs separately if cache information is available. Do not count
omitted mandatory material or root-to-worker forwarding as savings.
Pin a proposed universal-guidance byte fixture before accepting the 16 KiB
target: six invariants, full gate, status/verdict and archive rules, code
freeze, measurement/worktree hygiene, authority and mandatory routing all
remain in startup. Paired agent runs verify task success, not token billing.

## 5. Parallel schedule and ownership

Use one integrator plus at most three **Sol/medium** workers. Spawn each with
no history fork, an explicit worktree, task/card, expected HEAD, owned paths,
required constraint references, relevant report sections, exact checks and a
bounded return format. User-provided rules still apply even if a brief is small.

| Stage | Worker A | Worker B | Worker C | Integrator |
|---|---|---|---|---|
| Recovery | R1 native failure diagnosis | R2 dirty artifact/checkpoint review | C1 design + completeness fixtures | Preserve recovery copy; review findings. |
| Evidence | R1 gate when admitted | R2 targeted fixes | R3 evidence recovery | Serialize expensive gates; review shared files. |
| Context pilot | C1 isolated implementation | C2 benchmark fixtures | R4 semantic overlap review | Reconcile C1/C2; keep old rules authoritative. |
| Integration | Bounded independent reviews | C4 packet freshness tests | Targeted regressions | R1 -> R2 -> R3 -> R4, then C3 after C1/C2 acceptance. |

Read-only assessment may run concurrently. Each implementation worker owns an
isolated worktree. Never share an output directory or a mutable Cargo target
between concurrent writers. Store raw logs under each checkout's `measure/`.
Only one full workspace/provider/Lean gate runs at a time; constraint-16 rows
get isolated verification without weakening assertions. Worker handoffs contain
HEAD, dirty state, changed paths, exact commands/exits/counts, log paths,
unresolved failures and the next concrete action. Suggested handoff ceiling:
1,500 words; exceed it explicitly when correctness requires more context.

Only the integrator changes shared matrix/status/gate totals. The acceptance
generator is `docs/reports/remaining-d02/build_matrix.py`: preserve its
`section_ownership`, generate a scratch candidate, review the diff, and supply
complete measured `--gate` input when publishing new gate values. Do not turn
the current stale matrix into a manual count patch.

## 6. Completion gates and immediate next action

Before any implementation card is claimed complete, run the unchanged root
standing gate: initialized pinned submodule; workspace tests with
`--no-fail-fast`; fmt; clippy; both reevaluation examples; audited Lean. Also
run relevant Python/CT checks and provider package build/verify when required.
Semantic changes need matching audited Lean; context tooling is not a new
semantic-language card and must state that boundary. Full-gate obligations
still apply to it. The present planning artifacts do not close any of R1-R5
or C1-C4.

Immediate next action is to rerun R1's exact mapped-case test outside the
ptrace sandbox, retaining the sanitizer checks. Continue R2 artifact
verification and C1 design
independently while that result is unresolved. No blanket merge, stash,
cleanup, pin weakening, or history deletion is part of this restart.
