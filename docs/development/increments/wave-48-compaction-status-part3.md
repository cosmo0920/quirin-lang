# Wave 48 compaction checkpoint: status page, part 3 of 11 (`STATUS-COMPACT`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Current status](../../STATUS.md) · [Part 2](wave-48-compaction-status-part2.md) · [Part 4](wave-48-compaction-status-part4.md)

> **Preserved checkpoint text, not a current authority.** The fenced block is
> `docs/STATUS.md` lines 308–354 at `f77e97ae6`, verbatim (10,031 bytes), part 3 of the
> whole page that `STATUS-COMPACT` replaced with a current-state page.
> `git show f77e97ae6:docs/STATUS.md` reproduces it byte for byte, and
> [`checkpoint.py check`](../../reports/status-compact/scripts/checkpoint.py) asserts
> that the parts concatenated equal that page. Where this text and the current
> page disagree, **the current page wins**; figures here describe their recorded
> checkpoints. Relative links inside keep their original base, `docs/`.

~~~~~text
The [checked same-item detach](reports/cjson-detach-same-item/README.md) at analyzer/proof `9bfe86a41` implements the user's approved transfer: success returns the same contained item, NULL transfers nothing, with checked membership/identity/live-parent/alias obligations and an explicit conditional physical-list representation premise. The actual production declaration stays pinned-in-force / DeclaredAssumption (failed0/withdrawn0/unvalidated1/validated0, four dependent rows/zero Safe); ordinary callee projection and all six wrappers remain Unknown. Final corpus/system/slice audits all exit 0 against completed S3/RK: **961 Safe / 3,864 Unknown / 9 provisional Unsafe**, **99 / 345 / 4**, **53 / 346 / 4 Pending / 1**. Four corpus rows change diagnostics/dependencies only; all 5,781 null dictionaries, combined verdicts and runtime moved records are unchanged. Fresh bottom-up planning is **258 single-body waits / 2,223 candidate bodies / 363 hypothetical ceiling**, no-extra-body same/cross closure **4/4** (baseline260/2224/365,4/4). Two wrappers leave the call-only census for unsupported return effects; these are not discharged bodies. The decision is implemented; exact lookup membership, removed-result wrapper transport and independent representation closure remain tracked. The cumulative **schema19** gate closes S3's two public fields plus Removed: **8,325 core Rust / 342 Lean / 374 post-matrix Python**, fresh providers **133 Rust / 25 Python**, all five packages rebuilt/verified; prior failed gates, the superseded0091 production withdrawal and cancelled jobs1 control are retained.

The prior [S3 lifetime checkpoint](reports/fb-event-loop-lifetime-s3/README.md) at analyzer/proof source `d0a48bfd6` implements scoped conditional prerequisites and exact same-unit declared registration forwarding, with independent source replay and audited Lean. Fresh composed corpus/system/slice audits all exit 0 against the frozen lexical baseline: custody **961 Safe / 9 provisional Unsafe / 3,864 Unknown**, **99 / 4 / 345**, and **53 / 1 / 346 / 4 Pending**. The two corpus Safe → Unknown wrappers (`rd_kafka_event_destroy`, `ut_op_destroy_free`) follow the existing global unread callback-closure policy after new lifetime tokens expose 36 unread units; they are a tracked precision limit, not lifetime violations. All 5,781 static null dictionaries are unchanged. The separately directed RK envelope correction accounts exactly for runtime null Pending → Unknown **1/2/16** and combined **1/2/0** rows under all-loaded and RK-only; FLB-only is unchanged by that correction. Corpus all-loaded runtime custody is **931 Safe / 30 Pending / 3,864 Unknown / 9 provisional Unsafe**. Conditional record correlation, derived-only loop equalities, rich cross-unit replay and threaded teardown remain unresolved. Full semantic gate: **8,307 Rust / 341 Lean / 374 Python**, fresh provider **133 Rust / 25 Python**, five schema-18 packages verified; failed earlier attempts and the two extra strict slice outer-pin preflights (exit 1 despite reviewed-body reuse) remain disclosed. This frozen schema-18 semantic checkpoint is superseded for publishable compatibility by the cumulative schema-19 detach receipt above.

The [RK loader scope receipt](reports/rk-outer-loader-scope/README.md) removes the four Fluent Bit `.p` promises from `rd_kafka_plugin_new` (81 clauses), preserving the Fluent Bit envelope. Every runtime row matches the prior dependency census; no cross-loader coverage fallback is added. Loader-reach precision remains tracked as `RK-OUTER-LOADER-REACH-PRECISION`. The correction is included in the S3 gate and composed measurements; cumulative schema-19 validation is closed by the checked-detach receipt above.


