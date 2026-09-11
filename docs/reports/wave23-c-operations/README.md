# Wave 23: existing-C gateway operations

This card implements the fixed **scalar** `sizeof` and `_Alignof` family in
six expression positions and scalar assignments/initializers. It is the first
bounded slice of the next gateway wave, not completion of C support.
Quirin source syntax and its C emitter are outside this card.

## Why this slice

The latest available frozen diagnostic artifact at entry was
`wave22-aom-destroy/artifacts/diagnostics.json.gz`; the working ledger named a
wave-22 corpus consolidation that was not yet present on disk. That artifact
records 18,822 indirect-access refusals, 12,446 uncontracted-call refusals,
9,161 branch-condition refusals, 2,282 braced constructs, 1,722 unsupported
assignments, 1,503 return-expression refusals, and 471 `sizeof` operator
refusals. `baseline-census.json` and `census.py` preserve the whole inventory.
These are diagnostic occurrences across functions and translation units;
they are neither distinct operations nor projected gains from this change.

## The implementation and its premise

`type_query::scalar_operands` reads the same translation unit's existing typed
model. It admits a parenthesized scalar type name (including a resolved scalar
typedef) under `sizeof` or `_Alignof`, or one declared scalar variable under
`sizeof`, with or without parentheses. Arrays, pointer types, dereferences,
compound operands, unknown typedefs and GNU `typeof` operators remain refused
by this new rule. The declaration reader must be readable. A VLA typedef is
opaque; it never becomes a scalar just because it appears inside `sizeof`.

The C premise is [C11 N1570 §6.5.3.4](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf):
these admitted operands are unevaluated. VLA size expressions are outside the
premise. Faithfulness of the existing typed token reader remains a premise;
this card does not establish arbitrary C memory safety or a target ABI.

The normalized `1` is solely an operand-class marker for the existing custody
predicates. It is **never** a numerical size, alignment, branch decision or CFG
input. Original tokens still decide branch/null refinements and certificate
replay. Sibling calls still require their own declaration and neutral effects.
Expression-position assignments and increments alongside a query refuse the new
rule: a NoOp would otherwise leave stale scalar facts. Scalar statement
assignments use the existing `OpaqueValue` instruction, which forgets the old
value and preserves custody. The regression starts at `n = 0`, assigns a size,
and requires the subsequent `if (n)` leak to stay visible.

The six consumers are `if`, `while`, `for` condition, `do/while`, `switch`, and
`return`. Scalar assignment destinations must pass the existing declaration
and shadowing gates, and RHS mutations remain refused. Standalone query
statements, non-scalar fixed objects, and statement-call argument admission
are not widened by this card. Existing type-operator handling outside this
rule is unchanged.

Three earlier refusal pins are changed deliberately: the standalone
`unsigned long` return query in `neutral_expression_positions`, the scalar
`sizeof(n)` typed-predicate pin, and the `sizeof(int)`/`_Alignof(int)` cases in
the raw analyzer's type-operator table. Each gains admitted evidence; adjacent
VLA, uncontracted, pointer and GNU-operator refusals remain pinned.

## Proof and validation

`QCore.FixedScalarQuery` proves exact preservation of surrounding effect
sequences under admitted-query erasure, rejection of VLA/opaque queries, and
that assignment forgets the old scalar fact while preserving custody and
allowing both branches. Nine theorems are exposed by `AuditFixedScalarQuery`.
This is a model of the admission and forgetting rules, not extraction of the
Rust parser or a proof of the C standard's premise.

Measurement and closing gate results are recorded below when complete.

## Remaining gateway work, ordered by prerequisites

| Slice | Concrete boundary and acceptance requirement |
|---|---|
| Fixed object queries | Extend the typed reader to prove fixed arrays/complete records and operand extent before widening `sizeof`; retain VLA evaluation and unknown-type refusals. Census production sites first. |
| Typed scalar expression positions | Separate remaining casts, compound assignments and neutral operands from actual resource effects. Invalidate scalar facts for every modeled mutation; don't turn whole expressions into NoOp when they write state. |
| Aggregate access | `ADDRESS-OF-MEMBER-MAP-LIVENESS`, `-TAGGED-PAIR`, `-INLINE-AGGREGATE`, `-DOT-FORM`: require typed places, object liveness and measured production reach per shape. |
| Release declarations | Read `flb_input_chunk_destroy_corrupted` and `flb_input_chunk_destroy_direct_routes` independently; measure `AOM-DESTROY-SIBLINGS-UNDECLARED`, including the seven-leaf family, before installing any declaration. A neighbouring destructor's name is no evidence. |
| Joined provenance | `AOM-DESTROY-JOINED-PROVENANCE` needs a model for the lookup/acquisition join and its nullness. Keep unknown lookup identity and nullable acquisition separate; do not manufacture entry custody. |
| Indirect calls, retention, recursive summaries | Require new capabilities: target/effect evidence, retention relations, or validated recursion reasoning respectively. They are not missing dispatch arms. |

Every semantic slice needs held-out positive and refusing controls, Lean parity,
certificate rechecking, and an attributed pinned-corpus diff. Unit promotions,
function promotions and withdrawn violation claims must be reported separately.
