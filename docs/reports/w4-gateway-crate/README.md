# W4 gateway crate relocation

Date: 2026-09-07. The existing-C gateway now lives in `qcore-c-gateway` (L2),
completing W4's crate relocation after its [bounded custody extension](../w4-custody-extensions/README.md).
This change moves implementation and test ownership without changing analysis.
Unsupported custody modes and recursive summary inference remain open.

## Boundary and compatibility

`legacy_c`, `legacy_summary`, `clang` and `typed_c_ast` moved from the compiler
into `crates/qcore-c-gateway/src`. The analyzer and summary source files are
byte-identical before and after the move, as are their focused test modules.
Seven `.qcore` test fixtures moved with those tests; two compiler consumers now
include the same fixtures from their new location.

The new crate depends only on `qcore-contract`, `qcore-schemas` and `serde_json`.
It has no compiler, provider, core or frontend dependency of its own. The workspace
dependency-matrix guard checks this boundary and permits the compiler's downward
dependency on the gateway.

The existing `qcore_compiler::legacy_c` module path and compiler-root API remain
compatibility reexports of the gateway's public `legacy_c` types and functions.
Compiler integration/process tests continue to use that public compatibility API.
Custody inference internals, entry-state constructors and observation validators
remain private to the gateway crate.

Two small transport surfaces cross the new crate boundary:

- `clang_command()` builds a command using the existing checked Clang discovery
  policy. Invalid or empty `QCORE_CLANG` overrides still fail without fallback.
- `typed_c_ast::{ast_declarations, function_ast, kind, children}` supplies the
  compiler's source-profile interpreters with raw JSON. Public documentation
  states that these helpers expect gateway-validated input and provide no safety
  evidence on their own. Function bodies and fallback behavior are unchanged.

These are the only newly public helpers; summary internals were not widened to
make the move compile. [relocation.json](relocation.json) records exact-move
hashes and the narrowly scoped visibility/documentation changes.

## Verification and parity

All standing checks passed:

```sh
git -c protocol.file.allow=always submodule update --init --recursive
cargo test --workspace                       # 453 tests passed; 87 in gateway
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe
cargo run -p qcore-compiler --example reevaluation_followup
(cd proof && PATH=/home/cosmo/.elan/bin:$PATH lake build)
```

No Lean definitions or theorems changed; the complete 52-job proof build passed.
The workspace count is unchanged by relocation: 87 tests moved from compiler
ownership to the gateway.

The full pinned audit was repeated with the forced-Libevent Monkey database:

```sh
cargo run --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/gateway-source.json /tmp/gateway-diagnostics.json
```

The full 185-unit resource report is compared against the W4 extension report,
including all individual diagnostics and function statuses. There are **zero
changes**: **3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only**; source
function sets and projection coverage are identical. The 29 Monkey units are
included. The [parity record](parity.json) binds both complete report hashes;
[audit.log](audit.log) preserves the fresh console run.

Cycle 19 raw units remain **unpromoted**. The compiler's pinned orchestration
and provider routes are retained; this extraction removes no analysis route.
