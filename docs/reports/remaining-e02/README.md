# E02 — Mutation matrix for source-bound witness replay and cached summaries

2026-09-07. Baseline `ffb768a` (the E01 merge). Workstream W3,
`verify_or_extend`.

This unit is the systematic completion of what E01 and G01 spot-checked for one
admitted capability: **every way a replay input, an evidence set, a witness or a
cached summary can be mutated, and the observed outcome of each**. It promotes
nothing, changes no production code, and does not bump any version number.
Machine-readable results are in [`matrix.json`](matrix.json).

## Capability under audit

| Field | Value |
|---|---|
| Capability | ChunkIO Boolean-field source contract |
| `VerifierCapability` | `ResourceContracts` |
| Obligation property | `BooleanFieldContract` |
| Semantic profile | `qcore.c_boolean_field_contract.v2` |
| Source importer | `crates/qcore-c-gateway/src/boolean_source.rs` |
| Task derivation / witness checker | `crates/qcore-verification/src/source_effect.rs` |
| Firewall | `crates/qcore-verifier-host/src/lib.rs` (`verify_v2`, `check_evidence_v2`) |
| Composition | `compose_verification_with_inventory` |

New evidence: `crates/qcore-verification/tests/source_effect_mutations.rs`
(11 tests). Rows already pinned by E01
(`crates/qcore-verification/tests/source_effect_obligations.rs`), G01
(`crates/qcore-audit/tests/compile_task_identity.rs`,
`crates/qcore-audit/tests/configured_cli.rs`) or the pre-existing replay tests
(`crates/qcore-verification/tests/source_effect.rs`) are **cited** in the matrix
rather than duplicated.

## Result

**52 rows, 52 pass, 0 repairs, 0 accepted mutations.** No mutation in the matrix
was admitted, so no repair was required and no gate was touched. 42 rows are
newly pinned here; 10 cite existing tests.

Every "reject" in the matrix means the proposal is refused and the obligation
**remains `Unknown`**. Nothing in this card promotes a raw Cycle 19 unit, changes
a verdict, or alters CT coverage.

## 1. Reject on changed inputs (rows 1.1 – 1.24)

The identity chain, read out of the code, is:

```
BooleanSourceIdentity                       (raw bytes, effective preprocessing
  ├─ raw_source_digest                       input, dialect, include paths,
  ├─ preprocessed_source_digest              definitions, ordered typed options,
  ├─ configuration_digest ── analyzed        analyzed function scope, cwd,
  │                          function scope   compiler program/version/target)
  ├─ source_contract_digest, profile_digest, model_library, source_revision
  └─ helpers (recursive closure)
        │  sha256(serialize(identity))
        ▼
VerificationTask.unit.build_digest ─┐
VerificationTask.unit.source_digest ─┼─► task_digest() ─► obligation_id
VerificationTask.contract_sources ──┘                     (core-computed only)
```

Newly pinned here:

- **Import coverage.** The *analyzed function scope* selects which definitions
  the import covers. Two runs of the same source under
  `PreprocessedTranslationUnit` and `MainSourceDefinitions` produce a
  byte-identical schema record but different configuration digests, hence
  different tasks; each refuses the other's witness and still discharges its own
  (row 1.11). Widening the covered set with a definition the grammar refuses
  leaves the checked function's paths untouched and still retires its task,
  because the whole translation unit is the import's identity (row 1.12).
- **Checker and schema version fields.** `task_version`, the task's
  `evidence_version`, the obligation's `evidence_version`, the request's
  negotiated `evidence_version`, and the source projection's `schema_version`
  are each bound; a manifest that does not declare evidence v2 never reaches the
  firewall (rows 1.13 – 1.18).
- **Mismatched obligation identifiers**, the row E01 explicitly did not cover.
  Using two checked functions of one translation unit: a *foreign vector* with
  the right count (row 1.19), a *substituted identifier* (1.20), a *widened*
  two-obligation vector (1.21), its *permutation* (1.22), and the other
  function's real identifier in the coverage set (1.23). The first four reject
  with `QP001` at the request/task binding; the fifth with `QP009`. In rows 1.21
  and 1.22 the core checker is also called directly and refuses a task it did
  not derive, independently of the request frame.
