# `CT-CTWIN-LANE` — wave 35's CT mapping lane

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


**Result: CT46 mapped (45/50 → 46/50). CT41, CT42, CT10 and CT24 re-measured
and deliberately left unmapped.**

The lane was briefed to map **CT41** on the C-twin route as its primary
deliverable. It did not, and the first section says why: the brief's premise is
contradicted by a measurement already archived on this tree, and re-measuring
confirmed that measurement rather than overturning it. CT46 was mapped in its
place, on a capability card whose own report had declared CT46's blocker
discharged and left three questions for a mapping lane to answer.

Sanitized-native stays **9** and proof stays **3**; both denominators grow to
46. A grown denominator is mapping, not conformance (constraint 23).

---

## 1. CT41: the brief's primary deliverable, declined on measurement

### 1.1 What the brief asked for, and why it cannot be done

The brief said CT41's blocker "is recorded as *a decision*, and the decision has
been made", and asked for a mapping on `legacy_c_twin` with "both directions
asserted (the volatile and non-volatile twins produce the same verdict and the
same 'no synchronization' refusal)".

That last clause is the problem, and it is not a detail. The standard a C-twin
mapping is held to is **clause (b) of the wave-9 attribution rule**
(`docs/reports/wave9-ctwin/README.md`): the gateway's verdict on the twin must
be

> (a) the verdict class the scenario demands and (b) *attributable to the
> construct the scenario names* — shown by a control.

Attribution means a control in which **removing the named construct moves the
verdict**. The construct CT41 names is `volatile`. "The volatile and
non-volatile twins produce the same verdict" is therefore not a way of
satisfying clause (b) — it is the definition of **failing** it.

And the sameness is not an accident this lane could engineer around. It is a
**decision** the project has already taken, in card `CTWIN-PUBLISH` §5:

> C11 `volatile` is an access-ordering qualifier for the abstract machine, not a
> synchronization primitive: §5.1.2.4 gives happens-before to atomics, mutexes
> and thread creation/join, and to nothing else. A `volatile` completion flag
> establishes no order between a producer's store and a consumer's release, so a
> projection that derives **no** synchronization fact **must** answer
> identically with and without it.

So the obstruction is **structural, not a gap**. A gap is closed by later work;
this one cannot be, because the thing a mapping needs — a verdict that moves
when `volatile` is removed — is the thing the project has decided would be a
**defect**. Two prior lanes reached the same conclusion in these words: wave 17
recorded "CT41 stays unmapped because a vacuous mapping is worse than none", and
the wave-9 record already had it as "fails (b); the gateway has no
synchronization model, so no verdict of its is evidence about one".

### 1.2 Re-measured anyway, on this tree, over five shapes

The tree has moved a great deal since wave 17 (wave 31's `C-GATEWAY-WAVE`
changed scalar-assignment handling and narrowed the binding poison), so the
prior measurement was re-taken rather than cited. Ubuntu clang 21.1.8,
`/usr/bin/clang`, through `validate_unrestricted_c11` and
`analyze_unrestricted_c11` over the same synthetic `q_*` contract surface
CT40/CT43/CT45 use.

| Shape | Verdict | Reason | Diagnostics |
|---|---|---|---|
| **A** `volatile int g_done;` + `g_done = 1;` beside acquire/release | `unknown` | `resource projection unresolved: 1 unsupported operation(s), 0 unresolved alias(es), 0 custody summary conflict(s)` | `[subject:4:unsupported:unsupported assignment expression]` |
| **B** same body, plain `int g_done;` | `unknown` | *byte-identical to A* | *byte-identical to A* |
| **C** same body, no flag at all | `safe` | — | none |
| **D** `volatile int g_done;` + `while (!g_done) { } q_free(p);` | **`safe`** | — | **none** |
| **E** same body, plain `int g_done;` | **`safe`** | *byte-identical to D* | **none** |

Two things follow, one confirming wave 17 and one sharper than it.

**The qualifier moves nothing; the store does.** A and B are byte-identical, and
C shows the `unknown` is bought entirely by the unsupported assignment. A CT41
row resting on A would be "an Unknown attributable to something else" — the
second horn wave 9 named.

**Clause (a) fails harder than wave 17 recorded.** D and E are the scenario's
*own* shape — a consumer spinning on a completion flag — and they are answered
**`safe` with no diagnostic at all**, identically with and without the
qualifier. Wave 17 measured only producer shapes and so only ever saw the
`unknown` horn. A mapping today would have to record a **certification** against
a case whose expected outcome is a non-certification. The `safe` is correct
within the gateway's own scope (one translation unit, one acquisition, one
release, resource projection only) and is not a defect; it is simply not
evidence about a synchronization proof, which is exactly the point.

All five shapes, the contracts they were analyzed against, the measured outputs
and a reproduction recipe are archived under
`docs/reports/ct-ctwin-lane/artifacts/` — and the numbers in that record were
re-derived **from those committed files**, so the archive is the thing that was
measured rather than a transcription of a scratch run. Nothing in the committed
suite runs them: CT41 is unmapped, so it has no runner, and adding one would be
the vacuous mapping this lane declined.

