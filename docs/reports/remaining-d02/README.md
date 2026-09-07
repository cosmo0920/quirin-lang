# D02 — Revision 0.3 implementation and acceptance matrix

**Date:** 2026-09-07. **Checkpoint:** `67b1a9a` (*Merge C02 typed direct
scalar-field access classification*), the complete merged remaining-work wave.
**Machine-readable deliverable:** [`../../reviews/r0_3/acceptance_matrix.json`](../../reviews/r0_3/acceptance_matrix.json).
**Updated ledger:** [`../../reviews/r0_3/acceptance.json`](../../reviews/r0_3/acceptance.json).

**Reproduce:**

```sh
python3 -B docs/reports/remaining-d02/build_matrix.py "$PWD" <workspace-test-count>
python3 -B docs/reports/remaining-d02/update_acceptance.py "$PWD" <head-sha> \
    <workspace-tests> <provider-tests> <lean-audits>
```

`build_matrix.py` reads every count it emits out of a manifest or an on-disk
fact — T00's `coverage.json` for CT stages, B00's `ledger.json` for the corpus
and checkpoint reconciliation, `contract/qcore/*/*.qcore`, `Cargo.toml`,
`providers/Cargo.toml`, `crates/qcore-schemas/Cargo.toml` and `proof/Audit*.lean`
— and asserts the CT inventory it derives against `ct_manifest.json` before
writing. The requirement rows are the analysis and cite the report that
established each cell. `update_acceptance.py` asserts that exactly the rows it
names changed.

This is the closing accounting card of the wave. It writes no code, changes no
pinned expectation, promotes no verdict, maps no CT case and rewrites no
historical report. It reconciles what the wave's twenty-five reports establish
into one traceable requirement → implemented profile → source/model entry point
→ executed test stage → proof/evidence → remaining-boundary matrix.

**Counts in this card and in the matrix are run metadata.** A count says what
was executed, not what was proved. Passing suite totals, CT mapping counts,
contract-module counts and audit unit totals are not specification coverage:
projection coverage is not safety coverage, unknowns stay in the denominator,
and a provisional `Unsafe` is never presented as an upstream defect. Full
language conformance remains **false** and raw Cycle 19 units remain
**unpromoted**.

---

## 1. Matrix dimensions

| Dimension | Size |
|---|---:|
| Requirements | **17** — 10 from the supplied r0.3 review (R01–R10) + 7 introduced by this wave (W01–W07) |
| Evidence classes | **4** — source · model · observation · mechanized |
| Requirement × evidence cells | **68** |
| CT rows | **50** (11 mapped, 39 unmapped) |
| CT stage names | **12** |
| CT row × stage cells | **600** |
| `unverified_specs` items dispositioned | **7** |
| Named follow-ups | **11** (plus the two pre-existing `open_followups`) |

The four evidence classes are marked **independently**. Support in one class is
never inferred from another: a proved Lean theorem is not source evidence, an
executed native run is not a model proof, and a model regression is not an
observation. Status vocabulary is `executed` (a named standing-gate step runs it
at HEAD) · `recorded_not_gated` (an archived probe log or transcript that
nothing re-runs) · `declared_premise` · `absent` · `not_applicable`.

### Independent support, tallied

| Class | executed | recorded_not_gated | absent | not_applicable |
|---|---:|---:|---:|---:|
| source | 12 | 0 | 4 | 1 |
| model | 10 | 0 | 1 | 6 |
| observation | 8 | 7 | 2 | 0 |
| mechanized | 6 | 0 | 4 | 7 |

The four `source: absent` rows are the honest ones: R01 and R03 (the concurrency
checkers consume trusted explicit traces; no source lowering exists), R06 (no
source loop is admitted at all — V05 measured `Q500` on every loop probe), and
W07 (a discovery card whose constructor call statement is rejected before its
declared contract is consulted). The seven `observation: recorded_not_gated`
rows are the CLI probe matrices of V03/V04/V06/V07/D01/O00 and V05's loop
probes: real executed evidence, archived with its sources, that no standing gate
replays.

