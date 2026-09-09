# P04-COMPOSE — the ChunkIO composition capabilities through the process boundary

Workstream W5 (Providers), follow-on to `P04-NEXT`'s seventh classification
row: the **seven composition modules**, whose evidence is a reading over
*other capabilities'* certificates rather than over one body alone.

This card censuses all seven against today's transported set and moves the
composable-now subset — which measurement says is exactly **one**:
`unmap_wrapper` (`munmap_file`). It travels as the **fifth** transported source
vocabulary, `qcore-schemas::composition_summary`.

- Worktree cut from `main` at **`bf210eb`** (wave-12 consolidation); branch
  `worktree-agent-ab2456384ca09e324`.
- Libevent submodule initialised at the pinned revision.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code was
  added, changed or reordered; no `QP`-code was added. No raw unit is promoted.
- The in-process adapter is **retained** (hard invariant 3) and now *delegates*
  to the same relocated decision procedure the moved route runs, so the two
  cannot drift.

---

## 1. Census of the seven composition capabilities

Read module by module from `compiler/src/verifier_plugins/chunkio/`, classified
by **which certificates each consumes** and **whether the capability that
produces each one has moved**. A composition is "composable-now" only when
every prerequisite it consumes can be named by a `PrerequisiteIdentity` — that
is, only when a `*SourceTask` exists for it that the core can re-derive and
re-evaluate.

Moved capabilities available at census time: Boolean field (`chunk_source`,
W3), unsigned counter (P04), list unlink and nullable-field destructor
(P04-NEXT), call-result def/use (P04-NEXT-AB), symbolic cleanup replay —
native outcome and guarded delete (P04-B2).

| Composition module | Consumes | Class | Reason |
|---|---|---|---|
| `unmap_wrapper::check_unmap_wrapper_source` | call-result def/use of `munmap_file`; cleanup replay of `cio_file_native_unmap`; counter contract of `cio_chunk_counter_total_up_sub` | **composable-now — moved by this card** | all three prerequisite capabilities have moved (P04-NEXT-AB, P04-B2, P04), and each has, or gains here, a task the core can re-derive |
| `file_close::{check_file_close_source, check_file_close_deletion_source}` | call-result def/use of `cio_file_close`; **`unmap_wrapper`**; cleanup replay of `cio_file_native_close`; (deleting arm) guarded delete of `cio_file_native_delete` | needs a still-in-process prerequisite **at census time** | its only unmoved prerequisite was `unmap_wrapper`, which this card moves. It is therefore composable-now *for the next unit*, recorded as **P04-COMPOSE-A** |
| `memory_close::check_memory_close_source` | frame-helper `ChunkFrameSummary` + the `error_state.qcore` contract; nullable-field destructor; list unlink; counter | **needs a still-in-process prerequisite** | three of the four have moved, but the frame helper has an **importer** (`qcore_c_gateway::boolean_source::import_chunk_frame_helper`) and a **contract evaluator** (`qcore_contract::chunk_contract::evaluate_chunk_frame_contract`) and **no verification task at all**. With no `*SourceTask`, there is no identity for a `PrerequisiteIdentity` to name and nothing for the core to re-derive. Recorded as **P04-COMPOSE-FRAME** |
| `filesystem_chunk_close::check_filesystem_chunk_close_source` | `memory_close`; `file_close` (deleting arm); its own filesystem-kind probe | blocked by dependency | needs both P04-COMPOSE-FRAME and P04-COMPOSE-A first |
| `unsupported_close::check_unsupported_close_source` | `filesystem_chunk_close` | blocked by dependency | one level higher again |
| `nested_memory::check_nested_memory_source` | two `TraversalSource`s and two `ParentSource`s — both `SourceMatch` — plus `memory_close` | **excluded — sealed `SourceMatch` boundary** | `SourceMatch` is a sealed reviewed-library wrapper with no public constructor, conversion or deserialization (W5 typed-body boundary). A process cannot propose one without breaking that boundary. Unchanged from P04-NEXT |
| `nested_cleanup::check_nested_cleanup_source` | `nested_memory`; `filesystem_chunk_close` | **excluded — sealed `SourceMatch` boundary** (transitively) | same |

