# CT41 bounded atomic-publication prerequisite

This card adds a fail-closed API prerequisite for one C11 atomic publication
shape. It does **not** map CT41. Under the historical attribution rule CT41
remains unmapped, there is no `reevaluation/ct41` fixture directory, and
`CT41-TYPED-SOURCE-UNDISCRIMINATING` remains registered rather than retired.

## Scope and result

`qcore-core::atomic_publication` checks a finite five-event model: atomic flag
initialization, one payload write, one release store, one acquire load that
observes the published value, and one payload read. The initializer must differ
from the published value, so the successful observation identifies the unique
release store in this model. Audited Lean proves the matching three-edge
happens-before path and proves that plain, volatile and relaxed controls do not
establish its synchronization edge.

The C gateway extracts only closed typed-source facts. It checks the complete
preprocessed translation-unit AST, exact selected declarations and bodies,
static `_Atomic(int)`/`int` objects, the canonical `memory_order` enum identities,
and exact reference inventories. `qcore-verification`, which is already above
both gateway and core in the enforced workspace layering, is the only adapter
that combines that source certificate with the core model certificate. This
preserves the dependency rule: `qcore-c-gateway` does not depend on
`qcore-core`.

The published certificate binds raw and preprocessed source digests, all
declared analysis-configuration fields, the caller's revision label, selected
API names, property, semantic profile, scope, execution bounds, assumptions and
evidence version. The invocation bounds are different evidence from the AST
inventory: **exactly one publisher invocation and exactly one successful
consumer invocation are caller-declared premises**. The AST proves only the
static declarations, source order and absence of other visible accesses and
entry references in the selected translation unit. It does not prove dynamic
call counts.

## Fail-closed controls

The focused tests distinguish release/acquire from relaxed, plain and volatile
flags; reversed publication order; a consumer bypass; equal initializer and
ready values; effectful payload right-hand sides; extra writers, readers and
entry references; unsupported bounds; and shadowed memory-order parameters.
Three review findings received dedicated controls:

- Integer literals are accepted only when Clang types the literal itself as
  `int`; the value-preserving `_Atomic(int)` initialization wrapper is the only
  admitted implicit cast. Narrowing `4294967296`/`4294967297`, unsigned literals
  and other integral conversions refuse rather than comparing token values.
- `GCCAsmStmt`, `MSAsmStmt`, `FileScopeAsmDecl`, `AsmLabelAttr` and `AliasAttr`
  refuse anywhere in the full AST. Assembly strings and assembler names cannot
  bypass `DeclRefExpr` counting or alias the flag and payload behind distinct
  declaration ids.
- Any second `VarDecl` or `FunctionDecl` with a selected profile name refuses,
  including block-scope `extern` redeclarations and function prototypes. A
  redeclaration cannot move a reference onto a different Clang declaration id
  and escape the inventory.

## Validation

Private logs are under
`measure/ct41-sync-model-gate-20260919T2130JST/`; the complete pre-layering run
is under `measure/ct41-sync-model-gate-20260919T2100JST/`. The latter completed
all targets with **4,764 passed, 1 failed, 10 ignored over 242 result
summaries**. Its sole failure was the real layering defect introduced by the
first checkpoint: `workspace_dependencies_point_strictly_downward` rejected
`qcore-c-gateway -> qcore-core`. Commit `eb5e283b` corrected that ownership;
the final-tree focused results are core **2/2**, gateway **9/9**, verification
adapter **2/2**, and workspace layering **11/11**. A repeated complete workspace
run was started on `eb5e283b` and intentionally stopped after the integrator
waived another per-card run in favor of the definitive combined-tree gate. No
per-card clean-full-pass claim is made.

The remaining standing steps on `eb5e283b` are clean: submodule initialization,
fmt, clippy, both reevaluation examples, and **144/144 Lean audits** all exit 0.
The direct `AuditAtomicPublication.lean` run is also green. Clang 21.1.8 was
recorded in this worktree, not borrowed. Provider **122** is carried rather than
remeasured because neither providers nor schemas changed. The matrix generator
exited 0, changed only derived/current validation fields (the named-follow-up
array itself was not edited), and the required post-regeneration Python gate
passed **305/305**. `gate.json` records the incomplete standing-gate status
honestly; the integrator's combined gate must replace it as current-tree
evidence.

## Limits and integrator reconciliation list

This result is conditional ordering for one bounded API shape. It proves no
termination, eventual observation, dynamic invocation count, arbitrary C
atomic pattern, C-wide safety, or Rust-to-Lean extraction/refinement theorem.
Faithful Clang extraction and Lean-model-to-Rust correspondence remain explicit
review premises. Native execution is not claimed.

For integration:

1. Merge source checkpoint `eb5e283b` or its final descendant, never
   `fc85fae7` alone; the latter violates workspace layering.
2. Run the combined-tree full gate and publish an integration `gate.json` before
   treating the validation row as passing.
3. Keep CT41 unmapped under historical attribution clause (b), add no CT41
   fixture directory, and do not retire `CT41-TYPED-SOURCE-UNDISCRIMINATING`.
4. Do not change global status, increments, rules or named follow-ups from this
   card. A future mapping card must supply an executed CT artifact and separate
   native evidence.
