# SOURCE-ANNOTATION-ENFORCEMENT

This card closes the executable-source hole identified by external review D3:
the legacy sequential reader found the initializer's `=` and parsed
`qcore_acquire(...)` without deciding any ordinary type annotation written on
the binding.  It also supplies one narrow typed-annotation operation that later
source operations can use without moving route dispatch.

The review's D1 and D3 are findings, not authority.  This implementation keeps
the existing source profiles and assigns each refusal to its owning semantic
pass.

## 1. Inventory and reproduced boundary

The executable `let` readers at base `119115de` divide into four classes.

1. `source.scalar-calls.v1` and the task route already compare supported
   `i32`/`unit` annotations with the lowered value.  Their behavior is retained.
2. `source.string.v1` deliberately admits only its checked `: string` forms.
3. The extern, buffer, socket, event, HTTP, mutex and epoch readers require an
   exact `let NAME = ...` head and already refuse an explicit annotation.
4. The legacy sequential reader's `parse_acquire` searches for `=` and ignores
   every token before it.  Both an entry-level acquisition and an acquisition
   nested in `source.region.v1` therefore accepted `: Widget` and `: i32`
   inertly.  The archived probes under `docs/reports/remaining-v03/probes/`
   record the same boundary.

The constant-binding spelling was exact and rejected every annotation at the
base.  It is the narrow positive extended here: `let n: i32 = 3;` is now checked
and admitted, with the same reference observation and emitted C as its inferred
twin.

## 2. Typed annotation operation

`qcore_frontend::source_annotation` parses only the local head
`let NAME [: TYPE] [use ...] =`.  It retains ordinary type and usage presence as
separate fields.  Its checked vocabulary is deliberately bounded to `i32`,
`unit`, and `own<NAME>`; every other explicit type is represented as
unsupported and refuses when checked.

The compatibility operation admits an omitted annotation, an exact scalar
match, or an owned annotation only when the lowering independently publishes
the same nominal payload.  An owned identity with no published nominal payload
rejects every explicit `own<NAME>` claim.

That last case is the `qcore_acquire` boundary.  The intrinsic establishes a
numeric `ResourceId` and custody obligations, but carries no declaration or
operation result that proves the identity is a `File`, `Buffer`, or any other
nominal.  Consequently `let r: own<File> = qcore_acquire(...);` is refused at
`Q020`; accepting it would infer the missing evidence from the annotation being
checked.

## 3. Diagnostics and source origin

Ordinary annotation mismatch, unsupported syntax, malformed pre-`=` tokens,
and an unprovable nominal owned annotation are `OrdinaryTypes` refusals at
`Q020`, attributed to the complete source statement.  Local `use` annotations
remain separate `Usage` refusals at `Q100`, including inside an error region.
The pipeline reader continues to own its narrower grammar: an annotated bound
pipeline refuses at `Q036` before desugaring, so no unsupported annotation can
disappear from the post-desugaring token stream.

The untyped acquisition forms are unchanged.  In particular the region
positive still produces `Acquire(1), Release(1)`, returns integer zero, and
emits the release for resource identity 1.

## 4. Rust and Lean evidence

Rust coverage lives in
`crates/qcore-verification/tests/source_annotations.rs` and the frontend
module's unit tests.  It exercises a matching scalar annotation; scalar
mismatch; unsupported and malformed syntax; `i32`, `Widget`, and `own<File>` on
direct and region acquisitions; post-desugaring-route refusal; the untyped
direct and region positives; reference/emission preservation; and the `Q100`
usage distinction.

`QCore.SourceAnnotation` models the same admission relation.  Its audited
theorems prove accepted annotations are compatible, every compatible pair is
accepted, unsupported annotations refuse, an opaque owned result refuses a
nominal annotation, and a wrong scalar annotation refuses.
`AuditSourceAnnotation.lean` registers the theorem audit, while `AuditAll.lean`
continues to inspect the complete project namespace for forbidden axioms and
vacuous statement shapes.

## 5. Standing gate

The final run is recorded under the unique worktree-private directory
`private/source-annotation-enforcement/gate-20260920-1/`; every command writes a
separate log and exit file.  `gate.json` carries only accepted generator keys,
and the Clang recording made in this checkout is committed in this report's
`gate/` directory.

