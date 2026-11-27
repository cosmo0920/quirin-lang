# GEN-REASON — the D02 generator's non-CT sections, and the last three reason-code residuals

Card: **GEN-REASON** (follow-ons **D02-GEN-REST** and **REASON-CODES-B**), wave 9.
Date: 2026-09-09. Base: `05c51a5` (*Consolidate the wave-8 increment into the
status ledgers and follow-ons*). Branch: `claude/wave9-gen-reason`.

**Nothing is promoted.** No verdict moved on any unit or any function. Raw
Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units, CT coverage stays 28/50 mapped, the contract corpus stays 60
modules, `qcore-schemas` stays 7.0.0 and full language conformance stays false.
No archived artifact byte was edited: §5.3 records the one measured
pinned-byte movement this card creates and hands it, unedited, to the next
deliberate corpus re-pin.

---

## 1. What each half was, and what it became

| Follow-on | Recorded state before | Disposition |
|---|---|---|
| **D02-GEN-REST** | only `ct_conformance` was derived; the requirement rows, unverified-spec dispositions, header prose and `named_followups` were Python literals from the original D02 wave, and the gate block hard-coded its counts. A whole-file regeneration reverted every later reconciliation. | **Closed.** Every section is now derived, integrator-supplied or explicitly labelled hand-maintained *in the output*; the hand-maintained ones are carried from the committed matrix instead of being held as a second copy. §2. |
| **REASON-CODES-B1** (opaque OS/tool text) | "likely unclosable" | **Closed on the reachable surface, open on the rest.** The reachable surface is exactly three conversions, and it was smaller than the follow-on assumed. §5.1. |
| **REASON-CODES-B2** (`ChunkSourceResult::Unknown` untyped) | open | **Closed, byte-neutral.** §5.2. |
| **REASON-CODES-B3** (adapter-assembled sidecars carry no `reason_code`) | open | **Implemented; pinned bytes deliberately NOT moved.** The emitter carries the field; the archived artifacts are left for the next re-pin, with the exact delta measured and pinned. §5.3. |

---

## 2. D02-GEN-REST — per-section disposition

`docs/reports/remaining-d02/build_matrix.py` now publishes its own ownership
split in a new top-level `section_ownership` block, and asserts that **every
emitted section is labelled exactly once**. Three owners:

| Owner | Means |
|---|---|
| `derived` | recomputed on every run from a named manifest, validated record or the working tree. A hand edit here is overwritten, deliberately. |
| `integrator_supplied` | a measurement the script cannot take. From `--gate PATH`; without it the previous numbers are carried **and labelled as carried**, so a stale count is visible in the file rather than invented. |
| `hand_maintained` | recorded analysis the integrator owns. Carried verbatim from the previous matrix; the script never authors or reverts it. |

### 2.1 The complete table

| Section | Owner | Source |
|---|---|---|
| `schema_version`, `card`, `generated`, `report`, `acceptance_record` | derived | generator constants |
| `section_ownership` | derived | this block |
| `header` | hand_maintained | vocabulary and framing prose |
| `versions.qcore_schemas_rust_api` | derived | `crates/qcore-schemas/Cargo.toml` |
| `versions.provider_packages` | derived | **new** — `providers/*/provider-template.json` |
| `versions.*` (prose, protocol/evidence versions, version-cause note) | hand_maintained | carried |
| `dimensions` | derived | counted from the emitted rows |
| `derivation` | derived | generator constant naming its own inputs |
| `unverified_specs` | hand_maintained | B02's disposition of the supplied inputs |
| `requirements` | hand_maintained | R01–R10 / W01–W07 analysis rows, revalidated here |
| `ct_conformance` | derived | **unchanged logic** — the manifest through `scripts/ct_stage_summary.py` |
| `corpus.contract_modules`, `contract_modules_pinned_by`, `contract_clause_pin`, `contract_modules_per_directory`, `libevent_submodule_pin` | derived | disk + `remaining-b00/ledger.json` |
| `corpus.*` (the measured audit figures) | hand_maintained | carried; the measurement lives in the card report named beside each |
| `workspace` | derived | `Cargo.toml`, `providers/Cargo.toml` |
| `validation.lean_audits`, `lean_audit_files`, `full_language_conformance`, `measurement_provenance` | derived | `proof/Audit*.lean` on disk / constants |
| `validation.*` (the standing-gate numbers) | integrator_supplied | `--gate`, else carried |
| `named_followups` | hand_maintained | the open follow-on ledger |
| `updated` | integrator_supplied | the integrator's own note |

