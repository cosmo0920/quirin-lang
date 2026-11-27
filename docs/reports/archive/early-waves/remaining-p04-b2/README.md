# P04-B2 — the symbolic cleanup-replay vocabulary through the process boundary

Workstream W5 (Providers), follow-on **P04-NEXT-B2**. Bounded increment: the
two ChunkIO **cleanup-outcome** capabilities — `file_outcomes`
(`cio_file_native_close`, `cio_file_native_unmap`) and `file_delete`
(`cio_file_native_delete`) — move through the process boundary as the **fourth**
transported source vocabulary, `qcore-schemas::cleanup_replay_summary`.

- Worktree cut from `main` at **`67961f1`** (wave-11 consolidation); branch
  `worktree-agent-a134e19f858b3b0c4`.
- Libevent submodule initialised at the pinned revision.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code and no
  `QP`-code was added, changed or reordered. No raw unit is promoted.
- `crates/qcore-verification/src/cleanup_outcome.rs` — the checker that decides
  both families — is **byte-identical to `67961f1`**. It is the sealed core's
  own evaluator on both routes.

---

## 1. The blocker being closed

P04-NEXT-AB §6.3 recorded two independent blockers for these two dependents.
(a) — no contract-side referent for the `CleanupProfile` — was closed by
**P04-B1**, which made both contracts declare their field/reset/callee
vocabulary under `cleanup_vocabulary_v1`. (b) was left standing, quoted here
verbatim from that report's measurement:

> the reading a producer would have to compute (`CleanupCase`) is not
> syntactic: `field_writes`, `fields_preserved` and `fields_reset` are functions
> of a symbolic *field state* evolved by a 9-constructor value lattice over four
> (respectively five) sampled `(entry, syscall outcome)` prestates, with the
> syscall's argument identity checked against the initial field values. That is
> a symbolic replay evaluator, not a def/use fold. By P04-NEXT's own rule […]
> that is a separate card.

This is that card. It does not widen the third vocabulary; it defines a fourth
one, and it uses P04-B1's declaration as the thing a transported profile is
cross-checked against.

### 1.1 Why a fourth vocabulary, by construction

| Existing vocabulary | Why it cannot carry this question |
|---|---|
| Boolean / counter path projections | They carry a path so the receiver can *evaluate* it to a field state under one prestate pair. This family evolves a field state over four or five sampled prestates and must report **which path each prestate selects** |
| Teardown projection (P04-NEXT) | No `ReturnValue`, no `LocalStart`, and a call table closed to one co-transported helper or exactly `free`. This family needs both missing forms and three declared callees |
| Call-result def/use (P04-NEXT-AB) | Structurally closest, but its reading is a dependency fold that discards *values*: it cannot express `fields_reset`, a selected path, or the syscall's argument identity against the initial field state. Its step vocabulary also admits a bare `Return`, which is not a complete cleanup path in either family |

So `cleanup_replay_summary` defines its own enums and shares nothing with the
other three, except `PrerequisiteIdentity` — which is the point of §4.

---

## 2. The vocabulary, exactly

### 2.1 The closed value lattice — nine constructors, no catch-all

`CleanupLatticeValue` mirrors the trusted checker's private `Value` domain
constructor for constructor, in the same order:

| Constructor | What it stands for |
|---|---|
| `Null` | the null pointer, the only false pointer |
| `Root` | the record parameter itself, when entry is not null |
| `Pointer { field }` | a live pointer field's opaque identity |
| `Integer { value }` | an exact machine integer |
| `Symbol { field }` | a non-pointer field's opaque symbolic identity |
| `ErrnoPointer` | the declared errno accessor's result |
| `Errno` | the integer read through it |
| `Text` | a string literal, or a pointer decayed from one |
| `ReportResult` | the declared diagnostic helper's result |

There is deliberately **no** `Unknown`/`Other` constructor: an unresolved value
is a refusal, never an opaque token, so a producer cannot answer over a domain
the core does not share. A wire test round-trips all nine and refuses
`{"lattice":"unknown"}` and `{"lattice":"opaque"}`; a Lean theorem family pins
that the enumeration has exactly nine distinct tags and is complete.

