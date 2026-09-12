# Wave-26 CT lane — what the current tree can actually wire

Wave 26, card `CT-WIRE-LANE`. The user's question was *"we need to attempt the
CT wires. Anything else we can wire with the current implemented tools?"* This
report answers it with a row for every one of the seventeen unmapped CT cases,
and moves the three the answer is **yes** for.

CT coverage moves **33/50 → 36/50 mapped**, unmapped **17 → 14**.
Harness-gated sanitized-native evidence moves **4 of 33 → 5 of 36**. Proof
stays **0 of 36**.

Base: `main` at `e26bb0c3` (the wave-25 close). Branch:
`claude/wave-23-hatch-opening-f8b35d` in an isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is test and
fixture wiring in `compiler/tests/ct_corpus.rs`. No checker rule, no diagnostic
code (the Q-space icebox needed none — this card allocates zero), no lowering,
no model. A grown denominator is mapping, not conformance.

---

## 1. The answer, in one table

Seventeen cases, seventeen rows. "Reason" is the label and text the derivation
currently resolves for that case — the newest CT lane record that measured it,
else card T00's frozen 2026-09-07 archive
(`scripts/ct_stage_summary.py::unmapped_labels`).

| Case | Recorded reason (abridged) | What has landed since | Verdict |
|---|---|---|---|
| **CT06** | *"expected result is not an accept/reject verdict, and aliasing is refused outright"* | `STRING-CONCAT`'s `borrow_utf8` bridge plus `STRING-SLICE`'s slices: an owned buffer's bytes get an immutable view, and a whole-extent slice of that view is the same pointer and length under a second name | **RETIRED — mapped** (§2.1) |
| **CT07** | *"no array types, indexing or disjointness in source"* | `STD-BUFFER` (`bytes(N)`, `b[i]`, `extent`) and `LOOP-V1` (`for i in 0..N`). Bounds are decided at check time: `Q053` on an out-of-extent index, measured | **PARTIALLY RETIRED** — bounds yes, disjointness no (§3.1) |
| **CT08** | *"as CT07; also wants an `unknown` verdict the harness lacks"* | Same surface, same `Q053` | **PARTIALLY RETIRED** — same half; `Unknown` is still the gateway's vocabulary, not the source harness's (§3.1) |
| **CT09** | *"no struct/field access in source"* | Still nothing: the source language has no aggregate type. The gateway models members (`AOM`, wave 20) but derives no *access proof* | **STANDS** (§3.2) |
| **CT10** | *"no bit-fields; no C-twin stage"* | The C-twin stage now exists (`legacy_c` route, CT40/CT43/CT45). Bit-fields and "independent concurrent writes" do not: the resource projection derives no concurrency fact at all | **PARTIALLY RETIRED** — the stage, not the construct (§3.2) |
| **CT11** | *"ScopedReadSharesV1 has no local alias form at all, and the party retaining the share is an unjoined child task"* (wave-3 CT lane) | `STRING-CONCAT`'s `borrow_utf8`: a shared immutable alias of an owned buffer's own bytes, bound in the same frame, with no task and no second party | **RETIRED — mapped** (§2.2) |
| **CT21** | *"no source path emits `RegionOp::QuantOpen`"* | Nothing. Re-verified by inspection: outside `qcore-core`, the only mention of `RegionOp::QuantOpen` in the workspace is a no-op traversal arm in `crates/qcore-frontend/src/whole_program.rs` | **STANDS** (§3.3) |
| **CT22** | *"as CT21; also needs credits"* | Credits landed (see CT23), the quant opening did not | **STANDS** (§3.3) |
| **CT23** | *"quantitative credits are not in any executable subset"* | `source.epoch.bound.v1` (`T08-EPOCH-SRC`) spells `credit N effort M`, checked by the unedited `FlatEpochBoundV1` — and CT38, *"child credit splits total more than original credit"*, is **mapped on it** | **PARTIALLY RETIRED** — credits yes, an error path over them no (§3.4) |
| **CT24** | *"DECIDABLE-NOW (with a distinguishing fixture)"* | Re-measured here. The distinguishing fixture is measurably unwritable: the cleanup path is refused first, on the task obligation | **STANDS**, with a measured reason (§3.5) |
| **CT25** | *"joins now exist; loan expiry does not"* | Loan expiry now has a source form — the `borrow_utf8` window, whose expiry CT11's own boundary control demonstrates as an admitted destroy-after-expiry — but only on the sequential route, which has no joins and no cleanup | **PARTIALLY RETIRED** — expiry yes, with joins no (§3.6) |
| **CT41** | *"the `volatile` invariance is asserted as correct by `CTWIN-PUBLISH` §5 … a mapping would be vacuous"* (wave-17 CT lane) | Nothing bearing on it; this is a decision, not a gap | **STANDS** — carried, not re-measured (§3.7) |
| **CT42** | *"an int-typed resource identity is not expressible at all"* (wave-17 CT lane) | Nothing bearing on it | **STANDS** — carried, not re-measured (§3.7) |
| **CT44** | *"the available rejection is about the handle, not about a capture whose environment expires"* | Nothing: `thread scope` is lexical, so a capture's environment cannot outlive the scope that could observe it | **STANDS** (§3.7) |
| **CT46** | *"`CT46-MUTEX-CAPTURE`: the mutex ledger and the task ledger compose by mutual exclusion rather than over a capture"* (wave-13 CT lane) | Nothing bearing on it | **STANDS** — carried, not re-measured (§3.7) |
| **CT47** | *"DECIDABLE-NOW (coarser reason): a real cross-thread boundary now exists and refuses a raw pointer"* | Re-measured here. The boundary refuses, but by grammar (`Q000`) and by the **backend** (`Q500`) — not by a transfer/share analysis, and the language has no raw pointer type | **STANDS**, with a measured reason (§3.8) |
| **CT49** | *"the harness pins an exact total transcript (`stdout == render_task_observation`), which is the opposite of a partial-order check"* | `OBS-REORDERING` (wave 22): a source line in a worker body declares `qcore.obs-reordering.v1`, and streams are compared window by window and emitter by emitter, with byte equality kept as that relation's degenerate case | **RETIRED — mapped** (§2.3) |

