# r0.3 source result, axis and signed-literal validation

Review findings R04, R05 and R07 were reproduced on main `5d0fb6d` before
repair. The source-only review is supplied in `unverified_specs/REVIEW.md`;
this package records fresh Rust/Lean execution rather than importing the
review archive's historical completion claims.

## Implemented boundary

- **R04, bounded repair:** function result syntax, token structure and spans
  survive structural parsing and elaboration. Every body admitted by the
  source driver must declare exactly `i32` or `unit`. Integer results require
  an in-range literal return; unit results require a bare return. Bool,
  unknown names, aliases, owned results and `result<...>` ABIs are rejected
  by this executable consumer. Empty annotations are parse errors. Existing
  missing-return, post-return operation and unsupported-expression gates
  continue to reject inconsistent exits and return-of-call expressions.
- `CompiledModule::result_profile` records the scalar type for every checked
  body and `CustodyPayloadProfile::SeparateIdentityLedger`. The scalar slot
  does **not** type the separate resource payload produced by Cycle 12 `move`.
  Existing ownership/cleanup identity checks and CT02/CT19/CT37 evidence are
  preserved. A callee that returns custody cannot silently emit through the
  reduced call path; the test fixture is rejected by the earlier conservative
  resource bound gate (Q200), with emission's Q500 gate still retained.
  Full owned-result, ordinary alias/type resolution, and general per-exit
  payload typing remain open. Structural contract declarations may retain
  unsupported result annotations; structural acceptance is not typed ABI
  acceptance.
- **R05:** dimension and axis declarations are checked unconditionally,
  before quantitative-type resolution. Unknown dimensions, duplicate axes,
  duplicate dimensions and malformed axis kinds fail with and without later
  quantitative types. Nominal `Count<T>` does not require a runtime type
  declaration; forward declared ordinary dimensions remain valid.
- **R07:** the driver and observation interpreter parse the final negative
  decimal spelling into the signed target type. They never parse the minimum's
  magnitude as positive signed data or apply unchecked negation. Nonliteral
  predicate negation still uses checked arithmetic. Both extrema, zero,
  negative zero, neighboring out-of-range values, and minimum-valued facts
  are exercised. Native emitted C agrees with the reference for i32 extrema
  and unit. Observation arithmetic remains the existing bounded subset;
  division/modulo are unsupported and fail closed.

The W3 fixture used `i64` while expecting source compilation success; it now
uses supported `i32`. Its witness/obligation mutation coverage is unchanged.
No existing-C analyzer, provider verdict rule or pinned source route changed.
No CT case was added, removed or remapped.

## Reproduction and validation

`before-driver.log` records actual acceptance of `fn main() -> bool { return
2; }` and rejection of `-2147483648`. `before-declarations.log` records actual
acceptance of an unused `credit<Missing>` axis and rejection of the i128
minimum. These logs come from regression tests added before the repair.

The standing Rust gate uses `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, and two build jobs
for tests/Clippy to fit the shared disk quota. An initial build with full
debug information exhausted the quota during linking; only this agent's old
completed audit build cache was removed before a clean rebuild. This changes
artifact size, not tested source semantics. All 521 workspace tests, formatting, Clippy, both probes, Lean (55 jobs)
and the new axiom audit passed. Gate logs and machine-readable results are
archived alongside this report. The pinned Libevent submodule
was initialized at `d82464a277d0f42703702c4dfd9af6af38595a83`.

`proof/QCore/SourceValidation.lean` models the scalar result shape gate,
signed range checks, validation independent of type uses, and preservation
of the separate custody input. `proof/AuditSourceValidation.lean` prints the
axiom surfaces: no project axioms or proof placeholders. These are reduced
model theorems, not a parser/Rust/C refinement or proof of owned-result types.
