# QCore-C agent briefing

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

Compressed project state for LLM agents; this file is the source of truth for
agents. Humans should read [README.md](README.md) for the summary and
[DEVELOPMENT.md](DEVELOPMENT.md) for the development table of contents —
repository layout, cycle records, per-increment evidence narratives and the
full build/validation instructions are divided under `docs/development/`.

**Sync rule (steady state since card DOCS-COMPACT, 2026-09-10).** Status
*history* lives under `docs/development/increments/`, indexed by
`DEVELOPMENT.md#increment-records` — one paragraph per increment, nowhere else.
This file carries only the compact ledger, the hard invariants, the standing
subsystem constraints, the per-change gate and the entry points. A new
increment therefore adds **one** paragraph, newest first, to the **current
record file named by** `docs/development/increments/README.md`, updates that
index when a new wave file opens, updates this file's headline numbers (and, if
it lands a standing "when touching X, do not Y" rule, adds it to the
constraints section below), and leaves `README.md` alone. Do not reintroduce
per-increment chronicles here, in `CLAUDE.md` or in the root table of contents.
A recurring compaction pass over these three files runs every 2–3 waves; the
cadence rule is in `CLAUDE.md`.

## Documentation retrieval and compact context

Prospective production-system cards and their relative priority live in
`docs/plans/fluent-bit-system-coverage/README.md`; these supplement the named
follow-ups, without changing CT coverage or assigning an implementation wave.

Use `DEVELOPMENT.md` as a table of contents, then read the relevant setup,
layout, cycle-evidence or increment page; the increment index names the current
record file and every historical wave. Historical counts and old instructions
describe their checkpoints, not current status. This file remains the authority
for agent rules; each report owns its own evidence, and frozen reports stay
untouched.

Retrieve headings and matching card names before reading long bodies: `rg -n`
with an exact card, symbol or heading in the relevant directory, then read the
surrounding section. Select relevant keys from large JSON records (for example
`named_followups` and `section_ownership`) instead of dumping a whole matrix.
Return commands, exits, counts and log paths — raw logs stay worktree-private
per `CLAUDE.md`'s hygiene rule — and fetch failure context only as needed.

A delegated brief supplies the task, worktree, declared base and HEAD, owned
and planned paths, topic, applicable rule references, report sections and exact
checks. Relevance includes **committed changes since the base**, staged and
unstaged edits, renamed/deleted/untracked paths and planned edits. Shared files
need the union of their path and topic rules. Unknown relevance requires more
authoritative reading, never silent omission. Navigation is not a substitute
for reading all applicable standing constraints before editing.