**Headline: 3 of 17 reasons retired and mapped, 5 partially retired and still
unmapped, 9 stand.** Of the nine, three (CT41, CT42, CT46) are carried from
lanes that measured them, two (CT24, CT47) are re-measured here and get a better
reason than the archive gave them, and four (CT09, CT21, CT22, CT44) were
re-read and found unchanged.

---

## 2. The three cases mapped

### 2.1 CT06 — "same pointer copied into differently named variables"

Expectation: **"Still same permission identity"**, which is a modelling
property rather than a verdict. The wave-9 attribution rule applies anyway: the
verdict must be the class the scenario demands *and* attributable to the
construct it names.

`compiler/tests/fixtures/ct/CT06/case.qcore` binds an owned `std.buffer`, views
it with `borrow_utf8`, reads through the view once, and then copies the view:

```
let view = borrow_utf8(store);
print view;
let same = view[0:2];      // {ptr = p + 0, len = 2 - 0}: the same pointer
drop store;                // refused
print same;
```

The release is refused, and the refusal **names the copy**:

> `Q078`: this statement would release `store` while **`same`** borrows it; a
> `borrow_utf8` view is immutable UTF-8 for its whole lifetime, so the buffer it
> addresses may not be written or released until the last statement that reads
> the view or anything derived from it (statement 7 of this entry)

The obligation followed the copy — that is the case's expectation, stated as a
measurement instead of as a claim. Two controls bound it:

| Control | Role | Measured |
|---|---|---|
| `no_copy.qcore` | attribution | Removing **exactly** the copy and its read, leaving the release where the case refuses it: **admitted** (`StandardStringV1`, emits C) |
| `copy_read_first.qcore` | boundary | Keeping the copy and moving only the release past the copy's own last read: **admitted** |

So the second name carries the original's obligation for exactly as long as it
is read, and not a moment longer — not a new obligation created by naming, and
not a blanket refusal of aliasing.

### 2.2 CT11 — "full permission reconstructed while local shared aliases survive"

Expectation: **Reject**. The wave-3 CT lane measured the right refusal
direction on `ScopedReadSharesV1` and refused to map it, because the party
retaining the share there is an **unjoined child task**, not a local alias. It
recorded exactly what was missing: *"ScopedReadSharesV1 has no local alias form
at all."*

`borrow_utf8` is that form. `compiler/tests/fixtures/ct/CT11/case.qcore`:

```
let view = borrow_utf8(store);
store[0] = 106;            // refused: reconstructing exclusive write authority
print view;
drop store;
```

> `Q078`: this statement would write a byte of `store` while `view` borrows it …

