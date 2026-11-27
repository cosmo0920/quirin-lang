# Cycle 19: source-bound nested memory teardown

The complete memory-backed dependency chain now composes through
`cio_destroy` → `cio_stream_destroy_all` → `cio_stream_destroy` →
`cio_chunk_close_stream` → `cio_chunk_close` → `cio_memfs_close`, including
the reset, both chunk list removals, stream list removal, name/parent frees
and total counter decrement. The composition requires the correct private
source witness for each role and rechecks memory-close dependencies.

The instantiation API accepts an existing checked lifecycle prefix, a selected
context identity and ordered stream/chunk ring bindings. It requires exact,
duplicate-free membership and counter equality, binds each memory chunk through
the checked close certificate, then executes the emitted operations through
the lifecycle checker. It closes each child before its stream and all streams
before the context. Null contexts perform no operation; empty contexts and
streams still release their corresponding parent allocations in the model.

Missing, repeated, foreign, already closed or filesystem children reject, as
do outstanding lock, busy, task, route and transaction obligations. Other contexts
remain live. Returned operations are a checked prefix; the entire program must
still pass `check_chunk_program` before receiving a terminal model certificate.
The API bounds prefixes to 4,096 operations and teardown to 512 streams/chunks.

Native allocation provenance, actual ring order, ordinary live reciprocal
storage, exclusive access, all component preconditions and cross-unit ABI/call
bindings remain caller premises in a new 10-clause QCore policy. This is a
conditional source/model composition, not proof that arbitrary native callers
meet those premises. Parent-owned fields are explicitly disjoint from child and
unrelated storage. Filesystem failure states are not sent through the core
model's unconditional cleanup operation.

All 396 workspace tests, 42 contract modules, audit example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency
audit and pinned external audit pass. Existing Lean lifecycle, traversal and
parent-release models are reused; no new theorem claims to verify the Rust
binding or Clang. The generic inventory, Monkey source, scalar results and all
17 prior teardown entries are unchanged. One new nested source certificate is
recorded. Aggregate verdicts remain two Safe, 98 provisional Unsafe, 81 Unknown,
four data-only units and zero gateway failures across 185 units.

Reproduce with the [traversal commands](../cycle19-teardown-traversal/README.md)
using `/tmp/cycle19-nested` as output prefix. Logs, compressed evidence, parity
and hashes accompany this report. No native execution or upstream defect is
claimed. Filesystem residual composition, native caller/constructor provenance,
sync/OS guarantees, aggregate custody and wider event composition remain open;
Cycle 19 remains in progress.
