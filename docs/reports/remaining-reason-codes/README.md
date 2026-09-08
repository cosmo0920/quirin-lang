# REASON-CODES — the closed L21.1 reason-code vocabulary for uncovered obligations

**Date:** 2026-09-08. **Base:** `cf2e5ee` (*Consolidate the wave-5 increment into
the status ledgers and CT accounting*). **Card type:** implementation, run
deliberately as a **pin-moving** card.

`docs/QCORE_SPEC_r0_3.md` §L21.1 specified a closed seven-category vocabulary
for classifying an `Unknown` row's free-text reason, and explicitly deferred
implementation: *"It is a **specification, not yet an implemented field**"*,
because either candidate carrier moves a digest-bearing artifact. D01 recorded
the same constraint and named the condition —
*"do not add a category field to `DerivationIncomplete` without regenerating
every task digest, provider package and witness in the same change"*
(`CLAUDE.md`, quoting `docs/reports/remaining-d01/README.md` §3).

This is that change. Both carriers land together, the provider packages are
rebuilt in the same commit, and every pin that moved is named below with its
old and new digest.

**Nothing is promoted.** No verdict changed on any unit or any function. Raw
Cycle 19 units remain 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units, CT coverage remains 20/50 mapped, the contract corpus remains
60 modules and full language conformance remains false. The canonical CMetrics
audit still reports one checked function contract, six Unknown functions and a
provisional Unsafe raw unit; the `chunkio-core` profile still reports
0 Safe / 2 provisional Unsafe / 11 Unknown across 13 units.

---

## 1. The vocabulary

`crates/qcore-schemas/src/reason.rs` publishes the seven categories of §L21.1
with their spec kebab-case spellings, plus a three-way `ReasonSubject`
partition that is the whole point of the vocabulary: it separates a statement
about the **analysed source** from a statement about the **machine analysing
it**.

| Category | Subject | Means | May support a provisional `Unsafe` |
|---|---|---|---|
| `unsupported-syntax` | analysed source | a construct outside the modeled grammar | yes |
| `residual-custody` | analysed source | the model ran and left an obligation open | yes |
| `missing-premise` | analyser | a declared assumption or trusted input was refused or unusable | no |
| `incomplete-obligation-coverage` | analyser | no derivation, or only a partial one, in this increment | no |
| `stale-evidence` | analyser | evidence bound to a different identity than the subject | no |
| `provider-failure` | analyser | the selected provider was absent, failed, or failed attribution | no |
| `environment-unavailable` | **environment** | a required tool, build tree or configuration input was missing or mismatched | **no** |

§L21.1 rule 1 — an environment failure must never be relabeled a source defect
— holds **by construction, not by review**: `ReasonSubject::Environment` has
exactly one inhabitant, `subject` is a total function on a closed enum, and
`describes_analysed_source` is defined from `subject`. There is deliberately
**no `Default`** on `ReasonCategory`; a defaulted category is exactly the silent
relabeling the rule forbids.

`ReasonCategory::classify` is fail-closed: a string no rule recognises is
**refused** (`Err(UnclassifiedReason)`). There is no fallback category, and in
particular no fallback onto a subject-`AnalysedSource` category.

### 1.1 Where the spec and D01 disagree — they do not

D01 §3 and §L21.1 give the same seven categories, the same two binding rules
and the same deferral reason. The one wording difference is that D01 says
implementation "is neither [additive nor low-risk]" while §L21.1 says it
"belongs to its own increment". Both are satisfied by this card being that
increment. No improvisation was needed and no disagreement is recorded.

---

## 2. The string mapping

The mapping was built from the emitting code, not sampled. Every family below
is exercised by `crates/qcore-schemas/tests/reason_codes.rs::reason_corpus`,
which enumerates 107 concrete reason strings this workspace can produce.