### 2.2 The closed step vocabulary — eight forms

`Read`, `Write`, `LocalStart`, `LocalWrite`, `LocalEnd`, `Guard`, `Call`,
`ReturnValue`. Refused **by name** at projection, each with its own message:

| Refused form | Why |
|---|---|
| `Return` (value-free) | a cleanup path without a terminal `ReturnValue` is not a complete path in either family — this is the one step form the call-use vocabulary has and this one does not |
| `ScanLoop` | the in-process analyzer refuses structured loops too |
| `Terminate` | a declared nonreturning call is not a complete path |
| `LocalStorage`, `LoopIndex`, `SizeOf`, `Aggregate`, `Index`, `Function` | expression forms outside the closed list |

**Callees.** Admitted only against the profile's **own three declared names** —
the syscall, the errno accessor and the diagnostic helper. Unlike P04-NEXT-AB's
closed callee table, this one is not a caller assertion at all: it is read off
the profile, which is itself read off the contract (§3). A body calling anything
else keeps its existing `Unknown`.

### 2.3 The fixed sampled prestate domains

`CleanupPrestateRecord` has exactly two variants whose inhabitants the family
fixes, in the exact order the trusted checker enumerates them:

| Family | Domain |
|---|---|
| `NativeCleanupOutcome` | `(Null, —)`, `(Absent, —)`, `(Present, -1)`, `(Present, 0)` — **4** |
| `GuardedDelete` | `(fd,—,—)`, `(fd,map,—)`, `(—,map,—)`, `(—,—,-1)`, `(—,—,0)` — **5** |

A proposal that answers a shorter, longer, reordered or foreign domain is
refused; the two domains are proved disjoint, so no case of one family can be
read as a case of the other.

### 2.4 What crosses the boundary

`CleanupReplaySourceInput` carries three things and nothing else:

1. `schema_version`;
2. `prerequisite` — a `PrerequisiteIdentity` triple (§4), a lookup key;
3. one `CleanupReplayFamilyInput`, carrying the imported body's complete typed
   path set, the record `profile`, and (delete only) the `mapping_field`.

**No derived reading travels, and neither does the contract or the policy.**
Two tests assert this over the real transported request: the serialized
projection contains `paths` and `profile`, and contains none of `cases`,
`field_writes`, `fields_preserved`, `fields_reset`, `source_path`, `returned`,
`cleanup_calls`, `verdict`, `certificate`, `ensures` or the policy name
`native_close_outcomes` / `native_unmap_outcomes`.

The proposal (`CleanupReplayRecord`) reports, per sampled prestate: the lattice
`initial_fields` the replay began from, the single `source_path` that prestate
selects, `returned`, the call counts, and the record's field transition. Each
case variant mirrors, field for field, the case the trusted checker publishes
for its family — nothing is invented, and a figure the checker does not publish
for a family has no place in that family's variant.

---

## 3. The profile is contract-derived, not wire-derived

This is the P04-B1 payoff made concrete, and it is stronger than the teardown
route's treatment of its owned-field inventory.

`CleanupReplaySourceTask::derive` does **not** accept the transported profile.
It calls `declared_profile(contract, policy)`, which reads the pinned contract's
own `cleanup_vocabulary_v1` clauses, **builds its own `CleanupProfile` from
them**, and then refuses unless the transported record is exactly that. The
profile it hands the checker is always its own. A producer therefore cannot
choose the record shape, the reset values, the syscall identity or the
diagnostic helpers it answers over.

The policy name is deliberately **not** a member of `CleanupProfileRecord`:
which declaration the core reads is the core's own selection and never travels.
A wire test pins that adding `policy` to the profile object is refused by
`deny_unknown_fields`.

The guarded-delete eligibility field is not a profile member, so the **family**
must agree with the declaration in both directions: the delete policy declares
a `cleanup_mapping_field__` clause and the projection must name that field; the
two outcome policies declare none and a guarded-delete projection over them is
refused. Both directions are measured (§6.4).

### 3.1 Digests the task binds