A byte write into a `std.buffer` is the reconstruction of the buffer's
exclusive write authority; the alias is local, shared and immutable; the write
is refused while it survives.

| Control | Role | Measured |
|---|---|---|
| `write_before_alias.qcore` | attribution | the identical write, before the alias exists: **admitted** |
| `write_after_alias.qcore` | boundary | the alias exists and the write follows its last read: **admitted** |

The second control is what makes the mapping a statement about *survival*:
full permission **is** reconstructible once no alias survives.

CT06 and CT11 are the first mapped cases on the **`sequential` route**, which
schema 2 declared and left empty. They share a diagnostic code and a
capability, and they measure different things: CT11 that the alias blocks
reconstruction, CT06 that the alias's identity survives renaming.

### 2.3 CT49 — "independent event order differs between C and interpreter"

Expectation: **"Validate partial order, not exact total trace."** Card T00's
reason for leaving it unmapped was that the harness does the opposite — it pins
`stdout == render_task_observation`. Card `OBS-REORDERING` (wave 22) built the
partial-order check, and this is its first CT use.

`compiler/tests/fixtures/ct/CT49/case.qcore` is a composed task/std-io module in
which each of two workers writes three lines **from its own body**, so the
emitted C performs those writes on those workers' own threads while the
reference execution runs each worker eagerly at its spawn. The three emitters
write disjoint bytes inside the concurrent window (11, 22 and 9), because the
relation identifies a line's writer by its bytes.

What `run_reordering_partial_order` measures, all of it on the case's own
artifact:

| Pinned | Measured |
|---|---|
| declared `stdout` | `QCORE_TASK_SPAWN 1 0`, `QCORE_TASK_SPAWN 2 0`, `QCORE_TASK_RESULT 2 I32 22`, `QCORE_TASK_JOIN 2`, `QCORE_TASK_RESULT 1 I32 11`, `QCORE_TASK_JOIN 1`, `22`, `QCORE_TASK_RESULT 0 I32 11` |
| declared `stderr` | `[INFO] 0`, `[INFO] 11`, `[WARN] 11`, `[FATAL] 11`, `[INFO] 22`, `[WARN] 22`, `[FATAL] 22`, `[INFO] 9`, `[WARN] 9`, `[INFO] 11` |
| `stderr` window structure | `[1, 8, 1]` — a barrier, the concurrent window of eight, a barrier |
| `stderr` emitters | 3 |
| relation strictness | `stdout` **strict**, `stderr` **not** |
| the runs | 16 sanitized executions of one binary built from the case's own emitted C; `stdout` byte-compared each time, `stderr` compared up to the relation each time |

Plus two deterministic witnesses, so nothing rests on what a scheduler
happened to do: an emitter-preserving regrouping of the declared stream is
**admitted and is not the declared order**, and a reversal inside one emitter's
own lines is **refused**.

| Control | Role | Measured |
|---|---|---|
| `parent_only.qcore` | boundary, pinned for **keeping byte equality** | Same spawn/join structure with every line in the parent's body: the derived relation is **strict on both streams** |
| `ambiguous_alphabet.qcore` | refusal | Both workers given the same value: the declaration is refused, `AmbiguousAlphabet` (code 2), the message naming `[INFO] 11` |

The first is what keeps this mapping from reading as a corpus-wide weakening:
the looser comparison is what **one declaration** buys. The second is what
keeps "compare up to a partial order" from becoming a licence to stop
comparing.

**Measured, and reported rather than asserted:** in the run recorded for this
card, **16 of 16** executions produced a `stderr` that was *not* the declared
order, and 1 distinct order was seen across the 16. A strict byte comparison of
this fixture would have failed on the first run — which is the "differs between
C and interpreter" half of the scenario, observed. No assertion depends on that
count, because observing one order is a legal outcome of a legal scheduler and
asserting on it would be a flake.

---

## 3. The fourteen that stay unmapped, and why

### 3.1 CT07 and CT08 — bounds are decided, disjointness is not expressible

Measured on this tree:

| Probe | Measured |
|---|---|
| `store[4] = 2;` on `bytes(4)` | `Q053`: *index 4 is outside `store`, whose declared extent is 4 bytes; `source.buffer.v1` decides every access at check time, so this is a refusal and not a run-time trap* |
| `for j in 4..8 { … }` | `Q097`: the counted loop counts from `0`, written out; a range with another low end is refused rather than silently re-based |

