# QTT alignment: usage checking and lambda introduction

Implemented on 2026-09-05 following a comparison with Gabriel Barrett's
[QTT gist](https://gist.github.com/gabriel-barrett/11988a272bc52f0e848db6eceb03417a).

## Semantic decision

QCore retains interval grades. `one = [1,1]` requires consumption;
`affine = [0,1]` permits omission. The gist's final affine interpretation of
`1` corresponds to `affine`, not `one`. Its three-element algebra is not a
literal replacement for QCore's interval arithmetic or containment order.

The corrected introduction principle is implemented in the reduced usage IR:
check a lambda body for one invocation, validate its bound parameter against
the declared grade, remove the bound variable, and only then scale the remaining
capture demands by the enclosing quantity. No division operation is required
for this demand-synthesis implementation. This does not assert an equivalence
between all interval rules and the gist's affine context-division calculus.

`UsageExpr::Lambda` implements this check; `synthesize_demands_at` scales a
successfully checked expression. Application also synthesizes arguments before
scaling them. Neither unrestricted nor zero surrounding demand can hide an
invalid binder. Name shadowing is lexical. Replicating a closure scales its free
captures, so an affine capture cannot supply unrestricted demand.

## Source enforcement

Concrete parameter grades now survive parsing and elaboration, including named
grades, natural literals, and literal intervals with finite or `inf` upper
bounds. Malformed, overflowing, and unsupported symbolic grades are rejected.
Annotations retain source spans. The whole-program API rejects reduced usage
declarations that omit or contradict an explicit source parameter annotation.

The executable driver now constructs usage declarations and demand expressions
instead of an empty map and `Unit`. It checks every function, including uncalled
functions. This previously accepted program now fails with `Q100`, identifying
`x`'s required `[1,1]` and actual `[0,0]` demand:

```text
fn discard(x: i32 use one) -> i32 { return 0; }
fn main() -> i32 { return 0; }
```

The driver records supported explicit local-resource and direct-function uses.
Locals use distinct internal names, so a shadowing local cannot discharge a
parameter's contract. Known failing acquisition/check operations stop later
runtime demands. Call continuations conservatively admit zero demand because
no normal-exit guarantee is available during synthesis.

Unannotated parameters and local demand entries use `many`; resource ownership
and implicit cleanup obligations are still checked separately. Local `use`
annotations are rejected in the executable subset. Indirect calls are also
rejected rather than resolving a shadowed local as a global function.

The executable source subset still accepts only zero-argument direct calls and
literal returns. It does not gain source lambdas, general parameter evaluation,
or closure/C lowering from this change. The lambda feature is a checked IR API.

## Lean verification

`proof/QCore/UsageDemand.lean`, imported by the normal `lake build`, adds an
executable expression-demand checker and an inductive demand-realization model.
Sequence and multi-argument application vectors are modeled by binary
composition. The new proofs establish:

- `synthesize_covers`: every modeled realization lies within the inferred
  intervals, including branch choice and admitted argument multiplicities;
- `lambda_parameter_covers`: an accepted lambda obeys its binder contract per
  invocation, for any enclosing quantity;
- `scaled_captures_cover`: admitted replication counts are covered by scaled
  capture intervals;
- `checked_parameters_cover`: checked source parameter contracts cover the
  modeled counts;
- rejection of an unused `one` parameter and the gist's duplicated-affine-
  parameter example at every enclosing quantity.

Demand realizations model resource demand/reservation, not operational
evaluation of lambda terms. Application grades remain supplied contracts in
both reduced cores. These proofs do not establish ordinary/dependent typing,
beta preservation, general substitution of typed terms, runtime erasure, or
mechanized correspondence between Rust parsing/lowering and Lean. Rust's finite
arithmetic rejects overflow; Lean uses unbounded naturals. The historical
`application_substitution` unfolding theorem remains intact for its existing
callers and is not promoted into a term-substitution theorem.

## Validation

- `cargo test --workspace -q`: 242 tests passed (222 unit, 20 integration).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cd proof && lake build`: passed.
- Axiom inspection of all six new principal theorems: only `propext` and
  `Quot.sound`; no `sorryAx` or custom axioms.

Regressions cover the original CLI gap, source contract substitution, concrete
grade parsing, malformed annotations, shadowing, capture replication, zero
scaling, branch obligations, and conservative call continuations.
