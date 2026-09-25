# W44-AGENT-GUIDANCE-OPUS55: the agent loop follows current prompting guidance

Card of 2026-09-25, base `94502987` (local `main`). This is a documentation and
tooling card. No crate, compiler, provider, contract or proof source changes,
and no verdict, pin or count moves.

## Outcome

The agent-cycle guides (`AGENTS.md`, `CLAUDE.md`, `docs/agents/operations.md`,
`validation.md`, the rule router and the topic-page headers) lost their duplicated
reading instructions, emphasis stacking and embedded incident history. They
gained a scope statement, a verification boundary, a delegation policy, report
length calibration and a one-screen card-brief standard with completion
conditions, legitimate stops and time budgets. `scripts/task_context.py` now
lists rule history on demand for agent-guidance paths instead of reading 23,862
bytes of it on every such edit.

Every numbered rule and the hard-invariant block are byte-identical to the base,
59 rule anchors and 1 block, checked mechanically. Every unnumbered project
requirement in the touched files keeps its force (table below). Every removed or
reworded base paragraph is preserved verbatim in the
[agent-cycle checkpoint](../../development/increments/structure-checkpoints/agent-cycle-guidance.md),
27 source ranges checked against the base byte for byte. No frozen report is edited.

## Guidance applied

Sources, both read for this card: [Prompting Claude Opus 5.5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)
and [Prompting Claude Opus 5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5).

| Guidance (section) | Applied as |
|---|---|
| Opus 5, *Task scope and over-verification*: the model verifies its own work; explicit verification instructions cause over-verification. Constrain scope. | Working rules: the gate and the rules' evidence requirements are the verification, with no extra self-review passes or re-verification subagents; a scope bullet adapted from the guide's scope paragraph. Project evidence gates are unchanged. |
| Opus 5, *Controlling subagent spawning* | Working-rules delegation bullet; card splitting follows the same test. |
| Opus 5, *Self-correction*: avoid "double-check" instructions | The one "re-read" in the operative files was an evidence requirement (take figures from primary sources) and is reworded "re-derive". |
| Opus 5, *Written deliverable length* | Working-rules report-length bullet. |
| Opus 5, *Capability improvements*: complete task specification up front, then run | Card-brief standard, items 1–5. |
| Opus 5.5, *Unattended agentic runs*: text-only end of turn is a report; checklist; name the unwanted and the wanted stops; bounded continuations; running background work is not done | Card-brief items 5–6 and the continuation paragraph. |
| Opus 5.5, *Time signals for multiagent harnesses* | Card-brief item 7, an advisory budget. |
| Opus 5.5, *Calibrate effort* and *Thinking instructions*: effort, not prompt text, controls thinking | No "think carefully" line existed; none added. |

## Inventory

Base line numbers. Class (a) is a project invariant or evidence requirement, (b)
model scaffolding, (c) embedded history, (d) missing guidance.

