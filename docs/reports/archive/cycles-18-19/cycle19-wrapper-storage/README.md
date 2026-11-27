# Cycle 19: wrapper constants and stack storage

Date: 2026-09-06. Status: bounded 19D source-import increment complete;
Cycle 19 remains in progress.

The pinned audit now retains **41 symbolic paths from 10/15 wrappers**, up
from 37 paths from eight. `mk_event_libevent_socketpair` contributes one path;
`_mk_event_wait_2` contributes three, preserving infinite, nonblocking and timed
wait alternatives. Four wrappers remain Unknown at assertion macros and inject
remains Unknown at its loop. All results are source projections with opaque
calls, not resource/event safety certificates.

Explicit enum initializers are resolved by Clang against the same preprocessed
input. Missing, ambiguous, implicit or unsupported enum definitions fail closed.
No host socket constants are substituted. The bounded stack profile retains
uninitialized two-int arrays and two-field timeval initializers, their lexical
storage starts/ends, ordered initializer elements and explicit cell reads.
Variably sized arrays, incomplete/unsupported initializers, and unsupported
consumer replay remain Unknown. Symbol checking recursively validates initializer
references; opaque calls do not manufacture initialized output values.

The callback execution binder explicitly rejects the new aggregate-storage
operations until its memory model supports them. Existing callback summaries
are unchanged. Four new Lean theorems cover abstract storage freshness and
identity removal/preservation; they do not prove Clang, C layout/initialization,
or Rust import/replay.

## Validation

Workspace tests, audit-example tests, formatting, warnings-denied Clippy,
Lean build and dependency audit pass. Mutation tests cover source enum changes,
unsupported initializers/VLAs, opaque out-cell reads and nested invalid symbols.
See the individual logs and [parity.json](parity.json).

The complete pinned resource inventory is byte-identical to the preceding
contract-loader run: 185 units, 4,688/4,775 projections, two Safe, 98 provisional
Unsafe, 81 Unknown, four data-only and zero gateway failures. Callback summaries
are also identical. Projections are not proved coverage; provisional Unsafe is
not an upstream defect claim. Native instrumentation was unchanged and was not
rerun for this source-only increment.

Reproduce from the repository root:

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/cycle19-wrapper-source.json /tmp/cycle19-wrapper-diagnostics.json
cargo test --workspace
cargo test -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
(cd proof && lake build && lake env lean AuditCycle19.lean)
```

19B concrete custody mapping, 19C source lifecycle/nested teardown, and 19D
assertions/loops, complete map provenance, failure custody and event-program
composition remain open. 19E must audit their eventual composition.
