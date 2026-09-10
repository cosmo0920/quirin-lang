# QCore-C agent briefing

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

Compressed project state for LLM agents; this file is the source of truth for
agents. Humans should read [README.md](README.md) for the summary and
[DEVELOPMENT.md](DEVELOPMENT.md) for implementation and status detail.

**Sync rule (steady state since card DOCS-COMPACT, 2026-09-10).** Status
*history* lives in `DEVELOPMENT.md`'s "Increment records" — one paragraph per
increment, nowhere else. This file carries only the compact ledger, the hard
invariants, the standing subsystem constraints, the per-change gate and the
entry points. A new increment therefore adds **one** paragraph to
`DEVELOPMENT.md`, updates this file's headline numbers (and, if it lands a
standing "when touching X, do not Y" rule, adds it to the constraints section
below), and leaves `README.md` alone. Do not reintroduce per-increment
chronicles here or in `CLAUDE.md`.

## What this project is

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency, lowering to C11.
It also ships a conservative checker for *existing* C source (the
"unrestricted-C gateway"). Normative specification: `docs/QCORE_SPEC_r0_3.md`
and `docs/QCORE_CONTRACT_SPEC_r0_3.md`. Current consumer grammars are in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`; implementation acceptance is tracked
in `docs/reviews/r0_3/acceptance.json`. Revision 0.2 remains historical; broad
parser/model support is not full source-language conformance. Work proceeds in
numbered bounded "cycles" recorded under `docs/cycles/`.

## Repository map

| Path | Contents |
|---|---|
| `compiler/` | Rust library crate `qcore-compiler` (lib `qcore_compiler`, 0.4.0): parser, checker, reference semantics, C backend, gateway/host compatibility APIs, first-party providers. Still the L1/L2 grab-bag; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `crates/qcore-core/` | L1 kernel: quantitative types, ownership/error/cleanup/CFG semantics, concurrency/translation models, chunk lifecycle, generic library/container/reference-count and reactor/descriptor custody, checked task IR/reference execution and C11 task emission. Also `os_endpoint`, `chunk_integrity`, `capacity_boundary`, `mutex_invariant`. No runtime or workspace dependencies |
| `crates/qcore-frontend/` | L1 front end: `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration and source lowering. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery: total-or-reject loader, observation and declared chunk-path policy evaluation. Depends on core/frontend/schemas |
| `crates/qcore-schemas/` | L0 published schema surface (Rust API **14.0.0**): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records, `reason` category vocabulary. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization |
| `crates/qcore-c-gateway/src/legacy_c.rs` | Generic unrestricted-C checker (Clang gateway, CFG, resource abstract interpretation, certificate recheck); contracts enter via `ResourceContracts` |
| `crates/qcore-c-gateway/src/legacy_summary.rs` | W4 interprocedural custody summary inference: SCC-ordered derivation, validation by re-analysis, declared-contract precedence |
| `crates/qcore-c-gateway/src/typed_source.rs` | Opaque typed C path import and source def/use |
| `crates/qcore-verifier-host/` | L2 generic registry (0.4.0), source-scope resolution, bounded process transport, execution profiles, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter (0.4.0): explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection, per-unit projection and reports over W2 profiles/W1 contracts. Links no provider module |
| `crates/qcorec/` | CLI binary `quirin` (crate `qcorec`) plus its process-level tests |
| `compiler/src/verifier_plugin.rs` | Compatibility facade for the host and first-party adapters |
| `compiler/src/verifier_plugins/{libevent,chunkio}/` | In-process first-party providers (Libevent/Monkey, ChunkIO) |
| `compiler/src/external_{integration,source}.rs` | Compatibility facades and Monkey-specific composition |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `providers/` | Separate six-member Cargo workspace (five `qcore-provider-*` packages plus the `conformance` harness), excluded from the core workspace. Artifact crates depend only on `qcore-schemas` and serialization; `providers/package.py` builds, verifies and archives the installable packages |
| `contract/qcore/` | Declarative `.qcore` contract corpus (**70 modules**); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `Audit*.lean` print axiom surfaces |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) — one directory per card |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking, schema 2) |
| `DEVELOPMENT.md` | Human-facing implementation and status detail: repository layout, cycle records, per-increment evidence narratives, full build/validation instructions |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. `crates/` holds nine crates; the core
Cargo workspace declares **ten** members (those nine plus `compiler`), and
`providers/` is a separate six-member workspace excluded from it.

