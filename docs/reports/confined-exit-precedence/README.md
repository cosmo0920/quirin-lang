# Confined process failure precedence

The write-confinement fixture can exit with code 6 before the host finishes
writing its request. The transport retained `Io(Broken pipe)` and returned it
before checking the observed unsuccessful exit, making its published outcome
scheduler-dependent. The final, fully drained completion branch now checks the
unsuccessful exit first and retains stderr detail. Successful exits with
incomplete input still refuse; deadlines and output limits retain precedence.
No confinement expectation, timeout or response-size control was weakened.

The existing standalone transport regression now forces incomplete input with
an 8 MiB request and covers exit 6, exit 6 with stderr, and SIGTERM. Its original
partial-request, descendant-timeout, live-child and complete-response controls
remain. Before the fix this regression failed with the user's exact
`Io` versus `Terminated` mismatch (exit 101, 0 passed / 1 failed).

`QCore.VerifierPlugin.ProcessCompletion` models the final completion decision;
three theorems registered in `AuditCycle20.lean` prove failed-exit precedence,
partial-request refusal, and successful/complete input as prerequisites for
response decoding. This is a reviewed model correspondence, not an OS pipe or
scheduler proof. Native observations here are Linux only. No Safe/Unsafe/Unknown
admission rule, package schema, contract or coverage figure changes.

The historical constraint-16 broken-pipe observation is preserved. This repair
identifies and fixes that classification mechanism; it does not discharge the
separate timing, readiness fallback, pipe-capacity or macOS follow-ons in
`verifier-host-control-budget`, nor retire any acceptance-matrix entry.

## Validation

Private logs: `private/confined-exit-fix/` in this checkout. The first restricted
focused run failed with 3 passed / 9 failed because the runner prohibits user/network
namespace creation (exit 101); it did not reach the standalone target. Outside
that restriction, the same command passed all 12 confinement and 3 standalone
rows (exit 0). The initial Lean run failed on a documentation-comment placement;
the corrected run passed all 165 audits (exit 0). A package verification command
mistakenly named nonexistent `qcore.libc` (exit 1); `qcore.chunkio` is the actual
fifth package and its verification passed (exit 0). All attempts are retained.

Final commands and measured exits (log filenames below are relative to the private directory):

| Command | Exit | Result / log |
|---|---:|---|
| `cargo test --workspace --no-fail-fast` | 0 | (5043, 0, 11) passed / failed / ignored; `workspace.log` |
| `cargo fmt --all -- --check` | 0 | pass; `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | pass; `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | pass; `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | pass; `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | 165 audits; `lean-confirmation.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | 336 tests; `python.log` |
| `cargo test -p qcore-verifier-host --test standalone_host --test confined_process_execution` | 0 | 15 passed; `focused-unrestricted.log` |
| `cargo test -p qcore-verifier-host --test confined_process_execution a_confined_fixture_cannot_write_outside_the_admitted_paths -- --exact` | 0 | 1 passed, 11 filtered out; `isolated-write.log` |
| `python3 providers/package.py build --output private/confined-exit-fix/packages` | 0 | 5 packages; `provider-build.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | (123, 0, 0) passed / failed / ignored; `provider-tests.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | pass; `provider-fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | pass; `provider-clippy.log` |
| `clang --version` | 0 | measured in this checkout; `clang-version.log` |
| `python3 providers/package.py verify private/confined-exit-fix/packages/qcore.libevent` | 0 | `verify-libevent.log` |
| `python3 providers/package.py verify private/confined-exit-fix/packages/qcore.chunkio` | 0 | `verify-chunkio.log` |
| `python3 providers/package.py verify private/confined-exit-fix/packages/qcore.cmetrics` | 0 | `verify-cmetrics.log` |
| `python3 providers/package.py verify private/confined-exit-fix/packages/qcore.ctraces` | 0 | `verify-ctraces.log` |
| `python3 providers/package.py verify private/confined-exit-fix/packages/qcore.librdkafka` | 0 | `verify-librdkafka.log` |

This was the existing checkout, not a fresh checkout; no submodule initialization was needed. The full workspace gate includes the original confinement assertions unchanged. No validation attempt remains incomplete.