| Emitter | Reason family (marker) | Category |
|---|---|---|
| `qcore-verification::obligation_derivation` | `no core derivation for this capability …` | `incomplete-obligation-coverage` |
| " | `per-allocation-site custody derivation runs in the unrestricted-C gateway …` | `incomplete-obligation-coverage` |
| " | `source lifecycle effect/ABI import is unavailable …` | `incomplete-obligation-coverage` |
| " | `… the evidence-v2.0 order bound covers a single analyzed function` | `incomplete-obligation-coverage` |
| " | `caller refuses the observations_trusted assumption …` | `missing-premise` |
| " | `required observation hook … has no `extern fn` contract …` | `missing-premise` |
| " | ``contract module `…` does not parse`` | `missing-premise` |
| " | ``ambiguous observation contract `…` `` | `missing-premise` |
| " | `… contains structured control flow; the evidence-v2.0 schedule bound …` | `unsupported-syntax` |
| " | `… has operations after a return` | `unsupported-syntax` |
| " | `… has a statement whose call structure could not be recovered` | `unsupported-syntax` |
| " | ``… has unknown call effect `…` `` | `unsupported-syntax` |
| " | ``… has unsupported model-call arity for `…` `` | `unsupported-syntax` |
| " | `… shadows a lifecycle identity` | `unsupported-syntax` |
| " | the eight `unresolved` details (`… is not bound to a name`, `… does not name a created/an opened …`) | `unsupported-syntax` |
| `qcore-c-gateway::legacy_c` | `resource projection unresolved: N unsupported operation(s), …` with `N > 0` | `unsupported-syntax` |
| " | the same string with `N = 0` | `residual-custody` |
| " | ``could not project body for C function `…` at input line …`` | `unsupported-syntax` |
| " | `production C parser rejected the translation unit: …` | `unsupported-syntax` |
| " | `production C parser is unavailable: …` | **`environment-unavailable`** |
| " | ``C function `…` was not found`` | `incomplete-obligation-coverage` |
| " | `translation unit input identity cannot answer a filename-sensitive construct: …` | `incomplete-obligation-coverage` |
| `qcore-c-gateway::typed_c_ast` | `Clang AST unavailable/failed`, `Clang stdin unavailable`, `Clang input writer panicked`, `Clang input failed`, `unsupported Clang AST JSON` | **`environment-unavailable`** |
| " | `Clang AST rejected source: …` | `unsupported-syntax` |
| " | `expected one selected definition after preprocessing, found N` | `incomplete-obligation-coverage` |
| `qcore-c-gateway::boolean_source` | `outside single borrowed chunk / int-or-void-result source profile` | `unsupported-syntax` |
| " | `Clang identity changed during Boolean source import` | **`environment-unavailable`** |
| " | `ChunkIO source revision does not match the pinned revision`, `chunk helper source revision mismatch`, `helper source/build/profile selection mismatch` | `stale-evidence` |
| " | `source expression/statement exceeds the import depth limit`, `duplicate helper identity` | `incomplete-obligation-coverage` |
| " | the 12 `helper …` record/field/assignment refusals, the 4 `expected one …` record refusals, `field assignment leaves the Boolean lifecycle domain`, `ambiguous Boolean field models …`, `unsupported Boolean field operation mapping` / `result ABI`, `function has no modeled chunk field access`, `source path falls through without an explicit return`, `lifecycle field must be an ordinary non-volatile int …` | `unsupported-syntax` |
| " | `helper signature or parameter binding mismatch` | `unsupported-syntax` |
| " | `unmodeled statement …` / `unmodeled expression …` | `unsupported-syntax` |
| `qcore-audit::compile_db` | all 21 compile-command refusals (dialect, driver, quoting, options, `-arch`, `-std=`, database ambiguity, …) | **`environment-unavailable`** |
| `qcore-audit::{lib,current}` | `source outside selected checkout` / `source outside current checkout` | **`environment-unavailable`** |
| " | ``selected source `…` does not match its pinned bytes`` | `stale-evidence` |
| " | `source is not UTF-8` | `unsupported-syntax` |
| `qcore-audit::source_audit` | `raw source definition absent from the selected typed import; possibly build-excluded` | `incomplete-obligation-coverage` |
| `qcore-audit::source_contract` | `source-contract configuration: …` | `provider-failure` |
| `cycle19_compatibility.rs` | `… requires evidence unavailable in current-revision resource projection` | `stale-evidence` |
| " | `historical source/native/provider evidence is not admitted …` | `stale-evidence` |