| Base location | Class | Item | Now |
|---|---|---|---|
| `AGENTS.md:3–5` | (c) | "This structure deliberately replaces the former monolithic briefing" | Checkpoint block 01 |
| `AGENTS.md:12–15` | (b) | Tool mechanics restated (third copy, with router and protocol) | Link to the task reading protocol |
| `AGENTS.md:40–47` | (b) | Record-keeping rules restated from operations | Merged into operations Working rules; AGENTS links |
| `CLAUDE.md:5–9` | (b) | Reading list restating AGENTS start-here | Removed; AGENTS is imported above it |
| `operations.md:53–56` | (c) | Main-ref guard hook removal history | "No hook enforces this" plus checkpoint block 07 |
| `operations.md:93–98` | (a)+(c) | "Re-read headline figures"; eighth/ninth firing narrative | "Re-derive"; due-state bullet |
| `operations.md:99–101` | (c) | "this routing pilot does not claim to have run it" | Due-state bullet |
| `validation.md:3–5` and the same lines in 10 topic pages | (b) | "Read every applicable rule…" repeated in 11 headers | One statement in the router, beside step 3 of AGENTS |
| `validation.md:23–28` | (c) | Wave-37 listing history | Operative sentence kept; history in block 11 |
| `validation.md:30–37` | (c) | Wave-26 incident narrative | Reason clause kept; history in block 12 |
| `validation.md:39–46` | (c) | Wave-33 and wave-35 incidents | Reason clauses kept, residual ids kept; block 13 |
| `validation.md:48–63` | (b)+(c) | Five bold emphasis spans; wave-40 attribution | Plain text; block 14 |
| `validation.md:23–63` | (b) | Seven bold spans in prose in total | 0 remain outside the hard-invariant block |
| `README.md:3–6` | (b)+(c) | Duplicate reading reminder; migration verbatim claim | Router paragraph; block 15 |
| `README.md:10–22` | (b)+(c) | Tool mechanics (second copy); test-coverage claim | Link to protocol; block 16 |
| `task_context.py:28,128–131` | (b) | Mandatory 23,862-byte history read for any agent-guidance path | Listed on demand; `--topic rationale` or unknown scope reads it |
| none | (d) | Scope statement | Working rules |
| none | (d) | Verification boundary | Working rules, gate bullet |
| none | (d) | Subagent delegation policy | Working rules |
| none | (d) | Report length calibration | Working rules |
| none | (d) | Card-brief standard: task and criteria, setup, evidence by reference, scope, completion condition and reply shape, legitimate stops | [Operations](../../agents/operations.md#card-briefs-and-delegation) |
| none | (d) | Unattended continuation: text-only stop is a report, two or three continuations, background work | Same section |
| none | (d) | Time budget for parallel cards | Same section, item 7 |

Totals: 9 (b) items removed (one covers 11 header copies, one covers 7 bold
spans), 10 (c) items moved to the checkpoint, 7 (d) items added. The prediction
freeze and arbiter comparison were existing card practice (`PREDICTION.md` in
the W43 and W44 corpus cards) but were not written in any guide. The card-brief
standard now names them as brief content, which records the practice without
changing the rules.

Out of scope and not rewritten: the numbered rule bodies. They are product
semantics, and they carry 122 `wave N` mentions across 181,685 bytes of topic
pages, which dominates every implementation packet (see measurements).

## Constraint table

| Rule | Before | After | Force |
|---|---|---|---|
| Hard invariants 1–6 | `AGENTS.md` preserved block | same | unchanged, byte-identical |
| 1–3 | `diagnostics.md` | same | unchanged, byte-identical |
| 4, 4a, 5, 5a–5i, 6–10, 10a, 10b | `gateway.md` | same | unchanged, byte-identical |
| 11, 12, 12a | `source-identity.md` | same | unchanged, byte-identical |
| 13, 14 (and its amendments anchor), 15, 15a, 15b | `source-routes.md` | same | unchanged, byte-identical |
| 15c–15g, 15i, 15p | `io.md` | same | unchanged, byte-identical |
| 15h, 15k (and its amendments anchor) | `buffers-records.md` | same | unchanged, byte-identical |
| 15j, 15l, 15m | `effects.md` | same | unchanged, byte-identical |
| 15n, 15o, 27 | `concurrency.md` | same | unchanged, byte-identical |
| 16–20, 26 | `providers.md` | same | unchanged, byte-identical |
| 21–25, 28 | `evidence.md` | same | unchanged, byte-identical |
| Start here 1, 3, 4, 5 (fail-closed reading, status, gate reporting) | `AGENTS.md:9–22` | same | unchanged; step 1 also names the card-brief section |
| Union covers committed/staged/unstaged/renamed/deleted/untracked; rerun; add topics | `AGENTS.md:14–15`, `README.md:10–17`, `operations.md:114–119` | operations Task reading protocol | reworded, same force (one authority) |
| Unknown paths or empty scope select every topic | `operations.md:119`, `README.md:12–13` | protocol and `task_context.py` (history included) | unchanged in force |
| History required when changing a rule | `operations.md:122`, `README.md:19–20` | protocol | same force; tool reads it on demand (next section) |
| One authority per rule; edit topic page; add router topic | `AGENTS.md:40–41, 45–46` | operations Working rules | reworded, same force |
| Increment paragraph; STATUS from primary evidence, not history; 32 KiB pages; frozen evidence | `AGENTS.md:41–45`, `operations.md:23–27` | operations Working rules | reworded, same force |
| Compaction cadence and preservation | `AGENTS.md:46–47`, `operations.md:93–97` | operations Maintenance cadence | "re-read" became "re-derive"; history moved to block 08 |
| Ninth compaction and stability firing five due, never silently discharged | `operations.md:97–101` | due-state bullet | reworded, same force; history in block 08 |
| Delegated card never advances main | `operations.md:53–56` | `operations.md` worktree bullet | reworded, same force; history in block 07 |
| Gate command block | `validation.md:12–21` | same | unchanged, byte-identical |
| Python suite after matrix regeneration | `validation.md:23–28` | same file | reworded; history in block 11 |
| `--no-fail-fast` (`GATE-NO-FAIL-FAST`) | `validation.md:30–37` | same file | reworded; history in block 12 |
| Submodule step per fresh checkout; exits captured, never piped | `validation.md:39–46` | same file | reworded; history in block 13 |
| Clang recording is disclosure; record beside the gate file; borrow only when noted | `validation.md:48–63` | same file | reworded; history in block 14 |
| Lean CI hygiene | `validation.md:71–95` | same | unchanged, byte-identical |
| Read every applicable rule, cross-topic included | 11 page headers, `README.md:4` | router table paragraph; AGENTS step 3 | reworded, same force (one authority) |
| Bytes are not token billing; measure retrieval separately | `AGENTS.md:49–50` | same | unchanged |
| Every other Working-rules, worktree, integration and reconciliation bullet | `operations.md` | same | unchanged, byte-identical |

No rule changes force. No rule was found obsolete, so none is registered.

## `task_context.py`

Before, any path under `AGENTS.md`, `CLAUDE.md`, `docs/agents/` or the tool
itself selected the `rationale` topic, which read the 23,862-byte pre-pilot
operations text in full. Now:

- those paths still select `docs` and `maintenance`, and list both history
  checkpoints under `on_demand` with the condition for reading them;
- `--topic rationale` reads both checkpoints;
- an unknown path or empty scope still selects every topic and reads history, so
  unknown relevance still widens;
- `--topic integration` also selects the new card-brief section.

History is not a rule. The requirement to read it when changing a rule stands in
the protocol. The on-demand pointer makes the agent choose between the two
readings instead of paying for the larger one on every guidance edit.

## Measurements

Reproduce with `python3 docs/reports/w44-agent-guidance-opus55/measure.py
94502987 <tip> measurements.json`. It exports each revision's own files and
`task_context.py` with `git archive`, builds explicit packets for the tasks
below, and exits nonzero if any constraint id, operations section or gate
command selected before is missing after, or if any rule body differs. These
are source bytes of explicit reading packets, not runtime telemetry or token
billing.

| Surface | Before | After | Change |
|---|---:|---:|---:|
| Injected startup (`AGENTS.md` + `CLAUDE.md`) | 6,538 | 5,817 | −11.0% |
| Five operative agent-cycle files | 29,570 | 31,082 | +5.1% (the card-brief standard) |
| Packet: gateway change (`crates/qcore-c-gateway/src/typed_source.rs`) | 103,076 | 102,743 | −0.3% |
| Packet: provider contract change (`providers/…/libc/stdio.qcore`) | 115,300 | 114,898 | −0.3% |
| Packet: integration record (increment, STATUS, gate, matrix; `--topic integration`) | 48,774 | 50,928 | +4.4% (adds the card-brief section) |
| Packet: agent-guidance edit (`docs/agents/operations.md`) | 47,747 | 23,788 | −50.2% |

All four packets keep every rule id (31, 37, 6 and 6) and every operations
section, and their gate commands are identical. The operative files' emphasis
markers outside the hard-invariant block went from 7 bold spans to 0, `wave N`
mentions from 11 to 3 (the due-state bullet), and self-check phrases from 1 to 0.
[measurements.json](measurements.json) has the per-file breakdown.

The implementation packets barely move because 85–87% of their bytes are topic
pages (`evidence.md` included), whose numbered bodies this card deliberately left untouched. A smaller
startup surface proves no task-cost gain by itself (AGENTS.md); the claim here is
limited to the bytes above.

## Tests

`scripts/tests/test_task_context.py` gains three tests: agent-guidance paths list
history on demand; `--topic rationale`, an unknown path and empty scope all read
it; integration selects the card-brief section. No existing assertion was
changed, since none asserted moved text.

## Validation

Gate records are in [gate.json](gate.json), with [clang-version.txt](clang-version.txt)
recorded in this worktree. Raw logs are private under
`private/w44-agent-guidance/`.

All steps ran at `337b15b1`, the tip that carries every guidance change. Each
exit was captured to its own file, with no pipes.

| Step | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `cargo test --workspace --no-fail-fast` (`CARGO_INCREMENTAL=0`) | 0 | 5,503 passed / 0 failed / 11 ignored, 292 summaries |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Second attempt (see below) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 183 audits |
| `build_matrix.py . --gate gate.json` | 0 | Validation section only (3 lines) |
| `python3 -m unittest discover -s scripts/tests`, after regeneration | 0 | 333 tests (330 + 3 new) |
| `python3 scripts/build_docs_index.py --check` | 0 | |
| `checkpoint.py --check 337b15b1` | 0 | 27 ranges verbatim; coverage complete |
| `measure.py 94502987 337b15b1` | 0 | Every rule id, section and command kept |

The workspace count equals the base's integration gate. The first clippy run
was stopped before it finished, to meet a coordinator disk request
(`cargo clean --profile dev` after the test step), and was rerun at the same
tip. The stopped run is not evidence. The provider gate was not run because
nothing under `providers/` changed. Its figure, 124, is carried from the base's
integration gate, as `gate.json` says.

## Integrator notes

- Likely conflicts: `docs/agents/validation.md` (W44-TEST-SUITE-WALL-TIME may
  change the core test step; this card left the command block byte-identical
  and edited only the prose below it), `docs/agents/operations.md` (W44-AUDIT-PARALLEL-UNITS
  `--jobs` wording, if it lands there), `docs/development/increments/wave-44.md`,
  `docs/reports/INDEX.md` and `docs/reviews/r0_3/acceptance_matrix.json`
  (regenerated gate row).
- Ledger: no entry added or retired. The follow-on below belongs to the existing
  `DOCS-COMPACT-O` cadence, not a new residual.
- For DOCS-COMPACT-9: the numbered rule bodies hold 122 `wave N` mentions and
  incident narratives in 181,685 bytes of topic pages. Gateway 40,218 bytes,
  diagnostics 34,348 and buffers-records 32,132 are the largest; the last two
  have 36 and 26 mentions. Splitting each body into its operative rule and a
  rationale link would shrink every implementation packet. It needs the
  rule-by-rule force table this report uses, and it must keep the
  `preserved:constraint-*` markers and the rule anchors. The same pass can
  decide whether `validation.md`'s Lean CI paragraph, which describes the
  audit runner's internals, belongs in `docs/development/`. It should also
  close or postpone the two due firings the maintenance bullet names.
- The card-brief standard is new policy text for the integrator. Briefs written
  after this merge can cite it rather than restating setup and gates.