So half of the archived reason is retired: arrays, indexing and a counted loop
all exist, and every access is decided before anything runs. The other half is
not. *Independently* in CT07 means two parties holding disjoint permission over
two ranges, and **no source form lends a sub-extent of a buffer to anyone** —
that is unit F1 ("source slice surface") of
`docs/design/DISJOINT_FOOTPRINT_PROFILE.md`, whose own §8 names CT07–CT10 as
blocked on it. A sequential mapping would accept two halves written one after
the other by one party, which answers a different question, so it is not taken.

Both rows move bucket in this lane's record, to
`blocked-on-slice-permission-surface (bounds decided, disjointness not)`.
Neither declares a supersession: both labels were held by card T00's archive
rather than by a lane record, and a lane label beats the archive by the
resolution rule.

### 3.2 CT09 and CT10 — re-read, not re-measured

CT09 wants distinct struct fields accepted *after a layout/access proof*. The
source language still has no aggregate type at all. The gateway does model
members — wave 20's address-of-member work — and carries an ABI projection
under the recorded `LOADER-ABI-LP64` premise, but a premise is not a proof
(invariant 28), and the gateway's verdict vocabulary answers custody, not
access. The reason stands; the row is **not** re-published, because this lane
re-read the surface rather than running a gateway measurement.

CT10 gains half its reason back and keeps the other half. A C-twin stage now
exists — the `legacy_c` route, which did not when CT10's reason was written —
so *"no C-twin stage"* is retired. But *"reject independent concurrent writes"*
needs the projection to derive a concurrency fact, and card `CTWIN-PUBLISH` §5
asserts it derives none; that is the same standing decision that keeps CT41
unmapped. Bit-fields remain unmodelled.

### 3.3 CT21 and CT22 — the quant opening has no source path

Re-verified by inspection rather than by a probe: `RegionOp::QuantOpen` is
defined and interpreted in `crates/qcore-core/src/error_region.rs`, and the
workspace's only other mention is a no-op arm in a traversal
(`crates/qcore-frontend/src/whole_program.rs`). No source lowering constructs
one, so no fixture can ask the question either case asks. Unchanged and not
re-published.

### 3.4 CT23 — credits exist now; an error path over them does not

This is the row whose archived reason is most out of date. *"Quantitative
credits are not in any executable subset"* was true when it was written and is
not true now: `source.epoch.bound.v1` spells `credit N effort M`, checks it
against the unedited `FlatEpochBoundV1`, and **CT38 is already mapped on
exactly that construct**.

What CT23 adds is *"the parent abandons child-owned credit during its own
error"*, and the epoch route is a declarative accounting block: no `?`, no
typed error, no error region, and published check-only with `c_source: None`.
The task route has all of those and no credits. The composition
(`ComposedEpochTasksV1`) puts a checked task region under a flat epoch, but the
epoch half stays check-only on top of it, so an error in the parent moves no
credit. Follow-on `CT23-CREDIT-ERROR-PATH`; the row is not re-published,
because this is a re-reading of two route definitions rather than a
measurement.

### 3.5 CT24 — the distinguishing fixture is unwritable, measured

Card T00 called CT24 `DECIDABLE-NOW (with a distinguishing fixture)`, the
distinction being that scope-exit cleanup is a different mechanism from an
explicit `drop`. Two shapes, measured on this tree:

| Shape | Measured |
|---|---|
| explicit `drop buffer;` before the join, with a child holding `read buffer share 512` | `Q600`: `LoanOutstanding { resource: 1, outstanding: 512, borrowers: [2] }` |
| a `?` inside the thread scope with the child unjoined — the shape whose scope exit would run cleanup over the owner | `Q600`: `ScopeBusy(5)`, **at the `?` itself** |

The cleanup path never reaches the owner: the task obligation refuses the scope
exit first. So no program can be written in which cleanup destroys an owner a
child still borrows, and mapping the explicit-drop shape instead would record a
refusal attributable to `drop` — not the construct CT24 names. The
classification is unchanged: re-verification reaching the same bucket is the
record saying the label holds. The measurement is published in this lane's
record under `cases_examined_and_left_unmapped` and in `acceptance.json`'s own
hand-kept reasons, and deliberately **not** as a labelled `unmapped_reasons`
row — see §8.2. Follow-on `CT24-CLEANUP-PREEMPTED`.

### 3.6 CT25 — loan expiry exists, but not where the joins are