**Summary: one composable-now and moved; three blocked on a prerequisite whose
capability has not moved (one of them newly unblocked by this card); one
blocked two levels up; two excluded behind `SourceMatch`.**

### 1.1 Why the frame-helper gap is structural, not clerical

It would have been easy to call `memory_close` composable-now, because three of
its four dependency slots are moved capabilities. It is not, and the reason is
the mechanism this whole family rests on. `PrerequisiteIdentity` is a *lookup
key* whose three fields — semantic profile, task digest, obligation id — are
re-derived by the core from its own import. A capability with no task has none
of the three. Transporting the frame summary itself would be transporting a
derived result, which is exactly what this vocabulary is built not to do.

Standing up a frame-helper task is a capability definition (a profile, a
contract policy binding, an obligation property, a projector), not a widening
of this one — the same rule P04-NEXT applied to `file_attempts`. Recorded as
**P04-COMPOSE-FRAME**.

---

## 2. What actually crosses the boundary

`CompositionSourceInput` carries three things and nothing else:

1. `schema_version`;
2. `prerequisites` — **three** named `PrerequisiteIdentity` triples
   (`wrapper_call_use`, `native_cleanup`, `counter`), which are lookup keys;
3. one `CompositionFamilyInput::UnmapWrapper { wrapper, native,
   native_profile }` — the wrapper's complete typed path set, the **prerequisite
   body's** complete typed path set, and the record profile the prerequisite
   replay evolves.

**No composed result and no prerequisite conclusion travels, in either
direction.** No case vector, no selected path, no counter figure, no verdict,
no contract text and no policy name. A composed certificate has no wire
representation on this route at all — which is the property that distinguishes
this from a naive "composition" wire format, where a producer could send the
prerequisite's conclusion and thereby assert it.

The consequence is deliberate: **the producer must recompute each prerequisite
reading itself.** `providers/common/composition_effect.rs` calls the same
`replay_native_cleanup_outcome` this package already performs for the
cleanup-replay capability standalone, then runs its own wrapper replay over the
family's eight sampled entry states. It evaluates no contract and reads no
policy.

Three tests assert the absence, on the real transported request: the
serialization contains `paths`, `native` and `native_profile` and contains none
of `cases`, `native_source_path`, `counter_decrements`, `wrapper_writes`,
`returned`, `field_writes`, `fields_reset`, `certificate`, `verdict` or the
policy name `native_unmap_outcomes`.

### 2.1 The composed figure

`UnmapCaseRecord::native_source_path` is the one published figure that is a
function of *another capability's* reading: for each entry state that reaches
the native call, it names the path the **prerequisite's** replay selects for
that outcome. It is what makes this a composition rather than a body replay,
and the positive suite pins that it is not constant — the failure and success
outcomes select different prerequisite paths, and the two entry states that
never reach the call name no prerequisite path at all.

### 2.2 Shared path records, recorded rather than hidden

The two bodies travel as `CleanupReplayFunctionRecord`s. The wrapper replay
executes exactly the same eight step forms and eight expression forms the
cleanup replay does, so a private copy of those enums would be a byte-for-byte
duplicate with no distinct refusal surface. What is **not** shared is the
projector: `qcore_c_gateway::composition_source` has its own `value`/`project`
with its own closed callee table, and the composition route's admission is
decided there. Recorded as boundary **P04-COMPOSE-SHARED-PATHS**; a wire test
pins the exact variant inventory this route admits, so a future widening of the
shared enums fails there instead of silently widening this route.

### 2.3 The wrapper's callee table is a caller assertion

`contract/qcore/chunkio/unmap_wrapper.qcore` declares **no callee vocabulary**,
so unlike the cleanup route's contract-derived table this one cannot be read off
a declaration. `UNMAP_WRAPPER_CALLEES` is a four-entry closed table matched on
**both** name and declared C signature, mirroring the trusted composition's own
call table. It buys no authority — the core runs its own composition over its
own import and its own contracts regardless — and declaring it in the contract
is recorded as follow-on **P04-COMPOSE-CALLEE-DECL**.