| Key | Binds |
|---|---|
| `qcore:cleanup-replay-source-input:v1` | the complete projection |
| `qcore:cleanup-replay-import:v1` | the importer's captured source/build identity |
| `qcore:cleanup-replay-contract:v1` | the pinned contract text |
| `qcore:cleanup-replay-policy:v1` | the selected declaration |
| `qcore:cleanup-replay-profile:v1` | the profile **the core derived**, not the one transported |
| `qcore:prerequisite-identity:v1` | the prerequisite's identity (§4) |

A test pins that these six, and only these six, are the task's
`contract_sources`, and that two distinct bodies never share them.

---

## 4. The prerequisite mechanism gets its first production consumer

P04-NEXT-AB landed `PrerequisiteIdentity` and
`FileAttemptSourceTask::revalidate_prerequisite` with the note: *"this API has
no production consumer yet, because no dependent has been moved."* Both
dependents moved here, so it now has one, and follow-on
**P04-NEXT-AB-CONSUMER** closes.

The dependent's request carries the identity triple and nothing else about the
prerequisite. Before any dependent proposal is looked at,
`CleanupReplaySourceTask::check` calls `revalidate_prerequisite`, which:

1. re-derives the prerequisite task **from the core's own import**;
2. refuses unless the claimed triple *is* the identity that task has;
3. **re-runs the prerequisite's own contract evaluation**, and refuses if it
   refuses.

A dependent's admission is therefore bought entirely by work the core does. The
decisive measurement is §6.3's last row: an **honest** triple over a
prerequisite whose own contract has been falsified is refused, with the
dependent's own contract untouched — so step 3 is not decorative.

---

## 5. One decision procedure, not two

Both routes call the same unedited `check_cleanup_outcomes` /
`check_guarded_delete_outcomes`. Nothing was relocated and nothing was
duplicated: `cleanup_outcome.rs`, `file_outcomes.rs` and `file_delete.rs` are
byte-identical to `67961f1`, and the retained in-process adapters keep their
certificate shapes (pinned by field-order assertions in
`compiler/tests/cleanup_replay_route_parity.rs`).

**One derivation is duplicated and is recorded rather than hidden.**
`initial_lattice_state` in `qcore-verification` mirrors the private
`Replay::new` rule so the core can compare the proposal's lattice-valued
`initial_fields`. It is a derivation of an *input*, not a decision, and it is an
**additional** agreement requirement — every published figure of a case still
comes from the unedited checker — so it can only make the route refuse more,
never admit more. Recorded as boundary **P04-B2-INITIAL-MIRROR**.

---

## 6. Evidence

### 6.1 Byte-identical certificate parity with the retained in-process routes

`compiler/tests/cleanup_replay_route_parity.rs` (4 tests). For all **three**
selected bodies, the moved route's certificate is compared to the in-process
adapter's, both as a `serde_json::Value` and as a serialized **byte string**:

| Body | In-process route | Moved route | Result |
|---|---|---|---|
| `cio_file_native_close` | `check_file_outcome_sources` | `CleanupReplaySourceTask::evaluate` | byte-identical |
| `cio_file_native_unmap` | `check_file_outcome_sources` | `CleanupReplaySourceTask::evaluate` | byte-identical |
| `cio_file_native_delete` | `check_native_delete_source` | `CleanupReplaySourceTask::evaluate` | byte-identical |

The moved route builds its profile from the contract alone and never sees the
adapter's hardcoded table, so this parity *is* the statement that the adapters'
`CleanupProfile`s are the declarations — P04-B1's binding measured end to end
rather than argued.

**20 parity refusal rows**, count pinned: thirteen outcome-body and seven
delete-body mutations that the in-process unit tests already pin as refused.
Each row asserts the in-process control really refuses *and* that the moved
route refuses it too. Nothing in this card makes a previously refused body
admissible.

### 6.2 Core-side positive suite

`crates/qcore-verification/tests/cleanup_replay_source.rs` (4 tests), driving
the real gateway import, the real task derivation and the real trusted checker:

- all three bodies replay their whole sampled domain, in order (4/4/5 cases);
- the unmap success arm reports `["alloc_size", "map", "map_truncated_warned"]`
  with `fields_reset`, and its failure arm writes nothing and preserves
  everything;