*"Joins now exist; loan expiry does not"* is half retired. `borrow_utf8`'s
window **is** a lexical loan with an expiry, and CT11's own boundary control
(`write_after_alias.qcore`) is an admitted *reconstruct-after-expiry*. But it
lives on the sequential route, which has neither joins nor a cleanup mechanism,
and the task route has joins and no local loan of this kind. CT25 needs the two
in one program — the same missing composition CT24 needs. Follow-on
`CT25-EXPIRY-WITH-JOINS`.

### 3.7 CT41, CT42, CT44, CT46 — unchanged, and carried

CT41 (a decision, not a gap: a projection deriving no synchronization fact must
answer identically with and without `volatile`), CT42 (an int-typed resource
identity is not expressible; the contract loader refuses an owned non-pointer
result) and CT46 (`CT46-MUTEX-CAPTURE`: no source text can ask for
mutex-mediated sharing across a task boundary) keep the rows the wave-17 and
wave-13 lanes measured. CT44's available rejection is still about the handle
rather than about a capture whose environment expires, and `thread scope` being
lexical is what keeps it that way. None of the four is re-published by this
lane: nothing landed that bears on them, and re-reading is not measuring.

### 3.8 CT47 — the refusals are real and the attribution is not

Three shapes, measured:

| Shape | Measured |
|---|---|
| `spawn s reader(buffer)` — no declared mode | `Q000`: *capture requires copy, move, read … share, or write* |
| `spawn s reader(copy buffer)` — a bit-copy of an owned handle | `Q500`: *only copy i32, move own<Buffer> and read own<Buffer> share FULL captures are executable* |
| `spawn s reader(read buffer share 512)` | admitted, emitted |

There **is** no automatic cross-thread acceptance: the grammar has no modeless
capture. But that is a syntax gate, and the refusal that would actually carry a
mapping is `Q500`, the C-emission refusal — *"the reduced C11 CFG cannot encode
this"*, which is precisely the "the profile cannot express this at all" vacuity
the wave-9 attribution rule excludes. The case's premise also names a *raw
pointer*, and this language has no pointer type for a program to copy. Mapping
it today would rest on a syntax gate plus an emission refusal, so it is not
taken; the measurement is published — in this lane's record under
`cases_examined_and_left_unmapped`, in `acceptance.json`'s hand-kept reasons and
here, but not as a labelled row (§8.2) — so a future lane can decide it
deliberately. Follow-on `CT47-COPY-CAPTURE`.

---

## 4. Sanitized-native growth: 4 of 33 → 5 of 36

| Case | Runner | What is compiled and run |
|---|---|---|
| CT03 | `run_native_heap_scenarios` | (unchanged) |
| CT17 | `run_native_heap_scenarios` | (unchanged) |
| CT26 | `run_checked_mutex_sanitized` | (unchanged) |
| CT34 | `run_mutex_controls` | (unchanged) |
| **CT49** | **`run_reordering_partial_order`** | **the case's own emitted C, built once with `-std=c11 -pedantic -pthread -Wall -Wextra -Werror -g -O0 -fsanitize=address,undefined -fno-omit-frame-pointer` and run 16 times under `ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`, `UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`** |

One case is added, and only one, because the sanitizer gate means what it says:
a case reads `native_sanitized: executed` only where a named workspace test
really compiles its C with `-fsanitize=…` and runs it.

**CT06 and CT11 add nothing here, and were checked rather than assumed.** Their
own fixtures are refusals that emit nothing, so there is no artifact of theirs
to sanitize; the sequential route has a `c_emission` stage, so the honest status
is `unassessed` rather than `not_applicable`; and their accepting controls
compile through the ordinary pipeline for a verdict, which is not an emission or
execution observation under the `CT34-SANITIZED` convention. A control *could*
carry a sanitized run on this route in a later card — that is a real
opportunity, named `CT-SEQ-SANITIZED` below — but it would be the control's
artifact, not the case's, and this lane did not do the work.

The macOS arm runs `detect_leaks=0`: Apple's AddressSanitizer ships no leak
detector, so leak detection stays a Linux observation under the standing
platform boundary. The Windows arm asserts the emission exists and returns
before compiling; it makes **no sanitizer claim**, exactly as the two existing
sanitized runners do.

## 5. The proof stage: still 0, and the near-misses named

`proof` reads `unassessed` for all 36 mapped cases. The bar is the case's
**own artifact** covered by a registered audit theorem, and nothing in the 107
audited Lean modules meets it. Three near-misses, one sentence each:

* **CT49 / `QCore.ObsReordering`** — proves the comparison relation's own
  properties (reflexivity/symmetry/transitivity, preservation of window sizes,
  emitter projections, per-window content and barriers, and
  `strict_iff_byte_equality`) over an abstract `Stream` whose line bytes are
  deliberately opaque, so no theorem can mention this fixture's derived
  relation.
* **CT06 / CT11 / `QCore.StringConcat`, `QCore.StdString`** — carry the
  borrow-window and static-UTF-8 decisions over the model, not over this
  fixture's statement sequence.
* **CT01 / `QCore.TaskTraceRelation`** — names CT01 in a docstring and proves a
  property of a trace *shape*, which is a property of the model the fixture
  instantiates rather than of the fixture.

Closing this needs a fixture-bound theorem — task T00.7, open since the schema
landed — and this lane does not open it, because a proof claim bought by
renaming a model theorem would be worse than none.

## 6. Fail-closed verification — 18 scratch mutations, 18 refused

Each was applied to the committed tree, run against the named gate, and
reverted with `git checkout --`. **None was committed**; the tree was verified
clean afterwards. The driver's output is archived at
[`artifacts/mutations.json`](artifacts/mutations.json).

| # | Mutation | Refused by |
|---|---|---|
| M01 | CT49 claims its `stderr` relation is strict | *if this case's own stream relation were strict it would be measuring byte equality under another name* |
| M02 | CT49 drops the parent-only control that keeps byte equality | *the partial order must be bounded by a control that keeps byte equality* |
| M03 | CT49 inflates `c_emission` to one emission per run | derived-plan mismatch + `observation_counts` drift |
| M04 | CT49 keeps the sanitizer gate but records `native_sanitized: unassessed` | sanitizer gate + `stage_histogram` drift |
| M05 | CT49's ambiguous-alphabet control claimed to derive a strict relation | *a relation that is never refused is not a premise* |
| M06 | CT49's declared `stderr` loses its trailing barrier line | *the reference's standard error drifted from the pinned rendering* |
| M07 | CT49's concurrent window widened to swallow both barriers | *a relation with no barrier on either side of the concurrent window would not pin where commuting stops* |
| M08 | CT11 declares the `task` route | *the declared route must be the one this verdict mode really takes* |
| M09 | CT06 drops both accepting controls | *a sequential-route rejection needs an admitted control* |
| M10 | CT06's attribution control claimed refused rather than admitted | *control `no_copy.qcore` must be refused* |
| M11 | CT11 pins an empty cascade | *a sequential-route rejection pins its exact ordered code vector* |
| M12 | CT11 back to `unmapped` while its fixture exists | mapped-count pin (36 ≠ 35) |
| M13 | the CT11 retirement dropped from this lane's record | *records an unmapped-case bucket, but the manifest maps this case* |
| M14 | the retirement names a record holding no row for CT11 | *retires the bucket in …, but the record holding it is …* |
| M15 | this lane's record inflates CT49's own sanitized run count | `DRIFT: CT49/native_sanitized: manifest counts 16 runs, … counts 32` |
| M16 | `acceptance.json` left at the wave-25 `ct_mapped: 33` | Rust: *`ct_mapped` is generated from the manifest* |
| M17 | this lane's record removed from the Python generator's file set | the wave-3 CT11 row becomes unretired and refuses |
| M18 | CT06 credits its `parse` stage to CT11's artifact | *must name the exact file its runner executed* |

## 7. Derived numbers

Both derivations agree, independently:

```
ct_mapped 36   ct_unmapped 14
depth_profile     elaborate 2, epoch_bound_check 4, legacy_c_twin 3,
                  mutex_invariant_check 8, native_execution 9,
                  native_sanitized 5, task_ledger_check 4, task_lowering 1
observations      parse 33 -> 36, elaborate 30 -> 33, task_lowering 16 -> 17,
                  task_ledger_check 15 -> 16, task_reference_execution 26 -> 27,
                  c_emission 29 -> 30, native_execution 29 -> 45,
                  native_sanitized 10 -> 26
native_sanitized  executed 5, recorded_not_gated 5, not_applicable 7, unassessed 19
proof             unassessed 36
unmapped_label_sources   5 of 14 lane-measured (CT07/CT08 wave 26, CT41/CT42
                         wave 17, CT46 wave 13); 9 still t00-archive
```

