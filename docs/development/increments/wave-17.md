# Wave 17 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-17 delegated increment (2026-09-10): eight bounded follow-on cards landed
through delegated worktrees, merged onto main under the verified failure-gated
chain discipline and closed by one integrator corpus consolidation. This
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- The constructor's last two roots (CONSTRUCTOR-RESIDUALS): card
  ALIAS-AGGREGATE's two reasoned refusals are dispositioned, one by
  implementation and one by correcting its own premise. **ERRNO**: a
  dereference of a neutral call's result — `errno`, spelled
  `(*__errno_location ())` after preprocessing — is admitted as one outer
  argument under a **refined** proved sentence rather than a violated one,
  because card W4-EXPR-POS's *no pointer-valued operand* cannot reach a
  pointer-valued call; the replacement is stated over what the projection
  tracks (an admitted dereference *binds, releases, escapes and addresses
  nothing*), its four conjuncts discharged by N6, N7, N9 and a new
  scalar-pointee N10. `__errno_location` and `flb_errno_print` are declared
  against read bodies; measured reach is 18 sites for the admission and 24
  more for the declaration alone, across 13 units. **PUBLICATION**: measured,
  `mk_list_add` needs no publication mode at all — card HASH-CONTRACTS had
  already settled the question and the read body frees nothing and moves no
  obligation, so plain `resource_borrowed` is faithful and the retention stays
  unmodeled (B03/R13). The real blocker on the two statements the card could
  not admit is card LOADER-ABI's declared-type reader (a function-pointer
  typedef; a union typedef), named `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`, and a
  withdrawal experiment pins what remains: without those three statements the
  constructor reports **0** diagnostics and discharges all six exits. It moves
  **11 → 6** diagnostics with **5 of 6** exits discharging and stays
  `Unknown`. One **pre-existing fail-open** was found and repaired: a released
  identity at an admitted *nested* call's borrowed actual was `Safe` with no
  diagnostic, because card BORROW-LIVENESS's precondition read only the
  enclosing call's actuals; the repair is a set inclusion and
  `C02-P-ALIAS-LEAK-ORDER` is undisturbed by construction. 29 audited theorems
  (`QCore.DereferencedResult`); contract corpus **67 → 69** by two deliberate
  additions. Evidence:
  `docs/reports/remaining-constructor-residuals/README.md`.
- Declared/derived custody-mode agreement (LOADER-ABI-MODES): a declared
  contract whose custody **duty** the validated W4 summary of the callee's own
  body in the analysed unit *contradicts* is now withdrawn for that unit —
  every call site refusing with `mode-parameter-contradiction` or
  `mode-result-contradiction` — closing the fail-open card LOADER-ABI
  recorded, where types were compared against the source and modes against
  nothing. The evidence was already computed and already reported
  (`SummaryConflict` makes the *defining* function `Unknown`) while the
  declaration went on governing every call site; **three `Safe` callers**
  resting on that — a double release, a hidden leak and a discarded
  acquisition — are reproduced and closed. The relation is proved to *be* the
  duty inequality rather than sampled, and deliberately stops short of
  refinements inside one duty: that boundary is not academic, because the
  census measures **every** derivable comparison in the pinned corpus as a
  nullability refinement in three production families (113 rows), so
  withdrawing on inequality would have withdrawn a release contract in every
  unit including those headers. Withdrawal never installs the derived summary
  in the declaration's place, an inferred summary being possibly weaker than
  the declaration it convicts; a fail-closed pin from card C03-NFR caught the
  first implementation destroying its own evidence and was repaired rather
  than weakened. Census: 598 undefined / 484 no-summary / 113 disagreed /
  **0 contradicted** over all 185 units, of which 8 rows are inside the
  audit's analysed function set and all 8 read `no-summary` — so the corpus
  was measured in **both** directions and all five artifacts are byte-identical
  between them. 36 audited theorems (`QCore.DeclaredModeAgreement`); contract
  spec §C04.1 gains the mode-agreement requirement. Evidence:
  `docs/reports/remaining-loader-abi-modes/README.md`.