- the lattice distinguishes `Pointer{map}` from `Symbol{alloc_size}` in the live
  prestate and `Null` in the `Absent` prestate — which is what the eligibility
  guard and the syscall argument check turn on;
- the guarded delete reaches `unlink` only when both eligibility fields are
  clear, every arm preserves the record, and the two overrides are visible in
  the prestate lattice;
- no derived reading and no policy name travels with the request.

### 6.3 Core-side mutation matrix — 9 tests, all rows refused

`crates/qcore-verification/tests/cleanup_replay_mutations.rs`. Every row is
asserted to really differ from the honest input **before** it is submitted, and
the row counts are pinned so a row cannot silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed replay | **25** (pinned) | function, revision, signature, profile assumptions; case dropped/duplicated/reordered/emptied; and — on both a failure arm and the success arm — selected path, returned, cleanup calls, fields-preserved, fields-reset; write vector dropped/reordered/widened/renamed; prestate entry, outcome set and outcome erased |
| Forged lattice values | **9** (pinned) + 3 | each of the nine constructors substituted into a real case's `initial_fields`, plus a field added, a field removed, and the `Absent` reset value swapped for the live pointer identity it is precisely distinguished from |
| Sampled-domain and family substitution | 7 | the sibling body's **real** reading under the same contract (both directions); the other family's **real** reading (both directions); a foreign case grafted in; a foreign case *variant* grafted in; the family label swapped; a wrong witness kind |
| Task and witness binding | 6 | witness bound to another task; empty digest; a task the checker did not derive; a moved semantic profile; an invented contract-source key; the six-key digest set pinned |
| Prerequisite forgeries, both directions | 10 | each of the three fields emptied (refused at import); each replaced (refused at derivation); another real function's real identity; the honest triple as positive control; and an **honest triple over a falsified prerequisite contract** |
| Transported profile vs the declaration | **12** (pinned) + 5 | syscall, signature, root type, presence field, reset value, field type, field order, narrowed inventory, argument order, errno helper, report helper, function; plus family mismatch in both directions, an undeclared policy, the sibling policy, and a contract with no declaration at all |
| False contract | ≥8 | every `ensures normal` clause of both policies falsified two ways; each refuses |
| Moved source bytes / build options | 4 | changed bytes and a changed macro definition each retire the task; neither context accepts the other's witness |
| Bodies the in-process route refuses | **12** (pinned) | each stays refused on the process route |

**No mutation was admitted. No gate was changed and no repair was required.**

### 6.4 Provider-side matrix — 8 tests

`providers/common/cleanup_replay_effect_tests.rs`, driven over the real
projections the gateway emits (three committed fixtures, asserted live by a
conformance test). Every refusal row is a *decline*: the process returns no
proposal, the host sees no witness, and the obligation stays Unknown.

- three positive readings, each pinned against its family's fixed domain;
- **16 falsified-profile rows** — reset value, field type, renamed field,
  unknown syscall, wrong signature, reordered arguments, wrong eligibility
  field, unknown errno/report helper, wrong record type, widened and narrowed
  inventory, duplicated field, empty inventory, empty argument list, argument
  outside the inventory, wrong function;
- the delete family's own shape rules — four bad mapping fields, the three-field
  shape, each field's declared type and reset, and a body read under the wrong
  family;
- degenerate/oversized: unsupported versions both directions, three incomplete
  prerequisite triples, no path, an empty path, `MAX_PATHS`, `MAX_OPERATIONS`,
  three empty identity strings, a wrong signature, and an expression deeper than
  `MAX_EXPRESSION_DEPTH` — all decline without panicking;
- path shapes outside the replay rules: a non-terminal return value, a return of
  the wrong C type, a dangling symbol, an undeclared callee;
- the request gate declines every other capability, profile, dialect and an
  absent projection.

### 6.5 End-to-end through the real built package