---

## 3. Where the authority is: four things the core does for itself

`CompositionSourceTask::derive` and `evaluate` do all of the following before
any proposal is compared.

1. **The prerequisite record profile is contract-derived, not wire-derived.**
   `derive` calls P04-B2's `declared_profile(contract, policy)`, builds its own
   `CleanupProfile` from the pinned contract's `cleanup_vocabulary_v1` clauses
   (card P04-B1), and refuses unless the transported record is exactly that. It
   additionally refuses a policy that declares a guarded-delete eligibility
   field, so the wrong family's declaration cannot be selected. The profile
   handed to the composition is always the core's own.
2. **Every prerequisite is re-established, never read.** For each of the three,
   the core re-derives the prerequisite task from its own import, refuses
   unless the claimed triple *is* the identity it derives, and re-runs that
   prerequisite's own contract evaluation. The cleanup prerequisite in turn
   re-establishes *its* prerequisite (`revalidate_as_prerequisite` calls
   `revalidate_prerequisite` before `evaluate`), so a chain cannot be admitted
   by its weakest link.
3. **The bodies are tied to the prerequisites.** The wrapper summary this route
   imported must be the summary the def/use prerequisite checked, and the
   prerequisite body summary must be the one the cleanup prerequisite checked.
   Without this a projection could compose one body's paths with another body's
   re-established certificate.
4. **Acceptance terminates in the core's own composition.** The core runs the
   unedited procedure over its own imports, its own contracts and its own
   derived profile, and only then requires the proposal to be that same
   reading. Agreement alone discharges nothing — §5.3's decisive rows measure
   both halves.

### 3.0 The published premise set is the union, per E03-B

A composition that published only its own premises would be claiming its
prerequisites' obligations under **fewer** assumptions than those prerequisites
were checked under — exactly the failure card E03-B repaired at the host level
when it replaced QP012's unequal-premise-set refusal with a premise union.

`composition_premise_union` applies the same semantics at capability level: the
task's and the obligation's `assumptions` are the union of all three
components' premise sets and the composition's own, and a key two components
**disagree** on refuses rather than being silently resolved. This route is a
single-provider capability, so the merge happens in the task rather than in
`compose_verification`; the function is public so the disagreement direction can
be driven directly, and a test asserts both — every component premise survives
with its value, the published set *is* the union, and it is a strict superset
(no component assumes the wrapper's external sync and logging frames; the
composition does).

### 3.1 Digests the task binds

Ten keys, and a test pins that they are exactly the task's `contract_sources`:

| Key | Binds |
|---|---|
| `qcore:composition-source-input:v1` | the complete projection |
| `qcore:composition-import:v1` | the importer's captured source/build identity |
| `qcore:composition-contract:v1` | the composition's own pinned contract |
| `qcore:composition-native-contract:v1` | the **prerequisite's** pinned contract |
| `qcore:composition-native-policy:v1` | the selected prerequisite declaration |
| `qcore:composition-native-profile:v1` | the profile **the core derived** |
| `qcore:composition-counter-contract:v1` | the counter prerequisite's contract |
| `qcore:composition-prerequisite-call-use:v1` | that prerequisite's identity |
| `qcore:composition-prerequisite-cleanup:v1` | that prerequisite's identity |
| `qcore:composition-prerequisite-counter:v1` | that prerequisite's identity |

A composition binds **every contract it composes under**, not only its own: a
moved prerequisite contract is a different question. The three prerequisite
identities get three **distinct** keys, so no key can absorb another's
movement, and a test asserts the three digests differ.

---

## 4. One decision procedure, not two

`unmap_wrapper`'s decision procedure — `UnmapCase`, `UnmapCertificate`, the
eleven-constructor `Value` domain, the `Replay` and the eight sampled entry
states — is **relocated verbatim** into
`crates/qcore-verification/src/unmap_composition.rs`. The only change to the
moved body is that the constants it read from its former module scope (the
pinned revision, the prerequisite's contract and record profile, the counter
contract and its declared root types) are now explicit parameters
(`UnmapCompositionContracts`).