Headline numbers at the wave-20 checkpoint (`docs/reports/wave20-corpus/`, the
current re-pinned baseline; wave-17's archive stays frozen beneath it):

| Measure | Value |
|---|---|
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — nothing promoted; 4,834 projected functions (150 Safe / 85 provisional Unsafe / 4,599 Unknown incl. `flb_input_chunk_create` function-level **Safe**, wave 20), leak **273** (271 → 272 → 273, each deliberate and accusing; see constraint 7) |
| CT01–CT50 conformance | **33/50 mapped**, 17 unmapped; sanitized-native 4 of 33, proof 0 of 33 |
| Contract corpus | **70 modules** |
| `qcore-schemas` | 14.0.0 · `qcore-compiler` 0.4.0 · `qcore-verifier-host` / `qcore-verification` 0.4.0 · ChunkIO provider package 0.13.0 |
| Merged gate | 3,277 workspace tests (8 ignored) · 122 provider · 193 Python · **89 Lean audits** · both probes exit 0 · fmt/clippy clean (wave-20 close + wave-21 partial (QUIRIN-RENAME); corpus re-pinned to `docs/reports/wave20-corpus/` — 12 leaves removed / 3 added, all attributed, unit headline unchanged) |
| Full language conformance | **false** |

Cycles 1–17 are complete at their bounded milestones; Cycle 18 is bounded
implementation complete with aggregate acceptance pending; Cycle 19 (the
unrestricted-C gateway on pinned Fluent Bit) is in progress with **raw units
unpromoted** until per-path identity imports and real observation evidence
satisfy the fail-closed gates in `docs/cycles/CYCLE_19.md`; Cycle 20 (the
verifier-plugin ecosystem) is active — protocol v1 carries Boolean assertions
only, the core **refuses all v1 `Safe` proposals** including empty scopes, and
unchecked `Unsafe` proposals downgrade to `Unknown` (`QP010`).

Where the detail lives, and the only places to look for it: per-increment
history in `DEVELOPMENT.md` → "Increment records" (every wave and card
paragraph, in order, each keeping its own checkpoint and boundaries); open
follow-ons in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`,
27 entries) plus each report's own follow-on list; per-card evidence in
`docs/reports/<card>/README.md`, which is authoritative for that card's counts,
boundaries and premises; direction in `docs/GENERALIZATION_PLAN.md` (W0–W7) and
`docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table.

## Hard invariants (violating these is a defect, not a style issue)

1. **Fail-closed.** Missing contracts, unmatched profiles, unsupported
   constructs, provider absence/failure/mismatch, unproven call effects →
   `Unknown`. Never optimistic fallback. Refactors must not flip any
   `Unknown` to `Safe`.
2. **Verdict contract.** Every published result carries
   `(artifact, property, semantic profile, scope, bounds, assumptions,
   evidence version)`. `Safe` only after all obligations discharge; `Unsafe`
   only for a validated reachable violation; everything else `Unknown`.
   Projection coverage is not safety coverage; unknowns stay in the
   denominator; a provisional `Unsafe` is never presented as an upstream
   defect.
3. **Parity before deletion.** A hardcoded analysis route is removed only
   after its replacement reproduces the pinned verdicts/coverage, with the
   diff archived under `docs/reports/`.
4. **Gate ordering** (`docs/REEVALUATION_SPEC.md` §9): certificate
   containment (G1) before provider expansion; Cycle 19 routes stay until
   G4 parity.
5. **Pinned expectations are intentional.** `qcore_contract_corpus.rs` pins
   per-file contract/clause counts (editing `contract/qcore` requires
   updating its tables); a test asserts the `third_party/libevent` submodule
   revision; pinned commit hashes are load-bearing identity checks.
6. **Lean parity.** Semantic changes need matching Lean definitions/theorems;
   audits must stay placeholder-free with no project axioms.

## Standing constraints when touching specific subsystems

Each rule below was landed by a card and must survive refactors. The evidence
pointer is the card that established it; read it before deviating.

**Diagnostics and codes**

1. `docs/QCORE_SPEC_r0_3.md` §L21 is the canonical per-code ownership table. A
   **semantic pass** owns a range, `qcore_frontend::frontend::source_diagnostic`
   is the authoritative pass → code map, any crate running that pass emits from
   its range, and a code may not be reused by a second pass. Codes are stable:
   a code change is semantic, and new codes go through the §L21 process (as
   `Q624`/`Q625` and `Q626`/`Q627` did). Fixing where a diagnostic is
   *attributed* (file, line, span) is not a code change. `remaining-d01`,
   `remaining-ct33-ranks`, `remaining-ct34-growth`.
2. `Q020` (task route) and `Q021` (sequential route) both refuse an unenforced
   source `contract { … }` and **must stay separate** until a source-contract
   consumer lands: the task gate also refuses an empty `contract { }` and the
   sequential gate accepts it, so unifying them would change rejection
   coverage. `source.extern-call.v1` is not that consumer — it consumes
   explicitly selected *declared* contracts. §L21, `remaining-d01`,
   `extern-call-v1`.
3. §L21.1's reason vocabulary is implemented as `qcore_schemas::reason`, but
   **do not add a category field to `DerivationIncomplete`** without
   regenerating every task digest, provider package and witness in the same
   change. An `environment-unavailable` row must never be relabeled a source
   defect, and must never contribute to a provisional `Unsafe`.
   `remaining-reason-codes`, `remaining-d01`.

**W4 custody inference and the C gateway**

4. A call into an internal C function is modeled only through a declared
   `.qcore` contract or a summary that `observations_match_contract` validated
   against a fresh analyzer run; everything else stays `Unknown`. When touching
   `crates/qcore-c-gateway/src/legacy_summary.rs`: **do not** let a summary be
   published without validation, **do not** seed a formal parameter with entry
   custody the body did not demand (that asserts non-nullness and deletes
   failure paths), and **do not** accept an assume-guarantee fixed point for a
   recursive component. Declared contracts always win; a disagreement is a
   `SummaryConflict` diagnostic, never a silent override.
   `w4-custody-inference`, `w4-custody-extensions`, `w4-gateway-crate`.
5. The typed scalar-field whitelist's last premise — the base is a name this
   function never binds to a resource identity — exists because dropping it
   turned a read through a released pointer into `Safe`. The fail-closed pin
   caught it and **was not weakened**; keep it that way. `remaining-c02`.
6. **C02-P-ALIAS-LEAK-ORDER:** `report_leaks` on intermediate iterates is
   scheduling-sensitive; a pre-pass keeps it scheduling-neutral with
   `AbstractState` byte-identical. Recorded, not repaired — do not disturb it
   incidentally. `remaining-c02-p-alias`, `remaining-borrow-liveness`,
   `remaining-hygiene-14`.
7. Leak diagnostics are **273** across the pinned 185-unit corpus. A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. It has moved exactly twice, deliberately, each in the *accusing*
   direction: 271 → 272 at the wave-17 re-pin
   (`flb_input_chunk_get_direct_routes` L19399, on a function and a unit
   already `Unsafe`, when a neighbouring refusal that hid it was admitted;
   provisional as `CONSTRUCTOR-RESIDUALS-LEAK-271` — deciding it would
   *withdraw* six standing claims), and 272 → 273 at the wave-20 re-pin
   (`input_chunk_get` L19849: modelling the constructor let W4 bind its
   summary, exposing a release through the uncontracted
   `flb_input_chunk_destroy`; the function moved `Unknown` → provisional
   `Unsafe`, and `ADDRESS-OF-MEMBER-DESTROY-UNDECLARED` names the
   declaration that would decide the leaf). `remaining-o04-b08`,
   `corpus-repin`, `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`.
8. **A declared contract must agree with the C the unit declares.** A contract
   whose parameter or result ABI projections the analysed translation unit's
   own C declaration contradicts is **withdrawn for that unit** — every call
   site rewritten to a marked uncontracted callee, and the name removed from
   the environment the derivation seeds. Both sides are read from what a
   certificate already holds (the preprocessed token stream; the environment's
   retained texts), never from the typed Clang AST, so W4 derivation, W4
   validation and certificate rechecking reproduce the decision — do not move
   either reader. A withdrawal is monotone in the refusing direction but **can
   turn a `Safe` unit into a provisional `Unsafe`** when the withdrawn
   declaration was discharging an acquisition; that is `Unknown`-class
   evidence about the declaration, never an upstream defect claim (invariant
   2). The scalar projection assumes the LP64 model of the audited targets and
   the reader has no target (`LOADER-ABI-LP64`). `remaining-loader-abi`.
   **The same rule now covers modes, not only types.** A declared contract
   whose custody *duty* the validated W4 summary of that callee's body in the
   analysed unit contradicts is withdrawn for that unit too, refusing at every
   call site with `mode-parameter-contradiction` or
   `mode-result-contradiction`. The derived summary is **never installed in the
   declaration's place** — an inferred summary can be weaker than the
   declaration it convicts. The gate takes the *duty* inequality
   (`Retained`/`Transferred`/`Delivered`/`Restored`; `Absent`/`Produced`) and
   deliberately does **not** withdraw on a refinement inside one duty:
   nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
   parameter keep `SummaryConflict` as their only report, because every
   derivable comparison in the pinned corpus is a nullability refinement and
   withdrawing on inequality would withdraw a release contract in every unit
   including `rd_free`/`mk_mem_free`/`flb_free`'s headers. Absence of evidence
   is not evidence: nothing is withdrawn on a body this unit does not define or
   W4 derives no summary for. `remaining-loader-abi-modes`.
9. **A fixture prelude is not a production declaration.** A fixture result is
   a result *for that fixture's declarations*: card O04-B08's flagship
   aggregate store was measured never to reach production, because the real
   member is `void *` and the reduced prelude's is `struct cio_chunk *`. When
   a card publishes a per-function count, say which declarations it rests on
   (`NEIGHBOURS-FIXTURE-PROXY`). `remaining-alias-aggregate`,
   `remaining-neighbours`.
10. **A store into an object that outlives the call is refused, not escaped.**
    Publishing a tracked identity into a file-scope object or a `static` local
    withdraws both directions and retires the residual leak through the
    unresolved-effect mechanism. The escape model was measured to **promote**
    the publish-without-release body from `Unsafe` to `Safe`, which invariant 1
    forbids; do not reintroduce it. A null store and a scalar store stay
    admitted on stated grounds, and the decision reads no per-site status.
    **The borrowed and `Out` admissions are withdrawn** (wave 17): a body
    publishing a *borrowed* formal no longer certifies, because `Borrowed`
    asserts the caller's obligations are unchanged and a publication falsifies
    exactly that — the launder it enabled crossed the **summary** boundary, so
    W4 published `Borrowed` for the helper and the helper's caller certified
    too. A declared `Out` slot naming such a destination is refused before the
    call's effects apply, being the `Acquire` arm seen from the callee's side.
    The rule reads no provenance; one exclusion, a declared *scalar* source,
    was added by measurement. The indirect (pointer-base) shape carried no
    false `Safe`, so its residual *violation* claim is withdrawn rather than
    strengthened, each statement keeping the refusal it already had.
    `remaining-ctwin-publish`, `remaining-ctwin-residuals`.

**Compile-task identity and source transport**

11. **Do not normalize the spelled input path** (that changes `__FILE__`), **do
    not** let a declared identity survive onto derived text, and **do not** move
    the pinned Cycle 19 compatibility route off `HistoricalStdin` without
    re-measuring its 185-unit artifacts. `CompileTaskIdentity` binds this
    transport as `source_input` — keep it there, or a standard-input run and a
    declared-file run record the same identity for different preprocessing.
    `remaining-g02`, `remaining-g01`.
12. A `#line` directive in analysed source must not be able to move the audited
    scope; attribution is by include-depth flags no directive can move, and
    `Safe` requires full analysed coverage at the certificate gate.
    `remaining-e01-tail`.

**Concurrency source routes**

13. Concurrency is opt-in: a file declares `import std.concurrency;`, the
    context is unavailable by default, and imports do not leak across
    compilation inputs (`Q014`/`Q015` refuse before checking each file). The
    task profile keeps its separate `concurrency checked;` marker.
    `core-concurrency-prelude`.
14. New keywords are a recorded compatibility hazard and must be checked
    against every fixture: T03-F added five reserved words, T08-EPOCH-SRC eight
    (with `work` renamed `effort` rather than editing the pinned golden
    surface), `source.extern-call.v1` added `succeeds`, and STD-MODULES added
    eight (`print`, `log`, `flush`, `dec`, `hex`, `info`, `warn`, `fatal`) —
    `error` was measured to collide with the reserved error-type keyword, so
    the severity is spelled `fatal` on the `work` → `effort` precedent rather
    than by moving the pinned surface. EXTERN-MODES deliberately added **none**:
    `move NAME` spells all four departing modes and the declaration decides
    which ran. `remaining-t03-f`, `remaining-t08-epoch-src`, `extern-call-v1`,
    `wave17-std-modules`, `remaining-extern-modes`.
15. Lock ranks and protection growth are **opt-in** disciplines: unranked
    modules stay byte-identical, and a rank is a declared position, not an
    assumption. `remaining-ct33-ranks`, `remaining-ct34-growth`.

**Providers, packages and limits**

16. **P02-FLAKE:** four provider/host rows are load- or parallelism-sensitive
    and are verified in isolation, never weakened: the `RLIMIT_NPROC`
    conformance test (it counts every process of the real UID, and keeps a
    deterministic companion), `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
    (wall-clock, measured failing at load 50 and passing at load 37 on the same
    binary), the confined-fixture write test (`Io(Broken pipe)` under load),
    `real_packages_preserve_unknown_and_reject_mutations` (`ExecutableFileBusy`:
    a sibling test in the same binary executes the file it mutates), and —
    fifth, wave 20 — `enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
    (the same `Io(Broken pipe)` shape as its sibling row, at load ~40).
    `remaining-p03` §9.1, `remaining-p03-f`, `wave17-gen-ctreasons` §4.1,
    `wave17-corpus`.
17. **EXTERN-F1-STORE-REACH:** the abandonment gate refuses a second live
    acquisition and was deliberately not weakened, which is why
    `resource_stored` is checked but unreachable from a single source module.
    `extern-call-f1`, `remaining-extern-err-rel`.
18. Provider executables embed their manifest template and installed manifests
    are compared field-for-field against it, so **all five packages must be
    rebuilt and verified** after any change to the template or to a
    `qcore-schemas` major version; `providers/package.py --locked` is part of
    CI. `remaining-p01-f2`, `remaining-p01-f3`, `remaining-p03-f`.
19. **E03-B deliberately supersedes E03's `QP012`** unequal-premise-set refusal
    with the premise union (differing *values* still refuse). Do not restore
    the old refusal. `remaining-e03-b`, `remaining-e03-b-o`.
20. P04's in-process counter route was deliberately **not** removed (live
    production callers), and routes P09-A..F are retained with named gaps.
    Removal requires invariant 3 parity first. `remaining-p09`,
    `remaining-p09-routes`.