**One recorded delta from wave 17, attributed.** Wave 17's producer pair carried
`1 unresolved alias(es)` and a second diagnostic `subject:4:unknown-alias:p`.
Both are gone here. That is wave 31's `AOM-BINDING-POISON` narrowing (card
`C-GATEWAY-WAVE`, constraint 10a): a refused statement no longer poisons
bindings its own text cannot have written, so `p` survives the refused
assignment. The movement is in the refusal-*reducing* direction and changes
nothing about this case's mapping question.

### 1.3 The typed-source arm, examined for the first time

CT41's recorded blocker has always carried a parenthetical no lane had checked:
*"the gateway plausibly already decides it (`typed_source.rs` refuses
volatile/_Atomic/restrict) but no harness path runs it"*.

Both halves were examined.

**"No harness path runs it" is RETIRED.** The C-twin harness path
(`run_legacy_c_twin`) has existed since the wave-17 lane, and this lane ran it.
That clause is stale and this record says so rather than repeating the reason
whole.

**The arm itself cannot carry the case, for a reason about the rule's shape.**
`crates/qcore-c-gateway/src/typed_source.rs:522` is:

```rust
if ty.contains("volatile") || ty.contains("_Atomic") || ty.contains("restrict") {
    return Err("qualified access requires additional synchronization/alias evidence".into());
}
```

Superficially this looks promising: the message even names synchronization, and
removing `volatile` *would* move the verdict, which is clause (b). But it is
**one branch with one message** covering a real synchronization primitive
(`_Atomic`), a pure aliasing qualifier (`restrict`), and the access-ordering
qualifier that is neither (`volatile`). CT41's entire content is the difference
between the first and the third. A rule that refuses all three alike cannot be
evidence about a distinction it does not draw, and mapping CT41 onto it would
claim the route *decides* the scenario when it in fact declines the whole
qualifier class.

This is recorded as a **measured boundary, not a defect**:
`CT41-TYPED-SOURCE-UNDISCRIMINATING`. Refusing the qualifier class wholesale is
fail-closed and correct for what that importer does.

