# Cycle 19: opaque backend custody through parent release

The checked context/stream teardown chain now accepts the source-derived
unsupported backend class as a conditional child case. It releases the outer
chunk, stream and context while retaining the exact stream identity, chunk
identity and integer backend kind in an opaque residual inventory. This inventory
includes any unclassified memory or OS obligations; an empty typed filesystem
residual list does not imply their absence. Either inventory or a nonzero up
counter blocks conditional discharge.

The extension requires independently rechecked close evidence with exactly the
same source and dependencies as the enclosing chain. The original API continues
to reject unsupported child cases until that evidence is attached. Known backend
values cannot select the unsupported class. Duplicate identities, incomplete or
underflowing counters and source mismatches reject. All entry provenance, ring,
exclusivity, dependency ABI and caller assumptions in the existing 12-clause
policy remain premises. Native constructors are not assumed to return these
children; the projection does not establish reachable executions.

Three new Rust tests cover multiple unsupported children, mixed streams with
memory and unresolved filesystem cases, extreme integer discriminants, missing
or mismatched evidence, known-kind substitution, duplicate identities, counts,
policy mutations and exact parity for all 43 previous filesystem cases. Four
conditional examples are added to the pinned audit. Three Lean theorems prove
exact opaque inventory preservation, refusal to discharge an opaque residual and
the requirement that both inventories and the up counter be empty. They concern
the abstract composition, not the C compiler or Rust importer implementation.

All 407 workspace tests, 46 contract modules, two audit-example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency audit
and the pinned external audit pass. Every preceding source/diagnostic/contract
record remains identical after removing the new nested-unsupported entry.
Aggregate verdicts remain 2 Safe, 98 provisional Unsafe, 81 Unknown, four
data-only units and zero gateway failures across 185 units.

Reproduce using the [standing gate and audit commands](../cycle19-unsupported-close/README.md)
with `/tmp/cycle19-nested-unsupported` as the output directory. Validation logs,
the complete source evidence and per-entry parity are archived alongside this
report. Cycle 19 remains in progress: native caller/failure provenance, full
filesystem guarantees, general aggregate source custody, wrapper lifecycle and
full Monkey/Cycle 18 event composition remain open. No raw unit is promoted.
