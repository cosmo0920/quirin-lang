# P04-COMPOSE-FRAME — a verification task for the Boolean frame helper

Workstream W5 (Providers), follow-on named by
[`docs/reports/remaining-p04-compose/README.md`](../remaining-p04-compose/README.md) §1
and §9.

That card's census recorded the structural blocker precisely: `ChunkFrameSummary`
has an importer (`qcore_c_gateway::boolean_source::import_chunk_frame_helper*`)
and a contract evaluator (`qcore_contract::chunk_contract`) and **no verification
task at all**. `PrerequisiteIdentity` is a lookup key whose three fields —
semantic profile, task digest, obligation identifier — a dependent's core
re-derives from its own import. A capability with no task has none of the three,
so nothing could name the frame helper, and `memory_close`,
`filesystem_chunk_close` and `unsupported_close` stayed blocked behind it.

This card supplies the missing identity. It does **not** move `memory_close`
through the process boundary; §4 records why, by measurement.

- Worktree cut from `main` at **`009bde0`** (wave-13 consolidation); branch
  `worktree-agent-a6edde17105db5abe`.
- Libevent submodule initialised at the pinned revision.
- **The evaluator is unedited.** No checker rule, diagnostic code, `Q`-code or
  `QP`-code was added, changed or reordered. No raw unit is promoted, no CT case
  moves, the contract corpus stays 61 modules and no `.qcore` file was touched.
- **No production caller.** `crates/qcore-audit/src` and `compiler/src` are
  untouched; the whole production diff is 533 additive lines across six files.

---

## 1. What the task is

`crates/qcore-verification/src/frame_source.rs` — `FrameHelperSourceTask`,
semantic profile `qcore.c_frame_helper_contract.v1`, in the shape every other
`*SourceTask` in the crate uses:

```rust
pub fn derive(
    summary: &ChunkFrameSummary,        // the sealed gateway import
    translation_unit: &str,
    contract_source: &str,              // the pinned contract's frame clauses
    flavour: FrameContractFlavour,
    target: TargetProfile,
) -> Result<Self, String>
```

`evaluate` calls exactly what the in-process routes call — the same
`evaluate_chunk_frame_contract` / `evaluate_boolean_frame_contract`, on the same
sealed import — and returns a `FrameHelperCertificate` recording the function,
source revision, record, both protected-field vectors, the flavour, the ordered
declared layout, the ordered constant writes and the checked-postcondition
count. `prerequisite_identity` and `revalidate_as_prerequisite` are the surface
a dependent names it by, identical in semantics to the counter route's.

### 1.1 The two contract vocabularies are a selection, not a reinterpretation

The two unedited evaluators state the same property under different ghost
names — `chunk_is_live` / `exclusive_chunk_access` / `lock_is_preserved` for
`error_state.qcore`, and `object_is_live` / `exclusive_object_access` /
`modeled_fields_are_preserved` for `error_frame.qcore`. `FrameContractFlavour`
selects which, and the selection is **part of the checked question**: it is
bound into the task identity, and a contract written in the other vocabulary is
refused by the evaluator with its own message rather than reinterpreted (§5.3).

The published premise set names the vocabulary that was assumed, so a dependent
performing an E03-B premise union records which spelling it inherited.

### 1.2 The containment condition, and the asymmetry it makes safe

`evaluate_chunk_frame_contract` pins its protected set to `{"lock"}`, while the
ChunkIO import protects `{"busy", "lock"}`. The evaluator is unedited, so that
asymmetry is real. `derive` requires the **evaluated** protection to be
contained in the **imported** one, and refuses otherwise.

The soundness argument, recorded rather than assumed: the importer refuses every
write to the imported vector, and the evaluator checks preservation of the
evaluated one. Containment means the evaluator never certifies preservation of a
field the importer did not already forbid. `busy` is refused at import, not
certified at evaluation. Both halves are proved in Lean (§6) and pinned in Rust
(§5.1).

The Chunk flavour additionally binds the `cio_chunk` record only: an unrelated
declared record cannot borrow the evaluator's pinned `{"lock"}`.

---

## 2. What the task binds — five keys, and why each is separate

Under the G01 compile-task-identity discipline, a task binds its complete
inputs. A test pins that these are exactly the task's `contract_sources` and
that the five digests are distinct.