- **Substituted source projection.** `verify_v2` is public, so a caller can hand
  it a request whose `source_effect` is a different import of the *same* symbol.
  `task.contract_sources` catches it (`QP001`, row 1.2); omitting the projection
  under a Boolean-field obligation is refused by `VerificationRequestV2::validate`
  (row 1.3).
- **Declared assumptions** at the evidence layer: added, removed and flipped
  entries each reject with `QP009`, with an honest control that certifies
  through the same envelope (row 1.9).

Cited, not repeated: raw source bytes and contract snapshots (1.1, 1.8), the
used-header/effective-preprocessing-input rows from G01 (1.4, 1.5), ordered
preprocessing options (1.6), compile-task identity components (1.7), witness
`profile_assumptions` (1.10), and E01's coverage omission/substitution/addition
(1.24).

## 2. Evidence-set mutations (rows 2.1 – 2.19)

### Duplication is not coverage

Evidence v2 carries exactly one `witness` per proposal, and `coverage` is a set
of core-computed identifiers. Duplication was probed at both levels, using two
genuinely checked functions of one translation unit:

| Shape | Outcome |
|---|---|
| honest, distinct certificates for both obligations (control) | `Safe` |
| one witness whose coverage claims **both** identifiers | `QP009` |
| the **same certificate submitted twice** at composition | `QP011`, "1 core-derived obligation(s) are not covered" |
| one certificate for a two-obligation inventory | `QP011` |
| function A's honest witness answered to function B's request | `QP009`, composition `Unknown` |
| `Safe` with no evidence at all | `QP008` |

The composition denominator is the union of certificate coverages compared to
the core's required set, so a repeated certificate contributes one identifier,
not two.

### Forged witness with a recomputed binding

`BooleanFieldContractWitness` has exactly two fields — `task_digest` and
`summary` — and `BooleanSourceIdentity` is `serde(skip)`, so it never travels in
the witness. "Recomputing inner hashes" can therefore only mean re-pointing the
outer digest at some real task. All three variants reject: another function's
real, well-formed paths spliced into this function's record and left bound to
its own task (2.7), the same record **rebound to the donor function's real task
digest** (2.8), and rebound to an invented digest (2.9) — each checked against
*both* tasks. The outermost binding plus the core's comparison against its own
sealed import is what catches them.

### Oversized and degenerate

- The schema's path vector is a fixed pair. Wire witnesses with 0, 1, 3 or 512
  paths fail deserialization rather than being truncated or padded (2.10); an
  unknown field fails under `deny_unknown_fields` (2.11).
- 200,000-element branch and write vectors plus 50,000 frame writes are refused
  by comparison with no panic and no unbounded work (2.12).
- Zero-length task digest, function name, source revision, a zero-length
  dependency entry, and cleared profile assumptions all reject (2.13). *Honest
  note:* the `SOURCE` fixture calls no frame helper, so its dependency map is
  already empty; the degenerate mutation there is an inserted zero-length entry,
  not a clear, and the test asserts that fixture shape so the row cannot become
  vacuous.
- A coverage set flooded with 5,000 invented identifiers rejects (2.14).
- Process providers are additionally bounded by `PluginLimits.output_bytes`
  before any of this; that limit is unchanged and not re-audited here.

### Unsupported evidence versions, and legacy v1 preservation

A proposal declaring evidence version 1 or 3 against a version-2 request rejects
with `QP009` in both directions (2.15), and a nondeterministic proposal rejects
whatever else it carries (2.16).

Protocol v1 is **preserved, not disabled** (2.17 – 2.19):

| v1 response | Outcome |
|---|---|
| `Unknown` with the provider's own diagnostic | `Unknown`; the diagnostic flows through verbatim; **no QP code is injected** and it is not an error |
| `Safe` with a complete, envelope-valid v1 evidence record | `QP009`, terminal refusal: "protocol v1 lacks core-derived obligations and capability-specific semantic witnesses" |
| `Unsafe` | `QP010` downgrade to `Unknown`, provider diagnostics retained |

