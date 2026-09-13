# Third delegated increment and final observation records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Third delegated increment (2026-09-08): eleven bounded cards plus a
documentation restructure landed through a third and fourth delegated wave of
isolated worktrees with per-card reports; this paragraph supersedes nothing
above and each card's record keeps its own boundaries. A stricter closing
standard applied from this increment on: a card closes only with a Rust
implementation plus audited Lean modules; Python artifacts are supplementary.
- Counted read shares (T05): `ScopedReadSharesV1` turns T04's whole loan into
  a counted budget (`FULL_READ_AUTHORITY` = 1024, at most two holders);
  exclusive authority, `drop`/`move`/re-lend and the new checked Buffer write
  return only with the last share. 19 Lean theorems
  (`QCore.ReadShare`). Separation-logic fractions, slicing, write shares and a
  third reader stay refused. Evidence: `docs/reports/remaining-t05/README.md`.
- Typed owned error payloads (T06): `ScopedErrorPayloadV1` carries exactly one
  owned Buffer inside a declared error tag through completion and join;
  uniform rows, entry exclusion, one-shot `qcore_error_payload` take, 19 Lean
  theorems (`QCore.ErrorPayload`), six fixed sanitizer runs. Evidence:
  `docs/reports/remaining-t06/README.md`.
- Flat epoch accounting (T08): `FlatEpochBoundV1` checks a residency/work
  bound (shared union once, private peaks summed, declared overhead,
  completed-child results live until transfer; work never becomes wall-clock)
  with diagnostics `Q630`-`Q641` and 20 Lean theorems. Model-level only.
  Evidence: `docs/reports/remaining-t08/README.md`.
- OS failure post-states (O03): the Linux `close(2)` failure post-state
  relation — five descriptor states over three separated identities, EBADF as
  refutation, POSIX-unpinned targets Indeterminate with no permitted retry —
  is specified (`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`), implemented in
  `qcore-core::os_endpoint` (39 tests) and proved (27 Lean theorems); the
  Python matrix/probes are supplementary. Two declared-model defects
  (`DescriptorCustody.lean` failed-close retention; the `evutil_closesocket`
  error clause) are recorded open as U4/U5, not repaired. Evidence:
  `docs/reports/remaining-o03/README.md`.
- Bounded composition audit (P08): the five-provider registry reproduces each
  implemented capability's single-provider outcome, with absence/mismatch/
  timeout/conflict rows per provider; only four capability routes exist and
  cross-route composition is measured unreachable (QP015), so the full
  five-provider gate is recorded open, not satisfied. Evidence:
  `docs/reports/remaining-p08/README.md`.
- Error-region task credit (T07): `CheckedRegionTaskCreditV1` composes the
  region credit discipline with the unedited task ledger — duties are derived
  from the ledger and never convert into credit; abandonment waits for joins,
  loan expiry and child endowments; rethrow carries remaining duties.
  Diagnostics relocated to `Q670`-`Q677` at integration; adapted to T05's
  counted-share API. 23 Lean theorems. Model-level; CT21-23 need a source
  form. Evidence: `docs/reports/remaining-t07/README.md`.
- Disjoint footprints (T09): `DisjointFootprintsV1` decides byte-interval
  disjointness against one allocation generation with an 11-rule layout
  contract, bit-field storage units as single locations and protocol-cell
  descriptor identity; cross-allocation disjointness deliberately refuses.
  Diagnostics relocated to `Q650`-`Q667` at integration; 28 Lean theorems.
  Evidence: `docs/reports/remaining-t09/README.md`.
- Source-to-native trace relation (F02): `qcore.task-trace-relation.v1`
  relates reference and native renderings of the pre-T06 owned round trip by
  admitted causal constraints (nine conjuncts, 20 violation classes); the
  concurrent domain admits six schedules and the shipped backend renders one;
  whole-domain verdict digests tie the Rust relation to 26 Lean theorems. At
  integration the T05/T06 event forms were made explicit refusals and the two
  pinned emission digests were deliberately re-pinned after the T05/T06/macOS
  emission changes, with the native relation re-verified. Which order the
  native execution actually took remains unestablished. Evidence:
  `docs/reports/remaining-f02/README.md`.
- Route removal (P09): the hardcoded per-plugin first-party selection route is
  deleted after byte-identical re-measured parity with W2 H6 package
  discovery; `qcore-compiler` is versioned 0.2.0. P04's counter route was
  deliberately not chosen (live production callers). Seven remaining legacy
  routes are named follow-ons P09-A..G. Evidence:
  `docs/reports/remaining-p09/README.md`.
- Aggregate composition (O05): a compositional teardown contract for one
  aggregate parent and one wrapper — distinct child identities across parent
  release, callback quiescence and joined tasks before teardown, durability
  separate, status facts refused as evidence — decides `Composed`/`Unknown`
  only; on the real Cycle 18/19 subject it yields `Unknown` with named
  per-member blockers. 39 Lean theorems. Evidence:
  `docs/reports/remaining-o05/README.md`.
- Wave-3 CT integration: CT03/CT04/CT05/CT17/CT18 are mapped with
  stage-derived evidence, moving CT coverage 15/50 to **20/50**; CT03/CT17
  are the second and third harness-gated sanitized-native cases. CT11 stays
  unmapped on T05's alias caveat; CT35/CT36/CT38/CT39 stay unmapped pending
  epoch source syntax. CT03 records `cfg: executed`, matching the task-route
  convention over T05's proposal. Evidence:
  `docs/reports/remaining-wave3-ct-integration/README.md`.
