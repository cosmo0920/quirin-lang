# Quant condition prediction — before implementation

User decision 2026-10-26: introduce `quant condition` with explicit entry/exit
selectors. `entry` is a prerequisite; `normal`, `error(E)` and `any` are exit
assertions. Reuse the bounded proposition AST/type checker. Preserve selector,
source spans, guards and mathematical formulas. Structural admission supplies
no proof, execution, source binding or interpreted call summary.

This interface increment predicts zero native corpus/system/slice verdict moves.
Selected resource/bounds/local-condition consumers must refuse the new clauses.
Hypothesis graph inspection must refuse an unsupported entry condition rather
than dropping it; exit conditions must never become entry hypotheses. An
explicit typed condition inspection view retains all conditions. The existing
provider postcondition migration will follow this source interface, with each
replacement and remaining source/model obligation recorded.