`compiler/src/verifier_plugins/chunkio/unmap_wrapper.rs` becomes a 79-line
adapter that supplies the ChunkIO half and delegates. Its public API is
preserved: `CONTRACT`, `check_unmap_wrapper_source`, and `UnmapCase` /
`UnmapCertificate` re-exported at the same paths. **The certificate's
serialized shape is byte-identical**, which the retained adapter's archived
corpus artifacts depend on and §6.1 measures.

`file_outcomes::profile` moves from `pub(super)` to `pub` so the parity test can
assert directly that the retained adapter's hardcoded record profile **is** the
vocabulary the pinned contract declares — P04-B2 measured that equality end to
end through certificate parity; here it is asserted as an equality of profiles.

Nothing else moved and nothing was duplicated:
`crates/qcore-verification/src/{cleanup_outcome,counter_contract,call_use_contract}.rs`
and `compiler/src/verifier_plugins/chunkio/{file_outcomes,file_attempts,file_close}.rs`
are otherwise unedited.

---

## 5. Evidence

### 5.1 Byte-identical certificate parity with the retained in-process route

`compiler/tests/composition_route_parity.rs` (4 tests).

| Check | Result |
|---|---|
| `file_outcomes::profile("cio_file_native_unmap")` vs `declared_profile(filesystem_outcomes.qcore, "native_unmap_outcomes")` | **equal**, and the policy declares no eligibility field |
| in-process `check_unmap_wrapper_source` certificate vs moved `CompositionSourceTask::evaluate()` certificate, as serialized bytes | **byte-identical** |
| the certificate is a real composition | 8 composed wrapper cases each naming a prerequisite path, plus the embedded prerequisite certificate's own 4 cases |

**Six refusal-agreement rows**, count pinned: a wrapper that stops decrementing
the counter, one that decrements twice, one that stops writing a size field,
one that returns success on the failing native outcome, one that decrements the
wrong counter, and a *prerequisite* body that stops resetting a declared field.
Each row asserts the retained route really refuses **and** that the moved route
refuses it too. Nothing in this card makes a previously refused body
admissible.

### 5.2 Core-side positive suite

`crates/qcore-verification/tests/composition_source.rs` (7 tests + a
fixture generator), driving the real gateway imports, the real task derivations
and the real trusted composition:

- the composition answers its family's whole eight-state sampled entry domain,
  in order, and the ordered state vector is pinned;
- every reached case names the prerequisite's own selected path, the two
  outcomes select **different** prerequisite paths, and the two unreached
  states name none;
- only the successful native outcome decrements the counter and writes
  `["data_size", "alloc_size"]`; every other state returns −1;
- the request carries no composed or prerequisite result (§2);
- the ten digest keys are exactly the task's contract sources and the three
  prerequisite digests are distinct;
- the record profile the composition is handed is the contract's own
  declaration;
- the published premise set is the union of all three components' premises and
  the composition's own (§3.0), the obligation carries the same set, and the
  union refuses a disagreement rather than resolving it.

### 5.3 Core-side mutation matrix — 7 tests, all rows refused