- Publication residuals (CTWIN-RESIDUALS): the four shapes card CTWIN-PUBLISH
  left open are measured and closed, and the brief's soundness ordering is
  **refuted by measurement** — INDIRECT, named the likeliest launder, is the
  one shape with *no* false `Safe` at all (nine indirect shapes probed, all
  already refused), while three others each carried a live publish-then-free
  `Safe` with no diagnostic. The sharpest, `CTWIN-PUBLISH-BORROWED`, is not the
  lifetime question wave 16 supposed but a custody launder across the
  **summary** boundary: a helper publishing a borrowed formal certified, so W4
  published `Borrowed` for it and a caller that acquired, called the helper and
  released certified too. The exclusion is withdrawn, on the argument that
  `Borrowed` asserts the caller's obligations are unchanged and a publication
  falsifies exactly that, with both faithful alternatives proved to promote in
  the mirror direction. `CTWIN-PUBLISH-OUT` is withdrawn the same way — a
  declared `Out` slot naming such an object is the `Acquire` arm seen from the
  callee's side — and a latent false `Unsafe` closes with it.
  `CTWIN-PUBLISH-INDIRECT` has no launder to close, so its residual
  `violation` **claim** is withdrawn instead, each statement keeping the
  refusal it already had. `CTWIN-PUBLISH-SCAN` narrows: `_Atomic ( type-name )`
  is a specifier, not a declarator, and the parenthesised spelling — CT40's
  literal one — *was* hiding its destination, correcting CTWIN-PUBLISH's
  "`_Atomic` is not load-bearing" note, which had measured only the postfix
  spelling; monotonicity is preserved and proved. Two theorems and one test
  are retired by name and one C03 pin moved earlier, each keeping its
  load-bearing half. `QCore.PublicationCustody` grows 30 → **50** audited
  theorems. Corpus reach is **zero**, measured twice. Evidence:
  `docs/reports/remaining-ctwin-residuals/README.md`.
- Extern released and reallocated modes (EXTERN-MODES): two of the five open
  `source.extern-call.v1` call modes become callable, each at a measured
  boundary. `resource_released(p, nonnull)` is the mode the error arm was
  already describing — §C04.2 separates discharging an obligation from
  transferring it, and card EXTERN-ERR-REL recorded that the profile could
  spell only the transfer — so the caller-side ledger delta is unchanged and
  nothing about leak accounting moves, but a release is now published as a
  release and premised by name. `resource_return_reallocated(p, nullable)`
  over an `Owned` subject is the one departing argument an acquiring call may
  carry, because §C04.3 states its failure arm where ordinary declarations are
  silent; the subject therefore stays live on the error arm and must be
  dispositioned there, which is the unweakened abandonment rule.
  `resource_removed` stays refused on a **measured ground** — a removal names a
  child the profile stopped tracking when it was stored, and card EXTERN-F1
  premised that residual rather than modelling an index, so admitting it would
  mint a caller identity out of a premise — and the sentinel/errno adapter is
  recorded as a design boundary with its three constraints. No diagnostic code
  and **no reserved word** were added: `move NAME` spells all four departing
  modes and the declaration decides which ran. Two Lean theorems retired by
  name with their load-bearing halves kept; three new fixed ASan/UBSan/LSan
  fixtures link a real C `free` and a real `realloc`, including the preserving
  arm where a double free would be reported. Evidence:
  `docs/reports/remaining-extern-modes/README.md`.
- ChunkIO symbolic other-backend classification (P04-COMPOSE-UNSUPPORTED): the
  fifth and **last reachable** composition module moves through the process
  boundary, closing the ladder. It is the first family that imports **no body
  of its own** — its whole question is the nested filesystem question, verbatim
  — the deepest question the vocabulary carries (four composition levels, all
  re-established by the core), and the only one whose published conclusion is
  an explicit `Unknown` that no proposal can move, proved as such. Its reading
  is over a **class**, not a value: the excluded class is fixed jointly by the
  two probes *and the wrapper's own guards*, and a claimed class that leaves a
  guard undecided is refused rather than sampled — a measured correction to the
  card's own first draft, caught by a failing row and kept. Parity with the
  retained in-process route is byte-identical on the serialized certificate
  across three excluded-constant pairs; a 69-row core mutation matrix, a 31-row
  provider matrix and 12 route-boundary refusal rows all refuse, including both
  masquerade directions and a falsified prerequisite contract *two* levels
  down. Three enum variants make `qcore-schemas` **14.0.0** with existing wire
  bytes and every existing task digest unchanged; ChunkIO package **0.13.0**;
  37 audited theorems (`QCore.SymbolicClassAdmission`). Two modules stay behind
  the sealed `SourceMatch` boundary, re-measured rather than assumed: they need
  a reviewed-body attestation the core can re-derive, not a bigger vocabulary.
  Its headline boundary is named rather than blurred
  (`P04-COMPOSE-UNSUPPORTED-REACHABILITY`): the route establishes what happens
  *if* a chunk's stream carries a discriminant outside both constants, and
  **nothing** about whether any caller can produce one. Evidence:
  `docs/reports/remaining-p04-compose-unsupported/README.md`.
