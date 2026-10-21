# Gateway and plugin test portability (2026-10-22)

The reported macOS failures came from Linux measurement and libc fixture
assumptions in tests. Production analysis, transport bounds, contracts and Lean
semantics are unchanged.

- `deep_ast_transports` keeps both admitted/beyond-bound reader controls active
  on every host with Clang. Stack high-water observations and their 1 MiB budget
  remain mandatory on Linux; unreadable pagemap still fails there. Other hosts
  return `None` and explicitly report unavailable measurement. Only the
  measurement calibration test is ignored outside Linux. Passing behavioral
  controls on macOS is not a macOS stack-budget measurement.
- `declared_entry_check` declares its modelled `__assert_fail` entry explicitly.
  `deep_unit_ast` spells out the GNU statement-expression assertion fixture,
  following the existing CG21 fixture, instead of importing the host's
  `<assert.h>`. The controls now exercise the same source form on every host;
  the old Windows exclusions are removed.
- CG21's system-header `setjmp` control accepts the complete named refusal for
  either `setjmp` (Darwin) or `_setjmp` (glibc) and explicitly requires `Unknown`.
  The non-local-exit refusal remains unchanged.

- The subcommand timeout fixture consumes its input with the existing shell
  builtin helper rather than launching `cat` before creating its child PID file;
  the 500 ms timeout and 20 s outer bound are unchanged. Scratch directories
  include the test-process PID so concurrent runs cannot delete each other's
  files. Child-state checking uses `ps` on Unix instead of silently treating
  missing `/proc` on macOS as proof of death. A missing PID or an inspection
  failure still fails the test; only absent or zombie children satisfy it.

The attached checkpoint failure is a separate executable-identity invalidation:
decoding its two `.qo` byte arrays shows `identity.run.analyzer.binary_sha256`
changed from `e4d7d0095f2f5cbb8018f37d6b29802aa083323b520d18b331ce09b5848c0ce2`
to `9d7c1f5950fe6dca3a60c16f895e660f766663ecdff9b778790e35bf8429a524`.
The remaining decoded payload differences are timing/job provenance and the
corresponding section/content hashes. Reuse must refuse a changed executable;
no identity check was weakened. Avoid concurrent rebuilds of the same target
while running checkpoint tests. The second attached failure has no checkpoint
payload pair, so the same cause is not established for that row from its log.

Fresh macOS focused checks (all exit 0):

- `cargo test -p qcore-c-gateway --lib deep_ast_transports::tests -- --nocapture`:
  4 passed, 0 failed, 2 ignored (Linux calibration and external-data measurement).
- `cargo test -p qcore-c-gateway --test declared_entry_check --test deep_unit_ast --test cg21_call_exits --no-fail-fast`:
  22 passed, 0 failed, 0 ignored across three binaries.

Final validation receipts are in
`private/pagemap-portability-20261022-unsandboxed/`; the full Rust runner uses
`private/test-binaries/20261021T163032Z-79574/`. The gate results and remaining failures are recorded below. The checkout already carried unrelated audit/provider changes,
which this repair preserves.

The initial sandboxed build was interrupted (recorded exit 143) after executable
startup stalls, with no suite total; logs remain in
`private/pagemap-portability-20261022/` and
`private/test-binaries/20261021T161933Z-45962/`. A one-second sample placed the
checkpoint decoder at `_dyld_start`; rerunning it outside the sandbox completed
with exit 0. The initial focused transport run passed 4/0/2 before the fixture
changes; separate fixture and call-exit runs also subsequently passed. An initial
`rustfmt --check` found formatting differences (exit 1), then the four changed
Rust files were formatted. Linux stack observations were not remeasured on this
macOS host.

The timeout failure was reported while the full suite was in its parallel phase.
Its test binary was rebuilt separately before the runner's isolated
`subcommand_plugins` phase; this test-only rebuild does not change the analyzer
binary. The initial full build therefore predates that last fixture edit;
`plugin-build.log` records the supplemental build.

## Final verification and follow-ups

The first complete gate exercised the revised timeout fixture but still failed
that row: its child PID was absent with sibling plugin fixtures running. The
same row passed alone (1 passed, exit 0, `plugin-isolated-before.log`). The final
repair serializes the 14 tests within this timing-sensitive binary, complementing
the runner's existing isolation from other binaries. Both the 500 ms quota and
20 s outer assertion remain unchanged. The complete repaired binary passes
14/14, exit 0 (`plugin-final.log`); the earlier failure is retained rather than
reported as a green full run. The checkpoint suite passed all 8 tests, including
both attached failures, without a code change.

Commands below ran in this macOS checkout with Apple Clang 21.0.0
(`clang --version`, exit 0, `clang-version.log`). The existing checkout was used,
so the fresh-checkout-only submodule initialization step was not applicable.
Each gate step has its own `.log` and `.exit` under
`private/pagemap-portability-20261022-unsandboxed/`.

| Command | Exit | Result / log stem |
|---|---:|---|
| `python3 scripts/run_test_binaries.py` | 101 | 535 units, 548 summaries; 8,383 passed, 1 failed, 15 ignored; `rust` |
| `cargo test -p qcore-audit --test subcommand_plugins` (after final repair) | 0 | 14 passed, 0 failed; `plugin-final` |
| `cargo fmt --all -- --check` | 0 | `fmt`; repeated after the final fixture edit as `fmt-final` |
| `cargo clippy --all-targets -- -D warnings` | 101 | Existing `drop(quota)` on a `Copy` value at `crates/qcore-verifier-host/src/lib.rs:1562`; `clippy` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `probe` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `followup` |
| `python3 scripts/check_lean_audits.py` | 0 | 360 audits passed; `lean` |
| `python3 -m unittest discover -s scripts/tests` | 1 | 374 tests: 353 passed, 15 failed, 6 skipped; `python` |

The full 535-unit suite was not rerun after serializing only the plugin tests;
the complete affected binary was rerun instead. Thus this report does not claim
a green standing gate. Remaining findings outside the reported Rust failures:

- Clippy's pre-existing `dropping_copy_types` error at the verifier-host site
  above. Resolve the intentional quota lifetime on macOS, then rerun Clippy.
- Twelve reconciliation-sweep Python failures compare resolved paths against an
  unresolved macOS temporary root and report that fixture files escape the root.
- Two test-runner Python environment controls reject Darwin's injected
  `__CF_USER_TEXT_ENCODING` variable.
- One census Python control expects `/var/...` include paths while the command
  uses their canonical `/private/var/...` forms.

These Python failures are in `test_reconciliation_sweep`,
`test_run_test_binaries` and `test_w42_unprojected_census`; the private log names
every assertion. They need separate portability fixes and a fresh Python gate.
The separately committed per-library provider changes were already present when
this validation began and are included in these measured Rust/Lean results.