| Key | Binds |
|---|---|
| `qcore:frame-helper-summary:v1` | the serialized `ChunkFrameSummaryRecord` — function, preprocessed source, ordered field/type layout, ordered constant writes |
| `qcore:frame-helper-import:v1` | the importer's captured `SourceInputIdentity` |
| `qcore:frame-helper-contract:v1` | the pinned contract bytes |
| `qcore:frame-helper-binding:v1` | the contract flavour, the declared record, the imported protected-field vector |
| `qcore:frame-helper-profile:v1` | the declared profile selection, or the explicit absence a legacy import records |

The last two are not decorative. **Measured finding:** the legacy and
profile-selected imports of the *same body* produce a byte-identical
`SourceInputIdentity` and a byte-identical serialized summary record — a helper
records no `model_library` and no `profile_digest`, because it has no field-model
owner, and the profile binding lives in a field the summary does not serialize.
Without these two keys the two would be **one task**, and a witness derived
under one declared protection would answer for the other. The test asserts the
first three keys equal and the last two different, then asserts the tasks
distinct.

`profile_binding()` on `ChunkFrameSummary` is the one new gateway API (an
accessor; no behaviour changed).

---

## 3. The recorded boundary `P04-FRAME-OBLIGATION-ID`

The shared `VerificationTask::obligation_id` scheme reads the task version, the
three `UnitIdentity` fields, the semantic profile, the capability, the property
and the subject. It does **not** read `contract_sources`. So two tasks over the
same body under *different pinned contracts* carry the same obligation
identifier.

This surfaced as a failing assertion in the first mutation matrix and was
corrected by measurement rather than by weakening the check. It is sound as it
stands, because `PrerequisiteIdentity` is a triple carrying the task digest, and
`revalidate_as_prerequisite` compares all three; a future consumer keying on the
identifier alone would be wrong. Recorded, pinned in Rust
(`the_obligation_identifier_alone_is_not_an_identity`) and **proved** in Lean
(`the_obligation_identifier_does_not_read_the_bound_inputs`,
`the_triple_separates_what_the_identifier_cannot`) so the assumption is visible
instead of implicit. It is a property of the shared scheme, not of this
capability, and is not repaired here.

---

## 4. `memory_close`: measured disposition — the task lands, the move does not

The card authorised moving `memory_close` through the boundary *if it now
composes cleanly in the established pattern*, and recording precisely otherwise.
It does not. Two findings, both by measurement.

### 4.1 A second blocker existed, and this card closes it

`memory_close` consumes four readings. The frame helper was the recorded
blocker; a census of the other three found a second:

| Dependency | Task | `prerequisite_identity` / `revalidate_as_prerequisite` before this card |
|---|---|---|
| `reset: ChunkFrameSummary` | **none existed** | — (this card) |
| `unlink: ListUnlinkCertificate` | `ListUnlinkSourceTask` | **absent** |
| `backend: FieldDestructorCertificate` | `FieldDestructorSourceTask` | **absent** |
| `counter: CounterCertificate` | `CounterSourceTask` | present (P04-NEXT-AB) |

Three of four could not be named, not one. Both missing surfaces land here, in
the counter route's exact shape, with the two teardown routes' bodies otherwise
untouched. `all_four_memory_close_dependencies_are_now_nameable` asserts the
identities are complete, pairwise distinct, and carry distinct semantic
profiles.

**After this card every one of `memory_close`'s four dependencies is nameable
by a `PrerequisiteIdentity` the core can re-derive and re-evaluate.** The census
row in `remaining-p04-compose` §1 moves from *"needs a still-in-process
prerequisite"* to composable-now.

### 4.2 Why the move itself is a separate card

Moving `memory_close` is a **sixth transported source vocabulary**, the same
size of unit as P04-COMPOSE was for the fifth. Measured:

- `compiler/src/verifier_plugins/chunkio/memory_close.rs` is **717 lines**
  against `unmap_composition.rs`'s 527; its decision procedure (the `Value`
  domain, the path replay, `checked_paths` / `uncovered_paths`, the
  `instantiate(&ChunkState, Option<u32>, bool)` surface) would have to relocate
  into `qcore-verification` before anything can travel.
- It needs a new `SemanticWitness` variant, a new `ObligationProperty`, a new
  `memory_close_source` projection slot on `VerificationRequestV2` and its
  `verify_v2` QP001 binding — a further `qcore-schemas` major bump — plus
  provider-side recomputation, a ChunkIO package version bump, a conformance
  suite and its own mutation matrix.
- It carries a transport question no moved capability has faced: `check_memory_close_source`
  appends a `PROBE` to the translation unit and imports from a **derived** unit
  whose input identity is `for_derived_text()` — deliberately *not* the declared
  file. What identity that derived unit records, and whether it may travel at
  all, has to be decided before a projection can be defined.
