# Cycle 19: filesystem cleanup result-use evidence

The pinned audit now retains complete typed paths and executable result-use
policies for four filesystem cleanup functions. The policies live in
`contract/qcore/chunkio/filesystem_attempts.qcore`; they describe source result
handling, not OS success or resource discharge.

| Source function | Paths | Selected cleanup calls across paths | Ignored | Used in guards |
| --- | ---: | ---: | ---: | ---: |
| `cio_file_close` | 4 | 6 | 6 | 0 |
| `munmap_file` | 8 | 6 | 0 | 6 |
| `cio_file_native_close` | 4 | 2 | 0 | 2 |
| `cio_file_native_unmap` | 4 | 2 | 0 | 2 |

Counts are per symbolic source path, not distinct runtime executions. On each
nonnull `cio_file_close` path, `munmap_file` precedes `cio_file_native_close`,
then two free calls follow; neither cleanup result is used. Guard presence in
the lower helpers does not prove the guard correct, establish an OS outcome,
or prove that the outer caller discharges a resource on an error path. The
native functions return errors and preserve their mapping/descriptor fields
on their corresponding OS failure branches; interpreting those outcomes and
caller continuation requires further custody composition.

`source_call_use.rs` is a shared producer-use analysis. It visits every flat
path operation and records source-symbol dependencies in reads, writes, local
assignments, call arguments, returns and guards. The typed importer has already
substituted local aliases. A result stored into a local is conservatively a use,
even if never read afterward. Guard uses refer to explicit symbolic dependency;
heap reloads are not assumed to retain an earlier stored result identity.
Structured loops reject this flat analysis. These facts cannot turn an ignored
result into an assumed success.

The ChunkIO consumer checks every path against four native policy functions,
including nonvacuous presence of cleanup attempts, source ABI shape, ignored
versus guarded results, paired outer cleanup calls, following free-call count
and order. It records the complete source projections, all producer/use indices,
exact policy text and explicit `OS success/discharge ... Unknown` status under
`backend_teardown`, `kind: filesystem_call_results`. No filesystem or plugin
safety certificate is issued, and these policies remain outside the simple
resource-contract manifest.

## Validation and limits

All **361 workspace tests** and the 30-module native corpus pass. Tests cover
alias propagation, passing/returning/guarding versus discarding results, flat
analysis rejecting a structured loop, changed outer result handling, missing
cleanup/free calls, and every policy postcondition changed to false or an
unsupported predicate. Formatting, warnings-denied Clippy, audit example tests,
both reevaluation probes and the pinned audit pass.

Three new Lean theorems prove that an ignored producer has no recorded use,
a guarded producer is used, and ignored result observations cannot distinguish
success from failure. `lake build` and the Cycle 19 dependency audit pass without
placeholders or project axioms. Clang, the Rust importer and producer-use walk
remain trusted boundaries; the proof is not arbitrary C or OS semantics.

The generic inventory is byte-identical to the memory-close baseline. The full
Monkey sidecar, 96 scalar results and every backend/counter/unlink/memory-close
certificate remain identical. Aggregate totals remain 185 units, 4,688/4,775
projections, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures. Neither projected paths nor provisional Unsafe classifications
are proof coverage or confirmed upstream defects. Native observation code was
unchanged; no additional runtime failure execution is claimed.

Evidence: `parity.json`, `chunk-contracts.json.gz`, validation logs and `evidence.json`.

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo run --quiet -p qcore-compiler --example reevaluation_probe
cargo run --quiet -p qcore-compiler --example reevaluation_followup
(cd proof && lake build && lake env lean AuditCycle19.lean)
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-file-attempts-source.json /tmp/cycle19-file-attempts-diagnostics.json
```

Cycle 19 remains in progress. Filesystem source/OS outcome binding and failure
custody, stream/context traversal, constructor identity/provenance, remaining
wrapper lifecycle and full Monkey event composition remain open. The checked
memory close result continues to cover only its null and memory-backed paths.
