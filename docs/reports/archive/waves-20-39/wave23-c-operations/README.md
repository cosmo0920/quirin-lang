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
and shadowing gates. RHS tokens containing `=` or increment/decrement keep
the old refusal (including equality comparisons on an assignment RHS); this
conservative scalar-RHS boundary is `C-QUERY-SCALAR-RHS`. Standalone query
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

The before run exits 0, with 185 units, and reproduces the frozen diagnostic
artifact **byte for byte**: 272 leaks, 4,408 unknown aliases and 49,821
unsupported diagnostics. The chunk-contract artifact also matches byte for
byte. `diff-archive-to-before.json` itemizes all five artifacts. The relocated
CMake input paths change the compile-entry envelope and four wrappers in the
source artifact (including assertion-file strings); three source-effect rows
carry different identities. Those differences already exist before this card.
They are not normalized out of the source and are not counted as this card's
payoff. The implementation comparison uses fresh BEFORE and AFTER runs with
the same source paths and package. `raw-contract-keys.json` publishes both raw
17-key inventories; the standard itemizer finds no normalized contract change.

Both fresh audits exit 0. The AFTER has the same 185 units (2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only) and 4,834 functions (150 Safe /
84 provisional Unsafe / 4,600 Unknown). No unit or function changes verdict.
Leaks stay **272**, unknown aliases stay **4,408**, and unsupported diagnostics
fall **49,821 → 49,794**: exactly 27 removals, zero additions. These comprise 20
branch-condition refusals, six `sizeof` operator refusals and one `for`
condition refusal. The removed leaves span 11 functions in seven units:

| Unit | Removed diagnostics |
|---|---:|
| librdkafka/nanopb/pb_decode.c | 9 |
| librdkafka/nanopb/pb_encode.c | 9 |
| fluent-bit-chunk/flb_input_chunk.c | 2 |
| librdkafka/lz4.c | 2 |
| librdkafka/rdkafka_cgrp.c | 2 |
| monkey-libevent/mk_server/mk_net.c | 2 |
| librdkafka/rdbuf.c | 1 |

`diagnostic-diff.json` identifies every leaf; `diff.json` is the standard
five-artifact comparison. The other four artifacts are byte-identical BEFORE
and AFTER, including both normalized and raw contract inventories (17 modules).
`artifacts-before/` and `artifacts/` freeze both runs, with uncompressed and
compressed SHA-256 receipts in `artifact-manifest.json`. The prior archives
remain untouched. `after-census.json` records the remaining diagnostic inventory.

The closing gate passes **3,575 workspace tests (8 ignored), 193 Python tests,
95 Lean audits**, both reevaluation probes (exit 0), fmt and clippy. Six new
workspace tests cover the positive and refusing boundaries above. Provider
code and packages are unchanged; the ledger's 122 provider tests are carried
from the preceding checkpoint, not rerun for this card. `logs/` contains the
gate and audit logs; `gate.json` supplies the matrix's measured gate fields.

The workspace gate required infrastructure recovery, recorded rather than
hidden: a sandboxed run encountered LeakSanitizer's ptrace restriction and was
rerun outside the sandbox with leak detection intact. That full workspace run
passed the first seven crates, then a shared-target executable disappeared
while starting a verification test. The remaining three crates were rebuilt
and passed in `target/wave23-c-operations`; every workspace doctest was then
run there too. `workspace-gate.json` partitions and deduplicates those runs
(3,151 + 393 + 31), without counting the partially repeated verification
crate twice. These were infrastructure interruptions, not failing assertions.

During verification the shared branch advanced to `1a7846dd`, whose wave-22
closing commit also included this card's implementation, tests and proof.
`measurement.json` records that provenance; the subsequent closing commit
adds this card's completed evidence and ledger reconciliation.

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