`crates/qcore-verification/tests/composition_mutations.rs`. Every forged
reading is asserted to actually differ from the honest one **before** it is
submitted, and the row counts are pinned so a row cannot silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed composition | **25** (pinned) | function, revision, signature, profile assumptions; cases emptied / dropped / duplicated / reordered; and on the successful arm: selected wrapper path, backend presence, mappedness, prior sync, sync outcome, native outcome, **the prerequisite's selected path (retargeted and erased)**, return value, counter decrement (inflated and erased), wrapper writes (dropped, reordered, renamed); and on a failure arm: a claimed write, a claimed decrement, and a prerequisite path given to an unreached case |
| Prerequisite forgeries, both directions | **12** (pinned) | each of the three triples emptied (refused at import); two repeated triples (refused at import); a foreign triple in each of the three slots and two real identities exchanged (refused at derivation); and **three decisive rows** — an *honest* triple over a prerequisite whose own contract has been falsified, once for each prerequisite, with the composition's own contract untouched |
| False composition contract | **≥20** | every `requires` and `ensures normal` clause of `unmap_wrapper.qcore` falsified two ways; each refuses |
| Transported profile vs the declaration | **10** (pinned) | syscall, signature, root type, presence field, field type, narrowed inventory, field order, argument order, errno helper, report helper |
| Task and witness binding | 4 | witness bound to another task, in both directions; another witness kind; a task the checker did not derive |
| Import and body binding | **3** (pinned) | an unadmitted callee in the wrapper; a substituted callee; a wrapper body the def/use prerequisite never checked |
| Moved source bytes | 2 | changed bytes retire the task; neither context accepts the other's witness |

**No mutation was admitted. No gate was changed and no repair was required by
the matrix.** The three decisive prerequisite rows are the point of the card:
the composition refuses when a prerequisite's own contract is false, even
though every transported figure and every identity triple is honest, because
the core re-runs each prerequisite's evaluation itself.

### 5.4 Provider-side matrix — 4 tests

`providers/common/composition_effect_tests.rs`, driven over the **real**
projection the gateway emits for these bodies
(`providers/common/fixtures/unmap_wrapper_composition_projection.json`, asserted
live by a conformance test). Every refusal row is a *decline*: the process
returns no proposal, the host sees no witness, and the obligation stays
Unknown.

- the producer recomputes the whole composed reading, all eight entry states,
  with the prerequisite's own selected paths distinct across outcomes;
- a changed prerequisite body changes the composed reading rather than being
  ignored — the prerequisite half really is recomputed, not echoed;
- **10 degenerate rows** (pinned): empty, oversized (`MAX_PATHS`,
  `MAX_OPERATIONS`) and operation-free bodies; an anonymous function; a wrong
  signature; a moved revision; the wrapper and prerequisite being the same body;
  an empty prerequisite inventory; a falsified reset value — all decline without
  panicking;
- **4 shape rows** (pinned): a truncated path, duplicated paths, a single path,
  and a prerequisite body with no complete path;
- **7 request-gate rows** (pinned): another profile, another capability, another
  dialect, an absent projection, an unsupported version, an incomplete triple
  and a repeated triple.

### 5.5 End-to-end through the real built package

`providers/conformance/tests/composition_capability.rs` (2 tests). The shipped
`qcore.chunkio` **0.9.0** executable composes the real bodies; the core admits
only after re-establishing all three prerequisites, rebuilding the profile from
its own contract and re-running its own composition. Also measured:

- **fixture drift**: the committed provider fixture is the live projection —
  schema version, both bodies' complete path sets and the declared profile
  compared exactly;
- **missing evidence**: no core checker → Unknown with **QP016** and no
  certificate; no registered provider → Unknown; the shipped `qcore.libevent`
  package, which does not implement this capability → Unknown with no
  certificate; a disabled provider → Unknown;
- **tampered transport**: a projection the core never imported is refused by the
  host's digest binding with **QP001**; rebinding it to a recomputed task digest
  gets past that and is then refused by the core's own comparison, with no
  certificate;
- **false contracts**: a Safe proposal over a falsified *composition* contract is
  rejected with **QP009**, and a Safe proposal over a falsified *prerequisite*
  contract — with the composition's own contract untouched — is rejected too.

**One repair this card made, recorded rather than hidden.** The host's
`verify_v2` binds each transported projection's digest to the core task
(`QP001`); the new projection needed its own arm in that list, which it now has
(`crates/qcore-verifier-host/src/lib.rs`). Without it the tampered-transport row
was caught one layer later, by the core's own comparison (QP009) rather than by
the outer binding. The gate was strengthened, not weakened, and the
conformance test measures both layers.

### 5.6 Wire compatibility

