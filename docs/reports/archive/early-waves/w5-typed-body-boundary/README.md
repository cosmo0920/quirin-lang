# Generic typed-body comparison and reviewed witness boundary

Baseline `1ff50b7`, 2026-09-07. W5 moves the remaining generic complete-body C
comparison from the compiler into `qcore-c-gateway::typed_source_match`. Consumers
can compare an unrelated C function with a supplied reference without depending
on the compiler or a library provider. The existing reviewed ChunkIO traversal
and parent-teardown routes retain their private witness constructor.

## Boundary

`compare_typed_body` returns an opaque `TypedBodyComparison`. It checks actual
Clang-derived typed ASTs after validating/preprocessing the source, validates the
caller-selected ordinary record/field representations, and appends the reference
after preprocessing so input macros cannot rewrite it. Normalization preserves
local declaration identity, types, calls, control flow and tokenized `offsetof`
operands that Clang omits from its JSON. Attributed, volatile/atomic or unequal
source remains rejected. This deliberately conservative equality does not infer
that two differently shaped programs are semantically equivalent.

The caller supplies the reference, field profile and revision label. A successful
comparison does **not** authenticate that label, establish reference review,
validate a lifecycle contract or prove the function safe. One regression compares
a null-dereferencing function with itself to make this distinction explicit.
The result has private fields, no public constructor and no deserializer.

The compiler's public `SourceMatch` type now wraps that comparison through a
private field. Its constructor remains crate-private; no public conversion accepts
an arbitrary comparison. Existing integration callers still select the reviewed
reference, check the pinned revision and apply their contract policy. A transparent
serializer preserves the original four-field report exactly. Public getters for
function and source revision retain their behavior. Four compile-fail tests reject
construction/deserialization of the generic comparison and reviewed wrapper.

This separates the reusable algorithm from reference review authority without
turning the latter into a public caller assertion. Library-specific references,
policy premises and semantic composition remain in their integration adapters.
No provider gains access to the gateway through a new dependency, and no schema
or wire protocol changes. The compiler's now-unused raw typed-AST import is removed.

## Evidence

Five provider-free gateway tests use unrelated C record/function names. They
check local renaming, changed callees/values/extra calls, ordinary field types,
bitfields, volatile/atomic fields, missing records, macro redirection, `offsetof`
member identity, attributes and the absence of a safety claim for an arbitrary
reference. Existing library mutation tests continue to execute.

`check-relocation.py` compares the original and moved algorithm's production
tokens, retaining exact string literals and diagnostics. All **1,447** tokens
match after only the documented type/function renames and public visibility
change. `relocation-parity.json` records source and normalized token hashes.
This is source parity evidence, not a theorem of Rust semantics. No semantic model
or Lean definition changes; the existing Lean build and all 20 axiom audits pass.

All **784 workspace tests**, **31 provider tests**, **43 Python tests**, format,
Clippy and both reevaluation probes pass. The first Clippy run identified the
obsolete compiler import; it was removed and the affected gates rerun. The final
logs and source hashes are archived in `validation.json` and adjacent files.

The full 185-unit Cycle19 run preserves all four artifacts byte-for-byte against
the baseline: source projections, diagnostics, chunk contracts and source effects.
Counts remain **2 Safe /42 provisional Unsafe /137 Unknown**, plus four data-only
units, and **3 checked /93 Unknown** source-effect functions. The serialized
reviewed witnesses therefore retain exact evidence identity as well as verdicts.
The 58 contract modules and CT coverage (**9/50 mapped, 41 unmapped**) are unchanged.

```sh
python3 -B docs/reports/w5-typed-body-boundary/check-relocation.py
cargo test -p qcore-c-gateway --test typed_body_comparison
cargo test --workspace
```

Provider semantic extraction, source-bound lifecycle composition, registry-based
corpus replacement and compatibility-route deletion remain open. This increment
does not promote raw units or establish arbitrary source/reference safety.
Existing subagents remain stopped by their reported usage limits; work continued
locally without starting replacement agents against the same limit.
