# P04-COMPOSE-A — the `cio_file_close` compositions through the process boundary

Workstream W5 (Providers), the follow-on `P04-COMPOSE` named at its §9: both
`file_close` entry points became composable-now the moment the `unmap_wrapper`
composition moved, because that composition was their only unmoved
prerequisite.

This card moves them. They are **the first route whose own prerequisite is
another composition**, so the core's re-establishment recurses two composition
levels — and every level is work the core does itself.

- Worktree cut from `main` at **`009bde0`** (wave-13 consolidation); branch
  `worktree-agent-a9e507c16f85c28df`.
- Libevent submodule initialised at the pinned revision.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code was
  added, changed or reordered; no `QP`-code was added. No raw unit is promoted.
- The in-process adapters are **retained** (hard invariant 3) and now *delegate*
  to the same relocated decision procedure the moved route runs, so the two
  cannot drift.

---

## 1. Vocabulary decision — extended, but not additively

**Decision: extend `qcore-schemas::composition_summary`, the fifth transported
source vocabulary. No sixth vocabulary was added.** The `file_close` question is
the same *kind* of question the leaf family asks — a wrapper replay whose call
results are the readings of separately checked prerequisite capabilities — so a
sixth module would have duplicated eight expression forms, nine operation forms
and the whole projector for no distinct refusal surface, which is exactly what
`P04-COMPOSE` §2.2 argued against.

**The extension is not purely additive, and the card's preference for additive
could not be honoured.** Two shapes had to change, and both changes are forced
by the question, not by convenience:

| What changed | Why an additive alternative was rejected |
|---|---|
| `CompositionSourceInput::prerequisites` moves from the flat `UnmapWrapperPrerequisites` triple to a family-tagged `CompositionPrerequisiteSet` | The leaf family names three prerequisites, none a composition; `file_close` names three or four, one of which **is** the leaf composition. Keeping one flat triple would have made a field called `prerequisites` mean the *route's own* prerequisites in one family and the *nested route's* in another. A field whose meaning depends on a sibling field is precisely what these vocabularies exist not to have |
| `CompositionFamilyInput` gains a `FileClose` variant whose `unmap` field is a **boxed `CompositionFamilyInput`** | The producer must recompute the nested composed reading, so the nested question must travel in full. Carrying its two bodies loose would have been a second, differently-shaped copy of a question this enum already expresses; carrying the enum makes the recursion structural and lets the core require the transported nested question to be, verbatim, the one its own nested task imported |

Everything else is additive: `FileClosePrerequisites`, `FileCloseDeleteInput`,
the case/reading records, `CompositionProposalRecord::FileClose`, and the two new
semantic-profile constants.

### 1.1 One shared enum was widened, deliberately and narrowly

`cio_file_close` returns `void`, so its complete paths end without a value, and
`CleanupReplayOperationRecord` had no `Return`. One variant was added.

The widening is confined by refusal on both sides, not by convention:

* the gateway's cleanup-replay projector already refuses
  `SourceOperation::Return` **by name**, and still does, so that family cannot
  emit one;
* the cleanup-replay producer now declines `CleanupReplayOperationRecord::Return`
  **by name**, so it cannot consume one;
* the composition projector admits it only for a body the caller declared
  `void`-returning — the `file_close` wrapper — and the leaf family's producer
  declines it too.

This is the mechanism `P04-COMPOSE-SHARED-PATHS` asked for: a future widening of
the shared enums fails at an explicit refusal rather than silently widening a
family.

**A dangling claim was found and repaired rather than corrected downwards.**
`composition_summary`'s module doc named `composition_vocabulary_is_pinned` as
the test that pins the exact variant inventory this route admits. **That test did
not exist.** Adding the ninth operation form is exactly the change it was
supposed to catch, so the pin is now written: it asserts the sorted tag
inventory of both shared enums (nine operation forms, eight expression forms)
and of the composition families themselves, so a future variant must be decided
there before it can travel.

### 1.2 Two entry points, two questions

`check_file_close_source` and `check_file_close_deletion_source` are separate
in-process entry points, and the deleting one computes the non-deleting reading
inside itself. The moved route mirrors that exactly: one family variant with an
`Option` delete arm, **two** semantic profiles
(`qcore.c_file_close_composition.v1`,
`qcore.c_file_close_delete_composition.v1`), two tasks, two obligations. The
deleting reading publishes the whole non-deleting case vector *and* the deleting
one; the non-deleting reading leaves the deleting fields **absent, not empty**,
so "no deleting arm" and "an empty deleting arm" cannot be confused. A test pins
that each entry point's witness is refused against the other's task, in both
directions.

---

## 2. What actually crosses the boundary

`CompositionSourceInput` for a `file_close` question carries three things:

1. `schema_version` (now `2`);
2. `prerequisites` — a `FileClose` set of **three or four** named
   `PrerequisiteIdentity` triples (`wrapper_call_use`, `unmap_composition`,
   `native_cleanup`, and on the deleting arm `native_delete`), which are lookup
   keys;