`crates/qcore-schemas/tests/composition_request_wire.rs` (5 tests): a request
carrying none of the **seven** projections serializes with none of the seven
keys and round-trips; the projection round-trips and cannot express a derived
reading; unknown fields are refused in the projection, the triple and the
profile (the policy name is deliberately not a profile member); the request
admission rule is exact (one obligation of its own property for the wrapper it
names, never beside another projection, three complete and distinct triples);
and the completeness and distinctness rules are non-vacuous.

The two existing wire tests' absent-key lists grow to include
`composition_source`.

### 5.7 Lean

`proof/QCore/CompositionAdmission.lean` and
`proof/AuditCompositionAdmission.lean` add **32 audited theorems** in a new
`QCore.CompositionAdmission` namespace, with **no new axiom** — every one
depends on `propext`, `Classical.choice` and `Quot.sound` only.

- *prerequisite revalidation is total and fail-closed* (8): revalidation is a
  decided function on every (context, claim) pair with no partial case, and its
  answer is exactly the decided conjunction; a foreign claim, an incomplete
  claim and a refusing prerequisite each refuse; the core's own identity over an
  evaluating prerequisite is established; each of the three identity fields is
  load-bearing on its own;
- *fail-closed admission* (12): a refusing core, an unbound witness, each of the
  three refusing prerequisites, a profile that is not the declaration, each of
  the two body ties, an incomplete triple, a repeated triple, a foreign triple
  and a differing proposal each refuse on their own;
- *what an admission means* (5): admission implies every conjunct; an admitted
  proposal **is** the core's own reading; two admitted proposals cannot
  disagree; a changed core reading retires the witness; an admission establishes
  all three prerequisites;
- *non-vacuity* (3): the core's own composition over its own prerequisites is
  admitted, and the completeness and distinctness rules really discriminate;
- *no composed result travels* (2): the transported projection **determines no
  reading** — two cores that publish different readings present the very same
  projection — and the same projection is refused by a core that did not
  compose;
- *the chain* (2): an unestablished inner prerequisite refuses the whole chain,
  and an established chain is established.

Judged **not applicable** and recorded rather than modelled: the transported
vocabulary's faithfulness to Clang's typed AST (a gateway/importer premise, as
on every existing source route), and the composition body itself, which is
unedited and whose figures are decided by the Rust tests of §5.1–5.4.

---

## 6. Schema decision — flagged loudly

**`qcore-schemas` moves 9.0.0 → 10.0.0. This is a major schema bump.**

Additive was preferred and is not available. Three of the four additions are
source-incompatible for exhaustive matches and struct literals, exactly as in
4.0.0, 5.0.0, 7.0.0, 8.0.0 and 9.0.0:

| Addition | Kind |
|---|---|
| `SemanticWitness::Composition` | new variant on an exhaustively matched enum |
| `ObligationProperty::CompositionContract` | new variant on an exhaustively matched enum |
| `VerificationRequestV2::composition_source` | new required struct field (skipped when absent) |
| `composition_summary` module | purely additive |

No existing witness variant can hold this reading: `CleanupReplay` carries one
body's prestate cases, `CallResultUse` a dependency fold, `TeardownContract`
alias-class or nullable-state cases, `CounterContract` a single-path unsigned
update. None of them can carry a figure that names *another capability's*
selected path.

**Existing wire bytes are unchanged and pinned.** `composition_source` is an
`Option` that defaults to absent and is skipped on serialization, exactly like
the six projections before it.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 9.0.0 | **10.0.0** |
| `qcore-verification` crate | 0.3.0 | **0.4.0** |
| `qcore-verifier-host` crate | 0.3.0 | **0.4.0** |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (unchanged) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Composition source input | — | 1 |
| Composition semantic profile | — | `qcore.c_unmap_wrapper_composition.v1` |
| Composition input digest key | — | `qcore:composition-source-input:v1` |
| ChunkIO package | 0.8.0 | **0.9.0** |

