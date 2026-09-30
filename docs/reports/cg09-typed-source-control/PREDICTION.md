# CG09-TYPED-SOURCE-CONTROL: frozen prediction

Frozen before any importer change, on base `fe9c5b48` (main after CG14's
integration). It re-freezes CG00's
[CG09 prediction](../cg00-clang-gap-baseline/predictions/CG09.md) (0 corpus
functions: the corpus arbiter runs no route-T function that a control form
blocks) and amends it only with the per-consumer and CT/profile predictions
below, which CG00 did not make.

## Route and plan

Route **T** only (`crates/qcore-c-gateway/src/typed_source.rs`). The importer
will *read* `while`, general `for`, general `do`, `switch`/`case`/`default`
(with fallthrough), `goto`/labels, `break` and `continue`. Acyclic forms are
lowered into T's existing path vocabulary (guards, `LocalEnd`); a loop's
back-edge needs the consumer's invariant or summary. Lowering through route G's
CFG was considered and declined: G's CFG compiles to custody instructions
(`LegacyInstruction`), while T's product is a finite set of typed operation
paths, so there is no shared representation to lower into.

Every importer call passes a control admission. The legacy one admits no form.

## Predictions

1. **Corpus arbiter byte-identical.** `resource-diagnostics.json`,
   `…chunk-contracts.json` and `monkey-source.json` identical to this card's
   BEFORE. T reaches the arbiter only through the Monkey wrapper and callback
   imports, and every pinned wrapper and callback already imports at base
   (CG01's finding, re-checked in BEFORE), so no summary can move.
   **0 functions move.**
2. **System and tail-pause slices: not run.** Route T does not reach them.
3. **Consumers.** The 19 LEGACY import sites refuse every control form by
   name. The Monkey wrapper projection, which certifies nothing, admits the
   acyclic forms. No consumer admits a loop or `continue`, because none
   supplies an invariant or summary (card CG27 owns those). So loops stay
   refused everywhere, now by name.
4. **CG00 controls.** C01 flips from the generic statement refusal, and from
   the scan-shape and literal-zero messages, to a named loop refusal. C02 and
   C03 flip to named refusals under the legacy consumer, and each is shown
   admitted under an admitting consumer.
5. **CT and source profiles.** No CT stage moves.
   `scripts/ct_stage_summary.py . --check` passes before and after. No
   source-profile test changes, because no pinned profile function contains a
   control form outside T's existing shapes.
6. **Lean.** One new audit module with the five properties the brief names:
   jump-out lifetime ends, a back-edge without an invariant refused, budget
   exhaustion never Safe, switch fallthrough composition, and unlisted
   statements refused.
