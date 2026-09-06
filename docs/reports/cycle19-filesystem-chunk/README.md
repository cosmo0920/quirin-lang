# Cycle 19: enclosing filesystem chunk close

The filesystem arm of `cio_chunk_close` now composes the previously checked
backend outcomes with error reset, two embedded list removals, name/chunk frees
and the total-chunk decrement through the saved context. The source forwards
the exact Boolean delete argument and chunk identity to `cio_file_close`.
Backend kind is imported through a compiled `CIO_STORE_FS` probe and must be
distinct from the separately imported memory kind.

| Entry family | Cases | Nested discharge gate | Successful deletion gate |
| --- | ---: | ---: | ---: |
| Null chunk, either delete mode | 2 | 2 | 0 |
| Nonnull chunk, delete zero | 43 | 17 | 0 |
| Nonnull chunk, delete one | 30 | 17 | 8 |

These are conditional source cases, not runtime executions. Null backend cases
still release the enclosing chunk; a null chunk makes no call. All 73 nonnull
chunk cases remove both links, release name/chunk and decrement total once.
The backend's up-counter changes are retained exactly. Thirty-nine cases keep
unresolved mapping or descriptor custody after the chunk allocation is freed.
Eight eligible unlink failures discharge nested mapping/descriptor obligations
conditionally but do not establish backing deletion; neither does a null backend.

The implementation reuses the memory-close identity/order interpreter for the
filesystem dispatch arm. It rechecks the private memory/common dependencies and
both backend modes against canonical contracts. The original memory certificate
and its serialized scope stay unchanged. Filesystem scope checks two of four
paths; memory plus filesystem scope covers three. Unsupported backend kinds
remain explicitly uncovered rather than silently modeled as successful cleanup.

The new 22-clause `filesystem_chunk_close.qcore` states owned/disjoint storage,
live reciprocal list memberships, context/stream lifetime, counter bounds,
absence of wrapper obligations, backend outcome/frame premises and cross-unit
source/ABI/free bindings. Callers and actual OS outcomes are still assumptions.
No lifecycle operation is emitted without such binding, and this conditional
report does not promote a whole chunk, stream or translation unit to Safe.

All 385 workspace tests and 39 native contract modules pass. Mutations reject
wrong/fixed/null backend arguments, duplicate calls, early return/free, duplicate
list removal, post-free context reads, wrong counter roles and overlapping
backend kinds. Every new premise/postcondition rejects false or unknown facts;
different distinct backend constants are supported through the probe.
Six new Lean theorems cover residual outcome preservation, final allocation/list
discharge, total-only decrement, zero-counter and duplicate-release rejection,
and unresolved custody surviving chunk release. The abstract post-backend model
does not prove the Rust interpreter or Clang correct.

Formatting, Clippy with warnings denied, audit example tests, both reevaluation
probes, Lean build/dependency audit and pinned external audit pass. No placeholders
or project axioms are introduced. The generic diagnostic inventory and Monkey
source sidecar are byte-identical to the native-observation baseline. All prior
scalar results and teardown entries are identical; one new
`filesystem_chunk_close` entry records this step. Aggregate counts remain 185
units, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures. No new runtime execution or upstream defect is claimed.

Evidence and reproduction use `/tmp/cycle19-filesystem-chunk` as output prefix
with the prior report's validation commands. `evidence.json` hashes logs, compressed
sidecar and parity report. Cycle 19 remains in progress: full sync framing,
actual OS/caller identity binding, constructor provenance, unsupported backend
classification, stream/context traversal and wider event composition remain open.
