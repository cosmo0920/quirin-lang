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

Removal-only C helpers additionally export ordered formal-index traces in the
same effect-bearing channel. Their complete CFG must be one acyclic path of
removal calls and exits, with borrowed/none custody, exact unrebound formal
operands and typed nonvolatile/non-atomic parameter slots. No extra call,
member/scalar write, branch, alias, release or mixed effect class supplies a
heap frame. Ordinary W4 publication rejects removals; the richer hook applies
them in source order against the actual caller's records. Source replay checks
the trace and refuses omitted/reversed/erased effects or pre-seeded local
inference tables. Local inference must be reconstructed from the unit itself.

Cross-unit S3 records use `ResourceContracts.lifetime_imports`, separate from
ordinary borrowed imports and local inference. The public schema moves from 22.0.0 to 23.0.0 with matching path dependencies
and locks. The new effect-bearing summary-record field must not be projected
as an ordinary borrowed record by an old reader. Empty fields remain omitted;
that encoding convenience does not turn this into a minor compatibility change. Export requires a Safe external home and validated summary. Independent
admission reconstructs its selected scope, source/request/environment identities,
link premise, exact contract and exact lifetime effects. Caller substitution
retains the named protocol premise and source-qualified registration. Missing,
changed, duplicate or chained homes/effects refuse. Unaware custody/lookup/frame
routes continue to withhold these imports.

The registered `OrderedLifetimeReplay` Lean audit currently passes 14 theorem
checks: complete sequential composition, sticky refusal, exact/poisoned/unmatched
removal controls, import effect/identity equality, missing-home/empty-projection
refusal, identical-trace joins and complete bounded admission. Source extraction
and model-to-Rust correspondence remain review premises. Rust controls cover
model composition, exact substitution, conditional/source linkage boundaries,
and independent home/caller certificate replay with altered-evidence controls.

The initial focused Lean build failed on the reserved `prefix` binder and a
nonexistent theorem accessor; corrected runs pass without weakening theorems.
Private logs: `private/ordered-lifetime/lean*.log`. The initial focused Rust
run passed the three existing cross-unit controls and the new forwarding/replay
path, then failed a new expectation that removal after `starts(c)` would be
Safe. That expectation lacked the existing member-holder snapshot frame:
passing the record whole is deliberately outside `P-HOLDER-PATH-STABLE`.
The control now explicitly expects this refusal; no existing pin was weakened.
Ordered model/source-helper controls and cross-unit outgoing-effect replay
remain separate capabilities. A checked whole-record holder frame is an open
prerequisite for that additional caller-removal admission. Submodule initialization first
failed under sandbox DNS restrictions; host initialization succeeded at the
pinned d82464a277d0f42703702c4dfd9af6af38595a83 revision. Focused Rust controls,
The original focused batch passed seven tests and the audit-selector control
passed independently. The first removal-carrier batch passed 10 tests and
failed one member-actual admission; the explicit effect-aware argument reader
was extended, and the corrected batch passed all 12 tests, including actual
C caller order, early-release, poison and cross-unit trace tamper controls.
Typed volatile-slot and local-seed controls were added afterward and await the
final run. Initial carrier compilation failed on module paths and the optional
formal-name inventory; corrected compilation and Clippy pass. The full
source/provider gate and native comparison remain pending here.

Collector-by-id identity, conditional record/instance correlation, derived
loop-path equality, general all-path threaded join/destruction, descriptor
custody and automatic backend contract-body validation remain open. The existing
FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME tracker will be amended, not retired.
