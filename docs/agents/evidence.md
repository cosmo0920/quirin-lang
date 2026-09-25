# Evidence rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<a id="constraint-21"></a>

<!-- preserved:constraint-21:start -->
21. **Never edit a frozen archive in place.** Card T00's `coverage.json` was
    rewritten by a later lane and is restored byte-exactly (including its
    original missing final newline) by HYGIENE-16. Publish label provenance
    **beside** the frozen archive (`blocker_current`, `label_source`,
    `archived_bucket`) rather than editing it — the dual publication makes the
    edit unnecessary, and `QCore.BucketTally` proves an archive edit under lane
    coverage moves no count. `wave16-hygiene`, `wave15-ct-labels`,
    `wave11-ct33`.
<!-- preserved:constraint-21:end -->

<a id="constraint-22"></a>

<!-- preserved:constraint-22:start -->
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
<!-- preserved:constraint-22:end -->

<a id="constraint-23"></a>

<!-- preserved:constraint-23:start -->
23. CT coverage grows by **mapping, not conformance**. A grown denominator is
    not new conformance, and a grown proof numerator is evidence about one
    case's artifact: the sanitized-native and proof counts are the ledger row's
    — read them there and **never copy them here**, because a copy in this
    paragraph has gone stale twice and each time this line claimed authority it
    does not have. And
    a stage may read `executed` only where a named workspace test runs it on the
    case's own artifact. A mapping lane retires an earlier lane's
    unmapped-bucket row by declaring `retired_unmapped_reasons` in **its own**
    stage-evidence record, checked total-or-reject in both derivations — never
    by editing the earlier record (constraint 21). A lane may also **withdraw a
    row it measured** rather than loosen a test around it: wave 26 re-measured
    CT24 and CT47, and because `newly_decidable_not_mapped_reasons` is derived
    from that judgment's own key set (constraint 25), publishing labelled rows
    for them would have put three Python tests red against a matrix the lane may
    not regenerate. The rows were withdrawn, the measurements published in the
    lane's own record and in `acceptance.json`'s hand-kept reasons, and both
    cases keep card T00's DECIDABLE-NOW bucket — a re-verification reaching the
    same bucket is the record saying the label holds. `wave5-ct-integration`,
    `remaining-t00-ct-schema`, `remaining-ct34-sanitized`, `wave17-ct40`,
    `ct-wire-lane`.
<!-- preserved:constraint-23:end -->

<a id="constraint-24"></a>

<!-- preserved:constraint-24:start -->
24. **REASON-CODES-B3**'s sidecar `reason_code` tripwire test is flipped at the
    corpus re-pin, per its own design — not silently. `wave9-gen-reason`.
<!-- preserved:constraint-24:end -->

<a id="constraint-25"></a>

<!-- preserved:constraint-25:start -->
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
    beside a live corpus (96 after Cycle 18 broader integration) — recorded as
    `D02-B00-LEDGER-SNAPSHOT`, to be repaired by
    publishing provenance beside them, never by editing the archive.
    `wave9-gen-reason`, `wave17-gen-ctreasons`, `wave17-corpus`.

<!-- preserved:constraint-25:end -->

<a id="constraint-28"></a>

<!-- preserved:constraint-28:start -->
28. Declared profiles, `observations_trusted`, `GuardFootprintAuthority`,
    `SynchronizedCustodyEdges`, `DeclaredFootprintShape`,
    `InvariantPredicateDeclared` and the C layout/linker equalities are
    **premises**, not proofs. A witness admission conveys no provider
    authority: acceptance terminates in the core re-evaluating its own sealed
    import, so agreement alone discharges nothing. The **two largest premises
    in the gateway** belong on this list and were missing from it until wave 34
    (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`): **faithful C token/type
    extraction** — that Clang's preprocessed text and typed AST, as this
    analyzer reads them, are the translation unit C means — and the
    **Lean-model-to-Rust correspondence** — that a theorem proved of a Lean
    model of a rule is a theorem about the Rust that implements it. No
    extraction, refinement or simulation theorem exists in `proof/`, so
    invariant 6's "Lean parity" names a correspondence that is **maintained by
    review, not proved**; a gateway module's theorems are abstract rule
    theorems about the model, and constraint 4's "its soundness is an induction
    on execution prefixes" is a statement about `N(m)` in the Lean model, not
    about `legacy_neutral.rs` as compiled. `remaining-t02`, `remaining-e02`,
    `remaining-p04`, `c-gateway-wave`, `stability-reeval-1` §3.2.

<!-- preserved:constraint-28:end -->

## CT41 negative synchronization attribution

CT41 retains its volatile-completion scenario and “no synchronization proof”
expectation. Its bounded publication route uses protocol replacement for
attribution: shape-validated volatile and plain cases both refuse, a matching
release/acquire atomic control checks, and relaxed-store/load controls refuse.
Removing `volatile` must never confer ordering. This explicitly supersedes the
historical removal-must-change-verdict criterion for CT41 only. Mapping credits
its own executed source stages, not its control's certificate, model theorem or
native evidence. [Decision and bounds](../reports/ct41-publication-mapping/README.md).
