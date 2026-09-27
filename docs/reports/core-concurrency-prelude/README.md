# Core concurrency prelude — 2026-09-08

> Runtime import maintenance: archived source inputs and runnable snippets now use `#import`; original results and hashes describe their recorded revision. See the [spelling attribution](../runtime-source-imports/spelling-attribution.json) for maintained source identities.

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


The concurrency implementations were already in `qcore-core`. This increment
consolidates their reusable import surface as `qcore_core::prelude::concurrency`
and routes the frontend and verification driver through it. It does not move or
duplicate the semantic implementations. The original core module paths and the
compiler compatibility paths retain the same Rust types and functions.

QCore source now requires an explicit `#import std.concurrency;` in each file
that uses concurrency. This intentionally changes source admission: concurrency
is unavailable by default, including in uncalled bodies and library modules.

## Import and construct a context

```rust
use qcore_core::prelude::concurrency::{
    ScopedTaskLedger,
    scoped_tasks::{ROOT_TASK, TaskOutcome},
    task_execution::{check_task_module, execute_task_module},
};
```

The prelude exposes the `scoped_tasks`, `task_execution`, `task_c_emit`,
`task_trace_relation`, `concurrency`, `mutex_invariant`, `mutex_c_emit`,
`native_concurrency`, `weak_memory` and `region_task_credit` namespaces. It also
exports the principal program types and checker entry points directly.
Identity aliases remain qualified by their original model; the different task,
resource, outcome and certificate domains are not merged.

`ScopedTaskLedger::default()` constructs the modeled root task context. Scope
entry, reservation, creation commitment or rollback, completion and explicit join
remain checked ledger operations. Imports grant no authority and launch no OS
threads. A ledger is a semantic model, not a runtime thread-pool handle.

The prelude is a Rust API for compiler and library implementers. Source imports
use dot-separated identifiers after `import`, terminated by a semicolon, after
the optional module header at any top-level declaration boundary. The initial
bundled registry contains only `std.concurrency`. Imports are file-local and
repeated imports are idempotent. `Q014` rejects unknown modules and `Q015`
rejects missing concurrency imports; both belong to the name-resolution pass.
Each input is checked before combining declarations so imports cannot leak
between files. Declarative contract ghost fields named `thread` or `lock` do
not request concurrency. The task profile still requires its separate
`concurrency checked;` marker. Importing a module enables vocabulary, not safety
certification or otherwise unsupported constructs.

This implements exact bundled-module imports. User-file/package resolution,
aliases, wildcards, reexports, implicit/transitive imports and a general
standard-library ABI remain outside the implemented scope.

## Source examples

The [example mapping](../../../examples/threads/README.md#mapping-source-examples-to-the-core-prelude)
connects source scopes, spawn, worker completion, join and read shares to the
existing core and Lean models. The paired examples are:

- [`core_context.qr`](../../../examples/threads/core_context.qr): one
  buffer passed to a worker, returned at join and released; creation failure
  restores the original binding, which the parent releases.
- [`concurrency_prelude.rs`](../../../crates/qcore-core/examples/concurrency_prelude.rs):
  direct construction of the corresponding custody model for both creation
  outcomes, including refusal to leave the scope before collecting the child.

The Rust example models identity/custody, not the source program's buffer value
or native scheduling. The source integration test actually lowers the `.qcore`
example, emits and runs C with successful and injected failed creation, compares
each trace with the core reference execution and checks exactly one buffer
allocation/release pair. The Windows example build script includes the new
source; this increment's native validation runs on Linux.

## Lean proof surface

```lean
import QCore.ConcurrencyPrelude
```

This imports the existing concurrency lifetime, scoped custody, task execution,
owned execution, read-loan/share, error-payload, mutex, task-trace, task-credit and
lifetime-correspondence models. Fourteen selected properties are exported under
`QCore.ConcurrencyPrelude`, preserving their original statements and premises:

- Reservation rollback, preservation of live identities, inaccessible unjoined
  capsules, creator custody return and refusal of joined-task resource use.
- Failed creation preserves handles, join consumes its handle once, and handler
  exit requires quiescence while preserving the exact outcome.
- Outstanding lent shares stay within the budget; exclusive authority returns
  only after the final share is collected.
- Mutex transitions conserve modeled parked authority, guards authorize only
  their declared footprint, unlock requires a closed invariant, and forged
  synchronization witnesses are refused.

`ModuleImports.lean` additionally models the new resolver and capability gate.
Six theorems establish empty defaults, unknown-module refusal, explicit-import
necessity, duplicate-import idempotence, refusal of missing required capability
and sequential admission without concurrency. Source syntax detection and
file-local driver wiring are tested implementation boundaries, not proved
Rust-to-Lean refinement.

`AuditConcurrencyPrelude.lean` audits all 14 exports and six new theorems.
`QCore.lean` imports the module, so `AuditAll.lean` includes it in the full axiom
closure check. No project axioms or stronger native/refinement claims are
introduced. Importing task and mutex proofs together does not establish their
composition. Platform emitters and weak-memory APIs retain their own existing
evidence boundaries; the selected 14 properties are not blanket proofs of those
implementations.

## Validation

[`semantic-parity.json`](semantic-parity.json) records baseline and final SHA-256
identities for the unchanged Rust model/emitter files and Lean model files.
The underlying concurrency transitions, emitters and original proof premises
are unchanged. Source grammar/admission now includes explicit imports and two
new diagnostics. Existing executable concurrency fixtures and examples opt in
explicitly; the original contract corpus is unchanged. Certificate acceptance
and production C verdict rules are unchanged.

The standing gates and example validation are recorded in
[`validation.json`](validation.json).

The Libevent submodule was already initialized at the pinned revision
`d82464a277d0f42703702c4dfd9af6af38595a83`. Re-running initialization encountered
the workspace's read-only Git metadata; `git submodule status --recursive`
confirmed the existing checkout and the workspace tests check its pin.

Final result: **1387 workspace tests passed, zero failed, 1 ignored**;
formatting, Clippy, both reevaluation probes, all 32 Lean audits and nine Emacs
ERT tests passed. All four QCore thread examples were checked, emitted, compiled
with strict Clang warnings and run on Linux, returning their expected main
results. The initial sandboxed workspace attempt could not run LeakSanitizer;
the complete final workspace gate ran outside that sandbox with sanitizer
checks enabled. See [validation summary](validation-summary.txt) for compact
command evidence and `validation.json` for native traces and hashes.