- `MemoryCloseDependencies.reset` would change from `ChunkFrameSummary` to a
  task-mediated prerequisite, a compiler-side signature change rippling into
  `filesystem_chunk_close` and the pinned Cycle 19 compatibility adapter.

Recorded as follow-on **P04-COMPOSE-MEMCLOSE**. `filesystem_chunk_close` and
`unsupported_close` stay blocked behind it, one and two levels up.

---

## 5. Evidence — 26 new tests, all passing

### 5.1 Positive suite — `crates/qcore-verification/tests/frame_source.rs` (8)

Both flavours are driven through the **real** gateway importers over real C
bodies; nothing constructs a summary by hand, and nothing can.

- the five bound-input keys are exactly the task's `contract_sources`, with five
  distinct 64-hex digests;
- exactly one obligation, `FrameHelperContract` under `ResourceContracts`, with
  its exact subject, scope, `PathBounds::SinglePath` and `evidence_version: 2`
  — one path is the whole admitted body (a straight-line sequence of constant
  field assignments), not a sampled one;
- the published premises carry the four trust premises plus the **flavour's own
  vocabulary**, and neither flavour carries the other's;
- the core's own evaluation reads `cio_error_reset` writing `("error_n", 0)`
  over the declared `[lock:int, error_n:int]` layout with **3** checked
  postconditions — measured, and corrected from an initial guess of 5: the
  evaluator returns the `ensures normal` count, while the contract corpus's
  pinned 5 for `cio_error_reset` also counts its two `requires` premises;
- the evaluated protection `{lock}` is a strict subset of the imported
  `{busy, lock}` on the Chunk flavour, and equal to it on the generic one;
- the prerequisite identity is the three-field key, and re-establishment
  recomputes rather than returning a stored reading;
- deriving twice gives an equal task, digest, identity and certificate;
- the capability refuses every witness, and refuses a foreign task first, on its
  own ground.

### 5.2 Mutation matrix — `frame_source_mutations.rs` (13), every row refused

Every row asserts the mutated input really differs from the honest one **before**
it is submitted, so no row passes vacuously. Identity rows check **both
directions**: neither context accepts the other's prerequisite identity.

| Group | Rows |
|---|---|
| Identity substitutions | changed contract bytes; a build change invisible to the projection; a changed body; the profile selection and declared protection; the contract flavour; the translation unit |
| The obligation-identifier boundary | §3, asserted in both halves plus a `UnitIdentity` row that *does* move the identifier |
| Prerequisite triple | each of the three fields substituted; each of the three emptied |
| Target and import refusals (no task exists) | empty library; another capability's profile; an unadmitted dialect; a dialect the import did not use; another revision; another platform; a non-native backend; empty translation unit; empty contract; the Chunk flavour over an unrelated record |
| Evaluator parity | five falsified contracts, each asserted **byte-for-byte equal** to a direct call of the unedited evaluator |
| Vocabulary crossing | each flavour over the other's contract, refused with the evaluator's own message |
| Fail-closed | an honest identity over a false contract discharges nothing |

The **isolation row** for the import key: an unused `-D` leaves the preprocessed
text, and so the whole serialized summary record and the binding and profile
keys, byte-identical, and moves only `configuration_digest`. Without the
separate import key the two tasks would be one.

An earlier attempt at that isolation used a comment-only source edit; measurement
showed the comment becomes a space in the preprocessed text and moves the summary
record too, so the row was replaced with one that actually isolates.

### 5.3 Teardown prerequisites — `teardown_prerequisites.rs` (5)

Naming and re-establishment on both routes; each of the three identity fields
load-bearing; cross-capability refusal in both directions; a build change
retiring the identity on both routes; and the four-dependency census of §4.1.

---

## 6. Lean — 27 audited theorems, no new axiom

`proof/QCore/FrameHelperTask.lean` and `proof/AuditFrameHelperTask.lean` add the
`QCore.FrameHelperTask` namespace. Every theorem depends only on `propext`,
`Classical.choice` and `Quot.sound`; `check_lean_audits.py` moves 62 → **63**
audits.

- *derivation is a function, and every bound input is load-bearing* (8):
  the same inputs give the same identity; equal digests are equal tasks; each of
  the five keys retires the task on its own; and no bound input can be absorbed
  by another's slot;
- *the recorded boundary* (2): the obligation identifier is blind to the bound
  inputs, **proved**, and the triple is what separates two tasks the identifier
  cannot;
