# Cycle 19: native deletion eligibility and outcomes

The native deletion helper now has a source-derived outcome certificate, beyond
its previous result-use evidence. Five symbolic cases cover all four source
paths: both mapping states with an active fd field, active mapping with an
inactive fd field, and inactive fields with unlink returning -1 or zero.

Active fd/map bookkeeping blocks unlink and returns -1. An eligible unlink uses
the original path identity; failure reports errno and returns -1, success returns
zero. Every case preserves all record fields and both C allocations. The checker
uses symbolic fd/path identities, not sampled descriptor integers. Unsupported
comparisons, calls, writes, signatures or unresolved branches reject. This does
not establish actual filesystem deletion or caller provenance.

The shared `cleanup_outcome` interpreter supplies the replay; a separate native
profile selects fd/map/path and the syscall/diagnostic bindings. The new native
`filesystem_delete.qcore` has 12 executable clauses. It requires a nonnull live
exclusive record, a readable terminated path, returning external-call frames
and matching source/ABI. These are explicit assumptions. No null-record case is
claimed: the C function dereferences its record without a null guard.

All 378 workspace tests and the 36-module native corpus pass. Tests cover changed
eligibility gates, wrong path/null arguments, wrong return checks, record writes,
fd/type mutations, revision mismatch and every premise/postcondition changed to
false or unsupported facts. Five new Lean theorems cover eligibility, return and
diagnostic outcomes, and preservation of record/path allocations. Lean build
and Cycle 19 audit pass without placeholders or project axioms. Formatting,
Clippy, audit example tests, both probes and the pinned audit pass.

Generic diagnostics, the Monkey sidecar, all 96 scalar results and every prior
teardown certificate are identical to the cleanup-frames baseline. A separate
`filesystem_delete_outcomes` entry holds the new certificate and exact contract.
Generic counts stay at 185 units, 4,688/4,775 projections, two Safe, 98 provisional
Unsafe, 81 Unknown, four data-only and zero gateway failures. These are triage
counts, not proven coverage or confirmed upstream defects. No new runtime
execution is claimed. Clang/Rust replay and external-call framing remain trusted.

Evidence: `parity.json`, `chunk-contracts.json.gz`, validation logs and
`evidence.json`. Commands match previous reports with `/tmp/cycle19-delete-outcomes`
as output prefix. Outer deleting close, full sync frames, actual OS/provenance
binding, enclosing chunk close, traversal and event composition remain open.
