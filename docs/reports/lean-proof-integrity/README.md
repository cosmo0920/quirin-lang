# Lean proof integrity and Lean 4.34 warning cleanup

2026-09-16. Reviewed base: `aeef9bbc352880085b7651552e2d9786f5bb4983`.
This is proof and validation maintenance: no Rust implementation, model
operation, checker verdict, corpus pin or conformance stage changes.
Historical reports and their logs remain frozen; the statement corrections
below supersede only their claims about the named theorems.

## Findings and repairs

The base passed all 128 axiom audits. That establishes an allowed axiom
closure, not that every statement provides independent safety evidence.

- `BoundFieldRead.an_exposed_base_never_admits_a_custody_read` assumed its
  exact conclusion. It now derives refusal from `admitsCustodyRead` with no
  such premise. Its existing `premise_f_is_required` sibling was already
  independent; the repair does not widen the gateway or change premise F.
- `TypedErrors.ExitMap.validated_catch_join_has_one_normal_ledger` assumed
  `NormalLedger` of the completed catch. It now derives the result from normal
  incoming paths agreeing on the baseline and matching error paths having no
  local obligations beyond that baseline. A biconditional establishes that
  these conditions are necessary as well as sufficient. A nonempty admission
  and two counterexamples exercise the ordinary and recovered path premises
  separately. These concern the existing cleanup-free catch model.
- `StringConcat.a_view_carries_no_custody` reconstructed a record from its own
  projections, which elaborates to self-equality. Its legitimate purpose was
  a representation check, not an ownership proof. The replacement,
  `views_equal_iff_components_equal`, compares two independently quantified
  views and proves that equality of the three components is both necessary
  and sufficient. A new independent field would invalidate sufficiency.

The following ten literal self-equality statements were removed, together
with their print-audit entries. They had no proof callers. Determinism or
absence of state inputs is a property of these pure definitions, not an
independent proof of Rust implementation correctness or certificate replay.
Other substantive theorems and their audits remain in place.

| Module | Removed theorem(s) |
|---|---|
| `NativeLibrary` | `the_binding_is_a_function`, `admission_is_deterministic` |
| `NativeLibraryContract` | `the_binding_is_a_function`, `admission_is_deterministic` |
| `NativeLibraryHeader` | `the_binding_is_a_function` |
| `NativeLibrarySourceBinding` | `resolution_is_a_function` |
| `UncheckedLinkage` | `admission_is_deterministic` |
| `BranchJoin` | `evaluation_is_state_free` |
| `Bitwise` | `evaluating_an_operand_reads_no_state` |
| `FrameHelperTask` | `derivation_is_a_function` |

Also removed `Bitwise.an_operand_has_exactly_one_value`: its statement merely
existentially named the evaluator's output, without expressing uniqueness.
The integer/condition vocabulary restrictions are now documented directly;
`the_fold_preserves_the_value`, the branch interpretation theorems and the
other computational properties remain audited. The removed Lean names are
intentional proof-API changes; no tautological compatibility aliases remain.

Relevant historical evidence: [bound custody reads](../remaining-o04-b09b10/README.md),
[branching](../wave19-if-statement/README.md),
[bitwise conditions](../wave20-bitwise-conditions/README.md),
[native artifact binding](../wave19-unsafe-linkage-artifact/README.md),
[contract slices](../wave20-native-library-contract-slice/README.md),
[header binding](../wave22-native-library-header/README.md),
[source binding](../native-library-source-binding/README.md),
[frame task identity](../remaining-p04-frame/README.md), and
[string composition](../string-concat/README.md).

## Regression boundary

`AuditAll.lean` retains its complete axiom-closure check over public, private
and generated QCore declarations, allowing only `propext`, `Classical.choice`
and `Quot.sound`. It additionally checks theorem declarations with public,
non-internal names for two patterns in their elaborated types:

1. The conclusion equates an expression with that exact same expression.
2. A parameter has exactly the conclusion's type.

Internal names are excluded only from the statement check: generated
`DecidableEq` helpers legitimately contain reflexivity. Ordinary proofs using
`rfl`, `simp`, `decide`, contradiction, or definition unfolding are not banned.
The check does not unfold definitions to decide whether a premise is the
conclusion. The original `TypedErrors` defect needed semantic review; its new
proof callers and premise counterexamples now also guard its repaired type.
Reintroducing its original assumed-result statement in an isolated copy fails
those callers with type mismatches (exit 1).

Permanent detector controls in `AuditAll` require both bad statement forms
to be recognized and allow a computational identity and conjunction
elimination. Additional injected public theorems were rejected by the actual
whole-project audit with exit 1 and the intended diagnostic. No axiom or
placeholder was needed for those controls, illustrating why the axiom audit
alone could not detect these defects.

**Remaining boundary — `LEAN-STATEMENT-SEMANTIC-REVIEW`:** this narrow check is
not an exhaustive review of contradictory premises, uninhabited domains,
weakened specifications, definitional restatements or Rust/Lean correspondence.
Discharge requires a theorem-by-theorem review against intended claims, with
inhabited controls or counterexamples where applicable. No such blanket
semantic-completeness claim is made here.

## Lean 4.34 warnings

The supplied log ran Lean 4.34.0 even though the repository was pinned to
4.33.1: invoking `lake -d proof ...` from the repository root lets elan select
the caller's default before Lake applies `-d`. The pin now selects 4.34.0 and
144 deprecated lemma references in 33 modules use its replacement names:

| Old | Replacement |
|---|---|
| `if_pos` | `ite_eq_left` |
| `if_neg` | `ite_eq_right` |
| `if_true` | `ite_true` |
| `if_false` | `ite_false` |

`proof/lakefile.toml` now sets `warningAsError = true` for the QCore library,
so build warnings fail before print audits. A minimal library using the exact
same Lake configuration rejects a deprecated lemma (exit 1) and builds with
its replacement (exit 0). No warning suppression was added. The pinned-toolchain
invocation remains `(cd proof && lake build)` or the audit runner.

## Validation

Full results are recorded in `validation.json`. Raw logs and injected control
files stay in the ignored `private/lean-integrity-review/` directory.

- Lean 4.34 library build and all 128 audits: exit 0, no warnings.
- Axiom closure: 42,938 declarations, only the three allowed logical axioms.
- Statement check: 8,884 public theorems with non-internal names. This includes
  generated public equations, so it is not a source-written-theorem count.
- Two injected bad statements: exit 1, each with its intended diagnostic.
- Strict-warning negative/admitting controls: exit 1 / exit 0.
- Exact user command `lake -d proof lean proof/QCore.lean`: exit 0, empty output.
- Workspace: 4,544 passed / 0 failed / 8 ignored,
  216 test executables and 11 doctest result summaries; command exit 0.
  The 227 `test result:` lines explain the older ledger's "227 test binaries"
  label; this run distinguishes executable launches from result summaries.
- Python suite: 227 passed. Formatting, Clippy and both reevaluation examples:
  exit 0.

The initial sandboxed workspace gate encountered LeakSanitizer's ptrace
restriction and was interrupted. The complete workspace gate passed outside
the sandbox with every original assertion and sanitizer enabled. Submodule
initialization likewise needed writable Git metadata; the pinned libevent
revision was retained. Neither environment failure is treated as a proof or
checker defect.