*Stated as a reading of the rule plus its existing tests
(`typed_source_match.rs`'s `"Unknown: volatile/atomic source outside reference
profile"`), not as a new measurement: the branch is a literal substring test on
the type text, so the claim is settled by the source.*

### 1.4 Verdict on CT41

**Stays unmapped.** The bucket is unchanged —
`blocked-on-gateway-expressiveness (C-twin measured)` — so no supersession is
declared; a re-verification reaching the same label is the record saying the
label still holds. The *reason* is narrowed to its structural half and is
republished in this lane's own record.

---

## 2. CT46: mapped, on the `mutex_guard` route

### 2.1 Why now

Card `MUTEX-SHARE-CROSS` (wave 32) landed the capture form CT46's accept half
had always lacked — `thread scope NAME share PLACE under MUTEX { … }`, a
`TransferMode::ShareUnderMutex` requesting `ShareProtocol::CheckedMutex` for a
`TransferKind::ProtectedResource` payload — and declared the recorded blocker
`CT46-MUTEX-CAPTURE` discharged clause by clause. Its §10 left three questions
for a mapping lane. All three are answered here by measurement, and two of the
three answers are **no**:

| Question from `MUTEX-SHARE-CROSS` §10 | Answer |
|---|---|
| (a) Does a mutex generation satisfy "known shared wrapper"? | **No.** There is no wrapper type in this language at all. Published as an untranslatable word (§2.4) rather than rested on. |
| (b) May the row read `executed` for the native stages? | **No.** Measured: `c_source` is `None` and `interpret_mutex_program` refuses. Four stages read `unassessed` (§2.3). |
| (c) Does the "Send" direction have a source spelling? | **Not as a trait.** Carried by the judgment's classification, asserted against `MUTEX_SHARE_CROSSING` itself (§2.4). |

### 2.2 The case and its controls, all measured from disk

Fixture: `compiler/tests/fixtures/ct/CT46/`. Verdict mode
`checked_mutex_crossing`, new in this lane; runner
`compiler/tests/ct_corpus.rs::run_mutex_crossing`.

**`case.qcore`** — admitted. The lowered event list is asserted *whole*:

```
[0] CreateMutex { task: 0, mutex: 1, protects: [0..4], invariant: 1 }
[1] Spawn { parent: 0, child: 1 }
[2] Lock { task: 1, mutex: 1, guard: 1 }
[3] OpenInvariant { task: 1, guard: 1 }
[4] Access { task: 1, Write, guard: Some(1) }
[5] CloseInvariant { task: 1, guard: 1, witness: Declared { invariant: 1 } }
[6] Unlock { task: 1, guard: 1 }
[7] Join { parent: 0, child: 1 }
[8] Lock { task: 0, mutex: 1, guard: 2 }
[9] Access { task: 0, Read, guard: Some(2) }
...
```

The crossing is a real task boundary — a `Spawn{0,1}`, the child taking the
generation **itself**, a `Join{0,1}` — and the guarded-reach vector is
`[(1, guarded), (0, guarded)]`, rebuilt whole from the event list rather than
looked up by keys the fixture names, so a fixture cannot pin a convenient subset
of the payload accesses. Every reach names a guard: that vector *is* what
"accepted only through a verified mutex protocol" means here.

**The four refusing controls** — the `only` half — each refused by a distinct
rule, each pinning a distinguishing **message fragment** as well as its code
vector, so a control cannot pass by reaching the right code for the wrong
reason:

| Control | Code | Fragment | What it isolates |
|---|---|---|---|
| `unguarded_reach.qcore` | `Q611` | `UnguardedProtectedAccess` | The crossing is declared exactly as the case declares it, and the child then reaches the payload **unguarded**. Refused by the **unedited core** — `Q611` is `CheckedMutexInvariantV1`'s own code, not this route's. Declaring the crossing buys the right to reach the payload *under the protocol* and nothing else. |
| `absent_protocol.qcore` | `Q600` | ``a declared crossing is exactly`` | `share buf` with no `under m`. A crossing cannot be requested without naming the discipline that admits it. |
| `mismatched_protocol.qcore` | `Q600` | ``guards `other`, not `buf` `` | A protocol **is** named, and it is a real verified mutex generation — it just protects a different payload. **This is the control that keeps the mapping non-vacuous**: without it, naming any lock at all would satisfy "verified mutex protocol". |
| `share_generation.qcore` | `Q600` | `KindNotTransferable(NotAValue)` | The generation itself is shared instead of the payload, refused by the **transferability judgment**. Shows the judgment is consulted on the payload's *kind*, so the case's admission is a classification decision and not a syntactic one. |

### 2.3 The emission boundary, pinned in both directions

This is the part most easily gotten wrong, so it is measured rather than
assumed. For the case's own module:

* `compile_files` **admits** it and returns `c_source: None` — no C at all;
* `interpret_mutex_program` **refuses** it with `UnsupportedEvent { event: 1 }`
  — event 1 being the `Spawn` the crossing introduces.

That is card `MUTEX-SHARE-CROSS`'s own `MUTEX-SHARE-CROSS-EMISSION` residual,
measured here. Consequently `mutex_reference_execution`, `c_emission`,
`native_execution` and `native_sanitized` all read **`unassessed`** and none
reads `executed` — constraint 23's rule that a stage reads `executed` only where
a named workspace test runs it on the case's own artifact. They read
`unassessed` rather than `not_applicable` because the `mutex_guard` vocabulary
genuinely has them (CT26 executes all four), so they are an open item a later
card could close, not a structural absence.

**The other direction is what makes those four rows a measurement rather than an
omission.** `no_crossing.qcore` is the same route, the same `protected 4`
payload and the same generation with the crossing clause removed. It is
admitted, it **does** render C (10,714 bytes, carrying `mtx_init`, `mtx_lock`,
`thrd_create`, `thrd_join`), and it **does** interpret, to the observation the
fixture pins:

```
resource 1 4 / create 1 / lock 1 1 / read 1 0 4 0 / unlock 1 / destroy-mutex 1 / destroy 1
```

So the case's empty emission is attributable to **the crossing clause** — the
`Spawn` it introduces is outside the emitted subset — and not to the mutex
route, the protected payload, or this fixture's shape.

### 2.4 Boundaries published, not papered over

CT46's scenario is *"Known shared wrapper with non-Sync but Send payload"*. Two
of those words have no referent in this language, and the mapping says so rather
than quietly substituting.

* **"wrapper"** — there is no wrapper type at all. No `Arc<Mutex<T>>`; the
  payload `buf` and the generation `m` are two separate names bound in one
  scope. The mapping rests on the *pairing the route checks*, not on a wrapper
  value.
* **"non-Sync but Send"** — there is no `Send`/`Sync` trait. The words are
  carried by `qcore_core::transfer::TransferKind::ProtectedResource`, which that
  module documents verbatim as *"This is CT46's payload"*: sendable because
  custody really moves over a derivable synchronization edge, and not freely
  shareable because `ShareProtocol::CheckedMutex` is its **one** sharing
  discipline in both the read and the write direction. The harness asserts this
  vocabulary against `MUTEX_SHARE_CROSSING` itself, so a rename in the judgment
  cannot leave the fixture asserting a classification the code no longer has.

Also not claimed:

* **Nothing about two threads actually running.** The authority is held and
  checked, never exercised: the module emits no C and is never executed.
* **The task route is untouched.** `MUTEX_SHARING_GAP.requestable` stays
  `false`: an `own<Buffer>` still cannot request mutex mediation
  (`MUTEX-SHARE-CROSS-TASK-BUFFER`). This mapping is about the checked-mutex
  route's `protected` payload and no other kind.
* **No proof.** `MUTEX-SHARE-CROSS`'s own
  `QCore.MutexShareCross.aSharedPayloadIsReachableOnlyThroughTheProtocol` is a
  theorem about the *model*, not about this case's artifact, so citing it as
  CT46's proof stage would be precisely the overstatement constraint 23 forbids.
  `proof` stays `unassessed`; T00.7 stays open.

---

## 3. CT42: both arms measured, still unmapped

Earlier lanes measured one arm. This lane measured both.

**(i) The C-twin arm — unchanged, re-verified rather than inherited.** The
contract loader still refuses an int-typed owned result, verbatim:

```
contracts.qcore: q_open_fd at byte 18: owned result requires a pointer return type
```

`CT-CTWIN-DESCRIPTOR` stands exactly as recorded. An int-typed resource identity
is not expressible at all, so CT42's *preserved identity* has nothing to be
established by.

**(ii) The socket arm — measured for the first time, and found unrouted.** Card
`CT42-PROTOCOL-CELL` (wave 31) landed the derived cell at model level in
`qcore_core::socket_footprint`: an accepted connection becomes one cell with two
custodians, refused in the unedited disjointness checker as `SharedProtocolCell`
(`Q664`). The brief asked whether that route "maps the scenario's words
honestly". The prior question turns out to be whether a CT fixture can reach it
at all, and it cannot:

```
$ grep -rn "derive_socket_cells\|footprint_program" --include='*.rs' crates/ compiler/
crates/qcore-core/tests/unit/socket_footprint.rs: (11 call sites)
# and nothing else — not the frontend, not the checker, not compile_files
```

Both functions are called from **one unit test** and from nothing in the
pipeline; card `CT42-PROTOCOL-CELL`'s own report already states that no `.qcore`
statement spells `AssertDisjoint` over sockets. A CT fixture on a source route
is a program the harness compiles through the real pipeline, so a CT42 row on
this arm would name a stage the harness **cannot execute** — exactly what
constraint 23 forbids. Wiring the deriver into the route needs new source syntax
and a checker change: a route card, not a mapping lane.

Recorded as `CT42-CELL-UNROUTED`. The bucket is unchanged, so no supersession is
declared; the reason is republished with both arms in it.

---

## 4. CT10 and CT24

**CT10 — unmapped, on half a scenario.** The scenario has two halves and exactly
one exists. The **concurrency** half landed at *byte* granularity in wave 31
(`WRITE-LEND-TASKS`: disjoint range write leases across a real spawn,
overlapping leases refused). The **sub-byte** half does not exist on any source
route: no `.qcore` syntax declares a bit-field, so "adjacent bit-fields in one
memory location" has no source form. The model does carry the rule —
`disjoint_footprint`'s `SharedBitfieldUnit` (`Q661`), exercised by
`crates/qcore-core/tests/unit/disjoint_footprint.rs::adjacent_bitfields_of_one_storage_unit_are_never_disjoint`
— but that is a hand-constructed `LayoutContract`, not a program, so it is not a
case artifact. The C-twin arm cannot supply the missing half either: the gateway
refuses a bit-field's whole *enclosing function*
(`typed_field_access.rs::a_bitfield_refuses_the_whole_record`), which is a
refusal for inexpressibility, not the scenario's "reject independent concurrent
writes". Mapping it would rest on half a scenario. CT10 keeps card T00's
archived bucket, and the deferred residual `CT10-REASON-ROW` **stays deferred**:
this lane did not take it, and says so rather than leaving it ambiguous.

**CT24 — withdrawn, untouched, by instruction and by constraint 25.** It has
been examined and withdrawn twice (wave 26; wave 30's
`CT24-CLEANUP-PREEMPTED`). Publishing a labelled row would move
`newly_decidable_not_mapped_reasons`' key set, which constraint 25 derives from
the judgment's own keys, against a matrix this lane may not hand-edit — the trap
both prior lanes measured. CT24 keeps its `DECIDABLE-NOW` bucket and its
`t00-archive` label source.

---

## 5. Falsification: every new assertion was broken before it was believed

A green suite is evidence about the paths it took. Six mutations of the new
runner and its fixture were applied one at a time; **each made the CT corpus
test fail**, and the tree was restored between each.

| Mutation | Result |
|---|---|
| M1 — the creator's reach dropped from the pinned `reaches` vector | caught |
| M2 — `emission_refusal` claims `UnsupportedEvent { event: 7 }` | caught |
| M3 — `child_lock_task` set to a task that never locks | caught |
| M4 — `mismatched_protocol`'s fragment inverted (right code, wrong reason) | caught |
| M5 — one byte-count changed in the emitting control's observation | caught |
| M6 — `unguarded_reach.qcore` made admissible (guard restored) | caught |

Separately, the lane's own **retirement declaration** was measured to be
load-bearing rather than decorative: deleting `retired_unmapped_reasons` from
this record makes `scripts/ct_stage_summary.py --check` exit **1** and the CT
corpus test **fail** (`CT46: docs/reports/wave13-ct34/ct_stage_evidence.json
records an unmapped-case bucket, but the manifest maps this case`). Restored,
both are green.

---

## 6. Harness and record changes

* **`compiler/tests/fixtures/ct/CT46/`** — new: `case.qcore`, five controls
  (`no_crossing`, `unguarded_reach`, `absent_protocol`, `mismatched_protocol`,
  `share_generation`), and `expected.json`.
  **No shared fixture**: every file is new, and all six hashes were checked
  against every `.qcore`/`.c` file under `compiler/tests/fixtures/ct/` — none
  collides (`CT-SHARED-FIXTURE-DISCLOSURE` has nothing to disclose here). That
  sweep did surface two **pre-existing, undisclosed** duplicates, recorded in
  §6.1 rather than silently passed over.
* **`compiler/tests/ct_corpus.rs`** — `MAPPED_CASES` gains `CT46`; new
  `MUTEX_CROSSING_CASE`, `CheckedMutexCrossing` / `TaskEdge` / `GuardedReach` /
  `MutexCrossingControl`, the `checked_mutex_crossing` arms in `route_for_case`,
  `derive_stage_plan`, the fixture-validation block, the `evidence_target`
  disjunction and the dispatch; the new runner `run_mutex_crossing` and its
  `RUNNER_REGISTRY` entry; `EVIDENCE_FILES` 20 → 21.
* **`docs/reevaluation/ct_manifest.json`** — CT46's row mapped (schema 2
  unchanged, no route-vocabulary change); the note's wave-35 paragraph.
