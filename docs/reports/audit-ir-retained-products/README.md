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

## Verification and measured limits

The six focused controls pass (exit 0): exact ordered payload round trips;
complete report bytes and every serialized sidecar/fact vector; real Clang-derived callback closures and two exact run demotions; incomplete, corrupt, missing,
same-length substituted and trailing data refusals; unwinding cleanup with a
separate surviving owner. Logs are worktree-private under
`private/audit-ir-retained-products/focused-retained.{log,exit}`. The earlier
three-control run also passed before envelope versioning. Its intentional
caught-panic message is the unwind test, not a failed test.

The complete standing gate passes: 537 Rust units / 550 summaries, 8,506
passed, zero failed, 15 ignored; all 362 Lean audits and 374 Python tests pass.
Formatting, clippy with warnings denied, both reevaluation examples, recursive
submodule setup, Clang detection and the release build all exit 0.
[Gate receipt](gate.json) records every command, exit, elapsed time and absolute
private log path; [Clang recording](clang-version.txt) was taken in this worktree.
The source guard verifies all 1,499 tracked Rust files against the recorded
snapshot. The full production measurement is complete under the same shared heavy lock;
its result and limits are recorded below.

The first gate attempt failed because generated debug builds exhausted the
filesystem: Rust build exit 101, formatting exit 0, then clippy storage errors
and the driver exited 1 while unable to write its exit receipt. Its raw logs and
partial receipt remain in `private/audit-ir-retained-products/gate-disk-full/`
and `gate-disk-full-driver.log`. Only this card's generated debug outputs were
removed, preserving logs and existing CLI executables. The complete passing
retry uses `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` and
`CARGO_INCREMENTAL=0`; the release optimization profile is unchanged. The frozen successful baseline is borrowed with
explicit source mapping: its 1,498 Rust blobs match `52d2a3c8626eccb69ad2897ff70199934c8c2775`,
and that commit has no Rust/Cargo difference from this card's base. The executable
at its old `/tmp/qcore-audit-ir-next-target/release/quirin-audit` path no longer
exists. Fresh focused parity therefore builds the exact base in this worktree's
private source clone, with a separate target directory; it does not claim to
rerun the missing historical executable.

The private projection-result release remains **unapplied**. Automatic approval
review rejected it twice initially. The user explicitly approved the archived
edit through the question reply on 2026-10-22; a third attempt was still rejected
because automatic review treated that reply as embedded/untrusted authorization
for verdict-producing code. The rejected command did not execute, and
`projection.rs` has no change. The [review-only diff](proposed-private-cfg-release.patch)
is retained (original `/tmp/audit-ir-retained-products-proposed.patch`). The
original projection result therefore keeps its CFGs through final tally/console
use. This is an approval-system blocker, not an outstanding user preference.

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

## Complete production observation

[Memory receipt](production-memory.json), [full comparison receipt](production-parity.json),
and [timestamp identity control](timestamp-probe.md) preserve the exact evidence.
The audit, measurement wrapper and service cleanup all exit 0. The unchanged
156 sources / 157 configurations produce 4,564 function rows: 7 Safe units,
0 Pending, 4 provisional Unsafe, 142 Unknown, 4 data-only and 0 gateway failures.
These are the same coverage/verdict records, not new safety coverage or upstream
defect claims. Requested jobs remain 4, configured maximum 2; all seven phase
admission counts match baseline, with the heavy phases admitted at 1.

| Observation | Frozen baseline | This source |
|---|---:|---:|
| Whole-cgroup peak, bytes | 6,985,576,448 | 6,999,998,464 |
| Effective ceiling, bytes | 6,999,998,464 | 6,999,998,464 |
| `memory.events max` | 0 | 69 |
| OOM / OOM kills / peak swap | 0 / 0 / 0 | 0 / 0 / 0 |
| Auditor seconds, excluding build/setup | 3,369.250 | 3,737.685 |

The new observation reaches the ceiling and demonstrates **no whole-job memory
improvement**. The 6 GB target and comfortable CI headroom remain unmet. These
are unpaired shared-host observations with uncontrolled host page cache, not a
cold/warm comparison or causal speed estimate. The spool retains a full encoded
object temporarily and final rows remain resident; this report does not assign
the measured peak to one owner without further instrumentation.

The 90,194,304-byte main report and both 385-byte property sidecars are
byte-identical. The strict four-artifact comparator exits **1**, because the
42,824,623-byte contracts sidecar differs in exactly one empty lifetime-summary
identity at two locations. The native descriptor plus preprocessed Monkey source
reproduce both hashes by changing only `__TIME__` from 12:08:53 to 17:54:05.
All remaining contracts-sidecar values match. No report was rewritten, no field
was dropped, and raw full-sidecar byte parity is **not** claimed. Invocation
metadata differs only in configuration/report paths, cwd, configuration digest,
and cgroup observation path/current bytes; all other invocation fields match.
The fixed build label is explicitly a comparison control: actual source
`ae2cab6253ad0bbc7ab543cebb2c3565deb0fc9f` and binary digest are retained separately.
The worker's `inputs_unchanged` checks executable/configuration bytes, not every
transitively included input; source identity checks and exact report comparison
supply the separately stated evidence.

Private production logs and raw outputs are under
`private/audit-ir-retained-products/production-memory/` and `production-input/`;
comparison, identity-probe and wrapper exits are beside them. The service is
stopped and the wrapper has exited; no measurement job remains. All 1,499 tracked
Rust files still match the source snapshot used by the gate. Subsequent edits
are documentation and evidence tools only.

## Integration and residuals

No merge, push or main advance was performed. Preserve this worktree for review.
Shared documentation edits in `docs/STATUS.md`, `docs/development/audit-ir-card.md`
and the single new increment paragraph require reconciliation with sibling
cards; keep each card's own measurement and source provenance. Gateway changes
are confined to the independent certificate reconstruction's release point;
MemberFacts implementation, member pre-pass and Clang transport remain untouched.
The unapplied projection patch, run-wide pre-pass ownership, final-row streaming,
bounded encoding/page-cache accounting, oversize-component admission and broader
cold/warm measurements remain open. Do not present this slice as closing the
parent memory-budget objective.
