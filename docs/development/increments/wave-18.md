# Wave 18 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-18 delegated increment (2026-09-10): three user-directed cards landed
through delegated worktrees, merged onto main under the failure-gated chain
with one integration-time diagnostic-band relocation; this paragraph
supersedes nothing above and each card's record keeps its own boundaries.
Wave 18 is corpus-neutral by construction — the gateway, audit, contract and
provider surfaces are byte-untouched against the wave-17 tree, so the
185-unit baseline stays `docs/reports/wave17-corpus/` unmoved: 2 Safe /
42 provisional Unsafe / 137 Unknown / 4 data-only, leak 272, corpus 69
modules, CT 33/50, `qcore-schemas` 14.0.0. Nothing is promoted.
- Unchecked external linkage (UNSAFE-LINKAGE): `source.extern-call.unsafe.v1`
  lands — an external library with no contract at all links end to end behind
  `import std.extern.unsafe;` and `unsafe extern fn`, scalars by value in and
  a scalar or unit out, every pointer refused by name (an assumption that
  cannot be stated cannot be carried, so no custody crosses the boundary).
  `ExternAssumption::UncheckedLinkage { callee, signature }` rides the
  existing declared-assumption channel with three independent omission guards
  and a Lean totality theorem; `qcorec check` prints the premises, so a Safe
  obtained this way is never displayed bare. The Q023 default is pinned
  unmoved; the new `UncheckedLinkage` pass owns Q032–Q035 (spec §L21). The
  prior workflow's `NATIVE_LIBRARY_LINKAGE_V1.md` is adopted with provenance
  and reconciled as one rule: pointers cross exactly where a contract exists
  to state their premise; the manifest-resident artifact-identity entry is
  staged as UNSAFE-LINKAGE-ARTIFACT. 33 audited theorems, a fixed sanitizer
  run linking real uncontracted C. Evidence:
  `docs/reports/wave18-unsafe-linkage/README.md`.
- Proof-phase shortening (LEAN-PERF): the audit gate drops cold 121.4 s to
  61.0 s and incremental 36.5 s to 4.2 s with nothing proved changed — no
  theorem statement, domain, digest or pinned number moved, `native_decide`
  not introduced, the axiom closure pinned before/after. TaskTraceRelation
  shares one `cbv` expansion of its 3,025-trace domain (its residual ~27 s
  named the cost of the D3 claim); CapacityBoundary decides one fused
  predicate; `decide +kernel` was applied only where measured to help and
  reverted where it regressed. The audit phase parallelises behind a
  still-serialised build; there is deliberately no cross-worktree cache and
  CI stays cold, which is the recorded trust argument. Evidence:
  `docs/reports/wave18-lean-perf/README.md`.
- Pipe and the operator table (PIPE-OPERATOR): `|>` lands as a pure
  token-stream desugaring (first-argument insertion, left-associative) with
  byte-identical emitted C against the hand-written twin and linearity
  through pipelines inherited from the unedited checkers; a pre-existing
  divergent re-tokenization path in `source_extern` was found and repaired.
  Two measurements bound the rest: QCore has no `if` statement (Q500, zero
  occurrences in 294 source files) and no boolean expression tier, so the
  logical operators land as claimed tokens with actionable refusals — by
  decision, or is single `|` and and is `&&`, with `||` and a logical-position
  lone `&` redirecting, bare `&` reserved for a future binary operation, and
  bitwise-or's missing symbol flagged open, not answered. The Pipe pass owns
  Q036–Q038 after an integration-time band relocation (UncheckedLinkage
  shipped Q032–Q035 first; the T07/T09 precedent), and the adopted
  native-library design note moves its reservation to Q039+, closing a latent
  third collision. Multi-stage chains (PIPE-CHAIN) and the or/and lowerings
  wait on a source branching construct — the named wave-19 prerequisite.
  35 audited theorems. Evidence: `docs/reports/wave18-pipe-operator/README.md`.
Merged gate at this checkpoint: 2,959 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 79 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).


<!-- migrated-section:end -->