* **`scripts/ct_stage_summary.py`** — this lane's record added to
  `EVIDENCE_FILES`.
* **`scripts/tests/test_ct_stage_summary.py`** — the tree-fact pins rehomed.
  CT46 had been these tests' worked example of an *unmapped, lane-labelled*
  case; mapping it retires its wave-13 row, and a retired row is no longer
  consulted, so six pins would have passed **vacuously**. They now sit on CT42's
  live row in this lane's own record (`LANE35`), and the admitted-direction pin
  gains wave 35's retirement as its third example. No test function was added or
  removed: 229 before, 229 after.
* **`docs/reviews/r0_3/acceptance.json`** — regenerated (`--write`), plus the
  hand-kept `ct_evidence.derived_from`.
* **`docs/reviews/r0_3/acceptance_matrix.json`** — regenerated **through the
  generator**, `python3 docs/reports/remaining-d02/build_matrix.py .`, never by
  hand. Two top-level sections moved: `ct_conformance` (derived) and
  `validation` (see §7.3 — the gate *numbers* are byte-identical; only the
  provenance label changed). `named_followups` gains this lane's two residuals
  **before** the final regeneration, so `dimensions.named_followups` is derived
  in sync at 84 = 84 — the `D02-DIMENSIONS-FOLLOWUP-COUNT` hazard, avoided by
  ordering rather than by luck.
