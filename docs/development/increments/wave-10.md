# Wave 10 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-10 delegated increment (2026-09-09): seven bounded follow-on cards landed
through delegated worktrees at the doubled-throughput fleet (launched eleven,
trimmed to seven when worktree storage became the binding constraint — the
four stopped cards defer to wave 11 with their briefs unchanged), merged onto
main under the standing gate and closed by one integrator corpus
consolidation; this paragraph supersedes nothing above and each card's record
keeps its own boundaries.
- Reactor close-failure vocabulary (O03-REACTOR): CloseSocket failures may
  carry a declared (target, errno) pair resolved total-or-reject; the reactor
  answers with the os_endpoint relation's classification — POSIX-2024 EINTR
  retains with one retry (the only such row in the 4×8×3 domain, proved),
  releasing Linux classes retire the number, everything unpinned stays
  Poisoned. The two-sided digest grows to 2016 rows with Rust and Lean
  agreeing independently. Nine audited theorems. `qcore-compiler` 0.4.0.
  Evidence: `docs/reports/remaining-o03-reactor/README.md`.
- Cleanup-outcome vocabulary (P04-B1): both cleanup-outcome contracts declare
  the referents their profile binds under `cleanup_vocabulary_v1`, bound
  structurally against pinned source (13 source-binding, 17 profile and 20
  grammar refusals; clause counts 16/16/12 → 26/29/25, corpus stays 60
  modules). 37 audited Lean theorems (`QCore.CleanupVocabulary`). The only
  corpus movement of the wave: 18 contract_source leaves. Evidence:
  `docs/reports/remaining-p04-b1/README.md`.
- Loader and TOCTOU (P01-F3): no unverified provider path remains reachable
  from production (two real holes closed: discover_packages registered a
  package with no executable at all, and the compiler's first-party registry
  built providers from metadata alone; compile-fail pins); the
  verify-to-spawn window narrows from unbounded to a measured 0.5–24 ms via
  held-descriptor re-read plus (device, inode) identity, with the residual
  PROVED open in Lean rather than claimed closed, and a self-caught deadline
  regression repaired. 24 audited theorems (`QCore.PackageIdentity`);
  `qcore-verifier-host` 0.3.0. Evidence:
  `docs/reports/remaining-p01-f3/README.md`.
- Lock ranks (CT33-RANKS): the recorded CT33 blocker was measured incomplete
  — nested `with lock` was itself refused — so both moved together: `mutex
  rank N` declarations, an opt-in strict-order discipline (new Q624/Q625
  through the §L21 process; the mutex band is Q610–Q625) and rank-gated
  nesting, with unranked modules byte-identical. A rank is a declared
  position, not an assumption. 12 audited theorems; CT33's mapping is
  proposed with the measured [Q624,Q612] cascade for the next CT lane.
  Evidence: `docs/reports/remaining-ct33-ranks/README.md`.
- Composition residuals (E03-B-O): an absent provider's obligations now
  derive and refuse on absence (ObservationHooks withholds its vacuous
  inventory — E03-B-O1-HOOKS); composition is grouping-invariant on the
  whole outcome; BoundedPaths composes through the real firewall with a
  census tripwire for the still-unemitted count kinds. `Composition` grows
  to 61 audited theorems; `qcore-verification` 0.3.0. Evidence:
  `docs/reports/remaining-e03-b-o/README.md`.
- Extern out/stored modes (EXTERN-F1): `resource_out(nullable)` acquires on
  the success arm only (an unwritten slot is observed as failure and
  refuted, not believed); `resource_stored` consumes permanently under the
  declared owner with `resource_owned` admitted in exactly that position per
  §C04.2. Measured: the stored mode is checked but unreachable from a single
  source module — the abandonment gate refuses a second live acquisition and
  was deliberately not weakened (EXTERN-F1-STORE-REACH). `QCore.ExternCall`
  grows to 48 audited theorems; two new sanitized native fixtures. Evidence:
  `docs/reports/extern-call-f1/README.md`.
- Bound-base reads (O04-READ-BOUND): a member read through a base bound by an
  admitted aggregate store is admitted as a whole Borrowed actual with the
  linked children's liveness required (the released-child admission and a
  call-index span confusion were caught and closed during implementation);
  the READ-BOUND-BASE blocker falls and C6's residual narrows ten to nine
  refusals — C6 itself stays Unknown. All five corpus artifacts
  byte-identical. 44 audited theorems (`QCore.BoundFieldRead`). Evidence:
  `docs/reports/remaining-o04-read-bound/README.md`.
- Corpus consolidation: one integrator measurement on the merged tree
  (`docs/reports/wave10-corpus/`) re-pins the baseline — 2 Safe /
  42 provisional Unsafe / 137 Unknown / 4 data-only unchanged, and the only
  movement against wave 9 is P04-B1's 18 contract_source leaves, attributed
  pair by pair. The acceptance matrix regenerates whole-file at the merged
  numbers.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-10 baseline, CT coverage stays 30/50 mapped, the contract
corpus stays 60 modules and full language conformance stays false.
`qcore-schemas` stays 8.0.0; `qcore-compiler` is 0.4.0; `qcore-verifier-host`
and `qcore-verification` are 0.3.0. Merged gate at this checkpoint: 2034
workspace tests (4 ignored), 76 provider tests, 125 Python tests, 51 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean, and all five
provider packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

<!-- migrated-section:end -->
