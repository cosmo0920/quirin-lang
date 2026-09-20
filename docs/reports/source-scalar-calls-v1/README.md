# SOURCE-SCALAR-CALLS-V1

2026-09-20. Base: `418af31dfa4567085f68882d7fab7d2e69a042c7`.
Implementation and validation record for ordinary pure Quirin functions.

## Implemented boundary

`source.scalar-calls.v1` admits explicit i32/unit signatures, i32 parameters,
immutable i32 locals with optional annotations, direct nested source calls,
parentheses, unary minus and `+ - * / %`, with a final explicit return.
Multiplication/division/remainder bind more tightly than addition/subtraction;
binary operators associate left. Calls and operands evaluate left to right.
A unit function may return a unit call; unit values cannot be stored in locals
or passed as i32 arguments. Call statements may discard either result type.

The core type-checks every definition, including uncalled ones, and rejects
cycles across the entire call graph. Signatures and bodies remain in a private
checked value; C emission consumes that value, retains real functions and calls,
and never substitutes a precomputed entry result. Library modules have no entry
observation or C artifact. An executable has one closed, zero-argument entry.
Its interpreter result is published separately from the all-definition type check.

Arithmetic is checked signed i32. Division truncates toward zero; remainder has
the dividend's sign. Zero divisors and MIN/-1 division **and remainder** refuse.
The closed entry is evaluated during checking, so its overflow refuses with Q020.
Other input values are not claimed overflow-free. Emitted C uses widened int64
arithmetic and explicit narrowing/division guards, terminating with exit 86 and
`scalar arithmetic fault` on a fault. No wrapping or undefined signed overflow
is used. Pure functions cannot allocate, transfer ownership, throw, perform I/O,
or declare effects other than an empty row.

Bounds: 256 functions, 16,384 expression nodes across the IR, 16,384 source tokens
per function, expression and call-graph depth 64, and 100,000 evaluated expression
steps per closed invocation. Parser recursion and left-associated chains are
bounded independently. Bounds refuse rather than truncate. No runtime branches,
loops, mutation, generics, indirect calls or cross-profile calls are admitted.
Usage annotations remain on the old route unless a new expression/call feature
selects this profile; the new profile refuses them rather than skipping them.

## Integration and compatibility

The implementation lives in qcore-core (typed IR, checker, interpreter and C
backend), qcore-frontend (reader over retained post-desugaring tokens), and
qcore-verification (driver). The CLI names the new profile. No provider, C gateway,
contract corpus, ownership rule or existing route implementation is changed.
Selection happens after specialized source profiles. A nonliteral return/local,
typed local, or argument-taking call to a declared function selects the new
profile. Sequential qcore_acquire/qcore_check keep their custody route; the
qcore_ prefix is not reserved. Zero-argument/literal legacy programs keep their
route. Existing while and usage-diagnostic pins are explicitly tested.

No words or tokens are reserved. New conditions extend OrdinaryTypes Q020;
nonempty source contracts retain Q021. The diagnostic namespace is not expanded.
Initial compatibility testing caught while being misclassified as a call; the
selector now requires a declared function at the statement call position.

## Proof and validation

QCore.ScalarCalls defines checked arithmetic, argument binding, immutable frame
extension, decreasing call ranks, and relational expression/body/call semantics.
AuditScalarCalls audits range and arithmetic exactness, invalid division,
argument binding, cycle refusal, slot preservation, unit result separation and
an actual increment(41)=42 derivation. Rust reads the eight Lean arithmetic
agreement vectors and checks them against its kernel. This finite differential
and reviewed operational correspondence are **not** a compiler extraction or
universal Rust-to-Lean refinement proof. Implementation budget counters and
source parsing are outside the relational theorem claims.

Focused tests additionally compare 500 arithmetic boundary combinations against
widened arithmetic, exercise nested calls, type errors, uncalled bad definitions,
mutual recursion, bounds, library mode, multi-file input, and old route selection.
Sanitized native checks exercise the real call graph and five faulting helper
inputs (addition overflow, signed division/remainder overflow, zero division,
unary negation overflow). A first sandbox run preserved its LeakSanitizer ptrace
failure; subsequent native runs use the ordinary host with sanitizers enabled.
A fixture omitted its error declaration initially; that fixture was corrected.

Full standing gate: in progress. Private logs:
`/home/cosmo/Documents/GitHub/qcore-c/private/source-scalar-calls-v1/`.

## Integrator reconciliation list

- Apply the grammar/specification and source-route diagnostic amendments together;
  this is a pure profile extension, not general type-system conformance.
- Keep runtime conditions, profile composition and record/resource call values
  open. Existing IF-RUNTIME-CONDITION and AGGREGATE-ACROSS-CALL are not discharged.
- Publish measured gate counts after validation and regenerate the matrix. C
  projection counts are carried evidence: this card does not remeasure that corpus.