For documentation maintenance, preserve historical text and evidence links and
keep one current authority for each rule; new history pages should stay below
32 KiB where a complete record boundary permits it. Measure startup guidance,
cumulative retrieval and unique source bytes separately: byte counts are not
token billing, and a smaller index alone does not reduce this file's startup
cost. The measured restart plan and remaining packet/router work are in
`docs/plans/resume-2026-09-13/README.md`; the development split's preservation
evidence is in `docs/reports/development-docs-structure/`.

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
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization. Its six load-bearing modules, all under `crates/qcore-c-gateway/src/`: `legacy_c.rs` (generic unrestricted-C checker — Clang gateway, CFG, resource abstract interpretation, certificate recheck; contracts enter via `ResourceContracts`), `legacy_summary.rs` (W4 interprocedural custody summary inference: SCC-ordered derivation, validation by re-analysis, declared-contract precedence — constraint 4), `typed_source.rs` (opaque typed C path import and source def/use), and, added by the waves their constraints name, `legacy_location.rs` (wave 27, the presentation-only preprocessed → original `file:line` map — constraint 12a), `legacy_neutral.rs` (C03-NFR, the neutral-element route for undeclared recursive components — constraint 4) and `legacy_joined_provenance.rs` (wave 31, the conditional-provenance pre-pass, extended to cyclic CFGs by a greatest fixed point at wave 35 — constraint 10b) |
| `crates/qcore-verifier-host/` | L2 generic registry (0.4.0), source-scope resolution, bounded process transport, execution profiles, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter (0.4.0): explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection, per-unit projection and reports over W2 profiles/W1 contracts. Links no provider module |
| `crates/qcorec/` | CLI binary `quirin` (crate `qcorec`) plus its process-level tests |
| `compiler/src/` facades | `compiler/src/verifier_plugin.rs` is the compatibility facade for the host and first-party adapters; `compiler/src/verifier_plugins/{libevent,chunkio}/` are the in-process first-party providers (Libevent/Monkey, ChunkIO); `compiler/src/external_{integration,source}.rs` are compatibility facades plus Monkey-specific composition |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `providers/` | Separate six-member Cargo workspace (five `qcore-provider-*` packages plus the `conformance` harness), excluded from the core workspace. Artifact crates depend only on `qcore-schemas` and serialization; `providers/package.py` builds, verifies and archives the installable packages |
| `contract/qcore/` | Declarative `.qcore` contract corpus (**76 modules**); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `Audit*.lean` print axiom surfaces. The gateway-backing modules are **abstract rule theorems, not extraction of the Rust analyzer** and not whole-C correctness — every such module says so in its own docstring, no extraction, refinement or simulation theorem exists anywhere in `proof/`, and constraint 28 carries the two premises this leaves standing (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`) |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) — one directory per card |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking, schema 2) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. `crates/` holds nine crates; the core
Cargo workspace declares **ten** members (those nine plus `compiler`), and
`providers/` is a separate six-member workspace excluded from it.

Headline numbers after `JOINED-PROVENANCE-CALL-FRAMES` (2026-09-18). The **corpus
baseline is `docs/reports/joined-provenance-call-frames/artifacts/`** — wave 36's
second merged arbiter, whose BEFORE reproduces `w36-corpus-contracts-3`'s
`diagnostics.json` and `chunk-contracts.json` byte for byte and which moved
**zero** leaves, zero functions and zero units. Beneath it, `w36-corpus-contracts-3`
is wave 36's first merged arbiter and the chain's first **declarative** link: it changed no crate
source at all, so **one audit binary served both its stages** and the only
difference between them is which modules the contract manifest selects. Its
BEFORE reproduces `joined-provenance-cycles`' `diagnostics.json` and
`chunk-contracts.json` byte for byte. **The chain is what makes each
delta attributable**, and it is one
rule applied at every link: each arbiter's BEFORE run reproduces its
predecessor's verdict-bearing artifacts (`diagnostics.json`,
`chunk-contracts.json`) **byte for byte**, which isolates that arbiter's own
movement from the gateway change beneath it. Since wave 33 the identity is
**asserted inside the archiving tool**, which refuses to write if it ever stops
holding, rather than merely logged. The links, newest first:
`joined-provenance-call-frames` (wave 36, the second of that wave's two arbiters by merge order) over `w36-corpus-contracts-3` (wave 36, the first, declarative) over
`joined-provenance-cycles` (wave 35, the second of that wave's two arbiters)
over `c-gateway-constant-do-while` (wave 35) over `c-gateway-return-call-w4`
(wave 34) over `c-gateway-return-position`
(wave 33) over `w31-corpus-contracts-2` (wave 31, the second of that wave's two
arbiters) over `c-gateway-wave` (wave 31) over `w4-recursive-summaries`
(wave 30). Each arbiter's leaf attribution and gate live in its own report,
which is authoritative for them. All earlier archives remain frozen
(constraint 21): those seven plus `wave23-c-operations`, `aom-destroy-siblings`,
`aom-binding-poison`, `gateway-c-location` and `w29-corpus-contracts`.
Diagnostics retain two coordinates: the presented original
`file:line` and the internal preprocessed line used for pin, leaf and
certificate identity (`GATEWAY-LOCATION-INTERNAL-CHANNELS`, constraint 12a).

| Measure | Value |
|---|---|
| Cycle 19 raw corpus (185 units) | **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** — **no unit status has moved at any arbiter from wave 31 through wave 36**. 4,834 projected functions: **205 Safe / 84 provisional Unsafe / 4,545 Unknown**. `flb_input_chunk_create` remains function-level **Safe**. Leak **272**, `unknown_alias` **4,308**, `double_release` **0**, `unsupported` **49,922**, `summary_conflict` **1** — all five read from the wave-36 baseline named above. The function and `unsupported` chain, each delta attributed leaf by leaf in its own arbiter's report §5 and narrated in that wave's increment record: `C-GATEWAY-WAVE` (wave 31) four scalar counter helpers Safe → Unknown plus one constant-loop abstraction exposing a provisional repeated release, five function moves; `W31-CORPUS-CONTRACTS-2` **zero** functions and zero units, `unsupported` 50,155 → 50,150 on the four uncontracted-call leaves naming `cio_chunk_get_content_size`/`flb_input_chunk_total_size` decided by two declared contracts, plus one argument-expression refusal retired at its chained-actual site, zero leaves added and two units changed; `C-GATEWAY-RETURN-POSITION` (wave 33) **33 functions Unknown → Safe**, none withdrawn and none made provisional Unsafe, `unsupported` 50,150 → 50,117 as **33 withdrawn return-position refusals and zero added**, the return position widened to admit a pointer-valued bounded typed access chain; `C-GATEWAY-RETURN-CALL-W4` (wave 34) **7 further functions Unknown → Safe** (179 → 186 Safe, 4,570 → 4,563 Unknown, provisional Unsafe 85 unmoved), `unsupported` 50,117 → **50,106** as **17 withdrawn and zero added** — 14 return-expression refusals plus 3 second-order `call to uncontracted function` leaves decided by summaries newly derived from the bodies that card promoted — the position widened again to admit a *modeled call* whose result mode is `None` or `Owned`, `Reallocated` refused by name as the arm that would discharge the replaced parameter's obligation at the exit. `C-GATEWAY-CONSTANT-DO-WHILE` (wave 35) **one function provisional Unsafe → Unknown** (85 → 84 Unsafe, 4,563 → 4,564 Unknown, Safe **186 unmoved**, so nothing is promoted), `unsupported`, `unknown_alias` and leak all **unmoved**, and `double_release` **1 → 0** as **one withdrawn leaf and zero added** — the corpus's only repeated-release row, `rd_kafka_jwt_b64_decode_payload` at internal L21340, **withdrawn** because `do S while (0)` runs `S` once (C11 §6.8.5.2) and the CFG's back edge was a path the C does not have; the function keeps 18 other leaves and so goes to `Unknown`, not `Safe`, and four of the five artifacts are byte-identical with BEFORE. `unknown_alias` moved 4,208 → 4,214 at wave 34, all six added rows `Unknown`-class and in the accusing direction; leak **272** (266 → 272 at `C-GATEWAY-WAVE`: six added, none withdrawn) is unmoved at every re-pin since, constraint 7 carrying that chain in full, and `double_release` stood at **1** from wave 31 until wave 35 withdrew it. `JOINED-PROVENANCE-CYCLES` (wave 35's second arbiter) then moved **zero** leaves, zero functions and zero units, so all four counts above stand as its first arbiter left them: the conditional-provenance relation was extended to cyclic CFGs by a greatest fixed point, is exercised on 40 corpus functions, and is blocked at `release_site`'s nullable-site and naming premises at every one of the 14 sites that reach them. `W36-CORPUS-CONTRACTS-3` (wave 36's second arbiter, and the chain's first **declarative** link) then moved **21 functions and zero units**: **20 Unknown → Safe** (186 → 205 Safe, 4,564 → 4,545 Unknown) as six declared contracts decided the last blocker of twenty projected functions, and **one Safe → Unknown** — `rd_kafka_mem_free`'s own defining function, on the corpus's first **`summary_conflict`** (0 → 1), W4 deriving `Released{nonnull}` where the faithful declaration says `nullable` because `rd_free(NULL)` reaches `free(NULL)`; the declaration governs every call site and declaring `nonnull` to silence it would assert a precondition the body does not have (constraint 4), so this is constraint 8's gate working in the conservative direction, not a defect. `unsupported` 50,106 → **49,922** as **217 withdrawn and 33 added**, of which exactly **204 withdrawn leaves name the six declared callees** — the card's own ex-ante prediction, met exactly — and `unknown_alias` 4,214 → **4,308**, every added row `Unknown`-class and in the accusing direction, which is what modelling a release *does*: once a release is real, a later mention of the released identity is unresolved rather than unexamined. Leak **272 unmoved**, so constraint 7's pin needed no re-pin, and `chunk-contracts.json` is byte-identical across that card's two stages. Three predicted function moves did **not** happen: the `cJSON_Delete` trio exchanged its decided leaf for a `nested call … inside a modeled call` refusal that was unreachable while the outer call was uncontracted, which makes a sole-blocker cover an **upper bound** and never a forecast (`W36-COVER-HIDDEN-BLOCKER`, `CJSON-DELETE-NESTED-DETACH`). Provisional findings remain subject to the reports' coverage and reachability boundaries; the production `input_chunk_get` residual stays Unknown `JOINED-PROVENANCE-CALL-FRAMES` then moved **zero** leaves, zero functions and zero units — its AFTER is byte-identical to its BEFORE — so all counts above stand as `W36-CORPUS-CONTRACTS-3` left them: conditional provenance is now preserved across a call the frame shows it cannot write, which takes `release_site`'s naming premise from 0 of 14 to **6 of 14**, while the nullable-site premise still blocks all 14 |
| CT01–CT50 conformance | **46/50 mapped**, 4 unmapped; sanitized-native **10 of 46** (wave 32's `CT-COMPOSED-NATIVE` gave the `composed_epoch` route a runner that really compiles and runs an admitted case's own emitted C under the platform sanitizers and compares its stdout byte for byte with that module's own checked task half, moving CT22's and CT25's emission/native/sanitized rows off `unassessed`; CT23 stays out, being the route's refusal, and every arm of the runner was falsified before it was believed); **proof 3 of 46** — CT01 (wave 29, `CT01-PROOF-LEG`), CT37 (wave 30, `CT-PROOF-LEG-2`, chosen by a measured survey of all 42 mapped cases) and CT03 (wave 31, `CT03-PROOF-ALPHABET`, the first leg to need the F02 alphabet itself to grow — by the Buffer write, proved **conservative** so that a write-free trace relates exactly as before and no existing digest moved — while the *sibling* arm of that same alphabet decision, the two lease events still refused, was at the time **observed and not proved**, the occasioning case for `CLAUDE.md`'s `STABILITY-REEVAL` cadence; wave 32's `TRACE-ALPHABET-LEASE-BOUNDARY` **discharged** that residual and constraint 15n now carries the measured statement — the arm is **reachable and driven**, dropping is proved an *admitting* mistake rather than a lossy one, and the refusal's index is proved to be the first offending event's own, so this row no longer says "no test drives it" (the forward question, whether the observable alphabet should grow once a worker exercises its authority, stays open as `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`)): each case's scenario property proved in Lean of the checked artifact's reference run and its admitted renderings (CT37's completion strictly before the join in every admitted rendering), bound to the case by a named host-unconditional test through the F02 digest relation, native execution deliberately observation-only; the per-case gate is the pinned fail-closed `PROOF_GATED_CASES` table in `ct_corpus.rs`, and each flip's stage evidence supersedes card T00's frozen archive from its own record (constraint 21). Wave 28 (`CT-WIRE-LANE-2`) mapped CT07, CT08, CT09, CT21 and CT23 (CT07 stating the read-loan/retained-write boundary; CT09's layout leg an executed *observation*; CT23 on the `composed_epoch` route); wave 29 (`CT22-WIRE`) mapped **CT22** on `REGION-CREDIT-JOINS`' evidence with the wave-28 narrowed reason retired in its own record; wave 30 (`CT07-WRITE-REMAP`) upgraded **CT07's evidence** onto the write-lend scenario — two write authorities over disjoint ranges of one buffer, both held by one sequential entry's bindings, **not two threads** (`WRITE-LEND-TASKS` the residual) — superseding the wave-28 record's CT07 entry from its own record with **no count moved** (evidence quality, not coverage). Wave 31 (`CT-WIRE-LANE-3`) mapped **CT25** — *cleanup destroys owner after joins and loan expiry*, every word carried by an unedited model's checked fact — and **examined CT24 and withdrew it** with the measurement published: on the only route with an executed cleanup-destroy a region before the join refuses for its *placement* whatever its duties, so no control isolates the destroy, and mapping it would move the derived reasons map's keys against a matrix the lane may not regenerate (constraint 25, measured rather than assumed). Wave 32 (`CT-WIRE-LANE-4`) mapped **CT47 and CT44**: CT47 on the transferability judgment's refusal at the crossing construct — verified in source that the owned buffer is `MovedOnly` and *sendable*, so the refusal is for the **mode** and not a vacuous kind refusal, with all three contracts the kind does carry supplied as admitting controls — and CT44 after measuring both of card T00's archived blockers **obsolete** (the refusal names the borrowing task, and "environment moves before join" now has a source form), attribution separated in both directions. Each publishes the boundary its scenario's untranslatable words rest on (no pointer type; the storage is heap-bound, so "stack" has no referent). Wave 35 (`CT-CTWIN-LANE`) mapped **CT46** on card `MUTEX-SHARE-CROSS`'s composition, under a new `checked_mutex_crossing` verdict mode on the `mutex_guard` route — the corpus's first **admitted** case that deliberately stops at the checker: the module is accepted and the C backend renders nothing for it, `interpret_mutex_program` refusing the `Spawn` the crossing introduces, which is `MUTEX-SHARE-CROSS-EMISSION` measured rather than inherited and is why four stages read `unassessed` and none reads `executed`. The emission boundary is pinned in both directions, one control being the same route without the clause, which does render and does interpret. Two of CT46's own words are published as untranslatable — there is no wrapper type and no `Send`/`Sync` trait — both carried instead by the transferability judgment's `TransferKind::ProtectedResource`, asserted against `MUTEX_SHARE_CROSSING` itself. The same lane **re-measured the four remaining cases and mapped none**: CT41's obstruction is **structural** — clause (b) of the wave-9 attribution rule needs a control in which removing `volatile` moves the verdict, and `CTWIN-PUBLISH` §5's decision is that it moves nothing — with the scenario's own consumer shape now measured **`safe` with no diagnostic**, and the typed-source arm examined for the first time and found unable to tell `volatile` from `_Atomic` (`CT41-TYPED-SOURCE-UNDISCRIMINATING`); CT42 stays unmapped with **both** arms measured, the socket arm being **unreachable from source** (`CT42-CELL-UNROUTED`); CT10 stays unmapped on half a scenario; CT24 stays withdrawn. A grown denominator is mapping, not conformance, and a grown proof numerator is evidence about one case's artifact (constraint 23) |
| Contract corpus | **82 modules** (73 → 74 at wave 29: `fluent_bit/input_chunk_sizes.qcore`; 74 → 76 at wave 31: `chunkio/chunk_sizes.qcore` and `fluent_bit/input_chunk_total_size.qcore`; 76 → 82 at wave 36 (`W36-CORPUS-CONTRACTS-3`), six declarations chosen by **function-cover yield** over the 193 baseline functions blocked only by uncontracted-call leaves — `cfl/sds_lifetime.qcore`, `cmetrics/context_lifetime.qcore`, `ctraces/context_lifetime.qcore` (the corpus's first `ctraces` family), `librdkafka/cjson.qcore`, `librdkafka/kafka_mem.qcore` and `monkey/logging.qcore`, the last carrying this corpus's second `resource_variadic_data_tail()` — each deliberate and each read line-cited from the pinned body before it was declared; resource-v1 declarations 39 → 41 → 43 → **49**, manifest-selected modules 18 → 19 → 21 → **27**) |
| `qcore-schemas` | 14.0.0 · `qcore-compiler` 0.4.0 · `qcore-verifier-host` / `qcore-verification` 0.4.0 · ChunkIO provider package 0.13.0 |
| Merged gate | **4,580 passed / 0 failed / 8 ignored, 228 test executables / 230 result summaries** · 122 provider (carried; **Unix** — see below) · **247 Python** · **135 Lean audits** · both probes exit 0 · fmt/clippy clean. Measured by the **wave-35 close** at its own base `4cf9bdf9` (the `JOINED-PROVENANCE-CYCLES` merge), `git submodule update --init --recursive` run first and exit 0 (`WORKTREE-SUBMODULE-UNGATED`); one uninterrupted `cargo test --workspace --no-fail-fast` of 15m54s, **every step's exit captured to its own file with no pipe in the command** — a `| tail` swallowed a gate exit for the integrator earlier in this wave. Per-command exits in `docs/reports/wave35-close/gate.json`, and each card's own gate in its own report. **A gate figure is a measurement on one machine, never a property**, and two qualifications travel with this one. First, `qcore-verifier-host`'s `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` was **green here**, and this row says so because constraint 16 requires a gate report to state which it saw: this is the **first full-suite observation after** card `VERIFIER-HOST-CONTROL-BUDGET` measured the mechanism (an 8 KiB pipe capacity against a 1 ms backoff quantum) and repaired it with `poll(2)` **without touching the test file**, and it agrees with that card's 38/38 — but it is a full-suite run on a machine carrying its own build at load ≈6.5, **not** the quiet-host isolated number `HOST-ROW-STARVATION-DESCRIPTION` still asks for, and it is not offered as one. Second, provider **122 is carried, not re-measured** (`git diff` over `providers/` and `crates/qcore-schemas/` across the whole wave is empty, so constraint 18 is not engaged) **and is a Unix figure** (`PROVIDER-CONFORMANCE-UNIX-ONLY`); its `RLIMIT_NPROC` row was separately measured **unverifiable under this wave's own conditions**, five sibling cards being *inside* its measurement rather than ambient load (`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`). Prior figures, for the delta only: 4,559 / 0 / 8 at the wave-34 close, 4,544 / 0 / 8 at wave 33, 4,534 / 0 / 8 at wave 32 |
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
and card paragraph keeping its own checkpoint and boundaries); open
follow-ons in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`,
**190** entries at the wave-36 close (+19: the **18** residual-shaped names its
sweep found reaching no tracker — all nine of `W36-CORPUS-CONTRACTS-3`'s §9 and
all nine of `C-GATEWAY-INDIRECT-CENSUS-2`'s, both cards having left them to the
close deliberately — plus the close's own `DECISION-ALSO-REGISTERED-AS-FOLLOWUP`;
**retirements: none**, stated because the union merge loses an unnamed removal).
The wave's own chain: 171 after WRITE-LEND-TASKS-EXERCISE (+7 −1), 165 after
MUTEX-SHARE-CROSS-EMISSION, 161 after JOINED-PROVENANCE-CALL-FRAMES (+3 −2),
159 after STABILITY-REEVAL #2 (+31), 128 after CARD-SCOPE-GUARD (+7), from
**121** at the wave-35 close, which itself registered **28** names — 27 of the
62 its widened sweep returned, plus `DOCS-COMPACT-4-TABLE-SYNC` — and retired
**10** as discharged by named cards; the section is *carried*,
`dimensions.named_followups` is
*derived*, and the two are checked against each other on the committed file, so
add entries through `docs/reports/remaining-d02/build_matrix.py` and never by
hand — constraint 25) plus each report's own follow-on list; per-card evidence in
`docs/reports/<card>/README.md`, which is authoritative for that card's counts,
boundaries and premises; direction in `docs/GENERALIZATION_PLAN.md` (W0–W7) and
`docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table; trust/verdict rules and
audit history in `docs/REEVALUATION_SPEC.md` and `docs/REEVALUATION_FOLLOWUP.md`;
the architecture boundary and open gates in `docs/cycles/CYCLE_20.md`.

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
   recursive component **above the neutral element**. The one admissible
   optimistic start is `N(m)` — every formal `Borrowed`, result `None`, seeded
   for every member of an undeclared component and descended to the greatest
   post-fixed point (`legacy_neutral.rs`, card C03-NFR) — and it is admissible
   on exactly two premises, both load-bearing: (1) `N(m)` states **no
   precondition**, installs no entry resource and asserts no non-nullness, so
   it cannot delete a failure path — the hazard this sentence names; (2)
   `N(m)` is a conjunction of **negative, prefix-closed** facts, so its
   soundness is an induction on execution prefixes
   (`QCore.NeutralPrefixPreservation.neutral_prefix_preservation`) rather than
   a termination rank, and divergence cannot manufacture it — while validation
   by re-analysis alone provably cannot refuse a circular *effectful*
   candidate (`an_assumed_effect_appears_in_no_divergent_prefix`). Any route
   that seeds any element above `N(m)` for a member of its own component must
   supply a well-founded rank, as the contextual and countdown routes do.
   Declared contracts always win; a disagreement is a `SummaryConflict`
   diagnostic, never a silent override.
   **"Defined in this translation unit" is not "W4-derivable", and a reach
   argument that counts the former overstates this route by two orders of
   magnitude.** Validation by re-analysis requires the callee's *own* body to be
   fully modeled and defect-free, so a callee that is itself `Unknown` yields no
   summary and its callers gain nothing from it. Measured over the 643
   `return CALL(...)` refusal leaves of the pinned corpus, on one population so
   the shares are comparable: **379 (58.9%) have a callee defined in the same
   `.c`**, but the callee's own body is `Safe` — the precondition for a
   validated summary — in **4 (0.6%)**; the other 375 are 366 `Unknown` and 9
   `Unsafe` bodies, and 264 (41.1%) of the 643 are external. A card scoped on the larger
   figure will not find the smaller one waiting for it: wave 34's arbiter
   withdrew 14 such leaves, of which **13 were decided by a declared `.qcore`
   contract and exactly 1 by this route**. `w4-custody-inference`,
   `w4-custody-extensions`, `w4-gateway-crate`, `remaining-c03`,
   `w4-recursive-summaries`, `c-gateway-return-call-w4` §3.3.
5. The typed scalar-field whitelist's last premise — the base is a name this
   function never binds to a resource identity — exists because dropping it
   turned a read through a released pointer into `Safe`. The fail-closed pin
   caught it and **was not weakened**; keep it that way. `remaining-c02`.
5a. A fixed C `sizeof`/`_Alignof` query's operand-class marker is **never a
    computed value**. Do not use normalized query tokens to construct a CFG or
    refine nullness. Scalar assignments forget old scalar facts; expression
    mutations need modeled writes rather than a `NoOp`. Standalone declared
    scalar increments/decrements and custody-neutral scalar assignments use
    `ForgetScalar` when no precise value is modeled. This includes enum,
    compound and scalar-field assignments: `flag = !0` must not retain an old
    zero. Multiple scalar writes and expression-position assignments or
    increments remain refused until their evaluation is modeled.
    `wave23-c-operations`, `c-gateway-wave`.
5b. **A `return` admits what a condition admits, and the difference escapes
    nothing.** Card `C-GATEWAY-RETURN-POSITION` (wave 33) amends card
    REACH-GATE's return-position sentence **deliberately**: a pointer-valued
    bounded typed access chain leaf is now admitted in a `return`, so
    `ExpressionRole::Returned` and `ExpressionRole::Decides` admit the **same**
    expressions. What the distinction still decides is the parser's answer, and
    it must stay decided there — an expression the strict `Returned` rule admits
    keeps its `canonical_place`, so `return p;` still escapes `p` and the
    discharging direction is untouched; an expression **only** the `Decides`
    rule admits is answered `returned_place: None`, making the node a `NoOp`.
    **Do not answer the second case with `canonical_place`**: it spells
    `view->head->next` perfectly well, and escaping it would be the discharging
    direction on a place the admission never established (invariant 1). It is
    sound to answer `None` because `admitted_chain` refuses a bound base while
    every tracked `->` place is rooted at a body-acquired name, which `may_bind`
    reports bound by construction. **That second half is a premise, not a
    theorem, and this constraint said so only from wave 34**
    (`RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`).
    `QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place` is
    **conditional on `TrackedBasesAreBound`**, which it takes as an explicit
    hypothesis; the Lean module states in its own words that this is "a premise,
    stated and not proved here" — that the projection tracks no place whose base
    the body does not bind — resting it on the aggregate store's construction
    fact in `QCore.AggregateStore`. The theorem therefore discharges the
    `admitted_chain` half and **assumes** the `may_bind` half, which is the
    load-bearing one; the Lean file is more explicit about this than the ledger
    was. The widening's refusal-monotonicity is unconditional
    (`the_widening_never_creates_an_escape`). This is not academic: a card that
    widens member typing far enough to put a projection-bound place on an
    admitted chain falsifies exactly this premise, so re-read it before taking
    `FB-BACKLOG-CUSTODY`.
    Two boundaries are **measured, not assumed, and are reported as findings**.
    The `None` answer is a **backstop no test distinguishes**
    (`RETURN-POSITION-ESCAPE-UNREACHABLE`): replacing it with `canonical_place`
    leaves the suite green, because the base premise makes that escape a no-op —
    `the_two_answers_agree_on_every_admitted_chain`. Do not simplify it away
    (`without_the_base_premise_the_answers_part` shows the answers separate the
    moment the premise weakens), and do not read the green suite as evidence for
    it; read the theorem. And the **accusing** arm is proved and unit-tested but
    **reaches no function of the pinned corpus** — zero leaves added at the
    arbiter — so the unchanged leak pin records that absence rather than that
    arm's verification (`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`).
    `c-gateway-return-position`.
5c. **A constant `do … while (0)` is sequential composition, and the key is
    the token.** `do S while (0)` executes `S` exactly once (C11 §6.8.5.2),
    so its CFG header has `continuation` as its **only** successor and the
    back edge to `body_entry` is a path the C does not have. Removing that
    edge is the **admitting** direction (invariant 1), which is why three
    things must survive refactors. The rule is keyed on the **single literal
    token `0`** and never on an evaluated expression — constraint 5a's rule,
    applied here — so `while ((0))`, `while (1 - 1)`, `while (0 == 0)` and
    `while (0 && 0)` keep the back edge, and answering "not literal zero" is
    the conservative answer, so any widening of `condition_is_literal_zero`
    (the unclaimed spellings `0u`, `0L`, `0x0`, `'\0'` included) is an
    admitting change needing its own measured re-pin. The **exit** edge is
    never the one removed and the header node is retained, so the rule removes
    a path and never a destination, and no leaf's internal coordinate moves
    for an unrelated reason. And the rule does **not** generalise to
    `while (1)`: modelling that exactly would remove an *exit*
    edge, which is the accusing direction and needs its own Lean statement and
    re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`). A pre-existing route already
    removes the **exit** edge for `for (;;)`: `condition.is_empty()` at
    `legacy_c.rs:3312` sets `condition_always_true`, which `legacy_c.rs:5161`
    uses to drop `continuation` from the header's successors. It is exact
    (C11 §6.8.5.3) and `break` still reaches `continuation`, but it carries
    neither a Lean statement nor a pin, and it is not owned by card
    `C-GATEWAY-CONSTANT-DO-WHILE` (`FOREVER-FOR-EXIT-EDGE-UNOWNED`).
    The 32 `unmodeled call,
    aggregate or indirect access in do-while condition` leaves are a different
    question and stay refused — measured unmoved at 32 across the re-pin.
    Proved as `QCore.ConstantDoWhile`: `constant_do_while_is_its_body` the
    `≡ S` equality, `removing_the_back_edge_only_removes_paths` and
    `the_successor_list_never_grows` the conservativity for **every**
    condition, `the_repeated_release_is_a_path_the_c_does_not_have` the
    non-vacuity. Separately, and **not owned by that card**, a pre-existing
    route already decides a `do`-`while` from a *modelled scalar value*; it
    answers conservatively wherever measured and is pinned by
    `a_modeled_scalar_condition_is_a_separate_pre_existing_route`
    (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`). Finally, removing a back edge
    **makes a function acyclic**, and that changes which route
    `legacy_joined_provenance::derive` takes for it (constraint 10b) — so this
    rule silently changes *that* pre-pass's reach: **654** of 4,771 analysed
    functions at this card's re-pin, with a measured diagnostic effect of zero.
    Any widening of the key therefore moves two subsystems and must re-measure
    both (`docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py`,
    `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`).
    **Two clauses of this paragraph are the integrator's, not the card's**, and
    the wave-35 close's `reconciliation.json` carries both before-and-after
    texts: the card wrote that `derive` "returns no conditional-provenance facts
    at all for a cyclic CFG", which wave 35's **second** arbiter
    (`JOINED-PROVENANCE-CYCLES`) falsified later in the same wave by solving a
    cyclic CFG with a greatest fixed point — landing that sentence verbatim
    would have put a statement contradicting 10b into this file — and the card's
    script path was its own gitignored `measure/`, replaced by the committed
    copy it archived. The rule itself is unchanged, and the residual it names is
    **answered** by that second arbiter's §8, which reproduced the 654
    independently and measured the flip's cost at six functions and 20 leaves of
    servable population — reach that moved to the topological pass, not reach
    lost. `c-gateway-constant-do-while`, `joined-provenance-cycles`.
6. **C02-P-ALIAS-LEAK-ORDER:** `report_leaks` on intermediate iterates is
   scheduling-sensitive; a pre-pass keeps it scheduling-neutral with
   `AbstractState` byte-identical. Recorded, not repaired — do not disturb it
   incidentally. `remaining-c02-p-alias`, `remaining-borrow-liveness`,
   `remaining-hygiene-14`.
7. Leak diagnostics are **272** across the pinned 185-unit corpus. A change
   that moves that number is a finding to attribute in the re-pin, not to
   absorb. Through wave 30 it had moved four times, each deliberately and each
   either in the *accusing* direction or by **deciding** a leaf. The current
   six-row increase is attributed after that historical chain below.
   271 → 272 (wave 17): `flb_input_chunk_get_direct_routes` L19399,
   accusing, on a function and a unit already `Unsafe`, held provisional as
   `CONSTRUCTOR-RESIDUALS-LEAK-271`. 272 → 273 (wave 20): `input_chunk_get`
   L19849, accusing — modelling the constructor exposed a release through the
   then-uncontracted `flb_input_chunk_destroy`. 273 → 272 (wave 22): that same
   leaf decided by the faithful `flb_input_chunk_destroy` contract — a
   **withdrawal**, not a discharge, since the release refuses at the call on
   joined provenance and `input_chunk_get` goes provisional `Unsafe` →
   `Unknown`; residual `AOM-DESTROY-JOINED-PROVENANCE`. 272 → **266**
   (wave 23): the faithful **`nullable`**
   `flb_input_chunk_destroy_direct_routes` contract (`nullable` where its
   already-declared neighbour carries `nonnull`, because that body's third
   statement is `if (!routes) { return; }`) retires six leaves of
   `flb_input_chunk_get_direct_routes` — L19384, L19392, L19399, L19434,
   L19489, L19505 — again by **withdrawal-by-refusal**: at each preceding call
   site the projection cannot resolve the actual it is handed, so the site is
   marked through the unresolved-effect mechanism and `report_leaks` skips it.
   The cause is that every refused statement poisons every pointer binding in
   the state, so an acquisition's binding does not survive to its release —
   residual `AOM-SIBLINGS-BINDING-POISON`, whose repair moves those sites
   toward the *discharging* arm, a direction invariant 1 constrains. That move
   **DECIDES — withdraws** `CONSTRUCTOR-RESIDUALS-LEAK-271` at a measured cost
   of **five** standing claims, not the six wave 17 recorded: the sixth leaf,
   L19544, is on the success path where no destroy call runs, so it **stands**,
   the function keeps its leak and its provisional `Unsafe`, and the pin reads
   266 rather than 265. Nothing is promoted by any of the four: zero unit status
   moves, zero function status moves. At the wave-25 re-measurement there was
   **no fifth move**: the L19399 leaf wave 23 decided by *withdrawal* is decided instead by the **discharging** arm of that same contract, its first
   firing anywhere in the pinned corpus: a strictly better answer to the same
   question, changing no count, the pin still **266** with not one leak leaf
   added or removed. The wave-27 arbiter also made **no further move**: it
   re-pinned *presentation* only, and the 266 leak rows are exactly the 266 whose
   description text was rewritten — each verified to be the substitution of the
   **site alone**, the text on both sides of the phrase byte-identical. A leak
   leaf's identity is still its preprocessed line.
   `remaining-o04-b08`, `corpus-repin`,
   `remaining-constructor-residuals` §6.4, `wave17-corpus`,
   `wave20-abi-address-of-member`, `wave20-corpus`, `wave22-aom-destroy`,
   `wave22-corpus`, `aom-destroy-siblings` §6 and §6.1,
   `aom-binding-poison` Headline and §5.2, `gateway-c-location` §5.3.
   The current `C-GATEWAY-WAVE` re-pin advances **266 → 272**: six added
   provisional leak leaves, no baseline leak leaf withdrawn. Two arise from
   complete-expression frames and four from nullness refinement; every site is
   listed in `docs/reports/c-gateway-wave/README.md`'s final measured result.
   Four intermediate missing leaves were restored by the scalar-assignment
   repair before publication. The original worklist schedule is retained;
   neither the leak rows nor the new constant-loop repeated-release row is a
   validated upstream defect claim.
   At the wave-34 `C-GATEWAY-RETURN-CALL-W4` arbiter the pin is **unmoved at
   272** — the first gateway *admission* card in this chain to move no leak leaf
   in either direction. That is not luck and is worth the sentence: the exits it
   newly models are allocator wrappers that retain nothing, so exposing them
   accuses nobody. Admitting a previously unmodeled exit remains a move in the
   accusing direction by construction, because `report_leaks` **skips** an
   unmodeled exit; a future admission card reaching bodies that do retain should
   expect the pin to rise, and that rise is a finding to attribute, not a
   regression. `c-gateway-return-call-w4` §5.
   Card `W36-CORPUS-CONTRACTS-3` (wave 36) moved the number **not at all**:
   272 → 272 across a re-pin that added six declared contracts and moved 21
   function statuses. A declarative card that decides 204 uncontracted-call
   leaves is therefore not automatically a leak-pin event, and the pin was
   re-verified rather than re-derived.
8. **A declared contract must agree with the C the unit declares** — in types
   and in modes. A contract whose parameter or result ABI projections the
   analysed translation unit's own C declaration contradicts, or whose custody
   *duty* the validated W4 summary of that callee's body in this unit
   contradicts, is **withdrawn for that unit**: every call site is rewritten to
   a marked uncontracted callee (`mode-parameter-contradiction` /
   `mode-result-contradiction` for the mode arm) and the name is removed from
   the environment the derivation seeds. Both ABI sides are read from what a
   certificate already holds (the preprocessed token stream; the environment's
   retained texts), never from the typed Clang AST, so W4 derivation, W4
   validation and certificate rechecking reproduce the decision — do not move
   either reader. A withdrawal is monotone in the refusing direction but **can
   turn a `Safe` unit into a provisional `Unsafe`** when the withdrawn
   declaration was discharging an acquisition; that is `Unknown`-class evidence
   about the declaration, never an upstream defect claim (invariant 2). The
   derived summary is **never installed in the declaration's place** — an
   inferred summary can be weaker than the declaration it convicts. The mode
   gate takes the *duty* inequality
   (`Retained`/`Transferred`/`Delivered`/`Restored`; `Absent`/`Produced`) and
   deliberately does **not** withdraw on a refinement inside one duty:
   nullability, a `Stored`/`Removed` owner index and a `Reallocated` replaced
   parameter keep `SummaryConflict` as their only report, because every
   derivable comparison in the pinned corpus is a nullability refinement and
   withdrawing on inequality would withdraw a release contract in every unit
   including `rd_free`/`mk_mem_free`/`flb_free`'s headers. Absence of evidence
   is not evidence: nothing is withdrawn on a body this unit does not define or
   W4 derives no summary for. The scalar projection assumes the LP64 model of
   the audited targets and the reader has no target (`LOADER-ABI-LP64`).
   `remaining-loader-abi`, `remaining-loader-abi-modes`.
   The mode gate's `SummaryConflict`-only arm for a **nullability
   refinement** was exercised for the first time by a *declaration* rather
   than by the corpus: card `W36-CORPUS-CONTRACTS-3`'s `rd_kafka_mem_free`
   declares `Released{nullable}` where W4 derives `Released{nonnull}`, and
   the declaration is the faithful one — `rd_free(NULL)` reaches
   `free(NULL)`. The declaration governs every call site, the defining unit's
   own function moves `Safe → Unknown`, and **that is the gate working**: a
   disagreement is reported, never silently resolved, and declaring `nonnull`
   to silence it would assert a precondition the body does not have
   (constraint 4). `w36-corpus-contracts-3` §5.2.
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
    admitted on stated grounds, and the decision reads no per-site status and no
    provenance (one exclusion, a declared *scalar* source, was added by
    measurement). **The borrowed and `Out` admissions are withdrawn** (wave 17):
    a body publishing a *borrowed* formal no longer certifies, because
    `Borrowed` asserts the caller's obligations are unchanged and a publication
    falsifies exactly that — the launder it enabled crossed the **summary**
    boundary, so W4 published `Borrowed` for the helper and the helper's caller
    certified too. A declared `Out` slot naming such a destination is refused
    before the call's effects apply, being the `Acquire` arm seen from the
    callee's side. The indirect (pointer-base) shape carried no false `Safe`, so
    its residual *violation* claim is withdrawn rather than strengthened, each
    statement keeping the refusal it already had. `remaining-ctwin-publish`,
    `remaining-ctwin-residuals`.
10a. **A refused statement invalidates only the bindings it could have
    written.** The blanket poison — every refused statement setting `unknown`
    and `may_be_null` on every binding in the state, including bindings its own
    text cannot have written — is bounded, and only where the reader positively
    recognised all three premises: a plain name, an automatic object of this
    body whose address is never taken, and no write position for that name in
    the refused text. The allow-list is built **positively**, so an
    unrecognised place is always poisoned. A parser-delimited complete
    condition or return expression now carries its own write frame; a
    structural control-flow failure without complete expression text **keeps
    the blanket poison**. Prefix mutation through parentheses is a write
    (`++(p)` must not spare `p`). The refusal
    itself is never weakened: the `Unsupported` diagnostic and `unmodeled`
    path flag are retained, which prevents this rule from certifying a refused
    function. Exposing an additional violation can make its status provisional
    `Unsafe`. Wave 25 measured
    zero unit/function status moves and an unchanged leak pin; the condition
    extension's separate measurements are in `c-gateway-wave`. The frame
    premise is the ordinary C object model under the **no-undefined-behaviour**
    reading (`AOM-POISON-NO-UB`), and the declaration reader
    (`publication::body_declarations`) over-approximates in the direction that
    *reduces* refusal, so the file-scope exclusion is applied unconditionally
    here rather than with `publication::destinations`' shadowing exemption
    (`AOM-POISON-DECLARATION-READER`). Do not widen `may_write_names` without
    measuring: sparing a binding can **unmask** a stale scalar fact a `NoOp`
    mutation left behind (`AOM-POISON-NOOP-INCREMENT`, constraint 5a's already
    recorded hazard). `aom-binding-poison`, `c-gateway-wave`.
10b. **Non-nullness does not establish lookup identity.** A null guard on an
    unknown binding may narrow only that binding; it must retain its identity
    refusal and may not remove resources or refine another alias. Conditional
    provenance is derived separately from the custody worklist, for unshadowed
    automatic locals whose addresses are never taken:
    a name denotes a local site on every path where that site
    exists. An absent predecessor is neutral; present predecessors intersect.
    No caller custody is seeded. A **call** forgets every relation it could have written and keeps only those it provably could not (card `JOINED-PROVENANCE-CALL-FRAMES`, wave 36): it forgets its destination and every `Out` slot — both already removed by `rebound_names` — and a `Reallocated` parameter, and it forgets every name outside `cfg.conditional_names`, the unshadowed automatic locals of this body whose addresses it never takes. A name on that list designates a slot no callee can reach under the same no-undefined-behaviour C object model constraint 10a rests on, so the call cannot change what it denotes; passing the name as an argument hands over its value, never its slot. An unrecognised name is always forgotten, and the rule only ever weakens (`QCore.JoinedProvenanceCallFrames.a_call_never_adds_a_name`), the pre-card blanket forget being exactly this rule at the empty frame (`the_empty_frame_is_the_old_blanket_forget`). An **acyclic**
    CFG is solved in one topological pass; a **cyclic** CFG is solved by the
    greatest fixed point of the same transfer (card
    `JOINED-PROVENANCE-CYCLES`, wave 35), which derives facts **only for sites
    whose creating node lies on no cycle** — a site created inside a loop stands
    for one abstract resource per *site* rather than per dynamic object, so a
    per-site identity fact would not be a statement about one of them — and
    which declines the function outright if it has not settled within its
    iteration bound. The two routes agree wherever both apply: the fixed-point
    equation has exactly one solution on an acyclic graph
    (`QCore.JoinedProvenanceCycles.acyclic_solution_is_unique`), so no acyclic
    body's verdict can move, and soundness on a cycle is
    `fixed_point_is_sound`, proved by induction on the arrival with no
    acyclicity hypothesis. Refusing is always available and always safe
    (`weakening_stays_sound`). A conditional shallow release may settle a known local
    site's disposition only under the checked premises in
    `legacy_joined_provenance::release_site`, retaining the lookup's
    `UnknownAlias` and every other uncertainty. Never apply it to overwritten
    live acquisitions, unresolved prior effects, child/reallocation relations,
    or additional failed call premises. This does not close the production
    `input_chunk_get` residual: its address-taken binding crosses other refused
    operations, and **cycles were never that residual's blocker** — card
    `JOINED-PROVENANCE-CYCLES` measured that function's CFG acyclic, so the
    wave-31 pass already served it. `c-gateway-wave`,
    `joined-provenance-cycles`, `joined-provenance-call-frames`.

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
12a. **A presented coordinate is derived, and no file spelling is ever read from
    a flagless marker.** A gateway diagnostic presents the original,
    pre-preprocessing `file:line`; the preprocessed `L`-number stays the
    **internal** coordinate of the certificate, `source-effects.json`,
    `chunk-contracts.json`, the census APIs and the probe examples, which is the
    user's ruling and not an omission (`GATEWAY-LOCATION-INTERNAL-CHANNELS`). The
    coordinate is derived by pairing the analysed `-E -P` text with a second,
    **presentation-only** marked `-E` run, and the pairing is **checked, not
    assumed**: same length, same trimmed text line for line, or the whole map is
    refused. Three rules carry the anti-forgery property and must survive
    refactors — a frame's spelling is adopted **only** from an *entering* marker
    (flag `1`), so a `#line`, which Clang re-emits flagless, can never write one;
    include depth 0 is not named by a marker at all but resolved by the caller
    against the audit's own configured file, which is what keeps the pinned
    transport's guessable `<stdin>` out of every report; and a flagless marker
    naming a file other than the one its frame was entered as **withdraws** the
    coordinate for every line it governs rather than relabelling it, the
    diagnostic falling back to `preprocessed line N`, which names itself as
    internal. Proved in general as `presented_files_are_entered_files`
    (`QCore.SourceCoordinate`), not on witnesses. What a `#line` *can* still do is
    renumber the file it is written in, which is its C11 §6.10.4 meaning and
    indistinguishable from one of Clang's own resynchronisations
    (`GATEWAY-LOCATION-LINE-ONLY-DIRECTIVE`) — it moves no file identity, no
    include depth and **no audited scope**, so constraint 12 is untouched: the
    scope reader (`legacy_enumeration`) is not read, not written and cannot be
    influenced by this module, and the map is built only *after* a unit's result
    is settled. All four refusal grounds are
    `ReasonCategory::EnvironmentUnavailable` on the producer-supplied path, so
    §L21.1 rule 1 applies and none can contribute to a provisional `Unsafe`; a
    refused unit keeps every internal position and is never given an approximate
    coordinate. `gateway-c-location`.

**Concurrency source routes**

13. Concurrency is opt-in: a file declares `import std.concurrency;`, the
    context is unavailable by default, and imports do not leak across
    compilation inputs (`Q014`/`Q015` refuse before checking each file). The
    task profile keeps its separate `concurrency checked;` marker.
    `core-concurrency-prelude`.
14. New keywords are a recorded compatibility hazard: every candidate is
    **censused against every fixture** before it is reserved, and the census is
    run rather than assumed. The ledger, in card order:
    * **Reserving cards.** T03-F **+5**; T08-EPOCH-SRC **+8** (with `work`
      renamed `effort` rather than editing the pinned golden surface);
      `source.extern-call.v1` **+1** (`succeeds`); STD-MODULES **+8** (`print`,
      `log`, `flush`, `dec`, `hex`, `info`, `warn`, `fatal`); the
      quiet-emission amendment **+2** (`trace`, `debug`) plus the explicit
      system-trace `log_level` declaration, after a full fixture census. The
      already-reserved `error` keyword is accepted contextually as a logger
      level **only after `log`**; it was not removed from the global keyword set.
    * `QUIRIN-SOCKET-V1` **+10** — `tcp`, `udp`, `bind`, `listen`, `connect`,
      `accept`, `send`, `recv`, `sendto`, `recvfrom` — each a clause word in a
      statement and never a reference, each measured load-bearing, all ten
      census-clean across every `.qcore`/`.qc` fixture (412 files) and every
      Rust string literal (41,529). `let`, `close` and `return` are reused
      rather than respelled, which is why the release is `close NAME;`. Two
      candidate spellings were **rejected by the census and must stay
      rejected**: `stream` (a contract `extern fn` parameter name, shielded
      only by the following `:`) and `socket` (it would name a transport the
      route deliberately makes explicit at creation).
    * `LOOP-V1` **+1** (`for`, census-clean at zero code occurrences across all
      418 `.qcore`/`.qc` files) and **no new token**, `..` being two existing
      `Dot` tokens. **`in` is deliberately not reserved**: the census found it
      as a genuine reference inside the pinned contract corpus
      (`ensures normal resource_borrowed(in);`), exactly the shape that kept
      `len` out of the global list, so it takes that card's narrow contextual
      rule instead. Do not promote it to the global set.
    * `LOOP-UNROLL-BUDGET` **+0 words — a first for this family — and +1
      token**, `#`, promoted from a lexer error on `OR-AND-LOWERING`'s
      precedent for `^` and census-clean at **zero** code occurrences across
      422 `.qcore`/`.qc` files. It reserves nothing because a `[| … |]` clause
      list contributes **no referenced name** —
      `frontend::lexical_references` returns the empty set for the attribute
      statement — so `unroll` and `where` stay contextual and
      `let unroll = 1;` is still a program. That matters most for `where`,
      which the census found **eight** times in code, every one inside a
      quantitative index type whose readable alias is spelled
      `quant[| n where n := 5 |]` — the very brackets this attribute borrows.
      The disambiguation therefore takes **two** tokens (a statement head is
      `[|` *followed by* `#`), the alias's continued behaviour is
      **re-measured** rather than asserted, and `#` is still not an identifier
      character, so the scoped-name premise behind `c#0` survives.
    * `NATIVE-LIBRARY-SOURCE-BINDING` **+0**: `extern library NAME;` reuses
      `extern` and `library`, claiming only the two-word sequence at
      declaration position, which was a `Q000` parse failure before. Its census
      found the two whole-token-stream route selectors —
      `module_imports::concurrency_span` and `source_tasks::has_source_tasks` —
      and **excluded binding spans from both**, so a library named `spawn`,
      `join`, `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write` or
      `qcore_error_payload` cannot select a route from its own spelling. **Do
      not remove that exclusion.**
    * `SOCKET-INET-ADDRESSES` **+0, and no lexer rule changed**: a declared
      address (`bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`) is read back
      out of the tokens the existing lexer already produces — a dotted quad is
      `Float Dot Float` because `127.0` is a float spelling, `::` is the
      existing atomic `ColonColon`, and `[|` still wins over `[` by longest
      match, so the `[| |]` and `..` golden surfaces are untouched. An address
      position is told from a name position by the one fact that **every
      address of this route spells a port**, so its token run carries a `:`;
      the run's tokens must be lexically adjacent. Its one name-resolution
      exclusion is **the address reader itself** —
      `source_socket::address_host_span` returns the token range of a
      statement's address *host*, deliberately leaving the **port** outside it
      so a port name stays a genuine reference — and the census measures it
      claiming a span in **no** `.qcore`/`.qc` file of this repository outside
      the card's own two examples.
    * `AGGREGATES-V1` **+0 words and +0 tokens**, and its census is the one that
      **rejected a spelling on archived evidence**. The head is the
      already-reserved `type`, claimed contextually at `type NAME {` — censused
      at **zero** bare occurrences across 435 `.qcore`/`.qc` files (all eleven
      `type NAME {` in the tree are `resource type`, which an earlier branch
      claims) and zero in Rust string literals, so the shape was a parse failure
      before the card, the `extern library NAME;` precedent. **`record` is
      rejected and must stay rejected**:
      `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` is the single
      line `record R { }` and **two archived reports pin it to `Q000`**, so
      claiming the word at declaration position would make that file parse and
      falsify a frozen archive (constraint 21) — the `buffer` → `bytes` and
      `work` → `effort` shape, the syntax renamed rather than the pin edited.
      `struct` is census-clean and was still not taken, on the ground that the
      language already has a declaration head for a named type. The card also ran
      the census in the **other** direction — which *existing* reserved words does
      the construct put in a new position — and found the six whole-token-stream
      route-selector words: a record naming `spawn`, `join` or one of the four
      buffer/payload intrinsics refuses `Q015` declared and `Q000` constructed.
      Both are fail-closed, so `NATIVE-LIBRARY-SOURCE-BINDING`'s span exclusion
      was measured and **deliberately not copied** — it would retire the `Q015`
      half and leave the `Q000` half, buying a type that declares and cannot be
      constructed (`AGGREGATE-SELECTOR-WORDS`).
    * **+0, exact rather than statistical:** `QUANT-REGION-SOURCE`. Every word of
      `error region … catch (…) quant open … as …` and `discharge` is reserved
      already (Cycle 6; `credit`/`effort` from `T08-EPOCH-SRC`) and the shape
      already parsed, so the reservation question does not arise: before the card
      the statement collected `R` as a referenced name and refused at `Q013`,
      measured, so **no program in this repository could contain it at all**. The
      `credit`/`effort` seam with the epoch route is one-way and was probed in
      both directions rather than argued.
    * **+0, censused not assumed:** EXTERN-MODES (`move NAME` spells all four
      departing modes and the declaration decides which ran),
      `SOCKET-EVENT-COMPOSE`, `STRING-CONCAT`, `HTTP-V1-COMPUTED-LENGTH`,
      `CT-WIRE-LANE`; and in wave 27 `CT07-SLICE-PERMISSIONS` (`borrow_utf8` has
      been contextual since `STRING-CONCAT`, `,` and `)` are existing tokens, and
      **no `.qcore`/`.qc` file on disk spells the new three-argument arity**, so
      no shipped program's meaning changes — with **`part` recorded as colliding**
      in binding position in a shipped fixture, and `lend`, `range`, `low`,
      `high`, `sub`, `window` recorded free for a later card) and
      `CT23-CREDIT-ERROR-PATH` (whose census is **vacuous by construction**:
      `error`, `move`, `drop`, `close` and `effort` are all already reserved, so
      the `error(…)` row clause can stop no existing source parsing); and in
      waves 28–29 `REGION-CREDIT-JOINS` and `REGION-JOINS-DUTIES`, vacuous by
      construction on the same ground (every word of the composed form already
      reserved and the shape previously refused, measured not assumed).
    * `SLICE-PERMISSIONS-WRITE-LEND` **+0 words, one contextual clause word**:
      `lend` — `let lend = 5; print dec lend;` is still a program (pinned),
      route selection claims only the `lend(` call shape, and the read card's
      census had already recorded the word free and reserved it for this card
      (census archived with reproduction commands).
    * `WRITE-LEND-TASKS` **+0 words, one contextual clause word reused**: the
      wave-29 `lend` is recognised in two further positions that refused before
      the card — immediately before `<Buffer>` in a task parameter type, and as
      a capture's first word inside a spawn's argument list — so `lend` stays
      contextual and no global reservation is taken.
    * `WRITE-LEND-TASKS-EXERCISE` (wave 36) **+0 words and +0 tokens**, and the
      census is a **pre-card** measurement taken at base `0a5c2ecc` rather than
      a claim checked afterwards. The worker-side byte access takes **no new
      word at all**: `qcore_buffer_write` and `qcore_buffer_read` are the
      existing intrinsics, and the **arity** selects the lease form on the
      `borrow_utf8` precedent (constraint 15h's own mechanism), censused at
      **zero** occurrences of either new arity across **514** `.qcore`/`.qc`
      fixtures, so no shipped program's meaning changes.
    * `MUTEX-SHARE-CROSS` **+0 reserved words, +0 tokens, +0 name-resolution
      exclusions** — the cleanest outcome in this ledger, and the card asked
      for this row three waves before it was written
      (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`). Both clause words of
      `thread scope NAME share PLACE under MUTEX { … }` were **already
      globally reserved** — `share` by the scoped-task route's
      `read PLACE share N`, `under` by card `T03-GUARD-JOIN` — so the card
      claims only a **position**, the fourth token of a child-task header,
      which was a parse failure before it. Censused over **490** `.qcore`/`.qc`
      files outside `third_party/`: 33 total hits, **8** in code rather than
      comments, **0** occurrences of `share` at any position other than the
      read capture's clause, **0** occurrences of `under` in code anywhere, and
      **0** occurrences of `thread scope NAME share|under` in any fixture. No
      exclusion was carved out of `referenced_names`, unlike
      `SOCKET-INET-ADDRESSES` and `WRITE-LEND-TASKS`, because the card claims
      no word — and both clause **names** stay genuine references, measured
      rather than asserted (an unbound one refuses at `Q013`).
    * **Cards that do not engage this constraint at all** — none reserves a
      word or moves the lexer, and the documentation cards touch no crate
      source: `EXAMPLES-SUITE-WIRE`, `DOCS-COMPACT-3`, `GATEWAY-C-LOCATION`;
      waves 28–29's `EXAMPLES-AGGREGATES`, `CT47-COPY-CAPTURE`, `CT22-WIRE`,
      `CT01-PROOF-LEG` and `W29-CORPUS-CONTRACTS`; wave 30's `DOCS-COMPACT-4`,
      `CT24-CLEANUP-PREEMPTED`, `CT-PROOF-LEG-2`, `REGION-JOINS-LOAN`,
      `CT07-WRITE-REMAP` and `W4-RECURSIVE-SUMMARIES`; wave 31's
      `C-GATEWAY-WAVE`, `CT42-PROTOCOL-CELL`, `CT-WIRE-LANE-3`,
      `W31-CORPUS-CONTRACTS-2` and `CT03-PROOF-ALPHABET`; waves 32, 33 and 34's
      `CT-WIRE-LANE-4`, `CT-COMPOSED-NATIVE`, `TRACE-ALPHABET-LEASE-BOUNDARY`,
      `W32-RECONCILE-SWEEP`, `PROOF-LEG-BATCH`, `DOCS-COMPACT-5`,
      `C-GATEWAY-RETURN-POSITION`, `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`,
      the Lean proof-integrity pass, `STABILITY-REEVAL` #1,
      `C-GATEWAY-RETURN-CALL-W4` and `DOCS-COMPACT-6`; and **wave 35's seven**
      — `SWEEP-BLIND-SPOTS`, `WAVE-34-CLOSE`, `VERIFIER-HOST-CONTROL-BUDGET`,
      `CT-CTWIN-LANE`, `C-GATEWAY-CONSTANT-DO-WHILE`, `MUTEX-SHARE-CROSS-ARMS`
      and `JOINED-PROVENANCE-CYCLES`; and **wave 36's six of seven** —
      `CARD-SCOPE-GUARD`, `W36-CORPUS-CONTRACTS-3`, `STABILITY-REEVAL` #2,
      `C-GATEWAY-INDIRECT-CENSUS-2`, `JOINED-PROVENANCE-CALL-FRAMES` and
      `MUTEX-SHARE-CROSS-EMISSION`, none of which reserves a word or moves the
      lexer (the last changes only what an already-admitted program *emits*,
      and the corpus card adds a `.qcore` contract module, which is no
      source-language surface). Wave 36's **seventh**,
      `WRITE-LEND-TASKS-EXERCISE`, is **not** here: it ran a census and belongs
      in the `+0` family above, where its row stands — the
      `MUTEX-SHARE-CROSS` distinction, applied rather than restated.
      **This bullet said "every card of waves
      32, 33 and 34" over a list that named neither `PROOF-LEG-BATCH` nor
      `DOCS-COMPACT-5` nor `MUTEX-SHARE-CROSS`, and the universal was
      therefore false**; card `SWEEP-BLIND-SPOTS` measured that
      (`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`) and offered the integrator
      either arm. The wave-35 close took **both**, because they are not
      interchangeable: the quantifier is narrowed to the cards actually
      enumerated, the two documentation-and-proof cards join this list, and
      `MUTEX-SHARE-CROSS` does **not** — it ran a census (its §9) and belongs
      in the `+0` family above, where its row now stands. Adding it here in the
      card's stead would have replaced a false universal with a false member.
      **Waves 28 through 35 each closed at zero reserved words**, the longest
      such run in this ledger; the source surface has not grown a global
      keyword since `LOOP-V1`.

    `remaining-t03-f`, `remaining-t08-epoch-src`, `extern-call-v1`,
    `wave17-std-modules`, `remaining-extern-modes`,
    `native-library-source-binding`, `quirin-socket-v1`, `loop-v1`,
    `socket-event-compose`, `string-concat`, `socket-inet-addresses`,
    `loop-unroll-budget`, `http-v1-computed-length`, `aggregates-v1`,
    `quant-region-source`, `ct07-slice-permissions`, `ct23-credit-error-path`,
    `region-credit-joins`, `region-joins-duties`,
    `slice-permissions-write-lend`, `write-lend-tasks`.
15. Lock ranks and protection growth are **opt-in** disciplines: unranked
    modules stay byte-identical, and a rank is a declared position, not an
    assumption. `remaining-ct33-ranks`, `remaining-ct34-growth`.
15a. **A source line in a composed task module stands in the parent's body
    or in a spawned worker's, and the two are compared differently.** A
    parent line's position is determined because every *trace* write an
    emitted task program performs is performed by the parent's thread
    (workers buffer events privately; the parent replays them at collect),
    so a parent-only program's streams are compared **byte for byte** and
    every published `examples/threads` program is of that kind. A
    **worker**-body line is written by that worker's own thread, and placing
    one **declares** `qcore.obs-reordering.v1`: that program's streams are
    then compared window by window and emitter by emitter, where a window
    boundary is a point at which no worker is live and an emitter is the
    thread that wrote the bytes — the opposite of F02's causal attribution,
    and right for the opposite reason. The strict comparison is that
    relation's own degenerate case (`strict_iff_byte_equality`), so nothing
    was replaced. Do **not** weaken the two premises the declaration rests
    on: two emitters of one window may not share a line's bytes
    (`AmbiguousAlphabet` refuses the declaration, never a multiset
    comparison), and a worker that writes declares `io` in its **own**
    effect row. `wave21-compose-concurrency`, `wave22-obs-reordering`.
15b. **A route reads its statement tokens from the parsed program's
    post-desugaring stream, never by re-lexing source bytes,** and no reader
    may attribute a diagnostic to a synthesized token's span (synthesized
    tokens are zero-width). The last raw re-lexing site (the sequential
    driver's `statement_tokens`) made `qcore_acquire` through a pipe refuse
    while naming the hand-written form; it now uses the same span filter the
    extern, std-io and task readers have used since their own cards.
    `wave22-pipe-intrinsic-stage`.
15c. **An event program's streams are compared byte for byte, and that is a
    measurement about the reactor profile rather than a convention.**
    `source.event.v1`'s one accepted profile is single-threaded and the reactor
    refuses a cross-thread activation under it (`L04`), so the thread running a
    callback is the entry's own, every record of the derived
    `qcore.obs-reordering.v1` relation (constraint 15a) is a **barrier**, and
    the relation is strict. `observe_event_program`'s live-set walk over this
    model's regions (a dispatch turn) derives the window structure, and the
    emitters a region makes live are read from
    `ReactorProfile::threading_enabled`. A route that runs a callback
    on another thread, or gives a callback a source-level body, **supplies its
    emitters there** rather than declaring strictness or a reordering by hand.
    Do not weaken the two premises the composition rests on: a source line is
    program output and writes no trace record, and a `wake`'s payload is a value
    and never a reactor fact — which together are why every transcript this
    route published before the card is byte-identical after it.
    `wave22-std-event`, `wave22-obs-reordering`, `compose-event-lines`.
15d. **Windows `std.event` selects AF_UNIX by capability, not by compilation.**
    A usable `<afunix.h>` only exposes the candidate; the emitted shim probes
    `socket(AF_UNIX, SOCK_STREAM, 0)` on the running system. Only an explicit
    unsupported-family/type/protocol result from that probe selects the
    retained AF_INET loopback pair; other probe failures refuse. Once the
    probe succeeds, pathname, bind/listen/connect/accept or peer-validation
    failure refuses pair creation and must not be hidden by changing
    transports. Both routes stay native-tested and preserve `SOCKET`,
    `WSAPoll`, the abstract endpoint identities and the byte-identical trace.
    `windows-std-event-af-unix`.
15e. **A rendered string's observation is the view's exact bytes, and the
    emitted write is length-bounded.** `source.string.v1`'s text lines write no
    escaping, no terminator and no newline beyond the framing every line already
    carries, because a payload's `0` is an ordinary byte inside `len` and a
    scan-to-terminator write and the model **disagree** on a value the route
    admits. Do not render a string through any `printf` conversion: `%s` and
    `%.*s` both stop at the first `0`. The write lives in `io_emit`'s second
    prelude fragment, inserted only into a program that renders text — which is
    what keeps every other emitted program byte-identical — and exempted from the
    single-emission-route scan by whole-line identity exactly as the first is;
    its `fwrite` result is **checked**, because nothing in the checker rules a
    short write out. The stream, the `[LEVEL] ` prefix and the single newline are
    `std_io::line_framing` for **both** line kinds, so a rendered scalar line is
    the text line of its own digits (`rfl` in Lean); do not give either kind a
    second framing. A string takes **no** rendering selector and one written in
    front of a string refuses at `Q072`. `string-slice`, `string-slice-io`.
15f. **An HTTP message's framing is constructed, and its `Content-Length` is
    decided.** `source.http.v1` writes every CRLF itself — one after the start
    line, one after each field, one to end the head — and **no admitted component
    may carry a `CR` or an `LF`**. Do not relax that exclusion: it is what makes
    "every CRLF in an admitted head is one the compiler wrote" true, and header
    injection unexpressible rather than merely unintended. The exclusion applies to
    **composed** bytes exactly as to written ones — a composition is framing-free
    *if and only if* every piece is — so injection stays unexpressible however a
    value was built. A declared `Content-Length` must **equal** the content's byte
    count, decided at check time **because every component is check-time-known**: a
    quoted token, or (card `HTTP-V1-COMPUTED-LENGTH`) a value the entry composed
    from check-time-known pieces. Do not turn that equality into a premise, and do
    not admit a content whose length the route cannot compute. A component is
    **resolved to bytes before `admit_field` and `admit_framing` run**, which is
    what keeps both gates unedited and keeps a computed length decided rather than
    assumed; the field **name** stays a written token. The transport half is
    `source.socket.v1`'s **unedited**
    checker over a program this route lowers to, and its refusals are
    re-attributed at `Q095` carrying that checker's own error value — do not
    re-implement a custody, transport-state or determinism rule here. The
    composition's emission goes through `c_program::merge` plus one
    `c_program::assemble`; do not fork a whole-program backend. `std.http` grants
    **no** transport and **no** text: a quoted token at a message component is
    this module's syntax and constructs no `string` value
    (`STRING_SLICE_V1.md` §2), while a `let`-bound one does and takes
    `std.string`'s gate. `http-quirin-v1`.
15g. **A reactor registration is a loan, and the reactor owns no descriptor of
    `source.socket.v1`.** A composed entry's projection carries **no**
    `CreateSocketPair` and **no** `CloseSocket` — those are the operations by
    which `event_reactor` owns a number — and `watch E S …;` is an
    `OwnershipOp::BorrowShared` of `S` to `E` running to the matching
    `unwatch E;`. Do not give the reactor a descriptor of this route: the three
    rules a reader wants (a close while registered refuses, a
    `bind`/`listen`/`connect` on a watched socket refuses, a
    `send`/`recv`/`accept` on one is admitted) are the **unedited** ownership
    checker's drop, write and read under a shared loan, and "deregister before
    close" is their consequence rather than a rule. The loan's span is
    **lexical**, because the ownership IR's borrow nodes are; a crossed pair is
    refused by name and must not be approximated
    (`SOCKET-EVENT-CROSSED-SPANS`). A dispatch turn **drains nothing** — it
    observes level-triggered readiness and the entry's own transport statements
    do the work — and the four bookkeeping statements emit no call and derive no
    effect, because the reactor an emitted program runs *is* the declared
    registration set materialised as the `poll` set at each turn. `Q083`'s
    condition is "an inherited, unedited custody model refused, and the variant
    carries that model's own error value": do not split it per model.
    `socket-event-compose`.
15h. **A `concat` result is static storage the compiler computed, and a
    `borrow_utf8` view is the buffer's own — since wave 27, a view of a
    *range* of it (amended below).** `source.string.v1` composes text
    from **check-time-known** pieces into a new `static const` payload of the
    emitted program: no allocation, no run-time copy, no custody, and a length
    that is the **sum of the pieces' lengths** and is itself a check-time
    constant. Do not make the composition materialise into an owned buffer —
    that was measured and rejected, because it would add an allocation, a copy
    and a release to an operation whose result the compiler already holds, and
    would turn `len` of the result into a run-time quantity, which is the one
    property the operation exists to preserve (`STRING-SLICE-CONCAT-RUNTIME`).
    The `std.buffer` bridge is **borrow only**: an owned string is a new custody
    shape the `{storage, offset, len}` view has nowhere to carry, and composing
    a borrowed view already launders its bytes into static storage, so nothing
    is blocked on the freeze (`STRING-SLICE-FREEZE`). A borrow's UTF-8 validity
    is decided **statically**, out of the buffer's own write history —
    `source.buffer.v1` admits only constant bytes at constant indices — and its
    **window** runs from the borrow to the last statement reading the view or
    **anything derived from it**, a slice of a borrow being itself a borrow of
    the same buffer. Do not widen the window's computation without revisiting
    it: the interval is exact because the route is one flat statement sequence,
    and a loop construct invalidates that premise — which is why the composed
    route **refuses** a `for` today (`STRING-CONCAT-LOOP-WINDOW`). A borrow
    contributes an ordinary `Read` to the cycle-3 ownership operation list; do
    not re-implement the custody rules beside it. **Route selection changes only
    for the bridge construct** — a module holding a buffer without spelling
    `borrow_utf8` keeps `source.buffer.v1` — and a composed module is one that
    **acquires** a buffer, which is what keeps `drop s;` on a string refusing at
    `Q068` where it always did. `string-concat`.
    **Amended, card `CT07-SLICE-PERMISSIONS` (deliberate; it narrows a sentence
    15h already carried rather than taking a new number).**
    `borrow_utf8(NAME, LOW, HIGH)` lends the bytes `[LOW, HIGH)`, and the
    whole-extent spelling is that form at `[0, extent)` — an **equation**, proved
    against the shipped definition and measured on the certificate, the
    observation and the emitted bytes, not an abbreviation. The window is
    unchanged; what changed is what it **excludes**: a write is refused when its
    byte lies inside a live loan and **admitted when it lies outside every one of
    them**, with disjointness decided by `max low < min high`, which is the
    set-theoretic question itself (`overlaps_iff_a_shared_byte`) because every
    bound on this route is a check-time constant. Three asymmetries must survive
    refactors. A **release is refused for any live loan however narrow**, because
    it ends the storage all of them address — do not make that arm range-wise. An
    index the route cannot resolve to a constant is **not provably disjoint** and
    conflicts with every live loan, because the exclusion is decided *before*
    `source.buffer.v1` judges the statement; falling through would make an
    unresolvable index an admission at this gate. And two overlapping **borrows**
    are deliberately **not** refused: both are immutable, they conflict on
    nothing, and a slice of a loan is a fully overlapping second loan of the same
    buffer by construction, so refusing overlap would refuse the mechanism this
    constraint already rests on. There **is now a write lend** — landed by the
    card the old "no write lend" sentence's own residual named
    (`SLICE-PERMISSIONS-WRITE-LEND`, wave 29), superseding that sentence
    **deliberately** — on the **uncomposed buffer route only**:
    `let NAME = lend(BUFFER, LOW, HIGH);` transfers write authority over
    `[LOW, HIGH)` for the loan's lexical window, decided by the lend-extended
    cycle-3 ownership IR rather than by a route rule — a `LendRange` borrow node
    carries the half-open byte interval `STD-BUFFER-SLICE` predicted (that
    recorded blocker was measured to be about the **write** direction, and this
    is it) — and the IR decides that two live lends are admitted exactly when
    their ranges are disjoint, that an owner access is refused exactly where its
    stated footprint meets a live lend and admitted outside every one (reads
    included; an extent query's empty footprint meets nothing), that an access
    through a loan must state a rebased footprint inside the lend (rangeless
    conflicts with everything, `WRITE-LEND-COMPUTED-INDEX`), and that a release
    under any live lend refuses however narrow, by the existing drop rule. **Do
    not move these rules out of the IR into a route, and do not give the release
    arm a range.** On the composed string route every sentence of this
    constraint stands unchanged: a `lend(…)` beside `borrow_utf8`'s read loans
    refuses by name at `Q077` (`SLICE-PERMISSIONS-COMPOSED-LEND`), so the
    window-exactness argument and the `for` refusal at `Q068` are untouched
    there (`STRING-CONCAT-LOOP-WINDOW` re-measured, not assumed). On the buffer
    route the exactness premise is defended by three refusals that must survive
    refactors: a `for` in an entry that lends (`SLICE-PERMISSIONS-LOOP-WINDOW`),
    a crossed pair of lend windows (`SLICE-PERMISSIONS-CROSSED-WINDOWS`), and an
    acquisition inside a window (`WRITE-LEND-NESTED-ACQUIRE`). The lend's bound
    tier is the loop bound's (`WRITE-LEND-BOUND-FROM-BYTE`); a loan is never
    released and never re-lent; and a module with no lend builds the custody
    operation list it always built.
    **Appended, card `WRITE-LEND-TASKS` (wave 31; it supersedes nothing above,
    discharging the sibling residual the wave-29 card named).** The write lend
    now also **crosses the spawn boundary**, as a range write lease:
    `lend PLACE LOW..HIGH` in a spawn capture transfers write authority over
    `[LOW, HIGH)` of an owned Buffer's payload (`BUFFER_PAYLOAD_BYTES = 4`, the
    `int32_t`'s real bytes) to one worker for exactly spawn..join, decided by
    the **lease-extended scoped-task ledger** (`ResourceState::WriteLeased`)
    rather than by a route rule. The disjointness decision is
    `ownership::axes_overlap` — made `pub` and **called** by
    `scoped_tasks::LeaseRange::overlaps`; do not re-implement it in the ledger
    and do not move it out of the IR. Three rules must survive refactors: the
    owner holds **no** whole-payload authority, read included, while any lease
    is out (stricter than both the read-share route and the sequential lend's
    per-byte exclusion, because the owner's only spellable footprint here is
    the whole payload); an **empty** lease refuses (`InvalidLease`) where the
    sequential empty lend admits, the crossing of no authority being vacuous
    and failing closed; and only the creator's join reclaims a lease, in the
    same transaction that collects the capsule. The judgment's mode is
    `LendWrite`, deliberately distinct from the still-refused `write` capture
    (`aWriteCaptureIsRefusedByName` re-decided over the grown matrices), and
    exclusivity is audited as
    `QCore.WriteLendTasks.no_byte_has_two_write_authorities`.
    **Appended, card `WRITE-LEND-TASKS-EXERCISE` (wave 36); it supersedes the
    "no operation admits a lease in a worker body" sentence deliberately,
    discharging the residual of that name by landing the three things that
    sentence demanded together.** A worker body now **exercises** its lease:
    `qcore_buffer_write(LEASE, INDEX, VALUE)` writes one byte and
    `qcore_buffer_read(NAME, INDEX)` reads one, at an index **relative to the
    authority's own base** — a lease's `low`, or 0 for an owner — which is the
    sequential lend's rebasing law (`WRITE-LEND-COMPUTED-INDEX`) crossing the
    spawn boundary. The decision is **in the IR and split across the two places
    its halves are known**, and must stay split: the worker's body states the
    exclusive bound it touches through its lease, because a body is checked
    without knowing where its lease sits; the **creator's spawn site** decides
    whether that bound fits the real `[low, high)`. Do not move the fit into
    the worker (it cannot be decided there) or the footprint into the creator
    (it is a property of the body). The equivalence of that one comparison with
    every per-index containment fact is proved
    (`QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law`),
    which is what licenses checking a summary once instead of every write at
    every creation. Four rules must survive refactors. A byte index is a
    **check-time constant or nothing** — an unresolvable index is not *provably*
    inside its authority and refuses, and the refusal is non-vacuous because a
    nonempty lease always has indices it does not admit
    (`some_index_is_always_outside`). The **owner gains no byte write**: the
    whole-payload write stays its only write footprint
    (`WRITE-LEND-EXERCISE-OWNER-BYTE-WRITE`), so wave 31's "the owner holds no
    whole-payload authority while a lease is out" is inherited unchanged rather
    than re-measured against a second owner-side footprint. A **read share
    admits no byte view** (`WRITE-LEND-EXERCISE-LOAN-BYTE-READ`). And the
    owner's byte **read** takes the *same* ledger gate its whole-payload read
    takes, so a lease outstanding refuses it by the existing rule and not by a
    second one written here. Exclusivity is re-audited about the writes
    themselves — `QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties`
    — and exhibited natively: two threads writing distinct bytes of one
    `int32_t` payload under ASan/UBSan/LSan with stdout byte-identical to the
    reference run. The emitted worker writes
    `((unsigned char *)&buffer->payload)[base + index]`, a distinct memory
    location in the sense of C11 3.14; the lease's **base** travels in the task
    value's otherwise-unused scalar field, which is the one fact the holder
    must not be able to choose. Three boundaries are **measured, not claimed**:
    ThreadSanitizer cannot run any program this route emits, lease or not
    (`TASK-ROUTE-TSAN-SEGV`); a stray byte at `payload + 4` lands in
    `struct qcore_buffer`'s own tail padding, which the allocation owns, so no
    memory tool sees it — which is why the emitted helper carries its **own**
    extent guard rather than delegating to a sanitizer, and that guard must not
    be removed; and a program whose observation depends on the payload's object
    representation is host-dependent (`WRITE-LEND-EXERCISE-BYTE-ORDER`), the
    published flagship being representation-independent by construction rather
    than by luck. No Windows or Apple run is claimed
    (`WRITE-LEND-EXERCISE-WINDOWS-RUN`).
    `ct07-slice-permissions`, `slice-permissions-write-lend`,
    `write-lend-tasks`, `write-lend-tasks-exercise`.
15i. **A declared address is a check-time constant parsed once, and the two
    address families are disjoint.** `source.socket.v1`'s `bind NAME ADDRESS;`,
    `connect NAME ADDRESS;` and `sendto NAME ADDRESS …;` carry a literal IPv4
    dotted quad or a bracketed IPv6 literal and a port that is an integer literal
    or a check-time-constant binding. The address is parsed **once**, by
    `qcore_core::socket_address`, and the emitted program carries its **octets** —
    do not add an `inet_pton` (or any other text conversion) to the emitted C, or
    two parsers can read one spelling differently. **The address form admits no
    external peer**: a target is resolved against the addresses this program's own
    binds declared, and one no bind answers to refuses at `Q085`
    (`SOCKET-EXTERNAL-PEER`); an **ephemeral** port never names a socket, which is
    also what keeps every pre-existing implicit-bind program unreachable by
    address. Custody and the transport check read **one** resolution
    (`std_socket::resolve_addresses`) so a `connect` through an address and a
    `connect` through a name write the *same* listener — do not add a second
    resolution, or constraint 15g's loan could be evaded by spelling a socket
    differently. Two per-arm decisions are **explicit and must stay explicit**,
    because the platforms' defaults differ and an inherited default is a silent
    divergence: every IPv6 socket sets `IPV6_V6ONLY` to **1** on every arm (proved
    constant in `QCore.SocketInetAddresses`), and a declared bind must *succeed*
    when no live socket holds the address and *fail* when one does — which needs
    `SO_REUSEADDR` on the POSIX arms (a `TIME_WAIT` remnant is not a live holder)
    and needs it **absent** on Winsock, where it would admit a second live socket.
    `SO_REUSEPORT` is set nowhere. A host without IPv6 is **probed on the running
    system** (constraint 15d's shape): only an explicit
    unsupported-family/type/protocol result selects the distinct recorded exit
    status, every other failure refuses, and **nothing falls back to IPv4** — a
    fallback would bind a different address from the declared one. The implicit
    `bind NAME;` and `connect NAME NAME;` forms emit the statements they always
    did. `socket-inet-addresses`.
15j. **A flat epoch's credit is conserved across the creator's own error edges,
    and the disposition is declared while the edge is derived.** For every task
    of an admitted plan `restored + spent = credit` and `restored ≤ credit`,
    componentwise on both axes, and over the epoch
    `restored_credits + spent_credits = granted_credits ≤ allocation`. Both
    directions are **refusals**, not conventions: an abandoned grant — neither
    restored nor spent — is always refused (`Q635`, and this is CT23), and a
    restoration larger than its grant is refused per task and per axis (`Q636`),
    because it would put credit into the epoch that the epoch never granted. The
    split is spelled and checked, never inferred both ways: the creator's error
    edges are **derived** from the checked task IR (`TrySpawn.spawn_error`, an
    eliminated `TryOutcome` join), while what the edge does with the grant is
    **declared** and then checked against the grant the epoch actually makes — so
    a plan can neither claim an error path the program does not have nor restore
    credit that was never granted. Three things must stay as they are.
    **`Q637` is deliberately not extended**: because `spent ≤ granted`, the credit
    committed on every error edge is bounded by the success exit's, which `Q637`
    already bounds by the allocation, so a second allocation check there would be
    an unreachable condition rather than a gate (proved as
    `the_error_arm_commits_no_more_than_the_granted_total`). **Restored credit is
    not re-granted** — making the committed total path-sensitive would *admit*
    programs `check_flat_epoch` refuses today, the forbidden direction
    (`CT23-RESTORE-REUSE`). And the **declarative** route has no error edge at
    all, so a clause written there is refused by the owning band
    (`ErrorArmWithoutErrorEdge`, `Q630`) rather than by a grammar that pretends
    the word does not exist; an elided arm is `error(drop);`, which is exactly the
    accounting that existed before, and `qcore_core::epoch_accounting` is
    untouched. `ct23-credit-error-path`.
15k. **A v1 record is a plain copyable value, and the field-custody answer is a
    measurement rather than a simplification.** `source.record.v1`'s
    `type NAME { FIELD: i32, … }`, `let p = NAME { FIELD: OPERAND, … };` and
    `let a = p.FIELD;` are admitted on `source.buffer.v1` only, with every other
    route's refusal measured and pinned rather than widened. Every field is an
    `i32`, so a record contributes **no** `qcore_core::ownership` operation, opens
    no obligation and derives **no effect** — do not give a field custody without
    the four rules that do not exist: the cycle-3 ownership IR addresses an owner
    by a plain **name** and has no path, so `p.buf` is not a question
    `check_ownership` can be asked, and an owned field needs a field-path owner
    vocabulary, a partial-move rule, a drop-through-aggregate rule and a leak rule
    at normal exit that sees into the aggregate (`AGGREGATE-OWNED-FIELD`;
    `AGGREGATE-VIEW-FIELD` for the smaller view-field step, which constraint 15h's
    exact borrow window cannot reach through). The consequences are the admitting
    route's **existing** rules and not new ones — `drop p;` is `Q052`, a leak
    beside a record is still the inherited checker's `Q054` — and construction
    totality is what makes projection total, which is what makes the field read a
    function rather than a partial one. Two emission facts are load-bearing: the
    `qcore_record_`/`f_`/`r_` prefixes, so a QCore name that is a C keyword still
    emits compilable C and a record, a buffer and a scalar of one source name are
    three distinct C objects; and **a program that acquires no buffer carries no
    buffer prelude**, forced by `-Wall -Wextra -Werror` on an unused `static` and
    byte-neutral for every previously admitted program by an argument about which
    programs the emitter could reach (among admitted programs, reading or writing
    a byte *implies* acquiring a buffer) — measured over the whole tree at 80
    emitting programs byte-identical, not asserted. `aggregates-v1`.
15l. **An admitted quantitative region erases from emission, and the sealed pack
    is a linear capability consumed exactly once.** `source.region.v1` —
    `error region NAME { … } catch (ERROR) quant open NAME as BINDING { discharge
    BINDING; }` — lands on the **sequential** route, is not a route of its own and
    selects nothing, which is what gives it the existing certificate, reference
    execution, C emission and native comparison unchanged. Every outcome is a
    check-time literal, so the region has one path and compiles to
    **byte-identical C with the hand-written straight-line program it is**
    (measured against a twin, the whole difference being the `#line` directive
    naming the other file): a region buys checking, not code. Every *semantic*
    refusal is the unedited `qcore_core::error_region` model's own at the code
    that pass already allocates (`Q303`–`Q309`, plus `Q300` for an undeclared
    error type); only the reader's own **shape** refusals land on `Q302`. Two
    things must not be "repaired". `RegionOp::QuantOpen` — the operation
    **variant** — is **unreachable from a valid program by design**: the admitting
    constructor is the `error_region::QuantOpen` **struct** on a catch clause, and
    the variant is retained so an opening outside a catch has an explicit `Q302`
    (`REGION-QUANTOPEN-OP-VARIANT`). And the v1 **path** bound is refused by name
    rather than admitted silently — no error edge, a second catch clause, a second
    region, an uncaught raise, a failing acquisition as the edge, a region in a
    branch arm, a handler statement other than a discharge, a payload binder —
    because `analyze_error_regions` calls `handle_catch` only for an error exit
    that reaches a matching clause, so a clause on a path the region does not take
    is a clause **nothing decides**, and admitting one would be silent rather than
    fail-closed (invariant 1). `quant-region-source`.
15m. **A composed region on the epoch task route opens its pack after every
    join, and the pack's duties are the parent's own owners.**
    `REGION-CREDIT-JOINS` (wave 28) let a `source.epoch.tasks.v1` entry carry
    one `error region … catch (E) quant open … { discharge …; }` whose
    `AbandonCredit` publishes the parent-local unused credit — CT22's scenario.
    `REGION-JOINS-DUTIES` (wave 29) **deliberately superseded** the duty-free
    premise this constraint recorded: the composed body also admits the task
    route's own acquisition and release — `let NAME = qcore_buffer_new(INT)?;`
    sealing a real `Destroy` duty, `drop NAME;` of a region owner erasing one —
    and the discharge destroys the remaining owners in reverse acquisition
    order, CT25's duty half. `REGION-JOINS-LOAN` (wave 30) **deliberately
    discharged `T08-COMPOSE-LOAN`** and landed the loan half: a worker may
    capture a counted read share of a creator owner (`read PLACE share N` at
    the spawn; the place-free `read;` clause in the plan), admitted because the
    lender on this route is always the creator and creator-scope bytes are
    outside the epoch allocation (`REGION-JOINS-EPOCH-AXIS`), so the loan has
    no epoch residency; and the region body also admits the **adoption**
    `move NAME;`, which takes a live entry owner's cleanup duty into the pack —
    an acquisition from the pack's side, erasing completely, its release
    written into the blanks after every join. "After joins and loan expiry" is
    the composition of unedited rules: the share returns at the join under the
    task ledger, the joins precede the region (`Q600`), and a destruction while
    a share is live refuses as the ledger's own `LoanOutstanding` — do not
    re-implement any of the three. Premises that must survive refactors, each a
    refusal today: every `TrySpawn`/`Join` span of the checked task IR
    **precedes** the region (`Q600` in both directions — children live at the
    opening stays `CheckedRegionTaskCreditV1`'s territory, `REGION-JOINS-T07`);
    the region **erases into its straight-line twin** — a duty-free region
    blanks completely (emitted C, source map and reference observation
    byte-identical with and without it, the wave-28 measurement kept), a
    duty-carrying one keeps its acquisitions and body releases at their own
    byte offsets in the task half with one `drop NAME;` per remaining duty
    written into the blanked space after them, first-fit, newlines never
    crossed, refusing when it does not fit (`REGION-JOINS-ERASURE-FIT`), and an
    adoption keeps nothing (the owner's acquisition already stands in the
    entry's own text) — so the composed checkers (the scoped-task route and its
    ledger, `check_flat_epoch` + `EpochCreditRecoveryV1`,
    `analyze_error_regions`) stay **unedited**, each refusing at its own codes.
    **Do not re-implement a custody rule beside the erasure**: two
    simultaneously live owners refuse at the task route's existing custody
    `Q600` — the second acquisition's `?` failure edge discards the first, the
    erased program's real leak path (`REGION-JOINS-MULTI-OWNER`, reachable
    since wave 30 with an adopted owner beside an acquisition) — and an adopted
    name that is not a live owned Buffer in the erased program refuses at the
    task route's own codes (`Q500` for a non-buffer, `Q600` for custody). Both
    published figures are re-derived before publication — `abandoned_credit`
    against the reader's quantity fold, `released` against its duty fold, over
    the model's own cleanup trace. An **acquired** owner's name is the region's
    own (a collision with an entry binding refuses, since the erasure flattens
    the body's block into the task scope); an **adopted** owner's name is the
    entry's own (a name the entry does not bind refuses); the sequential
    `qcore_acquire` spelling names no task-route resource and stays refused;
    and the declarative epoch route gains no error path (constraint 15j's
    shape). Zero new codes: `Q302` condition widenings and the one stated
    `Q600` narrowing, in §L21. `region-credit-joins`, `region-joins-duties`,
    `region-joins-loan`.
15n. **The F02 trace boundary refuses a range write lease, and refusing is the
    measured answer — do not turn it into a drop.** `reference_trace` crosses
    from the reference run's observation alphabet
    (`qcore_core::task_execution::TaskEvent`) into the F02 observable alphabet
    (`ObservedEvent`). Card `CT03-PROOF-ALPHABET` grew the observable side by
    the T05 Buffer write and proved that conservative; card `WRITE-LEND-TASKS`
    added two range-write-lease constructors to the observation side, and they
    stay **outside** the observable alphabet. `TRACE-ALPHABET-LEASE-BOUNDARY`
    (wave 32) discharged the residual that recorded the arm as an unproven,
    unexercised integration-time resolution. Three facts must survive
    refactors. **First, the arm is reachable today** — card
    `WRITE-LEND-TASKS`' own admitted flagship produces a reference run whose
    third event is a `BufferLeased`, so this is a live path and not a
    defensive one; `compiler/tests/trace_alphabet_lease_boundary.rs` drives it,
    and **three of its five tests** were shown non-vacuous by mutating the arm —
    not five, as this constraint claimed until wave 34
    (`LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`; the card's own report published
    the three counts honestly and the ledger's paraphrase inflated them). The
    measurement, re-derived against `crates/qcore-core/src/task_trace_relation.rs`:
    silent-drop 3/5, admitting mis-encode 3/5, wrong index 2/5, union of
    arm-sensitive tests **3**. The other two exercise **neighbouring routes by
    construction** and no arm mutation could falsify them:
    `dropping_the_lease_events_would_have_produced_an_admitted_order` builds the
    dropped encoding itself and calls `reference_trace` on an already-lease-free
    observation, and `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too`
    exercises `parse_observed_trace`. The discharge claim itself is **not**
    weakened by this correction: the arm is genuinely driven.
    **Second, dropping is an *admitting* mistake, not a lossy one**: the
    flagship's lease-free remainder is a trace `AdmittedCausalOrder::declare`
    **accepts**, as a causal order for a run in which two workers held
    outstanding write authority the order never mentions — measured, and proved
    as `QCore.TaskTraceLeaseBoundary.the_dropped_encoding_would_have_been_admitted`.
    A `filter` in place of the `return Err` therefore moves in the direction
    invariant 1 forbids. **Third, the index a refusal carries is the offending
    event's own position and the first such** — proved
    (`refusal_points_at_a_refused_event`, `refusal_is_the_first`), so the Rust
    pin `index: 2` is an instance rather than a coincidence; and on success the
    encoding is positionwise, which is the formal content of "never silently
    dropped" (`encoding_is_positionwise`, `encoding_preserves_length`). The two
    other construction routes into `AdmittedCausalOrder` cannot carry a lease
    at all: `parse_observed_trace` has no tag for the rendered lease lines, and
    `declare` takes `Vec<ObservedEvent>`, which has no lease constructor. **Amended, card
    `WRITE-LEND-TASKS-EXERCISE` (wave 36), which DECIDES the forward question
    `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` rather than leaving it open.** That
    residual asked whether the observable alphabet should grow to carry leases
    once a worker exercised its authority. A worker now does, and **the answer
    is no — the alphabet does not grow**, on a ground the exercise itself
    supplies rather than on the absence of one: `AdmittedCausalOrder` is a
    **sequence**, so admitting a run means stating one causal order over its
    events, and two workers holding disjoint write leases write their bytes
    with no causal edge between them and no synchronization until their joins.
    Any order the alphabet could carry would **assert an ordering the run does
    not have** — which is the situation constraint 15a answers with a
    window-and-emitter relation, and is the opposite of F02's premise. The two
    byte events therefore join the lease events on the **refusing** side, so
    the alphabet did not grow and no existing digest moved. Reachability is
    deliberately *not* the ground, and that is measured: in a real exercised
    program the lease event always precedes the byte write, so growing the
    alphabet by the byte events alone would have decided nothing — both facts
    are driven, the refusal on a synthetic observation carrying no lease event
    so the arm is exercised rather than shadowed, and the shadowing itself on a
    compiled program. The operative consequence is unchanged and is now a
    decision rather than a pending question: write-lend programs sit outside
    F02 relational comparison **by construction**, their native evidence being
    sanitizer, allocation-count and the byte-identical stdout of a strict
    comparison whose windows are the joins. Re-opening this needs CT03's
    method — a conservativity theorem plus a rule shown non-vacuous in both
    directions — **and** a causal-order semantics for concurrent byte writes,
    which is the half the sequence shape denies.
    `trace-alphabet-lease-boundary`, `ct03-proof-alphabet`, `write-lend-tasks`,
    `write-lend-tasks-exercise`.
15o. **(card `MUTEX-SHARE-CROSS`.)** A `protected N` payload crosses a task
    boundary only through `thread scope NAME share PLACE under MUTEX { … }` on
    `source.mutex.guard.v1`, and the decision is in **three** halves that must
    not be merged: classification is `qcore_core::transfer`'s (the fifth row;
    each sharing mode requests **one** protocol and is admitted exactly when
    the kind's discipline is that protocol — do not restore the arm that took
    whatever protocol the kind named); the **absent** and **mismatched**
    protocol gates are the crossing's, because the judgment cannot see a
    generation's declared footprint; and **unverified** is the *unedited*
    `CheckedMutexInvariantV1` checker's own `Q611`. Do not re-implement a
    guard rule beside the judgment. The clause is **opt-in**: a child without
    it lowers byte-identically, and the load-bearing reach rule
    (`Frame::admits_reach`) applies only to a declaring child and only in the
    refusing direction. `MUTEX_SHARING_GAP` stays `false` — the *task* route's Buffer still has
    no mutex. A checked mutex trace is rendered when the backend has a
    rendering for its task structure and published without C when it does
    not. Since card `MUTEX-SHARE-CROSS-EMISSION` (wave 36) there are **two**
    rendered shapes, decided by `mutex_c_emit::classify_trace`: the single-
    task trace, byte for byte what it always emitted, and a **non-nested
    fork/join** trace whose children's events are *contiguous* between their
    `Spawn` and `Join` — which is what the declared crossing lowers to —
    rendered with real `thrd_t` children over the existing `mtx_t` emission,
    the generations and protected allocations moved to file scope so a
    child's own function can name them. Everything else stays outside and is
    published **without** C, never with C the backend did not produce: a
    guard-local join, a nested spawn, a child's events interleaved with its
    parent's, a custody transfer, a registered waiter, and a typed-error
    exit inside a child, whose exit status the model gives no meaning at the
    join. Three things must survive refactors. The **contiguity** premise is
    what makes "create the thread, then block in `thrd_join`" a realisation
    of the *checked* order rather than one interleaving of it — do not widen
    the admitted shape without re-deriving it. The **creator performs no
    trace event while its child runs**, proved as `QCore.MutexShareCrossEmis
    sion.the_creator_performs_no_event_while_its_child_runs`, so this
    backend witnesses a real thread boundary and **nothing about contention,
    a data race or the C memory model**; no such claim may be made from it
    (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`), and no TSan row exists
    anywhere in this repository. And the **operation/log adjacency** is
    load-bearing evidence, not formatting: the native comparison compares
    the *logged* sequence, so deleting a real `mtx_lock` while keeping its
    log line was measured byte-identical and clean under the sanitizers —
    one renderer writes both, adjacently, and `run_mutex_crossing` asserts
    that on the emitted text (`MUTEX-SHARE-CROSS-EMISSION-LOGGED-SEQUENCE`).
    `mutex-share-cross`, `mutex-share-cross-emission`.
    **Landed at the wave-35 close, three waves after the card proposed it**
    (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`, the measured cost of
    `WAVE-32-CLOSE-UNSWEPT`): the words above are the card's §13.1 block quote
    reproduced by card `SWEEP-BLIND-SPOTS` §6.1 and landed unchanged. The
    number `15o` is the integrator's, the card having left it unassigned.
    Wave 35's `MUTEX-SHARE-CROSS-ARMS` supersedes one sentence of that card's
    report from its own record (constraint 21, not by editing it): the four
    arms of these rules that no test drove are now **two driven** by
    `crates/qcore-frontend/tests/mutex_share_cross_arms.rs` and **two proved
    unreachable** and kept as backstops justified by
    `QCore.MutexShareCrossArms` — and the two dead arms are **not** dead for
    the same reason, arm C's admitting protocol being a literal (so its residue
    is empty for *every* kind, independent of the only-pair property) while
    arm D does rest on that property. `mutex-share-cross`,
    `mutex-share-cross-arms`, `sweep-blind-spots`.

**Providers, packages and limits**

16. **P02-FLAKE:** six provider/host rows are load- or parallelism-sensitive
    and are verified in isolation, never weakened: the `RLIMIT_NPROC`
    conformance test (it counts every process of the real UID, and keeps a
    deterministic companion), `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
    (wall-clock, measured failing at load 50 and passing at load 37 on the same
    binary), the confined-fixture write test (`Io(Broken pipe)` under load),
    `real_packages_preserve_unknown_and_reject_mutations` (`ExecutableFileBusy`:
    a sibling test in the same binary executes the file it mutates), and
    `enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
    (added wave 20; the same `Io(Broken pipe)` shape as its sibling row, at
    load ~40). `remaining-p03` §9.1, `remaining-p03-f`,
    `wave17-gen-ctreasons` §4.1, `wave17-corpus`.
    **The sixth, landed at the wave-35 close in card
    `VERIFIER-HOST-CONTROL-BUDGET`'s own words:**

    > `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
    > (`crates/qcore-verifier-host/tests/standalone_host.rs`) is a **sixth**
    > load-sensitive row, and the only one sensitive at the *top* of the range:
    > its first arm's `assert!(start.elapsed() < Duration::from_secs(1))`, a
    > fixed one-second guard over a 40 ms deadline, was measured failing 3 of 6
    > repetitions at 1-minute load ≈100 on a 16-CPU host
    > (`docs/reports/verifier-host-control-budget/README.md` §5). Its *other*
    > historical sensitivity — the large-response admitting control failing when
    > the host was **least** loaded — was a transport defect, not a host
    > condition, and was repaired by card `VERIFIER-HOST-CONTROL-BUDGET`; do not
    > re-file that half here.

    **This replaces, rather than re-issues, the wave-34 paragraph** that
    recorded the row beside the five as not-a-load-flake
    (`HOST-ROW-STARVATION-DESCRIPTION`, card `CT-CTWIN-LANE`'s request, and the
    reason it is replaced is that the mechanism is now *known*). What that
    paragraph got right is kept and what it got wrong is named. Right: the
    large-response failure was real, was not a flake to re-roll, and was **not**
    a weakness in the `VERIFIER-HOST-CLOSED-INPUT-DEADLINE` repair, whose
    assertions were never weakened — `VERIFIER-HOST-CONTROL-BUDGET` changed
    `crates/qcore-verifier-host/src/lib.rs` **only**, leaving the test file
    untouched. Wrong: "deterministically 3/3 in isolation" was three samples of
    a biased coin — 22 pre-repair isolated repetitions measured **7 pass / 15
    fail** — and card `MUTEX-SHARE-CROSS-ARMS` independently saw the row red in
    isolation, red under a full suite **and** green under a full suite on one
    host in one session. The mechanism was never a host throughput limit as
    such: an **8 KiB** pipe capacity against a **1 ms** backoff quantum put a
    ceiling on the 8 MiB-each-way arm, repaired with `poll(2)`, after which the
    control passes **38/38**. Two successors carry what stays open —
    `VERIFIER-HOST-CONTROL-MARGIN-UNPINNED` (nothing pins the 42–112 ms margin
    against the 1,000 ms budget, which is exactly how the defect survived wave
    33 behind a published `0 failed`) and `HOST-PIPE-CAPACITY-UNEXPLAINED` (the
    8 KiB is itself unexplained, so the defect's *size* is a function of an
    unexplained number). The standing instruction is unchanged and now applies
    to the first arm: a gate report **states which it saw**, and nothing is
    discharged by shrinking the response, raising the deadline or deleting a
    control (invariant 1). A seventh row is **measured but deliberately not
    added**: the `RLIMIT_NPROC` row above could not be cleanly verified during
    wave 35 at all, because it counts every process of the *real UID* and the
    wave's five sibling cards were **inside its measurement**, not ambient load
    (`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`). `remaining-p03` §9.1,
    `verifier-host-control-budget` §5 and §7.1, `mutex-share-cross-arms` §10,
    `stability-reeval-1` §2.1 and §7.1, `docs-compact-6` §8,
    `c-gateway-return-call-w4`.
    **A second, independent measurement of the same sixth row, by card
    `W36-CORPUS-CONTRACTS-3` (wave 36).** The card proposed this row's
    text as *adding* the row — "joins this list as its **sixth** row" — and
    that half is **declined**, because the row had already been the sixth
    since the wave-35 close and a list cannot gain the same member twice; the
    card offered the alternative reading itself and asked only that the
    measurement land somewhere either way. It lands here, in the card's own
    numbers: `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
    (`crates/qcore-verifier-host/tests/standalone_host.rs:175`) asserts a
    **one-second wall-clock bound** over an 8 MiB pipe write and a process
    spawn against a 40 ms declared timeout, **failed once inside a full gate at
    load average ~40 on 16 cores** with sibling cards building, and passed
    **12 of 12** re-run in isolation at load 23.7 to 27.6. Verify it in
    isolation; never weaken the bound. This is the row's **first** measurement
    at the middle of the load range — wave 35 measured it at ≈100 and at ≈6.5 —
    and it is post-`poll(2)`-repair, so the repair narrows the failure window
    without closing it (`STANDALONE-HOST-CLOSED-INPUT-LOAD`).
    A **seventh** row is separately named and deliberately not added, because
    card `C-GATEWAY-INDIRECT-CENSUS-2` saw constraint 16's signature and family
    on a test that is **none of the six**: `LOAD-SENSITIVE-ROW-UNNAMED`. Until
    that test is identified and measured in isolation, this list is known
    incomplete rather than assumed complete. `w36-corpus-contracts-3` §8.1,
    `c-gateway-indirect-census-2` §10.1.
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
    not new conformance, and a grown proof numerator is evidence about one
    case's artifact: the sanitized-native and proof counts are the ledger row's
    (**9 of 45 and 3 of 45** at the wave-34 close — read them there, not here;
    this parenthetical is a checkpoint that has been stale before, which is why
    the row and not this line is the authority), and
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
    beside a live corpus (76, unmoved since wave 31) — recorded as
    `D02-B00-LEDGER-SNAPSHOT`, to be repaired by
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

## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # once per checkout
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
python3 scripts/check_lean_audits.py
```

**`--no-fail-fast` is part of the gate as of the wave-26 close (`GATE-NO-FAIL-FAST`,
a deliberate strengthening):** cargo's default stops at the first failing binary,
so one of constraint 16's five load-sensitive rows silently hides every target
after it in crate order — a wave-26 card's first run ended at
`qcore-verifier-host` and never reached the `qcorec` crate at all,
`examples_cli.rs` included, making the number that run printed a lower bound
rather than a measurement. A flake must not be able to shrink the suite it is
reported against.

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `quirin
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan). Provider changes additionally
need `providers/package.py` build + verify (constraint 18).

## Lean CI hygiene

CI builds `proof/lean-toolchain` with Lean and runs every `Audit*.lean` via
`scripts/check_lean_audits.py`, which builds from the pinned proof directory
under finite time limits and cleans up workers on timeout or cancellation.
`AuditAll.lean` checks the compiled QCore axiom closure, allowing only
`propext`, `Classical.choice` and `Quot.sound`; stale audit names and warnings
fail the gate, including library-build warnings under the pinned Lean 4.34.0.
The same audit rejects self-equality and exact assumed conclusions in public,
non-internal theorem statements. This narrow check is not a semantic
non-vacuity proof; do not weaken its generated-name boundary or substitute it
for statement review (`docs/reports/lean-proof-integrity/`).
Both reevaluation examples run in Linux and Clang-enabled
Windows CI. Generic Lean names keep historical aliases
(`QCore.DescriptorCustody`, `EventComposition` / `checkEventComposition`,
`busy_effect_preserves_lock`), so the old Monkey module, composition accessors
and ChunkIO lock theorem remain usable. `docs/reports/w7-lean-ci/README.md`,
`docs/reports/w7-generic-lean-names/README.md`,
`docs/reports/lean-build-performance/README.md`.

## Key entry points

- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI (binary `quirin`, crate `qcorec`): `quirin check <file>`,
  `quirin emit -o out.c <file>`,
  `quirin plugins list|inspect <id>|verify <package>|enable|disable`.
- Configured audits: `quirin-audit CONFIG.json` — the binary of crate
  `qcore-audit`, whose package and library names are unchanged. Inputs are
  prepared by `scripts/prepare_cycle19_clang_audit.py`,
  `scripts/prepare_cmetrics_clang_audit.py` and
  `scripts/prepare_libevent_core_audit.py`.
- Wave close: `scripts/reconciliation_sweep.py` sweeps each merged card's
  integrator-addressed section into individually addressable items and
  `--check`s the close's own `docs/reports/wave<N>-close/reconciliation.json`
  for a verdict on every one; `--residuals` lists residual names the wave's
  reports introduced that no tracker mentions. It decides nothing — see
  `CLAUDE.md`'s close rule and `docs/reports/w32-reconcile-sweep/README.md`.
- Published examples: `examples/README.md` indexes one directory per family —
  `stdio`, `pipes`, `branching`, `extern`, `unsafe-linkage`, `threads`,
  `events`, `strings` (text reaching a stream), `sockets` (checked TCP/UDP over
  loopback, now including declared IPv4/IPv6 addresses), `http` (an executed
  HTTP/1.1 subset, beside the same bytes written by hand), `loops` and
  `records` (the first aggregate type; `EXAMPLES-AGGREGATES`, wave 28). Every
  program compiles through the real pipeline and is byte-compared against its
  reference execution by a workspace test. Since card `EXAMPLES-SUITE-WIRE` all
  **twelve** families are held to one standard by
  `compiler/tests/examples_suite.rs`, which reaches them by **directory scan**
  rather than a hand-kept list — so a new family is covered structurally rather
  than by an author's diligence — while `examples/{strings,sockets,http}` keep
  their own card test files' sanitized native runs unedited beside it
  (`STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE`
  all closed there). The index also records the family's **spelling rule** —
  *pipe the flow, parenthesize the assembly* — under which `examples/events/`
  keeps its own standing pipes-only rule and `examples/{pipes,stdio}` stay as
  the deliberate piped/unpiped twins (`docs-compact-3`).
- Native threading examples: `examples/threads/README.md` (`cl.exe`);
  `build-windows.ps1 -Run` emits, builds and runs the copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
- Editor: `editors/emacs/qcore-mode.el` gives lexical `.qcore`
  highlighting; see `editors/emacs/README.md`. Highlighting does not imply
  executable or verified support.

### Diagnostics namespaces

Authority is `docs/QCORE_SPEC_r0_3.md` §L21 (ownership by semantic pass) —
this table is a summary of it, not a second allocation.

**Q-space note — `Q-NAMESPACE-DECISION`, an open decision in the ice box.** The
`Q000`–`Q099` route-code space is **exhausted**: `Q077`–`Q079` (`STRING-CONCAT`)
and `Q097`–`Q099` (`LOOP-V1`) were the last two pools and are spent, so no free
range is left below `Q100`. The next allocation needs a §L21 namespace decision
rather than a range — candidates: open `Q101`/`Q102`/`Q104`+ around the legacy
three-digit bands, or take a new prefix. **By user direction (2026-09-12) that
decision is iceboxed**, on the stated ground that naming collisions happen
easily; it is recorded as an open decision, **not** a rule, and is
integrator/spec work when taken up. Operative consequence until then: **no card
allocates new `Q` codes.** A new refusal lands as a **condition extension of an
existing band**, on the `SOCKET-EVENT-COMPOSE` /
`NATIVE-LIBRARY-HEADER-DECORATION` precedent — the same condition reached by a
new route — with the widening stated in §L21 and in the card's report rather
than taken silently. Constraint 1 is unchanged: a code is still allocated to a
condition, still stable, still not reusable by a second pass. **Waves 26–36 ran
the consequence fourteen times, on nine bands, without strain** — wave 26:
`LOOP-UNROLL-BUDGET` (`Q097`/`Q098`), `HTTP-V1-COMPUTED-LENGTH`
(`Q090`/`Q093`/`Q094`), `SOCKET-INET-ADDRESSES` (widening `Q084` inside
`Q080`–`Q089`); wave 27: `CT23-CREDIT-ERROR-PATH` (`Q630`/`Q635`/`Q636`/`Q641`),
`AGGREGATES-V1` (`Q051`/`Q052`/`Q053` — that wave's **language head**, an
aggregate type, taking a condition extension rather than a pass of its own),
`QUANT-REGION-SOURCE` (`Q302`, and `Q303`–`Q309` reached from a second route),
`CT07-SLICE-PERMISSIONS` (`Q077`/`Q078`); wave 28: `REGION-CREDIT-JOINS`
(`Q302`); wave 29: `CT47-COPY-CAPTURE` (`Q600`), `REGION-JOINS-DUTIES`
(`Q302`) and `SLICE-PERMISSIONS-WRITE-LEND` (`Q051`–`Q055`, `Q077`); wave 30:
`REGION-JOINS-LOAN` (`Q302` a fourth time, plus the one deliberate `Q600`
**narrowing**); wave 31: `WRITE-LEND-TASKS` (`Q020` and `Q600`, the ninth
band) — each stating its widening in §L21 and in its report and allocating
nothing. Three cards allocated nothing either and are **not** instances of the
rule: `GATEWAY-C-LOCATION` and `CT24-CLEANUP-PREEMPTED`, because fixing where a
diagnostic is *attributed* is not a code change (constraint 1), and
`CT42-PROTOCOL-CELL`, which reaches the footprint checker's existing `Q664`
condition from a new deriver without widening it. Wave 36's
`WRITE-LEND-TASKS-EXERCISE` is the **fourteenth**, on the same two
bands wave 31 used (`Q020` and `Q600`, so no tenth band): the worker-side byte
access lands entirely as condition extensions — the at-most-one-lease parameter
rule and the byte spellings' kind refusals on `Q020`, and the footprint fit,
the constant-index tier and the three authority rules on `Q600` — with the
**arity** selecting the form on the `borrow_utf8` precedent, so it reserves no
word and takes no code. **Waves 32–34 allocated no code and ran no condition
extension either**, so the count before it stood at thirteen: the two gateway
arbiters `C-GATEWAY-RETURN-POSITION` and
`C-GATEWAY-RETURN-CALL-W4` move **gateway diagnostics**, which are categories
rather than `Q` codes, so the icebox does not engage; and the remaining cards
of those waves touch no diagnostic at all. The icebox has now held for eleven
waves without a card needing a range.

| Codes | Owning pass | Meaning |
|---|---|---|
| `Q000` | `NameResolution` (per file) | input file does not parse as a QCore declaration |
| `Q001`, `Q016` | `Index` | index/quant presentation; dimension/axis validity and quant-index type resolution |
| `Q010`–`Q015` | `NameResolution` | duplicates, unknown name, `Q014` unknown imported module, `Q015` concurrency without its file-local import |
| `Q020`, `Q021` | `OrdinaryTypes` | unsupported ordinary construct / task-route source-contract gate; sequential whole-program contract gate. **Route-specific, not to be unified** (constraint 2). Card `WRITE-LEND-TASKS` (wave 31) widens `Q020`'s task-parameter condition and **allocates no code**: the admitted worker parameter types grow by the ungraded `lend<Buffer>` range write lease, receivable only from a `lend PLACE LOW..HIGH` capture, with the place/parameter agreement check covering the new pair exactly as it covers `borrow<Buffer>`; `lend` stays contextual (constraints 14 and 15h). Card `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the same condition again and **allocates no code**: a worker takes **at most one** `lend<Buffer>` parameter, and the byte spellings `qcore_buffer_write(LEASE, INDEX, VALUE)` / `qcore_buffer_read(NAME, INDEX)` refuse here when the named binding is the wrong kind for the position. The **arity** selects the form (the `borrow_utf8` precedent), so +0 words and +0 tokens |
| `Q022`–`Q027` | `ExternCall` | admission and bridging of `source.extern-call.v1` declared extern calls |
| `Q028`–`Q031` | `StandardIo` | `source.std-io.v1` print/log/flush admission behind `std.fmt`/`std.io`/`std.logger` |
| `Q032`–`Q035` | `UncheckedLinkage` | `source.extern-call.unsafe.v1` explicit unchecked linkage; scalars only, every pointer refused |
| `Q036`–`Q038` | `Pipe` | `\|>` pipeline desugaring shapes; `Q038` an operator token used where this revision gives it no meaning (`\|`, `&`, `^` reserved for bitwise) |
| `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067` | `NativeLibrary` | `source.native-library.v1` in four bands: manifest load, binding and agreement plus manifest-resident waivers (`Q039`–`Q042`); manifest-bound contract slices, identity stacking, withdrawal (`Q047`–`Q049`); exported-header reading and manifest/header disagreement (`Q063`–`Q064`); the source file's own `extern library NAME;` binding — resolution refusals, a foreign manifest's symbol, the publication tripwire (`Q065`–`Q067`) |
| `Q043` | `ComposedStdIo` | composed extern/std-io route's own refusals (`source.std-io.v1` × `source.extern-call.v1`) |
| `Q044`–`Q046` | `Branch` | `source.branch.v1` `if`/`else` admission: shape and arm subset, decidable condition tier (connectives, `!`, bitwise, constant bindings), custody join over survivors |
| `Q050` | `ComposedTasks` | `source.std-io-tasks.v1`: the composed task/std-io route's own refusals — a worker-body line now admissible where `qcore.obs-reordering.v1` covers it (constraint 15a) |
| `Q051`–`Q057` | `StandardBuffer` | `source.buffer.v1`: the owned extent-carrying byte buffer (`bytes`, `[i]`, `extent`, `drop`). Card `AGGREGATES-V1` admits the first source-level aggregate on this route — `source.record.v1`'s `type NAME { FIELD: i32, … }`, its construction and its field read — and **allocates no code**, widening three conditions: `Q051`'s to a record declaration, a field declaration, a construction outside the form (including one supplying a field twice, an undeclared field, or not every declared field), a field value that is not one operand and a numeric field path; `Q052`'s to a name at a record or field position that is not one (a construction of an *undeclared type* refuses earlier still, at `Q013`, a type name being an ordinary reference); and `Q053`'s from an index/byte/extent range to those and a **field count** outside `1..=32`. `Q054` (leak beside a record) and `Q055` (a buffer at a field position) are the route's existing rules reached unchanged, and `Q056`'s condition is **not** widened — the recheck comparing two more published halves is that condition working. Card `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) lands the **write lend** `let NAME = lend(BUFFER, LOW, HIGH);` on this route (constraint 15h carries the rules) and again allocates no code, widening five conditions: `Q051` to the lend's window shapes (a `for` beside a lend, crossed windows, an acquisition inside a window, a malformed or shadowing `lend(…)`); `Q052` to a lend of anything but an owned buffer this entry acquired, and a `drop` of a loan; `Q053` to the lend's range beside the existing ranges; `Q054` from the four listed refusals of the unedited checker to **any refusal of the lend-extended cycle-3 checker** — the checker itself was extended, in the semantic model where every other custody rule lives, and the route still inherits rather than re-implements; and `Q055` to a loan at a scalar, index or rendering position. `Q056` is once more not widened — the recheck comparing the published `lends` field for field is that condition working |
| `Q058`–`Q062`, `Q074` | `Event` | `source.event.v1` in two bands: the checked event vocabulary over the reactor/descriptor custody models, `Q061` the determinism boundary (`Q058`–`Q062`); its composition with the source line vocabulary (`print`, `log`, `flush`) and that composition's one value form, `let NAME = dispatch ready E;` (`Q074`, relocated from `Q065` at integration) |
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `source.string.v1` behind `std.string`, three bands. `Q068`–`Q073` (relocated from `Q065`–`Q070` at integration) the immutable UTF-8 `{ptr, len}` view: entry shape, an undecidable slice bound, a quoted token that does not decode, a bound out of range or off a code-point boundary, a string at a position it may not cross, the declared `effects` row. `Q075`–`Q076` (`STRING-SLICE-IO`, which returned `Q077`–`Q079` to the pool) text reaching a stream: `Q075` the line's shape, `Q076` a text operand naming no string this entry bound. `Q077`–`Q079` (`STRING-CONCAT`, reassigned to this pass) **computed text** — `concat(…)`, `dec`/`hex`, `borrow_utf8`: `Q077` shape and bound refusals **including an embedded `source.buffer.v1` statement carrying that route's own message under this pass's code**, `Q078` the borrow-exclusion window with the inherited ownership checker's own refusal, `Q079` the **static UTF-8 decision** on a buffer's write history. Card `CT07-SLICE-PERMISSIONS` makes the borrow lend a **range** (`borrow_utf8(NAME, LOW, HIGH)`, arity one or three) and allocates **no code**: `Q077` also covers a range bound violating `0 <= low <= high <= extent` and the two-argument spelling (deliberately not `Q071`, whose condition is a bound on a *view* decided against `len(s)`); **`Q078`'s condition is widened** from "a store into or a release of a buffer inside a live borrow's window" to "a store **into the bytes** of a live loan, or a release of a buffer any live loan holds", a whole-extent loan reporting the sentence it always reported byte for byte; and `Q079` is reached **without widening at all**, because that condition was always *the bytes this borrow would view* — so a range whose end splits a code point refuses there by the rule already written. A bound that is not a check-time constant stays `Q069`/`Q013` and a buffer or string at a bound position stays `Q072`. `SLICE-PERMISSIONS-WRITE-LEND` (wave 29) widens `Q077` once more: a `lend(…)` in a composed string/buffer entry refuses **by name** until a cross-model overlap rule with `borrow_utf8`'s read loans exists (`SLICE-PERMISSIONS-COMPOSED-LEND`, constraint 15h) |
| `Q080`–`Q089` | `StandardSocket` | `source.socket.v1`: the `std.socket` bundled module's checked TCP and UDP surface over loopback — `Q080` entry shape, `Q081` the operand rule, `Q082` the transport state machine, `Q083` the inherited custody refusal, `Q084` byte/payload/count ranges, `Q085` the **determinism boundary** (a receive the program's own committed history does not satisfy), `Q086` the crossing boundary, `Q087` the `effects` row, `Q088`/`Q089` the certificate recheck and the backend refusal. Band assigned by the integrator before the card, with `Q075`–`Q079` held for a sibling card. Two later cards extend the row and **allocate no code**, landing on `Q080`–`Q082` and `Q085`–`Q087`: `SOCKET-EVENT-COMPOSE`, the **reactor composition** (a checked socket registered with `source.event.v1`'s reactor), and `SOCKET-INET-ADDRESSES`, **declared IPv4/IPv6 addresses with ports** (`bind S 127.0.0.1:8080;`, `connect C [::1]:8080;`, `sendto U ADDRESS […];`), whose external-peer refusal is `Q085`. Each **widens one condition**, stated here and in §L21 (constraint 1): `Q083`'s from "the inherited ownership checker refused" to "an inherited, unedited **custody model** refused", so a reactor `L00`–`L12` diagnostic lands there carrying its own value; and `Q084`'s from a byte/payload-length/receive-count range to those and a **port** range. Per-condition mapping in §L21 |
| `Q090`–`Q096` | `StandardHttp` | `source.http.v1`: the `std.http` bundled module's checked HTTP/1.1 message subset — `Q090` entry and statement shape, `Q091` the start line, `Q092` header fields (including the required single `Host` and the `CR`/`LF` exclusion that makes the constructed framing unforgeable, so header injection is not expressible), `Q093` framing (signature condition: a declared `Content-Length` that is not the content's byte count), `Q094` a name at a message position that is not one, `Q095` the **inherited, unedited** `source.socket.v1` refusal carrying that checker's own error value, `Q096` an undeclared derived effect. Band pre-assigned as `Q090`–`Q099`; `Q097`–`Q099` unused and returned to the pool (`HTTP-QUIRIN-V1`). Card `HTTP-V1-COMPUTED-LENGTH` admits a **computed** header value or content — a quoted token, a name this entry bound to text, a `concat(…)` of check-time-known pieces, or `dec(NAME)`/`hex(NAME)` — allocating **no code** and widening no gate, since every component is resolved to bytes *before* `admit_field` and `admit_framing` run and `Q093`'s equality stays decided rather than trusted (constraint 15f). It widens `Q090`'s condition (a malformed `concat`/`len`, a composition of fewer than two pieces, a composed payload above the 4096-byte bound, a selector without its operand, a value name rebound) and `Q094`'s (a component, piece or `len` operand naming a binding of the other kind). The field **name** stays a written token |
| `Q097`–`Q099` | `Loop` | `source.loop.v1`: the counted `for NAME in 0..BOUND { … }` loop, admitted on `source.buffer.v1` only — `Q097` the loop's **shape** (a header outside that form, a low end not the written-out `0`, a `break`/`continue`/`return` in the body, a body binding shadowing one the entry made), `Q098` the **bound** (not a check-time constant the lowering can compute, zero, negative, above `MAX_ITERATIONS`, or an entry above `MAX_UNROLLED_EVENTS` unrolled), `Q099` one iteration of the body is not the **identity** on custody. A separate pass on the `IF-STATEMENT` precedent: a control shape, not a capability of a bundled module. The band `HTTP-QUIRIN-V1` returned to the pool, now spent (`LOOP-V1`). Card `LOOP-UNROLL-BUDGET` widens two conditions and allocates **no code**: `Q097` also covers the statement-adjacent `[| #unroll, where #bound < N |]` clause list's **shape** and a list that licenses nothing (before a non-`for`, doubled, or dangling; an *unterminated* list stays a `Q000` parse failure), and `Q098` becomes the **three-layer budget** — an absolute ceiling of 4096 no declaration may raise, a default budget of 256 an unattributed `for` is held to (refusing *with the licensing clause list written into the message*), and a licence that is **checked, never trusted**, a declared `N` above the ceiling refusing before this loop's bound is read. The licence **erases from emission**: attributed and unattributed programs of the same admitted bound emit byte-identical C |
| `Q100`, `Q103` | `Usage`, `Ownership` | usage grade; ownership |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | resource flow; cleanup lowering and reference interpretation |
| `Q300`, `Q302`–`Q309` | `TypedErrors`, `ErrorRegion` | `Q300` typed errors; `Q302` error region, with `Q303`–`Q309` the finer conditions the pass has allocated since Cycle 6 and `RegionTaskDiagnostic::code` already reads back — `Q303` a `quant open` naming another region, `Q304` a pack consumed twice, `Q305` a handler returning with its pack open, `Q306` a **nonempty** pack caught without being opened, `Q307` an error leaving a handler undischarged, `Q308` a cleanup action that is not total and noexcept, `Q309` one resource in both the payload and a pack. Card `QUANT-REGION-SOURCE` allocates **no code** and widens two conditions: `Q302`'s from "the error-region model refused" to that **and** `source.region.v1`'s own shape refusals, each v1 path boundary refused **by name** there (`REGION-NORMAL-EXIT`, `-MULTIPLE-CATCH`, `-MULTIPLE`, `-PROPAGATE`, `-FAILING-ACQUIRE`, `-IN-ARM`, `-HANDLER-BODY`, `-CATCH-PAYLOAD-BINDING`); and these codes are now emitted from the **driver** as well as the frontend. `REGION-CREDIT-JOINS` (wave 28), `REGION-JOINS-DUTIES` (wave 29) and `REGION-JOINS-LOAN` (wave 30) widen `Q302`'s composed-route condition a second, third and fourth time — the region on the epoch task route after every join, then its sealed pack carrying real duties, then the **adoption** `move NAME;` beside a worker's counted read share (with an adopted name the entry does not bind, and one owner taken twice through either shape, as that card's own `Q302` conditions) — constraint 15m's admitted set, each stated in §L21, each allocating nothing |
| `Q400`, `Q401`, `Q403` | driver invocation | no inputs; missing/ambiguous entry; `--entry` against a library module |
| `Q500` | `CEmission` | construct the reduced C11 CFG cannot encode |
| `Q600` | `Concurrency` | scoped-task checking and the carried-over structural refusal shared by every checked concurrency profile. Five later cards move this row and **allocate nothing**, each stated in §L21: `REGION-CREDIT-JOINS` (wave 28) widens the composed-route reconciliation condition — a spawn or join whose span follows the entry's `error region` is refused, the ordering derived from the checked task IR and never declared; `CT47-COPY-CAPTURE` (wave 29) widens the boundary condition — the transferability judgment's refusal of a task-creation capture, attributed at the crossing construct and naming the missing transfer/share contract, previously falling through to the `Q500` executable-subset catch-all (kept unedited as the fail-closed backstop); `REGION-JOINS-LOAN` (wave 30) **narrows** one condition deliberately and widens none, discharging the composed epoch route's blanket refusal of read-loan captures (`T08-COMPOSE-LOAN`, constraint 15m); and `WRITE-LEND-TASKS` (wave 31) widens the boundary and scoped-task conditions three ways on the `CT47-COPY-CAPTURE` precedent — the judgment's grown vocabulary (`TransferMode::LendWrite`, the delivered `lend<Buffer>`, `ShareProtocol::DisjointWriteLeases`), the lease's check-time bound decisions (a varying bound, an empty range, a range past `BUFFER_PAYLOAD_BYTES`), and the ledger's own `OverlappingLeases` / `LeaseOutstanding` / `InvalidLease` / `TooManyWriters` arriving through the unedited `check_task_module` (constraint 15h). `WRITE-LEND-TASKS-EXERCISE` (wave 36) widens the scoped-task condition a fourth time and allocates nothing: the worker-side byte access — the creator's **footprint fit** (the bound a worker's body states against the lease that creation supplies), the constant-index tier (an unresolvable index is not provably inside, a negative one and one past `BUFFER_PAYLOAD_BYTES` refuse beside it), and the three authority rules (no owner byte write, no read-share byte view, the owner's byte read on the whole-payload read's own gate) — all decided in the IR, constraint 15h. `MUTEX-SHARE-CROSS` (wave 32) is the fifth, widening `Q600`'s boundary condition and the `Q610`–`Q627` sub-band's route and allocating nothing, on the same precedent: the judgment's vocabulary grows again (`TransferKind::ProtectedResource` and the delivered `MutexShare`, `TransferMode::ShareUnderMutex`, each sharing mode now requesting **one** named protocol), and the crossing's two protocol gates — **absent** (the `under` name is not a mutex generation of this module) and **mismatched** (it guards some other resource) — land at `Q600` at the clause, while the third refusal, `unverified`, is the unedited checker's own `Q611` (constraint 15o). This row's entry is the wave-35 close's, three waves late; the authority it summarises, spec §L21, has carried the widening since the card (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`). `CT24-CLEANUP-PREEMPTED` (wave 30) moves no code and widens no condition: the `ScopeBusy` refusal is rendered from the exit rule's own loan backstop, same code, same span, same refused set |
| `Q610`–`Q627` | `Concurrency` | `CheckedMutexInvariantV1`; `Q624`/`Q625` opt-in lock ranks, `Q626`/`Q627` opt-in protection growth. Card `MUTEX-SHARE-CROSS` (wave 32) reaches `Q611` `UnguardedProtectedAccess` from a **new route** and allocates nothing: the third refusal of the declared task crossing (`thread scope NAME share PLACE under MUTEX { … }`, constraint 15o) — a child reaching the shared payload outside a critical section — is reported here under this **unedited** checker's own code rather than at `Q600`, because a protected footprint has no unguarded access at all and no rule of the mutex checker is re-implemented beside the judgment. Row landed at the wave-35 close (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`) |
| `Q630`–`Q641` | `Concurrency` | `FlatEpochBoundV1` flat-epoch accounting (`source.epoch.bound.v1`), and its sibling `EpochCreditRecoveryV1` — the error path over the same epoch's credits, reached from source by `source.epoch.tasks.v1`'s `error(move\|drop\|close);` row clause (card `CT23-CREDIT-ERROR-PATH`). The same pass and the same quantities, so it **extends four of this band's conditions and takes no code of its own**: `Q630` a plan that does not describe its epoch (including a clause on the declarative route, which has no error edge), `Q635` credit **abandoned** at the creator's error edge, `Q636` a restoration larger than its grant, `Q641` the error path's arithmetic. `Q637` is deliberately **not** extended (constraint 15j) |
| `Q650`–`Q667` | `Concurrency` | `DisjointFootprintsV1` disjoint footprints |
| `Q670`–`Q677` | `Concurrency` | `CheckedRegionTaskCreditV1` error-region task credit |
| `Q700`–`Q703` | `PluginVerification` | scope resolution, undischarged obligations, conflicting evidence, unvalidatable provider allegation |
| `QP001`–`QP016` | provider host | protocol and evidence-firewall diagnostics |
| `<plugin-id>.<local-code>` | providers | provider-owned; exact plugin ID plus one 1–128-byte ASCII local component. Libevent uses `qcore.libevent.L00`–`L12` |

## Where to read more

Beyond the pointers in the ledger above: `docs/QCORE_MODULE_MODES.md` and
`docs/QTT_ALIGNMENT.md` for the language surface, `contract/qcore/README.md`
for contract corpus conventions, `docs/CYCLE19_BUILD_INPUTS.md` for the
persistent audit setup (it replaces ephemeral `/tmp` build paths), and
`docs/design/` for design notes.