| Command | Exit and result |
|---|---|
| `git submodule update --init --recursive` | 0 |
| `clang --version` | 0; Ubuntu clang 21.1.8 |
| `CARGO_BUILD_JOBS=2 cargo test --workspace --no-fail-fast` | 101; raw 4,903 passed, 1 failed, 10 ignored; 255 result summaries over 253 targets (244 `Running` plus 9 doc-test sections) |
| isolated `a_confined_fixture_cannot_write_outside_the_admitted_paths --exact --test-threads=1` | 0; 1 passed |
| `cargo fmt --all -- --check` | 0 |
| `CARGO_BUILD_JOBS=2 cargo clippy --all-targets -- -D warnings` | 0 |
| `CARGO_BUILD_JOBS=2 cargo run -p qcore-compiler --example reevaluation_probe` | 0; zero failed expectations |
| `CARGO_BUILD_JOBS=2 cargo run -p qcore-compiler --example reevaluation_followup` | 0; zero failed expectations |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0; 152 audits |
| Python suite before derived regeneration | 1; 317 ran, one expected stale-matrix failure (`151 != 152` Lean audits) |
| matrix generator with this report's `gate.json` | 0; 152 audits re-derived |
| `python3 -m unittest discover -s scripts/tests` after regeneration | 0; 317 ran, `OK` |

The full Rust run's sole red row is constraint 16's named load-sensitive
confined-write test.  It produced the rule's exact recorded signature:
`Broken pipe (os error 32)` where the assertion expected `Terminated`.  The same
row passed immediately in isolation on the same binaries.  The reconciled
published result is therefore **4,904 passed, 0 unresolved failed, 10 ignored**.
Every card-local annotation test was green in the full run.  No test, pin or
assertion was weakened.

Provider 122 is carried Unix evidence and was not remeasured: this card changes
no provider or schema path.  No corpus, contract, unit, function, CT,
sanitized-native or provider figure moves, and full language conformance
remains false.

## 6. Integrator reconciliation list

1. **Diagnostics authority.** Extend the `Q020` `OrdinaryTypes` condition in
   `docs/QCORE_SPEC_r0_3.md` and the post-migration diagnostics amendment to
   include executable local annotation compatibility, unsupported explicit
   local types, malformed local heads before `=`, and an owned annotation whose
   operation publishes no nominal payload.  Record that `Q100` still owns local
   usage annotations.  This card allocates no diagnostic code.
2. **Implemented grammar.** Record `let NAME [: i32] = INTEGER;` on the legacy
   sequential executable route and the explicit refusal of annotations on
   `qcore_acquire` until nominal result evidence exists.  Do not describe
   `own<NAME>` as an admitted acquisition annotation.
3. **Shared-file union.** Reconcile this card's `qcore-frontend/src/lib.rs` and
   `qcore-verification/src/driver.rs` hunks with `RESOURCE-HELPERS`; its route
   dispatcher additions are independent of this card's sequential lowering
   edits.  The annotation helper's `Owned { nominal: Some(...) }` arm is
   available only where that sibling independently establishes its exact
   nominal `bytes` result.
4. **Status/history/matrix.** This card moves no corpus, contract, unit,
   function, CT, sanitized-native or provider figure.  The added Lean audit
   requires derived matrix regeneration.  Root owns the current status,
   increment record and acceptance-policy edits.

## 7. Explicit residuals

* **`ACQUIRE-NOMINAL-TYPE-UNPUBLISHED`** — the legacy `qcore_acquire` operation
  has no nominal result declaration.  An explicit `own<T>` remains rejected.
  Discharge requires a checked operation/callee result that publishes `T`; the
  annotation itself cannot supply that evidence.
* **`LOCAL-ANNOTATION-SURFACE-BOUNDED`** — the shared operation represents
  `unit` and nominal ownership for routes that can establish them, but this card
  extends only the legacy integer literal bind.  Other executable routes keep
  their exact existing grammars.  Any future widening must connect its lowered
  value type to this check and add route-specific emission/reference evidence.

No acceptance item remains open for this card: wrongly typed or unsupported
explicit annotations cannot silently compile on the affected paths, the
supported scalar match passes, region nesting cannot bypass the decision, and
scalar-call/task behavior is unchanged.
