# Cycle 19: backend release retains unresolved OS custody

The non-deleting `cio_file_close` chain is now bound to source-checked
`munmap_file` and native close dependencies. It follows the actual backend,
parameter context and nullable path identities through both cleanup attempts,
path free and backend free. The same-translation-unit unmap summary must match
the caller's freshly imported source; both dependencies are rechecked before
composition. Cleanup results remain ignored, exactly as in the pinned source.

The finite conditional inventory has 43 cases: one null backend and 42 nonnull
combinations of seven unmap cases, three descriptor-close cases and two path
allocation states. The null/no-delete source paths 0 and 3 are checked; deletion
paths 1 and 2 remain explicitly uncovered. A nonnull backend allocation is
released in all 42 cases, and a live path allocation is released in 21.

**Allocation release does not erase OS custody.** Under the recorded external
success-discharge and source/frame premises, 17 cases have no nested mapping or
descriptor obligation, including the null case. The other 26 retain unresolved
mapping and/or descriptor custody and fail `require_nested_discharge`, even
though the backend allocation has been freed. These are conditional symbolic
combinations, not counts of actual executions, probability estimates, verified
caller states or confirmed upstream leaks. A failed close is not assumed to
leave the descriptor open. No whole-file/chunk/unit Safe verdict is issued.

The new 16-clause `file_close.qcore` policy requires delete exactly zero, owned
nullable backend/path provenance and disjointness, live context/stream/name
identities, unmap premises and counter bounds, cross-call descriptor/mapping/
path frames, actual successful syscall discharge and source/ABI/libc bindings.
Actual OS outcome and constructor/caller binding remain open. The chunk root
and its backend pointer are not written by this function; the pointer can be
left dangling after backend free. Chunk unlink/name/root/counter teardown is a
later enclosing operation, not inferred here.

Tests cover all 43 cases and the 26 rejected nested-discharge gates, both nullable
path states, missing/reordered/wrong-identity frees and calls, changed cleanup
result handling, backend writes and stale same-unit dependency summaries. Every
native postcondition is tested with false and unsupported replacements. The
unmap fixture is shared as a C file, preserving its prior source semantics.
All 371 workspace tests and the 34-module native corpus pass, as do formatting,
Clippy, audit example tests, both probes and the pinned audit. Six new Lean
custody theorems and the full build/audit pass without placeholders or project
axioms. They prove that backend release preserves nested custody and unresolved
custody blocks discharge. Rust/Clang/source and external framing remain trusted
boundaries. No new runtime execution is claimed in this step.

Generic diagnostics and the Monkey sidecar are byte-identical to the unmap
wrapper baseline. All prior scalar and teardown evidence remains identical.
The new evidence is a separate `filesystem_close_no_delete` sidecar entry.
Aggregate counts remain 185 units, 4,688/4,775 projections, two Safe, 98
provisional Unsafe, 81 Unknown, four data-only and zero gateway failures.

Evidence: `parity.json`, `chunk-contracts.json.gz`, required validation logs and
`evidence.json`. Commands match the previous reports with
`/tmp/cycle19-file-close` as output prefix. Cycle 19 remains in progress:
deletion paths, full sync framing, actual OS/provenance binding, enclosing
filesystem chunk close, stream/context iteration and full event composition
still require work.