---

## 2. Requirement rows

`S/M/O/P` = source / model / observation / mechanized. `E` executed ·
`R` recorded, not gated · `—` absent · `n/a` not applicable.

| ID | Requirement | Implemented profile | S | M | O | P | Acceptance status |
|---|---|---|:-:|:-:|:-:|:-:|---|
| R01 | Destroyed-generation access and destruction quiescence | `TrustedExplicitTrace` | — | E | — | E | repaired_model |
| R02 | Terminal child task execution after join | `TrustedExplicitTrace` + `ScopedTaskLedger` + `ScopedOwnedTasksV1`/`ScopedCopyTasksV1` | E | E | E | E | repaired_model |
| R03 | Explicit mutex and synchronization trust boundary | **`CheckedMutexInvariantV1`** (additive) | — | E | — | E | **checked_profile_added_model_only** |
| R04 | Ordinary source result type and exit compatibility | `SequentialCleanupV1` (i32/unit) + task profiles (`own<Buffer>`) | E | n/a | R | — | partially_repaired |
| R05 | Unconditional axis declaration validation | unconditional declaration environment | E | n/a | R | — | repaired_source |
| R06 | Exact versus maximum loop custody | `LoopBoundEvidence` over supplied premises | — | E | R | E | repaired_model |
| R07 | Signed decimal minimum literals | three i32 routes + the i128 observation route | E | n/a | R | — | repaired_consumers |
| R08 | Consumer-specific contract coverage | `qcore.c_boolean_field_contract.v2`/`.v3` | E | E | E | E | validated_bounded_consumers |
| R09 | Current specification, commands and diagnostic ownership | documentation reconciliation; `source_diagnostic` is the authoritative map | E | n/a | R | n/a | reconciled |
| R10 | Artifact/evidence composition and snapshot provenance | `BooleanSourceIdentity` + `CompileTaskIdentity` + `CSourceInput` + `ProcessExecutionProfile` | E | E | E | n/a | hardened_bounded_evidence |
| W01 | Typed direct scalar-field access classification | eight-premise conjunctive whitelist | E | E | E | E | implemented_bounded |
| W02 | Provider layering, lifecycle and enforced execution profile | `ProcessExecutionProfile::{UnenforcedLegacy, LinuxRlimitV1}` | n/a | E | E | n/a | implemented_bounded |
| W03 | A second pinned source set from configuration alone | source-set manifest v1 + audit configuration v1 | E | n/a | E | n/a | demonstrated_bounded |
| W04 | Source-derived obligation completeness and replay resistance | core-computed obligation IDs and coverage denominators | E | E | E | n/a | audited_bounded |
| W05 | Compile-task identity and declared source input transport | `CompileTaskIdentity` v1 + `CSourceInput` | E | E | E | n/a | implemented_bounded |
| W06 | Diagnostic ownership and source attribution | per-file offset attribution; slice-mapped declaration spans | E | n/a | R | n/a | implemented_bounded |
| W07 | Constructor-to-caller premise inventory | none (discovery) | — | — | R | — | discovery_only |

Every cell in the matrix carries its entry point, the test stage that executes
it, the wave report that established it, and the requirement's remaining
boundary. The boundaries are the load-bearing half: the matrix is not a
scoreboard, and no row's `executed` mark widens the scope its boundary states.

---

## 3. `unverified_specs` dispositions (B02)

These are **declared inputs, not implementation authority.** A disposition
records what this repository adopted, not what it implements; implementation
status lives in the requirement rows above. `unverified_specs/` is untracked in
the primary checkout and therefore absent from every worktree, and B02 verified
all five files byte-identical (sha256) to their archived copies under
`docs/reviews/r0_3/supplied/`.

