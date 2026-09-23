# QCore-C agent briefing

This file is the startup router and authority for which rules to read. Detailed
rules live in the linked topic files; their existing numbers and force are
unchanged. This structure deliberately replaces the former monolithic briefing.

## Start here

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

Use `rg -n` on an exact symbol/card/heading before reading a long section.
Select JSON keys rather than dumping a matrix. Historical reports retain their
checkpoint claims; each report owns its evidence. The authoritative specifications
are [language](docs/QCORE_SPEC_r0_3.md) and
[contracts](docs/QCORE_CONTRACT_SPEC_r0_3.md); supported source forms are in
[implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).

## Documentation retrieval and compact context

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

Measure startup guidance, cumulative retrieval and unique source bytes separately;
byte counts are not token billing. A smaller index alone proves no task-cost gain.

<!-- preserved:hard-invariants:start -->
## Hard invariants (violating these is a defect, not a style issue)

1. **Fail-closed.** Missing contracts, unmatched profiles, unsupported
   constructs, provider absence/failure/mismatch, unproven call effects →
   `Unknown`. Never optimistic fallback. Refactors must not flip any
   `Unknown` to `Safe`.
2. **Verdict contract.** Every published result carries
   `(artifact, property, semantic profile, scope, bounds, assumptions,
   evidence version)`. `Safe` only after all obligations discharge; `Unsafe`
   only for a validated reachable violation; everything else `Unknown`.
   Projection coverage is not safety coverage; unknowns stay in the
   denominator; a provisional `Unsafe` is never presented as an upstream
   defect.
3. **Parity before deletion.** A hardcoded analysis route is removed only
   after its replacement reproduces the pinned verdicts/coverage, with the
   diff archived under `docs/reports/`.
4. **Gate ordering** (`docs/REEVALUATION_SPEC.md` §9): certificate
   containment (G1) before provider expansion; Cycle 19 routes stay until
   G4 parity.
5. **Pinned expectations are intentional.** `qcore_contract_corpus.rs` pins
   per-file contract/clause counts (editing provider-owned contracts or
   the shared `providers/quirin-provider-*/contracts` requires updating
   its tables); a test asserts the `third_party/libevent` submodule revision; pinned commit hashes are load-bearing identity checks.
6. **Lean parity.** Semantic changes need matching Lean definitions/theorems;
   audits must stay placeholder-free with no project axioms.

<!-- preserved:hard-invariants:end -->

## Standing constraints when touching specific subsystems

Read [the numbered rule router](docs/agents/README.md). The full text lives in
topic files, not in this startup guide. Evidence/trust rules 21–28 apply across
topics; diagnostic and source-identity rules also cross crate boundaries.

## Status ledger (do not overstate)

See [current measurements and provenance](docs/STATUS.md).

## Repository map

See [architecture](docs/development/architecture.md#repository-map).

## What this project is

See [the public overview](README.md).

## Standing per-change gate

Read and run [the full gate](docs/agents/validation.md#standing-per-change-gate).

## Lean CI hygiene

See [Lean audit policy](docs/agents/validation.md#lean-ci-hygiene).

## Key entry points

See [code entry points](docs/development/architecture.md#key-entry-points).

### Diagnostics namespaces

See [namespace policy](docs/agents/diagnostics.md#diagnostics-namespaces).

## Where to read more

[Documentation hub](docs/README.md) · [Rule router](docs/agents/README.md) ·
[Preservation evidence](docs/reports/docs-structure-compression/README.md).
