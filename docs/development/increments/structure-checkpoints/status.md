# Pre-restructure status

> Historical checkpoint at the migration base, not current instructions.
> Original wording is preserved; current rules start at [AGENTS.md](../../../../AGENTS.md).

<!-- preserved:status:start -->
## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. `crates/` holds nine crates; the core
Cargo workspace declares **ten** members (those nine plus `compiler`), and
`providers/` is a separate six-member workspace excluded from it.

Headline numbers after `W37-CORPUS-CONTRACTS-4` (2026-09-18). The **corpus
baseline is `docs/reports/w37-corpus-contracts-4/artifacts/`**, wave 37's second
arbiter and the chain's second **declarative** link, which moved 8 functions and
zero units.

**The chain is what makes each delta attributable**, and it is one rule applied
at every link: each arbiter's BEFORE run reproduces its predecessor's
verdict-bearing artifacts (`diagnostics.json`, `chunk-contracts.json`) **byte for
byte**, which isolates that arbiter's own movement from the gateway change
beneath it. Since wave 33 that identity is **asserted inside the archiving
tool**, which refuses to write if it ever stops holding, rather than merely
logged. Two link kinds are worth naming, because each weakens what a link has to
carry. A **declarative** link changes no crate source at all, so one audit binary
serves both its stages and the only difference between them is which modules the
contract manifest selects (`w36-corpus-contracts-3` was the first). A
**zero-movement** link is one whose AFTER is byte-identical to its own BEFORE, so
several consecutive ones collapse into a single reproduction — which is itself
the measurement that those code changes moved nothing this corpus can see.