**Deliberate pin updates**, each with its reason recorded beside the row:

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.8.0 → 0.9.0. The new
   profile is reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other target entry are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version
   in its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/Cargo.lock` — schemas 10.0.0 so the
   pinned standalone-provider fixture's `--locked` build resolves.
4. `crates/qcore-verification/tests/composition_bounds.rs` — the E03-B-O3
   census of production sites naming `PathBounds::BoundedPaths` grows from five
   to six with `composition_source.rs`. **The load-bearing negative half is
   untouched**: no production site states `schedules_explored: Some` or
   `max_operations: Some`, and that assertion still runs. The new site is the
   same kind as the others — a function-scoped source task binding its exact
   complete-path count, here the *wrapper's*, which is the scope its single
   obligation is about — so E03-B-O3's vacuity argument for the *count* kinds is
   unaffected.

All five provider packages were rebuilt and verified. The contract corpus is
untouched at **60 modules** (no `.qcore` file was added or edited).

---

## 7. Corpus and audits

No gateway analysis rule changed: the whole `qcore-c-gateway` diff is one new
module plus its `lib.rs` line, and `legacy_c.rs`, `legacy_summary.rs`,
`typed_source.rs`, `borrowed_actual.rs` and every other analysis file are
untouched. The measured artifacts are reported in §8.1.

---

## 8. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2343 passed, 0 failed, 5 ignored** (+24 new tests) |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **58 audits passed** (57 → 58), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **94 passed, 0 failed** (87 → 94) |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.8.0 → **0.9.0** |
| `python3 -m unittest discover -s scripts/tests` | 125 tests, **1 failed** — see §8.2 |
| 185-unit corpus vs the wave-12 baseline | 3 of 5 artifacts byte-identical; 45 leaves, all attributed — see §8.1 |

Two failures surfaced by the first full run were repaired rather than
tolerated, and both were the card's own consequences:
`compiler/tests/fixtures/w3/provider/Cargo.lock` needed the 10.0.0 schema
requirement for its `--locked` build, and E03-B-O3's `BoundedPaths` census
needed its deliberate pin update (§6, row 4).

### 8.1 The 185-unit corpus, re-measured

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

| Artifact | vs the wave-12 baseline |
|---|---|
| `resource-diagnostics.json` (11 MB) | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` (4 MB) | **byte-identical** |
| `monkey-source.json` (1.6 MB) | **byte-identical** |
| `resource-diagnostics.contracts.json` | 10 leaves, path-literal provenance only |
| `resource-diagnostics.source-effects.json` | 35 leaves, identity/package only |

`chunk-contracts.json` being byte-identical is the load-bearing one: it embeds
the composition's contract text and the `UnmapCertificate` the Cycle 19
compatibility adapter serializes, so a moved certificate, contract or case
would appear there — and the relocation did not move one byte of it.

**Every moved leaf, attributed** (`artifacts/corpus-leaf-diff.txt`; the run log
is `artifacts/corpus-run.log.gz`):

- **`contracts.json` — 10 leaves, one residue class.** The same five contract
  file contents, keyed under this worktree's absolute path instead of the main
  checkout's. Measured: after normalising only the measurement-directory prefix
  in the source keys, the two `sources` maps are **equal**, and every
  non-source member is byte-identical. This is the recorded path-literal
  provenance class (waves 9, 10, 12), not drift.