* Rust: `ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` — passes.
* Python: `scripts/ct_stage_summary.py . --check` — see §8 for its one expected
  remaining line.

CT06 and CT11 land at depth `elaborate`, which is what the harness's own rule
derives and this lane did not touch: `derive_stage_plan` treats a refusal whose
codes are all below 500 as not having reached the route's lowering stage. `Q078`
is such a code, so the honest record is that the refusal is reached at or before
elaboration *as far as this derivation can tell* — it is not a claim that the
sequential whole-program check did not run. Changing that rule would move every
route's depth profile and is not this card's business; it is named
`CT-SEQ-DEPTH-RULE` below.

## 8. Gate

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`; see §8.1 |
| `cargo test --workspace --no-fail-fast` | **4,089 covered, 8 ignored.** 4,078 passed in the single run; the 11 that did not are accounted for one by one in §8.1 and all pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | **107** Lean audits, all pass |
| `python3 -m unittest discover -s scripts/tests` | **193** tests, OK |
| `python3 scripts/ct_stage_summary.py . --check` | exit 0, no DRIFT |
| `cargo test -p qcore-compiler --test ct_corpus` | 5 passed |

### 8.1 The eleven, named

Neither group is a source defect, and no assertion was weakened to get past
either.

**Ten: an uninitialised submodule.** This is a fresh worktree, and the first
full run happened before `git submodule update --init --recursive` — the
standing gate's own once-per-checkout step. `libevent_submodule` (1),
`native_library_header` (4), `native_library_header_decoration` (3) and
`profile_loader` (2) all read `third_party/libevent`. After the step, all four
targets pass: 1, 17, 22 and 18 tests. Nothing about this is specific to this
change; the wave-17 CT lane recorded the same thing.

**One: a recorded load-sensitive row.** `enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
failed with `Io` where it expects `Terminated`, the message being
`Broken pipe (os error 32)` — the exact `Io(Broken pipe)`-under-load shape
constraint 16 (`P02-FLAKE`) records for this file's rows. Verified in isolation
per that constraint: `cargo test -p qcore-verifier-host --test
enforced_process_limits -- --test-threads=1` → **12 passed, 0 failed**.

### 8.2 The derived-CT tripwire, and why it does not fire

The acceptance **matrix**'s `ct_conformance` section is derived by
`docs/reports/remaining-d02/build_matrix.py`, and the integrator regenerates it
at close; this card does not hand-edit it (constraint 25). Its counts, histogram
and label sources are therefore stale on this branch by design, and no test
compares those against the manifest, so nothing goes red for them.

One derived field *is* test-compared against the records:
`newly_decidable_not_mapped_reasons`, whose keys are card T00's newly-decidable
ids still unmapped — `["CT24", "CT47"]` — and whose text is read out of the
newest record that labels each case. None of the three cases mapped here is in
that judgment, so the key set does not move.

The text would have moved. This lane re-measured CT24 and CT47 and initially
published labelled `unmapped_reasons` rows for both, which put three Python
tests red against a matrix the card may not regenerate. **The rows were
withdrawn rather than the tests loosened.** Both cases keep card T00's
DECIDABLE-NOW buckets — their classification did not move, and re-verification
reaching the same bucket is the record saying the label holds — so nothing
measured is lost: the measurements are published in this lane's record under
`cases_examined_and_left_unmapped`, in `acceptance.json`'s own hand-kept
reasons, and in §3.5 and §3.8 above. A later lane that wants the *derived* map
to carry them publishes the rows and regenerates the matrix in the same change.

CT07 and CT08 keep their labelled rows: neither is a key of that map, so neither
touches a derived, test-compared field.

`acceptance.json`'s hand-kept copy of the newly-decidable list **is** this
card's to maintain, and was: CT11's row is removed (the manifest maps it now),
and the CT24/CT47 rows carry this lane's measurements.

Three Python tests encoded tree facts this lane moved and were updated:
`test_newly_decidable_ids_must_still_be_unmapped` staged CT11 as its
now-mapped case and stages CT24 instead;
`test_a_measured_label_wins_over_the_archive_and_says_so` pinned the
lane-measured label set (CT11 out, CT07/CT08 in); and
`test_a_changed_lane_label_moves_the_tally_and_drifts_the_record` pinned the
unmapped tally at 17, now 14. No assertion was weakened.

## 9. What this does not establish

