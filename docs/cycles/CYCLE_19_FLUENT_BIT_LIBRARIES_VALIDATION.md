# Cycle 19 Fluent Bit core-library validation

## Scope and source identity

This is a local verification run. It does not add, require, or modify a CI
workflow.

The adjacent Fluent Bit checkout was audited at:

```text
66910c10a4d7eafa810b84229cf2dcf7b0f26f97
v5.1.1-127-g66910c10a
```

The first-stage targets are the top-level C translation units for CMetrics and
CTraces that are present in Fluent Bit's generated
`build/compile_commands.json`. Generated nested sources and platform sources
that are absent from this build are not included in this stage.

## Gateway changes

The unrestricted-C gateway now has an explicit dialect gate for `c11`,
`gnu99`, `gnu11`, and `gnu17`. C11 remains the default. Arbitrary compiler
arguments are not accepted by the verifier.

The library audit extracts only include paths, preprocessor definitions, and
the allowlisted language dialect from each compilation-database entry. When a
command has no explicit `-std`, the audit records GNU17, matching the GNU C
default used by the local GCC 15 build. Definitions containing unresolved CMake
or Make `$` substitutions are not forwarded.

The gateway can also restrict proof scope to functions defined in the main
source file. Functions introduced by expanded headers remain available to
Clang for parsing and type checking, but their bodies do not block the selected
source-file projection. Function discovery is file-scope aware so block macros
such as `cfl_list_foreach(...) { ... }` are not mistaken for function
definitions.

Returned local allocations are modeled as ownership transfers. This closes the
simple factory case while leaving call-in-return expressions fail-closed.

## Reproduction

From the QCore-C root:

```sh
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- ../fluent-bit
```

The audit consumes the adjacent checkout and its existing compilation
database. It does not modify Fluent Bit.

## Local result

| Library | Built top-level units | Gateway failures | Source functions | Projected functions | Projection coverage | Safe units | Unsafe units | Unknown units |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| CMetrics | 31 | 0 | 540 | 287 | 53.1% | 0 | 16 | 15 |
| CTraces | 16 | 0 | 274 | 225 | 82.1% | 1 | 9 | 6 |

All 47 selected translation units pass the production Clang gateway under
their gated GNU dialect and build definitions. This resolves the previous
strict-C11 and header-expanded xxHash blockers.

These numbers are projection results, not memory-safety coverage. In
particular, an `Unsafe` result at this stage is not evidence of an upstream
defect. The current resource projection does not yet express owned parameters,
aggregate/container custody, arena ownership, or `realloc`, so it can report a
local obligation as leaked when ownership actually moves into one of those
structures. `ctr_version.c`, which is resource-neutral, is the single fully
`Safe` CTraces unit.

Six CMetrics units and one CTraces unit still stop before a partial resource
summary because the reduced statement parser cannot form a body for at least
one selected function. The remaining `Unknown` units reach a partial summary
and fail closed on unsupported operations.

## Next resource-model gates

The next stage should be implemented and validated in this order:

1. retain per-function partial results when a sibling function is malformed;
2. import `for`, `switch`, `goto` cleanup, labels, `break`, and `continue` into
   the CFG rather than consuming them as unknown blocks;
3. distinguish owned, borrowed, consumed, and out-parameter function
   contracts;
4. add aggregate fields and intrusive-list custody, including destructor
   summaries;
5. add nullable acquisition and path refinement;
6. model `realloc` as preserve-on-failure and replace-on-success;
7. add CFL arena parent/child lifetime discharge; and
8. rerun the same pinned local audit before expanding to Fluent Bit `src`.

Only functions whose resource obligations are fully discharged may contribute
to memory-safety coverage. `Unknown` and provisional `Unsafe` functions remain
outside the proved numerator.