- **`source-effects.json` — 35 leaves, two residue classes, zero unexplained.**
  All 35 are confined to the three checked source-contract results (16, 24, 63)
  and every one matches a whitelist of identity-only paths: `provider_version`
  0.8.0 → 0.9.0 and its `package_digest` (this card's deliberate package bump),
  and the `configuration_digest` / `build_digest` / `obligation_id` /
  `witness.task_digest` ripple from `current_dir()` being this worktree, which
  re-keys the obligation-keyed `bounds` map and the coverage entry.
  Programmatically checked: **0 unexplained leaves**, and all **282**
  verdict-surface leaves are equal.

No unit verdict, function verdict, diagnostic or case moved in either
direction, and no newly surfaced provisional Unsafe unit exists. The archived
wave-12 baseline is **not** re-pinned by this card; the integrator's corpus
consolidation should expect exactly the two residue classes above, with the
package-version leaves as the only deliberate content.

### 8.2 One Python failure the card cannot fix, by instruction

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails: the committed `docs/reviews/r0_3/acceptance_matrix.json` carries
`['9.0.0', 57]` and the freshly derived value is `['10.0.0', 58]`, because this
card bumps the schema crate and adds one audited Lean module. The card's
constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 9.0.0 →
**10.0.0** and `validation.lean_audits` 57 → **58**, with
`AuditCompositionAdmission.lean` added to `validation.lean_audit_files`.
Regenerate the matrix (or supply `--gate`) at the wave's close, exactly as
waves 9–12 did. Nothing else in the matrix moves from this card: the contract
corpus stays 60 modules, CT coverage is untouched at 31/50, and no verdict
moves.

---

## 9. Open items and named boundaries

- **P04-COMPOSE-A** — `file_close` (both entry points) becomes composable-now
  the moment this card lands: its only unmoved prerequisite was
  `unmap_wrapper`. It is the substantive next unit, and it would be the first
  composition whose prerequisite is *itself* a moved composition, exercising
  the chain rule §5.7 proves.
- **P04-COMPOSE-FRAME** — `memory_close` is blocked on the frame-helper
  `ChunkFrameSummary`, which has an importer and a contract evaluator but no
  verification task, so it cannot be named by a `PrerequisiteIdentity`.
  Standing one up is a capability definition, not a widening of this one. Until
  it exists, `filesystem_chunk_close` and `unsupported_close` stay blocked
  behind it as well.
- **P04-COMPOSE-CALLEE-DECL** — the wrapper's closed callee table is a caller
  assertion, because `unmap_wrapper.qcore` declares no callee vocabulary. It
  buys no authority; declaring it (as P04-B1 did for the cleanup families)
  would make it contract-derived like the prerequisite's.
- **P04-COMPOSE-SHARED-PATHS** — the transported bodies reuse
  `CleanupReplayFunctionRecord` and its step/expression enums. The projector,
  the callee table and the admission are this route's own, and a wire test pins
  the exact variant inventory this route admits so a future widening of the
  shared enums fails there.
- **P04-B2-PREREQ-IDENTITY-CWD** (inherited, unchanged) — a prerequisite
  identity triple is not a content-only function of the body:
  `SourceInputIdentity` binds `current_dir()` by design, so a projection
  generated in one directory carries different triples than the same bodies
  projected in another. The conformance drift test pins this as a measured
  fact.
- **`SourceMatch` (P04-NEXT-C)** — `nested_memory` and `nested_cleanup` stay
  behind the sealed reviewed-library boundary. They need a different design — a
  reviewed-body attestation the core can re-derive — not a bigger vocabulary.
- Nothing about the OS is established. A declared and recorded `munmap` call is
  a recorded call; whether a mapping was torn down is outside every claim here,
  and the sync and logging frames remain explicit unproved external-call
  premises of the pinned contract, recorded in the task's assumptions.
- The in-process adapter is **retained** (hard invariant 3). Its removal would
  need the P09-style archived parity measurement plus a production caller for
  the process route; §5.1 supplies the parity half only — and it is now stronger
  than a parity record, because both routes call one implementation.
- The moved capability has **no production caller yet** — it is exercised by
  these tests, not by the Cycle 19 audit, which still runs the in-process
  adapter. This is the same position P05's moved reactor capability and P04-B2's
  moved cleanup families occupy.
- **P04-COMPOSE-RELOCATE** — the relocation moved the *decision procedure* into
  `qcore-verification`; `file_close`, `memory_close`, `filesystem_chunk_close`,
  `unsupported_close` and the two nested modules remain compiler-side ChunkIO
  adapters. Consolidating them is W5 work that follows their capability moves,
  not this card.
- Of P04-NEXT's original thirteen in-process ChunkIO capabilities, six have now
  moved (counter, list-unlink, field-destructor, call-result def/use, the two
  cleanup-outcome families) plus this card's composition — seven routes across
  five vocabularies. The remaining classification is §1's table.