### 2.2 What the defect actually was, measured

Regenerating the committed matrix with the *old* generator changed these
sections, all in the wrong direction:

| Section | Old generator would have written | Committed value |
|---|---|---|
| `named_followups` | the original 11-entry literal (`CORPUS-REPIN`, `E01-F1`, `C02-P`, `O04`, `T03`, `P03`, `P01-F2`, `G03-F1`, `G03-F2`, `REASON-CODES`, `T00-CT-SCHEMA`) | 19 entries, including this card's own two |
| `validation.provider_tests` | `47` | `51`, and the real number today is `67` |
| `validation.python_evidence_validator_tests` | `43` | `43`, real number today `125` |
| `versions.provider_packages` | a 2026-09-07 literal (`qcore.chunkio: 0.4.0`) | `0.5.0`, and the real template says `0.6.0` |
| `versions.qcore_schemas_version_cause` | the P02-only sentence | the P02+P04 sentence |
| `updated` | **dropped entirely** — the generator never emitted the key | the wave-8 note |

`requirements`, `unverified_specs` and `header` were measured **identical**
between the literal and the committed file: nobody had hand-edited them, so
the clobber risk there was latent rather than realised. They are carried
anyway, because "nobody has edited it yet" is not a property.

### 2.3 Fail-closed behaviour added

* A **missing previous matrix** refuses with a message naming what it needs,
  and writes nothing — rather than emitting a matrix stripped of every
  hand-maintained section.
* A previous matrix **missing a carried section** refuses by name.
* A **partial `--gate`** refuses rather than merging; an **unknown `--gate` key**
  refuses by name.
* Carried requirement rows are **revalidated** against the closed class
  vocabulary: an invented `status` word or a missing evidence class refuses.
  This is strictly stronger than before, when the validator ran over the
  literal it was written beside.
* `header.support_status_vocabulary` is compared against the script's own
  `CLASS_VOCABULARY`; a hand edit that widened the published vocabulary would
  otherwise silently widen what the rows are checked against, so drift refuses.

### 2.4 The new tests

`scripts/tests/test_build_matrix_sections.py`, 13 tests:

| Test | Property |
|---|---|
| `..._preserves_the_sections_the_generator_does_not_own` | a regeneration over the committed matrix leaves `header`, `requirements`, `unverified_specs`, `named_followups`, `updated` and all three mixed sections' prose byte-identical |
| `..._is_a_fixed_point_for_the_sections_it_owns` | a second regeneration over the generator's own output is identical (except `section_ownership.carried_from`, which records *where this run read from* and differs by construction) |
| `every_emitted_section_is_labelled_exactly_once` | the published ownership map's keys equal the emitted keys, and each section carries the owner this test states independently |
| `the_derived_facts_are_recomputed_rather_than_carried` | the four derived facts equal a fresh independent reading of the tree, with an explicit non-vacuity assertion that at least one really was stale |
| `gate_numbers_are_supplied_or_labelled_as_carried` | `--gate` is honoured; without it the provenance string says the numbers were **not** measured |
| five refusal tests | partial gate, unknown gate key, missing previous, missing carried section, widened vocabulary, invented status word, missing evidence class |
| `the_positional_workspace_count_still_works` | T00-CT-SCHEMA's documented command `build_matrix.py . <count>` is unchanged |

**The CT section is untouched.** Its derivation code is byte-identical and
`scripts/tests/test_ct_stage_summary.py::test_build_matrix_regenerates_the_ct_section_from_the_manifest`
passes unchanged; the regenerated `ct_conformance` reproduces 28/50 mapped with
the same histogram, depth profile, observation counts and `case_depth`.

### 2.5 For the integrator

Regenerate with the measured gate:

```sh
python3 -B docs/reports/remaining-d02/build_matrix.py . --gate GATE.json
```

where `GATE.json` carries exactly the nine keys the script names in its refusal
message. Without `--gate` the run still succeeds and the file says, in
`validation.measurement_provenance`, that its gate numbers were carried and not
measured.