- *re-establishment is fail-closed* (5): an incomplete claim, a foreign claim
  and a refusing core evaluation each refuse; each of the three fields is
  load-bearing on its own; the derived identity over an evaluating core is
  established;
- *what `derive` decides about the frame* (5): an empty write set, an empty
  imported protection and an uncontained evaluated protection are each not
  derivable; derivability implies containment and a nonempty write set;
- *the tie to the unedited frame-preservation theorems* (3): a derivable helper
  whose writes are disjoint from its imported protection is admitted by
  `QCore.ChunkSource.checkedFrameWrites` and preserves every imported protected
  field; **containment carries that preservation to the evaluated set**, which is
  the §1.2 argument as a theorem; and a write to an imported protected field is
  refused outright;
- *non-vacuity* (4): the real `cio_error_reset` shape is derivable, preserves
  both `lock` and `busy`, and really writes `error_n` — the preservation results
  are not the statement that nothing happens.

Judged **not applicable** and recorded rather than modelled: whether the C body
Clang parsed is the body that runs, cross-unit record ABI equality, live storage
and exclusive access. Those are the pinned contract's declared premises and the
audit's own; the task publishes them as assumptions rather than discharging
them. The model also gives a digest perfect injectivity — a task is its own
digest — so SHA-256 collision resistance is not modelled and is not claimed.

---

## 7. Schema decision — flagged loudly

**`qcore-schemas` moves 10.0.0 → 11.0.0. This is a major schema bump.**

One addition, source-incompatible for exhaustive matches exactly as in 4.0.0,
5.0.0, 7.0.0, 8.0.0, 9.0.0 and 10.0.0:

| Addition | Kind |
|---|---|
| `ObligationProperty::FrameHelperContract` | new variant on an exhaustively matched enum |

Additive was preferred and is not available. No existing property states this
question: `BooleanFieldContract` is the two-prestate borrowed-field contract of
a whole function, not the frame helper's write-footprint and preservation
property, and reusing it would misstate what was checked.

**Existing wire bytes are unchanged and pinned.** The variant is emitted by no
existing route, so no serialized task, witness, request or provider package
moves because of it. Package digests did move — all five packages were rebuilt
and verified under the new schema version, and only their identity fields
changed.

The one exhaustive match the compiler found is
`qcore_verification::verify_dispatch::predicate_for`; the new property joins the
resource-contract group's `FactPredicate::ResourceBalanced`, which that
function's own comment records as envelope compatibility only — the obligations,
not these facts, are the carrier.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 10.0.0 | **11.0.0** |
| `qcore-verification` crate | 0.4.0 | 0.4.0 (unchanged) |
| `qcore-verifier-host` crate | 0.4.0 | 0.4.0 (unchanged) |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (unchanged) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| ChunkIO package | 0.9.0 | 0.9.0 (unchanged) |
| Frame-helper semantic profile | — | `qcore.c_frame_helper_contract.v1` |

**One deliberate pin update:** `compiler/tests/fixtures/w3/provider/Cargo.lock`
moves to schemas 11.0.0 so the pinned standalone-provider fixture's `--locked`
build resolves. `providers/Cargo.lock` is regenerated for the same reason.

The verification and host crate versions are deliberately **not** bumped: their
public APIs gain only additive items (`frame_source`, two methods on the
teardown tasks), so no consumer breaks.

---

