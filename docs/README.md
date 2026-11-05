# Documentation hub

Use this page to find the authority for a question. Specifications define the
language; implemented grammars define available source forms; status records
measurements; plans propose work; reports preserve evidence at a checkpoint.
They are not interchangeable.

## Start by task

| I need to… | Start here |
|---|---|
| Understand or try Quirin | [Overview](../README.md) · [examples](../examples/README.md) |
| Build or run the checks | [Setup](development/build-and-test.md) · [complete gate](agents/validation.md) |
| Find code or a crate boundary | [Architecture and entry points](development/architecture.md) |
| Know what is supported/proved | [Status and provenance](STATUS.md) · [implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md) |
| Implement or review a change | [Agent startup](../AGENTS.md) · [numbered rule router](agents/README.md) |
| Select future work | [Planning](#planning) |
| Find evidence for a past claim | [Reports index](reports/INDEX.md) · [wave index](development/increments/README.md) |
| Check a platform claim | [Platform evidence and limits](development/platform-evidence.md) |

## Specifications

- [Language specification r0.3](QCORE_SPEC_r0_3.md) and
  [contract specification](QCORE_CONTRACT_SPEC_r0_3.md): normative definitions.
- [Implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md): consumer support;
  broad parser support is not full conformance.
- [Module modes](QCORE_MODULE_MODES.md) and [QTT alignment](QTT_ALIGNMENT.md).
- [Reevaluation contract](REEVALUATION_SPEC.md) and
  [follow-up](REEVALUATION_FOLLOWUP.md): trust, verdicts and audit gates.
- [Revision acceptance review](reviews/r0_3/REVIEW.md),
  [acceptance record](reviews/r0_3/acceptance.json), and
  [acceptance matrix](reviews/r0_3/acceptance_matrix.json): implementation evidence.

## Planning

| Scope | Authority |
|---|---|
| Registered open work | `named_followups` in the [acceptance matrix](reviews/r0_3/acceptance_matrix.json), then the named card's report |
| Planned `.qv` / `.qr` / `.qrlib` naming | [Quirin extension migration](plans/quirin-extension-migration/README.md) |
| Architecture and W0–W7 direction | [Generalization plan](GENERALIZATION_PLAN.md) |
| Component audit IR and a 6–7 GB audit budget | [AUDIT-IR implementation card](development/audit-ir-card.md) |
| Prospective backlog, retry and engine cards | [Fluent Bit system coverage queue](plans/fluent-bit-system-coverage/README.md) |
| Prospective coverage cards (typed access paths, callee closure) | [Unsupported coverage queue](plans/unsupported-coverage/README.md) |
| Gate and measurement time requirements (scoper, count pins, incremental measurement) | [Gate time reduction](plans/gate-time-reduction/README.md) |
| Prospective expression, place and core-connection cards from the Clang gap analysis | [Clang gap card queue](plans/clang-gap-cards/README.md) |
| Atomic hypotheses below contract premises | [CONTRACT-HYPOTHESIS-GRAPH-V1](plans/contract-hypothesis-graph/README.md) |
| Executed system audit investigations and resume queue | [Three Fluent Bit investigation cards](reports/fb-system-audit-cards/README.md) |
| Broader workstreams | [Post-Cycle-9 roadmap](ROADMAP_AFTER_CYCLE_9.md) |
| Restart/context tooling plan | [Resume plan](plans/resume-2026-09-13/README.md) (dated checkpoint; revalidate old blockers) |

Search an exact card id before choosing work. A report's follow-on may have been
discharged later; neither a historical title nor a rule mention establishes
current open status. Add matrix entries through its generator, not by hand.

## Evidence and history

- [Reports index](reports/INDEX.md): alphabetical lookup by card directory and
  original report title; it assigns no completion or safety status.
- [Increment index](development/increments/README.md): one paragraph per
  increment, wave-by-wave; this names the current record file.
- [Cycle requirements/evidence](development/cycle-status.md) and
  [cycle files](cycles/): bounded milestones, not whole-language conformance.
- [Persistent production-audit inputs](CYCLE19_BUILD_INPUTS.md).

Frozen reports and artifact paths remain stable. A newer measurement belongs
in its own report with a link to the predecessor; do not rewrite the old claim.

## Maintaining documentation

Keep root guides short. Put current rules in `docs/agents/`, current numbers in
[STATUS.md](STATUS.md), setup in `docs/development/`, proposed work in plans,
and evidence in its report. Do not mirror volatile counts across these pages.
Update the hub/router when adding a new authority. Preserve rule numbers and
existing link targets; historical copies must say they are checkpoints.

[DEVELOPMENT.md](../DEVELOPMENT.md) retains legacy heading links. The
[restructuring report](reports/docs-structure-compression/README.md) records
the old-to-new map, exact preservation checks and size measurements.

For a change-specific agent reading packet, see the [task reading protocol](agents/operations.md#task-reading-protocol) and [router](agents/README.md#generated-task-packet).