Adopting the regenerated file applies, beyond the CT section: `contract_modules`
59 → 60 (O01's deliberate pin update, which the matrix had not picked up),
`qcore_schemas_rust_api` 4.0.0 → 7.0.0, `lean_audits` 21 → 45 with the current
file list, `provider_packages` to the template versions, and the new
`section_ownership` block. `acceptance_matrix.json` is **not edited by this
card**, per the standing constraint.

---

## 3. Reason-code carriers — the shape of the repair

`qcore_schemas::reason` is unchanged. What this card adds is *how a row carries*
the category it already had a vocabulary for:

* **text route** — recover it from the reason text (`ReasonCategory::classify`,
  unchanged; `UnitRefusal::classified`, `ChunkSourceResult::unknown`);
* **site route** — name it where the text is opaque
  (`UnitRefusal::environment`).

There is deliberately **no constructor that names a source-subject category at a
site.** `environment-unavailable` is the sole inhabitant of
`ReasonSubject::Environment`, so a site-named row can never be read as a
statement about the analysed source and can never contribute to a provisional
`Unsafe`. Naming at a site is therefore a *narrowing* of the `null` it replaces,
never a widening — that is the whole safety argument, and §6 proves it.

---

## 4. REASON-CODES-B2 — the typed `ChunkSourceResult`

`crates/qcore-c-gateway/src/boolean_source.rs`:

```rust
Unknown {
    function: String,
    reason: String,
    reason_code: Option<ReasonCategory>,   // fail-closed to None
}
```

built only by `ChunkSourceResult::unknown`, which classifies once at the enum's
**single** construction site. Fail-closed in both directions: an unrecognised
reason carries `None`, never a defaulted category, and `None` asserts nothing.

`crates/qcore-audit/src/source_audit.rs` now publishes that category through the
new `report::reason_code_value` instead of re-classifying the same text, so the
importer and the report cannot disagree; a unit test pins that the two routes
agree on a reason both can see.

**Pinned-byte measurement: none moved.** `ChunkSourceResult` derives `Serialize`
only, and no route serializes the enum whole — every consumer destructures it.
The compatibility facade in
`compiler/src/verifier_plugins/chunkio/chunk_source.rs` mirrors the field and
carries it across rather than re-deriving it, so the facade does not become a
second classification either.

Five held-out tests
(`crates/qcore-c-gateway/tests/chunk_source_reason_codes.rs`, Clang-free):
the three highest-frequency archived reasons classify; the row keeps its exact
`function` and `reason` bytes; three unrecognised texts carry `None`; two
environment reasons — one minted by this very module — classify to
`EnvironmentUnavailable` with `subject() == Environment` and
`describes_analysed_source() == false`; a pinned-revision mismatch is
`StaleEvidence` and likewise not a source statement.

---

## 5. REASON-CODES-B1 and B3

### 5.1 B1 — measured, and smaller than the follow-on assumed

The follow-on called this "opaque OS/tool reason text" without bounding it.
Measured census of the route that produces report rows:

* `crates/qcore-audit` has **50** `error.to_string()` conversions;
* exactly **one** production `record_unknown` call site exists
  (`current.rs:221`), fed by one closure;
* exactly **three** opaque conversions can reach it:
  1. `fs::read(&entry.file)` — `current.rs`;
  2. `path.canonicalize()` — `verify_pinned_bytes`, `lib.rs`;
  3. `Command::new("git") … .output()` — `verify_pinned_bytes`, `lib.rs`.

Every other refusal reaching that closure is already a classified string
(`source is not UTF-8`, the compile-command refusals, `source outside selected
checkout`, `selected source … does not match its pinned bytes`).

So B1 is **closable on the reachable surface**, and it is closed:
`report::UnitRefusal` lets those three sites name `EnvironmentUnavailable`
without changing one byte of reason text (§L21.1 rule 2 keeps the text, span,
property, profile, scope, bounds, assumptions and obligation id untouched).
`verify_pinned_bytes` returns `UnitRefusal`; its two non-report callers narrow
to the text with `into_reason`, and that loss is recorded inline as a residual —
they return a whole-run `Err(String)`, so the category has nowhere to travel.

**Pinned-byte measurement: none moved.** In the current baseline
(`docs/reports/remaining-e01-f1/artifacts/after/diagnostics.json.gz`) the
`reason_code` distribution is `unsupported-syntax` 136, `null` 48,
`incomplete-obligation-coverage` 1 — and **all 48 nulls are units with no
`reason` at all**. Zero archived rows carry an unclassified reason, so B1 had
no occurrence in the pinned corpus and repairing it changes nothing there.
That is pinned by `scripts/tests/test_sidecar_reason_inventory.py`.

**Recorded open (B1 residual):** the 47 other `error.to_string()` conversions
in `qcore-audit` sit on `Result<_, String>` channels that end in a whole-run
`Err`, not a report row. Typing them means rewriting those channels, which is
a separate increment; none of them can produce an uncoded *row*.

### 5.2 B3 — implemented in the emitter, pinned bytes deliberately not moved

`compiler/examples/support/cycle19_compatibility.rs` assembles
`source-effects.json` and `chunk-contracts.json` (this is why they carry no
`reason_code` while `qcore-audit`'s `diagnostics.json` does — CORPUS-REPIN
measured that and it is a documentation error in REASON-CODES §7, not an
implementation defect). Both sidecars' `unknown` rows now carry `reason_code`,
**sourced from B2's typed field**, so this is not a third vocabulary. The key is
emitted only on `unknown` rows, so `checked` rows keep their exact bytes.

### 5.3 The blocking measurement, for the next corpus re-pin

Measured read-only on the current baseline
(`docs/reports/remaining-e01-f1/artifacts/after/`):

| Artifact | Rows | `unknown` rows | Distinct reasons |
|---|---|---|---|
| `source-effects.json` | 96 | 93 | 4 |
| `chunk-contracts.json` | 96 | 93 | 4 |

| Reason | Count (each sidecar) | Category |
|---|---|---|
| `outside single borrowed chunk / int-or-void-result source profile` | 74 | `unsupported-syntax` |
| `unmodeled statement DeclStmt` | 13 | `unsupported-syntax` |
| `unmodeled expression CallExpr` | 4 | `unsupported-syntax` |
| `expected one selected definition after preprocessing, found 0` | 2 | `incomplete-obligation-coverage` |

**Predicted delta: one added `reason_code` key on 93 of 96 rows in each
sidecar — 186 rows in all — and nothing else.** The 3 `checked` rows in each
carry no `reason` and gain no key. This reproduces REASON-CODES §7's predicted
distribution (`unsupported-syntax` 91, `null` 3,
`incomplete-obligation-coverage` 2) by an independent count; the `null` 3 there
are the checked rows, which under this implementation carry **no key at all**
rather than a null one — a deliberate refinement of the prediction, recorded so
the re-pin is not surprised.

**The archived artifacts are not edited.** Moving them is a deliberate corpus
re-pin and requires re-running the pinned 185-unit route, which this card did
not do. `scripts/tests/test_sidecar_reason_inventory.py::test_the_archived_sidecars_do_not_yet_carry_the_field`
pins that they still lack the field and **fails the moment someone re-pins
them**, which is exactly when the prediction above must be replaced by a
measurement.

The test deliberately does **not** classify anything in Python — that would
create the second vocabulary this card exists to remove. It pins the reason
inventory and asserts every one of those strings is a row of
`crates/qcore-schemas/tests/reason_codes.rs`, where the Rust assertions fix
each string's category.

---

## 6. Lean

**Applicable for the reason half; not applicable for the generator half.**

*Generator half:* the subject is a JSON bookkeeping derivation over this
repository's own files. Its checkable properties — preservation, idempotence,
label totality — are facts about a Python script and a committed file, and are
checked as executable assertions over the real inputs (§2.4). Restating them in
Lean would prove them of a Lean-side re-encoding, and the encoding step is
exactly where a bookkeeping error would live. This is the same judgment
T00-CT-SCHEMA recorded for the same generator, for the same reason.

*Reason half:* the load-bearing claim is structural and was proved.
New module `proof/QCore/ReasonCarrier.lean`, audited by
`proof/AuditReasonCarrier.lean` — **15 theorems, no new axiom**
(`propext` and `Quot.sound` only):

* the text route **is** the unchanged classification relation (no second
  vocabulary), and the site route publishes one fixed category;
* fail-closed is preserved on the text route: an unrecognised reason publishes
  no category, and in particular no source-subject one;
* §L21.1 rule 1 across the site route: it never states anything about the
  analysed source, never supports a provisional `Unsafe`, and its subject is
  the environment;
* **non-widening**, in both halves of rule 1: neither the `null` being replaced
  nor the site-named category describes the analysed source or supports
  `Unsafe`;
* publication is single-valued (which is what licenses republishing a
  producer-supplied category instead of reclassifying it), and the two carriers
  agree wherever both decide the same fact;
* **non-vacuity in both directions** — the text route really does reach a
  source-subject category, and the site route really does publish one — plus
  totality and "only a refused text leaves a row uncoded".

Not proved, and stated as Rust boundaries in the module header: that the three
named opaque sites are the only ones on the route (a code census, §5.1), and
that no site names a source-subject category — which the Rust makes
*unconstructible* by publishing no such constructor, rather than by proof.

---

## 7. Gate results

Run on x86_64 Linux at this card's head commit (`5d307af`).

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1795 passed, 0 failed, 3 ignored** (1787 before) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **45 audits passed** (44 before) |
| `python3 -m unittest discover -s scripts/tests` | **125 tests, OK** (108 before, +17) |
| `cd providers && cargo test --workspace` | **67 passed, 0 failed** |

`git submodule update --init --recursive` was run once in this worktree. No
load-flaky test (P02 `RLIMIT_NPROC`, P03 confined-write, vp04) failed in this
run, so no isolated rerun was needed and no assertion was weakened.

**Provider packages were not rebuilt, and do not need to be.** `qcore-schemas`
is untouched at 7.0.0; `UnitRefusal` lives in `qcore-audit` and
`ChunkSourceResult` in `qcore-c-gateway`, neither of which a provider crate
depends on. The provider suite's package verification passes unchanged.

---

## 8. Files changed

| File | Change |
|---|---|
| `docs/reports/remaining-d02/build_matrix.py` | ownership model, carry-forward of hand-maintained sections, `--gate`/`--previous`, derived provider package versions, `section_ownership` output, five fail-closed refusals. CT derivation untouched. |
| `scripts/tests/test_build_matrix_sections.py` | **new** — 13 tests |
| `scripts/tests/test_sidecar_reason_inventory.py` | **new** — 4 tests |
| `crates/qcore-c-gateway/src/boolean_source.rs` | `ChunkSourceResult::Unknown::reason_code`, `::unknown`, `::reason_code()` |
| `crates/qcore-c-gateway/tests/chunk_source_reason_codes.rs` | **new** — 5 tests |
| `crates/qcore-audit/src/report.rs` | `UnitRefusal`, `reason_code_value`, `record_unknown` takes a refusal; 2 new unit tests |
| `crates/qcore-audit/src/lib.rs` | `verify_pinned_bytes` returns `UnitRefusal`; two opaque sites name their category |
| `crates/qcore-audit/src/current.rs` | the `fs::read` site names its category; the closure is typed |
| `crates/qcore-audit/src/source_audit.rs` | publishes the producer's category instead of reclassifying |
| `compiler/src/verifier_plugins/chunkio/chunk_source.rs` | facade mirrors and carries the field |
| `compiler/examples/support/cycle19_compatibility.rs` | both sidecars emit `reason_code` on `unknown` rows |
| `crates/qcore-verification/tests/{ctraces_capability_boundary,source_effect_obligations}.rs` | pattern updates only |
| `proof/QCore/ReasonCarrier.lean`, `proof/AuditReasonCarrier.lean`, `proof/QCore.lean` | **new** module + audit + registration |

Not touched, on purpose: `AGENTS.md`, `CLAUDE.md`, `README.md`,
`DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance_matrix.json`, `docs/reviews/r0_3/acceptance.json`
(no generated field required it), `crates/qcore-schemas/`, and every archived
185-unit artifact.

---

## 9. Open, named

* **REASON-CODES-B1-CHANNELS** — the 47 other `error.to_string()` conversions in
  `qcore-audit` sit on `Result<_, String>` channels ending in a whole-run
  `Err(String)`. None can produce an uncoded report row; typing them is a
  separate error-channel increment. `verify_pinned_bytes`'s two non-report
  callers narrow to the text today.
* **REASON-CODES-B3-REPIN** — the two adapter-assembled sidecars gain one
  `reason_code` key on 93 of 96 rows each. The archived copies are unchanged;
  the next deliberate corpus re-pin must carry the delta measured in §5.3 and
  replace the prediction with the measurement.
* **D02-GEN-REST-BOOTSTRAP** — the generator now refuses to run without a
  previous matrix rather than holding a second copy of the hand-maintained
  sections. Recovering from a deleted `acceptance_matrix.json` means
  `git checkout`, not regeneration. This is a deliberate trade: one copy that
  can go stale is the defect this card removed.
* **REASON-CODES-B1-CENSUS** — that the three named sites are the only opaque
  ones reaching a report row is a code census (§5.1), pinned by unit tests
  rather than by a mechanised reachability argument.