**Pins, archives and measurement**

21. **Never edit a frozen archive in place.** Card T00's `coverage.json` was
    rewritten by a later lane and is restored byte-exactly (including its
    original missing final newline) by HYGIENE-16. Publish label provenance
    **beside** the frozen archive (`blocker_current`, `label_source`,
    `archived_bucket`) rather than editing it — the dual publication makes the
    edit unnecessary, and `QCore.BucketTally` proves an archive edit under lane
    coverage moves no count. `wave16-hygiene`, `wave15-ct-labels`,
    `wave11-ct33`.
22. `contracts.json` is the fifth artifact `diff_artifacts.py` itemizes
    (document `schema_version: 2`, HYGIENE-16); before that it was not opened
    at all, so waves 14 and 15 verified inventory growth by hand. Use the tool
    now, and read its **normalised and raw** module keys together: the raw keys
    carry the checkout root, and a normalisation collision demotes the section
    to raw keys rather than merging two modules into one row. The archived
    pre-itemizer diff documents of the consolidation chain are **backfilled**
    (GEN-CTREASONS): append-only, behind a `backfilled_sections` provenance
    record, with each document its own oracle — the four entries it already
    carried must reproduce byte-identically from the artifact pair it names or
    nothing is written. `schema_version` is deliberately not bumped, and the
    per-card diffs under `docs/reports/remaining-*/artifacts/` are outside that
    chain (`DIFF-TOOL-CARD-DIFFS`). `wave16-hygiene`, `wave14-corpus`,
    `wave17-gen-ctreasons`.