| Item | Disposition |
|---|---|
| `REVIEW.md` | adopted |
| `QCORE_SPEC_r0_3.md` | adopted-and-amended |
| `QCORE_CONTRACT_SPEC_r0_3.md` | adopted-and-amended |
| `QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | adopted-and-amended |
| `QCORE_DESIGN_GRAMMAR_r0_3.bnf` | adopted-as-target-only |
| `docs/reviews/r0_3/probe/` (advertised Rust probe package) | superseded — never supplied; replaced by in-tree regressions and the 268 live CLI probe rows of V03/V04/V06/V07 |
| the reviewer's archive, `acceptance.json`, `source_inventory.json`, `validation.json` | superseded — never supplied; the local acceptance record is a new local record and the 43-test Python suite plus the standing gate replace `validation.json` |

---

## 4. CT rows are stage-derived

Per T00, a stage counts as `executed` only where a named workspace test runs it
on the case's own artifact. The matrix keeps `manifest_stages` beside
`stage_derived_status` so the divergence stays visible rather than being
resolved silently, and **no row claims a stage its executed evidence does not
carry.**

Derived for the Linux reporting host, from
`docs/reports/remaining-t00/coverage.json`:

| Stage | executed | recorded_not_gated | not_applicable | unassessed |
|---|---:|---:|---:|---:|
| `parse` / `elaborate` / `task_lowering` / `task_ledger_check` | 11 | 0 | 0 | 0 |
| `sequential_whole_program` | 0 | 0 | 11 | 0 |
| `sequential_reference_execution` | 0 | 0 | 11 | 0 |
| `task_reference_execution` | 9 | 0 | 2 | 0 |
| `c_emission` | 9 | 0 | 2 | 0 |
| `native_execution` | 9 | 0 | 2 | 0 |
| `native_sanitized` | **0** | 5 | 0 | 6 |
| `legacy_c_twin` | 0 | 0 | 11 | 0 |
| `proof` | 0 | 0 | 0 | 11 |

**Depth profile:** `native_execution` 9 · `task_ledger_check` 2 (CT02 and CT19,
which reject at `Q600` inside the task ledger — the deepest source stage
available to them). **Observation counts:** 18 task reference executions, 19
native compile-and-run cycles, 11 in-case source rejection controls, 0 gated
sanitized cases, 0 cases with proof evidence.

Three structural facts the scalars hide, all recorded in the matrix:

- All 11 mapped cases take `compile_task_source`, which never calls
  `synthesize_semantics` or `check_whole_program` and returns
  `reference_observation: None`. The **sequential** resource CFG and its
  reference execution run on no mapped case's own fixture, so the manifest's
  `cfg: executed` names the task IR, not the sequential machinery.
- The sanitized-native evidence for CT14/CT16/CT20/CT37/CT50 is
  `recorded_not_gated`: the ASan/UBSan/LSan transcripts live in
  `docs/reports/w6-*/README.md` and nothing re-runs them.
- `proof` is `unassessed` for all 11 and no Lean theorem is bound to any CT
  fixture.

**Unmapped buckets (39):** no mutex value 11 · other unimplemented syntax 7 ·
harness-schema C-twin stage 5 · share captures at task lowering 4 · W3/W4 4 ·
typed error payloads 2 · decidable-now 3 · ambiguous 3. T02 added the
`CheckedMutexInvariantV1` **model** after T00's audit and deliberately mapped no
CT case, so the eleven "no mutex value" rows keep their source blocker. The
three decidable-now cases (CT24, CT39, CT47) are assessed from probes, not
mapped, and two of the three would record a coarser reason than their spec row.

---

## 5. Version separation

Prose revision **0.3** is the specification revision. It does not move with, and
is not moved by, any of these:

| Surface | Version | Moved by this wave |
|---|---|---|
| Prose specification / documentation revision | 0.3 | no |
| `qcore-schemas` Rust API | **3.0.0** | **yes** — P02 added the public `PluginLimits::additional` field, a major change under the documented policy. Serde output for a manifest declaring no additional kinds is byte-identical |
| Plugin protocol | v1 | no |
| Semantic evidence | v2 | no |
| Program schema / profile | v2 | no |
| Source-set profile manifest | v1 | no |
| `compile_task_identity_version` | 1 | new (G01) |
| Provider packages | ChunkIO 0.4.0 · CMetrics 0.2.0 · CTraces/Libevent/librdkafka 0.1.0 | no |

---

## 6. What changed in `acceptance.json`, and why

All ten R rows changed, each because a wave report produced evidence for it.
Nothing was rewritten for tidiness.

| Row | Change | Cause |
|---|---|---|
| R01 | evidence += V01; boundary adds "no source-to-trace refinement" | V01's adversarial pass over the liveness gate: four closed coverage gaps, four new Lean destroy-arm theorems, no soundness failure |
| R02 | evidence += T00; boundary restated | T00 showed the mapped CT stage labels exceed executed evidence; T02 moved "checked mutex invariants" from open to a separate, uncomposed model profile |
| R03 | **status `explicit_trust_boundary` → `checked_profile_added_model_only`**; evidence += T01, T02, B02; boundary rewritten | B02 classified R03 unsupported-by-profile because no checked-invariant profile existed. T01 specified one; T02 implemented it with 48 tests and 31 registered Lean theorems, leaving `TrustedExplicitTrace` unchanged |
| R04 | boundary **replaced** (drift repair); evidence += V03, B02 | B02 found this row's boundary was a verbatim copy of R02's task-lifetime text, saying nothing about result typing. V03's 71 probes then superseded B02's own residual flag and named the four `Q500`-only rows and boundaries B-1/B-2 |
| R05 | evidence += V04, D01; boundary records the span repairs and the retained F3 | V04's 28-row matrix found three attribution defects on the rejecting path; D01 repaired F1 and F2 |
| R06 | evidence += V05; boundary adds source unreachability | V05 measured that no source loop is admitted, so the Exact/AtMost distinction is unreachable from source |
| R07 | evidence += V06; boundary adds the third route | V06 found a third signed literal route (checked-task scalar operand) with its own range rule that neither cited test covers |
| R08 | evidence += E01, E02, G03; boundary names E01-B1/B2, E02-B1..B4, G03-F1/F2 | obligation-inventory audit, 52-row mutation matrix, second configured source set |
| R09 | evidence += D01, B00, B02, V07; boundary records the ownership table and the unimplemented reason codes | D01 made the L21 table canonical and resolved the `Q700`–`Q703` drift in favour of the code |
| R10 | evidence += G01, G02, E02, P01, P02; boundary names every new exclusion | compile-task identity, declared input transport, mutation matrix, provider layering audit, enforced execution profile |

Top-level: `validation` moves to 997 workspace / 47 provider / 43 Python /
21 Lean audits; `latest_verified_implementation_revision` moves from `e3e7c2c`
to `67b1a9a`; new `acceptance_matrix`, `counts_are_run_metadata`,
`schema_versions`, `ct_evidence`, `unverified_specs_disposition` and
`wave_2026_09_07` keys; eleven named follow-ups appended to `open_followups`
beside the two that were already there.

`ct_mapped: 11` / `ct_unmapped: 39` are **retained**, with T00's O7 finding
recorded next to them: the scalars treat a source-rejection case with zero
native runs and a case with four reference executions plus four native runs as
one identical unit of coverage, and hide that 0 of 11 mapped cases carry proof
or gated sanitized evidence.

---

## 7. Documentation reconciliation (B00 F1/F2)

B00 recorded two documentation-completeness findings and did not fix them
(it was a read-only ledger). They are repaired here deliberately:

- **F1.** `AGENTS.md`'s repository map had 25 rows and no row for
  `crates/qcore-audit/` or `providers/`, both of which exist and were described
  only in prose further down. Two rows are added.
- **F2.** `AGENTS.md` said "The current workspace has nine crates." Nine is the
  `crates/` directory count; the Cargo workspace declares **ten** members (those
  nine plus `compiler`), and `providers/` is a separate six-member workspace.
  The sentence is corrected to say exactly that.

Both are verified against `Cargo.toml` and `providers/Cargo.toml` by the matrix
generator, not asserted.

B00's F3 (a first `git submodule status` of a fresh session may print the
uninitialized `-` prefix while the checkout is present, clean and at the pin) is
a reporting transient and needs no action; the submodule pin
`d82464a277d0f42703702c4dfd9af6af38595a83` reconciles across the gitlink, the
checkout and `contract/profiles/libevent/profile.json`.

---

## 8. Corpus figures carried forward, unchanged and unpromoted

| Item | Value | Re-measured here |
|---|---|---|
| Pinned 185-unit corpus | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | **no** |
| Full-corpus process sidecar | 3 checked function contracts / 93 Unknown | no |
| Configured CMetrics audit | 1 checked / 6 Unknown; raw unit provisional Unsafe; `unsupported` 36 → 35 after C02 | by C02 and G02, from the persistent build |
| Configured `chunkio-core` audit | 13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown | by G03 |
| Contract corpus | 59 modules; 100 extern entries across 32 files, 519 pinned clauses | counted on disk |
| CT | 11 mapped / 39 unmapped | inventory only; stages derived from T00 |

The four archived 185-unit artifacts were **not** edited. C02 touched shared
analysis code and G02 changed the declared input transport, so they would move
in both directions — including possible newly surfaced provisional `Unsafe`
units — and hard invariants 3 and 5 require them to be regenerated, diffed,
archived and re-pinned deliberately. The pin gate correctly refused to run the
audit in this environment and nothing was adjusted to fit. That is the
`CORPUS-REPIN` follow-up.

---

## 9. Named follow-ups

Recorded in the matrix and appended to `acceptance.json`'s `open_followups`.

| ID | Follow-up |
|---|---|
| `CORPUS-REPIN` | re-measure and re-pin the 185-unit corpus after C02 and G02 |
| `E01-F1` | complete the source function enumeration (macro-expanded and K&R definitions) |
| `C02-P` | borrowed pointer-field reads as contracted-call actuals |
| `O04` | per-arm constructor-to-caller cleanup discharge on four early-return arms |
| `T03` | source lowering for `CheckedMutexInvariantV1` |
| `P03` | filesystem and network confinement for process providers |
| `P01-F2` | bind non-digest manifest declarations to the embedded template at enable time |
| `G03-F1` | represent a source set rooted at the checkout root |
| `G03-F2` | consume a second real CMake project's unmodified compile database |
| `REASON-CODES` | implement the L21.1 reason-code vocabulary |
| `T00-CT-SCHEMA` | make the CT manifest and its summarizer stage-derived (T00.1–T00.9) |

---

## 10. Validation

Run at `67b1a9a` plus this card's documentation changes. This card touches only
Markdown and two files under `docs/reviews/`, and no test reads either of them.

| Gate | Result |
|---|---|
| `cargo test --workspace` | **997 passed, 0 failed, 0 ignored** across 73 suites, exit 0 |
| `cargo test -p qcore-compiler --test ct_corpus` | 4 passed, exit 0 |
| `cargo test -p qcore-compiler --test qcore_contract_corpus` | passed (59-module and 519-clause pins intact), exit 0 |
| `cargo test -p qcore-compiler --test module_modes` | passed, exit 0 |
| `cargo test -p qcore-compiler --test libevent_submodule` | passed, exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `python3 -B -m unittest discover -s scripts/tests` | 43 ran, OK |
| `python3 -B scripts/check_lean_audits.py` | 21 Lean audits passed |

---

## 11. What this card does not establish

- It executes no new analysis and adds no capability. Every requirement cell
  restates evidence a wave report already produced, at that report's own stated
  boundary.
- The matrix is a **traceability** artifact. An `executed` mark means a named
  gate runs that class of evidence; it is not a claim of completeness for the
  requirement, and it never widens the boundary recorded beside it.
- No raw Cycle 19 unit is promoted, no CT case is mapped, no pinned expectation
  is edited, no diagnostic is downgraded and no fail-closed gate is weakened.
  Full language conformance remains false.
- The wave's own non-claims stand unchanged: model-level results establish no
  source-to-trace refinement; native observations are fixed injected runs at
  harness callers; declared profiles, contracts and policies remain trusted
  declarations; and process separation is not an OS sandbox.
