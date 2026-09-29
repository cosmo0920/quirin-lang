# CG01-T-OPERATOR-CLOSURE: prediction (frozen before implementation)

Base `d3508e10` (local `main` = `origin/main`). Frozen after the BEFORE
measurements below and before any importer change was committed.

## BEFORE (measured at base)

- **Cited refusals.** `cg01_before_legacy_route_refuses_the_cited_operators`
  (the 124 rows of `tests/unit/source_operators_vectors.rs`) passed at base:
  - every E01 binary row (6 operators × 6 positions) is
    `unmodeled callback expression BinaryOperator`;
  - every E02 unary row (2 × 6) is `… UnaryOperator`;
  - every E14 row (6) is `… FloatingLiteral` / `… CharacterLiteral`;
  - every E03 row (10 × 7) is `unmodeled callback statement
    CompoundAssignOperator`, except `|=`: its five `int`-place rows import
    and its `short`/`long` place rows are `compound assignment requires int
    operands`.
  - [log](artifacts/before-refusals.log).
- **Corpus arbiter** (`--jobs 8`, base binary): exit 0.
  `resource-diagnostics.json` `ae17152b…`,
  `resource-diagnostics.chunk-contracts.json` `cbbcfd9c…` and
  `monkey-source.json` `57acfc58…` are byte-identical to the archived
  baseline, as W48-QR-SCALAR-TYPES-R2 reported.

## Design being predicted

- The importer reads every CG01 node, but a node is **admitted** only if the
  calling consumer's `NodeAdmission` classifies it. The existing entry points
  pass `NodeAdmission::LEGACY` (no CG01 node).
- Only the Monkey wrapper projection (`import_monkey_wrappers`) is given an
  admission. It is a projection whose every line already says custody (and,
  for scan loops, arithmetic) Unknown and which certifies nothing.
- Every other consumer stays LEGACY. The consumer census found at least one
  reader per chain that would accept an unknown operator without deciding it.

## Predictions

1. **Corpus arbiter AFTER: every artifact byte-identical to this card's BEFORE
   run.** That includes `resource-diagnostics.json` `ae17152b…` and
   `monkey-source.json` `57acfc58…`.
   - Route T reaches the arbiter only through the Monkey callbacks, which are
     LEGACY, and the Monkey wrappers.
   - Every pinned wrapper already imports at base, and the parse is total over
     the body. So no pinned wrapper contains a CG01 node, and its summary
     cannot change.
   - No function moves.
2. **Legacy route AFTER.** The same 124 rows stay refused (the `int |=` rows
   still import, unchanged). Only the reason text changes:
   - Each CG01 node row names the node, for example
     `typed-source node \`binary ^\` is not classified by the legacy
     typed-source consumer`.
   - The `|=` `short`/`long` rows keep `compound assignment requires int
     operands`.
3. **Admitting route AFTER, with every node admitted.**
   - Every E01/E02/E14 row imports (a summary).
   - Every E03 row imports. The place is evaluated once, then there is one
     read, the operation at Clang's computation types with explicit
     `IntegralCast`s, and one writeback.
   - `int |= int` yields a summary equal to the legacy route's.
4. **Refusal controls AFTER, all refused.**
   - `a[i] -= i++`, which the grammar refuses.
   - An RHS or place call in a compound assignment.
   - A compound assignment used as a value (CG02).
   - Pointer `-` and pointer `<=`/`>=` (M15/M16).
   - A node the consumer does not list, refused by name.
   - An admission calling a fault-bearing node custody-neutral, refused by
     validation.
   - `x /= 0` and `a << -1` import on the projection only as
     obligation-bearing. The numeric kernel `checked_binary` refuses them.
5. **CT and source-profile tests.** No CT stage moves. No existing test
   changes expectation except the named-refusal text, and no existing test
   pins that text.
6. **Lean.** One new module, `QCore.TypedSourceOperators`, registered in
   `AuditTypedSourceOperators.lean`. The audit count rises by one.