### 2.1 Two judgment calls, recorded

1. **`contract module … does not parse` and `ambiguous observation contract …`
   are `missing-premise`, not `unsupported-syntax`.** The unparseable or
   ambiguous text is a *trusted contract input* of the task — it lives in
   `contract_sources` and is digest-bound there — not the analysed source. The
   choice matters because the two candidates sit on opposite sides of the
   partition: `missing-premise` speaks about the analyser,
   `unsupported-syntax` about the analysed program. Calling it
   `unsupported-syntax` would attribute a defect in *our* contract corpus to
   the audited program.
2. **`structured control flow` is `unsupported-syntax` but `lifecycle call
   sites appear in N functions` is `incomplete-obligation-coverage`.** The
   first is triggered by a construct in the input; the second by how the
   analysis's own single-function bound meets a distribution of call sites. The
   trigger, not the prose, decides.

### 2.2 A gap the archived artifacts found

An earlier draft of the corpus was enumerated from the reason sites named in
D01 §3. Predicting the reason codes of the four archived 185-unit artifacts
(§7) surfaced two rows the draft did not cover:
`expected one selected definition after preprocessing, found 0`, minted in
`qcore-c-gateway::typed_c_ast`. Following it back produced the whole
`typed_c_ast` / `boolean_source` refusal family — 37 further strings — which
is now enumerated in the table and the corpus. This is why §7's prediction was
computed against real artifact bytes rather than asserted.

### 2.3 The ordering hazard this table exists for

Four `environment-unavailable` reasons contain the word "unsupported":

```
unsupported compilation dialect `c++17`
empty or unexpanded compiler option is unsupported
sysroot-relative include paths are unsupported
shell expansion in compile command is unsupported
```

A naive marker table with an `"unsupported"` rule classifies all four as
`unsupported-syntax` — i.e. reports a broken *compile command* as a defect in
the *audited program*. The table therefore matches every environment,
identity, premise and provider rule **before** any source-subject rule, and
`the_word_unsupported_in_a_compile_command_never_becomes_a_source_statement`
pins exactly that. An early draft of this table contained the naive rule; it
was removed after this hazard was measured, not after review.

For the same reason the twelve `helper …` refusals are enumerated one by one
rather than caught by a `"helper"` prefix rule: three strings beginning
`helper` are `stale-evidence` or `incomplete-obligation-coverage`, and a
prefix catch-all onto a source-subject category would have swallowed them.

---

## 3. Carrier 1 — `DerivationIncomplete::category`

`crates/qcore-schemas/src/obligation.rs`:

```rust
pub struct DerivationIncomplete {
    pub capability: VerifierCapability,
    pub category: ReasonCategory,   // required; no serde default
    pub reason: String,
}
```

The field is **required** on a `deny_unknown_fields` record. That is the
carrier's whole point: a record written before this change is refused now, and
a record written now is refused by the previous shape. Since
`DerivationIncomplete` is an element of `VerificationTask`, whose SHA-256 over
canonical JSON travels in every evidence-v2 request and every witness, a
witness recorded against a task with a non-empty marker list cannot replay
against a task derived after the change, in either direction.

**Every emission site names its category at construction.** Production never
recovers a category from prose. The sites are the seven
`task.derivation_incomplete.push` calls in
`crates/qcore-verification/src/obligation_derivation.rs` plus the nine
`LifecycleScan` gap sites, which now record `(ReasonCategory, String)` pairs
through two named helpers (`unsupported_syntax`, `incomplete_coverage`) instead
of bare strings.

`classify` and the typed sites are held in agreement by
`every_emitted_marker_agrees_with_the_documented_reason_mapping`, which runs
**live derivations** over 15 crafted fixtures, collects every marker actually
emitted, and requires `classify(reason) == marker.category` for each. The table
cannot drift from the typed carriers unnoticed.

---

## 4. Carrier 2 — the audit report's per-unit `reason_code`

