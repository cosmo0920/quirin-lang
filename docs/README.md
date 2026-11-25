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
| Select future work | [Planning](#planning) · [planning index](plans/README.md) |
| Find evidence for a past claim | [Reports index](reports/INDEX.md) · [wave index](development/increments/README.md) |
| Check a platform claim | [Platform evidence and limits](development/platform-evidence.md) |

## Specifications

- [Language specification r0.3](QCORE_SPEC_r0_3.md) and
  [contract specification](QCORE_CONTRACT_SPEC_r0_3.md): normative definitions,
  semantic domains, contract consumers, and verdict/diagnostic architecture.
- [Implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md): exact consumer
  input grammars and profile boundaries; broad parser support is not full
  conformance.
- [Module modes](QCORE_MODULE_MODES.md) and [QTT alignment](QTT_ALIGNMENT.md).
- [Reevaluation contract](REEVALUATION_SPEC.md) and
  [follow-up](REEVALUATION_FOLLOWUP.md): trust, verdicts and audit gates.
- [Revision acceptance review](reviews/r0_3/REVIEW.md),
  [acceptance record](reviews/r0_3/acceptance.json), and
  [acceptance matrix](reviews/r0_3/acceptance_matrix.json): implementation evidence.

## Planning

Start at the [planning and workstream index](plans/README.md), which separates
active engineering queues from completed or halted migration plans.

| Scope | Authority |
|---|---|
| Canonical open-work tracker | `named_followups` in the [acceptance matrix](reviews/r0_3/acceptance_matrix.json), then each named card's report |
| Active C-gateway & coverage queues | [Clang gap queue (`CG00`–`CG30` residuals)](plans/clang-gap-cards/README.md) · [Typed access paths & callee closure](plans/unsupported-coverage/README.md) · [Fluent Bit system coverage](plans/fluent-bit-system-coverage/README.md) |
| Active infrastructure & memory budgets | [Gate and measurement time reduction](plans/gate-time-reduction/README.md) · [AUDIT-IR 6–7 GB budget](development/audit-ir-card.md) |
| Architecture, crate layering & cycle milestones | [Generalization plan (`W0`–`W7`)](GENERALIZATION_PLAN.md) · [Pluggable card seams (`A`–`G`)](plans/pluggable-cards/README.md) · [Post-Cycle-9 roadmap (`Cycles 10–20`)](ROADMAP_AFTER_CYCLE_9.md) |
| Completed or halted plan archives | [Extension migration (`.qv`/`.qr`/`.qrlib` landed)](plans/quirin-extension-migration/README.md) · [Runtime `#import` (landed)](plans/runtime-imports/README.md) · [Hypothesis graphs (`v1`–`v4` landed; guarded adoption halted)](plans/contract-hypothesis-graph/README.md) · [Executed Fluent Bit investigations](reports/fb-system-audit-cards/README.md) |

Search an exact card id before choosing work. A report's follow-on or a plan's
row may have been discharged later; neither a historical title nor a rule
mention establishes current open status. Add matrix entries through
`scripts/apply_amendments.py` and the matrix generator, not by hand.

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
