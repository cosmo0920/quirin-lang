# AUDIT-IR-RETAINED-PRODUCTS

Base: `31c5ed500d9d4a1448a623b839e1644adb19efd1`.
Parent: [AUDIT-IR](../../development/audit-ir-card.md).

Specification and prediction, committed before implementation: trace every retained
CFG through fresh-source certificate replay and report/plugin consumers. Release
only owners whose final consumer has completed; preserve public full-record
summary APIs and certificate equality. Retained phase products must preserve all
ordered console output, errors, rows, counts, refutation tables and member facts.
Use a transactional run-local `.qc` spool where products must remain addressable,
with caller-held identity, bounded reads, corruption refusal and RAII cleanup.
A spool is temporary storage of already produced data, never independent evidence
or source-free admission. Fresh-source replay and opt-in IR capture stay unchanged.

Prediction: retaining serialized phase products on disk instead of owned report
JSON trees reduces the live phase backlog; releasing replay CFGs after equality
reduces overlap with subsequent validation. Final fold consumers may still require
run-wide rows/facts and must be disclosed separately. No numerical memory reduction
is claimed until measured. Exact report/full-record parity, corruption and cleanup
controls and the complete standing gate are required. Heavy gates and production
measurements use `/tmp/quirin-audit-ir-next-heavy.lock`. All new private outputs and
builds belong to this worktree. No semantic, coverage or 6 GB acceptance claim is
made by this prediction; the parent card remains open.

## Implemented ownership boundaries

| Owner | Last use and retained representation |
|---|---|
| Independent certificate reconstruction | Full summaries, including every CFG, are still compared with the presented certificate. The private reconstructed record is dropped immediately after exact equality; remaining checks read the presented certificate. |
| Public `LegacyFunctionSummary` / certificate | Complete CFGs remain present. No empty-CFG substitute or public full-record API change. |
| Current and configured JSON phase outcomes | Complete unit rows, bounds rows/evidence, null rows/evidence and callback/state facts are parked in one transactional `.qc` object per report fragment. Small refutation sets are derived from the original rows before parking; the same union chooses the second phase. Console strings, counts, source/compile identity and required prepared source leases remain owned. |
| Repeated and cross-unit phases | Discarding a phase drops its spool owners. A potentially promoted cross-unit phase and its fallback can coexist as spooled products. Accepted products restore in original task order; a restore failure aborts rather than publishing incomplete evidence. |
| Final current-route fold | Complete rows and member facts restore for existing run-wide callback/lifetime folds. Once both finish, the consuming owner releases callback facts and its pre-pass lease. Public repeated-fold and checkpoint APIs retain their full records. |
| Final report/sidecar readers | All final rows, diagnostic counts, sidecars and assumptions remain resident until publication. This slice does not make the final fold itself streaming or claim a bounded whole-job peak. |

The version-1 run-local JSON envelope is written to `pending.qc`, synced, then
renamed to `product.qc` in an exclusively created process/sequence directory under
`private/audit-retained-products/` in the invocation's working directory. Readers
use caller-held length and SHA-256, check actual length, read at most expected
length plus one byte, verify exact contents, then decode. The maximum serialized
object is 1 GiB; exceeding it or any I/O/decode/integrity failure is an explicit
command failure, never omitted rows or optimistic admission. Serialization and
restoration temporarily hold one encoded product; this is not constant-space
encoding. Serde's recursion limit stays enabled. Objects have no cross-run reuse
or independent evidence authority. Opt-in source IR capture remains separate.

RAII removes owned product directories on normal consumption, discarded phases,
errors and Rust unwinding. It never scans or deletes another run's directory.
An externally killed process can leave disposable stale directories; a later run
skips their names rather than trusting or overwriting them. Cleanup during Drop
is best-effort, so disk/filesystem cleanup failure is not a claim of removal.

No inference rule, summary-validation gate, source identity, refusal precedence,
checker plugin, public checkpoint encoding or Lean semantic definition changes.
Existing audited models remain the semantic correspondence; serialization
controls are observations, not a new Rust-to-Lean refinement proof.

## Verification in progress

The six focused controls pass (exit 0): exact ordered payload round trips;
complete report bytes and every serialized sidecar/fact vector; real Clang-derived callback closures and two exact run demotions; incomplete, corrupt, missing,
same-length substituted and trailing data refusals; unwinding cleanup with a
separate surviving owner. Logs are worktree-private under
`private/audit-ir-retained-products/focused-retained.{log,exit}`. The earlier
three-control run also passed before envelope versioning. Its intentional
caught-panic message is the unwind test, not a failed test.

The full standing gate and unchanged 157-configuration production measurement
are queued under `/tmp/quirin-audit-ir-next-heavy.lock`. No outcome is claimed
before their receipts exist. The frozen successful baseline is borrowed with
explicit source mapping: its 1,498 Rust blobs match `52d2a3c8626eccb69ad2897ff70199934c8c2775`,
and that commit has no Rust/Cargo difference from this card's base. The executable
at its old `/tmp/qcore-audit-ir-next-target/release/quirin-audit` path no longer
exists. Fresh focused parity therefore builds the exact base in this worktree's
private source clone, with a separate target directory; it does not claim to
rerun the missing historical executable.

One additional private projection-result compaction is pending user approval.
Automatic approval review twice rejected that edit as affecting verdict-producing
code without recognized authorization. It has not been applied; its review-only
diff is [archived here](proposed-private-cfg-release.patch) (the original
review file is `/tmp/audit-ir-retained-products-proposed.patch`). The original projection
result retains its CFGs through its existing final tally/console use. This is
separate from the accepted exact-replay and final-fold ownership releases above.

The member-fact spool uses the existing checkpoint serialization surface.
`MemberFacts` explicitly excludes eight construction-only scratch maps/sets from
that surface. Those fields are not final-fold inputs; closure uses retained
stores/calls/records/layouts, and state stability uses retained `StateFacts`.
The real-Clang control checks exact serialized evidence, closure results, both
run demotions, rows and diagnostic counts before and after restoration. This
slice does not edit or compact `MemberFacts` or the pre-pass's live run facts.

Fresh fixture parity passes at one and two workers over four units / twelve
function rows, including two applied refutations and two held callers, a Safe
certificate and a data-only unit. [Harness](scripts/parity.py). The exact base
is rebuilt from `31c5ed500d9d4a1448a623b839e1644adb19efd1`; for this control only,
the candidate uses the supported `QCORE_AUDIT_BUILD_COMMIT` override fixed to
that base. Actual source commit and both binary SHA-256 values remain recorded.
Main report and assumptions are byte-identical at both job counts; the two-worker
invocation differs only in explicitly checked jobs and its two phase observations.
No report field is removed or timing-normalized. Private receipt:
`private/audit-ir-retained-products/parity3/parity.json`.

Preserved earlier probes: parity1 exits 1 for a fixture source-set/directory
name mismatch; parity2 exits 1 because uncontrolled builds carry different
embedded commit labels (12 main-report and 20 assumption `evidence_version`
fields, with no other row differences), and the requested two-worker invocation
has its expected scheduling record. The initial missing historical-binary probe
also fails. A Python `py_compile` probe cannot create `__pycache__` through the
read-only sandbox; an AST-only syntax check succeeds. Two waiting measurement
wrappers exit 143 after explicitly stopping only their captured PIDs, before any
audit starts: the first is replaced to raise the known-too-short 1800-second
harness deadline to 7200, the second to make the production comparison an
explicitly labelled fixed-build-identity control. Their private wait logs remain.