**Partly implemented (CG16-INDEX-AND-OFFSET-EXTENTS, 2026-10-02):**
[reviewed index/extent contracts](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned)
specify a separate existing-C bounds property, `c.bounds.index-extent.v1`.
The route-G consumer, bounds contract v1 (G05b) and the audit selection
`QCORE_BOUNDS_PROPERTY` landed (gateway rule 11s); pins, placement and the
audit's bounds-module selection followed with CHUNKIO-META-LEN-BOUNDS-CONTRACTS
(gateway rule 12r: each provider crate's `bounds_view`, its
`bounds-contracts.lock`, a declaration out of pin held `contract-pin-mismatch`),
whose two pinned chunkio header declarations are the first to reach the audit
routes (C09.1's status paragraph). User-authored
external premises require human review, source-derived fixed bounds do not, and
every covered use is checked. Custody-Safe results are still not bounds
certificates: the bounds row is separate, and the combined row is Safe only
where both are (measurement in the table below). CG24-ALIAS-EXTENT-FRAME-VOCABULARY
(gateway rule 11e) extends the same vocabulary with closed read/write/move
frames, which are not custody modes, and derived results (offset/length,
several bases) checked against their bases and refuted by contradicting
bodies; with no bounds declaration selected on the audit routes they reach no
function yet. QUANT-INDEX-CHECKS (gateway rule 12l, contract spec C09.2) adds
the quantified index check `quant checks P, Idx #i: PRED;` to the same
vocabulary: checked on the body (an access decided outside the check refutes
the declaration, an undecided one stays Unknown) and as each caller's
obligation; since CHUNKIO-META-LEN-BOUNDS-CONTRACTS it reaches the audit
routes in reviewed, pinned provider modules. CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS
(gateway rules 14f–14j) publishes each declared body's check, header
definitions included, as evidence for its declaration (never a row), adds the
routes' cross-unit refutation phase and tip tracking for bounds pins, and
refuses what still does not transport (a checkpointed bounds run; inclusion and
inheritance of bounds modules).

| Measure | Recorded value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **50/50**; none unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT independent artifacts | **48** distinct source-fixture programs behind the 50 mapped cases (CT01/CT12/CT14 share one); **9** reviewed byte-identical fixture groups | Same manifest, `artifact_identity`, recomputed by `ct_corpus.rs`; [disclosure](reports/ct-shared-fixture-disclosure/README.md) |
| CT case-bound proof | **5/50**: CT01, CT03, CT07, CT37, CT50 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/50, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Verdict vocabulary | **Safe < Pending < Unknown < Unsafe**, composed by the join (four values since W45-USER-CONTRACT-PLUGINS, approved 2026-09-26): **Pending** = not decided, awaiting a named human act (`awaiting-human-review`, `contract-pin-mismatch`, `plugin-claim-uncertified`, and `outer-contract` in the runtime projection only, rule 31), never Safe and always in the denominator, its held reason only ever tool-minted and a Pending unit subject to every Safe scope gate (gateway 10i, W45-SR5-SOUNDNESS); the Cycle 19 corpus has **0 Pending units / 0 Pending functions** (4 deferred declarations since W47-LOOP-OWNER-INSTANCE-FLOW, none called by a corpus unit; every pin in force but one: since CONTRACT-PIN-TIP-MODE the tip-tracked `mk_vhost_set` is not in force at `66910c10` (`source-changed` against the tip of Monkey's fix branch, a warning notice) and no corpus unit calls it; no checker plugin), so the rows below are unchanged; since W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL a row is Pending only if its function carries no other reason with every held declaration in force as written (a held call no longer masks its statement's refusal), measured with no movement on the corpus, system and tail-pause slice; the tail-pause slice has 1 Pending function (`flb_input`, a held `flb_input_new` call); since W47-REVIEW-HOLD-CONCRETE-REASON (user decision 2026-09-29) every row holding a review-held call names each held declaration's markers, reason verbatim, with a structural category (`rewrite_before_acceptance`, C06.1) that changes no verdict: `flb_input` (both rows) stays Pending and names `flb_input_new`'s 3 markers, 2 `must-be-rewritten-before-acceptance`; 10 rows gain the member (slice 6, system 4, corpus 0), no status moves | [Spec §L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition), [report](reports/w45-user-contract-plugins/README.md), [held-call standing](reports/w45-review-hold-preempts-statement-refusal/README.md), [concrete reason](reports/w47-review-hold-concrete-reason/README.md) |
| External audit subcommands | `quirin-audit NAME` runs `quirin-audit-NAME` from the declared plugin path only (never `PATH`, never a built-in's name), out of process with a cleared environment and quotas; **0** subcommand plugins can reach `Safe` or `Unsafe` from the command line (no checker accepts their certificates: an uncertified claim is `Pending`, every failure `Unknown`); **1** shipped plugin, `quirin-audit libevent prepare`, byte-identical to the retired script on every prepared file (147 of 152 same-path files identical, the 5 others CMake/make logs with named differences); corpus verdicts unmoved | [Contract spec C11.2](QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins), [report](reports/w45-audit-subcommand-plugins/README.md) |
| Cycle 19 units | **6 Safe / 8 provisional Unsafe / 167 Unknown / 4 data-only**, 185 total, unchanged by W47-ACCESS-PATH-POSITIONS, W47-CALLEE-CLOSURE-R2, W47-TYPED-CONST-POINTER-VALUE and W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (no unit moves); W46 moved `rdcrc32.c` Unknown → Safe under its bounded typed-statement premises. Units remain unpromoted function roll-ups. | [W47 comparison](reports/w47-access-path-positions/artifacts/after/corpus-comparison/comparison.json), [previous movement](reports/w46-contract-access-expansion/census/combined-corpus-summary.json) |
~~~~~