`providers/conformance/tests/cleanup_replay_capability.rs` (2 tests). The
shipped `qcore.chunkio` **0.8.0** executable replays all three bodies; the core
admits only after rebuilding the profile from its own contract, re-establishing
the prerequisite and re-running its own replay. Also measured: missing evidence
(no core checker → QP016; no registered provider; a package without the
capability; a disabled provider), tampered transport (a well-formed body the
core never imported → QP001, then refused again by the core's own comparison
after the digest is rebound), and a false contract (→ QP009, never `Safe`).

**A measured finding, recorded rather than papered over.** The fixture-drift
test cannot compare the `prerequisite` triple across crates:
`SourceInputIdentity` binds `std::env::current_dir()` by design, so the
prerequisite's task digest and obligation id differ between the crate the
fixture was generated in and the conformance crate. The test compares the
schema version, the complete body path set and the declared profile exactly,
checks the prerequisite for shape and semantic profile, and **asserts the digests
differ** — so a future change that made the triple content-only fails here and
must be re-evaluated rather than silently inherited. Recorded as
**P04-B2-PREREQ-IDENTITY-CWD**.

### 6.6 Lean

`proof/QCore/CleanupReplayAgreement.lean` and
`proof/AuditCleanupReplayAgreement.lean` add **40 audited theorems** in a new
`QCore.CleanupReplayAgreement` namespace, with **no new axiom** — every one
depends on `propext`, `Classical.choice` and `Quot.sound` only.

- *the closed lattice* (5): exactly nine distinct constructors, a complete
  enumeration with no catch-all, every symbolic value has **no** truth value,
  and the decided half really is decided (including the reset-integer versus
  live-pointer distinction the replay turns on);
- *the closed step vocabulary* (9): a bare return, a scan loop, a termination,
  an out-of-vocabulary expression and an undeclared callee each have no
  projection; one refused step or tail refuses the whole path; a real cleanup
  shape projects and the same shape with a bare return does not;
- *the fixed sampled domains* (4): four and five prestates, duplicate-free, and
  the two domains disjoint;
- *fail-closed admission* (12): a refusing core, an undeclared profile, an
  undeclared family, a cross-task witness, a foreign task and a proposal over
  another domain each refuse on their own; a foreign prerequisite, a refusing
  prerequisite, and a dependent over a refusing prerequisite each refuse; every
  prerequisite field is load-bearing; a prerequisite does not cross tasks;
- *what an admission means* (8): every admission is backed by a replay the core
  ran itself, an admitted proposal **is** that replay, two admitted witnesses
  cannot disagree, a changed core replay retires the witness, and an admitted
  proposal answers its family's whole domain with its exact case count;
- *non-vacuity* (2): the core's own replay over its own declared profile and
  derived prerequisite identity is admitted, and the domain rule really
  discriminates.

Boundaries recorded in the module header and the audit file: the model
abstracts expression evaluation to a dependency-free step vocabulary and
abstracts the record profile to an equality-only token, so what is proved is the
*closure and the admission discipline*, not the arithmetic of the replay. Which
figures the checker computes is decided by the Rust tests of §6.1–6.3, and
whether a descriptor was released, a mapping torn down or a file deleted is
outside every statement.

---

## 7. Schema decision — flagged loudly

**`qcore-schemas` moves 8.0.0 → 9.0.0. This is a major schema bump.**

Additive was preferred and is not available. Three of the four additions are
source-incompatible for exhaustive matches and struct literals, exactly as in
4.0.0, 5.0.0, 7.0.0 and 8.0.0:

| Addition | Kind |
|---|---|
| `SemanticWitness::CleanupReplay` | new variant on an exhaustively matched enum |
| `ObligationProperty::CleanupReplayContract` | new variant on an exhaustively matched enum |
| `VerificationRequestV2::cleanup_replay_source` | new required struct field (skipped when absent) |
| `cleanup_replay_summary` module | purely additive |

No existing witness variant can hold this reading: `CallResultUse` carries a
dependency fold, `TeardownContract` alias-class or nullable-state cases,
`CounterContract` a single-path unsigned update.

