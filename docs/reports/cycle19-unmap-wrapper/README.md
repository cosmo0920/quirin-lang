# Cycle 19: conditional unmap wrapper composition

The complete pinned `munmap_file` source now composes with source-checked native
unmap and the unsigned `total_chunks_up` decrement. All eight source paths are
covered by exactly one null-backend, absent-mapping or sync/native-outcome case.

| Entry | Native call | Wrapper writes | Up-counter decrement | Return |
| --- | ---: | ---: | ---: | ---: |
| No backend or no mapping | 0 | 0 | 0 | -1 |
| Mapped, native error | 1 | 0 | 0 | -1 |
| Mapped, native success | 1 | 2 | 1 | 0 |

For a mapped unsynced file, sync runs first. A sync error logs once and continues
to native unmap. Only native success clears `data_size`/`alloc_size` and then
decrements the parameter context's up-counter. Three success cases each have
that transition; three native-error cases preserve wrapper bookkeeping and the
counter relative to the post-sync state. The no-backend/no-mapping cases make
neither call. Source path indices and the selected native dependency path are
retained, together with both rechecked dependencies and the exact native policy.

The new 16-clause `unmap_wrapper.qcore` policy explicitly assumes live/exclusive
chunk and backend provenance, parameter/context identity, Boolean sync state,
returning sync/log effects that preserve chunk identities, mappedness, counters
and lifetimes, native external-call frames, ABI binding and positive in-range
counters when decremented. Sync may change the actual mapping identity and size;
error preservation is relative to that post-sync state. The full sync body is
not yet checked: these premises do not establish it, and the outer file-close
failure chain remains incomplete. No unconditional filesystem safety verdict is
issued. C calls to sync/logging remain trusted conditional dependencies.

The counter projection rejects underflow, out-of-range values and invalid case
indices, decrements once on native success and preserves the counter otherwise.
It is a conditional scalar projection, not a pointer or full lifecycle binding.
The source checker rejects wrong cleanup outcomes, wrong/null call arguments,
wrong counter roles, premature/duplicate/missing field and counter writes,
wrong diagnostic name identities, null/no-mapping return changes and backend
release. Each native postcondition has false/unknown mutation coverage.

All 368 workspace tests and the 33-module native corpus pass, with formatting,
Clippy, audit example tests, both reevaluation probes and the pinned audit.
Five new Lean theorems prove failure preservation, zero rejection and exact
successful decrement/clearing; the build and Cycle 19 audit pass without
placeholders or project axioms. Clang and the Rust import/replay remain trusted.
The earlier 18 controlled native-helper executions remain evidence for those
helpers; no native execution of this wrapper is claimed here.

Generic diagnostics and the Monkey report remain byte-identical to the previous
cleanup-observation baseline. All 96 scalar results and all previous teardown
certificates are identical. Aggregate counts remain 185 units, 4,688/4,775
projections, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures. These are triage counts, not proof coverage or defect counts.

Evidence is retained in `chunk-contracts.json.gz`, `parity.json`, validation logs
and the `evidence.json` SHA-256 manifest. Required commands match the preceding
reports with `/tmp/cycle19-unmap-wrapper` as the output prefix. Cycle 19 remains
in progress: full sync framing, outer file-close failure custody, actual OS
outcome binding, stream/context iteration, constructor identity and wider
Monkey event composition remain open.
