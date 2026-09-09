# P04-COMPOSE-MEMCLOSE — the `memory_close` composition through the process boundary

Workstream W5 (Providers), the follow-on named by
[`docs/reports/remaining-p04-frame/README.md`](../remaining-p04-frame/README.md)
§4.2 and §9, and inherited from
[`remaining-p04-compose`](../remaining-p04-compose/README.md) §9 and
[`remaining-p04-compose-a`](../remaining-p04-compose-a/README.md) §9.

`P04-FRAME` supplied the last missing `PrerequisiteIdentity` — in fact three of
the four — so all four of `memory_close`'s dependencies became nameable. This
card moves the capability. It is the **third composition module** to travel and
the first whose bodies are imported from **derived text**.

- Worktree cut from `main` at **`19fbaa5`** (wave-14 consolidation); branch
  `worktree-agent-a119461a3896118c7`. Libevent submodule initialised at the
  pinned revision.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code was
  added, changed or reordered; no `QP`-code was added. No raw unit is promoted,
  no CT case moves, and no `.qcore` file was added or edited, so the contract
  corpus is untouched at **64 modules**.
- The in-process adapter is **retained** (hard invariant 3) and now *delegates*
  to the same relocated decision procedure the moved route runs, so the two
  cannot drift.

---

## 1. Vocabulary decision — extended, and this time additively

**Decision: extend `qcore-schemas::composition_summary`, the fifth transported
source vocabulary. No sixth vocabulary was added.**

The decision was made from the shapes, as the card asked, and it comes out the
same way `P04-COMPOSE-A`'s did but for a stronger reason. `P04-COMPOSE-A`
introduced the family-tagged `CompositionPrerequisiteSet` precisely so that one
family could name three prerequisites and another three-or-four, none of them
meaning the same thing. A four-member set is therefore not a new mechanism: it
is the mechanism that already exists, used once more. And the question this
family asks is the same *kind* — a wrapper replay whose admitted frames are
separately checked prerequisite capabilities — so a sixth module would have
duplicated nine operation forms, eight expression forms and the whole projector
for no distinct refusal surface, which is exactly what `P04-COMPOSE` §2.2
argued against.

**Unlike `P04-COMPOSE-A`, the extension *is* purely additive at the wire.**
Nothing existing is retyped, no existing family's bytes move, and
`COMPOSITION_SOURCE_INPUT_VERSION` **stays 2** with its digest key unchanged —
so the two already-moved composition routes' task digests do not move at all,
and `composition_source.rs`'s pinned ten- and fifteen-key lists needed no
update. The additions are:

| Addition | Kind |
|---|---|
| `CompositionPrerequisiteSet::MemoryClose` | new variant on an exhaustively matched enum |
| `CompositionFamilyInput::MemoryClose` | new variant on an exhaustively matched enum |
| `CompositionProposalRecord::MemoryClose` | new variant on an exhaustively matched enum |
| `MemoryClosePrerequisites`, `DerivedUnitRecord`, `CalleeIdentityRecord`, `MemoryCloseCalleeRecord`, `MemoryCloseCaseRecord`, `MemoryCloseCompositionRecord`, `MEMORY_CLOSE_SEMANTIC_PROFILE` | purely additive |
| `CompositionFamilyInput::native` / `native_profile` return `Option` | source-incompatible accessor change (this family has neither) |

No new `ObligationProperty` and no new `SemanticWitness` variant were needed:
`CompositionContract` is exactly this question's property, and
`SemanticWitness::Composition` already carries the family-tagged proposal. The
host's `verify_v2` QP001 binding for the `composition_source` projection already
exists (added by `P04-COMPOSE` §5.5), so **no host change was needed**.

### 1.1 What the family carries that no other family does

`CompositionFamilyInput::MemoryClose` carries a `wrapper` body, a `probe` body,
a `DerivedUnitRecord` and a `MemoryCloseCalleeRecord`. It carries **no**
prerequisite body and **no** record profile, and that is a deliberate minimality
rather than an omission:

* this family's published reading depends on its prerequisites only through
  *which frames the wrapper may resolve* and *whether their own pinned contracts
  hold*. No prerequisite's reading is a published figure of this composition —
  there is no `native_source_path` and no `unmap_case` here — so no prerequisite
  body needs to travel for the producer to recompute the reading;
* the contracts never travel at all, so the producer could not evaluate them if
  it had the bodies;