23. CT coverage grows by **mapping, not conformance**. A grown denominator is
    not new conformance: sanitized-native is 4 of 33 and proof 0 of 33, and a
    stage may read `executed` only where a named workspace test runs it on the
    case's own artifact. A mapping lane retires an earlier lane's
    unmapped-bucket row by declaring `retired_unmapped_reasons` in **its own**
    stage-evidence record, checked total-or-reject in both derivations — never
    by editing the earlier record (constraint 21). `wave5-ct-integration`,
    `remaining-t00-ct-schema`, `remaining-ct34-sanitized`, `wave17-ct40`.
24. **REASON-CODES-B3**'s sidecar `reason_code` tripwire test is flipped at the
    corpus re-pin, per its own design — not silently. `wave9-gen-reason`.
25. The acceptance-matrix generator carries a `section_ownership` block:
    sections are derived, integrator-supplied via `--gate`, or carried verbatim.
    Whole-file regeneration must not clobber later reconciliations. The CT
    `newly_decidable_not_mapped_reasons` map is **derived** as of wave 17 — its
    keys are the derived newly-decidable list's ids by construction, so a row
    for a mapped case and a row outside the published list are both structurally
    impossible; do not reintroduce a hand-carried copy. Two derived fields read
    from a **frozen** archive rather than from disk
    (`corpus.contract_modules_per_directory`, `corpus.contract_clause_pin`, both
    from card B00's ledger) and therefore describe the 59-module checkpoint
    beside a live 69 — recorded as `D02-B00-LEDGER-SNAPSHOT`, to be repaired by
    publishing provenance beside them, never by editing the archive.
    `wave9-gen-reason`, `wave17-gen-ctreasons`, `wave17-corpus`.

**Trust boundaries that are true statements, not TODOs**

26. Process providers have process-group containment plus the opt-in
    target-qualified `linux-rlimit-v1` and `linux-confined-v1` profiles — this
    is limit and two-resource-class confinement, **not an OS sandbox** (no
    seccomp, no other namespace). The recorded default remains
    `unenforced-legacy`. `remaining-p02`, `remaining-p03`, `remaining-p03-f`.
27. The mutex model supports initially-unlocked non-recursive mutexes only.
    Dynamic shapes, condition variables, try/timed locks, rwlocks and recursive
    acquisition are represented refusals, not omissions. `remaining-t02`.
28. Declared profiles, `observations_trusted`, `GuardFootprintAuthority`,
    `SynchronizedCustodyEdges`, `DeclaredFootprintShape`,
    `InvariantPredicateDeclared` and the C layout/linker equalities are
    **premises**, not proofs. A witness admission conveys no provider
    authority: acceptance terminates in the core re-evaluating its own sealed
    import, so agreement alone discharges nothing. `remaining-t02`,
    `remaining-e02`, `remaining-p04`.

## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # once per checkout
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
python3 scripts/check_lean_audits.py
```

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `quirin
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan). Provider changes additionally
need `providers/package.py` build + verify (constraint 18).

