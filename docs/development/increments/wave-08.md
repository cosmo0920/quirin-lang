# Wave 8 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-8 delegated increment (2026-09-09): four bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate; this paragraph supersedes nothing above and each
card's record keeps its own boundaries.
- Legacy route removals (P09-ROUTES): P09-G, the zero-helper
  `chunk_source::import_chunk_source` wrapper, is removed after archived
  byte-identical parity over all seven consumer source shapes; the public API
  removal moves `qcore-compiler` to 0.3.0. The six other routes are retained,
  each with its measured missing piece named (P09-A..F gaps), and two of the
  P09 report's own notes are corrected by measurement: C's real blocker is
  the Direct-mode no-selector default, and F is a test-rehoming refactor plus
  multi-path API break, not eleven pure re-exports. Evidence:
  `docs/reports/remaining-p09-routes/README.md`.
- Epoch CT lane (T08-SRC-CT): the schema-2 harness gains the `epoch` route
  and the exhaustive `epoch_bound_figures` verdict mode (the whole published
  figure map is pinned, not a convenient subset); CT35, CT36, CT38 and CT39
  map with every cascade and figure re-measured on fixture bytes — CT
  coverage moves 24/50 to 28/50, sanitized-native stays 3 of 28 and proof
  0 of 28, so the grown denominator is mapping, not conformance. Eighteen
  scratch mutations all refuse. One deliberate deviation from the parked
  rows is recorded (`native_sanitized: not_applicable` — the check-only
  route emits nothing to sanitize, structural under the route rule) and one
  correction found by re-measurement (the parked certificate blocks
  under-enumerated the published figures). Evidence:
  `docs/reports/wave8-epoch-ct/README.md`.
- Function enumeration (E01-F1): the main-source function denominator is
  completed from a second, enumeration-only preprocessing with line markers,
  leaving the published `-E -P` text and every bound digest untouched. The
  185-unit denominator moves 4,775 to 4,921 (+146) with zero verdict
  movement in either direction; the actual cause is a raw-tokenizer defect
  (a `#` on a macro continuation line swallows a closing brace, hiding 146
  of rdkafka.c's 165 definitions), named `absent_from_raw_scan` and recorded
  as follow-on E01-F1-RAW rather than repaired. K&R definitions count with
  explicit Unknown rows and classified reason codes; refused enumeration
  claims withhold whole-unit Safe. The baseline is deliberately re-pinned to
  E01-F1's after-measurement after its before-run reproduced the prior
  baseline byte-identically. 38 audited Lean theorems
  (`QCore.SourceEnumeration`). Evidence:
  `docs/reports/remaining-e01-f1/README.md`.
- ChunkIO teardown capabilities (P04-NEXT): of the thirteen remaining
  in-process capabilities, the list-unlink two-summary relation and the
  memory-backend nullable-field destructor move through the process boundary
  in the P04/P05 pattern — both bodies' inputs travel with no derived
  reading in any request, calls are admitted only against a closed
  per-family callee table, and the sealed core re-evaluates the pinned
  contract before requiring agreement, which discharges nothing. Parity is
  byte-identical with the retained in-process routes; a 92-row mutation
  matrix all refuses; 26 audited Lean theorems (`QCore.TeardownAdmission`).
  The remaining eleven are classified: one movable with a third vocabulary
  (P04-NEXT-A), two dependency-blocked (B), two behind the sealed
  SourceMatch boundary (C), seven compositions. `qcore-schemas` is 7.0.0
  (source-incompatible additions with reasoning recorded; existing wire
  bytes pinned unchanged); ChunkIO package 0.6.0; all five packages rebuilt
  and verified. Evidence: `docs/reports/remaining-p04-next/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units — the baseline
artifacts are deliberately re-pinned to E01-F1's after-measurement with the
denominator at 4,921 and zero verdict movement — the contract corpus stays
60 modules and full language conformance stays false. CT coverage advances
to 28/50 mapped, by executed-stage evidence only. `qcore-schemas` is 7.0.0;
`qcore-compiler` is 0.3.0. Merged gate at this checkpoint: 1787 workspace
tests (3 ignored), 67 provider tests, 108 Python tests, 44 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