**Existing wire bytes are unchanged and pinned.** `cleanup_replay_source` is an
`Option` that defaults to absent and is skipped on serialization, exactly like
the five projections before it.
`crates/qcore-schemas/tests/cleanup_replay_request_wire.rs` (3 tests) and the
extended `call_use_request_wire.rs` both pin that a request carrying none of the
**six** projections serializes with none of the six keys and round-trips.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 8.0.0 | **9.0.0** |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Cleanup-replay source input | — | 1 |
| Cleanup-replay semantic profile | — | `qcore.c_cleanup_replay.v1` |
| Cleanup-replay input digest key | — | `qcore:cleanup-replay-source-input:v1` |
| ChunkIO package | 0.7.0 | **0.8.0** |

**Deliberate pin updates**, each with its reason recorded beside the row:

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.7.0 → 0.8.0. The new
   profile is reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other target entry are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version
   in its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/{Cargo.toml,Cargo.lock}` — schemas
   9.0.0 so the pinned standalone-provider fixture's `--locked` build resolves.
4. `crates/qcore-verification/tests/composition_bounds.rs` — the E03-B-O3
   census of production sites naming `PathBounds::BoundedPaths` grows from four
   to five with `cleanup_replay_source.rs`. **The load-bearing negative half is
   untouched**: no production site states `schedules_explored: Some` or
   `max_operations: Some`, and that assertion still runs. The new site is the
   same kind as the other three — a function-scoped source task binding its
   exact complete-path count — so E03-B-O3's vacuity argument for the *count*
   kinds is unaffected.

All five provider packages were rebuilt and verified. The contract corpus is
untouched at **60 modules** (no `.qcore` file was added or edited).

---

## 8. The 185-unit corpus, re-measured

Measured through the canonical pinned route on the prepared pinned clone
(Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang`), with the provider packages rebuilt at this tree:

```sh
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone>/fluent-bit-clang-source \
  <clone>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Exit 0, **zero gateway failures**. Headline, unchanged:
**185 units — 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only.**

| Artifact | vs the wave-11 baseline |
|---|---|
| `resource-diagnostics.json` (11 MB) | **byte-identical** |
| `monkey-source.json` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` (4 MB) | **byte-identical** |
| `resource-diagnostics.contracts.json` | 10 leaves, path-literal provenance only |
| `resource-diagnostics.source-effects.json` | 47 leaves, identity/package only |

`chunk-contracts.json` being byte-identical is the load-bearing one: it embeds
both cleanup contracts' text and both families' certificates, so a moved
`CleanupCertificate`, contract or case would appear there and does not.

**Every moved leaf, attributed.** The complete itemisation is
[`artifacts/corpus-leaf-diff.json`](artifacts/corpus-leaf-diff.json); hashes are
[`artifacts/after.sha256`](artifacts/after.sha256) and the run log is
[`artifacts/corpus-run.log.gz`](artifacts/corpus-run.log.gz).

- **`contracts.json` — 10 leaves, one residue class.** The same five contract
  file contents, keyed under this worktree's absolute path instead of the main
  checkout's. Measured: after normalising only the measurement-directory prefix
  in the source keys, the two `sources` maps are **equal**, and every non-source
  member is byte-identical. This is the recorded path-literal provenance class
  (wave 9 §, wave 10 §), not drift.
- **`source-effects.json` — 47 leaves, two residue classes, zero unexplained.**
  All 47 are confined to the three checked source-contract results (16, 24, 63)
  and every one matches a whitelist of identity-only paths: `provider_version`
  0.7.0 → 0.8.0 and its `package_digest` (this card's deliberate package bump,
  3 + 3 leaves), and the `configuration_digest` / `build_digest` /
  `obligation_id` / `witness.task_digest` ripple from `current_dir()` being this
  worktree, which re-keys the obligation-keyed `bounds` map (18 leaves).
  Programmatically checked: **0 unexplained leaves**, and all **381**
  verdict-surface leaves are equal.

No unit verdict, function verdict, diagnostic or case moved in either direction,
and no newly surfaced provisional Unsafe unit exists. The archived wave-11
baseline is **not** re-pinned by this card; the integrator's corpus
consolidation should expect exactly the two residue classes above, with the
package-version leaves as the only deliberate content.

