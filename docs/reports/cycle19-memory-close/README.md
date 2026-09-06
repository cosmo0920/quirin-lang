# Cycle 19: source-derived memory chunk close composition

The pinned `cio_chunk_close` now has a checked native contract for its null and
memory-backed paths in `contract/qcore/chunkio/memory_close.qcore`. The source
binder rechecks the actual reset, memory-backend, list-unlink and total-counter
dependencies before interpreting calls. It binds `_head` and `_state_head` as
distinct embedded links, follows the name and chunk releases, and requires the
counter call to use the context read before the chunk is freed.

**Two of four source paths are checked**: null and memory-backed. Filesystem and
other-backend paths are explicitly uncovered. This is not a whole-function or
whole-unit Safe result. The certificate records every source path, both checked
indices, both uncovered indices, all dependency certificates and exact native
contracts. The memory store constant is obtained through an appended typed
probe using the selected source's `CIO_STORE_MEM`, not a host literal; the exact
probe and its projection are retained. The probe is generated audit machinery,
not an additional upstream function or coverage numerator.

The null path has no calls. The memory path has seven bound calls: reset,
backend close, two unlinks, two frees, and one saved-context decrement. All eight
native postconditions check under the declared domain. Calls on uncovered
backend paths remain opaque. Unsupported C anywhere in the imported definition
still prevents its source summary; an unmodeled operation on a selected path
prevents composition.

`MemoryCloseCertificate::instantiate` binds a supplied live memory chunk to a
`CloseChunk` operation in the existing identity-indexed lifecycle checker. It
checks live context/stream membership, both links, counter bounds and outstanding
lock/busy/task/route/transaction obligations. Null produces no operation. The
integration test executes the resulting operation and checks backend/name/object
release, both links, stream removal and context decrement, then rejects repeat
close. Context and stream destruction remain separate obligations.

## Premises and limits

The contracts require correct nullable chunk ownership, disjoint owned backend
and name allocations, distinct live stream-list memberships, exclusive access,
a context and stream that outlive the chunk, a counter covering the live chunk,
no outstanding wrapper obligations, libc free binding and matching cross-unit
source/ABI identity. Constructors and arbitrary call sites do not yet establish
these premises automatically. Model identities still need concrete C pointer
provenance supplied by the caller.

Composition review made the backend's root-disjointness premise explicit:
`memory_backend.qcore` now additionally requires
`owned_fields_are_disjoint_from_root`. Its nine replay states, source projection
and release traces are unchanged. Missing this premise rejects a new contract;
composition rechecks dependencies against the current native inputs, so an older
or weakened contract cannot bypass the strengthened requirement.

## Validation

All **357 workspace tests** and the 29-module corpus pass, as do formatting,
warnings-denied Clippy, audit example tests, both reevaluation probes, Lean build
and the Cycle 19 dependency audit. Source mutations cover duplicate/wrong link
identity, counter role, release order, freed-chunk reads, missing release,
unsupported loops, incorrect embedded-field type and an early-return variant
using `(!ch) == 0`. Logical C values are normalized to integers for comparison;
the latter variant is rejected rather than misclassified as an infeasible path.
Contract tests mutate signatures, domain/lifetime premises and every postcondition.

Six new Lean theorems model ordered memory close, saved-context binding,
chunk-before-counter discharge, exact decrement, duplicate counter rejection
and empty-trace preservation. They prove the abstract operations, not arbitrary
C, Clang translation, Rust replay or constructor provenance. There are no
placeholders or project axioms in the dependency audit.

The generic resource inventory is byte-identical to the list-unlink baseline.
The full Monkey sidecar, all 96 scalar results and the unlink/counter certificates
are identical. The backend certificate differs only in its strengthened native
input. Generic totals remain 185 units, 4,688/4,775 projections, two Safe, 98
provisional Unsafe, 81 Unknown, four data-only and no gateway failures. No
projection percentage is called safety coverage and no provisional Unsafe is
called a confirmed upstream defect. Native observation code was unchanged.

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
  /tmp/cycle19-memory-close-source.json /tmp/cycle19-memory-close-diagnostics.json
```

Cycle 19 remains in progress: filesystem cleanup/error outcomes, stream/context
iteration, constructor/source custody, wrapper lifecycle and full Monkey event
composition remain open. Filesystem close calls `munmap_file` and
`cio_file_native_close` without checking their results; future composition must
retain their failure obligations rather than infer success from calls.
