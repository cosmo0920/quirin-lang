# Wave 7 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-7 delegated increment (2026-09-09): five bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate; this paragraph supersedes nothing above and each
card's record keeps its own boundaries.
- OS-failure declared-model repairs (O03-U4U5): DescriptorCustody.lean's
  failed-close clause no longer certifies a false retention — the four-field
  Close record keeps its success arm and every re-exported name, its failure
  arm is withdrawn, and failed closes live in a new TargetedClose whose
  validity is decided by closePostState itself, digest-tied to
  qcore-core::os_endpoint over the same 1920-triple domain. The reactor's
  CloseSocket failure branch now poisons the descriptor (retry, error
  inspection, reuse and attachment refuse; strictly more refusing, no code
  change). The evutil_closesocket clause is corrected against the pinned
  source with a deliberate 4-to-8 clause-count pin update. 16 new audited
  theorems; the reactor's missing target/errno vocabulary is recorded as
  O03-U4-REACTOR-TARGET. Evidence: `docs/reports/remaining-o03-u4u5/README.md`.
- Source roots and CMake flags (G03-F): profile-manifest version 2 admits
  exactly "." as a source root (v1 spellings mean the same thing under both
  versions; traversal spellings keep their refusals at both), and the
  transported option table grows 4 to 28 exact spellings, each with a measured
  preservation argument — the seven -O levels are preprocessing-visible, so
  the archived attempt that dropped them analysed different text. The real
  Libevent submodule audits over CMake's unmodified database from
  configuration alone: 21 units, 0 Safe / 4 provisional Unsafe / 17 Unknown,
  zero option refusals, all reason codes classifying. Two-command ambiguity is
  narrowed by compile_target.object_prefix, never merged. 18 audited Lean
  theorems (`QCore.SourceRootAdmission`); CMetrics/chunkio-core artifacts
  byte-identical. Evidence: `docs/reports/remaining-g03-f/README.md`.
- Per-arm constructor discharge (O04-ARMS): the borrowed-actual admission
  widens to bounded typed access chains (every link declared exactly
  `struct P *`, re-entered in the declaration model per link, depth capped at
  4 by refusal). On byte-verbatim flb_input_chunk_create the logging arm
  discharges and four of six exits settle; C5 records its named B08 residual.
  Measured corrections to the card premise: array declarators were already
  satisfied and the borrowed callees are two, with variadic flb_error declared
  at its one call-site arity (O04-ARMS-VARIADIC, no schema change). The
  185-unit corpus was re-measured twice through the pinned route: headline
  counts and every unit verdict unchanged, diagnostics move in 4 units in the
  fail-closed direction, and the current baseline is deliberately re-pinned to
  `docs/reports/remaining-o04-arms/artifacts/` with the wave-6 record kept
  intact. 23 audited Lean theorems (`QCore.ChainedAccess`). Evidence:
  `docs/reports/remaining-o04-arms/README.md`.
- Epoch source syntax (T08-EPOCH-SRC): `source.epoch.bound.v1` lowers a
  lexical `thread scope NAME epoch { ... }` declaration onto the unedited
  FlatEpochBoundV1 checker with independent recheck; the route is check-only
  by stated grounds (an admitted epoch is an accounting statement, so no C
  emission or native run). Profile refusals are driven through the core's
  Q630-Q641; eight new reserved words with one real collision found and
  avoided (`work` became `effort` rather than editing the pinned golden
  surface). Measured schema-2 rows for CT35/CT36/CT38/CT39 are parked with
  three named harness prerequisites; CT stays 24/50. 37 audited Lean theorems
  (`QCore.EpochSourceLowering`); spec §L21's Q6xx row now enumerates the four
  concurrency sub-bands, closing the wave-3 follow-up edit. Evidence:
  `docs/reports/remaining-t08-epoch-src/README.md`.
- Confinement bounds (P03-F): the confined profile gains an enforced per-tree
  child quota — cgroup v2 pids controller under a delegated subtree, joined in
  the first pre_exec closure, with the new `max_execution_processes` declared
  kind deliberately distinct from the per-UID `max_processes`; absent
  delegation or a zero quota refuse pre-spawn. The fork-storm fixture measures
  peak 4 with one refused fork under quota 4. Execution-profile selection is
  explicit configuration with `unenforced-legacy` still the recorded default;
  CI now executes refusal grounds 1 and 4 (the userns half against a genuinely
  denying kernel) and runs the provider package build/verify, closing
  P09-lock. P02-FLAKE's assertion is byte-for-byte unchanged with a
  deterministic companion. 29 audited Lean theorems
  (`QCore.ProcessQuotaPlan`); qcore-schemas stays 6.0.0. Evidence:
  `docs/reports/remaining-p03-f/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units — the baseline
artifacts are deliberately re-pinned to O04-ARMS's after-measurement with
zero verdict movement — CT coverage stays 24/50 mapped, the contract corpus
stays 60 modules and full language conformance stays false. `qcore-schemas`
stays 6.0.0. Merged gate at this checkpoint: 1747 workspace tests (3 ignored),
58 provider tests, 108 Python tests, 42 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean, and all five provider packages rebuilt
and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