- Standard output modules (STD-MODULES): the census contradicts the card's
  premise and is the result — QCore source has **no** printing, logging,
  formatting or IO construct and never had one (21 statement kinds, none
  writing; no string or char type; zero string and character literals outside
  comments across 290 source-side files), so the migrated implicit-use count is
  **0**. What looked implicit is two other things, both recorded:
  compiler-synthesized trace instrumentation in four C backends, read back for
  translation validation, and an `Effect::Io` label stamped on every extern
  call regardless of what the callee does — a capability *name* with nothing
  gating it. With nothing to migrate, `std.fmt`, `std.io` and `std.logger` are
  **provided** instead, as `source.std-io.v1`: two renderings, a line to
  stdout, a level-tagged line to stderr and a `flush` barrier, each gated by
  its own module through the existing `Q014`/`Q015` and refused without it,
  with `Q028`–`Q031` under a new `StandardIo` pass through the §L21 process.
  The v1 effect discipline is **structural rather than checked**: the operand
  type has two constructors and neither can denote a resource, so a rendering
  cannot consume, release, store or retain custody, and the derived row is
  exactly `{Io}`. Two sanitized native runs match the reference **per stream**;
  no interleaving between the two streams is modeled or claimed. The
  amendment's asynchronous logger is refused on a **measured** blocker rather
  than faked — the task ledger's only parent-to-child custody edge is at
  creation and the checked mutex backend emits one thread, so a bounded queue
  cannot be modeled honestly — and only the barrier half landed, as sink
  handoff (`fflush`), explicitly not durability. `error` was measured to
  collide with the reserved error-type keyword, so the severity is spelled
  `fatal` (the `work` → `effort` precedent). 45 audited theorems
  (`QCore.StdIo`), no new axiom. Evidence:
  `docs/reports/wave17-std-modules/README.md`.
- Wave-17 CT lane (CT40-LANE): **CT40 maps on the `legacy_c` route, moving CT
  coverage 32/50 → 33/50**, after card CTWIN-PUBLISH closed the gateway hazard
  that had left it unmapped. Every parked candidate row was re-measured on the
  current tree's fixture bytes and two corrections found: the parked diagnostic
  string used `unknown_alias` where the harness renders `unknown-alias`, and
  one control was not enough. The mapping rests on the construct CT40's
  `expected` text names — the publication protocol, which the gateway names
  verbatim in its own diagnostic — and explicitly **not** on the two its title
  names: `_Atomic` is pinned non-load-bearing by a control byte-identical to
  the case, and "prematurely freed" by a control that publishes with no release
  and is refused just the same. `native_sanitized` is the structural
  `not_applicable` its route prescribes; sanitized-native stays 4 and proof 0,
  so the grown denominator is mapping, not conformance. CT41 and CT42 stay
  unmapped with blockers **re-measured** rather than inherited. One mechanism
  change: mapping a case whose bucket an earlier lane recorded no longer
  requires editing that lane's archived record — the mapping lane declares a
  `retired_unmapped_reasons` row in its own, checked total-or-reject in both
  derivations across seven refusal directions, so the wave-9 record stays
  byte-identical. 24 scratch mutations, all refused. Evidence:
  `docs/reports/wave17-ct40/README.md`.
- Ledger and archive hygiene (GEN-CTREASONS): the CT ledger's
  `newly_decidable_not_mapped_reasons` map is **derived** rather than carried,
  closing `D02-GEN-CTREASONS`, which HYGIENE-16 had recorded as *not*
  derivable. Measured, the map is the companion prose of a list the generator
  already derives, so its keys are that list's ids by construction — which
  makes both staleness classes structurally impossible rather than guarded, and
  the second was found by doing it: beside HYGIENE-16's CT39 row the committed
  map carried a **CT11** row explaining a case its own published list does not
  name, invisible to the wave-16 carry guard. The carry step
  `readd_ct_reasons.py` now retires itself loudly through its own tripwire,
  asserted by name, with its archived bytes untouched. 15 audited theorems
  (`QCore.CtReasons`). `DIFF-TOOL-CONTRACTS-BACKFILL` closes over the whole
  schema-1 consolidation chain: seven archived diff documents gain a
  `contracts` section regenerated from their own committed artifact pairs,
  append-only behind a `backfilled_sections` record and verified three ways,
  including against the pre-backfill blobs in git. Each document is its own
  **oracle** — the four entries it already carried must reproduce
  byte-identically or nothing is written — and that refusal is exercised. The
  card premise is corrected by measurement (the wave-16 diff already carried
  the section), and the backfill supplies what the hand comparisons could not:
  HASH-CLAUSE's *content* change in wave 13 → 14, and a measured zero contract
  movement in waves 9 → 13. Evidence:
  `docs/reports/wave17-gen-ctreasons/README.md`.

