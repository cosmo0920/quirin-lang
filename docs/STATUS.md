# Current status and evidence boundaries

This page states the current state, not a history. `STATUS-COMPACT` rewrote it
on 2026-11-21 (landed in integrate/r51 from main `590f7d9f1`); the whole page it replaced is preserved
verbatim in the [status checkpoint](development/increments/wave-48-compaction-status-part1.md)
(eleven parts, history only). Each card's report owns its evidence, its
increment paragraph is in the [increment records](development/increments/README.md),
and open work is the acceptance matrix's `named_followups`. A paragraph
`scripts/apply_amendments.py` prepends above this one records a landing after
the compaction; the next compaction folds it into the sections below.

## Evidence boundaries

These hold for every figure on this page.

- **Verdicts.** **Safe < Pending < Unknown < Unsafe**, composed by the join
  ([§L21.2](QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)).
  Safe only after every obligation discharges; Unsafe only for a validated
  reachable violation; everything else Unknown. **Pending** is not decided:
  it awaits a named human act (`awaiting-human-review`, `contract-pin-mismatch`,
  `plugin-claim-uncertified`, and `outer-contract` in the runtime projection
  only, rule 31), is never Safe, always stays in the denominator, its held
  reason is only ever tool-minted, and a Pending unit is subject to every Safe
  scope gate (gateway 10i). A row is Pending only if its function carries no
  other reason ([held-call standing](reports/archive/waves-40-47/w45-review-hold-preempts-statement-refusal/README.md));
  every row holding a review-held call names the held declaration's markers
  ([concrete reason](reports/archive/waves-40-47/w47-review-hold-concrete-reason/README.md)).
- **Unsafe is provisional.** Every Unsafe count below is provisional Unsafe: a
  candidate, never a validated upstream defect claim.
- **Unknowns stay in the denominator.** Projection coverage is not safety
  coverage: a projected function is counted, not certified.
- **Raw units remain unpromoted.** A unit status is a function-level roll-up,
  not a promotion. Function-level safety and the chunk **lifecycle** result
  are separate dimensions: no selected chunk function's lifecycle post-state
  is established, and a checked field contract is not a lifecycle result.
  `flb_input_chunk_create` was last recorded function-level Unknown at
  W45-FB-RETRY-LIFETIME, and the production `input_chunk_get` residual remains
  Unknown.
- **Separate properties.** Bounds, null dereference, C integer arithmetic,
  race freedom and reclamation are each a property beside custody, never in
  its denominator; a custody-Safe result is not a bounds certificate.
- **Conditional premises.** The corpus routes run under the eight-home
  cross-unit link map. The claim about actual link resolution, compatible
  types, calling conventions and interposition is an explicit result
  assumption, not a source-derived fact. Source-pinned contracts, named null
  premises and declared assumptions remain assumptions, not unconditional
  C-body proofs.
- **Plugins.** No checker accepts an audit subcommand plugin's or user
  contract plugin's certificate: an uncertified plugin claim is Pending, every
  failure Unknown, and no plugin can reach Safe or Unsafe from the command line
  ([C11.2](QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins)).
