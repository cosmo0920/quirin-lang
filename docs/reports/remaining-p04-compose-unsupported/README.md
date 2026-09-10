# P04-COMPOSE-UNSUPPORTED — the `unsupported_close` symbolic classification through the process boundary

Workstream W5 (Providers), the follow-on named by
[`docs/reports/remaining-p04-compose-fs/README.md`](../remaining-p04-compose-fs/README.md)
§9.1 and sized out in its §9.2:

> Moving it is therefore a fifth family — a new proposal record, a new producer
> replay arm carrying the symbolic domain, and its own mutation matrix — not an
> extension of this one.

That sizing was correct in every particular, and this card does exactly those
three things. It is the **fifth and last reachable composition module** to
travel, the first family that **imports no body of its own**, the deepest
question this vocabulary carries (four levels), and the only one whose published
conclusion is an explicit **`Unknown`**.

- Worktree cut from `main` at **`8ec8402`** (wave-16 consolidation); branch
  `worktree-agent-a7120a63454539e44`.
- **No checker rule, diagnostic code or verdict changed.** No `Q`-code was
  added, changed or reordered; no `QP`-code was added. No raw unit is promoted,
  no CT case moves, and no `.qcore` file was added or edited, so the contract
  corpus is untouched at **67 modules**.
- The in-process adapter is **retained** (hard invariant 3) and now *delegates*
  to the same relocated decision procedure the moved route runs, so the two
  cannot drift.

---

## 1. Vocabulary decision — extended a fourth time, and purely additively

**Decision: extend `qcore-schemas::composition_summary`, the fifth transported
source vocabulary. No sixth vocabulary was added.**

The decision was made from the shape, as the lineage requires. The FS card's
sizing said this family "needs a fifth proposal record carrying the symbolic
other-backend class" — and that is precisely what it needed, and all it needed.
A fifth proposal record is not a new mechanism: the family tagging
`P04-COMPOSE-A` introduced already carries a record of a different shape per
family, and this is that mechanism used once more.

**The extension is purely additive at the wire.** Nothing existing is retyped,
no existing family's bytes move, and `COMPOSITION_SOURCE_INPUT_VERSION` **stays
2** with its digest key unchanged — so the four already-moved composition
routes' task digests do not move at all. The additions are:

| Addition | Kind |
|---|---|
| `CompositionPrerequisiteSet::UnsupportedClose` | new variant on an exhaustively matched enum |
| `CompositionFamilyInput::UnsupportedClose` | new variant on an exhaustively matched enum |
| `CompositionProposalRecord::UnsupportedClose` | new variant on an exhaustively matched enum |
| `UnsupportedClosePrerequisites`, `UnsupportedCloseCompositionRecord`, `UNSUPPORTED_CLOSE_SEMANTIC_PROFILE`, `CompositionPrerequisiteSet::unsupported_close()` | purely additive |

**No accessor changed shape.** `nested()` already returned a vector after
`P04-COMPOSE-FS`, so this family's one nested question needed no signature
change — the previous card's cost is this card's saving. No new
`ObligationProperty` and no new `SemanticWitness` variant were needed, and the
host's `verify_v2` QP001 binding for the `composition_source` projection already
exists, so **no host change was needed**.

### 1.1 The structural novelty: a question with no body of its own

`CompositionFamilyInput::UnsupportedClose` carries **exactly one field**: the
nested filesystem question, verbatim. Nothing else.

That is not minimalism for its own sake. Both bodies this classification reads —
the `cio_chunk_close` wrapper and the filesystem discriminant probe — are
already inside that nested question, the wrapper two levels down inside its own
nested memory question. Restating either here would have let one question name
two bodies, which is the exact failure `P04-COMPOSE-FS` avoided by delegating
its wrapper. So:

* `wrapper()` delegates through the nested question (and a test pins that it
  equals the memory question's wrapper two levels down);
* `derivation()` returns `None` — this family derives no text at all;
* `probe()` returns `None` — its probe is the nested question's;
* `native()` and `native_profile()` return `None`.

The gateway importer is body-less to match: `import_unsupported_close_source`
imports nothing, and captures only the base unit's identity so the route can tie
its task to a translation unit. `CompositionFamily::is_body_less()` names the
class, and `import_composition_source` refuses this family by name — for the
*opposite* reason it refuses the two derived families: not because it must build
its own text first, but because it has no body to import and a wrapper selected
there would be a second naming of one the nested question already carries.

### 1.2 The proposal record is the thinnest of the five, deliberately

`UnsupportedCloseCompositionRecord` carries the wrapper's identity triple, the
two excluded constants, the one selected path, and the profile assumptions.
Nothing else.

The discipline every record in that module follows is that **a figure the
checker does not decide has no place in it**. The certificate this arm publishes
decides exactly two figures — the excluded pair and the selected path. The six
release facts (`primary_unlinked`, `state_unlinked`, `name_release_attempts`,
`chunk_released`, `total_counter_decrements`, `backend_calls == 0`) are
*constants of the admission*: `unsupported_path` refuses the reading outright
unless all six hold and exactly six calls resolve, so publishing them would have
manufactured agreement rather than measured it. The `Unknown` status is a
constant of the arm. Neither is in the record, and §4.6 pins that a conclusion
smuggled into the record is refused at parse time.

The delete-independence fact is decided (`check` computes the path under both
flags and refuses unless they are equal) but is **not** published, for the same
reason: it is a refusal, not a figure, and a second `source_path` field would
have been tautologically equal to the first on every admitted reading.

---

## 2. What actually crosses the boundary

`CompositionSourceInput` for a symbolic question carries three things:

1. `schema_version` (still `2`);
2. `prerequisites` — an `UnsupportedClose` set of **one** named
   `PrerequisiteIdentity` triple (`filesystem_chunk_close`), a lookup key;
3. one `CompositionFamilyInput::UnsupportedClose` carrying the whole nested
   filesystem question — which carries, in turn, the whole nested memory
   question, the whole nested deleting `cio_file_close` question, that
   composition's own nested unmap question, both probe bodies and both
   derivation records.

**No composed result and no prerequisite conclusion travels, at any of the four
levels.** No case vector, no selected path, no case index, no probed
discriminant, no excluded pair, no residual classification, no deletion outcome,
no verdict, no contract text, no policy name — and **no classification word**:
the string `Unknown` does not appear anywhere in the request. A composed
certificate has no wire representation on this route at all. Tests assert the
absence on the real transported request, in the core suite and end to end
through the real package.

### 2.1 One prerequisite, not two, and the reason is measured

The set names the filesystem composition and nothing else. It would have been
easy to name the memory composition beside it — this classification reads the
memory certificate, after all — and it would have been wrong:

> The filesystem composition already re-establishes the memory composition, as
> its own prerequisite, at every level. A second claim about it here could
> **disagree** with the first, and the route would then have had to decide which
> to believe. Naming one prerequisite means there is nothing to reconcile.

The cost of that decision is visible and is recorded rather than hidden:
`UnsupportedClosePrerequisites::are_distinct()` is **vacuously true** on a
one-member set. Two tests pin the vacuity explicitly — one in the core suite,
one at the wire — so nobody later reads that rule as a check that carries weight
here. What carries the weight is `is_complete()` and the equality against the
identity the core derives.

### 2.2 The unit tie: an equality, pinned in both directions

This route's own prerequisite **is** over the wrapper's own text — unlike the
filesystem family's backend half — so the tie is an equality, and it is checked
as one:

```
prerequisites.filesystem_chunk_close.task().unit.source_digest
    == import.base_identity().raw_source_digest()
```

Both directions are measured, not assumed. `the_prerequisite_is_checked_over_this_very_unit`
asserts the equality holds on the honest task, asserts the *backend* composition
two levels down is over a genuinely different unit (so the equality is not
trivially true of everything in the stack), and asserts that a nested filesystem
question built over a different chunk unit cannot reach a task at all — the
importer compares **both** derivations' base digests against the unit it was
handed, so a nested question assembled from two different units cannot pass by
matching on its filesystem half alone.

**The cross-unit premise is inherited, not restated.** `P04-COMPOSE-FS-CROSS-UNIT`
— that the linker resolves the wrapper's `cio_file_close` frame to the body
checked in the other translation unit — arrives here through the E03-B premise
union, and a test asserts it is present in this task's published assumptions. It
is one level down and it is not this route's to make; restating it would have
claimed this route had checked something it had not.

### 2.3 The symbolic class is what the guards fix, not a free parameter

This is the card's sharpest measured finding, and it came out of a failing test
row rather than out of design (§4.4).

The excluded class is not chosen by the two probes alone. It is fixed jointly by
the two probes **and the wrapper's own guards**. The wrapper compares the
stream's discriminant against the two macro expansions; the replay carries the
class `OtherBackend(memory, filesystem)`; and an equality against an integer the
class contains is *undecided* — that integer is inside the class and outside it
both. So a producer that claimed a class excluding 7 and 9, over a body whose
guards compare against 1 and 0, leaves `type == 1` undecided and the replay
**declines**. Measured directly:

```
unsupported_path(wrapper, callees, 1, 0, false) == Some(3)   // the honest class
unsupported_path(wrapper, callees, 7, 9, false) == None      // guards say 1 and 0
unsupported_path(wrapper, callees, 1, 9, false) == None      // one constant moved
unsupported_path(wrapper, callees, 7, 0, false) == None      // the other moved
```

A composition that sampled a representative would have answered all four.

---

## 3. Where the authority is: six things the core does for itself

`UnsupportedCloseCompositionSourceTask::derive` and `evaluate` do all of the
following before any proposal is compared.

1. **The transported nested question is the core's own.** It must be, verbatim,
   the family input the core's own filesystem composition context imported. That
   one comparison carries both derivation records and all three lower questions,
   because they are inside it — and a test pins that rather than assuming it.
2. **The claimed identity is re-derived, never read.** The core re-derives the
   filesystem prerequisite task from its own import and refuses unless the
   claimed triple *is* the identity it derives. Each of the three fields is
   load-bearing, measured.
3. **The prerequisite is re-established at every level.**
   `FilesystemChunkCloseCompositionSourceTask` gains
   `prerequisite_identity`/`revalidate_as_prerequisite` for exactly this, and
   `revalidate_as_prerequisite` re-runs that composition's **whole** evaluation
   — which re-establishes the memory composition (four prerequisites) and the
   deleting `file_close` composition (three plus a nested unmap composition) in
   turn. Four composition levels, all of them the core's own work.
4. **The unit tie**, as an equality, in both directions (§2.2).
5. **The classification is the core's own.** `evaluate` runs the unedited
   relocated `check_unsupported_close_bodies` over the certificates its own
   re-establishment produced, under its own pinned contract, its own pinned
   revision and its own probe-function constant. That re-runs the memory
   certificate's own recheck a further time before deciding anything.
6. **Acceptance terminates in the core's own reading**, and only then requires
   the proposal to be that same reading. Agreement alone discharges nothing;
   §4.3's decisive rows measure both halves, at two different depths.

### 3.1 The published premise set is the union, per E03-B

`composition_premise_union` is reused unchanged: the published set is the union
of the component's premise set and this route's own, and a key the two disagree
on refuses rather than being silently resolved. This route's own premises are
two, and both are about the class rather than about the body:

* `symbolic_backend_class_excludes_exactly_the_two_source_probed_constants`
* `int_backend_domain_is_exhausted_by_the_two_constants_and_their_complement`

The second is the load-bearing one and is stated as the assumption it is: that
the `int` domain is exhausted by the two constants plus their complement is what
makes the three-arm classification total, and this route does not prove it.

A test asserts every component premise survives, that the filesystem half's
cross-unit premise is among them, and that the published set is a strict
superset of the component's.

### 3.2 Digest keys the task binds

Six keys, and a test pins that they are exactly the task's `contract_sources`
with six distinct 64-hex digests:

| Key | Binds |
|---|---|
| `qcore:composition-source-input:v2` | the complete projection |
| `qcore:composition-unsupported-base-import:v1` | the base unit's captured identity |
| `qcore:composition-unsupported-contract:v1` | this composition's own pinned contract |
| `qcore:composition-unsupported-selection:v1` | the rest of the core's own selection — the pinned revision and the filesystem probe's function name — which the contract's bytes do not carry and which decide **which body** the classification will read at all |
| `qcore:composition-unsupported-fs-question:v1` | the nested filesystem question, verbatim |
| `qcore:composition-prerequisite-unsupported-filesystem:v1` | the one prerequisite identity |

**One derivation key, not two, and that is a deliberate difference from
`P04-COMPOSE-FS`.** This family derives no text, so it owns neither derivation;
both live inside the nested question, which is bound verbatim under its own key.
A moved recipe at either level therefore moves *this* task's digest through that
key, and a test asserts both derived digests are inside the bound bytes. Binding
them again separately would have been redundant rather than stronger.

The nested *question* and the nested *task* stay two separate keys, for the
reason `P04-COMPOSE-A` established: a moved question and a moved task are two
different retirements.

The `selection` key is new to this card and is worth naming. Without it two
different probe-function selections would give one task digest even though one
of them refuses; with it they are two tasks, and a test measures both halves
(the substituted selection refuses at the identity comparison, *and* the two
task digests differ).

---

## 4. Evidence

### 4.1 Byte-identical certificate parity with the retained in-process route

`compiler/tests/unsupported_close_route_parity.rs` (3 tests).

| Check | Result |
|---|---|
| in-process `check_unsupported_close_source` certificate vs moved `evaluate()` certificate, as **serialized bytes** | **byte-identical** |
| the published shape still carries `"status":"Unknown"` and `"excluded_kinds":[1,0]`, and does **not** carry the retained contract selection | asserted |
| both routes agree on the certificate over three different excluded-constant pairs — `(1,0)`, `(7,9)`, `(i32::MAX,42)` | **byte-identical** on all three |

**Twelve refusal-agreement rows**, count pinned: four about the *symbolic
domain* (a truth test on the class, a negated truth test, an equality against an
undecided integer, a self-comparison) and eight about the release chain (an
early return, a stray `free`, either backend dispatch, a stray unlink, a stray
counter decrement, a delete-dependent branch and a backend-field write). Each
row asserts the retained route really refuses **and** that the moved route
refuses it too. The two constants are moved off zero first, so `0` is genuinely
inside the class and the truth-test rows are not decided by accident.

Nothing in this card makes a previously refused body admissible.

### 4.2 Core-side positive suite — 15 tests

`crates/qcore-verification/tests/unsupported_close_composition_source.rs`,
driving the real gateway imports, the real filesystem composition task
derivation and the real trusted classification:

- the six bound-input keys are exactly the task's contract sources, with six
  distinct digests;
- exactly one obligation, `CompositionContract` under `ResourceContracts`, with
  its exact subject, scope and `PathBounds::BoundedPaths`;
- **the question carries no body, no probe and no derivation of its own**, its
  wrapper *is* the nested memory question's, and it nests **four** levels;
- the bound nested question carries **both** derivation records, over one base
  text, under two different recipes, and the base text is the unit this task
  names;
- the core reads the symbolic class, excludes `[1, 0]`, selects the one wrapper
  path the filesystem arm leaves *combined-uncovered*, publishes `Unknown` and
  refuses the nested discharge; `recheck` reproduces the evidence;
- the request carries the question and none of seventeen named derived figures
  — including the string `Unknown` — and the other six projections stay absent;
- the published premise set is the union of the component's and this route's
  own two, and carries the inherited cross-unit premise;
- the prerequisite is re-established at every level (75 filesystem cases, 2
  memory checked paths, 43 non-deleting and 30 deleting backend cases, and the
  backend half's own nested unmap composition's 8 cases);
- the prerequisite identity is the three-field key, each field load-bearing;
- **the one-member set's distinctness rule is vacuous**, pinned as such;
- deriving twice gives an equal task, identity and reading;
- the checker admits the core's own reading and refuses a foreign task first;
- **an admitted proposal still publishes `Unknown`** — the coverage it
  discharges is this task's own, the certificate still refuses the nested
  discharge, and the proposal record has no field that could say otherwise;
- the prerequisite is checked over **this very unit**, pinned in both
  directions (§2.2).

### 4.3 Core-side mutation matrix — 10 tests, **69 rows, all refused**

`crates/qcore-verification/tests/unsupported_close_composition_mutations.rs`.
Every forged reading is asserted to actually differ from the honest one
**before** it is submitted, and every row count is pinned so a row cannot
silently vanish.

| Group | Rows | What is mutated |
|---|---|---|
| Every figure of a proposed reading | **15** (pinned) | function, revision, signature; each excluded constant inflated, zeroed, set to `i32::MIN`, collided with the other and exchanged; the selected path retargeted, zeroed, set out of range and **retargeted onto a path the filesystem arm covers**; an invented profile assumption |
| Family and witness binding | **5** | a **filesystem-family** and a **memory-family** proposal against a symbolic task; a witness rebound to a foreign task digest; another witness kind entirely; and **the reverse masquerade** — this arm's own reading offered to the filesystem composition's checker |
| Prerequisite forgeries | **4** (pinned) | the identity emptied; a foreign identity; **the nested memory composition's own real identity** (a real triple, of the right shape, naming a real task one level too low); the honest identity under another semantic profile |
| **Decisive prerequisite rows** | **2** | an honest identity, an honest nested question and an untouched composition contract, over a prerequisite whose **own** pinned contract is falsified — once for the filesystem composition one level down, once for the memory composition **two** levels down |
| Nested questions | **5** (pinned) | a foreign filesystem question; the memory question; the deleting backend question; and each of the honest question's own two nested halves hoisted into the top slot |
| Importer gates | **3** | another family's prerequisite set; an incomplete identity; a memory question as the nested half — each refused before anything is imported |
| **The symbolic domain** | **6** (pinned) | a truth test on the class, a negated truth test, an equality against an undecided integer, an equality against the largest `int`, a self-comparison, and an equality against **zero, which is inside the class** — each driven through the relocated decision procedure over real C, with an accepted control asserting the class really is `{7, 9}`-excluding so no row was decided by accident |
| Every clause of the composition contract | **24** (pinned) | all six `requires` and six `ensures normal` clauses falsified two ways |
| The probe selection | **2** | a probe function the imported body does not carry, refused at the identity comparison; and the two selections measured to be two different task digests |
| Isolation | **3** | an unread `-D` leaves the projection byte-identical and still retires the task, in both witness directions |

**No mutation was admitted. No gate was changed and no repair was required by
the matrix.**

One measured correction the matrix forced, kept rather than explained away:

* **"the profile assumptions emptied" was a vacuous row.** This fixture's
  wrapper declares no profile assumptions at all, so clearing the map is not a
  mutation. The pre-submission difference assertion caught it. The row was
  **replaced** — with the sharper "retargeted onto a path the filesystem arm
  covers", which needed the arm's real coverage measured first — rather than the
  assertion weakened, and the finding is recorded in the test's own comment.

### 4.4 Provider-side matrix — 5 tests, **31 rows**

`providers/common/composition_effect_tests.rs`, driven over the **real**
projection the gateway emits for these bodies
(`providers/common/fixtures/unsupported_close_composition_projection.json`,
asserted live by a conformance test). Every refusal row is a *decline*: the
process returns no proposal, the host sees no witness, and the obligation stays
Unknown.

- the producer recomputes the whole four-level reading: the memory-close
  composition, the two-level deleting `cio_file_close` composition, both
  discriminants out of two different probe bodies, the filesystem arm's own 75
  cases, and only then its own symbolic replay — and the path it selects is
  asserted to be the one the filesystem arm leaves combined-uncovered, which
  this process also recomputed, so the two really agree;
- **the symbolic domain, producer-side** (5 rows): the honest class reads; the
  reading does not depend on the delete flag; and three claimed classes the
  body's own guards do not decide are declines (§2.3), plus a collapsed class;
- **15 degenerate rows** (pinned): the memory question and the backend question
  in the nested slot; the symbolic question nested inside itself; an emptied and
  a wrongly signed filesystem probe; a derivation that derives nothing and one
  naming another function; the two derivations sharing a recipe and over
  different base texts; a memory wrapper with no complete path and one with
  duplicated paths so the class would select two; a memory question whose callees
  collapse; the **non-deleting** backend arm; a backend wrapper with no complete
  path; a backend revision disagreement. Twelve of the fifteen break the
  *nested* question deliberately — the symbolic arm inherits every refusal that
  matrix already carries, and this measures the inheritance rather than assuming
  it;
- **11 request-gate rows** (pinned): another profile; the filesystem question
  and the memory question under this profile; no projection; another capability;
  an unadmitted dialect; an unsupported version; a future version; an incomplete
  identity; another family's prerequisite set; the filesystem family's
  prerequisite set;
- and the honest question **is** answered, with the proposal asserted equal to
  the reading the producer recomputed — never anything it was handed — and
  asserted to contain none of five conclusion words.

### 4.5 End-to-end through the real built package — 2 tests

`providers/conformance/tests/unsupported_close_composition_capability.rs`.
The shipped `qcore.chunkio` **0.13.0** executable classifies the real bodies;
the core admits only after re-establishing the whole four-level stack and
re-running its own classification. Also measured:

- **fixture drift**: the committed provider fixture is the live projection —
  schema version, the whole nested question in full, and **exactly** both
  derivation records inside it, which unlike a prerequisite identity *are*
  functions of the text alone. The one triple is checked for shape and semantic
  profile only, under the inherited boundary `P04-B2-PREREQ-IDENTITY-CWD`;
- **what an admission buys**: the certificate the core evaluated still publishes
  `Unknown`, still refuses the nested discharge, and its selected path is the
  one the admitted record names;
- **missing evidence**: no core checker → Unknown with **QP016** and no
  certificate; no registered provider → Unknown; the shipped `qcore.libevent`
  package, which does not implement this capability → Unknown with no
  certificate; a disabled provider → Unknown;
- **tampered transport**: a projection the core never imported is refused by the
  host's digest binding with **QP001**; rebinding it to a recomputed task digest
  gets past that and is then refused by the core's own comparison, with no
  certificate;
- **false contracts**: a Safe proposal over a falsified *composition* contract is
  rejected with **QP009**, and one over a falsified *prerequisite* composition's
  contract **two levels down** — with this classification's own contract and the
  filesystem composition's own contract both untouched — is rejected too.

### 4.6 Wire compatibility

`crates/qcore-schemas/tests/composition_request_wire.rs` grows from 18 to **22
tests**. The symbolic question round-trips and carries none of thirteen named
derived figures at any level; its subject is the memory wrapper two levels down;
it derives nothing and imports no probe; it nests four levels with one nested
question; it never borrows another family's prerequisite set in either
direction, against all four other families; a nested question of the wrong
family is refused, and so is a nested filesystem question whose backend arm is
the non-deleting one; its one-member set's distinctness is pinned **vacuous**;
**all fourteen prerequisite digest keys across the five families are distinct**;
and a conclusion smuggled into the proposal record is refused at parse time.

**One deliberate pin update.** `composition_vocabulary_is_pinned`'s family-tag
list grows from four entries to five. **Its load-bearing negative halves are
untouched**: the nine transported operation forms and eight expression forms are
asserted unchanged, so this card widened **no** shared enum — the boundary
`P04-COMPOSE-SHARED-PATHS` asked for exactly that check and it fired correctly.

**One boundary found by measurement and recorded, not asserted away.** The
family enum is internally tagged, and serde does not apply `deny_unknown_fields`
to an internally-tagged variant's own object, so an unrecognised key *beside*
the `family` tag is silently dropped. That is true of **all five** families, not
just this one; the test pins it in two places (this family and the filesystem
family) so the limitation is visible and cannot be mistaken for this card's
doing. It buys a producer nothing — a dropped key is not in the projection the
core digests, so the digest, the task and the comparison are all unaffected, and
the test asserts the parsed value equals the honest one exactly. Named
**`P04-COMPOSE-TAGGED-UNKNOWN`**.

### 4.7 Lean — 37 audited theorems, no new axiom

`proof/QCore/SymbolicClassAdmission.lean` and
`proof/AuditSymbolicClassAdmission.lean` add the
`QCore.SymbolicClassAdmission` namespace. Every theorem depends only on
`propext`, or on nothing at all. `check_lean_audits.py` moves 72 → **73** audits.

- *the class is a class, not a sampled value* (5): it **never** decides an
  equality true; it decides `false` for exactly the two excluded constants; an
  equality against any integer it contains is **undecided**; a truth test
  refuses exactly when `0` is inside it, and is decided only when one excluded
  constant is zero (in which case the whole class is nonzero);
- *both masquerade directions* (5): a supported discriminant is never
  classified symbolic; no symbolic member is ever classified supported; the
  symbolic arm is **exactly** the class, as an iff; collapsed constants hide the
  filesystem arm entirely; and distinct constants reach both supported arms, so
  requiring them to differ is not a vacuous requirement;
- *fail-closed admission* (12): admission is decided; a refusing core, a foreign
  family tag, a foreign identity, a foreign nested question, a refusing
  prerequisite, a prerequisite over another unit, a collapsed class, a differing
  proposal, a moved excluded constant and a retargeted path each refuse on their
  own; and the symbolic tag is no supported family's;
- *what an admission means* (6): it establishes the composition and, through it,
  three levels below; it names the core's own question and identity; it
  establishes the unit tie and the symbolic tag; an admitted proposal **is** the
  core's own reading; that reading names a real class; two admitted proposals
  cannot disagree;
- *what it deliberately does **not** mean* (4): the arm publishes `Unknown` for
  every admitted proposal; **no proposal moves that conclusion**, because the
  conclusion is a function of nothing a producer sends; no admission ever
  permits the nested discharge; and the gathered conjunction carries both halves
  at once;
- *non-vacuity* (5): the honest proposal is admitted over the honest context;
  and the core-refusal, family-tag, unit-tie and collapsed-class refusals each
  really refuse on that same context.

Judged **not applicable** and recorded rather than modelled: the transported
vocabulary's faithfulness to Clang's typed AST (a gateway/importer premise),
whether either derived text preprocesses as its base unit does (an inherited
published assumption), whether the linker resolves the backend prerequisite's
wrapper to the body checked in the other unit (an inherited published
assumption), **whether any caller can reach the symbolic class at all** (the
card's own headline boundary, §9), the C effective-type rules that make the
discriminant an `int`, and the classification body itself, which is unedited and
whose figures are decided by the Rust tests of §4.1–4.5.

---

## 5. One decision procedure, not two

`unsupported_close`'s decision procedure — the probe identity check, the
excluded-pair extraction, the collision refusal, the two `unsupported_path`
replays and their equality, and the eighteen-fact contract evaluation — is
**relocated verbatim** into
`crates/qcore-verification/src/unsupported_close_composition.rs`. The only
changes to the moved body are:

* the constants it read from its former module scope (the pinned revision, the
  filesystem probe's function name and this composition's own contract) are now
  explicit parameters (`UnsupportedCloseContracts`);
* the import half is separated from the decision half
  (`check_unsupported_close_source` / `check_unsupported_close_bodies`), so a
  caller that has the bodies — a recheck, or a mutation that falsifies the
  pinned contract — does not re-run the whole three-level filesystem composition
  to ask the same question;
* three accessors are added or widened (`filesystem_kind_probe`, `status`,
  `reason`) so the retained suites and the mutation matrix can re-decide over an
  already-imported reading without reaching into private fields.

The symbolic replay itself — `memory_close_composition::unsupported_path` and
the `Value::OtherBackend` arms of `truth`, `value` and `run` — was **already**
in `qcore-verification` before this card, relocated by `P04-COMPOSE-MEMCLOSE`.
Not one line of it changed. That is why this card's core-side diff is as small
as it is, and it is worth saying plainly: the symbolic domain was already
core-side; what this card moved is the *question*.

`compiler/src/verifier_plugins/chunkio/unsupported_close.rs` becomes a
**36-line** adapter (from 131) that supplies the ChunkIO half and delegates. Its
public API is preserved: `CONTRACT`, `check_unsupported_close_source` and
`UnsupportedCloseEvidence` re-exported at the same paths, with `recheck`,
`memory_scope`, `source_path`, `excluded_kinds` and `require_nested_discharge`
unchanged — so `nested_cleanup` and the pinned Cycle 19 compatibility adapter
are **untouched**.

`UnsupportedCloseEvidence::recheck()` keeps its argument-free signature by
retaining the owned contract selection in a `#[serde(skip)]` field, so its
callers did not change and **the certificate's serialized shape is
byte-identical** — which the retained adapter's archived corpus artifacts depend
on and §4.1 measures.

One visibility widening, deliberate and recorded: `memory_scope` and `recheck`
were `pub(super)` inside the compiler's `chunkio` module and are now `pub` on
the relocated type, because the moved route and the mutation matrix live outside
that module. No new *capability* is exposed: both return a reading the
certificate already carried.

Nothing else moved and nothing was duplicated:
`crates/qcore-verification/src/{memory_close_composition,unmap_composition,file_close_composition,filesystem_chunk_close_composition,cleanup_outcome,counter_contract,call_use_contract,field_destructor,list_unlink_contract,frame_source,teardown_source}.rs`
are unedited apart from the two additive methods §3 names on
`FilesystemChunkCloseCompositionSourceTask`.

---

## 6. The gateway diff is one module

The whole `crates/qcore-c-gateway` change is `composition_source.rs`: one new
`CompositionFamily` variant, one predicate (`is_body_less`), one new sealed
import type, one new body-less importer, and four exhaustive-match arms.
`legacy_c.rs`, `legacy_summary.rs`, `typed_source.rs`, `borrowed_actual.rs`,
`aggregate_store.rs`, `may_alias.rs` and every other analysis file are
**untouched**. **No gateway analysis rule changed, so no unit verdict, function
verdict or diagnostic can move from this card.**

---

## 7. Schema decision — flagged loudly

**`qcore-schemas` moves 13.0.0 → 14.0.0. This is a major schema bump.**

Additive was preferred and, at the **wire**, was achieved: no existing
projection's bytes move, `COMPOSITION_SOURCE_INPUT_VERSION` stays 2 and the
digest key stays `…:v2`, so **no existing task digest moves from this card**.
What forces the major version is the Rust API, under the same documented policy
as 4.0.0, 5.0.0 and 7.0.0–13.0.0: three new variants on exhaustively matched
public enums, each of which breaks a downstream exhaustive `match`.

Unlike `P04-COMPOSE-FS`, **no accessor changed shape** — `nested()` already
returned a vector — so the API break is variants only.

| Boundary | Before | After |
|---|---|---|
| `qcore-schemas` crate | 13.0.0 | **14.0.0** |
| `qcore-verification` crate | 0.4.0 | 0.4.0 (additive API) |
| `qcore-verifier-host` crate | 0.4.0 | 0.4.0 (no host change) |
| `qcore-compiler` crate | 0.4.0 | 0.4.0 (every published name still resolves, now as a re-export — the precedent `P04-COMPOSE`, `-A`, `-MEMCLOSE` and `-FS` set) |
| Plugin protocol | 1 | 1 |
| Evidence version | 1, 2 | 1, 2 |
| Composition source input | 2 | 2 (**unchanged**) |
| Composition semantic profiles | 5 | **6** |
| ChunkIO package | 0.12.0 | **0.13.0** |
| Contract corpus | 67 modules | 67 modules (unchanged) |

**Deliberate pin updates**, each with its reason:

1. `providers/conformance/tests/packages.rs` — ChunkIO 0.12.0 → 0.13.0. The new
   profile is reached through the `chunkio` target, whose profile list is
   already `*`, so the certificate checker, Lean family, capabilities, limits,
   trust class and every other manifest field are unchanged.
2. `providers/conformance/tests/composed_five_providers.rs` — the same version in
   its shipped-package inventory.
3. `compiler/tests/fixtures/w3/provider/Cargo.{toml,lock}` and
   `providers/Cargo.lock` — schemas 14.0.0 so the pinned standalone-provider
   fixture's `--locked` build resolves.
4. `crates/qcore-schemas/tests/composition_request_wire.rs` — the family-tag list
   grows by one (§4.6), with both load-bearing negative halves kept.

All five provider packages were rebuilt and verified.

---

## 8. Gate

Run in this worktree on Linux with Clang available.

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **2778 passed, 0 failed, 8 ignored** — see §8.1.1 for the one environment failure. The base count at `8ec8402` was not re-measured (a second full build was not affordable on this host, §8.1.1), so the delta is stated as the 39 new tests §8.1 enumerates rather than as a subtraction |
| `cargo fmt --all -- --check` (both workspaces) | clean |
| `cargo clippy --all-targets -- -D warnings` (both workspaces) | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **73 audits passed** (72 → 73), no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace --no-fail-fast` | **122 passed, 0 failed** |
| `python3 -B providers/package.py build` + `qcorec plugins verify` × 5 | all five built and verified (exit 0); ChunkIO 0.12.0 → **0.13.0** |
| `python3 -B -m unittest discover -s scripts/tests` | 161 tests, **1 failed** — §8.3, the predicted matrix pin |

### 8.1 New tests

Thirty-two new workspace tests — 15 core-side positive
(`unsupported_close_composition_source.rs`, one of which is the fixture
generator and is inert without `QCORE_WRITE_UNSUPPORTED_FIXTURE`), 10 mutation
(`unsupported_close_composition_mutations.rs`), 3 route parity
(`unsupported_close_route_parity.rs`) and 4 wire
(`composition_request_wire.rs`, 18 → 22) — plus seven in the providers
workspace: 5 producer-side (`composition_effect_tests.rs`, 20 → 25) and 1
conformance pair (`unsupported_close_composition_capability.rs`, 2 tests).

### 8.1.1 Two environment failures, neither a code failure

Recorded rather than quietly re-run, because both cost real time and the second
is a standing hazard on this host.

1. **The Libevent submodule was never initialised in this worktree.** The
   workspace run's single failure was
   `libevent_submodule_matches_the_audited_revision`, whose pin gate correctly
   refused a `third_party/libevent` that did not exist — it reported the
   worktree's own commit where the pinned revision should have been.
   `git submodule update --init --recursive` (AGENTS.md's once-per-checkout
   step) fixed it, and the test passes in isolation at
   `d82464a277d0f42703702c4dfd9af6af38595a83`. Nothing in this card touches
   Libevent.
2. **The disk filled twice.** This host ran at 100% with under 300 MB free
   mid-run, and the first full workspace attempt died in the linker with
   `rustc-LLVM ERROR: IO failure on output stream: No space left on device`
   followed by `ld terminated with signal 7 [Bus error]`. Recovering needed
   `rm -rf target/debug/{incremental,examples,build}` and then a full
   `cargo clean` (32.3 GiB). This is the same constraint `P04-COMPOSE-FS` §8.2
   recorded, one step worse, and it is why §8.2 below could not be measured.
   One self-inflicted consequence is worth naming so the next card avoids it:
   the providers run's log was written under `target/`, so the `cargo clean`
   destroyed it mid-run and that whole suite had to be re-run. Measurement
   output belongs in a gitignored directory **outside** `target/`.

### 8.2 The corpus expectation

**Not measured here, and the reason is recorded rather than glossed.** This
host's disk is at 100% with 3.2 GB free; preparing the pinned Fluent Bit clone
and its CMetrics Clang CMake build is not possible in this worktree. This is the
same constraint `P04-COMPOSE-FS` recorded for the 185-unit corpus, one step
worse.

What *is* measured, and what the expectation rests on:

1. **No gateway analysis rule changed** (§6). The whole gateway diff is one
   module, and it adds an importer nothing existing calls.
2. **No contract file changed.** `contract/qcore/**` is untouched, so the corpus
   stays at 67 modules and the loader's rejected count does not move.
3. **The retained in-process route publishes a byte-identical certificate**
   (§4.1), measured on three different excluded-constant pairs, which is what
   the archived Cycle 19 artifacts embed.
4. The hermetic `crates/qcore-audit/tests/second_source_set.rs` route, which
   exercises the same configured-audit machinery from configuration alone, is in
   the workspace gate above and passes.

**The integrator's corpus consolidation should expect no change from this card
beyond the package-identity residue class** — the ChunkIO `provider_version`
(0.12.0 → 0.13.0) and `package_digest` leaves in `source-effects.json`, as in
every card that bumps that package, plus the schemas-14.0.0 package digests. Any
other leaf moving is a finding, not noise.

### 8.3 One matrix-file action item for the integrator

`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`
compares the committed `docs/reviews/r0_3/acceptance_matrix.json` against freshly
derived values. This card bumps the schema crate and adds one audited Lean
module, and the card's constraints forbid editing that file.

**Integrator action item:** `versions.qcore_schemas_rust_api` 13.0.0 →
**14.0.0** and `validation.lean_audits` 72 → **73**, with
`AuditSymbolicClassAdmission.lean` added to `validation.lean_audit_files`.
Regenerate the matrix (or supply `--gate`) at the wave's close, exactly as waves
9–16 did. Nothing else in the matrix moves from this card: the contract corpus
stays 67 modules, CT coverage is untouched at 32/50, and no verdict moves.

---

## 9. Open items and named boundaries

- **`P04-COMPOSE-UNSUPPORTED-REACHABILITY`** (new, and the headline one) — this
  route establishes what happens *if* a nonnull chunk's stream carries a
  discriminant outside both constants. It establishes **nothing** about whether
  any caller can produce one. The entry state is stipulated, the certificate
  says so in its own published prose, `require_nested_discharge` refuses
  unconditionally, and the Lean model proves the conclusion is `Unknown` for
  every admitted proposal and that no proposal can move it. Closing this needs a
  constructor-reachability capability that does not exist; it is not a gap in
  the transport.
- **`P04-COMPOSE-UNSUPPORTED-GUARD-FIXED`** (new) — the excluded class is fixed
  jointly by the two probes *and the wrapper's own guards* (§2.3), and the
  replay declines when a claimed class leaves a guard undecided. That is
  strictly stronger than "the two probes decide the class", which is what this
  card's first draft assumed and a failing test row corrected. It is **not** a
  proof that the guards and the probes read the same macros: a body whose guards
  compared against two literals equal to the two expansions would pass. The
  probes are the only tie to the macros, and that tie is the memory and
  filesystem families' business, one and two levels down.
- **`P04-COMPOSE-UNSUPPORTED-VACUOUS-DISTINCTNESS`** (new) — the one-member
  prerequisite set makes `are_distinct()` vacuous. Recorded and pinned by two
  tests rather than dressed up; completeness and the identity equality are what
  carry the weight here.
- **`P04-COMPOSE-TAGGED-UNKNOWN`** (new, and pre-existing across all five
  families) — an internally-tagged variant silently drops an unknown key beside
  its `family` tag (§4.6). It buys a producer nothing and the test pins both the
  drop and the resulting equality with the honest value, in two families.
- **`P04-COMPOSE-FS-CROSS-UNIT`** (inherited, one level down) — the backend
  composition inside this route's prerequisite is checked over a **different**
  translation unit, and linker resolution is a declared premise. This route
  publishes it in its premise union and does not restate it, because it did not
  check it.
- **`MEMCLOSE-BASE-TIE-TEXT`**, **`MEMCLOSE-DERIVED-PREPROCESSING`**,
  **`MEMCLOSE-PROBE-ISOLATION`**, **`MEMCLOSE-COUNTER-SUGAR`**,
  **`P04-COMPOSE-FS-TWO-RECIPES`**, **`P04-COMPOSE-FS-BACKEND-ARM`**
  (inherited, unchanged) — every derivation, probe and recipe premise this
  question depends on lives inside its nested question and arrives through the
  premise union. This family adds no derivation of its own, so it adds no new
  premise of that class.
- **`P04-COMPOSE-CALLEE-DECL`** (inherited, now covering five wrappers) — the
  filesystem dispatch callee is a caller assertion in the memory family's
  `fixed_callees`, not read off a declaration. Unchanged by this card.
- **`P04-FRAME-OBLIGATION-ID`** and **`P04-B2-PREREQ-IDENTITY-CWD`**
  (inherited, unchanged) — the shared obligation-identifier scheme is sound only
  via the identity triple, and a prerequisite identity is not a content-only
  function of the body. This route names one prerequisite by triple and the
  conformance drift test pins it for shape only, comparing **both derivation
  records** exactly because each *is* a function of its text.
- **`P04-COMPOSE-SHARED-PATHS`** (inherited, unchanged) — the transported bodies
  reuse `CleanupReplayFunctionRecord` and its step/expression enums. This card
  widened neither, and §4.6's pin asserts it.
- **`SourceMatch` (P04-NEXT-C)** (inherited, and now the *only* thing between
  this ladder and its top) — see §9.1.
- **Nothing about the OS is established.** A recorded absence of a backend frame
  is a recorded absence; the unrecognised backend's own custody is exactly what
  this arm publishes as unresolved.
- The in-process adapter is **retained** (hard invariant 3). Its removal would
  need the P09-style archived parity measurement plus a production caller for
  the process route; §4.1 supplies the parity half only — and it is stronger
  than a parity record, because both routes call one implementation.
- The moved capability has **no production caller yet** — it is exercised by
  these tests, not by the Cycle 19 audit, which still runs the in-process
  adapter. Same position as P05, P04-B2, P04-COMPOSE, P04-COMPOSE-A, MEMCLOSE
  and P04-COMPOSE-FS.

### 9.1 The composable-now census, closed

Of `P04-NEXT`'s original thirteen in-process ChunkIO capabilities, six
non-composition capabilities have moved plus **five composition modules**. The
remaining classification, closing `P04-COMPOSE` §1's table:

| Composition module | Class | Reason |
|---|---|---|
| `unmap_wrapper` | moved (P04-COMPOSE) | — |
| `file_close` (both entry points) | moved (P04-COMPOSE-A) | — |
| `memory_close` | moved (P04-COMPOSE-MEMCLOSE) | — |
| `filesystem_chunk_close` | moved (P04-COMPOSE-FS) | — |
| `unsupported_close` | **moved by this card** | — |
| `nested_memory` | excluded — sealed `SourceMatch` boundary | **re-measured**: it holds a `TraversalSource` and two `ParentSource`s, and each of those holds a sealed `SourceMatch` built by `typed_source_match::check_source_match`. Unchanged by this card |
| `nested_cleanup` | excluded — sealed `SourceMatch` boundary (transitively) | **re-measured**: it holds a `NestedMemorySource`, so the same boundary applies through it. Its `UnsupportedCloseEvidence` dependency is *not* a blocker and never was — it consumes the certificate, and that certificate's public API and serialized shape are unchanged by this relocation |

**Summary: five moved, two excluded behind `SourceMatch`. Every reachable rung
of the composition ladder has been climbed.**

The two remaining modules are not blocked by anything this vocabulary lacks.
They are blocked by `SourceMatch` being a sealed reviewed-library type with no
public constructor, conversion or deserialization — the boundary
`P04-NEXT-C` — and their removal needs a **reviewed-body attestation the core
can re-derive**, not a bigger vocabulary. That is a different kind of work from
the five cards that built this ladder, and it is where the next increment on
this line has to start.

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
QCORE_WRITE_UNSUPPORTED_FIXTURE=$PWD/providers/common/fixtures \
  cargo test -p qcore-verification --test unsupported_close_composition_source \
  write_the_provider_fixture
```

and the conformance drift test refuses a stale one.

---

## 11. Draft increment paragraph

> ChunkIO symbolic other-backend classification (P04-COMPOSE-UNSUPPORTED): the
> fifth and **last reachable** composition module moves through the process
> boundary, closing the ladder. It is the first family that imports **no body of
> its own** — its whole question is the nested filesystem question, verbatim,
> which already carries both bodies it reads and both derivation records — the
> deepest question the vocabulary carries (four composition levels, all
> re-established by the core), and the only one whose published conclusion is an
> explicit `Unknown` that no proposal can move, proved as such in Lean. Its
> reading is over a **class**, not a value: the excluded class is fixed jointly
> by the two probes and the wrapper's own guards, and a claimed class that
> leaves a guard undecided is refused rather than sampled — a measured
> correction to the card's own first draft, caught by a failing row and kept.
> Parity with the retained in-process route is byte-identical on the serialized
> certificate across three excluded-constant pairs; a 69-row core mutation
> matrix, a 31-row provider matrix and 12 refusal-agreement rows at the route
> boundary all refuse, including both masquerade directions and a falsified
> prerequisite contract **two** levels down. Three enum variants make
> `qcore-schemas` 14.0.0 with **existing wire bytes and every existing task
> digest unchanged**; no accessor changed shape, because `P04-COMPOSE-FS` had
> already paid that cost. ChunkIO package 0.13.0; 37 audited Lean theorems in a
> new `QCore.SymbolicClassAdmission` namespace. Two remaining modules stay
> behind the sealed `SourceMatch` boundary, re-measured rather than assumed: they
> need a reviewed-body attestation the core can re-derive, not a bigger
> vocabulary. No checker rule, diagnostic code, contract file or verdict
> changed; nothing is promoted.