* what does travel is the four `(name, declared signature)` pairs the wrapper is
  allowed to resolve. Those are part of the *question*. The core reads its own
  off the prerequisite tasks it derived and refuses a transported pair that is
  not exactly that (§3, row 4).

That is why this is still a composition and not a body replay: the reading is
decided against four separately established capabilities, and §3's decisive rows
measure that a falsified prerequisite contract refuses it.

---

## 2. What actually crosses the boundary — and the derived-text question

`CompositionSourceInput` for a memory-close question carries three things:

1. `schema_version` (still `2`);
2. `prerequisites` — a `MemoryClose` set of **four** named `PrerequisiteIdentity`
   triples (`reset`, `backend`, `unlink`, `counter`), which are lookup keys;
3. one `CompositionFamilyInput::MemoryClose` carrying two bodies' complete typed
   path sets, the derivation record and the four callee identities.

**No composed result and no prerequisite conclusion travels.** No case vector,
no selected path, no checked/uncovered path list, no probed discriminant, no
verdict, no contract text and no policy name. A composed certificate has no wire
representation on this route at all. Tests assert the absence on the real
transported request, in the core suite and end to end through the real package.

### 2.1 The crux: importing from text no file holds

The backend discriminant `cio_chunk_close` branches on is a preprocessor macro.
Rather than guess it, the route appends a constant probe to the unit and imports
`cio_chunk_close` **and** the probe from the resulting text. That derived unit is
not the declared file, so `CSourceInput::for_derived_text` drops a declared
identity.

Two rules make that transport safe, and both are enforced rather than assumed.

**The core derives the text itself.** `derived_source_text` and
`derived_translation_unit` are one pair of helpers in
`qcore_c_gateway::legacy_c`, called by the relocated decision procedure *and* by
the gateway importer, so both derive the same bytes from the same base and the
same recipe. The recipe is a core-side constant supplied through
`MemoryCloseContracts::probe_recipe`. **Derived text never travels in either
direction**, and a producer's account of the derivation is compared against the
core's own, never believed.

**The derived identity cannot stand in for the base import.** This is the
measured finding, and it decides the shape of the task.

> `CSourceInput::for_derived_text` collapses `DeclaredFile` to `AnonymousText`,
> and `input_identity` writes its `qcore.declared-input.v1` block into the
> captured configuration **only** for a declared file. A derived unit therefore
> records neither the spelled input nor the directory the base unit declared:
> **two base units with equal bytes and different declared identities produce
> byte-identical derived identities.**

Measured, not argued. `the_derived_identity_alone_cannot_stand_in_for_the_base_import`
writes the fixture to a temporary directory, imports it once as an anonymous
text and once as a `DeclaredFile`, and asserts that

1. the two **derived** identities serialize byte-identically;
2. the two **base** identities do not;
3. the two tasks differ anyway — because the base import is bound under its own
   key — and neither accepts the other's witness.

A route that bound only the derived identity would have treated those two units
as one task. The same erasure is proved in Lean
(`two_declared_files_with_equal_bytes_derive_equal_derived_identities`,
`the_base_identities_are_not_equal`), so the assumption is visible instead of
implicit.

### 2.2 What happens when the recipe or the base moves — pinned