---

## 9. Gate

Run in this worktree on Linux with Clang 21.1.8 available:

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2139 passed, 4 ignored, 2 failed** — both the recorded load-flaky cases; see §9.1 |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **54 audits passed** (53 → 54), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **87 passed, 0 failed** (75 → 87) |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.7.0 → **0.8.0** |
| `python3 -m unittest discover -s scripts/tests` | 125 tests, **1 failed** — see §9.2 |
| 185-unit corpus vs wave-11 baseline | 3 of 5 byte-identical; 57 leaves, all attributed (§8) |

Against `67961f1`'s 2119/4, the workspace grows by **+20** tests, and the
provider workspace by **+12**.

### 9.1 Load-flaky, observed and recorded

Two tests failed in the saturated full run and **pass in a clean isolated
rerun**: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(20 passed isolated) and P02's
`a_declared_process_limit_stops_the_fixture_from_forking`
(12 passed isolated). Both are the recorded P02/vp04 flake class. Neither
assertion was weakened.

### 9.2 One Python failure the card cannot fix, by instruction

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails: the committed `docs/reviews/r0_3/acceptance_matrix.json` carries
`['8.0.0', 53]` and the freshly derived value is `['9.0.0', 54]`, because this
card bumps the schema crate and adds one audited Lean module. The card's
constraints forbid editing that file.

**Integrator action item:** `validation.schemas_version` 8.0.0 → **9.0.0** and
`validation.lean_audits` 53 → **54**, with `AuditCleanupReplayAgreement.lean`
added to `validation.lean_audit_files`. Regenerate the matrix (or supply
`--gate`) at the wave's close, exactly as waves 9–11 did. Nothing else in the
matrix moves from this card: the contract corpus stays 60 modules, CT coverage
is untouched at 31/50, and no verdict moves.

---

## 10. Open items and named boundaries

- **P04-B2-INITIAL-MIRROR** — `initial_lattice_state` duplicates the private
  `Replay::new` rule so the core can compare the proposal's lattice-valued
  prestate. It derives an *input*, not a decision; every published case figure
  still comes from the unedited checker, so it can only refuse more. Recorded
  rather than hidden (§5).
- **P04-B2-PREREQ-IDENTITY-CWD** — the prerequisite identity triple is not a
  content-only function of the body: `SourceInputIdentity` binds
  `current_dir()` by design, so a projection generated in one directory carries
  a different triple than the same body projected in another. Pinned as a
  measured fact by the conformance drift test (§6.5). Whether that is the right
  identity for a *transported lookup key* is a separate question from whether it
  is the right identity for a recorded compile task.
- **P04-B1-HELPER-SIG** (inherited, unchanged) — the errno and report helper
  *signatures* are checker literals, not profile data, so they are not declared
  and therefore not cross-checked against the contract on this route either.
  The transported profile carries the helper *names*; the signatures
  `int *(void)` and `int (int, const char *, int)` remain literals in both the
  core's replay and the provider's.
- **P04-NEXT-AB-CONSUMER — closed.** The prerequisite mechanism now has two
  production consumers.
- Nothing about the OS is established. A declared and recorded `close`,
  `munmap` or `unlink` call is a recorded call; whether a descriptor was
  released, a mapping torn down or a file deleted is outside every claim here,
  and the contracts' `requires` premises stay declared premises recorded in each
  task's assumptions.
- The in-process adapters for both capabilities are **retained** (hard
  invariant 3). Their removal would need the P09-style archived parity
  measurement plus a production caller for the process route; this card's
  §6.1 supplies the parity half only.
- The moved capabilities have **no production caller yet** — they are exercised
  by these tests, not by the Cycle 19 audit, which still runs the in-process
  adapters. This is the same position P05's moved reactor capability occupies.
- Of P04-NEXT's original thirteen in-process ChunkIO capabilities, five have now
  moved (counter, list-unlink, field-destructor, call-result def/use, and this
  card's two cleanup-outcome families make six routes across five vocabularies).
  The remaining classification — two behind the sealed `SourceMatch` boundary,
  seven compositions — is unchanged.
