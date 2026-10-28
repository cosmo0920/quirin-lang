# Ordered removal summaries and cross-unit lifetime replay

Implementation is in progress on `codex/event-ordered-lifetime`, based on
caller-removal merge c265a31fa. The ex ante forecast is in [PREDICTION.md](PREDICTION.md)
and scope/residuals in [BRIEF.md](BRIEF.md). This report does not yet claim a
completed standing gate or native measurement.

Complete ordered traces retain up to 4096 primitive register/remove/destroy/release
steps. Call composition substitutes exact source-qualified paths and replays
against incoming registrations, preserving poison and earlier release obligations.
Only identical exit traces retain ordered discharge; unresolved/lifted actuals,
unread effects, unsupported control flow and unstable holder snapshots retain
the conservative may/must summary. Same-unit ordered transport requires an
internally linked helper. The old unordered removal set never supplies order.

Cross-unit S3 records use `ResourceContracts.lifetime_imports`, separate from
ordinary borrowed imports and local inference. The public schema grows additively
from 22.0.0 to 22.1.0 with an omitted-when-empty field and matching path dependencies
and locks. Export requires a Safe external home and validated summary. Independent
admission reconstructs its selected scope, source/request/environment identities,
link premise, exact contract and exact lifetime effects. Caller substitution
retains the named protocol premise and source-qualified registration. Missing,
changed, duplicate or chained homes/effects refuse. Unaware custody/lookup/frame
routes continue to withhold these imports.

The registered `OrderedLifetimeReplay` Lean audit currently passes 12 theorem
checks: complete sequential composition, sticky refusal, exact/poisoned/unmatched
removal controls, import effect/identity equality, missing-home/empty-projection
refusal, identical-trace joins and complete bounded admission. Source extraction
and model-to-Rust correspondence remain review premises. Rust controls cover
model composition, exact substitution, conditional/source linkage boundaries,
and independent home/caller certificate replay with altered-evidence controls.

The initial focused Lean build failed on the reserved `prefix` binder and a
nonexistent theorem accessor; corrected runs pass without weakening theorems.
Private logs: `private/ordered-lifetime/lean*.log`. Submodule initialization first
failed under sandbox DNS restrictions; host initialization succeeded at the
pinned d82464a277d0f42703702c4dfd9af6af38595a83 revision. Focused Rust controls,
full source/provider gate and native comparison remain pending here.

Collector-by-id identity, conditional record/instance correlation, derived
loop-path equality, general all-path threaded join/destruction, descriptor
custody and automatic backend contract-body validation remain open. The existing
FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME tracker will be amended, not retired.