The links, newest first: `w37-corpus-contracts-4` (declarative) over
`joined-provenance-round-bound` over `scalar-route-exit-edge` (wave 37's three)
over `joined-provenance-call-frames` over `w36-corpus-contracts-3` (wave 36's
two, the second declarative) over `joined-provenance-cycles` over
`c-gateway-constant-do-while` (wave 35's two) over `c-gateway-return-call-w4`
(wave 34) over `c-gateway-return-position` (wave 33) over
`w31-corpus-contracts-2` over `c-gateway-wave` (wave 31's two) over
`w4-recursive-summaries` (wave 30). **Each arbiter's leaf attribution and gate
live in its own report, which is authoritative for them**; this file carries no
per-arbiter attribution and the increment records narrate each wave's. **Eleven**
of those twelve links published an archive and all eleven are frozen
(constraint 21) — `joined-provenance-round-bound` published none, having moved
nothing — as are `wave23-c-operations`, `aom-destroy-siblings`,
`aom-binding-poison`, `gateway-c-location` and `w29-corpus-contracts`.
Diagnostics retain two coordinates: the presented original `file:line` and the
internal preprocessed line used for pin, leaf and certificate identity
(`GATEWAY-LOCATION-INTERNAL-CHANNELS`, constraint 12a).

| Measure | Value |
|---|---|
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — **no unit status has moved at any arbiter from wave 31 through wave 37**. 4,834 projected functions: **213 Safe / 84 provisional Unsafe / 4,537 Unknown**; `flb_input_chunk_create` remains function-level **Safe**. Diagnostics: leak **272**, `unknown_alias` **4,309**, `double_release` **0**, `unsupported` **49,753**, `summary_conflict` **1** — all six figures above re-derived this pass from the wave-37 baseline named in the paragraph above, which is where they are read from. **Every delta is attributed leaf by leaf in its own arbiter's report §5 and narrated in that wave's increment record, and those are the authorities for it** — this row carries the current numbers, the paragraph above carries the chain, and constraint 7 carries the leak pin's own history in full. What belongs here is the **current shape of the evidence**, which three standing facts fix. (1) A **declarative** arbiter buys its function moves with *contracts, not code*, and the last two moved the leak pin not at all; wave 37's ten declarations were **not one of them a release** — every one the neutral element `N(m)`, which a faithful summary of a body that releases nothing cannot contradict, so `summary_conflict` stayed at 1. (2) The cover that chooses declarations is an **upper bound and never a forecast** (`W36-COVER-HIDDEN-BLOCKER`): a refusal reachable only once the first is removed is invisible to it, and the remaining frontier is dominated by unit-internal `static` helpers, where W4 and not declaration is the lever (`W37-COVER-FRONTIER-IS-STATIC`). A second, different limit is the ABI **reader**'s: an inline function-pointer parameter declarator it does not parse withdraws a declaration leaf-neutrally at every call site (`NESTED-CALL-IN-ARGUMENT-POSITION`, `LOADER-ABI-INLINE-FNPTR-DECLARATOR`). (3) **A pre-pass's reach is measured, never assumed, and a reach figure is a coverage fact and not a safety one**: conditional provenance (constraint 10b) runs on **40** corpus functions, `release_site`'s naming premise passing **6 of 14** sites and its nullable-site premise blocking all 14; the loop-header exit-edge route (constraint 5c) reaches **neither** deciding arm at any of the corpus's **201** loop-header `BranchOnNull` nodes, 89.6% of those edges landing in an `untracked` early return — which must not be carried forward as if it were stable (`SCALAR-ROUTE-UNTRACKED-ARM-DOMINATES`). Provisional findings remain subject to the reports' coverage and reachability boundaries; the production `input_chunk_get` residual stays Unknown |
| CT01–CT50 conformance | **47/50 mapped**, 3 unmapped (CT10, CT24, CT41); sanitized-native **10 of 47 — a Unix figure**; **proof 4 of 47** — all four figures re-derived this pass from `docs/reevaluation/ct_manifest.json`, which is where they are read from. **The sanitized-native count carries its host condition, and that is a measurement:** all ten executed `native_sanitized` rows declare `host_gate: ["linux","macos"]` in the manifest (CT03, CT07, CT09, CT17, CT22, CT25, CT26, CT34, CT46, CT49; read from the manifest at the wave-40 close), and each runner reaches its platform test behind a runtime `cfg!(any(target_os = "linux", target_os = "macos"))` placed **after** the reference-run and `c_source` assertions — so on another host the *emission* row is genuinely exercised and the sanitizer row asserts nothing, which is why the number is published with the condition it was measured on rather than bare (`CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`, discharged at that close). Which lane mapped which case, and on whose evidence, is the increment records' business — waves 28 through 39 each carry their lane's paragraph, and each lane's own report is authoritative for its measurements. What this row carries is the **current state and the standing method**. *The proof leg* is CT01, CT03, CT37 and CT07, and it has **two shapes, not one**. Three of them (CT01, CT37, CT03) prove a scenario property in Lean of the checked artifact's reference run **and its admitted renderings** — CT37's completion strictly before the join in every admitted rendering — bound to the case by a named host-unconditional test through the **F02 digest relation**; CT03's is the one that needed the F02 alphabet itself to grow, by the T05 Buffer write, proved **conservative** so that a write-free trace relates exactly as before and no existing digest moved. CT07's (wave 39) is bound **through the scoped-task ledger's lease set** instead: `reference_trace` refuses this run's lease events by constraint 15n's decided boundary and was not touched, so wave 31's `no_byte_has_two_write_authorities` is instantiated on the artifact's own two-thread ledger state, with a proved replay (`QCore.Ct07LeaseLedger.replay_reaches`) as the `Reachable` witness and no alphabet growth. In all four, native execution is deliberately observation-only; the per-case gate is the pinned fail-closed `PROOF_GATED_CASES` table in `ct_corpus.rs`, and each flip's stage evidence supersedes the earlier record **from its own record** (constraint 21). *The sanitized-native leg* rests on wave 32's `CT-COMPOSED-NATIVE` runner, which compiles and runs an admitted case's own emitted C under the platform sanitizers and compares its stdout byte for byte with that module's own checked task half — every arm of that runner falsified before it was believed. CT23 stays outside it, being the route's refusal. *Evidence can improve without a count moving*, and twice has for CT07 alone (wave 30's write-lend scenario, then wave 37's **two-thread** range write lease, the first time the scenario's word *independently* rests on two threads) — each superseding the earlier record from its own record, with **no count moved**. *The three unmapped*, each re-measured rather than assumed: CT41's obstruction is **structural** — clause (b) of the wave-9 attribution rule needs a control in which removing `volatile` moves the verdict, and `CTWIN-PUBLISH` §5's decision is that it moves nothing, the typed-source arm unable to tell `volatile` from `_Atomic` (`CT41-TYPED-SOURCE-UNDISCRIMINATING`); CT10 has half a scenario; and CT24 stays **withdrawn by the lane that measured it**, a withdrawal rather than a loosened test, because `newly_decidable_not_mapped_reasons` is derived from that judgment's own key set (constraint 25). Every mapped case publishes the boundary its scenario's untranslatable words rest on — no pointer type and heap-bound storage for the buffer cases, no wrapper type and no `Send`/`Sync` trait for CT46, whose crossing is carried instead by the transferability judgment's `TransferKind::ProtectedResource`. A grown denominator is mapping, not conformance, and a grown proof numerator is evidence about one case's artifact (constraint 23) |
| Contract corpus | **88 modules**, **59** resource-v1 declarations, **33** manifest-selected. Two of those are pinned expectations and move only deliberately (invariant 5): the module count by `compiler/tests/qcore_contract_corpus.rs`, the manifest selection by `crates/qcore-c-gateway/tests/neighbours.rs`. The growth chain wave by wave, and which declarations each corpus card chose, are in the increment records and in those cards' own reports. The standing method, unchanged across the two declarative arbiters that produced the last twelve modules (`W36-CORPUS-CONTRACTS-3`, 76 → 82; `W37-CORPUS-CONTRACTS-4`, 82 → 88): declarations are chosen by **function-cover yield** over the baseline functions blocked only by uncontracted-call leaves, and each is read **line-cited from its own pinned body** before it is declared — the cover being an upper bound and never a forecast (`W36-COVER-HIDDEN-BLOCKER`). Wave 37's ten declarations are **not one of them a release**, every one the neutral element, which is why they moved eight functions to `Safe` and added no `summary_conflict`; its new families are chosen deliberately too (`mpack/` is **not** the existing `msgpack/`, a different library) |
| `qcore-schemas` | 14.0.0 · `qcore-compiler` 0.4.0 · `qcore-verifier-host` / `qcore-verification` 0.4.0 · ChunkIO provider package 0.13.0 |
| Merged gate | **4,727 passed / 0 failed / 10 ignored, 238 result summaries over 236 test targets** (227 `Running` plus 9 doc-test sections; **two** of those sections print two summaries each) · **141** Lean audits · **303** Python · 122 provider (carried) · both probes exit 0 · fmt/clippy clean · Clang **present** (21.1.8, run in the measuring worktree and recorded beside the gate). **Measured by the wave-40 close in two detached runs at its own in-history tips, whose figures are identical**; accounting in `docs/reports/wave40-close/gate.json`. Both tips are ordinary commits of the close's own branch and therefore reachable from `main` once its merge lands — deliberately, because every tip this row has cited since wave 39 is **not** — `4786ae4c` then, `c188d3ab` at the wave-40 base, each a commit object no `git log` of `main` will show (`INTEGRATION-TIP-CITATION-UNREACHABLE`, seven instances; the close publishes the successor mapping for all of them in its record). Each run's tree differs from its predecessor in documentation, tracker and matrix content only, and no test in this tree reads `AGENTS.md`, `CLAUDE.md`, `README.md` or the acceptance matrix's prose, which is why a documentation-and-tracker close may carry this row at all. **The provenance sentence names the run that produced the figures, never a card's** — the second arm of `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`, whose first arm is the `lean_audits` and `clang` keys `build_matrix.py --gate` now requires (the Lean count **checked** against `proof/Audit*.lean`; the Clang claim only **disclosed** — see the standing-gate section, `CLANG-RECORDING-UNBOUND-TO-ITS-RUN`), so a figure that once had no machine-readable source is refused where it would be written. **What is still not checked is the summary arithmetic** (`GATE-ROW-SUMMARY-COUNT-UNCHECKED`), and it is worth knowing why the previous figure was wrong: this row read **240** summaries and four doubled sections until wave 40, when `DOCS-COMPACT-8` and `STABILITY-REEVAL-4` measured it independently and agreed on **238** and **two**, and the close's own run made that a **third** independent measurement on a third tree (236 targets = 227 `Running` + 9 doc-test; 238 strict, 240 loose; the two doubled sections `Doc-tests qcore_core` and `Doc-tests qcore_verifier_host`). 240 is what matching the prefix `test result` *without its colon* returns, because two ordinary tests are named `result_…`; `236 + 4 = 240` is then how "four doubled sections" was inferred, so one loose regex produced both figures. Summing only `ok.` summaries is the neighbouring defect (`GATE-SUMMARY-PARSER-OK-ONLY`, why `0d9e922f`'s merge line reads "0 failed" while naming three failed rows). Each card's own gate is in its own report, and each close's in its own; the **integration** gates an integrator measures where no card's `gate.json` can agree with the merged tree are `docs/reports/<card>/integration-gate.json` (their `clang` claims borrowed from the cards' own recordings, `INTEGRATION-GATE-CLANG-CLAIM-BORROWED`), and the close sweeps every one into its record. **A gate figure is a measurement on one machine, never a property**, and three boundaries travel with this one. First, constraint 16's rows, because that constraint requires a gate report to **state which it saw**: **four** of its five named rows live in this workspace and all four were green in both runs, as were the unnamed `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` and `a_forced_override_never_weakens_an_execution`; the fifth, `real_packages_preserve_unknown_and_reject_mutations`, lives in the separate `providers/` workspace. Nothing flaked, so no isolated re-verification was required and none is claimed — and these are full-suite runs on a machine carrying its own build, **not** the quiet-host isolated number `HOST-ROW-STARVATION-DESCRIPTION` still asks for. Second, provider **122 is carried, not re-measured** (nothing under `providers/` or `crates/qcore-schemas/` moved and `qcore-schemas` is still 14.0.0, so constraint 18 is not engaged) **and is a Unix figure** (`PROVIDER-CONFORMANCE-UNIX-ONLY`). Third, the tenth ignored row is the `#[ignore]`d F02 pin-agreement test, which stays **red by design** on `--ignored` (`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`, a Windows row no Linux gate can see), beside `TSAN-ROWS-2`'s `#[ignore]`d measurement harness. The per-wave narrative this row used to carry is in the increment records |
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
history in `DEVELOPMENT.md` → "Increment records" → the wave index (every wave
and card paragraph keeping its own checkpoint and boundaries); open follow-ons
in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, **257**
entries, re-derived from the committed file after `STABILITY-REEVAL-4`,
`DOCS-COMPACT-8` and the wave-40 close). **Which card registered or retired
which name, and each wave's arithmetic, is the increment records' and each
close's own business** — wave 40's three paragraphs carry this wave's,
including its four retirements — and
the retirement discipline itself is a standing rule in `CLAUDE.md`. The
operative rule here: the section is *carried*,
`dimensions.named_followups` is *derived*, and the two are checked against each
other on the committed file — a hand edit that leaves the two apart goes red, as
`171 != 187` did at the wave-36 close, which is `D02-DIMENSIONS-FOLLOWUP-COUNT`'s
tripwire working — so add entries through
`docs/reports/remaining-d02/build_matrix.py` and never by hand (constraint 25).
Then each report's own follow-on list; `docs/reports/<card>/README.md`, which is
authoritative for that card's counts, boundaries and premises; direction in
`docs/GENERALIZATION_PLAN.md` (W0–W7) and `docs/ROADMAP_AFTER_CYCLE_9.md`'s
workstream table; trust/verdict rules and audit history in
`docs/REEVALUATION_SPEC.md` and `docs/REEVALUATION_FOLLOWUP.md`; the
architecture boundary and open gates in `docs/cycles/CYCLE_20.md`.

<!-- preserved:status:end -->
