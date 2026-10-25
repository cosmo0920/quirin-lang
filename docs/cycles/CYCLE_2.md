# Cycle 2: Usage Grades

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 2 implements Stage 2 of the prototype roadmap. A cycle is complete only
when its executable Rust behavior and corresponding Lean model both pass their
builds.

## Executable semantics

The Rust core must provide:

- interval usage grades `[l,u]`, including an infinite upper bound;
- addition, multiplication, precision containment, and least-enclosing joins;
- the named grades `zero`, `one`, `affine`, `relevant`, and `many`;
- pointwise variable-demand synthesis for sequence and conditional expressions;
- usage substitution for function application,
  `gamma_f + sum(u_i * gamma_i)`;
- declaration checking that rejects an exact-one value when any path drops it or
  any path duplicates it;
- affine checking that permits zero or one demand but rejects duplication;
- rejection of runtime occurrences of erased (`zero`) variables.

Usage arithmetic overflow and application arity mismatch must be explicit
errors. A missing variable at a branch is treated as demand `zero`; therefore a
one-sided use joins to `affine` rather than `one`.

## Lean verification obligations

The Lean core must use the same interval interpretation and prove:

1. named usage grades are valid intervals;
2. addition and multiplication preserve interval validity;
3. each branch demand is contained by the branch join, and that join refines
   every other common enclosing interval;
4. a valid demand accepted by `one` is exactly `[1,1]`;
5. a valid runtime demand accepted by `zero` is exactly `[0,0]`;
6. application demand unfolds to the substitution law
   `gamma_f + u * gamma_a` pointwise.

These obligations are checked by `lake build`; theorem statements containing
`sorry` do not satisfy the cycle.

## Acceptance commands

```sh
cargo test --workspace
cd proof && lake build
```