| Movement | What moves | Where it is refused |
|---|---|---|
| the base text | base digest, derived digest, both identities, the projection, the task digest | at derivation, and the two contexts refuse each other's witness in **both** directions |
| the recipe the projection was derived under | the transported `recipe` field and the derived digest | at derivation: the transported record is not the one the core's own importer measured under the core's own recipe |
| the core's own recipe constant, with an honest projection | the core's own measured derivation | at derivation, in the opposite direction: the same comparison, failing the other way |
| a recipe that appends nothing | nothing | at **import**: `is_a_derivation()` is false, so a plain import can never be presented as a derived one |
| an unused `-D` on the base unit | only the captured configuration | at the task: the projection, the derivation record and every callee identity are byte-identical, and the tasks still differ (§4's isolation row) |

---

## 3. Where the authority is: five things the core does for itself

`MemoryCloseCompositionSourceTask::derive` and `evaluate` do all of the
following before any proposal is compared.

1. **The derivation is the core's own.** The transported record must equal the
   one this import measured, that record's recipe and recipe function must be
   the core's own contract constants, and its two digests must be the ones the
   core's own base and derived identities carry. Every field is checked; none is
   believed.
2. **Every prerequisite is re-established, never read.** For each of the four the
   core re-derives the prerequisite task from its own import, refuses unless the
   claimed triple *is* the identity it derives, and re-runs that prerequisite's
   own contract evaluation. The frame helper is re-established through
   `FrameHelperSourceTask::revalidate_as_prerequisite`, which is the surface
   `P04-FRAME` stood up for exactly this.
3. **The base tie** — new on this route. Every prerequisite must have been
   checked over the very unit the derivation derives from: each prerequisite
   task's own `unit.source_digest` must equal the base import's raw source
   digest. Without it a prerequisite task over an unrelated unit could supply a
   frame identity the wrapper resolves.
4. **The transported callee identities are the core's own reading** of its
   prerequisites' bodies, and must equal them exactly.
5. **Acceptance terminates in the core's own composition.** The core runs the
   unedited relocated procedure over its own imports and its own contracts —
   which re-checks all four prerequisite contracts a *second* time, on its own
   ground — and only then requires the proposal to be that same reading.
   Agreement alone discharges nothing; §4's decisive rows measure both halves.

### 3.1 The published premise set is the union, per E03-B

`composition_premise_union` is reused unchanged: the published set is the union
of all four components' premise sets and this route's own, and a key two
components disagree on refuses rather than being silently resolved. This route's
own premises are `libc_free_bindings_are_declared_premises`,
`derived_probe_text_preprocesses_as_the_base_unit_does` and
`backend_discriminant_is_read_from_a_source_probe` — the second being the card's
own: the bodies were read out of text this repository derived, and that the
derived text preprocesses as the base unit does is a **declared step, not a
proved one**. A test asserts every component premise survives and that the three
above are a strict superset.

### 3.2 Digest keys the task binds

Fifteen keys, and a test pins that they are exactly the task's
`contract_sources` with fifteen distinct 64-hex digests:

| Key | Binds |
|---|---|
| `qcore:composition-source-input:v2` | the complete projection |
| `qcore:composition-memory-base-import:v1` | the **base** unit's captured identity |
| `qcore:composition-memory-derived-import:v1` | the **derived** unit's captured identity |
| `qcore:composition-memory-derivation:v1` | the derivation record the core measured |
| `qcore:composition-memory-close-contract:v1` | the composition's own pinned contract |
| `qcore:composition-memory-frame-contract:v1` | the frame prerequisite's contract |
| `qcore:composition-memory-backend-contract:v1` | the destructor prerequisite's contract |
| `qcore:composition-memory-unlink-contract:v1` | the unlink prerequisite's contract |
| `qcore:composition-memory-counter-contract:v1` | the counter prerequisite's contract |
| `qcore:composition-memory-counter-binding:v1` | the exact counter field and delta this composition requires, neither of which is in the counter contract |
| `qcore:composition-memory-callees:v1` | the four callee identities the core derived |
| `qcore:composition-prerequisite-memory-{reset,backend,unlink,counter}:v1` | the four prerequisite identities, one key each |

Two import keys, not one, for the §2.1 reason. The `UnitIdentity` names the
**base** unit (`source_digest` is the file the audit selected, not the text the
recipe made) while `build_digest` binds *both* identities, so neither can move
invisibly.

---

## 4. Evidence

### 4.1 Byte-identical certificate parity with the retained in-process route

`compiler/tests/memory_close_route_parity.rs` (3 tests).

| Check | Result |
|---|---|
| in-process `check_memory_close_source` certificate vs moved `evaluate()` certificate, as **serialized bytes** | **byte-identical** |
| the reading is a real composition | 2 checked entry states, 2 explicitly uncovered paths |
| the committed C fixture still carries every shape all three routes depend on | asserted |

The fixture is now one committed file,
`compiler/tests/fixtures/chunkio/memory_close_source.c`, which the retained unit
suite's `FIXTURE`, this parity test and the core-side suite all `include_str!`,
so the three cannot drift onto different source.

**Six refusal-agreement rows**, count pinned: an extra null guard, a repeated
primary unlink, the wrong counter, the two frees transposed, the backend
destructor call removed, and the chunk release dropped. Each row asserts the
retained route really refuses **and** that the moved route refuses it too.
Nothing in this card makes a previously refused body admissible.

### 4.2 Core-side positive suite — 12 tests

`crates/qcore-verification/tests/memory_close_composition_source.rs`, driving
the real gateway imports, the real four prerequisite task derivations and the
real trusted composition:

- the fifteen bound-input keys are exactly the task's contract sources, with
  fifteen distinct digests;
- exactly one obligation, `CompositionContract` under `ResourceContracts`, with
  its exact subject, scope and `PathBounds::BoundedPaths`;
- the unit identity names the base unit and the derivation really derives;
- the core reads two entry states, publishes two uncovered paths and reads
  `memory_kind = 1` out of its own probe import; `recheck` reproduces the
  certificate;
- the request carries the question and none of ten named derived figures, and
  the other six projections stay absent;
- the published premise set is the union of all four components' and this
  route's own three;
- the prerequisite identity is the three-field key, each field load-bearing, and
  re-establishment recomputes rather than returning a stored reading;
- the transported question names the memory-close set and no other, carries no
  nested question, no native body and no record profile, and reports nesting
  depth 1;
- deriving twice gives an equal task, identity and certificate;
- the checker admits the core's own reading and refuses a foreign task first;
- **`the_derived_identity_alone_cannot_stand_in_for_the_base_import`** (§2.1).

### 4.3 Core-side mutation matrix — 8 tests, **80 rows, all refused**

`crates/qcore-verification/tests/memory_close_composition_mutations.rs`. Every
forged reading is asserted to actually differ from the honest one **before** it
is submitted, and every row count is pinned so a row cannot silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed reading | **29** (pinned) | function, revision, signature; the probed discriminant inflated and zeroed; cases emptied / dropped / duplicated / reordered; each entry's `nonnull` flipped; the selected path retargeted; the call count inflated and erased; each of the six release flags erased on the nonnull entry and claimed on the null one; checked paths emptied, widened and reordered; uncovered paths emptied, widened and claimed checked; an invented profile assumption |
| Family and witness binding | **3** (pinned) | a **leaf-family** proposal against a memory-close task; a witness rebound to a foreign task digest; another witness kind entirely |
| Prerequisite forgeries | **11** (pinned) | each of the four identities emptied; two repeated in two different pairs; a foreign identity (another counter body's own task) in each of the four slots; two real identities exchanged |
| **Decisive prerequisite rows** | **4** (pinned) | an *honest* identity set, an honest derivation and an untouched composition contract, over a prerequisite whose **own** pinned contract is falsified — once for each of the four |
| **Derived text** | **7** | substituted base text refused in **both** directions (neither context accepts the other's witness); three substituted recipes; the core's own recipe moved under an honest projection; a recipe that derives nothing |
| Callee substitution | **7** (pinned) | each of the four callees renamed; two resigned; two callees collapsed onto one frame |
| Every clause of the composition contract | **19** (pinned) | all eight `ensures normal` clauses falsified two ways, plus three `requires` premises withdrawn |
| Isolation | 1 (+2 witness refusals) | an unused `-D` leaves the projection, the derivation record and every callee identity byte-identical and still retires the task |

**No mutation was admitted. No gate was changed and no repair was required by
the matrix.**

Two measured corrections the matrix forced, both kept rather than explained
away:

* **falsifying the *first* `ensures normal` clause of a prerequisite contract is
  not always load-bearing.** `counters.qcore` declares several functions, and
  the first clause belongs to one this composition does not read. The row was
  corrected to falsify **every** `ensures normal` clause of the contract under
  test rather than the assertion weakened; the helper says so in its own doc
  comment.
* **the counter frame's call-site prototype is not its definition's.** The
  pinned ChunkIO counters return a `size_t` typedef, and Clang records the sugar
  at the call site and the canonical spelling at the definition, so an exact
  `(name, signature)` table refused the honest body. The trusted replay already
  compares name, argument list and result type separately for exactly this
  reason; the projector now derives the admitted counter pair from the body
  **under the core's own rule** (`counter_call_identity`). It buys no authority
  — the core re-runs its own replay over its own import regardless, and a call
  that fails the rule has no projection at all — and it is recorded as boundary
  `MEMCLOSE-COUNTER-SUGAR`.

### 4.4 Provider-side matrix — 3 tests, 24 refusal rows

`providers/common/composition_effect_tests.rs`, driven over the **real**
projection the gateway emits for these bodies
(`providers/common/fixtures/memory_close_composition_projection.json`, asserted
live by a conformance test). Every refusal row is a *decline*: the process
returns no proposal, the host sees no witness, and the obligation stays Unknown.

- the producer recomputes the whole reading — both entry states, the null entry
  resolving no frame at all and the memory entry resolving all seven, the two
  entries selecting different paths, and `memory_kind` read out of the **probe
  body** rather than received;
- a changed probe body changes the reading rather than being ignored, which is
  what makes the probe a question and not an answer;
- **15 degenerate rows** (pinned): either body emptied or wrongly signed; the
  two bodies identical; a revision disagreement; a derivation that derives
  nothing, is incomplete, or names another function than the probe; two callees
  collapsed; a callee identity emptied; a callee renamed so the wrapper resolves
  nothing; an oversized body; a truncated path; duplicated paths so an entry
  would select two — all decline without panicking;
- **9 request-gate rows** (pinned): another profile; another family's question
  under this profile; this family's question under another family's profile; no
  projection; another capability; an unadmitted dialect; an unsupported version;
  an incomplete identity; a repeated identity.

### 4.5 End-to-end through the real built package — 2 tests

`providers/conformance/tests/memory_close_composition_capability.rs`. The shipped
`qcore.chunkio` **0.11.0** executable composes the real bodies; the core admits
only after re-establishing all four prerequisites, checking the transported
derivation against its own and re-running its own composition. Also measured:

- **fixture drift**: the committed provider fixture is the live projection —
  schema version, both bodies' complete path sets, all four callee identities
  and, **exactly**, the derivation record, which unlike a prerequisite identity
  *is* a function of the text alone. The four triples are checked for shape and
  semantic profile only, under the inherited boundary
  `P04-B2-PREREQ-IDENTITY-CWD`;
- **missing evidence**: no core checker → Unknown with **QP016** and no
  certificate; no registered provider → Unknown; the shipped `qcore.libevent`
  package, which does not implement this capability → Unknown with no
  certificate; a disabled provider → Unknown;
- **tampered transport**: a projection the core never imported is refused by the
  host's digest binding with **QP001**; rebinding it to a recomputed task digest
  gets past that and is then refused by the core's own comparison, with no
  certificate;
- **the derived-text row end to end**: a projection derived under a substituted
  recipe never reaches a task at all;
- **false contracts**: a Safe proposal over a falsified *composition* contract is
  rejected with **QP009**, and one over a falsified *prerequisite* contract —
  with the composition's own contract untouched — is rejected too.

### 4.6 Wire compatibility

`crates/qcore-schemas/tests/composition_request_wire.rs` grows from 9 to **14
tests**. The memory-close question round-trips and carries none of thirteen
named derived figures; a derivation whose derived digest equals its base digest
is not a derivation and each of its six fields is required; the family never
borrows another family's prerequisite set in either direction, against both
other families; unknown fields are refused in all four new records; the four
identity rules are non-vacuous and **all eleven prerequisite digest keys across
the three families are distinct**.

**One deliberate pin update.** `composition_vocabulary_is_pinned`'s family-tag
list grows from two entries to three. **Its load-bearing negative halves are
untouched**: the nine transported operation forms and eight expression forms are
asserted unchanged, so this card widened **no** shared enum — the boundary
`P04-COMPOSE-SHARED-PATHS` asked for exactly that check and it fired correctly.

### 4.7 Lean — 32 audited theorems, no new axiom

`proof/QCore/DerivedTextAdmission.lean` and
`proof/AuditDerivedTextAdmission.lean` add the `QCore.DerivedTextAdmission`
namespace. Every theorem depends only on `propext` and `Quot.sound`; eight
depend on nothing at all. `check_lean_audits.py` moves 64 → **65** audits.

- *the measured erasure* (5): two declared files with equal bytes derive equal
  derived identities; a declared file and an anonymous text derive one identity;
  the base identities of those two units are **not** equal; the historical
  standard-input class is the one `for_derived_text` does not collapse; a
  derived identity reads only the text and the recipe;
- *a derivation is a change* (2): a nonempty recipe derives new text, an empty
  one does not;
- *fail-closed admission* (14): admission is decided; a refusing core, a foreign
  derivation, a substituted recipe, substituted base text, a derivation that
  derives nothing, an incomplete derivation, each of the **four** refusing
  prerequisites, a foreign identity set, **a prerequisite checked over another
  unit**, and a differing proposal each refuse on their own;
- *what an admission means* (5): admission implies every conjunct; an admitted
  proposal **is** the core's own reading; two admitted proposals cannot
  disagree; an admission establishes all four prerequisites; an admitted
  derivation names the text the core derived;
- *the two substitution directions are separated* (3): substituting the base and
  substituting the recipe each move the derived digest, and the record's own
  `recipe` field separates two recipes even where texts would collide;
- *non-vacuity* (3): the honest proposal is admitted, the honest derivation
  records the erasure (`declared_file` base, `anonymous_text` derived), and each
  of four refusals really refuses on that same honest context.

Judged **not applicable** and recorded rather than modelled, unchanged from
`P04-COMPOSE`: the transported vocabulary's faithfulness to Clang's typed AST (a
gateway/importer premise), whether the derived text preprocesses as the base
unit does (a published assumption), and the composition body itself, which is
unedited and whose figures are decided by the Rust tests of §4.1–4.5.

---

## 5. One decision procedure, not two

`memory_close`'s decision procedure — the eleven-constructor `Value` domain, the
`Replay`, `check_contract`, `probe_literal`, `filesystem_path`,
`unsupported_path` and `MemoryCloseCertificate::instantiate` — is **relocated
verbatim** into `crates/qcore-verification/src/memory_close_composition.rs`. The
only changes to the moved body are:

* the constants it read from its former module scope (the pinned revision, the
  probe recipe and probe function, the composition's own contract and the four
  prerequisite contracts with their declared root types, and the required
  counter field and delta) are now explicit parameters (`MemoryCloseContracts`);
* the string literals it matched callees and signatures against are now named
  public constants (`RELEASE_CALLEE`, `FILESYSTEM_CALLEE`, `WRAPPER_SIGNATURE`,
  `PROBE_SIGNATURE`, `RESET_SIGNATURE`) with the same values, so the gateway
  projector and the trusted replay cannot disagree about which frames exist;
* the import half is separated from the decision half
  (`import_memory_close_bodies` / `read_memory_close_bodies`), and one internal
  `read` returns the certificate **and** the per-entry detail, so the transported
  route compares a richer reading without a second procedure computing it;
* one added defensive check, recorded rather than hidden: the imported probe's
  function name must be the recipe's declared function. It is a tautology on the
  honest route, because the import selects by name.

`compiler/src/verifier_plugins/chunkio/memory_close.rs` becomes an **84-line**
adapter (from 717) that supplies the ChunkIO half and delegates. Its public API
is preserved: `CONTRACT`, `check_memory_close_source`, `probe_literal`,
`filesystem_path`, `unsupported_path` and `MemoryCloseCertificate` /
`MemoryCloseDependencies` re-exported at the same paths, so
`filesystem_chunk_close`, `unsupported_close`, `nested_memory`, `nested_cleanup`
and the pinned Cycle 19 compatibility adapter are **untouched**.

`MemoryCloseCertificate::recheck()` keeps its argument-free signature by
retaining the owned contract selection in a `#[serde(skip)]` field, so the three
existing callers did not change and **the certificate's serialized shape is
byte-identical** — which the retained adapter's archived corpus artifacts depend
on and §4.1 measures.

Nothing else moved and nothing was duplicated:
`crates/qcore-verification/src/{unmap_composition,file_close_composition,cleanup_outcome,counter_contract,call_use_contract,field_destructor,list_unlink_contract,frame_source}.rs`
are otherwise unedited, and `teardown_source.rs` gains only two additive
`summary()` accessors.

---

## 6. The gateway diff is two modules

The whole `crates/qcore-c-gateway` change is `composition_source.rs` plus 58
additive lines in `legacy_c.rs` (`CSourceInput::class`, `derived_source_text`,
`derived_translation_unit`). `legacy_c.rs`'s analysis body, `legacy_summary.rs`,
`typed_source.rs`, `borrowed_actual.rs`, `aggregate_store.rs`, `may_alias.rs` and
every other analysis file are untouched. **No gateway analysis rule changed, so
no unit verdict, function verdict or diagnostic can move from this card.**

The existing `import_composition_source` explicitly **refuses** the memory-close
family before importing anything, because that family must build its own text
first; the two are separate importers by construction, not by convention.

---

## 7. Schema decision — flagged loudly

**`qcore-schemas` moves 11.0.0 → 12.0.0. This is a major schema bump.**

Additive was preferred and, at the **wire**, was achieved: no existing
projection's bytes move, `COMPOSITION_SOURCE_INPUT_VERSION` stays 2 and the
digest key stays `…:v2`, so no existing task digest moves from this card. What
forces the major version is the Rust API, under the same documented policy as
4.0.0, 5.0.0, 7.0.0–11.0.0: three new variants on exhaustively matched enums and
two accessors that now return `Option` because the new family has neither a
prerequisite body nor a record profile.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 11.0.0 | **12.0.0** |
| `qcore-verification` crate | 0.4.0 | 0.4.0 (additive API) |
| `qcore-verifier-host` crate | 0.4.0 | 0.4.0 (no host change) |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (every published name still resolves, now as a re-export — the precedent `P04-COMPOSE` and `P04-COMPOSE-A` set) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Composition source input | 2 | 2 (**unchanged**) |
| Composition semantic profiles | 3 | **4** |
| ChunkIO package | 0.10.0 | **0.11.0** |
| Contract corpus | 64 modules | 64 modules (unchanged) |

**Deliberate pin updates**, each with its reason:

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.10.0 → 0.11.0. The new
   profile is reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other manifest field are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version in
   its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/Cargo.{toml,lock}` and
   `providers/Cargo.lock` — schemas 12.0.0 so the pinned standalone-provider
   fixture's `--locked` build resolves.
4. `crates/qcore-schemas/tests/composition_request_wire.rs` — the family-tag list
   grows by one (§4.6), with both load-bearing negative halves kept.

All five provider packages were rebuilt and verified.

---

## 8. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | see §8.1 |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **65 audits passed** (64 → 65), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | see §8.1 |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.10.0 → **0.11.0** |
| `python3 -m unittest discover -s scripts/tests` | one failure, §8.3 |
| Canonical CMetrics audit, before vs after | §8.2 |

### 8.1 Test counts

(filled in from the merged run; see the integrator summary)

### 8.2 The corpus and the canonical audit

No gateway analysis rule changed (§6) and the retained in-process entry point
publishes a byte-identical certificate (§4.1), so the pinned 185-unit artifacts
cannot move from this card. The measured statement is in §8.2 of the gate table
above.

The four pinned 185-unit artifacts were **not** re-measured here: that needs a
prepared pinned upstream clone and the audit's own pin gate refuses without one.
This is the same position `P04-COMPOSE-A`, `P04-B2` and several earlier cards
recorded. The integrator's corpus consolidation should expect **no change from
this card** beyond the package-identity residue class — the ChunkIO
`provider_version` and `package_digest` leaves in `source-effects.json`, as in
every card that bumps that package — plus the schemas-12.0.0 package digests.

### 8.3 One Python failure the card cannot fix, by instruction

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails: the committed `docs/reviews/r0_3/acceptance_matrix.json` carries
`['11.0.0', 64]` and the freshly derived value is `['12.0.0', 65]`, because this
card bumps the schema crate and adds one audited Lean module. The card's
constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 11.0.0 →
**12.0.0** and `validation.lean_audits` 64 → **65**, with
`AuditDerivedTextAdmission.lean` added to `validation.lean_audit_files`.
Regenerate the matrix (or supply `--gate`) at the wave's close, exactly as waves
9–14 did. Nothing else in the matrix moves from this card: the contract corpus
stays 64 modules, CT coverage is untouched at 32/50, and no verdict moves.

### 8.4 One pre-existing failure repaired in passing, recorded loudly

`cargo clippy --workspace --all-targets -- -D warnings` **fails at the untouched
base commit `19fbaa5`** on this host's toolchain (Rust 1.98 Clippy):
`crates/qcore-c-gateway/tests/prod_contracts.rs:699` triggers
`clippy::manual_contains`, a lint that post-dates the file. The file is untouched
by this card's subject matter. The fix applied is the mechanical, semantically
identical `iter().any(|t| *t == X)` → `contains(&X)`; **no assertion was
weakened and no expectation was deleted**. Recorded here rather than left as a
red gate the integrator would have to diagnose.

---

## 9. Open items and named boundaries

- **`MEMCLOSE-COUNTER-SUGAR`** (new) — the counter frame's call-site prototype
  may spell a typedef differently than its definition, so the projector derives
  the admitted counter pair from the wrapper body under the trusted
  composition's own three-part rule (name, argument list, result type) rather
  than from an exact signature table. It buys no authority — the core re-runs
  its own replay regardless — and a call that fails the rule has no projection.
  Declaring the callee vocabulary in `memory_close.qcore`, as `P04-B1` did for
  the cleanup families, would make the whole table contract-derived; that is the
  same follow-on as `P04-COMPOSE-CALLEE-DECL`, now covering three wrappers.
- **`MEMCLOSE-DERIVED-PREPROCESSING`** (new) — that the derived text
  preprocesses as the base unit does is a **declared premise**, published in the
  task's assumptions as
  `derived_probe_text_preprocesses_as_the_base_unit_does`. The derivation
  appends to the end of the unit, so a base unit ending inside a conditional
  group, an unterminated comment or a macro continuation would derive text that
  is not "the base plus a probe" in any useful sense. Nothing here detects that;
  the recipe is a pinned constant over a pinned unit and the premise is recorded
  rather than discharged.
- **`MEMCLOSE-PROBE-ISOLATION`** (new) — the probe defines a function in the
  derived unit's own namespace. A base unit that already defined that name would
  make the derived text ill-formed and the import would refuse, but nothing
  *proves* the name is unused; that is the same declared-premise class as above.
- **`MEMCLOSE-BASE-TIE-TEXT`** (new) — the base tie (§3, row 3) compares each
  prerequisite task's `unit.source_digest` against the base import's raw source
  digest, so it ties the prerequisites to the same **text**, not to the same
  build configuration. A prerequisite context the core built over the same text
  under different preprocessing would satisfy the tie; it would still be a
  different `PrerequisiteIdentity`, and the identity comparison in row 2 is what
  refuses it. Recorded because the tie's strength is text-level and should not
  be read as more.
- **`P04-FRAME-NO-WITNESS`** (inherited, now load-bearing) — the frame helper
  capability has no transported witness form, so this route's `reset`
  prerequisite is discharged only by the core's own `evaluate`, reached through
  `revalidate_as_prerequisite`. That is exactly how it is used here, and it is
  why the frame summary — not a certificate — is what the composition consumes.
- **`P04-FRAME-OBLIGATION-ID`** (inherited, unchanged) — the shared
  obligation-identifier scheme does not read `contract_sources`, so it is sound
  only via the identity triple. This route names four prerequisites by triple.
- **`P04-B2-PREREQ-IDENTITY-CWD`** (inherited, unchanged) — a prerequisite
  identity is not a content-only function of the body. The conformance drift
  test pins this for all four, while comparing the derivation record **exactly**,
  because that record *is* a function of the text.
- **`P04-COMPOSE-SHARED-PATHS`** (inherited, unchanged) — the transported bodies
  reuse `CleanupReplayFunctionRecord` and its step/expression enums. This card
  widened neither, and §4.6's pin asserts it.
- **`P04-COMPOSE-FS`** (inherited, now unblocked) — `filesystem_chunk_close` was
  blocked on `memory_close` and `file_close`; both have now moved, so it becomes
  composable-now, with `unsupported_close` one level above it. Both would need a
  further extension of this vocabulary: `filesystem_chunk_close` composes a
  *memory-close certificate* with a `file_close` deleting certificate and its own
  filesystem-kind probe, which is a second derived-text import in the same
  question.
- **`SourceMatch` (P04-NEXT-C)** (inherited, unchanged) — `nested_memory` and
  `nested_cleanup` stay behind the sealed reviewed-library boundary. Their
  removal needs a reviewed-body attestation the core can re-derive, not a bigger
  vocabulary.
- **`filesystem_chunk_close` and `unsupported_close`, restated.** Both remain
  **in-process only**, and both remain reachable through this card's retained
  adapter: they consume `MemoryCloseCertificate` through `recheck()`,
  `memory_kind()`, `filesystem_path()` and `unsupported_path()`, every one of
  which this card preserved at its existing path with its existing behaviour.
  Neither is moved, neither is promoted, and the classification in
  `P04-COMPOSE` §1 is otherwise unchanged.
- **Nothing about the OS is established.** A recorded `free` is a recorded call;
  whether storage was reclaimed, whether the backend's own allocations were
  released and whether the chunk's context outlives it are the pinned contract's
  declared premises, published in the task's assumptions.
- The in-process adapter is **retained** (hard invariant 3). Its removal would
  need the P09-style archived parity measurement plus a production caller for
  the process route; §4.1 supplies the parity half only — and it is stronger
  than a parity record, because both routes call one implementation.
- The moved capability has **no production caller yet** — it is exercised by
  these tests, not by the Cycle 19 audit, which still runs the in-process
  adapter. Same position as P05, P04-B2, P04-COMPOSE and P04-COMPOSE-A.
- Of `P04-NEXT`'s original thirteen in-process ChunkIO capabilities, six
  non-composition capabilities have moved plus **three composition modules**;
  the remaining classification is `P04-COMPOSE` §1's table with `file_close` and
  `memory_close` struck off and `filesystem_chunk_close` promoted to
  composable-now.