`crates/qcore-audit/src/report.rs` adds a `reason_code` sibling to the existing
`reason`, on `units[]` (all three constructors) and on
`source_contracts.units[].functions[]` (all three constructors). The envelope
gains one self-describing key, `reason_code_meaning`.

This carrier uses `classify` rather than typed sites, and that is a considered
choice, not a shortcut: the two highest-frequency strings in every checked-in
artifact are minted in `qcore-c-gateway`, not in `qcore-audit`, and the reasons
reaching `record_unknown` traverse `Result<_, String>` chains through
`compile_db`, `lib` and `current`. Typing them at the site would have required
changing reason *text* or rewriting those error channels — the first violates
§L21.1 rule 2, the second is out of scope for this card.

**Fail-closed shape.** `reason_code` is `null` when the unit has no reason, and
also `null` when the closed vocabulary does not recognise the reason. `null`
asserts nothing, so an unrecognised row can never be read as a statement about
the analysed source. It is never defaulted to a category.

**Recorded boundary (REASON-CODES-B1).** A small family of reasons is opaque
raw OS or tool text — `std::io::Error::to_string()` from `fs::read` in
`current.rs`, and the `io::Error`/`Command` errors in `verify_pinned_bytes`.
These carry no stable marker, so they classify to nothing and the report writes
`null`. That is honest (no claim) but incomplete (no code). Closing it means
carrying a category through those `Result<_, String>` channels, which is a
separate increment; it is pinned as-is by
`an_unrecognised_reason_asserts_no_category_rather_than_a_source_defect`.

**A third reason site is deliberately not a carrier.** §L21.1 also names
`ChunkSourceResult::Unknown { reason }` in
`qcore-c-gateway::boolean_source`. Its reasons *are* classified — they reach the
report through carrier 2 and appear as `reason_code` on the function rows — but
the `ChunkSourceResult` enum itself is unchanged. Adding a typed field there is
follow-on **REASON-CODES-B2**.

---

## 5. Deliberately moved pins

### 5.1 `qcore-schemas` 5.0.0 → 6.0.0

Under the crate's documented policy
(`crates/qcore-schemas/README.md` §"Compatibility policy") this is a major
version on **two independent grounds**:

* *Source incompatibility.* `DerivationIncomplete` is a plain struct, so every
  exhaustive construction stops compiling — the same precedent as 2.0.0, 4.0.0
  and 5.0.0.
* *A tightened validation rule.* `category` has no serde default on a
  `deny_unknown_fields` record, so previously valid data is now rejected.

Moved with it: eight `Cargo.toml` pins (`qcore-c-gateway`, `qcore-verification`,
`qcore-audit`, `qcore-verifier-host`, `qcore-contract`, `compiler`,
`compiler/tests/fixtures/w3/provider`, `providers`) and three lockfiles.

**No other version moved.** Plugin protocol v1, evidence versions 1 and 2,
program schema/profile v2, the reactor proposal and program versions, the
Boolean-field profile versions and every provider package version are
unchanged.

### 5.2 `providers/Cargo.lock` — including a pre-existing breakage

Beyond the schemas bump, this lockfile also corrects two entries that were
**already stale at `cf2e5ee`**: `qcore-verification` and `qcore-verifier-host`
were bumped to 0.2.0 by P09 (wave 3) without updating this lockfile. Because
`providers/package.py build` passes `--locked`, **the packaging gate already
failed on the merged wave-5 tree**. That was reproduced from a clean extract of
`cf2e5ee` before being fixed:

```
error: cannot update the lock file …/providers/Cargo.lock because --locked was
passed to prevent this
```

This is recorded as a finding, not silently absorbed. It also means the
"before" package build below required patching only those two unrelated lines,
so the before/after package comparison isolates this card's change.

### 5.3 Provider packages — all five rebuilt