## Lean CI hygiene

CI builds `proof/lean-toolchain` with Lean and runs every `Audit*.lean` via
`scripts/check_lean_audits.py`. `AuditAll.lean` checks the compiled QCore axiom
closure, allowing only `propext`, `Classical.choice` and `Quot.sound`; stale
audit names and warnings fail the gate. Both reevaluation examples run in
Linux and Clang-enabled Windows CI. The runner builds from the pinned proof
directory, reports progress, enforces finite time limits and cleans up workers
on timeout or cancellation. Generic Lean names carry historical aliases
(`QCore.DescriptorCustody`, `EventComposition` / `checkEventComposition`,
`busy_effect_preserves_lock`); the old Monkey module, composition accessors and
ChunkIO lock theorem remain usable. `docs/reports/w7-lean-ci/README.md`,
`docs/reports/w7-generic-lean-names/README.md`,
`docs/reports/lean-build-performance/README.md`.

## Key entry points

- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI (binary `quirin`, crate `qcorec`): `quirin check <file>`,
  `quirin emit -o out.c <file>`,
  `quirin plugins list|inspect <id>|verify <package>|enable|disable`;
  configured audits run as `quirin-audit CONFIG.json` (crate `qcore-audit`).
- Configured audits: `qcore-audit CONFIG.json`;
  `scripts/prepare_cycle19_clang_audit.py`,
  `scripts/prepare_cmetrics_clang_audit.py`,
  `scripts/prepare_libevent_core_audit.py`.
