# P04-COMPOSE-FS — the `filesystem_chunk_close` composition through the process boundary

Workstream W5 (Providers), the follow-on named by
[`docs/reports/remaining-memclose/README.md`](../remaining-memclose/README.md)
§9 ("`filesystem_chunk_close` … becomes composable-now … needing two
derived-text imports in one question") and inherited from
[`remaining-p04-compose`](../remaining-p04-compose/README.md) §1's census row,
which classified it *blocked by dependency* on both `P04-COMPOSE-FRAME` and
`P04-COMPOSE-A`. Both of those are now closed, so this card moves the
capability.

It is the **fourth composition module** to travel, the first with **two derived
units in one question**, and the first whose **every** prerequisite is itself a
composition — checked over **two different translation units**.

- Worktree cut from `main` at **`8069a99`** (wave-15 consolidation); branch
  `worktree-agent-abd9a36c8e7296f94`. Libevent submodule initialised at the
  pinned revision.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code was
  added, changed or reordered; no `QP`-code was added. No raw unit is promoted,
  no CT case moves, and no `.qcore` file was added or edited, so the contract
  corpus is untouched at **67 modules**.
- The in-process adapter is **retained** (hard invariant 3) and now *delegates*
  to the same relocated decision procedure the moved route runs, so the two
  cannot drift.

---

## 1. Vocabulary decision — extended again, and again additively

**Decision: extend `qcore-schemas::composition_summary`, the fifth transported
source vocabulary. No sixth vocabulary was added.**

The decision was made from the shapes, as the lineage requires, and it comes out
the same way `P04-COMPOSE-A`'s and `P04-COMPOSE-MEMCLOSE`'s did. The question is
the same *kind* — a wrapper replay whose admitted frames are separately checked
prerequisite capabilities — and the family tagging `P04-COMPOSE-A` introduced
already carries a set of a different size and shape per family. A two-member set
whose both members are compositions is not a new mechanism; it is the mechanism
that already exists, used once more.

**The extension is purely additive at the wire.** Nothing existing is retyped,
no existing family's bytes move, and `COMPOSITION_SOURCE_INPUT_VERSION` **stays
2** with its digest key unchanged — so the three already-moved composition
routes' task digests do not move at all. The additions are:

| Addition | Kind |
|---|---|
| `CompositionPrerequisiteSet::FilesystemChunkClose` | new variant on an exhaustively matched enum |
| `CompositionFamilyInput::FilesystemChunkClose` | new variant on an exhaustively matched enum |
| `CompositionProposalRecord::FilesystemChunkClose` | new variant on an exhaustively matched enum |
| `FilesystemChunkClosePrerequisites`, `FilesystemChunkCloseCaseRecord`, `FilesystemChunkCloseCompositionRecord`, `FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE`, `CompositionFamilyInput::probe()` | purely additive |
| `CompositionFamilyInput::nested()` returns `Vec<&Self>` | source-incompatible accessor change (§1.2) |

No new `ObligationProperty` and no new `SemanticWitness` variant were needed:
`CompositionContract` is exactly this question's property, and
`SemanticWitness::Composition` already carries the family-tagged proposal. The
host's `verify_v2` QP001 binding for the `composition_source` projection already
exists (added by `P04-COMPOSE` §5.5), so **no host change was needed**.

### 1.1 What the family carries that no other family does

`CompositionFamilyInput::FilesystemChunkClose` carries **no body of its own but
the probe**: two nested questions verbatim, one `CleanupReplayFunctionRecord`
for the filesystem discriminant probe, and one `DerivedUnitRecord`.

Its wrapper is the nested memory question's wrapper — the same body, replayed on
a different arm under a different semantic profile. Restating it here would have
let the two halves of one question name two different bodies, so
`CompositionFamilyInput::wrapper()` delegates to the nested memory question for
this family, and a test pins that the two are the same record.

### 1.2 One accessor had to change shape, and why

`nested()` returned `Option<&Self>` because every family until now had at most
one composition prerequisite. This family has two. A single-slot accessor would
have had to publish one of them and hide the other — precisely the kind of
silent narrowing these vocabularies exist not to do — so `nested()` now returns
a `Vec<&Self>`, empty on a leaf family, and `nesting_depth()` is one more than
the deepest of them. All seven existing call sites moved with it; both pinned
"the nested question is exactly the leaf family's own projection" assertions are
kept, now as vector equalities.

---

## 2. What actually crosses the boundary

`CompositionSourceInput` for a filesystem question carries three things:

1. `schema_version` (still `2`);
2. `prerequisites` — a `FilesystemChunkClose` set of **two** named
   `PrerequisiteIdentity` triples (`memory_close`, `file_close_delete`), which
   are lookup keys;
3. one `CompositionFamilyInput::FilesystemChunkClose` carrying the whole nested
   memory question, the whole nested deleting `cio_file_close` question, this
   family's own probe body and this family's own derivation record.

**No composed result and no prerequisite conclusion travels, at any of the three
levels.** No case vector, no selected path, no case index, no probed
discriminant, no residual classification, no deletion outcome, no verdict, no
contract text and no policy name. A composed certificate has no wire
representation on this route at all. Tests assert the absence on the real
transported request, in the core suite and end to end through the real package.

### 2.1 The crux: two derived units in one question

The memory-close family already imports its wrapper and its probe from text
derived by appending a constant recipe to the unit. This family composes *that*
certificate with a **second** derived import of its own: the filesystem
discriminant `cio_chunk_close` branches on is a different preprocessor macro, so
it is read out of a second probe under a second recipe over the same base text.

Three rules make that transport safe, and all three are enforced rather than
assumed.

**The core derives both texts itself.** `derived_source_text` and
`derived_translation_unit` — the one pair of helpers `P04-COMPOSE-MEMCLOSE`
stood up — are called by the relocated decision procedure *and* by the gateway
importer, for both recipes. Both recipes are core-side constants supplied
through `FilesystemChunkCloseContracts::probe_recipe` and the memory family's
own `MemoryCloseContracts::probe_recipe`. **Derived text never travels in either
direction**, and a producer's account of either derivation is compared against
the core's own, never believed.

**The two derivations must be two.** The transported pair must derive from one
base text, under two recipes that differ, defining two functions that differ,
into two texts that differ. This is a checked conjunct, not an assumption, and
the reason is concrete:

> A filesystem probe derived under the **memory** recipe returns
> `CIO_STORE_MEM`. The wrapper's guard would then select the memory arm while
> the reading claimed the filesystem one, and the two arms would collapse into
> one. §4.3's derived-text group measures that this is refused, in the importer
> and again at the task.

**The derived identity still cannot stand in for the base import.** The
`MEMCLOSE-BASE-TIE-TEXT` erasure is inherited unchanged: `for_derived_text`
collapses `DeclaredFile` to `AnonymousText`, so this family also binds **two**
import keys — its base import and its own derived import — and the task's
`UnitIdentity` names the base unit while its `build_digest` binds both.

### 2.2 The second crux: the two prerequisites are in different units

This is the measured finding that shapes the task, and it is a genuine
difference from every earlier composition.

> `filesystem_chunk_close` composes a `memory_close` certificate — over the
> **chunk** unit — with a deleting `cio_file_close` certificate — over the
> **file** unit. The retained in-process route has always done this: its unit
> test builds the backend half from `file_close_source.c` and the memory half
> from `memory_close_source.c`, and the Cycle 19 compatibility adapter builds
> the backend half from `cio_file.c` and the memory half from `cio_chunk.c`.

So this family **cannot** carry the memory family's single-unit base tie, and
saying it did would have been false. What the task requires instead is:

* the **memory** prerequisite's own `unit.source_digest` **must** equal the base
  import's raw source digest — the same tie the memory family has;
* the **backend** prerequisite's own `unit.source_digest` **must not** — a
  backend composition over the wrapper's own text is not the cross-unit question
  this family asks, and is refused rather than silently accepted as a stronger
  tie;
* what ties the backend prerequisite is therefore its own
  `PrerequisiteIdentity`, its declared source revision (compared against the
  memory half's) and its declared prototype (compared against
  `WRAPPER_SIGNATURE`), each re-derived by the core.

That the linker resolves the wrapper's `cio_file_close` frame to the body
checked in the *other* unit is **not** established by any of this. It is a
declared premise, published in the task's assumptions as
`backend_prerequisite_is_linked_from_another_translation_unit` and recorded as
boundary **`P04-COMPOSE-FS-CROSS-UNIT`**.

### 2.3 What happens when a recipe, a base or a prerequisite moves — pinned

| Movement | What moves | Where it is refused |
|---|---|---|
| the base text | both derived digests, both identities, the projection, the task digest | at derivation, and the two contexts refuse each other's witness in **both** directions |
| this family's recipe | the transported `recipe` field and this family's derived digest | at derivation: the transported record is not the one the core's own importer measured under the core's own recipe |
| this family's recipe replaced by the **memory** family's | the derived digest, and the separation | at **import**: the two recipes must differ, so the collapse never reaches a task |
| the memory family's recipe | the nested derivation record | at the nested-derivation comparison, which is bound under its own key |
| a recipe that appends nothing | nothing | at **import**: `is_a_derivation()` is false |
| either prerequisite's own pinned contract | that prerequisite's task digest, and its own evaluation | at re-establishment: the identity moves, and if the identities are made consistent the falsified contract refuses at evaluation (§4.3's decisive rows) |
| an unused `-D` on the base unit | only the captured configurations | at the task: the projection, both derivation records and both nested questions are byte-identical, and the tasks still differ (§4.3's isolation row) |

---

## 3. Where the authority is: seven things the core does for itself

`FilesystemChunkCloseCompositionSourceTask::derive` and `evaluate` do all of the
following before any proposal is compared.

1. **This family's derivation is the core's own.** The transported record must
   equal the one this import measured, its recipe and recipe function must be
   the core's own contract constants, and its two digests must be the ones the
   core's own base and derived identities carry.
2. **The nested derivation is the core's own too.** It must equal the record the
   core's own memory-close context measured — a second comparison under a second
   digest key, so a moved memory recipe retires this task even where this
   family's own recipe did not move.
3. **The two derivations are separate.** One base text, two recipes, two recipe
   functions, two derived texts. Checked in `derive` and again in the importer.
4. **Both prerequisites are re-established, never read.** For each, the core
   re-derives the prerequisite task from its own import, refuses unless the
   claimed triple *is* the identity it derives, and re-runs that composition's
   own whole evaluation — which re-establishes its own prerequisites in turn
   (four for the memory half, three plus a nested composition for the backend
   half). `FileCloseCompositionSourceTask` gains
   `prerequisite_identity`/`revalidate_as_prerequisite` for exactly this, and
   the latter **refuses the non-deleting arm**: this question crosses both case
   vectors and only the deleting entry point publishes both.
5. **Both transported nested questions are the core's own.** Each must be,
   verbatim, the family input its own composition context imported — the same
   tie `P04-COMPOSE-A` established for its one nested question, applied twice
   and bound under two separate keys.
6. **The unit tie, in both directions** (§2.2).
7. **Acceptance terminates in the core's own composition.** The core runs the
   unedited relocated procedure over its own certificates and its own contracts
   — which re-runs *both* prerequisite compositions a second time, on its own
   ground, because the relocated `check` calls `MemoryCloseCertificate::recheck`
   and `FileCloseDeletionEvidence::recheck` before deciding a single case — and
   only then requires the proposal to be that same reading. Agreement alone
   discharges nothing; §4.3's decisive rows measure both halves.

### 3.1 The published premise set is the union, per E03-B

`composition_premise_union` is reused unchanged: the published set is the union
of both components' premise sets and this route's own, and a key two components
disagree on refuses rather than being silently resolved. This route's own
premises are `libc_free_bindings_are_declared_premises`,
`derived_probe_text_preprocesses_as_the_base_unit_does`,
`filesystem_discriminant_is_read_from_a_source_probe` and — the card's own —
`backend_prerequisite_is_linked_from_another_translation_unit`. A test asserts
every component premise survives and that the published set is a strict
superset of either component's.

### 3.2 Digest keys the task binds

Eleven keys, and a test pins that they are exactly the task's
`contract_sources` with eleven distinct 64-hex digests:

| Key | Binds |
|---|---|
| `qcore:composition-source-input:v2` | the complete projection |
| `qcore:composition-fs-base-import:v1` | the **base** unit's captured identity |
| `qcore:composition-fs-derived-import:v1` | **this family's** derived unit's captured identity |
| `qcore:composition-fs-derivation:v1` | **this family's** derivation, as the core measured it |
| `qcore:composition-fs-memory-derivation:v1` | the **nested memory question's** derivation |
| `qcore:composition-fs-close-contract:v1` | the composition's own pinned contract |
| `qcore:composition-fs-memory-question:v1` | the nested memory question, verbatim |
| `qcore:composition-fs-backend-question:v1` | the nested backend question, verbatim |
| `qcore:composition-fs-backend-contracts:v1` | the backend prerequisite's whole pinned contract selection, which this route re-runs its recheck under and which is in neither nested question |
| `qcore:composition-prerequisite-fs-{memory-close,file-close-delete}:v1` | the two prerequisite identities, one key each |

Two derivation keys, not one, because one recipe can move without the other.
Two question keys, not one, because a moved nested *question* and a moved nested
*task* are two different retirements — and this family has two of each.

The memory prerequisite's own four contracts, and the backend prerequisite's
nested unmap contracts, are already bound inside those tasks' own digests, which
this task binds through their identities. The backend selection this route
*rechecks* under is separately bound because it is not in either nested question.

---

## 4. Evidence

### 4.1 Byte-identical certificate parity with the retained in-process route

`compiler/tests/filesystem_chunk_close_route_parity.rs` (3 tests).

| Check | Result |
|---|---|
| in-process `check_filesystem_chunk_close_source` certificate vs moved `evaluate()` certificate, as **serialized bytes** | **byte-identical** |
| the reading is a real three-level composition | 75 cases, 2 checked paths, 2 uncovered, 1 combined-uncovered |
| both committed C fixtures still carry every shape all three routes depend on | asserted |

The two fixtures are the very files the retained in-process suites read
(`compiler/tests/fixtures/chunkio/memory_close_source.c` and
`file_close_source.c`), `include_str!`-ed by the retained unit suites, this
parity test and the core-side suite, so none of them can drift onto different
source.

**Eight refusal-agreement rows**, count pinned: the delete flag hardcoded to `0`
and to `1`, a null chunk argument, the dispatch call removed, a release before
the dispatch, the memory discriminant substituted for the filesystem one, the
two discriminants collided, and the two unlinks confused. Each row asserts the
retained route really refuses **and** that the moved route refuses it too.
Nothing in this card makes a previously refused body admissible.

### 4.2 Core-side positive suite — 13 tests

`crates/qcore-verification/tests/filesystem_chunk_close_composition_source.rs`,
driving the real gateway imports, both real composition prerequisite task
derivations and the real trusted composition:

- the eleven bound-input keys are exactly the task's contract sources, with
  eleven distinct digests;
- exactly one obligation, `CompositionContract` under `ResourceContracts`, with
  its exact subject, scope and `PathBounds::BoundedPaths`;
- **the question carries two distinct derivations over one base text**: same
  base digest, different recipes, different recipe functions, different derived
  digests, neither equal to the base, and the unit identity naming the base;
- **the question nests three composition levels**, with the memory half at 1 and
  the backend half at 2, and this family's wrapper *being* the nested memory
  question's wrapper;
- the core reads 75 cases across both backend case vectors, publishes 2 checked,
  2 uncovered and 1 combined-uncovered path, reads `filesystem_kind = 0` out of
  its own probe and `memory_kind = 1` out of the nested one, requires the two to
  differ, and `recheck` reproduces the evidence;
- the request carries the question and none of fourteen named derived figures,
  and the other six projections stay absent;
- the published premise set is the union of both components' and this route's
  own four;
- both prerequisites are re-established at every level (43 non-deleting and 30
  deleting backend cases, and the backend half's own nested unmap composition's
  8 cases), and the composed case vector is exactly `2 + 43 + 30`;
- the prerequisite identity is the three-field key, each field load-bearing;
- deriving twice gives an equal task, identity and reading;
- the checker admits the core's own reading and refuses a foreign task first;
- **the two prerequisites are checked over two different units**, pinned in both
  directions (§2.2).

### 4.3 Core-side mutation matrix — 8 tests, **118 rows, all refused**

`crates/qcore-verification/tests/filesystem_chunk_close_composition_mutations.rs`.
Every forged reading is asserted to actually differ from the honest one
**before** it is submitted, and every row count is pinned so a row cannot
silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed reading | **45** (pinned) | function, revision, signature; each discriminant inflated, zeroed, collided and exchanged; cases emptied / dropped / duplicated / reordered; presence and delete flags flipped; the selected path retargeted; the backend case erased, retargeted and invented on the null entry; backend and path release flipped; every residual mapping and descriptor discharged and every mapping claimed unresolved; a deletion outcome upgraded to success, erased and invented on a non-deleting case; each of the four release flags erased and one claimed on the null entry; both counter decrements inflated and erased; checked paths emptied / widened / reordered; uncovered paths emptied, widened and claimed checked; combined-uncovered emptied, widened and set to the uncovered set; an invented profile assumption |
| Family and witness binding | **4** | a **memory-family** proposal and a **file_close-family** proposal against a filesystem task; a witness rebound to a foreign task digest; another witness kind entirely |
| Prerequisite forgeries | **8** (pinned) | each identity emptied; both emptied; each repeated in the other's slot; the two exchanged; a foreign identity in each slot |
| **Decisive prerequisite rows** | **2** | an *honest* identity set, honest derivations and an untouched composition contract, over a prerequisite whose **own** pinned contract is falsified — once for the memory composition, once for the deleting-close composition |
| **Two derivations** | **7** | a recipe that derives nothing; a recipe defining another function; a recipe returning the other macro; **this family's recipe replaced by the memory family's**; a probe function the recipe does not define; and substituted base text refused in **both** directions (neither context accepts the other's witness) |
| Nested questions | **5** (pinned) | a foreign memory question; the **non-deleting** backend question; the two exchanged; the memory question in both slots; the backend question in both slots |
| Every clause of the composition contract | **44** (pinned) | all `requires` and `ensures normal` clauses falsified two ways |
| Isolation | 3 | an unused `-D` leaves the projection, both derivation records and both nested questions byte-identical and still retires the task, in both witness directions |

**No mutation was admitted. No gate was changed and no repair was required by
the matrix.**

Two measured corrections the matrix forced, both kept rather than explained
away:

* **"a backend case retargeted" needed a real retarget.** The first nonnull
  deleting case already names backend case `0`, so setting it to `0` was not a
  mutation at all — the pre-submission difference assertion caught it, and the
  row was corrected to an out-of-range index rather than the assertion weakened.
* **the contract-clause count is 44, not the 42 predicted** from the retained
  unit suite's own loop. The pin was corrected to the measured number.

### 4.4 Provider-side matrix — 5 tests, 29 refusal rows

`providers/common/composition_effect_tests.rs`, driven over the **real**
projection the gateway emits for these bodies
(`providers/common/fixtures/filesystem_chunk_close_composition_projection.json`,
asserted live by a conformance test). Every refusal row is a *decline*: the
process returns no proposal, the host sees no witness, and the obligation stays
Unknown.

- the producer recomputes the whole three-level reading: the memory-close
  composition, the two-level deleting `cio_file_close` composition, both
  discriminants out of two different probe bodies, and then its own 75 cases —
  with the two null entries resolving no frame and naming no backend case, the
  73 present entries each naming one and releasing the chunk, deletion outcomes
  present exactly on the deleting arm, and at least one entry carrying
  unresolved OS custody and at least one not (so the composed figures really
  come from the backend composition);
- **a changed filesystem probe changes the reading rather than being ignored**:
  a discriminant equal to the memory one is refused outright (the arms would
  collapse), and a third value selects no complete filesystem path;
- **18 degenerate rows** (pinned): the probe emptied, wrongly signed and
  oversized; a derivation that derives nothing, is incomplete, or names another
  function; the two derivations sharing a recipe, over different base texts, or
  deriving one text; either nested slot holding another family; the
  **non-deleting** backend arm; a memory wrapper with no complete path;
  duplicated memory wrapper paths so an entry would select two; a memory
  question whose callees collapse; a backend wrapper with no complete path; a
  backend delete body emptied; a backend revision disagreement — all decline
  without panicking;
- **9 request-gate rows** (pinned): another profile; another family's question
  under this profile; no projection; another capability; an unadmitted dialect;
  an unsupported version; an incomplete identity; a repeated identity; another
  family's prerequisite set;
- and the honest question **is** answered, with the proposal asserted equal to
  the reading the producer recomputed — never anything it was handed.

### 4.5 End-to-end through the real built package — 2 tests

`providers/conformance/tests/filesystem_chunk_close_composition_capability.rs`.
The shipped `qcore.chunkio` **0.12.0** executable composes the real bodies; the
core admits only after re-establishing both compositions at every level,
checking both transported derivations against its own, and re-running its own
composition. Also measured:

- **fixture drift**: the committed provider fixture is the live projection —
  schema version, both nested questions in full, this family's probe paths and,
  **exactly**, both derivation records, which unlike a prerequisite identity
  *are* functions of the text alone. The two triples are checked for shape and
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
- **the two-derivation row end to end**: a projection derived under the memory
  family's recipe never reaches a task at all;
- **false contracts**: a Safe proposal over a falsified *composition* contract is
  rejected with **QP009**, and one over a falsified *prerequisite* composition's
  contract — with this composition's own contract untouched — is rejected too.

### 4.6 Wire compatibility

`crates/qcore-schemas/tests/composition_request_wire.rs` grows from 14 to **18
tests**. The filesystem question round-trips and carries none of fifteen named
derived figures at any level; its subject is the nested memory wrapper and its
`wrapper()` *is* that record; it nests three levels with two nested questions;
its two derivations are separate over one base; it never borrows another
family's prerequisite set in either direction, against all three other families;
the two nested slots are not interchangeable and the non-deleting backend arm is
refused; its two identity rules are non-vacuous; **all thirteen prerequisite
digest keys across the four families are distinct**; and unknown fields are
refused in its records.

**One deliberate pin update.** `composition_vocabulary_is_pinned`'s family-tag
list grows from three entries to four. **Its load-bearing negative halves are
untouched**: the nine transported operation forms and eight expression forms are
asserted unchanged, so this card widened **no** shared enum — the boundary
`P04-COMPOSE-SHARED-PATHS` asked for exactly that check and it fired correctly.

### 4.7 Lean — 35 audited theorems, no new axiom

`proof/QCore/TwoDerivationAdmission.lean` and
`proof/AuditTwoDerivationAdmission.lean` add the
`QCore.TwoDerivationAdmission` namespace, built on top of
`QCore.DerivedTextAdmission` rather than restating its single-derivation half.
Every theorem depends only on `propext` and `Quot.sound`, or on nothing at all.
`check_lean_audits.py` moves 67 → **68** audits.

- *the two derivations are two* (5): different recipes over one base derive
  different texts; **one recipe used twice is not two derivations**; two base
  texts are not one question; two recipes deriving one text are refused; a
  nonempty recipe really changes the text;
- *fail-closed admission* (19): admission is decided; a refusing core, a foreign
  own-derivation, a substituted recipe, substituted base text, a derivation that
  derives nothing, an incomplete derivation, a foreign nested derivation,
  **unseparated derivations**, each of the **two** refusing prerequisites, a
  **non-deleting backend** prerequisite, a foreign identity set, a foreign
  nested question, a memory prerequisite over another unit, **a backend
  prerequisite over *this* unit**, a backend prerequisite at another revision,
  one at another prototype, and a differing proposal each refuse on their own;
- *what an admission means* (8): it names both of the core's own derivations; it
  carries the separation; it establishes both compositions; it establishes the
  whole cross-unit tie; it names the core's own questions and identities; an
  admitted proposal **is** the core's own reading; two admitted proposals cannot
  disagree; every conjunct holds together;
- *non-vacuity* (3): the honest proposal is admitted over the honest context;
  each refusal really refuses on that same context; **the cross-unit expectation
  is two-sided** rather than a one-way convenience.

Judged **not applicable** and recorded rather than modelled, unchanged from
`P04-COMPOSE-MEMCLOSE`: the transported vocabulary's faithfulness to Clang's
typed AST (a gateway/importer premise), whether either derived text preprocesses
as its base unit does (a published assumption), whether the linker resolves the
backend prerequisite's wrapper to the body checked in the other unit (the new
published assumption), and the composition body itself, which is unedited and
whose figures are decided by the Rust tests of §4.1–4.5.

---

## 5. One decision procedure, not two

`filesystem_chunk_close`'s decision procedure — the case enumeration over both
backend vectors, the `FilesystemChunkCloseCase` shape, the contract fact table
and the `require_nested_discharge` / `require_successful_deletion` gates — is
**relocated verbatim** into
`crates/qcore-verification/src/filesystem_chunk_close_composition.rs`. The only
changes to the moved body are:

* the constants it read from its former module scope (the pinned revision, the
  probe recipe and function, this composition's own contract, and the backend
  prerequisite's whole pinned selection) are now explicit parameters
  (`FilesystemChunkCloseContracts`, `OwnedBackendContracts`);
* `super::file_close::recheck_deletion` — a five-line adapter wrapper that
  assembled the same constants — is replaced by the owned selection's own
  `close()`/`delete()` borrows and **deleted at its original site**, because it
  then had no caller. Its behaviour is preserved exactly: the moved procedure
  rechecks the backend reading under the same contracts, profiles and
  eligibility field the wrapper supplied;
* the import half is separated from the decision half
  (`import_filesystem_probe` / `check_filesystem_chunk_close_bodies`), so a
  caller that has the bodies — a recheck, or a mutation that falsifies one
  pinned contract — does not re-import to ask the same question;
* two accessors are added (`backend()`, `filesystem_kind()`) so the retained
  suites and the mutation matrix can re-decide over an already-imported reading
  without reaching into private fields.

`compiler/src/verifier_plugins/chunkio/filesystem_chunk_close.rs` becomes a
**106-line** adapter (from 295) that supplies the ChunkIO half and delegates.
Its public API is preserved: `CONTRACT`, `check_filesystem_chunk_close_source`,
`FilesystemChunkCloseEvidence` and `FilesystemChunkCloseCase` re-exported at the
same paths, with `recheck`, `memory_scope`, `filesystem_kind_probe`, `cases`,
`checked_paths`, `uncovered_paths`, `combined_uncovered_paths`,
`require_nested_discharge` and `require_successful_deletion` unchanged — so
`unsupported_close`, `nested_cleanup` and the pinned Cycle 19 compatibility
adapter are **untouched**.

`FilesystemChunkCloseEvidence::recheck()` keeps its argument-free signature by
retaining the owned contract selection in a `#[serde(skip)]` field, so its
callers did not change and **the certificate's serialized shape is
byte-identical** — which the retained adapter's archived corpus artifacts depend
on and §4.1 measures.

Three visibility widenings, all deliberate and all recorded: `recheck`,
`memory_scope` and `filesystem_kind_probe` were `pub(super)` inside the
compiler's `chunkio` module and are now `pub` on the relocated type, because the
moved route and the mutation matrix live outside that module. No new
*capability* is exposed by any of them: every one returns a reading the
certificate already carried, and the two genuinely new accessors (`backend()`,
`filesystem_kind()`) do the same.

Nothing else moved and nothing was duplicated:
`crates/qcore-verification/src/{memory_close_composition,unmap_composition,file_close_composition,cleanup_outcome,counter_contract,call_use_contract,field_destructor,list_unlink_contract,frame_source,teardown_source}.rs`
are unedited apart from the two additive methods §3 names on
`FileCloseCompositionSourceTask`.

### 5.1 Four incidental edits outside this card's subject, each recorded

1. `compiler/src/verifier_plugins/chunkio/file_close.rs` loses
   `recheck_deletion` (§5) — five lines with no remaining caller.
2. `crates/qcore-verification/tests/fixtures/memory_close_composition.rs`
   locates the profile directory by **walking up** from the crate manifest
   instead of a fixed `../..`, exactly as the `file_close` fixture already does.
   That fixture is now also included by the compiler crate's parity test, which
   sits at a different depth. Behaviour-preserving in the verification crate.
3. `crates/qcore-verification/tests/{memory_close,file_close}_composition_source.rs`
   and `crates/qcore-schemas/tests/composition_request_wire.rs` move to the new
   `nested()` shape (§1.2), keeping every assertion's force.
4. `compiler/tests/unit/filesystem_chunk_close.rs` reaches the decision half
   through the relocated `check_filesystem_chunk_close_bodies` and the
   certificate's accessors instead of the module-private `check` and private
   fields. **Every assertion is unchanged**, including the full
   `requires`/`ensures normal` falsification loop.

---

## 6. The gateway diff is one module

The whole `crates/qcore-c-gateway` change is `composition_source.rs`: one new
`CompositionFamily` variant, one new sealed import type, one new importer, and
four exhaustive-match arms. `legacy_c.rs`, `legacy_summary.rs`, `typed_source.rs`,
`borrowed_actual.rs`, `aggregate_store.rs`, `may_alias.rs` and every other
analysis file are **untouched**. **No gateway analysis rule changed, so no unit
verdict, function verdict or diagnostic can move from this card.**

The existing `import_composition_source` explicitly **refuses** both derived
families before importing anything, because each must build its own text first;
the three are separate importers by construction, not by convention.

---

## 7. Schema decision — flagged loudly

**`qcore-schemas` moves 12.0.0 → 13.0.0. This is a major schema bump.**

Additive was preferred and, at the **wire**, was achieved: no existing
projection's bytes move, `COMPOSITION_SOURCE_INPUT_VERSION` stays 2 and the
digest key stays `…:v2`, so no existing task digest moves from this card. What
forces the major version is the Rust API, under the same documented policy as
4.0.0, 5.0.0, 7.0.0–12.0.0: three new variants on exhaustively matched enums,
and one accessor (`nested()`) that now returns a vector because this family has
two nested questions (§1.2).

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 12.0.0 | **13.0.0** |
| `qcore-verification` crate | 0.4.0 | 0.4.0 (additive API) |
| `qcore-verifier-host` crate | 0.4.0 | 0.4.0 (no host change) |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (every published name still resolves, now as a re-export — the precedent `P04-COMPOSE`, `-A` and `-MEMCLOSE` set) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Composition source input | 2 | 2 (**unchanged**) |
| Composition semantic profiles | 4 | **5** |
| ChunkIO package | 0.11.0 | **0.12.0** |
| Contract corpus | 67 modules | 67 modules (unchanged) |

**Deliberate pin updates**, each with its reason:

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.11.0 → 0.12.0. The new
   profile is reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other manifest field are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version in
   its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/Cargo.{toml,lock}` and
   `providers/Cargo.lock` — schemas 13.0.0 so the pinned standalone-provider
   fixture's `--locked` build resolves.
4. `crates/qcore-schemas/tests/composition_request_wire.rs` — the family-tag list
   grows by one (§4.6), with both load-bearing negative halves kept.

All five provider packages were rebuilt and verified.

---

## 8. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2667 passed, 0 failed, 8 ignored** (2567+ → 2667) |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **68 audits passed** (67 → 68), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace --no-fail-fast` | **115 passed, 0 failed** (108 → 115) |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.11.0 → **0.12.0** |
| `python3 -B -m unittest discover -s scripts/tests` | 146 tests, **1 failed** — §8.3 |

### 8.1 New tests

Twenty-eight new workspace tests — 13 core-side positive
(`filesystem_chunk_close_composition_source.rs`, one of which is the
fixture generator and is inert without `QCORE_WRITE_FS_CLOSE_FIXTURE`), 8
mutation (`filesystem_chunk_close_composition_mutations.rs`), 3 route parity
(`filesystem_chunk_close_route_parity.rs`) and 4 wire
(`composition_request_wire.rs`, 14 → 18) — plus six in the providers
workspace: 5 producer-side (`composition_effect_tests.rs`) and 1 conformance
pair (`filesystem_chunk_close_composition_capability.rs`, 2 tests).

### 8.1.1 One recorded flake, deliberately not weakened

The first full workspace run recorded **one** failure:
`enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`,
the load-flaky `RLIMIT_NPROC` row recorded as **`P02-FLAKE`** in
`docs/reports/remaining-p03/README.md` §9.1 — it counts every process of the
real UID, and this wave ran a seven-agent fleet on one host. It **passes in
isolation** and its assertion is deliberately unchanged. The clean re-run
recorded in the table above is the one taken with the tree quiescent.

### 8.2 The canonical CMetrics audit is byte-identical, measured

Not argued: measured, in this worktree, with the five provider packages built
**once** from this branch's tree and used for both runs, so the comparison
isolates the analysis change from the package-identity residue.

```sh
python3 -B scripts/prepare_cmetrics_clang_audit.py \
  --fluent-bit <checkout> --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --output <build>
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit
cargo run --quiet -p qcore-audit -- <build>/fluent-bit-clang-production/cmetrics-audit.json.config
```

The audit was run at this branch's head, the worktree checked out at the
untouched base commit `8069a99`, and the audit run again. Both reports are
`431423131e1f98937b0db0cd6665a8557363d1332931ff12956c877367ae5d3c` and both
assumption sidecars are
`c0c15ed22c6ff96914ad3d9318caea8aab3baa545929895cd1c4a97f7ce540b9`, with the
same `1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0
gateway failures` and the same `1 checked function contract(s) / 6 Unknown`.

That is what the diff shape predicts (§6): no gateway analysis rule changed, no
contract file changed, and the retained in-process route publishes a
byte-identical certificate (§4.1).

**What was not re-measured here.** The `chunkio-core` profile route and the four
pinned 185-unit artifacts need a further prepared clone and a forced-Libevent
Monkey tree, and this host's disk was the binding constraint during the wave
(the shared worktree fleet filled it; a `cargo clean` was needed mid-run just to
complete the workspace gate). This is the same position `P04-COMPOSE-A`,
`P04-B2`, `P04-COMPOSE` and `MEMCLOSE` recorded for the 185-unit corpus.

**The integrator's corpus consolidation should expect no change from this card
beyond the package-identity residue class** — the ChunkIO `provider_version` and
`package_digest` leaves in `source-effects.json`, as in every card that bumps
that package, plus the schemas-13.0.0 package digests. The hermetic
`crates/qcore-audit/tests/second_source_set.rs` route, which exercises the same
configured-audit machinery from configuration alone, is in the workspace gate
above and passes.

### 8.3 One matrix-file action item for the integrator

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
compares the committed `docs/reviews/r0_3/acceptance_matrix.json` against freshly
derived values. This card bumps the schema crate and adds one audited Lean
module, and the card's constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 12.0.0 →
**13.0.0** and `validation.lean_audits` 67 → **68**, with
`AuditTwoDerivationAdmission.lean` added to `validation.lean_audit_files`.
Regenerate the matrix (or supply `--gate`) at the wave's close, exactly as waves
9–15 did. Nothing else in the matrix moves from this card: the contract corpus
stays 67 modules, CT coverage is untouched at 32/50, and no verdict moves.

---

## 9. Open items and named boundaries

- **`P04-COMPOSE-FS-CROSS-UNIT`** (new) — the backend prerequisite is a
  composition checked over a **different translation unit** than the wrapper, so
  this route has no text-level tie to it. What ties it is its own
  `PrerequisiteIdentity`, its declared source revision and its declared
  prototype, all re-derived by the core. That the linker resolves the wrapper's
  `cio_file_close` frame to the body checked in that other unit is a **declared
  premise**, published in the task's assumptions. It is the same class of
  premise as `MEMCLOSE-DERIVED-PREPROCESSING`, and the strongest thing this
  route could add without a whole-program symbol-resolution capability.
- **`P04-COMPOSE-FS-TWO-RECIPES`** (new) — the two-derivation separation is
  checked structurally (different recipes, different functions, different
  derived texts over one base). It does **not** prove that the two probes read
  two different macros: a recipe that spelled a different function name but
  returned the same macro would pass the separation and then be refused only
  because the reading collides at `filesystem_kind == memory_kind`. Both halves
  are pinned (§4.3, §4.4), and the second is the load-bearing one; the first is
  a cheap structural precondition, recorded as such.
- **`P04-COMPOSE-FS-BACKEND-ARM`** (new) — `revalidate_as_prerequisite` on
  `FileCloseCompositionSourceTask` refuses the non-deleting arm because this
  question needs both case vectors. A future dependent that needs only the
  non-deleting reading would want a second entry point rather than a widening of
  this one.
- **`MEMCLOSE-BASE-TIE-TEXT`** (inherited, now doubled) — this route binds the
  base import and its own derived import separately for the same measured
  reason, and additionally binds the nested question's derivation. The memory
  half's own base tie is unchanged; this family's is text-level on the memory
  side and identity-level on the backend side by construction.
- **`MEMCLOSE-DERIVED-PREPROCESSING`** and **`MEMCLOSE-PROBE-ISOLATION`**
  (inherited, now doubled) — two recipes now append two probe definitions to the
  same base unit, so both premises apply twice. Nothing detects a base unit that
  already defines either probe name, or one ending inside a conditional group;
  both recipes are pinned constants over pinned units and the premises are
  recorded rather than discharged.
- **`MEMCLOSE-COUNTER-SUGAR`** (inherited, unchanged) — the memory half's
  counter pair is still derived under the trusted composition's own three-part
  rule. It buys no authority on this route either.
- **`P04-COMPOSE-CALLEE-DECL`** (inherited, now covering four wrappers) — the
  filesystem dispatch callee (`cio_file_close`, `void (struct cio_chunk *, int)`)
  is a **caller assertion** in the memory family's `fixed_callees`, not read off
  a declaration. Declaring the callee vocabulary in the `.qcore` contracts, as
  `P04-B1` did for the cleanup families, would make the whole table
  contract-derived.
- **`P04-FRAME-OBLIGATION-ID`** and **`P04-B2-PREREQ-IDENTITY-CWD`** (inherited,
  unchanged) — the shared obligation-identifier scheme is sound only via the
  identity triple, and a prerequisite identity is not a content-only function of
  the body. This route names two prerequisites by triple and the conformance
  drift test pins both for shape only, comparing **both derivation records**
  exactly because each *is* a function of its text.
- **`P04-COMPOSE-SHARED-PATHS`** (inherited, unchanged) — the transported bodies
  reuse `CleanupReplayFunctionRecord` and its step/expression enums. This card
  widened neither, and §4.6's pin asserts it.
- **`SourceMatch` (P04-NEXT-C)** (inherited, unchanged) — `nested_memory` and
  `nested_cleanup` stay behind the sealed reviewed-library boundary. Their
  removal needs a reviewed-body attestation the core can re-derive, not a bigger
  vocabulary.
- **Nothing about the OS is established.** A recorded `cio_file_close` frame is
  a recorded call; whether a mapping was unmapped, a descriptor closed or a file
  deleted stays inside the backend prerequisite's own declared premises, which
  this route publishes in its union.
- The in-process adapter is **retained** (hard invariant 3). Its removal would
  need the P09-style archived parity measurement plus a production caller for
  the process route; §4.1 supplies the parity half only — and it is stronger
  than a parity record, because both routes call one implementation.
- The moved capability has **no production caller yet** — it is exercised by
  these tests, not by the Cycle 19 audit, which still runs the in-process
  adapter. Same position as P05, P04-B2, P04-COMPOSE, P04-COMPOSE-A and MEMCLOSE.

### 9.1 The composable-now census, updated

Of `P04-NEXT`'s original thirteen in-process ChunkIO capabilities, six
non-composition capabilities have moved plus **four composition modules**. The
remaining classification, updating `P04-COMPOSE` §1's table:

| Composition module | Class | Reason |
|---|---|---|
| `unmap_wrapper` | moved (P04-COMPOSE) | — |
| `file_close` (both entry points) | moved (P04-COMPOSE-A) | — |
| `memory_close` | moved (P04-COMPOSE-MEMCLOSE) | — |
| `filesystem_chunk_close` | **moved by this card** | — |
| `unsupported_close` | **composable-now** — recorded as **`P04-COMPOSE-UNSUPPORTED`** | its only prerequisite is `filesystem_chunk_close`, which this card moves and which now exposes `prerequisite_identity()`. §9.2 sizes it |
| `nested_memory` | excluded — sealed `SourceMatch` boundary | unchanged |
| `nested_cleanup` | excluded — sealed `SourceMatch` boundary (transitively) | unchanged |

**Summary: four moved, one newly composable-now, two excluded behind
`SourceMatch`. The ladder's reachable rungs are one from the top.**

### 9.2 Why `unsupported_close` was not taken in this card

The card's brief allowed taking it "ONLY if it falls out naturally". It does
not, and the reason is measured rather than assumed:

`check_unsupported_close_source` consumes a `FilesystemChunkCloseEvidence`,
calls `recheck()`, and then reads **two** things out of it — the memory
certificate and the filesystem probe — and passes them to
`memory_close::unsupported_path`, which replays the wrapper under a **symbolic**
backend class `Value::OtherBackend(memory, file)` that is *neither* probed
constant. That symbolic class is a third reading over the same wrapper, and its
published record (`excluded_kinds`, `source_path`, and five prose fields
recording the result boundary) is a shape no existing proposal record covers.

Moving it is therefore a fifth family — a new proposal record, a new producer
replay arm carrying the symbolic domain, and its own mutation matrix — not an
extension of this one. Taking it here would have doubled the card and buried the
two-derivation and cross-unit findings, which are the load-bearing ones. It is
recorded as **`P04-COMPOSE-UNSUPPORTED`** with this sizing, and its one
prerequisite is now nameable.

---

## 10. Reproduction

```sh
git submodule update --init --recursive
cargo test --workspace --no-fail-fast
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe
cargo run -p qcore-compiler --example reevaluation_followup
python3 -B scripts/check_lean_audits.py
cargo test --manifest-path providers/Cargo.toml --workspace --no-fail-fast
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit
cargo run -q -p qcorec -- plugins verify build/cycle19/provider-packages-unit/qcore.chunkio
python3 -B -m unittest discover -s scripts/tests -t scripts/tests
```

The provider fixture is regenerated from the live gateway with

```sh
QCORE_WRITE_FS_CLOSE_FIXTURE=$PWD/providers/common/fixtures \
  cargo test -p qcore-verification --test filesystem_chunk_close_composition_source \
  write_the_provider_fixture
```

and the conformance drift test refuses a stale one.

---

## 11. Draft increment paragraph

> ChunkIO filesystem-close composition (P04-COMPOSE-FS): the fourth and
> second-to-last reachable composition module moves through the process
> boundary — the first with **two derived units in one question** (its own
> filesystem probe and the nested memory question's memory probe, over one base
> text under two recipes the core requires to differ in text, function and
> result) and the first whose **every** prerequisite is a composition, checked
> over **two different translation units**. The measured consequence is recorded
> rather than papered over: this family cannot carry the memory family's
> single-unit base tie, so the unit tie is pinned in **both** directions (the
> memory half must be over this text, the backend half must not) and the
> backend prerequisite is tied by identity, revision and prototype, with linker
> resolution published as a premise (`P04-COMPOSE-FS-CROSS-UNIT`). Parity with
> the retained in-process route is byte-identical on the serialized
> certificate; a 118-row mutation matrix, an 18-row provider degeneracy matrix
> and 9 request-gate rows all refuse; and the two decisive rows measure that a
> falsified *prerequisite composition's* contract refuses at evaluation with
> this composition's own contract untouched. `nested()` returns a vector because
> one slot could not publish two nested questions honestly; that and three enum
> variants make `qcore-schemas` 13.0.0 with **existing wire bytes and every
> existing task digest unchanged**. ChunkIO package 0.12.0; 35 audited Lean
> theorems in a new `QCore.TwoDerivationAdmission` namespace built on
> `QCore.DerivedTextAdmission` rather than restating it. `unsupported_close` is
> now composable-now and honestly sized out as `P04-COMPOSE-UNSUPPORTED`: it
> needs a fifth proposal record carrying the symbolic other-backend class, not
> an extension of this one. No checker rule, diagnostic code, contract file or
> verdict changed; nothing is promoted.