* **Mapping is not conformance.** Three more cases execute their own fixtures
  through real machinery. Full language conformance stays `false`.
* **CT06 and CT11 measure one alias shape.** Immutable, shared, inside one
  entry, over an owned byte buffer. No write share, no fractional permission
  arithmetic, no alias crossing a function or a thread, and no pointer value —
  this language has no raw pointer type.
* **CT49 says nothing about the scheduler.** An admitted run says that run's
  stream lies in the admitted set. Sixteen admitted runs say it sixteen times.
  The relation's own premises — one emitter per line's bytes, `io` declared in
  the writing worker's own effect row — are card `OBS-REORDERING`'s, inherited
  unedited.
* **Nothing is promoted.** The Cycle 19 corpus is untouched: raw units stay
  2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only, and the leak
  pin stays 266. The contract corpus stays 73 modules; `qcore-schemas` stays
  14.0.0.
* **No new diagnostic code.** The `Q`-space icebox decision
  (`Q-NAMESPACE-DECISION`) is untouched, and this card needed nothing from it.

## 10. Named follow-ons

| Id | What |
|---|---|
| `CT07-SLICE-PERMISSIONS` | CT07 and CT08 are **one capability** away: the source slice surface (`docs/design/DISJOINT_FOOTPRINT_PROFILE.md` §8 unit F1) that lends a sub-extent of a buffer to a second party. Bounds and indexing already exist and are measured here. |
| `CT23-CREDIT-ERROR-PATH` | CT23 is one capability away: an error path over declared epoch credits. Credits exist (`credit N effort M`, CT38 mapped); the epoch route is check-only with no error region, and the task route has error regions and no credits. |
| `CT24-CLEANUP-PREEMPTED` | CT24's distinguishing fixture is unwritable because `ScopeBusy` refuses the scope exit before cleanup reaches the owner. Deciding it needs either a cleanup path reachable with a live task obligation, or a decision that the preemption *is* the answer. |
| `CT25-EXPIRY-WITH-JOINS` | CT25 needs a loan whose expiry is observable in a program that also joins — the same composition CT24 wants. Loan expiry now exists on the sequential route only. |
| `CT47-COPY-CAPTURE` | CT47 is one **decision** away, not one capability: whether a `Q500` emission refusal plus a grammar gate can carry "reject automatic cross-thread acceptance", given the language has no raw pointer type. The three measured shapes are in §3.8. |
| `CT-SEQ-SANITIZED` | The sequential route's accepting controls emit self-contained C that nothing compiles or runs. A card could give CT06 or CT11 a sanitized control run on the `CT34-SANITIZED` pattern — the control's artifact, never the case's. |
| `CT-SEQ-DEPTH-RULE` | `derive_stage_plan` credits no route stage below a refusal whose codes are all under 500, which puts CT06 and CT11 at depth `elaborate`. Revisiting that rule moves every route's depth profile and needs its own card. |
| `CT09-LAYOUT-PROOF` | CT09 wants an access proof over distinct struct fields. The gateway models members and carries an ABI projection as a **premise** (`LOADER-ABI-LP64`); a premise is not the proof the case asks for. |
| `T00.7` | Unchanged and still open: no theorem is bound to any CT fixture, which is why `proof` stays 0 of 36. §5 names the three near-misses. |
| `D02-GEN-REST` | Untouched: the acceptance matrix's non-CT sections stay the integrator's. |

## 11. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` — three cases mapped; the `note`'s
  sequential-route and CT06/CT11 sentences updated.
* `compiler/tests/ct_corpus.rs` — two new verdict modes
  (`reordering_partial_order`, and the `sequential` arm of `reject`), their
  runners, their pins.
* `compiler/tests/fixtures/ct/CT06/`, `.../CT11/`, `.../CT49/` — new.
* `scripts/ct_stage_summary.py` — this lane's record added to `EVIDENCE_FILES`.
* `docs/reviews/r0_3/acceptance.json` — regenerated (`--write`), plus
  `derived_from` and the hand-kept newly-decidable rows.
* `docs/reports/ct-wire-lane/` — this report, its stage-evidence record and its
  mutation artifact.

**Not** changed, and left for the integrator: `AGENTS.md`'s headline numbers
(CT row `33/50 → 36/50`, sanitized-native `4 of 33 → 5 of 36`),
`DEVELOPMENT.md`'s increment record, and
`docs/reviews/r0_3/acceptance_matrix.json` (regenerate; §8.1).