`python3 -B providers/package.py build` was run from a clean extract of
`cf2e5ee` (with only §5.2's two lines patched) and from this card's tree. Same
`rustc 1.98.0 (88d9e12ae)` both times.

| Package | field | before | after |
|---|---|---|---|
| `qcore.chunkio` | `executable_sha256` | `5f38cf90e09b…` | `743cd2c4064c…` |
| | `manifest_sha256` | `f3b0f6500023…` | `64f45dbe672f…` |
| | `archive_sha256` | `0746165d9350…` | `59e16e8f50e3…` |
| `qcore.cmetrics` | `executable_sha256` | `5f77c31d853e…` | `ba3110b61142…` |
| | `manifest_sha256` | `59bf26771c1e…` | `da6069bd2385…` |
| | `archive_sha256` | `662270c2f4c4…` | `5074bbcb01ee…` |
| `qcore.ctraces` | `executable_sha256` | `53effe7e0e7b…` | `487cf30e89c3…` |
| | `manifest_sha256` | `8b90dd7f92c4…` | `0f7946dea9ad…` |
| | `archive_sha256` | `2432c8b57819…` | `33c6d432c577…` |
| `qcore.libevent` | `executable_sha256` | `5ed8b5bd97f4…` | `df41b61e9019…` |
| | `manifest_sha256` | `3fa60ace05e1…` | `6fd3a3e8efc9…` |
| | `archive_sha256` | `51c93a6bac9f…` | `c8c3b68a40bd…` |
| `qcore.librdkafka` | `executable_sha256` | `40efbb05fc5f…` | `1d3bebed644d…` |
| | `manifest_sha256` | `e7021f1cb11c…` | `f3abe6f55f0d…` |
| | `archive_sha256` | `d9ff62fd0987…` | `6e9e5ca9086e…` |

`lock_sha256` moves `c15d6d27…` → `d2389f0f…`; `rustc` is byte-identical.
Full values are in `measurements.json`.

**Untouched-field parity.** Every field of every `verifier-plugin.json` is
byte-identical across the rebuild **except `package_digest`** — plugin id,
version, capabilities, evidence versions, trust class, entrypoint, limits and
targets are unchanged for all five packages. `plugin_id` is unchanged in
`packages.json`. All five rebuilt packages pass `providers/package.py verify`,
which re-executes each binary's `--verify-package` and compares its self-report
to the manifest.

No provider artifact crate references `obligation::` at all, so the digests
moved only because `qcore-schemas` recompiled into each binary. That was
measured rather than assumed.

### 5.4 The digest claim the spec deferred on, measured

§L21.1 and D01 both state that adding the field "changes **every** task
digest". **That is stronger than what the code does, and the difference was
measured.**

`DerivationIncomplete` is the *element* type of `Vec<DerivationIncomplete>`. A
task with `derivation_incomplete: []` serializes to `"derivation_incomplete":[]`
regardless of the element's shape, so its digest is byte-identical.
`an_empty_marker_list_keeps_its_pre_6_0_0_bytes_and_digest` pins this and
asserts the serialization does not mention `category` at all.

Every checked-in artifact in this repository that carries a `task_digest` also
carries `"derivation_incomplete": []`, and **no test pins a literal task or
witness digest** — all are recomputed live. So the set of task digests that
move because of the category field is, in this repository, empty. Digests move
only for tasks that populate markers, and those routes have no pinned
artifacts. The three-way attribution in §6.1 confirms this on the real
CMetrics run.

The refusal is still real and still pinned in both directions:
`a_marker_record_recorded_before_the_category_field_is_refused`,
`a_marker_record_written_now_is_refused_by_the_previous_shape`, and
`the_category_is_bound_into_the_task_digest_in_both_directions` (relabeling a
marker moves the digest, so a witness cannot cross the relabeling).

---

## 6. Regenerated audits

Three audits are runnable in this environment. Artifacts are archived under
`docs/reports/remaining-reason-codes/audit/`, following the project convention
that each card archives its own copy rather than rewriting another card's
evidence. `measurements.json` carries every digest.

| Audit | Result | before SHA-256 | after SHA-256 |
|---|---|---|---|
| canonical CMetrics (`contract/profiles/cmetrics-counter`) | 1 unit, 0 Safe / 1 provisional Unsafe; 1 checked function contract / 6 Unknown | `61c5637c8218d156…` | `648d755c0f8d7dca…` |
| `chunkio-core` (`contract/profiles/chunkio-core`) | 13 units, 0 Safe / 2 provisional Unsafe / 11 Unknown | `2b3a48a15ebc623a…` | `a686fb6f9acb9c2f…` |
| synthetic demo (`docs/reports/w5-configured-audit/create_demo.py`) | 3 units, 1 Safe / 1 provisional Unsafe / 1 Unknown | `154eaa5eb7182cd3…` (archived) | `7543a958004ddd55…` |

Every verdict, count, diagnostic and reason string is unchanged.

### 6.1 Attribution — three runs, not two

Naively diffing the CMetrics artifact showed 17 moved pointers including
`task_digest` and `obligation_id`, which would have contradicted §5.4. It did
not: `SourceInputIdentity::configuration_digest` binds
`std::env::current_dir()`, and the base-commit run had been executed from the
extracted tree's directory. Re-running it from the *same* working directory
gives a clean three-way attribution:

**A** = base commit + base packages, **B** = this card + base packages,
**C** = this card + rebuilt packages, all over identical prepared inputs.

* **A vs B (source change alone): exactly 8 added keys, nothing moved.**
  `$/reason_code_meaning`, `$/units/0/reason_code` (`null`), and six
  `$/source_contracts/units/0/functions/N/reason_code` (all
  `unsupported-syntax`). `task_digest`, `obligation_id`, `build_digest`,
  `configuration_digest` and every `contract_sources` entry are unchanged —
  because this task's `derivation_incomplete` is `[]`.
* **B vs C (package rebuild alone): exactly 2 moved pointers**, both
  `package_digest` (`5f77c31d…` → `ba3110b6…`) — the configuration's copy and
  the certificate's `evidence/reproducibility/package_digest`.

For `chunkio-core`, before and after are identical once `reason_code_meaning`
and the 13 `reason_code` keys are removed. The `chunkio-core` artifact also
differs from `docs/reports/remaining-g03/audit/chunkio-core-audit.json.gz` in
`diagnostic_counts` and `units`; that drift is present in the base-commit run
too and predates this card (gateway changes since G03, C02 among them). It is
recorded, not repaired.

### 6.2 Observed distribution

| Audit | `reason_code` histogram |
|---|---|
| CMetrics units | `null`: 1 (the provisional Unsafe unit has no reason) |
| CMetrics source-contract functions | `unsupported-syntax`: 6 |
| chunkio-core units | `unsupported-syntax`: 11, `null`: 2 (both provisional Unsafe) |
| synthetic demo units | `unsupported-syntax`: 1, `null`: 2 (Safe and Unsafe) |

Every classified row in all three audits is `unsupported-syntax` — an honest
result: these are prepared, working builds, so no environment row arises. The
environment path is exercised by the unit and schema tests instead, including
the real `audit requires the configured Clang executable …` text.

### 6.3 A stale prepared package, refused

The prepared `build/cycle19/provider-packages-unit` in the developer checkout
(built 2026-09-07) predates the embedded manifest-template requirement and was
refused:

```
audit rejected: source-contract package: …/qcore.cmetrics: executable carries
no `<<<qcore.provider-template.v1:begin>>>` manifest template envelope; its
declarations cannot be bound without executing it
```

That is a fail-closed refusal working correctly, and it predates this card. The
audits above use the packages rebuilt in §5.3.

---

## 7. The four pinned 185-unit artifacts — NOT this card

`docs/reports/w5-cmetrics-unit-provider/{source,diagnostics,chunk-contracts,source-effects}.json.gz`
and the 24 re-archived copies are **not touched by this card**, and no 185-unit
re-measurement was attempted: the prepared Fluent Bit corpus for that run is
not available here, and its pin gate would refuse in any case. They move at
**CORPUS-REPIN**, which runs after this card and must carry the new reason
codes.

**Handoff to CORPUS-REPIN.** The distribution below was *computed* by applying
this card's classification rule to the archived bytes, read-only
(`predict_185` in the card's working notes). It is a prediction of what the
re-pin will contain, not a claim that the artifacts were regenerated.

| Artifact | Carrier | Predicted distribution |
|---|---|---|
| `diagnostics.json` | `units[].reason_code` (185 units) | `unsupported-syntax` 136, `null` 48, `incomplete-obligation-coverage` 1 |
| `source-effects.json` | `results[].reason_code` (96 rows) | `unsupported-syntax` 91, `null` 3, `incomplete-obligation-coverage` 2 |
| `chunk-contracts.json` | `results[].reason` — **third carrier, not implemented** (96 rows) | would be `unsupported-syntax` 91, `null` 3, `incomplete-obligation-coverage` 2 |
| `source.json` | none — no `reason` field | unchanged |

Notes the re-pin must carry:

1. `diagnostics.json` also gains `reason_code_meaning` in its envelope. The one
   `incomplete-obligation-coverage` unit is
   ``C function `_cnd_timedwait_win32` was not found``. No unit in the archived
   bytes has a zero unsupported count, so `residual-custody` does not appear —
   but the rule is per-row, so a re-measurement may produce it.
2. `source-effects.json` is the only one of the four carrying `task_digest`
   values, and all three of them sit on tasks with
   `"derivation_incomplete": []`. Those digests therefore move **only** if the
   provider package the route selects is rebuilt (§6.1 measured that
   dependency exactly), not because of the category field.
3. `chunk-contracts.json`'s `results[].reason` is a **third** reason vocabulary
   that this card did not give a carrier — follow-on REASON-CODES-B3. Its rows
   classify cleanly under the same table, so implementing it is a small
   addition, but it belongs with whichever card owns that report shape.
4. `source.json` has no `reason` field at any level and is unaffected.
5. The re-pin must be deliberate and stated, and any newly surfaced provisional
   Unsafe unit recorded, per the standing CORPUS-REPIN note that already names
   C02, G02 and O04.

---

## 8. Lean

`proof/QCore/ReasonCategory.lean` (audited by `proof/AuditReasonCategory.lean`)
models the classification relation. The gate moves from 37 to **38 audits**;
`AuditAll.lean`'s closure check passes with no new axiom — only `propext`,
`Classical.choice` and `Quot.sound`.

The judgment was **not** vacuity: the relation has real content on two axes.

*Structure of the vocabulary and its partition (10 theorems).*
`all_is_complete`, `all_has_no_duplicates`, `spellings_are_pairwise_distinct`,
`Family.all_is_complete`,
`environment_subject_is_inhabited_only_by_environment_unavailable` (an
**iff**, so the environment category is neither under- nor over-applied),
`no_category_is_both_environment_and_source`,
`environment_unavailable_says_nothing_about_the_source`,
`an_environment_row_never_supports_unsafe`, `every_subject_is_reached`,
`exactly_two_categories_describe_the_analysed_source`.

*The mapping over the 33 enumerated real reason families (8 theorems).*
`the_mapping_is_total`, `every_category_is_reached` (non-vacuity: no arm of the
vocabulary is decorative), `classification_is_single_valued`,
`every_known_input_classifies`,
`an_environment_family_never_becomes_a_source_statement`,
`only_an_environment_family_is_labeled_environment`,
`an_environment_family_never_supports_unsafe`, and the composite
`a_source_statement_has_a_recognised_non_environment_origin` — if a
classification result describes the analysed source, its input was a
recognised, non-environment family.

*Fail-closed refusal (2 theorems).* `an_unrecognised_reason_is_refused`,
`refusal_never_yields_a_source_statement`.

**Explicit boundary.** The `Family` enumeration is a *transcription* of the
Rust corpus, not a mechanical extraction from it. That the Rust marker table
recognises exactly these families, and that each production site names the
category this relation assigns, are Rust boundaries — pinned by
`crates/qcore-schemas/tests/reason_codes.rs` and
`crates/qcore-verification/tests/unit/obligation_derivation.rs`.
`native_decide` is deliberately avoided; it would introduce
`Lean.ofReduceBool`, which `AuditAll.lean` forbids.

---

## 9. Tests

**`crates/qcore-schemas/src/reason.rs`** (5 unit tests) — the environment
subject is a singleton; no category is both environment and source; spellings
round-trip and reject unknown text; classification refuses rather than
defaulting; the derived predicates agree with `subject` on all seven and the
three subjects are exactly covered.

**`crates/qcore-schemas/tests/reason_codes.rs`** (10 tests) —
`the_documented_mapping_is_total_over_every_reason_this_workspace_emits` over
the 107-string corpus enumerated from the code;
`the_corpus_exercises_every_category_so_the_mapping_is_not_vacuous`;
`no_environment_reason_is_readable_as_a_statement_about_the_analysed_source`;
`the_word_unsupported_in_a_compile_command_never_becomes_a_source_statement`;
`an_unrecognised_reason_is_refused_and_never_defaulted`;
`an_empty_marker_list_keeps_its_pre_6_0_0_bytes_and_digest`; both digest-refusal
directions; `the_category_is_bound_into_the_task_digest_in_both_directions`;
`the_category_refines_the_row_and_replaces_nothing`.

**`crates/qcore-verification/tests/unit/obligation_derivation.rs`** (4 tests,
driven by live derivations over 15 fixtures) —
`every_emitted_marker_agrees_with_the_documented_reason_mapping`;
`derivation_never_labels_a_gap_an_environment_failure`;
`the_derivation_category_corpus_reaches_both_halves_of_the_partition`;
`relabeling_a_marker_retires_the_task_in_both_directions`.

**`crates/qcore-audit/src/report.rs`** (6 tests) — a unit without a reason
asserts no category; an unrecognised reason asserts no category rather than a
source defect; a missing Clang is recorded as an environment failure that
cannot support `Unsafe`; the projection summary separates unmodeled constructs
from open custody; the category refines the unit and replaces nothing (exact
nine-key inventory); the envelope explains the vocabulary it carries.

The existing substring assertions in `configured_cli.rs`,
`second_source_set.rs` and `report.rs` were **not** weakened — they still pass
against unchanged reason text.

---

## 10. Gate

Run in this worktree at the final commit, on x86_64 Linux:

| Gate | Result |
|---|---|
| `cargo test --workspace` | 1650 passed, 0 failed, 1 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | 38 audits passed |
| `cd providers && cargo test --workspace` | 58 passed, 0 failed |
| `providers/package.py build` + `verify` × 5 | all 5 built and verified |
| `python3 -m unittest discover -s scripts/tests` | 95 tests, OK |
| `python3 -B docs/reports/remaining-reason-codes/predict_185.py` | 0 unclassified rows across all four archived artifacts |
| canonical CMetrics audit | 1 checked contract / 6 Unknown, provisional Unsafe |
| `chunkio-core` audit | 0 Safe / 2 provisional Unsafe / 11 Unknown |
| synthetic demo audit | 1 Safe / 1 provisional Unsafe / 1 Unknown |

The three known load-flaky tests (P02 `RLIMIT_NPROC`, P03 confined-write,
vp04 process-lifecycle) passed on every run in this session and needed no
isolated rerun; no assertion was weakened.

---

## 11. Open, named

* **REASON-CODES-B1** — opaque OS/tool reason text (`io::Error`,
  `Command` failures in `current.rs` and `verify_pinned_bytes`) classifies to
  nothing and writes `reason_code: null`. Closing it means threading a category
  through those `Result<_, String>` channels without changing reason text.
* **REASON-CODES-B2** — `ChunkSourceResult::Unknown { reason }` keeps its
  free-text-only shape. Its reasons are classified on the report, but the enum
  is not a typed carrier.
* **REASON-CODES-B3** — `chunk-contracts.json`'s `/results/N/reason` is a third
  reason vocabulary with no carrier.
* **CORPUS-REPIN** — the four 185-unit artifacts, per §7.
* **P09-lock** — `providers/Cargo.lock` staleness (§5.2) is fixed here, but no
  gate prevents it recurring: nothing in CI runs `providers/package.py build`.
* Spec §L21.1's prose still says the vocabulary is "not yet an implemented
  field" and gives the deferral rationale. Updating it is an integrator edit
  and was not made here; §5.4 records that its digest claim is stronger than
  the measured behaviour.
