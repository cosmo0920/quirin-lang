# W6 frontend contract and lexical-scope gates

The two frontend findings recorded in W6 batch 2 are repaired at their bounded
interfaces. A source function with any nonempty `contract { ... }` now rejects
with Q021 at the clause span. `check_whole_program` owns this gate, so a caller
cannot bypass it by supplying a semantic bundle. `compile_files` propagates it
with the original file location, including clauses on an unused function in a
second input file. Requires, ensures and quant clauses are all rejected until an
enforcing source interpreter exists. Empty contracts and extern contract-module
interpreters retain their existing behavior.

Before this change, `fn main() -> i32 contract { quant normal wall_clock <= 5; }
{ return 0; }` compiled without enforcing the declared bound. It now rejects with
Q021, "source function contract clauses are not yet enforced; refusing unchecked
contract". This repairs optimistic acceptance; it does not implement quantitative
function contracts or map CT39.

`with lock m as g { ... }` and `thread scope s { ... }` bind g/s only in their
body's lexical scope. Header inputs resolve in the enclosing scope, including
`with lock m as m`. Each body binder has a fresh symbol, same-body duplicate
bindings reject with Q012, and use after leaving the body rejects with Q013.
The language's existing nested-block lowering still rejects these statements
with Q500; this change does not claim mutex checking or concurrency execution
from source. No additional CT case is marked mapped: 5 of 50 remain mapped,
45 unmapped.

Tests cover direct checker bypass attempts, all three clause families, source
span/file identity, empty-contract controls, body-local symbol identity,
shadowing, missing mutex inputs, sibling/body escape and duplicates. Driver
regressions ensure resolved guard/scope syntax still fails closed at the
unimplemented lowering boundary. Lean's whole-program relation now requires
zero unchecked source clauses, with rejection and acceptance theorems. A reduced
lexical-scope stack models binder entry and restoration; the audit exposes its
foundational dependencies. This is not a concurrency semantics theorem.

All standing gates pass. validation.json and compressed logs record workspace
tests, formatting, Clippy, both probes, the Lean build and Cycle 11 axiom audit.
No existing-C analyzer or provider route changed. The spec reserves Q021 for this
unsupported-contract boundary. SHA256SUMS binds the report.