3. one `CompositionFamilyInput::FileClose` carrying **five bodies' complete typed
   path sets** — the wrapper, the native close, the nested composition's wrapper
   and native, and (deleting) the guarded-delete body — plus three record
   profiles and the declared eligibility field.

**No composed result and no prerequisite conclusion travels, at either level.**
No case vector, no selected path, no case index, no counter figure, no residual
classification, no verdict, no contract text and no policy name. A composed
certificate has no wire representation on this route at all.

The consequence is the point of the card: **the producer must recompute the
whole nested composed reading itself.** `providers/common/composition_effect.rs`
runs its own `compose_unmap_wrapper` over the transported nested question, its
own `replay_native_cleanup_outcome` over the close body, its own
`replay_guarded_delete` over the delete body, and only then its own wrapper
replay over the entry domain. It evaluates no contract and reads no policy.

Tests assert the absence on the real transported request, for both arms: the
serialization contains `paths`, `close_profile` and `native_profile` and
contains none of `cases`, `unmap_case`, `close_case`, `delete_case`,
`native_source_path`, `counter_decrements`, `up_counter_decrements`,
`wrapper_writes`, `returned`, `field_writes`, `fields_reset`, `uncovered_paths`,
`certificate`, `verdict`, `unresolved`, or any of the three policy names.

### 2.1 The composed figures

Three published figures are functions of *other capabilities'* readings, which is
what makes this a composition rather than a body replay:

* `unmap_case` — the **nested composition's** own case index for this entry;
* `close_case` and `delete_case` — the two cleanup replays' own case indices;
* `up_counter_decrements` — read out of the nested composition's own case, not
  recomputed here.

The positive suite pins that none is constant: the counter decrement takes both
values over the domain, more than one nested case is selected, and the single
entry state that never reaches the nested composition names no prerequisite case
at all and decrements nothing.

### 2.2 A measured boundary: case indices, not selected paths

Recorded, not hidden, as **`P04-COMPOSE-A-CASE-INDEX`**.

This level publishes each prerequisite's **case index** over a fixed sampled
prestate domain, not the prerequisite's selected source path. Permuting a
prerequisite body's paths therefore moves that prerequisite's own reading — which
its own task publishes and its own witness binds — while leaving *this* reading
identical. The permuted bytes are still refused end to end, by the prerequisite
task's own source binding; the row
`permuting_a_prerequisite_bodys_paths_is_invisible_to_this_level` measures
exactly which layer notices, and asserts both halves (the nested reading really
does move; this level really does not).

This was found by a test that first asserted the opposite. The assertion was
corrected to the measurement rather than the measurement explained away.

---

## 3. Where the authority is: the two-level re-establishment

`FileCloseCompositionSourceTask::derive` and `evaluate` do all of the following
before any proposal is compared.

1. **Both prerequisite record profiles are contract-derived, not wire-derived.**
   `derive` calls P04-B2's `declared_profile` twice — once for the close policy,
   once for the delete policy — and refuses unless each transported record is
   exactly the declaration. It refuses a close policy that declares a
   guarded-delete eligibility field, and requires the delete policy to declare
   one and the transported `mapping_field` to be it. The profiles handed to the
   compositions are always the core's own.
2. **Every prerequisite is re-established, never read — recursively.** For each
   named identity the core re-derives the prerequisite task from its own import,
   refuses unless the claimed triple *is* the identity it derives, and re-runs
   that prerequisite's own evaluation. `unmap_composition`'s re-establishment is
   `CompositionSourceTask::revalidate_as_prerequisite`, which re-establishes that
   composition's **own three** prerequisites and re-runs its unedited
   composition; each cleanup replay in turn re-establishes its own call-result
   def/use prerequisite. No level is skipped and no level is believed.
3. **Every transported body is tied to the prerequisite that checked it**, at
   both levels — the wrapper to its def/use prerequisite, the close body to its
   cleanup prerequisite, the delete body to its delete prerequisite, and the
   nested wrapper to the nested composition. Without this a projection could
   compose one body's paths with another body's re-established certificate.
4. **The transported nested question must be the nested task's own.** The core
   requires `family.nested()` to equal, verbatim, the `CompositionFamilyInput`
   its own nested composition task imported. A nested question that is merely
   well-formed, or that belongs to another unit, or that carries a different
   declared profile for the same bodies, is refused here.
5. **Acceptance terminates in the core's own compositions.** The core runs the
   unedited relocated procedures over its own imports, its own contracts and its
   own contract-derived profiles, and only then requires the proposal to be that
   same reading. Agreement alone discharges nothing — §6.2's decisive rows
   measure both halves.

### 3.1 The measured recursion depth

Two figures, because they count different things, and both are pinned by
`the_measured_recursion_depth_is_pinned`:

| Figure | Value | What it counts |
|---|---|---|
| Composition levels | **2** | The transported question nests exactly one composition inside another. `CompositionFamilyInput::nesting_depth()` computes it from the question's own shape; a wire test pins that the leaf family reports 1 and both `file_close` families report 2, and that a question nesting a third level is refused by `matches` |
| Task levels along the deepest chain | **4** | `file_close` → the nested unmap composition → that composition's cleanup replay → *that* replay's own call-result def/use prerequisite |
| Distinct tasks re-established | **10** (deleting), **8** (non-deleting) | Eleven and nine counting the `file_close` task itself; the test enumerates them by task digest |

The deepest chain is worth naming explicitly, because it is where the decisive
row lives: the leaf contract that is falsified in §6.2's last three rows sits
three levels below the task whose witness is being admitted.

### 3.2 The published premise set is the union, per E03-B

`composition_premise_union` is reused unchanged. The published set is the union
of all three or four components' premise sets and this route's own, and a key two
components disagree on refuses rather than being silently resolved.

The union is **transitive**: the nested composition's own published set is
already the union of its three components' and its own, so the leaf
composition's private premise
(`sync_and_logging_frames_are_declared_premises`) reaches this level through it.
A test asserts that, and that the published set is a strict superset — no
component assumes this route's own `libc_free_bindings_are_declared_premises` or
`backend_release_does_not_discharge_os_custody`.

### 3.3 Digest keys the tasks bind

Ten keys on the non-deleting task, fifteen on the deleting one, and a test pins
that they are exactly the tasks' `contract_sources`:

| Key | Binds |
|---|---|
| `qcore:composition-source-input:v2` | the complete projection |
| `qcore:composition-import:v1` | the importer's captured source/build identity |
| `qcore:composition-file-close-contract:v1` | the non-deleting composition's pinned contract |
| `qcore:composition-close-contract:v1` | the close prerequisite's pinned contract |
| `qcore:composition-close-policy:v1` | the selected close declaration |
| `qcore:composition-close-profile:v1` | the close profile **the core derived** |
| `qcore:composition-nested-question:v1` | the nested question, as transported |
| `qcore:composition-prerequisite-close-call-use:v1` | that prerequisite's identity |
| `qcore:composition-prerequisite-unmap-composition:v1` | the **nested composition's** identity |
| `qcore:composition-prerequisite-close-cleanup:v1` | that prerequisite's identity |
| *(deleting only)* `qcore:composition-file-close-delete-contract:v1` | the deleting arm's own pinned contract |
| *(deleting only)* `qcore:composition-delete-prerequisite-contract:v1` | the guarded-delete prerequisite's contract |
| *(deleting only)* `qcore:composition-delete-prerequisite-policy:v1` | the selected delete declaration |
| *(deleting only)* `qcore:composition-delete-prerequisite-profile:v1` | the delete profile the core derived |
| *(deleting only)* `qcore:composition-prerequisite-close-delete:v1` | that prerequisite's identity |

The nested composition is bound **twice, under different keys**: its *identity*
(the task the core derives) and its *question* (the projection as transported).
A moved nested task and a moved nested question are two different retirements,
and both are visible to this task's identity. A test asserts the two digests
differ and that the identity one is the nested task's own triple digest.

Every prerequisite identity gets its own key, so no key can absorb another's
movement; a test asserts all four digests are pairwise distinct, and a wire test
asserts all seven key constants across both families are distinct.

---

## 4. One decision procedure, not two

`file_close`'s decision procedure — `ResidualCustody`, `FileCloseCase`,
`FileCloseEvidence`, `DeletionOutcome`, `DeletingCloseCase`,
`FileCloseDeletionEvidence`, the eleven-constructor `Value` domain, the phase
machine and both entry procedures — is **relocated verbatim** into
`crates/qcore-verification/src/file_close_composition.rs`. The only change to the
moved body is that the constants it read from its former module scope (the pinned
revision, the nested composition's contracts and record profile, the native close
contract and profile, and the guarded-delete contract, profile and declared
eligibility field) are now explicit parameters (`FileCloseContracts`,
`FileCloseDeleteContracts`).

`compiler/src/verifier_plugins/chunkio/file_close.rs` becomes a 159-line adapter
that supplies the ChunkIO half and delegates. Its public API is preserved:
`CONTRACT`, `DELETE_CONTRACT`, `check_file_close_source`,
`check_file_close_deletion_source`, and every published type re-exported at the
same path. **Both certificates' serialized shapes are byte-identical**, which the
retained adapter's archived corpus artifacts depend on and §6.1 measures.

`file_delete::profile()` moves from private to `pub` for the same reason
`file_outcomes::profile` did in `P04-COMPOSE`: so the parity test can assert
directly that the retained adapter's hardcoded table **is** the vocabulary the
pinned contract declares.

Two small consequences, recorded rather than hidden:

* `FileCloseDeletionEvidence::recheck` now takes its contracts explicitly, so its
  one dependent caller (`filesystem_chunk_close`) goes through a new
  `file_close::recheck_deletion` adapter that supplies them. That route's own
  unit tests are unchanged and still pass.
* The retained suite's `deletion_fixture()` is now also committed as
  `compiler/tests/fixtures/chunkio/file_close_source.c`, and a new test asserts
  the committed bytes **are** the ones that suite builds. Both routes read one
  file, so they cannot drift onto different source.

Nothing else moved and nothing was duplicated:
`crates/qcore-verification/src/{cleanup_outcome,counter_contract,call_use_contract,unmap_composition}.rs`
and `compiler/src/verifier_plugins/chunkio/{file_outcomes,file_attempts,unmap_wrapper}.rs`
are otherwise unedited.

---

## 5. The gateway diff is one module

The whole `crates/qcore-c-gateway` change is `composition_source.rs` plus its
`Cargo.toml` schema-version line.
`legacy_c.rs`, `legacy_summary.rs`, `typed_source.rs`, `borrowed_actual.rs`,
`aggregate_store.rs`, `may_alias.rs` and every other analysis file are untouched.
No gateway analysis rule changed, so no unit verdict, function verdict or
diagnostic can move from this card. §7 records what that means for the pinned
artifacts.

---

## 6. Evidence

### 6.1 Byte-identical certificate parity with the retained in-process routes

`compiler/tests/file_close_route_parity.rs` (3 tests), over the committed C
fixture both routes read.

| Check | Result |
|---|---|
| `file_outcomes::profile("cio_file_native_close")` vs `declared_profile(filesystem_outcomes.qcore, "native_close_outcomes")` | **equal**, and the policy declares no eligibility field |
| `file_delete::profile()` vs `declared_profile(filesystem_delete.qcore, "native_delete_outcomes")` | **equal**, and the policy declares `map` as its eligibility field |
| in-process `check_file_close_source` certificate vs moved `evaluate()` certificate, as serialized bytes | **byte-identical** |
| in-process `check_file_close_deletion_source` certificate vs moved `evaluate()` certificate, as serialized bytes | **byte-identical** |
| the certificates are real two-level compositions | 43 composed non-deleting cases each naming a nested case, 30 deleting cases each naming a guarded-delete case, the deleting reading embedding the non-deleting one, and the nested composition's own `native_source_path` figure surviving into this level |

**Fourteen refusal-agreement rows**, count pinned, deliberately spanning all
three levels: the wrapper releasing the backend before its path, freeing the
wrong allocation, not releasing the backend, reordering the two cleanup calls,
observing a result it must ignore, passing the wrong identity to the nested
composition, guarding the deleting arm on the wrong value, reporting the wrong
arm, transposing the two names it reports; the nested composition's wrapper
ceasing to decrement and decrementing the wrong counter; the nested
composition's own prerequisite body ceasing to reset a declared field; the
native close body ceasing to reset its declared field; and the guarded-delete
body dropping one of its two eligibility guards. Each row asserts the retained
route really refuses **and** that the moved route refuses it too. Nothing in this
card makes a previously refused body admissible.

### 6.2 Core-side positive suite

`crates/qcore-verification/tests/file_close_composition_source.rs` (12 tests
plus a fixture generator), driving the real gateway imports, the real task
derivations at both levels and the real trusted compositions:

- the non-deleting composition answers 43 cases with the very counts the
  retained suite pins (42 backend-released, 21 path-released, uncovered paths
  `[1, 2]`), and its deleting fields are **absent**;
- every reached case names the nested composition's own case and the close
  replay's own case; the unreached entry state names neither and decrements
  nothing; the counter decrement takes both values and more than one nested case
  is selected;
- releasing the backend never discharges an unresolved OS residual: 17 of 43
  cases discharge, and every non-discharged case still released the backend;
- the deleting entry point answers a strictly larger question — 30 cases over
  four disjoint outcomes (1 no-backend, 13 blocked, 8 failed, 8 succeeded) with
  every source path covered;
- the two entry points are separate tasks under separate profiles, and neither
  witness is admitted against the other's task;
- the re-establishment recurses two composition levels and four task levels,
  with the distinct task inventory pinned at 11 and 9;
- the request carries no composed or prerequisite result, at either level;
- the digest keys are exactly the tasks' contract sources, the four prerequisite
  digests are pairwise distinct, and the nested identity and nested question are
  bound separately;
- the published premise set is the union of every level's premises;
- both record profiles are the contracts' own declarations and the declared
  eligibility field is read from the contract.

One correction the suite made to its own first draft is recorded: the deleting
arm's diagnostic follows the **returned** value, not the syscall outcome, so a
deletion blocked by the record's own fields logs exactly as a failed one does —
21 rows, not 8. The assertion was corrected to the measurement.

### 6.3 Core-side mutation matrix — 7 tests, **171 rows, all refused**