* **`docs/reports/ct-ctwin-lane/`** — this report, its `ct_stage_evidence.json`
  (carrying `unmapped_reasons` for CT41 and CT42, and `retired_unmapped_reasons`
  for CT46), and `artifacts/` — the five CT41 shapes, their contracts, the
  measured outputs and a reproduction recipe, the numbers re-derived from the
  committed files.
* **`docs/development/increments/wave-35.md`** — one paragraph prepended; the
  file already existed and was not created.

### 6.1 A finding from discharging the disclosure duty

`CT-SHARED-FIXTURE-DISCLOSURE` (wave 33) records that CT01, CT12 and CT14 are
one byte-identical fixture and asks a lane not to create another without saying
so. Checking CT46's six files against the whole corpus meant hashing every
`.qcore`/`.c` file under `compiler/tests/fixtures/ct/`, and that turned up
**three** duplicate groups, not one:

| Hash | Files | Disclosed? |
|---|---|---|
| `a842abac…` | `CT01/case.qcore`, `CT12/case.qcore`, `CT14/case.qcore` | **yes** — `CT-SHARED-FIXTURE-DISCLOSURE` |
| `1e1761bb…` | `CT02/case.qcore`, `CT14/read_before_join.qcore` | **no** |
| `63e938d3…` | `CT03/case.qcore`, `CT04/write_after_joins.qcore` | **no** |

The two undisclosed ones are a **different relation** from the disclosed one:
not three cases sharing one artifact, but one case's own artifact being
byte-identical to another case's *control*. Each is individually defensible —
a control that is another mapped case's admitted program is a strong control,
and both reports describe their control in their own terms — so this is recorded
as a **disclosure gap, not a defect**. The hazard it creates is concrete and
one-directional: editing `CT02/case.qcore` or `CT03/case.qcore` silently changes
what CT14's and CT04's controls test, with nothing in either fixture, report or
harness saying so.

Not repaired here: the repair touches other cases' fixtures and records, which
is outside a mapping lane's boundary in both directions. Named
`CT-FIXTURE-CROSS-CASE-DUPLICATES` (§8).

**Not touched:** `crates/qcore-c-gateway/` (two arbiters own it this wave — no
mapping here needed it, and the one that would have, CT42's, is named as a
residual instead), `AGENTS.md`, `CLAUDE.md`, `README.md`, every frozen archive
(`docs/reports/wave13-ct34/ct_stage_evidence.json` keeps CT46's row **byte for
byte**; the retirement is a declaration in this lane's own record, constraint
21), and every non-CT matrix section.

