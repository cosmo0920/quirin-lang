# Historical guidance: pointers and operational notes

[Development table of contents](../../../../DEVELOPMENT.md) · [History index](../README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
#### Short pointer bullets carried in CLAUDE.md through wave 15

*Moved verbatim from `CLAUDE.md` lines 122-124 — W2 profile migration pointer.*

- W2's bounded profile migration and full 185-TU parity are recorded in
  `docs/reports/w2-profile-manifests/README.md`; remaining H4/H5/H6 and H7
  orchestration work is explicit there. Profile data remains declared trust.

*Moved verbatim from `CLAUDE.md` lines 134-140 — W3 evidence-v2 / verifier-host pointer.*

- W3 bounded evidence-v2 replay/dispatch is recorded in
  `docs/reports/w3-evidence-v2/README.md`. Positive observation certificates
  describe recorded runs under `observations_trusted`; lifecycle source
  derivation stays incomplete until real effect/ABI import exists. The generic
  host is extracted in `crates/qcore-verifier-host` (record:
  `docs/reports/w3-verifier-host/README.md`); trusted derivation/replay/dispatch now reside
  in `qcore-verification` above the dependency-free kernel. V1 Safe refusal is unchanged.

*Moved verbatim from `CLAUDE.md` lines 142-144 — W2 H6 provider discovery pointer.*

- W2 H6 provider metadata now loads from profile packages; see
  `docs/reports/w2-provider-discovery/README.md`. This does not extract providers
  into processes or change the v1 Safe refusal.

*Moved verbatim from `CLAUDE.md` lines 146-149 — Lean CI hygiene pointer.*

Lean CI hygiene is implemented (2026-09-07): the pinned proof build, all theorem
audits and the complete QCore axiom closure are checked, with both reevaluation
examples in Linux/Windows CI. Local gates pass; hosted CI awaits a push.
See `docs/reports/w7-lean-ci/README.md`.

*Moved verbatim from `CLAUDE.md` lines 151-153 — W7 generic Lean naming pointer.*

W7 generic Lean naming is complete for the bounded targets: DescriptorCustody,
EventComposition and busy_effect_preserves_lock, with historical aliases and
unchanged models (`docs/reports/w7-generic-lean-names/README.md`).

*Moved verbatim from `CLAUDE.md` lines 163-165 — qcore-audit driver pointer.*

- The generic `qcore-audit` driver and retained Cycle19 compatibility adapter
  are documented in `docs/reports/w5-configured-audit/README.md`. No provider
  module is linked by the new driver; production evidence routes remain.

#### Claude-specific working bullets carried in CLAUDE.md through wave 15

*Moved verbatim from `CLAUDE.md` lines 81-109 — the crate-layering rule, the W4 custody-pass record and its do-not rules, and the three working rules. The imperative halves now live in `AGENTS.md` (constraint 4) and in the rewritten `CLAUDE.md`; the W4 execution record is kept here.*

- Before starting checker/architecture work, read `docs/GENERALIZATION_PLAN.md`
  and land code according to its target crate layering (§3) and invariants
  (§2); do not add new code to the monolithic grab-bag once a target crate
  exists.
- W4 is executed for parameter/return custody with validation
  (`crates/qcore-c-gateway/src/legacy_summary.rs`,
  `docs/reports/w4-custody-inference/README.md`). A call into an internal C
  function is modeled only through a declared `.qcore` contract or a summary
  that `observations_match_contract` validated against a fresh analyzer run;
  everything else stays `Unknown`. When touching that pass, do not let a
  summary be published without validation, do not seed a formal parameter with
  entry custody the body did not demand (that asserts non-nullness and deletes
  failure paths), and do not accept an assume-guarantee fixed point for a
  recursive component. The 2026-09-07 extension validates direct `Out` slots,
  `Stored` formals, `Reallocated` results and possibly-null owned returns
  (`docs/reports/w4-custody-extensions/README.md`). Reassigned out pointers,
  Direct `Removed` wrappers are inferred and validated; general removal aliases and general recursive components remain unsupported; the
  `qcore-c-gateway` relocation is executed with compiler compatibility reexports
  (`docs/reports/w4-gateway-crate/README.md`).
- Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass. If a pinned table (contract corpus counts,
  submodule revision, archived report) conflicts with your change, the change
  must update the pin deliberately and say so.
- Status language matters in this project: keep the Safe/Unsafe/Unknown
  verdict vocabulary and the "unpromoted" phrasing exactly as defined in
  AGENTS.md when editing docs, reports, or commit messages.
- Run the full standing per-change gate from AGENTS.md before reporting a
  task complete; report probe/example exit codes honestly (a nonzero probe
  exit is a finding, not a build failure).
<!-- migrated-section:end -->