- **Evidence classes are independent.** CT mapping, model proof, source
  replay, native observations and runtime projections are reported separately;
  a Lean theorem is not source evidence and a native run is not a model proof.
  Lean gateway theorems are abstract rule theorems, not extraction of the Rust
  analyzer or whole-C correctness; faithful C extraction and Rust–Lean
  correspondence remain premises maintained by review
  ([trust boundary 28](agents/evidence.md#constraint-28)).
- **Mapping is not conformance.** CT01/CT03/CT37/CT50 bind their model proofs
  through the F02 trace/digest relation; CT07 binds through the scoped-task
  lease ledger, not F02, whose lease refusal remains intentional. CT50's
  binding grew no alphabet, and its closure over admitted renderings is the
  degenerate case — a single-actor run, proved to admit nothing but itself
  rather than assumed to. Every proof row is evidence about one case's
  artifact; **CT50's covers the capability half of its scenario only**,
  because the numeric address reuse that case is named after is invisible to
  an alphabet carrying generations and stays native observation
  (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`). No F02-shaped candidate remains among
  the mapped cases (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`, measured). Each leg's
  pinned digest is compared with the value its Lean side decides,
  total-or-reject both ways. Native execution is observed, not proved. The
  sanitizer count carries its host condition; emission assertions can run on
  other hosts without implying sanitizer execution there.
- **Frozen evidence.** Integration archives remain frozen; each subsequent
  arbiter must reproduce its predecessor's verdict-bearing artifacts and
  attribute every delta ([gateway rule 7](agents/gateway.md#constraint-7)).

## Headline measurements

### Published live-profile baselines

Primary source: each live profile's `contract/evaluations/<name>/baseline.json`,
published by `scripts/land.py publish`. Function rows: Safe / Pending /
Unknown / provisional Unsafe.

| Profile | Function rows | Units | Null rows | Measured | Source |
|---|---|---|---|---|---|
| `eight-home` (pinned corpus, Fluent Bit `66910c10`) | 4,834: **1,079 / 0 / 3,745 / 10** | 185: 9 Safe / 164 Unknown / 8 provisional Unsafe / 4 data-only | 4,921: 2,549 / 2,355 / 17 provisional Unsafe | `556460e31` | [landing](reports/landing-556460e31/README.md) |
| `fluent-bit-system` (`046accf6`, every configuration) | 448: **103 / 0 / 341 / 4** | 15: 13 Unknown / 2 provisional Unsafe | 454: 211 / 243 / 0 | `556460e31` | [landing](reports/landing-556460e31/README.md) |
| `fluent-bit-tail-pause` (`34155955`, the `90978ac6` export) | 404: **58 / 4 / 341 / 1** | 16: 15 Unknown / 1 provisional Unsafe | 406: 190 / 216 / 0 | `777fba16e` | [landing](reports/landing-777fba16e/README.md) |
| `twelve-home` (eight-home plus four comparator homes) | 4,704: **1,083 / 0 / 3,615 / 6** | 179: 9 Safe / 160 Unknown / 6 provisional Unsafe / 4 data-only | 4,796: 2,476 / 2,303 / 17 | `974434463` | [first baseline](reports/cg22-callback-candidate-link-premise/README.md) |

`twelve-home`'s first baseline is the card's worktree-private preparation of the
pinned inputs, not the `build/cycle19` preparation, so its counts are not
comparable with `eight-home`'s published row; a landing measurement on the
recorded inputs re-records it. Six frozen profiles (Cycles 17–19, ChunkIO,
CMetrics) keep their last measured revision. Per-rule contributions beside each
live profile's real-run total are in [contributions](development/contributions.md).

### Unpublished landings

<!-- unpublished-landings: generated by `python3 scripts/land.py unpublished --write` from the landing gate records and the publication records; never edit by hand. -->
Unpublished landings (user decision 2026-11-16): the newest committed landing baseline record is `556460e31` (eight-home and fluent-bit-system, [record](reports/landing-556460e31/README.md)); no later landing has one, so 77 landings completed steps 1–7 of `scripts/land.py` (records, full landing gate, merge) but not step 8: integrate/r8 (W45-FB-ENGINE-PROGRESS-MEASURE, MEASURE-LOOKUP-RECORD-REUSE and LICENCE-HEADER-TEST-REPAIR) to r81 (FB-CHUNK-LIFECYCLE), integrate/ag-gaps (AG1-INDIRECT-MEMBER-ACCESS, AG2-UNRESOLVED-ALIAS and AG3-CONDITION-RETURN-CALLS), integrate/cg25-thread-key (CG25-VOID-CAST-GUARD-ESCAPE and CG25-THREAD-KEY-AUTOMATIC-REVISIT), integrate/r38-connect (QUIRIN-AUDIT-CONNECT) and integrate/r40-linkage (CONNECT-ADOPT-DECLARED-DEPENDENCIES); r18 and r40 have no unpublished landing gate record. `python3 scripts/land.py unpublished` lists each with its cards, read from the landing gate records (`docs/reports/*/integration/gate.json`). The landing host holds no prepared live-route inputs (`--inputs-base` absent, no `build/cycle19`) and no `private/baselines/`, and a fresh preparation here would not reproduce the committed baselines, so `land.py publish` is deferred rather than forced. Their figures are landing-gate counts, not published baselines; publish from the host that holds the prepared inputs (`land.py publish --previous 556460e31`).
<!-- unpublished-landings:end -->

### Measured since the baselines

**Newest integrator measurement of the live routes:** the SP1 landing's
[`quirin-audit measure --declared`](reports/archive/contracts-and-premises/sp1-symbolic-openssl-premises/integration/native-comparison.json.gz)
of `2c07390b6` (main r39 plus SP1) against `b7f86a0de`, every route complete,
0 moves. Function rows, Safe / Pending / Unknown / provisional Unsafe; a
landing-gate count, not a published baseline:

| Route | Rows at `2c07390b6` | Against the published baseline |
|---|---|---|
| corpus (`eight-home`) | 4,834: **1,119 / 0 / 3,703 / 12** | +40 Safe, +2 provisional Unsafe |
| `twelve-home` (same preparation) | 4,834: **1,119 / 0 / 3,703 / 12** | first baseline on another preparation |
| `fluent-bit-system` | 448: **112 / 0 / 332 / 4** | +9 Safe |
| `fluent-bit-tail-pause` | 404: **63 / 4 / 336 / 1** | +5 Safe |

Most moves since `556460e31` are recorded in the landings' own
`integration/native-comparison.json.gz` records (THREAD-LOCAL-CUSTODY-FREE-READ:
system and slice +4 Safe each; CURRENT-CONTEXT-NEW-TRIAGE D3: corpus +1; AG1,
AG2, AG3 and their combined run: corpus 1,079 → 1,119 Safe and 10 → 12
provisional Unsafe, system +5 Safe), read by
[`landings.py`](reports/status-compact/scripts/landings.py). The corpus also
stood at 1,082 Safe at D3's tip, 1,078 at the CG25 bases and 1,079 at the AG
bases, changes no committed comparison records
(`LIVE-ROUTE-MOVES-BETWEEN-MEASURED-LANDINGS`). The landings after
it (r41–r53) did not measure the pinned routes: their gate records say
pinned-route parity was not run, the host holding no `build/cycle19` inputs.
Their cards predicted corpus moves (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3,
CG10-MEMBER-STORE-RETAINED-BY, CG10-READER-GAPS) that are not yet measured
there.

**Opt-in `current --monkey-linked` route** (Fluent Bit `046accf6`; Safe /
Pending / Unknown / provisional Unsafe), each card on its own base, not
composed: CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3 1,139 / 2 / 3,656 / 7 →
1,140 / 2 / 3,655 / 7; CG10-MEMBER-STORE-RETAINED-BY 1,140 → 1,150 Safe;
CG10-READER-GAPS 1,150 → 1,152 Safe; round 4 1,150 → 1,157 Safe, measured on
base `9b107caf1` without r45–r47, its seven moves resting on neutral
declarations not yet validated against source
(`CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED`); CG10-SMALL-BUNDLE no status move;
CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT 1,159 → 1,169 Safe, measured on base
`8e7e8d509` without r49, its ten moves resting on five pinned declarations not
yet validated against source (`CG10-OOM-DECLARATIONS-UNVALIDATED`). No combined
figure is measured.

### Conformance, proof and corpus

| Measure | Value | Source |
|---|---|---|
| Full language conformance | **false** | [Acceptance](reviews/r0_3/acceptance.json) |
| CT cases mapped | **50/50**; none unmapped | [Manifest](reevaluation/ct_manifest.json) |
| CT independent artifacts | **48** distinct source-fixture programs behind the 50 mapped cases; **9** reviewed byte-identical fixture groups | Same manifest, `artifact_identity`; [disclosure](reports/ct-shared-fixture-disclosure/README.md) |
| CT case-bound proof | **5/50**: CT01, CT03, CT07, CT37, CT50 | Same manifest, per-case `proof` stage |
| CT gated sanitized-native | **10/50, Linux/macOS only** | Same manifest, `native_sanitized` stage and host gates |
| Lean | toolchain **`leanprover/lean4:v4.35.0`**; **533** audits | `proof/lean-toolchain`; `proof/Audit*.lean`, r81 landing gate |
| Contract corpus | **154** modules (1,002 declarations, 8,801 clauses) in the eleven inventory locks `qcore_contract_corpus.rs` pins: five `qcore-provider-*` crates 95, six `quirin-provider-*` crates 59; user crates outside the locks: OpenSSL 21, libcurl 4, plugin 1 | `providers/*/contract-inventory.lock`, [load path](../providers/contract-load-path.json), [ownership](../providers/README.md#contract-ownership) |
| Registered follow-ups | **1,054** | [Matrix](reviews/r0_3/acceptance_matrix.json) `named_followups` |

Re-derived by [`rederive.py`](reports/status-compact/scripts/rederive.py); the
[report](reports/status-compact/README.md) records its output.

## Coverage by area

Each row names the card that last measured it and where; a row whose figure
predates the published baselines is dated, not current. Earlier figures and
per-card chains are in the [checkpoint](development/increments/wave-48-compaction-status-part1.md)
and the reports.

**Custody and properties on the corpus**

| Area | Current state and boundary | Source |
|---|---|---|
| Runtime projection of outer contracts (rule 31, C04.20) | A separate runtime scope beside unchanged static verdicts. At stage 2 all three loaders were in force; runtime all-loaded custody **909 / 32 / 3,884 / 9**, the exact 32 baseline Unknown rows Pending until human review of both loaded plugin classes (measured at stage 2, before the published baseline) | [Stage 1](reports/archive/contracts-and-premises/outer-contract/README.md), [stage 2](reports/archive/language-and-tooling/rk-plugin-library-outer-envelope/README.md) |
| Bounds (separate from custody) | **162 Safe / 0 Pending / 0 Unsafe / 4,759 Unknown** over 4,921 definitions; combined custody + bounds 113 / 4,799 / 9 at SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN (main `e0b92ece6`). User-authored external premises require review; every covered use is checked | [CG16](reports/archive/gateway-and-clang/cg16-index-and-offset-extents/README.md), [SUP03](reports/archive/language-and-tooling/sup03-bounds-tally/README.md) |
| Null dereference (separate; rules 14k–14z, 15q–15u, C09.3) | The live baselines above (corpus null 2,549 / 2,355 / 17 provisional Unsafe). Findings are exact paths validated from entry under a named condition, never upstream defects | [Null triage](reports/archive/contracts-and-premises/null-triage-unresolved-paths/README.md) |
| Contract dependencies per verdict (rules 13f–13k) | Every row lists the declarations and premises it rests on; the dependency field changes no verdict | [Report](reports/archive/language-and-tooling/verdict-contract-dependencies/README.md) |
| Callback contracts (rules 13u–13z, C04.19) | A callback-taking callee is decided only through its callbacks' own contracts, never as benign; registries only under a registration in force in the calling body | [Report](reports/archive/gateway-and-clang/cg22-callback-registry-contract/README.md) |
| Leak classification | Of W44-LEAK-PIN-AUDIT's 271 leaves: 139 validated, 31 projection artifacts, 101 premise or approximation gaps; provisional candidates, never upstream defect claims (classifies that card's leaves, not today's) | [Classification](reports/archive/waves-40-47/w44-leak-pin-audit/classification.md) |
| Refusal censuses (measurement, no verdict) | Uncontracted calls, branch conditions, indirect access, return expressions and unknown aliases are each censused by the card that last moved them; the indirect-access sole-blocker population is ranked by CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (round 4: 222 Unknown functions blocked by it alone on the opt-in route) | [Round 4](reports/archive/gateway-and-clang/cg10-indirect-access-sole-residuals-r4/README.md), [W47 blockers](reports/archive/waves-40-47/w47-access-path-positions/artifacts/after/blockers.json), [alias census](reports/archive/waves-40-47/w47-alias-residuals-census/README.md) |
| Raw source coverage | **4,921 definitions / 4,834 projected / 87 pinned-build exclusions**, 24 full-path units | [Census](reports/w42-unprojected-definitions/README.md) |
| Contract plugins | `quirin-audit openssl`, `quirin-audit libcurl` and the OpenSSL and libcurl user crates publish Pending at best, never Safe. The pinned corpus route refuses a manifest whose load path carries checker bindings (`checker-bindings-not-run`) | [OpenSSL](reports/archive/wave-48/w48-openssl-contract-plugin/README.md), [libcurl](reports/archive/wave-48/w48-libcurl-contract-plugin/README.md), [bindings](reports/archive/language-and-tooling/configured-route-plugin-bindings/README.md) |

**Fluent Bit, Monkey and Cycle 18 evidence**

| Area | Current state and boundary | Source |
|---|---|---|
| Loop-owner obligation (rule 10p, C04.9) | 4 declared collector operations, **refused on every per-unit route** (verdicts unchanged); discharge is measured only by the in_tail thread-entry census | [W47-LOOP-OWNER-INSTANCE-FLOW](reports/archive/waves-40-47/w47-loop-owner-instance-flow/README.md) |
| Condvar publication and join (rule 10q, C04.10) | 6 declared libc callees, **refused on every per-unit route** (verdicts unchanged); discharge is measured only by the condvar/join census | [W47-CONDVAR-JOIN-PUBLICATION](reports/archive/waves-40-47/w47-condvar-join-publication/README.md) |
| Engine loop census (measurement, no verdict) | 121 loops at `046accf6`: 115 bounded / 2 decreasing / 1 yielding / 3 unresolved under named premises; no progress verdict (Quirin proves resource safety, not progress) | [W45-FB-ENGINE-PROGRESS](reports/archive/waves-40-47/w45-fb-engine-progress/README.md) |
| Loop measures, list-length bounds and ledger transitions | Facts beside custody, never verdicts: a checked counted-loop measure or a named refusal (rule 44); 17 of the system slice's 87 list-length loops carry a bound under a declared list-finiteness premise (rule 48); a byte-ledger write is certified only as a guarded saturating transition paired with its list operation, otherwise refused (rule 92), which no route reports yet and whose pinned Fluent Bit bodies route T still refuses | [Gateway 44](agents/gateway.md#constraint-44), [48](agents/gateway.md#constraint-48), [92](agents/gateway.md#constraint-92) |
| Engine-thread joins (native observation, no verdict) | 240 runs: a second `flb_stop`, and a stop after an engine start failure, join the engine thread twice; under the strict pair these reproduce caller-contract violations, not library defects. **Native observation at the pinned builds under the stated inducers, not a proof, no Quirin verdict moves, not an upstream defect claim** | [W47-FB-DOUBLE-STOP-OBSERVATION](reports/archive/waves-40-47/w47-fb-double-stop-observation/README.md) |
| Native descriptor observation | **81 pinned Linux runs** (80 accepted, 36 with injected failures); release-at-most-once fails in 13 runs (two double closes read against the Fluent Bit source: local observations, provisional candidates, not upstream defect claims) and operand validity in 37 (`close(-1)`); the landed libc descriptor, stream and directory contracts bound to **3,149** executions of 18 declared callees with **0 contradictions** | [W43 descriptor native check](reports/archive/waves-40-47/w43-descriptor-native-check/README.md) |
| Chunk lifecycle results | **0 Safe / 96 Unknown** over 96 selected `flb_input_chunk.c` and `cio_chunk.c` functions; nine outstanding composition legs each | [Chunk source profile](reports/w43-chunk-source-profile/README.md) |
| Monkey callback accounting | **3/3 accounted, 0/3 Cycle 18 composed, 3 explicit Unknown**; 8 source paths; 51 relevant obligation cells, 17 discharged / 34 open | [Callback report](reports/w42-monkey-callback-coverage/README.md), [composition](reports/archive/waves-40-47/w43-monkey-callback-composition/README.md) |
| Observed Monkey timed wait | 1 instrumented Linux epoll success run; conditional replay, no coverage promotion | [Observed integration](reports/cycle18-observed-integration/README.md) |
| Cycle 18 | Priority-one: 8 failure cases (3 fault-model Unsafe / 5 Unknown), 4/4 payload paths under explicit authority, 15-operation timeout teardown. Bounded aggregate: 3 joined families / 8 source paths / 7 unresolved obligations, **aggregate Unknown**. Callback task handoff: 12 source-derived operations, production acceptance pending. Native platform matrix: **4 Linux Observed / 3 foreign-host Unknown** | [Priority one](reports/cycle18-priority1-integration/README.md), [broader integration](reports/cycle18-broader-integration/README.md) |

**Language and gateway capabilities.** The implemented `.qr` routes (fixed-width
scalars, arithmetic errors, records, task values and captures, closures,
floating point, imported C structs, `.qrlib` C linkage) are specified in
[implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md); the C gateway's
reading rules (elaborator, typed places, casts, unions and bit-fields, atomics,
weak memory, aggregates, loops and budgets, return-position correctness) are
the numbered rules in [gateway](agents/gateway.md). Each rule's report owns its
measurement; the former per-rule rows are in checkpoint parts 7–11.

## Milestones and open trust boundaries

Cycles 1–17 are complete only at their bounded milestones. Cycle 18 has bounded
implementation with aggregate acceptance pending. Cycle 19 is in progress;
per-path identity imports and observation evidence gate promotion. Cycle 20 is
active: all v1 Safe proposals are refused, including empty scopes; unchecked
Unsafe proposals downgrade to Unknown (`QP010`).

- The three broader Cycle 18 follow-ups remain registered
  (`CYCLE18-CALLBACK-TASK-HANDOFF`, `CYCLE18-NATIVE-PLATFORM-MATRIX`,
  `CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`). Production callback
  composition, task/cancellation/event-generation coverage and current
  foreign-host observations remain outstanding.
- Event-loop source audits (EVENT-CALLER-REMOVAL, EVENT-ORDERED-LIFETIME,
  EVENT-HOLDER-IDENTITY, EVENT-COLLECTOR-SELECTION) are independently replayed
  imports; membership/instance invariance, persistent frames and caller
  lifetime composition remain open, and no Safe route consumes them
  ([collector selection](reports/archive/language-and-tooling/event-collector-selection/README.md)).
- Guarded contract adoption is **halted at the user's request**
  ([supplemental card](reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md)).
- Memory: the bounded audit working set's one observed default current run
  peaked at 6,985,576,448 bytes; the 6 GB target, CI margin and repeats remain
  open, and the prior four-job 7 GB OOM remains a failed result
  ([working set](reports/archive/language-and-tooling/audit-ir-working-set/README.md)).
- Resource custody is kind-indexed: a release discharges only an obligation
  of its own kind, and a file descriptor is tracked as an `int` with sentinel
  validity ([W43-DESCRIPTOR-RESOURCE-TRACKING](reports/w43-descriptor-resource-tracking/README.md)).
  Every function declared in more than one module is a
  [checked view relation](reports/contract-view-projection-check/README.md).
- Host/load-sensitive tests and their reporting requirements are in
  [rule 16](agents/providers.md#constraint-16); the per-execution quota join is
  fail-closed ([rule 26](agents/providers.md#constraint-26)). The summary-count
  and Clang recording mechanisms keep their disclosed limitations
  ([validation policy](agents/validation.md)).
- No check ties this page's figures to a gate file (`STATUS-LATEST-GATE-STALE`).

## Latest validated gate

The [integrate/r81 landing gate](reports/fb-chunk-lifecycle/integration/gate.json)
(FB-CHUNK-LIFECYCLE) ran the full landing gate on `2e7eef1ea` through
`scripts/gate_scope.py --base origin/main --full --landing`.
Root Rust: **10,379 passed / 0 failed / 24 ignored**, 725 units / 770 summaries;
**533 Lean audits**; fmt, Clippy and both reevaluation examples exit 0.
Python: **814 tests**, which exited 1 on `test_build_matrix_sections`'
stale-matrix check only, and were rerun after the matrix was regenerated from
the record. No provider path changed; the provider figure, **182 passed / 1 ignored**, is carried from the
integrate/r74 [landing gate](reports/landing-batch-a/integration/gate.json), which ran it.
Clang: Ubuntu clang 21.1.8. Pinned-route parity was not run (no `build/cycle19`
inputs on the host), and no profile baseline changed.

## Where to go next

[Planning and registered work](README.md#planning) ·
[Rule router](agents/README.md) · [Platform evidence](development/platform-evidence.md).
The pre-restructure ledger narrative is preserved as a
[historical checkpoint](development/increments/structure-checkpoints/status.md),
and the page `STATUS-COMPACT` replaced as the
[wave-48 status checkpoint](development/increments/wave-48-compaction-status-part1.md);
neither is a second current status authority.