The same test then dispatches the same task over v2 and gets `Safe`, so v1
preservation is not achieved by disabling the v2 route.

## 3. The core replays semantics, not labels (rows 3.1 – 3.4)

Two checked functions were built so that **both** path sets are well formed and
**both** satisfy the same contract:

```c
int read_lock(struct cio_chunk *ch) { return ch->lock; }
int peek_lock(struct cio_chunk *ch) { if (ch->lock) { return 1; } return 0; }
```

`peek_lock` certifies on its own (4 checked prestates, `Safe`), and its paths
genuinely differ from `read_lock`'s. Splicing `peek_lock`'s paths into
`read_lock`'s record — a **well-formed but semantically wrong** path set —
rejects with `QP009` through the full registry; the obligation remains in
`task.coverage_for(ResourceContracts)` afterwards and composition over the core
inventory stays `Unknown` (3.1).

A proposal that carries the assertion text (`normal result == old_lock; normal
lock == old_lock`), the Lean theorem family (`QCore.BooleanFieldContract`) and
the verdict label `Safe` in the record's declared assumption map, with the paths
emptied, also rejects with `QP009` (3.2). The honest witness through the same
registry certifies (3.4), so the refusals are attributable to the proposed paths
alone.

Acceptance terminates in `BooleanFieldSourceTask::evaluate`, which re-evaluates
the pinned contract over all four Boolean prestates of the core's own sealed
import. Exact paths under a falsified postcondition are still refused (3.3,
cited).

## 4. Cached summaries

**No summary or evidence cache exists on this route.** This was established
statically and then pinned behaviorally.

Static survey (`crates/qcore-audit/src`, `crates/qcore-verification/src`,
`crates/qcore-verifier-host/src`, `crates/qcore-c-gateway/src`, `compiler/src`),
searching for `cache`, `memo`, `OnceLock`, `OnceCell`, `Lazy`, `thread_local`,
`static mut`, `RwLock` and digest-keyed maps:

| Site | Finding |
|---|---|
| `qcore-c-gateway/src/lib.rs` | `CLANG_AVAILABILITY: OnceLock<Result<(), String>>` and a `Once` warning latch — an availability probe, not a result cache; it stores no analysis output |
| `CompilerIdentity::capture()` | called per import (four call sites in `boolean_source.rs`), and re-captured and compared after analysis; not memoized |
| `SourceContractAudit::analyze_unit` | re-runs `import_chunk_source_with_profile` per unit; no map from digest to prior result |
| `SourceContractRegistry::verify` | re-derives `BooleanFieldSourceTask` per summary; no reuse |
| `VerifierRegistry::verify_v2` / `check_evidence_v2` | pure functions of `(request, task, checker, response)`; no state |
| composition | recomputed from the outcomes passed in |

The one input that *is* prepared once per audit run is the imported frame-helper
set (`SourceContractAudit::helpers`, keyed by library). That is not a result
cache: each helper's full identity is embedded recursively in every importing
unit's `BooleanSourceIdentity`, so a changed helper changes every dependent task
digest. It is a within-run scoping decision, already recorded by
`w5-configured-frame-helpers`.

Behavioral pins (rows 4.1 – 4.3), so that introducing a digest-keyed cache that
ignores a semantic change would fail this test:

- Interleaved imports of two semantically distinct sources (X, Y, X, Y) return
  results that follow their argument, not the call order, and re-import of one
  input is deterministic.
- Deriving over a changed contract and back again reproduces the original task
  digest exactly, and the changed contract is a different task.
- One registry admits the honest proposal, then refuses the *same* witness
  against the retired task (`QP009`), then admits the original again — the
  firewall keeps no per-digest memory in either direction.

Recorded state: **no cache exists; this is the current (safe) state.**

## 5. Boundaries found and pinned, not repaired

- **E02-B1.** Admission is not evidence that the provider computed anything.
  The witness is compared to the core's own sealed import and acceptance
  terminates in the core's own contract evaluation, so a provider that merely
  echoes the correct record is admitted. This is by design — the provider is
  untrusted and contributes no authority — and is not a soundness gap. It does
  mean that **no `Safe` result on this route may be attributed to provider
  analysis**, which is the honest reading of the existing "3 checked function
  contracts" line.
