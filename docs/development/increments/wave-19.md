# Wave 19 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-19 remainder increment (2026-09-10): the six-card spine closed through
delegated worktrees under the failure-gated chain, composed with three
Windows-validation batches from the user's host by ordinary merges. Five
cards are corpus-neutral by construction; the sixth measured the corpus in
both directions and moved exactly two attributed leaves, so the 185-unit
baseline stays `docs/reports/wave17-corpus/` unmoved: 2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only, leak 272, CT 33/50.
Contract corpus 70 modules. Nothing is promoted.
- Scalar results (PIPE-SCALAR-RESULT): `let n = f(x);` binds a
  non-acquiring call's custody-free scalar result and a scalar local
  passes as a `Data` actual, in a `ValueId` namespace beside `ResourceId`
  that nothing converts; the acquisition adapter stays the one spelling of
  an acquisition. Evidence: `docs/reports/wave19-pipe-scalar-result/README.md`.
- Branching (IF-STATEMENT): `source.branch.v1` — `if`/`else`, a decidable
  condition tier, custody-join agreement (band Q044–Q046, sequential route
  only, zero new reserved words). The census corrected wave 18's premise:
  five of six semantic models already branched; only the cleanup lowering
  lacked a branch constructor, so both arms flow through the unedited
  checkers. Evidence: `docs/reports/wave19-if-statement/README.md`.
- Named waivers (UNSAFE-LINKAGE-ARTIFACT): level L1 of the linkage
  spectrum — a manifest-resident waiver publishes "unchecked X at ABI S
  from artifact Y@hash under stated ABI premises" over a total-or-reject
  manifest; the identity is a pin, not a probe; header disagreement
  refuses rather than withdraws (band Q039–Q042, no schemas bump).
  Evidence: `docs/reports/wave19-unsafe-linkage-artifact/README.md`.
- Typedef reader (CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF): the declared-ABI
  reader commits to function-pointer and aggregate-definition typedefs
  (`fnptr` normative in §C04.1, outside every custody admission), and the
  card's own premise is REFUTED by its measurement: the constructor stays
  `Unknown` — the ABI withdrawal was stacked on the importer's
  address-of-member refusal (residual `ABI-TYPEDEF-ADDRESS-OF-MEMBER`).
  BEFORE reproduced the wave-17 verdict artifact byte-identically; the
  msgpack contract measured zero payoff and is deliberately not landed.
  Evidence: `docs/reports/wave19-abi-typedef/README.md`.
- Operators (OR-AND-LOWERING): the operator table is superseded by user
  direction — logical or is `||`, logical and `&&`, both short-circuit,
  lowered onto the branch construct with byte-identical C against
  hand-lowered branches; single `|`/`&`/`^` are reserved for bitwise with
  actionable refusals, resolving the open bitwise-OR-symbol note. No new
  diagnostic code. Evidence: `docs/reports/wave19-or-and-lowering/README.md`.
- Route composition (STD-IO-COMPOSE): the extern and std-io routes compose
  as a checked model union selected after every concurrent route (Q043,
  `ComposedStdIo`); `EXTERN-SCALAR-LITERAL` lands, and a declared
  `succeeds INTEGER` outcome is verified by the emitted program rather
  than believed. The literal `value |> render |> log` chain compiles,
  emits its hand-written twin byte-identically and byte-compares per
  stream sanitized. Evidence: `docs/reports/wave19-std-io-compose/README.md`.
Merged gate at this checkpoint: 3,134 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 85 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).


Wave-19 partial increment (2026-09-10): two user-directed cards landed
through delegated worktrees and were composed with two Windows-validation
batches pushed concurrently from the maintainer's Windows host (the
non-fast-forward chain merged them; one fmt/clippy hygiene repair on a
Windows fixture was applied at integration with test behavior unchanged).
Corpus-neutral by construction; the baseline stays
`docs/reports/wave17-corpus/`. Nothing is promoted.
- Stream pipelines (PIPE-CHAIN): multi-stage `|>` lands as
  statement-sequence desugaring with hygienic per-stage temporaries —
  custody flows through the unedited checkers, so pipelines inherit
  linearity, leak and use-after-move checking; `PIPE-LET` closes; the
  wave-18 span blocker is retired by measurement (consecutive stage extents
  provably disjoint). `x |> f(a) |> g(b)` composes with real custody; sinks
  are final-stage-only; an unsafe extern stage deposits its
  UncheckedLinkage assumption through a pipeline. The literal
  `value |> render |> log` stays gated on two pre-existing measured
  boundaries recorded as PIPE-SCALAR-RESULT and STD-IO-COMPOSE.
  21 new audited theorems (`QCore.PipeDesugaring` 35 → 56). Evidence:
  `docs/reports/wave19-pipe-chain/README.md`.
- One printing route (TRACE-STDIO): all 46 compiler-synthesized print sites
  across the five C backends migrate onto the single
  `QCORE_IO_WRITE`/`QCORE_IO_FLUSH` route `std.io` lowers to, with no
  printed byte moved — every line-for-line reader keeps its evidence, the
  F02 emission digests are deliberately re-pinned with the relation
  re-verified, and 23 audited theorems (`QCore.SingleEmissionRoute`)
  characterize the single-route property both ways. The census corrected
  the premise: `examples/threads` verified nothing before and now carries
  five tests (compiled under -Werror, run sanitized, byte-compared against
  reference execution, README transcript checked against a real run); a
  source-level `print` in a threaded module is measured impossible and
  pinned. Windows residuals recorded (TRACE-STDIO-WIN-DIGEST/-WIN-RUN).
  Evidence: `docs/reports/wave19-trace-stdio/README.md`.
Merged gate at this checkpoint: 2,986 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 80 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. The remaining wave-19 spine —
a source branching construct, the or/and lowerings onto it, and the
scalar-result binding gap — is recorded in the follow-on ledger.


<!-- migrated-section:end -->
