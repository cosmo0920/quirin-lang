# Cycle 19: deleting backend close composition

The outer `cio_file_close` now has checked conditional composition for both
Boolean delete modes. The previous 43 zero-delete cases remain unchanged; 30
one-delete cases cover the null return and the two previously uncovered delete
branches. Together they cover all four complete pinned source paths.

| Deleting case classification | Cases | Backends freed |
| --- | ---: | ---: |
| No backend, no deletion attempt | 1 | 0 |
| Native delete blocked by post-cleanup fd/map fields | 13 | 13 |
| Eligible unlink reports failure | 8 | 8 |
| Eligible unlink reports success | 8 | 8 |

These are symbolic combinations, not executions or probability estimates.
The native delete dependency is rechecked against its canonical contract.
Post-unmap/close field states select its exact eligibility case: failure leaves
an active bookkeeping field, while absent/success cases leave it inactive.
Actual OS custody after failure remains unresolved. The same backend identity
is passed to delete, its error result controls one outer diagnostic, and both
path and backend allocations are released afterward. The helper's own errno
report is distinct from this outer diagnostic.

A successful-deletion gate accepts only the eight conditional success cases.
Null backend is not evidence of deletion. Neither blocked requests, unlink
errors nor unresolved mapping/descriptor custody can be erased by freeing the C
allocations. Cleanup results remain ignored in the outer source; the new logic
checks their conditional effects through the dependency inventory.

`file_close_delete.qcore` has 13 executable clauses. It replaces the zero-delete
premise with delete exactly one while inheriting the checked custody/frame/ABI
premises, and requires a live owned terminated path for nonnull backends,
returning framed native-delete/logging calls and binding of successful unlink
to the backing path. It does not establish constructor/caller provenance or
actual OS behavior. Native-delete plus sync/log/external frames remain explicit
boundaries. The complete source-path result is not whole-unit safety.

All 381 workspace tests and the 37-module native corpus pass. Tests cover the
30 classifications, the deletion gate, wrong/null delete arguments, early free,
wrong error guards, early return, swapped diagnostic identities and every new
premise/postcondition changed to false or unsupported facts. Three new Lean
theorems prove failure cannot become success through allocation release, no
attempt is not success, and accepted deletion has no nested obligation. Lean
build/audit, formatting, Clippy, audit example tests, both probes and pinned
audit pass without placeholders or project axioms.

The generic inventory, Monkey sidecar and all prior scalar/teardown evidence
are identical to the native-delete-outcomes baseline. New evidence is a separate
`filesystem_close_with_delete` entry; the older zero-delete projection retains
its scope and uncovered-path list. Aggregate verdicts remain two Safe, 98
provisional Unsafe, 81 Unknown, four data-only and zero gateway failures across
185 units. No new runtime execution or confirmed upstream defect is claimed.

Evidence: `parity.json`, `chunk-contracts.json.gz`, logs and `evidence.json`.
Commands match prior reports with `/tmp/cycle19-deleting-close` as output prefix.
Full sync frames, actual OS/caller identity binding, enclosing filesystem chunk
close, stream/context traversal and wider event composition remain open.