- **E02-B2.** `BooleanFieldSourceTask::check` returns the *whole* coverage set
  of the task it validated. That is sound only because its own derivation emits
  exactly one obligation and it refuses every task it did not derive. Row 5.1
  pins both halves, so a future multi-obligation derivation for this capability
  cannot silently over-discharge; it would have to change `check` to attribute
  per obligation.
- **E02-B3.** The witness carries no import identity of its own. Two imports
  whose schema records are byte-identical have interchangeable witness
  *records*; only the outer core-computed task digest separates them. Rows
  2.7 – 2.9 pin that this outer binding is load-bearing.
- **E02-B4.** Every row is scoped to this capability. The CMetrics unit-result
  capability (`…v3`), chunk lifecycle and observation replay each need their own
  matrix; none is claimed here.
- **E02-B5.** E01-B1 is unchanged and still open: a definition visible only
  after macro expansion, and a K&R-style definition, are absent from the raw
  function enumeration and therefore from the derived inventory. No mutation row
  repairs it; it promotes nothing.

## 6. Standing gate

Run from this worktree at `ffb768a` + this change.

| Gate | Result | Exit |
|---|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | `third_party/libevent` at `d82464a2…` | 0 |
| `cargo test --workspace` | **861 passed, 0 failed, 0 ignored** (850 baseline + 11 new) | 0 |
| `cargo fmt --all -- --check` | clean | 0 |
| `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | — | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | — | 0 |
| `cargo test --manifest-path providers/Cargo.toml --locked --offline` | **46 passed, 0 failed** | 0 |
| `python3 -B -m unittest discover -s scripts/tests -p "test_*.py" -t scripts/tests` | **43 passed** | 0 |
| `cd proof && lake build` | 62 jobs, build completed | 0 |
| `python3 -B scripts/check_lean_audits.py` | **20 Lean audits passed** | 0 |
| `python3 -B providers/package.py build --output build/cycle19/provider-packages-unit` | five packages built | 0 |
| `python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit …/build/cycle19/fluent-bit-clang-source` | build + database persisted | 0 |
| `cargo run --quiet -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json` | **1 checked function contract / 6 Unknown**; 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only — unchanged | 0 |

Logs are under [`logs/`](logs/).

`--fluent-bit` selects the existing pinned, clean clone at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/fluent-bit-clang-source`,
because the sibling `/home/cosmo/Documents/GitHub/fluent-bit` checkout is off its
profile pin (recorded as finding F3 in `docs/reports/remaining-b01/README.md`)
and the script's default `--fluent-bit` resolves to a path that does not exist
for a git worktree. The developer's repository was not touched. The provider
package build must precede the audit, or the audit rejects fail-closed on the
missing `qcore.cmetrics` package.

## 7. Artifact parity

This card adds one integration-test file and this report directory. **No
production source file is touched**, so the pinned 185-unit Cycle 19 artifacts,
the contract corpus counts (59 modules), the CT manifest (11/50 mapped) and the
`third_party/libevent` submodule pin cannot change and were not re-measured. The
CMetrics audit *was* re-run end to end and is unchanged.

## 8. Follow-on units (named, no work done here)

- **E02-F1** — the same matrix for the CMetrics unit-result capability
  (`qcore.c_boolean_field_contract.v3`): distinct result ABI, two prestates,
  generic field predicates, and the `certificate_checker` gate that v3 adds in
  `SourceContractRegistry::verify`.
- **E02-F2** — the same matrix for `ChunkLifecycleReplay` and
  `ContractObservationReplay`, whose checkers do *not* return a whole task
  coverage set and therefore need per-obligation attribution rows that this
  capability cannot exercise.
- **E02-F3** — an out-of-process transport variant of rows 2.10 – 2.14, driving
  the oversized and degenerate payloads through a real process provider so that
  `PluginLimits.output_bytes` and the stdin/exit handling are exercised
  alongside the schema bounds.