`crates/qcore-verification/tests/file_close_composition_mutations.rs`. Every
forged reading is asserted to actually differ from the honest one **before** it is
submitted, and the row counts are pinned so a row cannot silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed reading | **58** (pinned) | Over both arms: function, revision, signature, profile assumptions; cases emptied / dropped / duplicated / reordered; uncovered paths emptied and widened; and per reached case: selected wrapper path, backend presence, path allocation, **the nested composition's case (retargeted and erased)**, the close replay's case (retargeted and erased), both residual classifications, both release flags, the counter decrement (inflated and erased). On the deleting arm additionally: the deleting arm erased, emptied, dropped, reordered; its uncovered paths widened; **the guarded-delete case retargeted and erased**; a failed deletion claimed successful; a blocked deletion claimed attempted; a diagnostic claimed. On the non-deleting arm: a deleting arm invented, and its uncovered paths invented |
| Prerequisite forgeries at both levels | **27** (pinned) | Each of the four identities emptied and two repeated (refused at import); a foreign identity in each of the four slots and two real identities exchanged (refused at derivation); three arm-agreement rows; and **thirteen decisive rows** — an *honest* identity set, an honest transported question and untouched composition contracts, over a prerequisite whose own contract is falsified, at **level one** (close cleanup, guarded delete, wrapper def/use), **level two** (the nested composition) and **level three** (the nested composition's own cleanup, counter and def/use prerequisites) |
| Every clause of both composition contracts | **60** (pinned) | Every `requires` and `ensures normal` clause of `file_close.qcore` (17) and `file_close_delete.qcore` (13) falsified two ways; each refuses |
| Transported profile vs the declaration | **15** (pinned) | Close profile: syscall, syscall signature, root type, presence field, field type, narrowed inventory, argument fields, errno helper, report helper. Delete profile: syscall, argument fields, field order. Nested profile: syscall, narrowed inventory. Plus the declared eligibility field |
| Task and witness binding | **6** (pinned) | Each arm's witness against the other's task in both directions; a witness rebound to a foreign task digest; another witness kind; a **leaf-family** proposal against a `file_close` task; a task the checker did not derive |
| Import and body binding | **3** (pinned) | An unadmitted callee in the wrapper; a substituted callee; a transported nested question that is well-formed but is not the one the nested composition imported |
| Moved source bytes | 2 | A change invisible to every checked figure (a renamed unrelated helper) still retires the task; neither context accepts the other's witness |

**No mutation was admitted. No gate was changed and no repair was required by
the matrix.**

**The decisive new row class**, and the point of the card: *a forged leaf
prerequisite under an honest middle certificate*. In the last three rows of
group two, every one of the four `file_close` identities is the identity the core
derives, the transported nested question is the core's own, both composition
contracts are untouched, and the nested composition's own identity is honest —
and the route still refuses, because re-establishing the nested composition
re-establishes *its* three prerequisites, one of which re-establishes *its* own.
The falsified contract sits three task levels below the witness being admitted.

Two measured findings the matrix produced, both kept:

* **The guarded-delete contract is a prerequisite of the deleting arm only.**
  Falsifying it must not — and measurably does not — disturb the non-deleting
  entry point, which makes no deletion attempt and names no delete prerequisite.
  The row is scoped to the arm it belongs to and a **positive control** asserts
  the non-deleting arm still admits its own honest reading under the falsified
  contract. The first draft ran the row over both arms and was corrected.
* **A substituted callee is refused at evaluation, not at derivation.** Deriving
  imports and binds; it does not replay. The row now asserts the whole route, so
  which stage refuses cannot move without notice.

The 60 contract-clause rows drive the relocated procedure directly, over the
certificates the honest route re-established for itself, rather than rebuilding
60 five-body Clang imports. What is measured is unchanged — the contract text the
core supplies decides — and the end-to-end direction is covered by §6.5's
false-contract rows through the real package.

### 6.4 Provider-side matrix — 6 tests

`providers/common/composition_effect_tests.rs`, driven over the **real**
projections the gateway emits for these bodies
(`providers/common/fixtures/file_close_{,delete_}composition_projection.json`,
both asserted live by a conformance test). Every refusal row is a *decline*: the
process returns no proposal, the host sees no witness, and the obligation stays
Unknown.

- the producer recomputes the whole two-level reading for both arms — 43 cases,
  uncovered `[1, 2]`, 42 backend-released, the nested case present exactly on the
  reached entries, both counter values, and 30 deleting cases with 8 successful
  deletions;
- **the nested reading is recomputed, not received**: truncating either of the
  nested question's own two bodies makes the whole two-level reading decline,
  though nothing at this level changed. A producer handed the nested conclusion
  would not have to look at those bodies at all;
- `P04-COMPOSE-A-CASE-INDEX` measured in both directions (§2.2);
- **7 nested degenerate rows** (pinned): the nested wrapper emptied, oversized,
  anonymous and revision-moved; the nested prerequisite oversized; the nested
  wrapper and prerequisite identical; and a nested question that is itself a
  `file_close` question;
- **9 outer degenerate rows** (pinned): the wrapper emptied, wrongly signed,
  truncated, duplicated, reduced to one path; the wrapper and close prerequisite
  identical; the close profile emptied; the delete eligibility field moved; the
  delete body emptied;
- **5 request-gate rows** (pinned): every cross pairing of the three semantic
  profiles with the three question shapes declines, so one family's question is
  never answered under another's profile.

The leaf family's existing 8-row request gate grows by one row: a projection
whose family and prerequisite set disagree declines before any replay.

### 6.5 End-to-end through the real built package

`providers/conformance/tests/file_close_composition_capability.rs` (2 tests). The
shipped `qcore.chunkio` **0.10.0** executable composes the real bodies for both
entry points; the core admits only after re-establishing every prerequisite at
every level, rebuilding both profiles from its own contracts and re-running its
own compositions. Also measured, for each arm:

- **fixture drift**: the committed provider fixtures are the live projections —
  schema version, all five bodies' complete path sets, the nested question and
  every declared profile compared exactly, and the transported nesting depth
  asserted at 2;
- **missing evidence**: no core checker → Unknown with **QP016** and no
  certificate; no registered provider → Unknown; the shipped `qcore.libevent`
  package, which does not implement this capability → Unknown with no
  certificate; a disabled provider → Unknown;
- **tampered transport**: a projection the core never imported is refused by the
  host's digest binding with **QP001**; rebinding it to a recomputed task digest
  gets past that and is then refused by the core's own comparison, with no
  certificate;
- **false contracts at all three levels**: a Safe proposal over a falsified
  *composition* contract is rejected with **QP009**, and one over a falsified
  *nested composition* contract or a falsified *leaf prerequisite* contract is
  rejected too, with the outer contracts untouched.

No host change was needed: `verify_v2`'s QP001 arm for the `composition_source`
projection already exists (P04-COMPOSE §5.5 added it), and this card reuses the
same projection field.

### 6.6 Wire compatibility

`crates/qcore-schemas/tests/composition_request_wire.rs` grows from 5 to **9
tests**: a request carrying none of the seven projections still serializes with
none of the seven keys and round-trips; all three question shapes round-trip and
carry no derived reading (a seventeen-name absence list); the nesting depth is
measured from the question; the two entry points are distinct questions with
distinct digests and absent-not-empty delete fields; unknown fields are refused
in every nested record **including inside the nested question**; a family never
borrows another family's prerequisite set, in either direction, with the deleting
halves required to agree and a three-level question refused; the request
admission rule is exact for all three shapes; and the identity rules are
non-vacuous in both families, with all seven digest-key constants distinct.

The superseded projection version is refused **in both directions**: version 1
and version 3 both fail request validation. The ninth test is
`composition_vocabulary_is_pinned` (§1.1).

### 6.7 Lean

`proof/QCore/CompositionAdmission.lean` grows by a `P04-COMPOSE-A` section of
**15 audited theorems** (32 → 47 in the namespace), with **no new axiom** — every
one depends on `propext`, `Classical.choice` and `Quot.sound` only, and six on
none at all.

- *the two-level re-establishment is total* (1): `two_level_revalidation_is_total`
  — re-establishing a nested composition answers on every (context, claim) pair
  and its answer is exactly the decided conjunction of the outer identity check
  and the **whole inner level**; there is no partial case;
- *a forged leaf refuses under an honest middle* (4):
  `a_forged_leaf_prerequisite_refuses_under_an_honest_middle_identity` states the
  decisive property — the claim is the identity the core derives, the nested
  composition's own composition succeeds, and it still refuses; the other two
  leaf prerequisites refuse the same way, and a nested composition whose own
  composition refuses is never established;
- *the middle identity is load-bearing on its own* (1): a foreign nested identity
  refuses even when the whole inner level holds;
- *order independence* (3): `all_id_is_permutation_invariant`,
  `establishing_the_obligations_is_order_independent` and
  `establishing_the_obligations_is_total` — the outer route's answer is the same
  however its three or four obligations are ordered, so the implementation's
  fixed order is a choice and not a load-bearing assumption;
- *the outer level* (3): a forged leaf refuses the whole outer route; the
  deleting arm's obligation is present exactly when the question has one, in both
  directions; and an honest outer route is established (non-vacuity);
- *the rules discriminate* (3): the outer distinctness rule including its optional
  fourth member, the honest two-level chain, and the nesting depth.

Judged **not applicable** and recorded rather than modelled, unchanged from
`P04-COMPOSE`: the transported vocabulary's faithfulness to Clang's typed AST (a
gateway/importer premise), and the composition bodies themselves, which are
unedited and whose figures are decided by the Rust tests of §6.1–6.5.

---

## 7. Corpus and audits

No gateway analysis rule changed (§5), and both retained in-process entry points
publish byte-identical certificates (§6.1), so the pinned 185-unit artifacts
cannot move from this card. The claim rests on those two measurements plus the
retained routes' own unit suites, which are unchanged and pass — including
`filesystem_chunk_close`'s, which consumes the deleting certificate through the
relocated procedure.

Every configured-audit route that runs **without** a prepared upstream clone did
run, unchanged and passing, as part of the workspace gate: the `chunkio-core`
second-source-set regressions, the root-source-set suite, the configured CLI
suite, the compile-option and compile-task-identity suites, and the
source-contract configuration suite. None of them moved.

**The 185-unit corpus and the canonical CMetrics audit were not re-measured
here**: both need a prepared pinned upstream clone that this worktree does not
have, and the audit's own pin gate refuses to run without one. This is the same
position `P04-B2`, `T02` and several earlier cards recorded. The integrator's
corpus consolidation should expect **no change from this card** beyond the
package-identity residue class (the ChunkIO `provider_version` and
`package_digest` leaves in `source-effects.json`, as in every card that bumps
that package).

The contract corpus is untouched at **61 modules**: no `.qcore` file was added or
edited.

---

## 8. Schema decision — flagged loudly

**`qcore-schemas` moves 10.0.0 → 11.0.0. This is a major schema bump, and unlike
the last six it moves an existing projection's wire bytes.**

Additive was preferred and is not available; §1 records why. Five of the changes
are source-incompatible for exhaustive matches and struct literals, and one is
wire-incompatible:

| Change | Kind |
|---|---|
| `CompositionSourceInput::prerequisites` retyped to `CompositionPrerequisiteSet` | **wire-incompatible** for the composition projection, and source-incompatible |
| `CompositionFamilyInput::FileClose` | new variant on an exhaustively matched enum |
| `CompositionProposalRecord::FileClose` | new variant on an exhaustively matched enum |
| `CleanupReplayOperationRecord::Return` | new variant on an exhaustively matched enum (§1.1) |
| `COMPOSITION_SOURCE_INPUT_VERSION` 1 → 2, key `:v1` → `:v2` | declared version move, refused in both directions |
| `FileClosePrerequisites`, `FileCloseDeleteInput`, the case/reading records, two profile constants | purely additive |

**Every other projection's wire bytes are unchanged and pinned.** `source_effect`,
`counter_effect`, `reactor_program`, `teardown_source`, `call_use_source` and
`cleanup_replay_source` are untouched, and a request carrying none of the seven
still serializes with none of the seven keys — asserted by the wire test.

The moved bytes are confined to the composition projection, which is one wave old
and has **no production caller**: the Cycle 19 audit still runs the in-process
adapters. The version field exists for exactly this, and it is used rather than
worked around: a version-1 witness cannot be replayed against a version-2 task,
and the digest key moves with it.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 10.0.0 | **11.0.0** |
| `qcore-verification` crate | 0.4.0 | 0.4.0 (unchanged; additive API) |
| `qcore-verifier-host` crate | 0.4.0 | 0.4.0 (unchanged; no host change) |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (unchanged; public API preserved) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Composition source input | 1 | **2** |
| Composition input digest key | `…:v1` | **`…:v2`** |
| Composition semantic profiles | 1 | **3** |
| ChunkIO package | 0.9.0 | **0.10.0** |

### 8.1 Deliberate pin updates, each with its reason

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.9.0 → 0.10.0. The two
   new profiles are reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other manifest field are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version in
   its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/Cargo.{toml,lock}` — schemas 11.0.0 so
   the pinned standalone-provider fixture's `--locked` build resolves.
4. `providers/common/fixtures/unmap_wrapper_composition_projection.json` —
   regenerated for schema version 2 and the tagged prerequisite set. Its bodies,
   profile and identities are unchanged in content; only the version number and
   the `unmap_wrapper` wrapper key move.
5. `crates/qcore-schemas/tests/composition_request_wire.rs` — the projection
   literal is rewritten for the new shape, and the version row now refuses
   version 1 rather than version 2. **The load-bearing negative halves are all
   kept and extended**: unknown-field refusal, admission exactness, and identity
   completeness/distinctness all grow rows rather than losing them.
6. `crates/qcore-verification/tests/composition_source.rs` — the leaf
   composition's pinned digest-key list moves `…-source-input:v1` to `:v2`, with
   the reason recorded beside it. The other nine keys and every other assertion
   in that suite are unchanged, and the leaf route's own behaviour is unchanged
   apart from the key name. This pin caught the version move rather than
   inheriting it.

All five provider packages were rebuilt and verified. The contract corpus is
untouched at 61 modules.

### 8.2 One Python failure the card cannot fix, by instruction

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
fails: the committed `docs/reviews/r0_3/acceptance_matrix.json` carries
`['10.0.0', 62]` and the freshly derived value is `['11.0.0', 62]`. The card's
constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 10.0.0 →
**11.0.0**. **`validation.lean_audits` does not move** — this card extends the
existing `AuditCompositionAdmission.lean` rather than adding a new audit file, so
the count stays 62 and the matrix's derived-facts test will pass on that half.
Regenerate the matrix (or supply `--gate`) at the wave's close. Nothing else in
the matrix moves from this card: the contract corpus stays 61 modules, CT
coverage is untouched at 32/50, and no verdict moves.

---

## 9. Open items and named boundaries

- **`P04-COMPOSE-A-CASE-INDEX`** (new) — this level publishes each prerequisite's
  case index over a fixed sampled prestate domain, not its selected source path,
  so permuting a prerequisite body's paths is invisible here and is caught one
  layer down by that prerequisite task's own source binding. Measured in both
  directions and pinned (§2.2).
- **`P04-COMPOSE-A-CLOSE-PROFILE-SHARED`** (new) — the close prerequisite's
  record profile and the nested composition's are both read from
  `filesystem_outcomes.qcore`, under different policies. The route binds both
  policies and both derived profiles under separate keys, so a change to either
  declaration retires the task; but a single contract file is a single pinned
  input, and a change to it moves both. Recorded, not repaired.
- **`P04-COMPOSE-FRAME`** (inherited, unchanged) — `memory_close` is still
  blocked on the frame-helper `ChunkFrameSummary`, which has an importer and a
  contract evaluator but no verification task, so it cannot be named by a
  `PrerequisiteIdentity`. With `file_close` moved, `filesystem_chunk_close` is
  now blocked on **only** that: its other prerequisite has landed. Recorded as
  the next unit, **`P04-COMPOSE-FS`**, which is unblocked the moment
  `P04-COMPOSE-FRAME` is.
- **`P04-COMPOSE-CALLEE-DECL`** (inherited, now larger) — the wrapper callee
  tables are caller assertions on this route, because neither
  `unmap_wrapper.qcore` nor `file_close.qcore` declares a callee vocabulary.
  `FILE_CLOSE_CALLEES` is a five-entry closed table matched on both name and
  declared C signature, mirroring the trusted composition's own call table. It
  buys no authority — the core runs its own compositions over its own imports and
  contracts regardless — and declaring both (as P04-B1 did for the cleanup
  families) would make them contract-derived.
- **`P04-COMPOSE-SHARED-PATHS`** (inherited, extended by §1.1) — the transported
  bodies reuse `CleanupReplayFunctionRecord` and its step/expression enums, which
  this card widened by one variant. The widening is confined by explicit refusals
  on four sides and pinned by test, but the enums remain shared.
- **`P04-B2-PREREQ-IDENTITY-CWD`** (inherited, unchanged) — a prerequisite
  identity is not a content-only function of the body: `SourceInputIdentity` binds
  `current_dir()` by design. The conformance drift test pins this as a measured
  fact for all four identities.
- **`SourceMatch` (P04-NEXT-C)** — `nested_memory` and `nested_cleanup` stay
  behind the sealed reviewed-library boundary, unchanged.
- Nothing about the OS is established. A recorded `close`, `munmap` or `unlink`
  call is a recorded call; whether a descriptor was closed, a mapping torn down
  or a file unlinked is outside every claim here, freeing the C path allocation
  is never accepted as successful backing deletion, and releasing the backend's C
  allocation never discharges an OS cleanup obligation. The sync, logging and
  `free` frames remain explicit unproved external-call premises of the pinned
  contracts, recorded in the tasks' assumptions.
- The in-process adapters are **retained** (hard invariant 3). Their removal
  would need the P09-style archived parity measurement plus a production caller
  for the process route; §6.1 supplies the parity half only — and it is stronger
  than a parity record, because both routes call one implementation.
- The moved capabilities have **no production caller yet** — they are exercised
  by these tests, not by the Cycle 19 audit, which still runs the in-process
  adapters. Same position as P05, P04-B2 and P04-COMPOSE.
- **`P04-COMPOSE-RELOCATE`** (inherited, narrowed) — the relocation moved
  `file_close`'s decision procedure into `qcore-verification`; `memory_close`,
  `filesystem_chunk_close`, `unsupported_close` and the two nested modules remain
  compiler-side ChunkIO adapters. Consolidating them follows their capability
  moves, not this card.
- Of P04-NEXT's original thirteen in-process ChunkIO capabilities, six
  non-composition capabilities have moved plus **two composition modules** —
  eight routes across five vocabularies, with this card's module contributing
  two semantic profiles and two obligations. The remaining classification is
  `P04-COMPOSE` §1's table with `file_close` struck off and
  `filesystem_chunk_close` promoted to "blocked on the frame helper alone".

---

## 10. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | *(see §10.1)* |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | *(see §10.1)* |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | *(see §10.1)* |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.9.0 → **0.10.0** |
| `python3 -m unittest discover -s scripts/tests` | 125 tests, **1 failed** — the acceptance-matrix derived fact, §8.2 |
| 185-unit corpus | not re-measurable here — §7 |