- Documentation split: `README.md` is now a ~110-line human summary and
  `DEVELOPMENT.md` holds implementation/status detail, with every status line
  preserved verbatim; new per-increment paragraphs belong in DEVELOPMENT.md's
  "Increment records".

Integration notes: the concurrency diagnostic bands are now `Q610`-`Q623`
(mutex invariants), `Q630`-`Q641` (epoch accounting), `Q650`-`Q667` (disjoint
footprints) and `Q670`-`Q677` (region task credit), with `Q600` shared as the
carried-over structural refusal — T07 and T09 were relocated at integration
after landing on T08's band from independent worktrees; recording all four
bands in spec §L21 remains a deliberate follow-up edit. A module using both
counted shares and a typed error payload is labeled `ScopedReadSharesV1` and
the combined share+payload(+credit) feature set remains unvalidated. Upstream
macOS native-threading support was merged from origin and its emission changes
are covered by the re-pinned F02 digests.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, the contract
corpus stays 59 modules and full language conformance stays false. CT coverage
advances to 20/50 mapped, by executed-stage evidence only. The four pinned
185-unit artifacts were not re-measured (CORPUS-REPIN remains open).
`qcore-schemas` is 5.0.0; `qcore-compiler` is 0.2.0. Merged gate at this
checkpoint: 1379 workspace tests, 58 provider tests, 81 Python tests, 31 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean in both
workspaces.

Final observation increment (2026-09-08): the last two cards of the 49-card
remaining-work plan landed through isolated worktrees with per-card reports;
this paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Reopen corruption/checksum family (O01): an eight-fixture ChunkIO reopen
  observation family runs the real pinned `cio_chunk_open` path with
  `CIO_CHECKSUM` enabled — the first recorded `CIO_ERR_BAD_CHECKSUM` run in
  this repository — over five content classes and three injection states as
  disjoint axes, with the content class derived from recorded bytes by an
  independent Rust CRC-32, never read out of the report. All eight corrupted
  or clean images are byte-identical before and after; custody accounts
  80/80/0 allocation generations, 6 library closes plus 2 harness
  reclamations, and 7 library mapping epochs. The decision procedure is
  `qcore-core::chunk_integrity` (44 tests, 30-row mutation matrix, all
  refused) with 42 audited Lean theorems
  (`QCore.ChunkIntegrityObservation`). Two deliberate pin updates: the
  contract corpus moves 59 to 60 modules
  (`contract/qcore/chunkio/filesystem_integrity_observation.qcore`, 21
  clauses, no `resource_contract_v1` marker) and the loader's rejected count
  moves 144 to 145. The family compiles pinned `lib/chunkio` sources through
  ChunkIO's standalone CMake because the pinned Fluent Bit build tree is
  unavailable; no object-code equivalence with a Fluent Bit build is claimed,
  no production route consumes the relation, and no corruption recovery was
  observed. Evidence: `docs/reports/remaining-o01/README.md`.
- Capacity grow boundary (O02): the file-backed chunk grow boundary
  (`ftruncate`/`fallocate`/`posix_fallocate`/extending `write` and the
  `ENOSPC`/`EDQUOT`/`EFBIG`-class refusal) is recorded on x86_64 Linux over
  six sanitized native sequences: length-only grants versus reserved grants
  (`st_blocks` measured), refusal with identical-repeat refusal, a
  returned-error-without-`errno` arm, a partial transfer and a live
  mapping-beyond-end-of-file, with 16 epochs acquired and released. The
  decided relation is `qcore-core::capacity_boundary` (42 tests, 27 Rust plus
  14 Lean mutation rows, all refused) with 30 audited Lean theorems
  (`QCore.CapacityBoundary`); refusals carry module-local keys, no Q-code.
  `ENOSPC`/`EDQUOT` are classified from man-pages, not observed; metadata
  boundaries, the pinned-unit route and any production consumer remain open.
  Evidence: `docs/reports/remaining-o02/README.md`.

With these two cards the 49-card execution plan of 2026-09-07 is fully
dispositioned: every card is closed at its stated boundary or recorded as an
explicit named follow-on. Nothing is promoted by this increment: raw Cycle 19
units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only
units, CT coverage stays 20/50 mapped and full language conformance stays
false. The contract corpus is now 60 modules by O01's deliberate pin update;
the four pinned 185-unit artifacts were not re-measured (CORPUS-REPIN remains
open). `qcore-schemas` is 5.0.0; `qcore-compiler` is 0.2.0. Merged gate at
this checkpoint: 1473 workspace tests, 58 provider tests, 95 Python tests,
34 Lean audits, both reevaluation examples exit 0, fmt and clippy clean in
both workspaces.

macOS Cycle19 compiler-option transport (2026-09-08): the configured compile
database gateway now retains Apple Clang `-Wundef-prefix=<prefix>` and the
two-word `-arch <architecture>` as ordered typed frontend inputs, forwarding
them through preprocessing, syntax checking and typed-AST import. Malformed
forms, absent explicit dialects and compiler substitution remain fail-closed;
the complete command and typed target selection stay bound into the compile-task
identity. The user's prepared audit at Fluent Bit `11651d47` now completes all
181 configurations across 180 source files with 2 Safe / 41 provisional Unsafe /
134 Unknown / 4 data-only and zero gateway failures or option refusals.
Historical evidence stays Unknown and this current-revision result neither
replaces the pinned 185-unit ledger nor promotes a raw unit. Evidence:
`docs/reports/macos-cycle19-compiler-options/README.md`.

<!-- migrated-section:end -->
