# Cycle 12: End-to-End QCore-C Compiler and CLI

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 12 connects source files to the Cycle 11 whole-program checker and the
cleanup-explicit C11 backend. The compiler accepts a deliberately conservative
executable subset and reports every unsupported source form rather than
silently lowering it.

## Rust implementation

`compiler/src/driver.rs` provides the public `compile_files` entry point:

- combines multiple named source files into one checked module;
- maps parse, elaboration, checking, lowering, and emission failures back to
  file, line, column, and source text;
- selects `main`, a requested entry, or the sole function;
- synthesizes and checks usage, ownership, resource-flow, typed-error,
  error-region, and cleanup-lowering inputs from executable source;
- lowers integer returns, `qcore_acquire`, `qcore_check`, `drop`, and `move`;
- checks cross-file zero-argument calls and emits them only when omitting the
  ignored call result is observationally pure in the reduced CFG model;
- rejects structured control flow, recursive execution, observable calls, and
  other backend-unsupported forms with stable diagnostics;
- retains the checked program and reference observation with emitted C; and
- emits both a machine-readable source map and a C `#line` directive.

The `qcorec` binary is the stable command-line boundary:

```text
qcorec check [--entry NAME] FILE...
qcorec emit [--entry NAME] [-o FILE] FILE...
```

Without `-o`, `emit` writes C11 to standard output. `check` reports the checked
function count and selected entry.

## Executable source subset

The Cycle 12 backend accepts flat function bodies containing:

```qcore
error Failure { Failed; }
fn main() -> i32 {
  let handle = qcore_acquire(7, true, Failure);
  qcore_check(true, Failure);
  move handle;
  return 42;
}
```

`qcore_acquire(identity, succeeds, ErrorType)` creates one tracked identity,
`qcore_check(passes, ErrorType)` creates a cleanup edge, `drop` destroys an
identity, and `move` transfers it to the result payload. Resource identities
must be unique, all normal exits must discharge or transfer live ownership,
and resource peak/cumulative bounds are synthesized and checked.

## Lean verification

`proof/QCore/Compiler.lean` models module assembly, checked entry selection,
CFG retention, C11 emission, and emitted-source mapping. It proves that:

1. compilation preserves the checked source module;
2. compilation implies Cycle 11 whole-program acceptance;
3. the selected function has the requested symbol;
4. the compiler retains that function's checked CFG;
5. successful emission retains the requested source origin; and
6. target and reference observations agree at the checked CFG boundary.

`proof/AuditCycle12.lean` audits every Cycle 12 theorem. No proof contains a
placeholder or project-defined axiom.

## Verification

From the repository root:

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle12.lean
```

Cycle 12 adds Rust tests for multi-file handling, stable source diagnostics,
source mapping, quantitative bounds, and source/reference/native-C
differential observations for values, errors, cleanup traces, and ownership
transfers.

Cycle 13 is the next implementation milestone.