- Native threading examples: `examples/threads/README.md` (`cl.exe`);
  `build-windows.ps1 -Run` emits, builds and runs the copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
- Editor: `editors/emacs/qcore-mode.el` gives lexical `.qcore`
  highlighting; see `editors/emacs/README.md`. Highlighting does not imply
  executable or verified support.

### Diagnostics namespaces

Authority is `docs/QCORE_SPEC_r0_3.md` §L21 (ownership by semantic pass) —
this table is a summary of it, not a second allocation.

| Codes | Owning pass | Meaning |
|---|---|---|
| `Q000` | `NameResolution` (per file) | input file does not parse as a QCore declaration |
| `Q001`, `Q016` | `Index` | index/quant presentation; dimension/axis validity and quant-index type resolution |
| `Q010`–`Q015` | `NameResolution` | duplicates, unknown name, `Q014` unknown imported module, `Q015` concurrency without its file-local import |
| `Q020`, `Q021` | `OrdinaryTypes` | unsupported ordinary construct / task-route source-contract gate; sequential whole-program contract gate. **Route-specific, not to be unified** (constraint 2) |
| `Q022`–`Q027` | `ExternCall` | admission and bridging of `source.extern-call.v1` declared extern calls |
| `Q028`–`Q031` | `StandardIo` | `source.std-io.v1` print/log/flush admission behind `std.fmt`/`std.io`/`std.logger` |
| `Q032`–`Q035` | `UncheckedLinkage` | `source.extern-call.unsafe.v1` explicit unchecked linkage; scalars only, every pointer refused |
| `Q036`–`Q038` | `Pipe` | `\|>` pipeline desugaring shapes; `Q038` an operator token used where this revision gives it no meaning (`\|`, `&`, `^` reserved for bitwise) |
| `Q039`–`Q042` | `NativeLibrary` | `source.native-library.v1` manifest load, binding and agreement; manifest-resident waivers |
| `Q043` | `ComposedStdIo` | composed extern/std-io route's own refusals (`source.std-io.v1` × `source.extern-call.v1`) |
| `Q044`–`Q046` | `Branch` | `source.branch.v1` `if`/`else` admission: shape and arm subset, decidable condition tier (connectives, `!`, bitwise, constant bindings), custody join over survivors |
| `Q047`–`Q049` | `NativeLibrary` | second band of the same pass: manifest-bound contract slices, identity stacking, withdrawal |
| `Q100`, `Q103` | `Usage`, `Ownership` | usage grade; ownership |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | resource flow; cleanup lowering and reference interpretation |
| `Q300`, `Q302` | `TypedErrors`, `ErrorRegion` | typed errors; error region |
| `Q400`, `Q401`, `Q403` | driver invocation | no inputs; missing/ambiguous entry; `--entry` against a library module |
| `Q500` | `CEmission` | construct the reduced C11 CFG cannot encode |
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile |
| `Q610`–`Q627` | `Concurrency` | `CheckedMutexInvariantV1`; `Q624`/`Q625` opt-in lock ranks, `Q626`/`Q627` opt-in protection growth |
| `Q630`–`Q641` | `Concurrency` | `FlatEpochBoundV1` flat-epoch accounting (`source.epoch.bound.v1`) |
| `Q650`–`Q667` | `Concurrency` | `DisjointFootprintsV1` disjoint footprints |
| `Q670`–`Q677` | `Concurrency` | `CheckedRegionTaskCreditV1` error-region task credit |
| `Q700`–`Q703` | `PluginVerification` | scope resolution, undischarged obligations, conflicting evidence, unvalidatable provider allegation |
| `QP001`–`QP016` | provider host | protocol and evidence-firewall diagnostics |
| `<plugin-id>.<local-code>` | providers | provider-owned; exact plugin ID plus one 1–128-byte ASCII local component. Libevent uses `qcore.libevent.L00`–`L12` |

## Where to read more

Direction and tasks: `docs/GENERALIZATION_PLAN.md`. Architecture boundary and
open gates: `docs/cycles/CYCLE_20.md`. Trust/verdict rules and audit history:
`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`. Language:
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_CONTRACT_SPEC_r0_3.md`,
`docs/QCORE_MODULE_MODES.md`, `docs/QTT_ALIGNMENT.md`. Contract corpus
conventions: `contract/qcore/README.md`. Persistent audit setup:
`docs/CYCLE19_BUILD_INPUTS.md`. Design notes:
`docs/design/`. Implementation, status detail and the full increment history
for humans and agents alike: `DEVELOPMENT.md`.