- Corpus consolidation: one integrator measurement
  (`docs/reports/wave17-corpus/`) re-pins the baseline. Three of the eight
  cards touched the gateway's decision surface and **only one moved a leaf**,
  so the attribution is by identity rather than by argument: the composed
  `diagnostics.json` is **byte-identical** to CONSTRUCTOR-RESIDUALS' archived
  AFTER, and `triangulate.py` asserts on committed bytes that the composed diff
  (12 added / 60 removed across 21 units) equals that card's exactly, with zero
  unattributed leaves — which is also what makes CTWIN-RESIDUALS' and
  LOADER-ABI-MODES' zero-reach claims hold on the composed tree. The composed
  `flb_input_chunk_create` count is **measured at 6** with 5 of 6 exits
  discharging, and it stays `Unknown`. **One pinned number moves deliberately:
  leak 271 → 272**, at `flb_input_chunk_get_direct_routes` L19399 — a movement
  in the *accusing* direction on a function and a unit that were already
  `Unsafe`, the seventh instance of a claim the same function already published
  six times, surfaced because a neighbouring refusal that hid it was admitted.
  It is not a promotion, it stays provisional in the documented sense, and it
  is recorded as `CONSTRUCTOR-RESIDUALS-LEAK-271`; standing constraint 7 is
  updated to 272 with that record named. `chunk-contracts.json` is
  byte-identical; `contracts.json` moves by exactly the two added modules and
  `logging.qcore`'s changed text, verified through HYGIENE-16's itemizer; the
  48 `source-effects` leaves and 17 `source.json` leaves are the two recorded
  residue classes, checked leaf by leaf by a new `residues.py` that reproduces
  its transcript from committed bytes, with every per-result status equal and
  the sidecar split unchanged at 3 checked / 93 Unknown. `named_followups`
  moves 26 → 27; `AGENTS.md`'s standing constraints stay 28, six of them
  extended in place (the leak count, the declared-contract agreement rule now
  covering modes as well as types, the publication rule's withdrawn borrowed
  and `Out` admissions, the reserved-word inventory, the archive-diff and CT
  rules, and the generator's now-derived reasons map). One
  pre-existing divergence is recorded rather than repaired
  (`D02-B00-LEDGER-SNAPSHOT`): the matrix's `contract_modules` derives from
  disk at 69 while `contract_modules_per_directory` and `contract_clause_pin`
  derive verbatim from card B00's **frozen** ledger and still describe the
  59-module checkpoint — constraint 21 working as designed, but a snapshot
  published beside a live count with nothing saying so.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-17 baseline, with zero unit and zero function status moves in
either direction and no newly surfaced provisional `Unsafe`; the single
diagnostic-level movement is the deliberate leak 271 → 272 above, in the
refusing direction. The contract corpus is **69 modules** by
CONSTRUCTOR-RESIDUALS' two deliberate additions and full language conformance
stays false. CT coverage advances to **33/50 mapped**, by executed-stage
evidence only, with sanitized-native 4 of 33 and proof 0 of 33 — a grown
denominator is mapping, not conformance. `qcore-schemas` is **14.0.0** and the
ChunkIO provider package **0.13.0**; `qcore-compiler`, `qcore-verifier-host`
and `qcore-verification` stay 0.4.0. Three fail-opens were found and repaired
this wave, each measured at base before it was closed: a released identity at
an admitted *nested* call's borrowed actual (CONSTRUCTOR-RESIDUALS), three
`Safe` callers resting on a declared mode the callee's own body contradicts
(LOADER-ABI-MODES), and the publication launder across the summary boundary
that `Borrowed` and declared `Out` slots were carrying (CTWIN-RESIDUALS).
Merged gate at this checkpoint: 2,902 workspace tests (8 ignored), 122 provider
tests, 186 Python tests, 77 Lean audits, both reevaluation examples exit 0, fmt
and clippy clean in both workspaces, and all five provider packages rebuilt and
verified at schemas 14.0.0 / ChunkIO 0.13.0. One environment-class failure is
reported honestly: the first providers run failed
`real_packages_preserve_unknown_and_reject_mutations` with
`ExecutableFileBusy` — a sibling test in the same binary executing the file it
mutates — and it passes in isolation and on the rerun; no assertion is
weakened, and it joins the three recorded load-flakes in `P02-FLAKE`. Follow-on
accounting lives in `docs/reviews/r0_3/acceptance_matrix.json`
(`named_followups`, updated for this wave).

<!-- migrated-section:end -->
