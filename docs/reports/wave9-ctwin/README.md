# CT-CTWIN — the blocked-on-harness-schema C-twin bucket

Date: 2026-09-09. Baseline: `05c51a5` (main, wave-8 consolidation).
Branch: `worktree-agent-ad790c24ed5204419` (isolated worktree). Wave 9,
lane CT integration, kind harness/manifest, scope M.

**What landed:** the schema-2 CT harness gains the `legacy_c` route that card
T00-CT-SCHEMA declared with no member, and the five cases of the
"blocked-on-harness-schema (C-twin stage)" bucket are censused by measurement
against the real unrestricted-C gateway. **Two map (CT43, CT45); three stay
unmapped (CT40, CT41, CT42) with measured reasons that replace their stale
"no legacy-C stage exists" blockers.** CT coverage moves **28/50 → 30/50
mapped (unmapped 22 → 20)**, by executed-stage evidence only.

No production checker, gateway, frontend or backend file changed: `git diff
05c51a5..HEAD --stat` over `crates/` and `compiler/src/` is empty. Sanitized-
native stays **3 of 30** and proof **0 of 30**, so the grown denominator is
mapping, not conformance, and nothing is promoted.

## 1. The five cases, and what each one actually requires

The bucket is CT40, CT41, CT42, CT43, CT45 (from
`docs/reports/remaining-t00/coverage.json`'s `unmapped_triage`, cross-checked
against the manifest's unmapped entries). Each was measured through the public
gateway entry points — `validate_unrestricted_c11` then
`analyze_unrestricted_c11` — over a synthetic `q_*` custody contract, before
any manifest or harness edit.

The rule this card applied, stated before the results so it is not a
post-hoc fit:

> **Attribution rule for a C-twin mapping.** A case maps only when the
> gateway's verdict on its twin is (a) the verdict class the scenario demands
> and (b) *attributable to the construct the scenario names* — shown by a
> control that removes exactly that construct and moves the verdict. A refusal
> that survives removing the named construct says only "the projection cannot
> express this at all", which is the vacuous mapping the harness convention
> already forbids for the other routes' controls.

| Case | Scenario / expected | What a C-twin stage must decide | Measured | Disposition |
|---|---|---|---|---|
| CT40 | Atomic pointer points to prematurely freed memory / "Not certified; unsupported lifetime/publication protocol" | that a program publishing an owned pointer through an atomic object and then freeing it is not certified | publish-then-free is **Safe, no diagnostic**; publish-without-free is a Leak (so the store does not discharge custody); the verdict is **identical with `_Atomic` and without it**; a unit that also *reads* the published object is Unknown, on the reader's `unknown-alias` diagnostics | **unmapped** — fails (b), and at function level fails (a) |
| CT41 | `volatile` used as a completion flag / "No synchronization proof" | that no synchronization is proved from a volatile flag | adding or removing the flag changes **neither the verdict nor one diagnostic**, in both shapes measured (global-pointer release: Unknown `[subject:3:unknown-alias:q_shared]` either way; local acquisition: Safe either way) | **unmapped** — fails (b); the gateway has no synchronization model, so no verdict of its is evidence about one |
| CT42 | Two descriptors share an underlying logical state / "Preserve shared protocol-cell identity" | a *preserved identity*, a positive modelling property | an int-typed resource identity is not even expressible — the contract loader refuses `extern fn q_open_fd() -> i32` with `owned result requires a pointer return type`; the descriptor twin is Unknown on three uncontracted calls. (The pointer-alias analogue `void *b = a;` + two releases is Unsafe `[subject:2:double-release:2]`, so identity survives a *syntactic* alias — a different question) | **unmapped** — fails (a); no measured verdict establishes a preserved identity |
| CT43 | Unresolved callback may run after API return / "Unknown; no whole-program proof" | Unknown, attributable to the deferral | Unknown, one diagnostic: `subject:6:unsupported:call to uncontracted function \`q_register\``; the same callback **called directly** is Safe | **mapped** |
| CT45 | Unknown destructor accesses shared global state / "Require access contract or report unknown" | Unknown without an access contract; checked with one | Unknown, one diagnostic: `subject:5:unsupported:call to uncontracted function \`q_destroy\``; the **same `case.c` bytes** with an access contract declared for `q_destroy` are Safe | **mapped** |

### 1.1 Two measured findings recorded, not repaired

* **CT-CTWIN-PUBLISH.** A store of an owned pointer into a file-scope object is
  invisible to the resource projection. It is fail-closed in the leak
  direction (publish-without-release is reported as a Leak, so the store is not
  read as discharging custody) and silent in the other: publish-then-release is
  `Safe` with no diagnostic, and the dangling file-scope reference has no
  representation. Within the documented scope — one translation unit's
  acquire/release accounting — this is not a wrong answer, and a unit
  containing any reader of the published object is Unknown. It is nonetheless
  the reason CT40 cannot be mapped, and the gap (publication custody is modeled
  in `qcore-core` but unreachable from C source) is named for a future card.
* **CT-CTWIN-UNRESOLVED-LEAK.** Registering an *owned local* through the
  uncontracted `q_register` yields a provisional `Unsafe`:
  `[subject:0:leak:7, subject:8:unsupported:call to uncontracted function
  \`q_register\`]`. The leak claim rests on the unresolved effect — if
  `q_register` takes custody, there is no leak — which sits in tension with the
  w4-nullable-removal rule that a residual leak *depending on* an unresolved
  effect stays Unknown. It affects no `Safe` verdict and it is not an upstream
  defect claim. It is pinned as a CT43 control (`owned_local.c`) so the current
  reading is visible rather than discovered later, and named for a future card.
* **CT-CTWIN-ENTRY.** See §5, mutation M20: the declared entry is not
  load-bearing for either mapped case.

## 2. The `legacy_c` route

The route vocabulary was already declared, with no member, by card
T00-CT-SCHEMA and reserved by name for exactly these five cases:
`[parse, legacy_c_twin, proof]`. It is used unchanged, and its shape is
what the census argues for: **neither mapped case has a QCore source form** —
the constructs are C constructs, which is why both sat in the batch-2
`unimplemented-syntax` bucket — so the case's artifact is a `.c` twin and there
is nothing to elaborate.

* `parse` — the gateway's own C admission: `validate_unrestricted_c11`, Clang
  preprocessing plus syntax acceptance of the case's own `.c` bytes. The runner
  calls it explicitly and asserts the preprocessed unit defines the analyzed
  entry, so a twin that "fails to analyze" only because Clang refused it would
  fail rather than measure nothing.
* `legacy_c_twin` — `analyze_unrestricted_c11`'s resource projection and unit
  verdict.
* Every other stage is **structurally absent**. `native_sanitized` is
  `not_applicable` under the same route rule card T08-SRC-CT narrowed for the
  check-only epoch route: the gateway analyzes C rather than producing any, so
  there is no `c_emission` stage and nothing to sanitize.
* `proof` is `unassessed` for both, as for every other CT case (T00.7 open).

### 2.1 The `legacy_c_twin` verdict mode

Following the wave-8 lesson (a fixture must not pin a convenient subset), the
mode measures the **whole verdict**, each half rebuilt from the analysis and
compared wholesale:

* the unit result (`safe` / `unknown` / `unsafe`);
* the withdrawal reason, **verbatim**, for an `unknown` unit;
* the **complete** per-function status map — every analyzed function, so
  omitting one is a mismatch (mutation M02);
* the **complete ordered** diagnostic inventory as `function:line:kind`,
  rendered by an explicit match in the harness rather than by `Debug`, so the
  pinned vocabulary is the harness's and cannot drift with a formatting change
  in the gateway.

Shape rules fail closed before any measurement: a `safe` unit may carry no
reason, no diagnostic and no undecided function; an `unknown` unit must state a
nonempty reason, at least one diagnostic and at least one undecided function;
an `unsafe` unit must name the function it accuses. Each case must carry at
least one control the gateway **checks** (mutation M05).

Controls come in two forms, and CT45 needs both:

* a **source control** analyzes its own `.c` file (CT43 `direct_call.c`,
  `owned_local.c`; CT45 `known_destructor.c`);
* a **contract-varying control** re-analyzes the *case's own bytes* under a
  different declared contract set. That is the form CT45's `expected` text
  literally asks about, and because it analyzes the case's own artifact it is
  counted as a second observation of that case's `legacy_c_twin` stage rather
  than as an `aux:` row — the one deliberate exception to "a control is never
  the case's own stage", recorded in the manifest note, in acceptance.json's
  note and in the archived evidence.

### 2.2 `tool_gate`: the Clang requirement, recorded

These stages run on **any** `target_os` and only where Clang is usable. A
stage record reading `host_gate: null` alone would say "this ran wherever the
tests ran", which is T00 finding O3 in the tool direction. Schema 2's stage
record therefore gains an optional `tool_gate` (registry: exactly `clang`), the
harness derives it from its own dispatch and refuses drift in both directions
(mutations M08, M17), and the derived summary gains a **`tool_scope`** axis
beside `host_scope`. `parse` now reads `["clang", "none"]` — honest: on the
task/mutex/epoch routes it is the QCore parser and needs nothing; on this route
it is a Clang invocation.

`schema_version` stays **2**: this is an additive per-stage evidence field and
a new route member, the same kind of extension wave 8 made when it added the
`epoch` route. `acceptance.json`'s pinned `generated_fields` list gains
`tool_scope` deliberately, in both derivations.

With no usable Clang the runner prints why the stages did not run and returns,
following the repository's existing convention for Clang-dependent tests; an
explicit but invalid `QCORE_CLANG` is an assertion failure, never a silent
fallback. The manifest's `executed` claim is qualified by exactly that
`tool_gate`.

## 3. Per-case rows, re-measured on fixture bytes

Every value below was measured through the harness on the committed fixture
bytes **before** the manifest was edited, and is re-measured on every run.

| Case | Executed stages | Unit verdict | Diagnostics (complete, ordered) |
|---|---|---|---|
| CT43 | parse (1), legacy_c_twin (1) | `unknown`, reason `resource projection unresolved: 1 unsupported operation(s), 0 unresolved alias(es), 0 custody summary conflict(s)`; `on_complete: safe`, `subject: unknown` | `subject:6:unsupported:call to uncontracted function \`q_register\`` |
| CT45 | parse (1), legacy_c_twin (2) | same reason string; `subject: unknown` | `subject:5:unsupported:call to uncontracted function \`q_destroy\`` |

Controls:

| Case | Control | Form | Outcome | What it isolates |
|---|---|---|---|---|
| CT43 | `direct_call.c` | source | `safe`, both functions safe, no diagnostic | the same callback body called here and now: the Unknown is the *deferral*, not the callback, the parameter or the declarations |
| CT43 | `owned_local.c` | source | `unsafe`, `[subject:0:leak:7, subject:8:unsupported:…q_register]` | the registration is not read as a hand-off — pinned as CT-CTWIN-UNRESOLVED-LEAK |
| CT45 | `case.c` + `destructor_contract.qcore` | contract-varying | `safe`, no diagnostic | the "require access contract" half of the expectation, over the *same bytes* |
| CT45 | `known_destructor.c` | source | `safe`, no diagnostic | the Unknown is the unknown destructor, not the parameter, the registry or the call shape |

Line numbers are positions in the **preprocessed** text (`-E -P`), which is
what the gateway reports.

The `semantic_profile` recorded for both is `UnrestrictedC11ResourceProjection`.
It is deliberately **not** a `SourceExecutionProfile` label: there is no source
route here, and inventing one would misdescribe what ran.

## 4. Derived numbers

`scripts/ct_stage_summary.py` and `ct_corpus.rs::ct_stage_summary` derive these
independently, and `ct_acceptance_summary_is_derived_from_the_manifest`
compares both against the committed `docs/reviews/r0_3/acceptance.json`:

```
ct_mapped 30   ct_unmapped 20
depth_profile     legacy_c_twin 2, epoch_bound_check 4, task_lowering 1,
                  task_ledger_check 4, mutex_invariant_check 7,
                  native_execution 9, native_sanitized 3
new observations  parse 28 -> 30, legacy_c_twin 3
tool_scope        legacy_c_twin ["clang"], parse ["clang","none"], all others ["none"]
native_sanitized  executed 3, recorded_not_gated 5, not_applicable 6, unassessed 16
proof             unassessed 30
```

`docs/reports/remaining-d02/build_matrix.py` regenerates its CT section from
the same manifest through the same summarizer;
`test_build_matrix_regenerates_the_ct_section_from_the_manifest` verifies it
row for row and passes. Its CT-derivation edits: a
`legacy_c_controls_executed` column beside the existing
`source_reject_controls_executed` / `epoch_controls_executed`, the new
`tool_scope` and `tool_gates_load_bearing` fields, the derivation-rule sentence,
and one note separating the 28 source-mapped cases from the 2 C twins.

`unmapped_bucket_counts` changes by construction: the bucket
`blocked-on-harness-schema (C-twin stage)` (5) is gone; CT40/CT41/CT42 are now
`blocked-on-gateway-expressiveness (C-twin measured)` (3), and CT43/CT45 leave
the unmapped set.

## 5. Fail-closed verification — 20 scratch mutations, 19 refused, 1 admitted and recorded

Each mutation was applied to a clean tree, run against
`cargo test -p qcore-compiler --test ct_corpus` and/or the Python summarizer
suite, then reverted with `git checkout -- .`. **None was committed**; the tree
was verified clean afterwards.

| # | Mutation | Refused by |
|---|---|---|
| M01 | CT43 diagnostic text: name a different uncontracted callee | `CT43 (case): the whole ordered diagnostic inventory` |
| M02 | CT43 function_status: drop `on_complete` (pin a convenient subset) | `CT43 (case): per-function statuses` |
| M03 | CT43 verdict: claim the twin is provisionally unsafe | `CT43 (case): gateway verdict` |
| M04 | CT43 withdrawal reason: inflate the unsupported count | `CT43 (case): the withdrawal reason is part of the measurement` |
| M05 | CT43 controls: drop the accepted control, keep only the refusal | ``CT43: a C twin needs a control the gateway checks, or its verdict says only that the projection cannot express this at all`` |
| M06 | CT45 contract-varying control: withhold the destructor contract | `CT45: fixture inventory differs from executed inputs` |
| M07 | CT45 manifest: `legacy_c_twin` observations 2 → 1 | derived plan mismatch + `acceptance.json observation_counts` drift |
| M08 | CT43 manifest: drop the parse stage's `tool_gate` | ``CT43: `parse` must record the exact external tools its runner needs`` |
| M09 | CT43 manifest: promote `native_sanitized` to executed | route rule + sanitizer gate + derived-summary drift |
| M10 | CT45 manifest: claim an `elaborate` stage on the `legacy_c` route | route rule (`not a stage of route legacy_c`) + summary drift |
| M11 | CT43 manifest: declare the `task` route instead | `CT43: the declared route must be the one this verdict mode really takes` |
| M12 | CT43 manifest: credit the stage to CT45's artifact | ``CT43: `legacy_c_twin` must be executed on the case's own fixture`` |
| M13 | `acceptance.json`: leave `ct_mapped` at 28 | Rust `ct_mapped is generated from the manifest` **and** the Python check |
| M14 | archived evidence: CT45 `legacy_c_twin` observations 2 → 3 | `test_archived_stage_evidence_agrees_with_the_manifest` |
| M15 | CT43 manifest: back to `unmapped` while its fixture exists | mapped-count pin (29 ≠ 30), fixture-exists gate, summary drift |
| M16 | CT45 `case.c`: call the contracted destructor instead | `CT45 (case): gateway verdict` (left `safe`, right `unknown`) |
| M17 | CT43 manifest: gate the stage on an unregistered tool (`gcc`) | tool registry + `acceptance.json tool_scope` drift |
| M18 | CT43 manifest: record the `reject` verdict mode | `CT43: the manifest must record the harness mode that produced its stage evidence` |
| M19 | CT45 contract-varying control: claim it stays Unknown with the contract declared | `CT45 (<case bytes>): gateway verdict` (left `safe`, right `unknown`) |
| M20 | CT43: analyze `on_complete` instead of `subject` | **ADMITTED** — see below |

**M20 is admitted, and that is a recorded finding.** The gateway's projection
covers every main-source definition, so both entries yield the same unit
verdict, the same per-function statuses and the same diagnostics: the declared
entry selects the summary's label, not what these two cases measure. The runner
now asserts the published summary is the one the declared entry asked for —
a weak check, described as weak in the code — and the boundary is named
**CT-CTWIN-ENTRY** rather than papered over. A future case whose evidence is
entry-dependent (a per-function certificate, say) would need a stronger binding.

## 6. Deliberate pin updates

1. `MAPPED_CASES` in `compiler/tests/ct_corpus.rs` gains CT43 and CT45; the
   mapped/unmapped counts move 28/22 → 30/20 in the harness doc comment, the
   pinned count test, `acceptance.json` (both derivations) and the regenerated
   matrix CT section.
2. `acceptance.json`'s `ct_evidence.generated_fields` gains `tool_scope` — the
   pinned "generated field set is itself pinned" assertion was updated in the
   same change, in both the Rust and Python generators.
3. `docs/reports/remaining-t00/coverage.json`: CT40/CT41/CT42's triage rows are
   rewritten with measured blockers and a new bucket; each keeps its original
   text under `superseded_blocker` / `superseded_bucket` / `superseded_by`,
   following the wave-3 precedent for correcting a stale T00 blocker label.
   (The file also gains a trailing newline it lacked.)
4. `acceptance.json`'s hand-maintained `ct_evidence` fields are corrected where
   this card's change made them false: the C-twin bucket count, the `note`'s
   "3 of 24 / 0 of 24" (now 30), and the `note`'s obsolete "`native_sanitized`
   is never `not_applicable`" sentence, which wave 8 had already narrowed in
   the manifest but not here. Two **pre-existing** staleness items are repaired
   in passing because they are in fields this card had to touch: `derived_from`
   did not list wave 8's evidence file, and `newly_decidable_not_mapped` still
   listed CT39, which wave 8 mapped.

Nothing else in `acceptance.json` was touched. One pre-existing divergence is
**recorded, not repaired**: that file's hand-maintained
`unmapped_bucket_counts` uses bucket names from several waves and now sums to
24 against 20 unmapped, while `build_matrix.py` derives its own counts from
`coverage.json` (which sum to 20). Repairing it means reconciling other waves'
bucket vocabularies, which is outside this card's ownership — named
**D02-BUCKETS** for the integrator.

## 7. Lean

**Not applicable, by the closing standard's own terms**, on the same grounds
recorded by cards T00-CT-SCHEMA and T08-SRC-CT: this card adds no checker rule,
no diagnostic code, no lowering and no model. It adds a harness route, two C
fixtures and manifest rows over an **unedited** gateway. `proof` stays
`unassessed` for both cases; no theorem is bound to any CT fixture (T00.7
remains open).

The Lean gate was run and passes: **44 audits**.

## 8. Standing per-change gate

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | ok (`third_party/libevent` at `d82464a2`) |
| `cargo test --workspace` | **1787 passed, 0 failed, 3 ignored** |
| `cargo fmt --all -- --check` | clean (both workspaces) |
| `cargo clippy --all-targets -- -D warnings` | clean (both workspaces) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | Passed **44** Lean audits |
| `python3 -B -m unittest discover -s scripts/tests` | 108 tests, OK |
| `cargo test` in `providers/` | 67 passed, 0 failed |

The workspace total is unchanged at 1787 because both new cases execute inside
the existing `ct_manifest_and_fixture_set_agree_and_mapped_cases_execute` test
rather than adding test functions.

One flake was observed and is the known one: the first full `cargo test
--workspace` run failed
`enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
(P02's load-flaky `RLIMIT_NPROC` assertion, recorded in
`docs/reports/remaining-p03/README.md` §9.1); it passed on the rerun above and
its assertion was not touched.

## 9. What this does not establish

* **Mapping is not conformance.** Two more cases execute their own fixture
  through real machinery. Sanitized-native evidence is unchanged at 3 of 30 and
  proof at 0 of 30; this route contributes neither and, having no emission,
  structurally cannot.
* **A different artifact and a different property.** The other 28 mapped cases
  are QCore source through `compile_files`. CT43 and CT45 are C twins through
  the unrestricted-C gateway, whose property is a resource projection over
  *declared* contracts. The `q_*` contracts in these fixtures are trusted test
  assumptions about a synthetic API, and CT45's `destructor_contract.qcore` is
  a declared premise about a body that is not in the unit — it discharges
  nothing about `q_destroy`.
* **The twins are synthetic.** They are shaped after the scenarios, not
  extracted from any upstream project, and no verdict here says anything about
  Fluent Bit, ChunkIO or any real code.
* **`Unknown` is not a proof of absence.** CT43 maps because the gateway
  declines, with the decline attributable to the registration; nothing here
  shows a callback does or does not run after return.
* **Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus stays 60
  modules; `qcore-schemas` stays 7.0.0; `qcore-compiler` stays 0.3.0; full
  language conformance stays false.

## 10. Integrator carry list

Files this card owns and changed: `docs/reevaluation/ct_manifest.json`,
`compiler/tests/ct_corpus.rs`, the two new
`compiler/tests/fixtures/ct/CT43/` and `CT45/` directories,
`scripts/ct_stage_summary.py`, `docs/reviews/r0_3/acceptance.json`,
`docs/reports/remaining-d02/build_matrix.py` (CT derivation only),
`docs/reports/remaining-t00/coverage.json` (three triage rows), and this report
plus its `ct_stage_evidence.json`.

The integrator must:

1. **Regenerate `docs/reviews/r0_3/acceptance_matrix.json`'s `ct_conformance`
   section** from `docs/reports/remaining-d02/build_matrix.py`, adopting it
   surgically as waves 6 and 8 did (the generator's non-CT sections remain the
   stale hand-coded snapshot, open as `D02-GEN-REST`). The regenerated CT
   section reads mapped **30** / unmapped **20**,
   `depth_profile.legacy_c_twin = 2`, and adds `tool_scope`,
   `tool_gates_load_bearing` and a `legacy_c_controls_executed` column.
2. **Update `named_followups`** in the same file: `CT-CTWIN` is closed;
   **`CT-CTWIN-PUBLISH`**, **`CT-CTWIN-SYNC`**, **`CT-CTWIN-DESCRIPTOR`**,
   **`CT-CTWIN-UNRESOLVED-LEAK`**, **`CT-CTWIN-ENTRY`** and **`D02-BUCKETS`**
   are new and open.
3. **Carry the status paragraphs** in `AGENTS.md`, `CLAUDE.md` and
   `DEVELOPMENT.md`: CT coverage 28/50 → **30/50 mapped**, unmapped 22 → **20**,
   sanitized-native **3 of 30**, proof **0 of 30**, nothing promoted. Note that
   28 of the 30 are QCore source and 2 are C twins on the gateway. The gate line
   at this checkpoint is 1787 workspace tests (3 ignored), 67 provider tests,
   108 Python tests, 44 Lean audits, both probes exit 0, fmt and clippy clean.
4. **Note the additive schema extension** when reconciling documentation: a
   schema-2 stage record may now carry `tool_gate`, and the derived summary a
   `tool_scope` axis; `schema_version` stays 2 and existing rows are unchanged.