## 8. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2482 passed, 0 failed, 7 ignored** (+26 new tests) |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **63 audits passed** (62 → 63), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **94 passed, 0 failed** |
| `providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0) |
| `python3 -m unittest discover -s scripts/tests` | 125 tests, **1 failed** — see §8.2 |
| Canonical CMetrics audit, before vs after | **byte-identical** — see §8.1 |

### 8.1 The CMetrics audit is byte-identical, measured

Not argued: measured. The prepared Clang build tree was regenerated, the audit
run at the untouched base commit `009bde0`, the branch checked out, and the audit
run again. Both reports are
`3794e234f90d218eb3d8159e1b501f55e7d990452a16c4286bd314499e1f0ea5`, with the same
`1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway
failures` and the same diagnostic inventory.

That is what the diff shape predicts: `crates/qcore-audit/src` and
`compiler/src` are untouched, the whole production diff is 533 additive lines
across six files, and no production route references the new module. The four
pinned 185-unit artifacts are therefore unchanged **by construction**, and were
not re-measured (the pinned corpus tree is not prepared in this worktree).

### 8.2 Two known-flaky rows, and one failure the card cannot fix

**Load-flaky, confirmed pre-existing.** `a_confined_fixture_cannot_write_outside_the_admitted_paths`
(P03) and `a_declared_process_limit_stops_the_fixture_from_forking` (P02) each
failed on one run with `Broken pipe (os error 32)` — `Io` where `Terminated` is
asserted. Both were then **run at the untouched base commit `009bde0` and fail
there too** (the confined suite failing two rows there, one more than on this
branch), so they are this host's recorded flakiness, not a consequence of this
card. A later full run passed all of them: **2482 passed, 0 failed**. No
assertion was weakened.

**By design, and forbidden to fix.**
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails: the committed `docs/reviews/r0_3/acceptance_matrix.json` carries
`['10.0.0', 62]` and the freshly derived value is `['11.0.0', 63]`, because this
card bumps the schema crate and adds one audited Lean module. The card's
constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 10.0.0 →
**11.0.0** and `validation.lean_audits` 62 → **63**, with
`AuditFrameHelperTask.lean` added to `validation.lean_audit_files`. Regenerate
the matrix (or supply `--gate`) at the wave's close, exactly as waves 9–13 did.
Nothing else in the matrix moves from this card: the contract corpus stays 61
modules, CT coverage is untouched at 32/50, and no verdict moves.

### 8.3 Overlap with P04-COMPOSE-A

`P04-COMPOSE-A` ran in parallel in the same crates. Recorded, not resolved here:

- `crates/qcore-schemas/src/obligation.rs` — both cards add an
  `ObligationProperty` variant and both bump the schema major version. The
  integrator should land one bump, not two, and reconcile the version number and
  the matrix action item in §8.2 accordingly.
- `crates/qcore-verification/src/verify_dispatch.rs` `predicate_for` — both
  cards add an arm to the same exhaustive match. The two arms are independent
  and both belong in the `ResourceBalanced` group.
- `crates/qcore-verification/src/lib.rs` — both add a `pub mod` line.
- `crates/qcore-verification/src/teardown_source.rs` — this card adds two
  methods to each of the two tasks. If P04-COMPOSE-A also needed those surfaces,
  the two additions are the same shape and one should be dropped.
- All eight `Cargo.toml` schema version requirements and both `Cargo.lock`
  files.

---

## 9. Open items and named boundaries

- **P04-COMPOSE-MEMCLOSE** — moving `memory_close` through the boundary is a
  sixth transported vocabulary, sized in §4.2, with the derived-probe-unit
  transport question to settle first. Its four dependencies are now all
  nameable, which is what this card was for. `filesystem_chunk_close` and
  `unsupported_close` stay blocked behind it.
- **P04-FRAME-OBLIGATION-ID** — §3. A property of the shared obligation-identifier
  scheme, recorded and proved, not repaired.
- **P04-FRAME-NO-WITNESS** — this capability has no transported witness form.
  No projection travels, no `SemanticWitness` variant can carry a frame reading,
  and `WitnessChecker::check` refuses every witness by construction. The
  obligation is discharged only by the core's own `evaluate`, reached directly
  or through `revalidate_as_prerequisite`. This is an explicit decision recorded
  as a refusal rather than left as an omission, so a future transported form has
  to be added deliberately.
- **P04-FRAME-CHUNK-PROTECTION** — the Chunk evaluator pins `{"lock"}` while the
  ChunkIO import protects `{"busy", "lock"}`. Made safe by the containment
  condition (§1.2), proved in Lean, and left as it is because the evaluator is
  unedited by this card's terms. Declaring the protected vector in
  `error_state.qcore` the way P04-B1 did for the cleanup families would make it
  contract-derived instead.
- **No production caller.** The task is exercised by these tests, not by the
  Cycle 19 audit or the configured audit, which still reach the frame helper
  through `check_chunk_frame_contract` and `evaluate_boolean_frame_contract`
  directly. This is the same position P05's moved reactor capability, P04-B2's
  moved cleanup families and P04-COMPOSE's moved composition occupy. Wiring
  `qcore-audit`'s `import_configured_frame_helpers` onto the task would move
  the `frame_helpers` evidence rows and is deliberately not done here.
- **Nothing about the OS or the C ABI is established.** A declared record layout
  is a declared record layout; ordered field/type names do not prove recursive
  layout or alignment equality across translation units, and that stays an
  explicit published assumption
  (`cross_unit_record_abi_equality_is_an_outer_audit_premise`).
- The in-process route is **retained** (hard invariant 3):
  `compiler/src/chunk_contract.rs`'s `check_chunk_frame_contract` and
  `memory_close`'s use of it are byte-identical. Removing either needs a
  production caller for the task route plus the P09-style archived parity
  measurement.
