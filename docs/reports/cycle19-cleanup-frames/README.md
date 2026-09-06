# Cycle 19: explicit cleanup storage separation

The unmap and non-deleting file-close contracts now state the storage separation
used by their symbolic identity interpreters. Their native policies each have
17 clauses. This strengthens the caller boundary; it does not establish caller
provenance or make a new whole-unit safety claim.

- `unmap_wrapper.qcore` requires disjoint storage for the chunk, backend, context
  and stream, so backend field writes and the context decrement cannot alias a
  different modeled record.
- `file_close.qcore` separately requires the owned backend and nullable path to
  be disjoint from the borrowed chunk root. Disjoint ownership of backend and
  path alone must not be read as proving the chunk's root allocation survives.

The canonical builders retain these premises in every certificate. File close
rechecks its unmap dependency against the strengthened canonical contract before
composition. Tests now replace every precondition as well as every postcondition
with false or unsupported facts and require rejection. The caller must still
establish the premises; source parsing does not prove allocation provenance.

All 375 workspace tests, the 35-module native corpus, formatting, Clippy, audit
example tests, both reevaluation probes, the Lean build/audit and pinned audit
pass. No new Lean semantics is introduced. Existing frame and custody proofs
continue to be conditional on their stated identity distinctions. The 18 earlier
sanitized native-helper observations remain unchanged and are not alias proofs.

Generic diagnostics, the Monkey sidecar and all 96 scalar results are
byte-identical/identical to the short-circuit baseline. The only teardown report
changes are the new exact contract texts in the unmap certificate and its
file-close embedding. Every source summary, case, effect, counter projection,
residual-custody classification and all other evidence remain identical. The
43 file-close cases still contain 26 unresolved nested-custody cases; deletion
paths remain uncovered. Aggregate generic verdict counts are unchanged.

`parity.json` records this precise premise-only delta; `chunk-contracts.json.gz`,
validation logs and `evidence.json` retain the evidence. Required commands match
previous reports with `/tmp/cycle19-cleanup-frames` as output prefix.

Cycle 19 remains in progress: source-derived deletion outcomes, full sync
framing, actual OS/caller identity binding, enclosing filesystem chunk close,
stream/context traversal and full event composition remain open.