---

## 7. Gate

Run in this worktree on `main` at `a300a39d` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

**Measurement condition, stated up front because it bears on constraint 16.**
This wave's sibling cards were running their own full workspace suites on the
same machine throughout: nine concurrent `cargo test --workspace` processes at a
one-minute load average of **21.8** while this row was being taken. That is
exactly the condition constraint 16's five load- and parallelism-sensitive rows
are recorded as failing under. Any such row that tripped is re-verified **in
isolation** and **both** facts are recorded below — the wave-32 close's
practice — rather than the isolated run being reported alone.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0, before anything expensive |
| `cargo test --workspace --no-fail-fast` | **4,558 passed / 1 failed / 8 ignored** across 228 result lines, exit **101**. The one failure is the known host row, §7.1 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 (one finding in this card's own code, fixed rather than allowed — §7.2) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | `Passed 129 Lean audits`, exit 0 — unchanged; this lane adds no Lean |
| `python3 -m unittest discover -s scripts/tests` | **229 tests, OK** — the wave-34 count, unchanged; this lane adds and removes no test function and rehomes pins inside existing ones (§6) |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT, `mapped 46 / unmapped 4` |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |

The workspace row is reported with its **honest nonzero exit**. Every other row
is green, and the failure is in a crate this card does not touch.

### 7.1 The one failure: the known host row, and a correction to its description

The failing test is
`qcore-verifier-host`'s
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
at `crates/qcore-verifier-host/tests/standalone_host.rs:212`:

```
assertion `left == right` failed
  left: Err(Timeout)
 right: Ok(VerificationResponse { … })
```

**It is not this card's.** Nothing in this change set is in
`qcore-verifier-host`, or in any crate it depends on: the lane touches CT
fixtures, the CT harness, the manifest, two derived records, two scripts and
documentation. The card was briefed that this row is known and not to be
touched, and it was not touched.

**What was measured, both facts, per the wave-32 close's practice:**

| Run | Load average (1 min) | Result |
|---|---|---|
| Under the full `--workspace --no-fail-fast` suite | ~22 | **FAILED**, `Err(Timeout)` |
| Isolated, `-p qcore-verifier-host --test standalone_host` | 9.5 | **FAILED**, `Err(Timeout)` (2 passed / 1 failed) |
| Isolated, repetition 1 | 6.4 | **FAILED** (2 passed / 1 failed, 3.42 s) |
| Isolated, repetition 2 | 6.2 | **FAILED** (2 passed / 1 failed, 3.07 s) |
| Isolated, repetition 3 | 6.2 | **FAILED** (2 passed / 1 failed, 3.20 s) |

**A quiet-machine measurement was not obtainable and is not claimed.** A run was
queued to fire when the one-minute load average fell below 4; it never fired
inside this card's window, because sibling wave-35 cards held the machine at 6
or above throughout. So this lane reports **red under the full suite and red 4/4
in isolation at loads 6.2–9.5**, and explicitly does *not* report what the row
does on an unloaded host — which is precisely the measurement
`HOST-ROW-STARVATION-DESCRIPTION` asks for.

**A correction to the brief's description of this row, offered because it
matters for the next card that meets it.** The brief characterised it as *"red
in isolation, green under a full suite"*. That is not what this lane saw — it
was red **both** ways — and the row's own repair record contradicts the
description too: card `VERIFIER-HOST-CLOSED-INPUT-DEADLINE` (2026-09-14,
`docs/reports/verifier-host-closed-input-deadline/README.md`) reports the test
passing **5/5 isolated repetitions** after its repair, and the integrator
measured the whole workspace at 4,544 / 0 / 8 on the merge candidate. So
"red in isolation" appears to describe the **pre-repair** state and to have
outlived it.

**The likelier explanation, from that same record.** The third arm gives an
**8 MiB** response a **1,000 ms** wall-clock budget, and the repair report's own
scope sentence reads: *"Scheduling under arbitrary host starvation remains
outside this observation."* This lane's gate ran with up to **nine concurrent
sibling `cargo test --workspace` processes** at a one-minute load average of
**22–26** — host starvation by any reading. The failure therefore falls inside
the exclusion the repairing card already declared, rather than being new
evidence against its repair. That is an observation about the measurement
condition, **not** a claim that the row is sound: this lane did not re-derive
the transport argument and does not vouch for it.

Recorded as `HOST-ROW-STARVATION-DESCRIPTION` (§8) so the stale description is
fixed once rather than re-encountered by each wave's cards.

### 7.2 The clippy finding, fixed rather than allowed

Clippy's `assertions_on_constants` fired on this card's own new code:
`MUTEX_SHARE_CROSSING.requestable` is a `const`, so asserting it at run time is
a constant assertion. It was moved into a `const` block rather than silenced:

```rust
const { assert!(MUTEX_SHARE_CROSSING.requestable) };
```

This is strictly stronger than the runtime assertion it replaces. If a later
card sets `requestable: false` for this kind — as `MUTEX_SHARING_GAP` still is
for the task route's own buffer — the CT harness **stops compiling**, instead of
a green test reporting CT46 mapped onto a capability the judgment no longer
offers. No `#[allow]` was added anywhere in this change set.

### 7.3 The `validation` block's provenance label

The matrix regeneration was run **without** `--gate`. The gate *numbers* are
therefore carried from the previous matrix byte-identically, and the one field
that changed is `validation.measurement_provenance`:

```
before: integrator_supplied (docs/reports/wave33-close/gate.json)
after : carried_from_previous_matrix: these numbers were NOT measured for this
        regeneration; pass --gate PATH with the standing-gate results of this
        checkpoint to replace them
```

This is the generator's documented behaviour and the honest option for a lane
that is not the wave close: this card's gate numbers (§7) are *this card's*, not
a checkpoint's, and passing them as `--gate` would label a card measurement as
the wave's. **The integrator restores an integrator-supplied provenance at the
close** — see §9.

---

## 8. Named follow-ons

| Id | What |
|---|---|
| `CT41-TYPED-SOURCE-UNDISCRIMINATING` | **New.** The typed-source arm named in CT41's archived blocker refuses `volatile`, `_Atomic` and `restrict` in one branch with one message, so it cannot express the very distinction CT41 is about. A measured boundary, **not** a defect: refusing the qualifier class wholesale is fail-closed and correct for that importer. Discharge: a statement that this arm is permanently out of scope for CT41, or a rule that distinguishes the qualifiers — which needs the synchronization model the gateway does not have. |
| `CT42-CELL-UNROUTED` | **New.** CT42's socket arm exists at model level and is reachable from no source program, so no CT fixture can execute it. Discharge: wire the deriver into the socket route behind syntax that asks the disjointness question (a route card), or record that the derived cell is model-only by design. |
| `CT-CTWIN-SYNC` | **Decided, not open** (card `CTWIN-PUBLISH` §5), re-measured here over five shapes including the scenario's own consumer shape. CT41 stays unmapped; the obstruction is structural and no measurement can retire it. |
| `CT-CTWIN-DESCRIPTOR` | Unchanged and re-verified verbatim on this tree. |
| `CT42-MAP` | Still open. This lane measured **why** it is open and did not take it: the model half landed, the routing did not. |
| `MUTEX-SHARE-CROSS-EMISSION` | Unchanged and now **measured on a CT artifact**: `c_source: None`, `interpret_mutex_program` refusing `UnsupportedEvent { event: 1 }`. Discharge would move CT46's four `unassessed` stages, which is a coupled edit (§9). |
| `MUTEX-SHARE-CROSS-TASK-BUFFER` | Unchanged. `MUTEX_SHARING_GAP.requestable` stays `false`. |
| `HOST-ROW-STARVATION-DESCRIPTION` | **New** (§7.1). The standing description of `qcore-verifier-host`'s `closed_input_does_not_mask_…` row — *"red in isolation, green under a full suite"* — does not match this tree: the row was red **both** ways here, and the card that repaired it (`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`, 2026-09-14) records it passing 5/5 isolated repetitions afterwards, so the description appears to have outlived the **pre-repair** state it described. What this lane measured is consistent with that card's own scope sentence — *"scheduling under arbitrary host starvation remains outside this observation"* — under nine concurrent sibling workspace runs at load 22–26. Discharge: re-measure the row on a quiet machine and replace the standing description with what holds (this lane's low-load attempt is recorded in §7.1's table), or give the 1,000 ms arm a load-independent budget. **Not** a claim that the repair is unsound: this lane did not re-derive the transport argument. |
| `CT-FIXTURE-CROSS-CASE-DUPLICATES` | **New**, found by discharging this lane's own disclosure duty (§6.1). Two byte-identical pairs beyond the disclosed CT01/CT12/CT14 one: `CT02/case.qcore` ≡ `CT14/read_before_join.qcore` and `CT03/case.qcore` ≡ `CT04/write_after_joins.qcore`, each a case's own artifact doubling as another case's control, neither disclosed anywhere. A **disclosure gap, not a defect** — such a control is arguably a strong one — but editing either case's artifact silently changes another case's control. Discharge: disclose both pairs in the owning reports (and, if the coupling is unintended, give one side its own file), or record that a case artifact reused as a control is deliberate practice. |
| `CT10-REASON-ROW` | **Deferred and untaken**, stated rather than left ambiguous. CT10 stays unmapped; §4 is this lane's measurement of why. |
| `CT24-CLEANUP-TASK-COMPOSITION` | Unchanged, untaken. |
| `T00.7` | Unchanged and open: no theorem is bound to CT46's artifact. Proof stays 3. |

---

## 9. For the integrator

These are the exact words to land.

**1. `AGENTS.md` ledger row — CT01–CT50 conformance.** Replace the mapped count
and both denominators:

> **46/50 mapped**, 4 unmapped; sanitized-native **9 of 46**; **proof 3 of 46**.

Then append to that row, after the wave-32 sentence:

> Wave 35 (`CT-CTWIN-LANE`) mapped **CT46** on card `MUTEX-SHARE-CROSS`'s
> composition, under a new `checked_mutex_crossing` verdict mode on the
> `mutex_guard` route — the corpus's first **admitted** case that deliberately
> stops at the checker: the module is accepted and the C backend renders nothing
> for it, `interpret_mutex_program` refusing the `Spawn` the crossing
> introduces, which is `MUTEX-SHARE-CROSS-EMISSION` measured rather than
> inherited and is why four stages read `unassessed` and none reads `executed`.
> The emission boundary is pinned in both directions, one control being the same
> route without the clause, which does render and does interpret. Two of CT46's
> own words are published as untranslatable — there is no wrapper type and no
> `Send`/`Sync` trait — both carried instead by the transferability judgment's
> `TransferKind::ProtectedResource`, asserted against `MUTEX_SHARE_CROSSING`
> itself. The same lane **re-measured the four remaining cases and mapped
> none**: CT41's obstruction is **structural** — clause (b) of the wave-9
> attribution rule needs a control in which removing `volatile` moves the
> verdict, and `CTWIN-PUBLISH` §5's decision is that it moves nothing — with the
> scenario's own consumer shape now measured **`safe` with no diagnostic**, and
> the typed-source arm examined for the first time and found unable to tell
> `volatile` from `_Atomic` (`CT41-TYPED-SOURCE-UNDISCRIMINATING`); CT42 stays
> unmapped with **both** arms measured, the socket arm being **unreachable from
> source** (`CT42-CELL-UNROUTED`); CT10 stays unmapped on half a scenario; CT24
> stays withdrawn.

**2. `AGENTS.md` merged-gate row.** Update from this card's §7 numbers at the
integration tip, per the usual rule that the wave's own close re-measures.

**3. Regenerate the matrix at the close *with* `--gate`.** This lane regenerated
without it deliberately (§7.3), so `validation.measurement_provenance` currently
reads `carried_from_previous_matrix`. At the close, run

```sh
python3 docs/reports/remaining-d02/build_matrix.py . \
    --gate docs/reports/wave35-close/gate.json
```

which restores an `integrator_supplied (…)` provenance. **Nothing else in
`validation` needs touching** — the numbers are already byte-identical to
wave 33's.

**4. `named_followups` is already updated.** This lane appended
`CT41-TYPED-SOURCE-UNDISCRIMINATING`, `CT42-CELL-UNROUTED`,
`CT-FIXTURE-CROSS-CASE-DUPLICATES` and `HOST-ROW-STARVATION-DESCRIPTION`
**before** its final regeneration, so `dimensions.named_followups` is derived in
sync (84 = 84). Do not re-append them.

**6. `CT-FIXTURE-CROSS-CASE-DUPLICATES` needs an owner outside this lane.** The
two undisclosed pairs (§6.1) live in CT02/CT14 and CT03/CT04 — other lanes'
fixtures and records — so the disclosure belongs with whoever next touches those
cases, or with a hygiene card. It is registered and measured; it is not fixed.

**7. The standing description of the `qcore-verifier-host` row should be
replaced, not re-issued.** Wave 35's cards were briefed that
`closed_input_does_not_mask_…` is *"red in isolation, green under a full
suite"*. This lane saw it red **both** ways, and the card that repaired it
records the opposite of the isolation half (§7.1). Whatever the next
measurement on a quiet machine shows, the brief text each wave hands its cards
should be updated to match it, so cards stop being told something the tree does
not do. `HOST-ROW-STARVATION-DESCRIPTION` carries the discharge condition.

**5. The coupled edit `MUTEX-SHARE-CROSS-EMISSION` would need**, stated here so
a later card does not have to rediscover it. If the backend learns to render a
declared crossing, **three** things move in **one** change or the harness
refuses: (i) `derive_stage_plan`'s `checked_mutex_crossing` block in
`compiler/tests/ct_corpus.rs` gains an executed arm crediting a runner with
`mutex_reference_execution` / `c_emission` / `native_execution`; (ii) CT46's four
manifest stage rows are rewritten to match; (iii) `run_mutex_crossing`'s
`module.c_source.is_none()` assertion — which carries that instruction in its own
failure message — is replaced by the positive one. CT46 would then also join
`SANITIZED_GATED_CASES` if a sanitized run lands, moving the sanitized-native
**numerator** off 9 for the first time since wave 32.

**6. `README.md`** — not touched by this lane; update if the CT headline appears
there.

---

## 10. What this lane does not claim

* **Nothing is promoted.** No corpus unit or function status moves; no leak pin
  moves; no contract module is added; no version changes.
* **A grown denominator is mapping, not conformance.** 45/50 → 46/50 is one more
  case with a measured artifact, not one more guarantee. Full language
  conformance stays **false**.
* **The CT46 mapping is checked-only.** It is not evidence about emission,
  execution, sanitizers, scheduling, contention or the C memory model.
* **The four re-measurements are measurements, not permissions.** CT41's
  structural obstruction in particular is a finding about what the project has
  *decided*, and a later card that wanted CT41 mapped would have to reopen
  `CTWIN-PUBLISH` §5's decision rather than route around this lane.
