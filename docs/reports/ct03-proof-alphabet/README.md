# CT03 proof leg — the third executed-proof capability, and the F02 alphabet grown by the Buffer write (CT03-PROOF-ALPHABET)

Wave 31, card `CT03-PROOF-ALPHABET`. Waves 29 and 30 landed the manifest's
first two executed `proof` rows (CT01, CT37) without touching the F02
alphabet. This card delivers the third — and the one the `CT-PROOF-LEG-2`
survey showed could not exist without growing that alphabet: CT03 is the only
other literal-Accept task case, and its default reference run carries
`BufferWritten` events (`qcore_buffer_write` at two sites), which
`AdmittedCausalOrder::derive` refused as
`Unsupported(UnsupportedObservedEvent { index: 1 })`. The card therefore has
two halves, in dependency order: a **conservative extension of the F02 trace
alphabet** admitting the T05 Buffer write — proved conservative in Lean,
measured conservative in Rust — and then **CT03's proof leg on the CT01/CT37
pattern** over the extended alphabet.

**The manifest, the acceptance matrix, `acceptance.json`, `ct_corpus.rs` and
every EVIDENCE_FILES list are untouched.** CT03's `proof` row still reads
`unassessed` — deliberately, per the card brief. The integrator flips the row
with the exact terms in §7; until then the honest statement is *the
capability exists and is gate-checked, the stage evidence is proposed*.

Base: `main` at `0ef07ba9` (`CT-WIRE-LANE-3`, the wave-31 integration tip). The
card was written against `469f6509` and interrupted by a model usage limit
before its close; the resumed run merged that work forward and **re-measured the
whole gate at the tip** — §8 is the tip measurement, not the earlier one, and
its Lean and Python counts moved with the base rather than with this card.

---

## 1. Half one: the alphabet extension, and exactly what "conservative" means

### 1.1 The Rust production change

`crates/qcore-core/src/task_trace_relation.rs` (the only production file this
card touches — `git diff` over `compiler/src/` is empty, no diagnostic code is
allocated, no word is reserved, no route changes):

* `ObservedEvent` gains `Written { task, buffer, value }`; its actor is the
  writing task, it mentions the generation it writes, and its subject is the
  writing task when that is a child — each the write's obvious reading under
  P1–P3, none of which move.
* `parse_observed_trace` reads the line the renderers already print,
  `QCORE_BUFFER_WRITE task buffer value` (`render_task_observation` and the
  emitted C's `io_emit` route both spell it; before this card the observer
  refused the whole observation at that line).
* `reference_trace` maps `TaskEvent::BufferWritten` into the alphabet instead
  of refusing. `ReferenceTraceError::UnsupportedObservedEvent` thereby lost
  its only producer and is **removed** (no consumer existed outside the
  module); `UnsupportedOutcome` stays — the T06 error payload
  (`ErrorWithBuffer`) is still refused, so the fail-closed boundary moves by
  exactly the one constructor the card names and no further.
* `check_order`'s payload rule (class `8`, `PayloadMismatch`) is split **per
  generation** on whether the generation is ever written:
  * a generation with **no** write keeps *exactly* the pre-extension rule —
    every read reports the allocated payload;
  * a **written** generation's reads must each report the value established
    by the nearest preceding allocation or write (a fold over the trace, the
    `establishedReads` walk).

  The split — rather than a uniform "last established value" rule — is what
  makes conservativity provable flag for flag; the two formulations coincide
  as observable `relate` behaviour, because on a write-free generation the
  last establishment before any read *is* the allocation whenever the
  allocation-order conjunct (class `7`, checked first) has not already
  refused. No other conjunct changes: a write is a *mention*, so use-before-
  allocation and use-after-release already cover a write outside the
  generation's lifetime, and the write needs no new violation class.
* `TRACE_RELATION_IDENTITY` stays `qcore.task-trace-relation.v1`,
  deliberately: the identity exists so that a changed relation retires
  recorded evidence rather than silently reinterpreting it, and the
  conservativity theorem plus the digest measurement below are precisely the
  proof that **no recorded evidence is reinterpreted** — every declarable
  pre-extension order keeps its exact admitted set (a write-carrying rendering
  against a write-free order refuses as `EventMultisetMismatch`, measured in
  `a_rendering_with_a_write_against_a_write_free_order_is_refused`). The
  extension only makes *derivable* orders that previously refused to derive.

### 1.2 The Lean side — `proof/QCore/TaskTraceWrites.lean`

`QCore.TaskTraceRelation` is **untouched except for five sentences of header
comment** pointing here; not one definition, theorem or pin in it moves, and
`Ct01OwnedTransfer.lean` / `Ct37CompletionCapsule.lean` are byte-identical.
The new module defines the extended alphabet (`written` is the eleventh
constructor), the extended relation — **the transcription of the extended
production relation** — with `checkOrder` stated as `firstFailure` over
sixteen *named* flags (only `payloadFlag` differs from the pinned relation,
and only on a written generation), and proves:

* **`relate_agrees_on_the_embedded_alphabet`** — for **arbitrary** traces of
  the old alphabet, conjunct for conjunct:

  ```lean
  theorem relate_agrees_on_the_embedded_alphabet
      (reference observed : List BaseEvent) :
      relate (reference.map embed) (observed.map embed)
        = TaskTraceRelation.relate reference observed
  ```

  with `declare_agrees_on_the_embedded_alphabet` beside it. The proof moves
  every ingredient across the constructor-for-constructor embedding —
  attribution, duplication, the multiset conjunct, the per-actor projections,
  and each of the sixteen order flags, the extended payload flag reducing to
  the pinned one because `isWriteOf` is false on every embedded event.
* **`write_free_traces_relate_exactly_as_before`** — the card's conservativity
  statement, for traces of the *extended* alphabet:

  ```lean
  theorem write_free_traces_relate_exactly_as_before
      {reference observed : List Event}
      (href : reference.all (fun e => !isWrite e) = true)
      (hobs : observed.all (fun e => !isWrite e) = true) :
      ∃ r o : List BaseEvent, reference = r.map embed ∧ observed = o.map embed ∧
        relate reference observed = TaskTraceRelation.relate r o
  ```

  A trace without `written` **is** an embedded old-alphabet trace
  (`write_free_traces_are_embedded`), and on it the extended relation is the
  pinned relation.
* **`mixTrace_agrees_on_the_embedded_alphabet`** — the digest domain grows
  without moving one existing digest: `encodeEvent` gives `written` the fresh
  code `11` and every old constructor its pinned code, so the fold agrees on
  embedded traces — as a theorem, not only by construction.
* **`the_pinned_domain_one_verdicts_are_carried`** — the pinned D1 verdict
  digest `6242011289262393457` re-derived over the extended relation **from
  the old theorem alone** (`verdictDigest_map_embed` plus
  `domain_one_verdict_digest`), no new kernel enumeration: the pins are
  carried, not re-measured.
* **`a_rewritten_payload_is_admitted` / `a_stale_read_is_refused`** — the one
  extended rule is non-vacuous in both directions on the smallest
  write-carrying run (the stale read refuses with the payload class, `8`).

The module also deliberately **promotes** the positional toolkit the CT37
module kept private (positions length/spec/completeness, unique-witness
counting, `mem_sortNats`, `firstFailure_ne_zero`, pair/triple sublists, the
general refusal lemmas restated for the extended relation, and the two named
flag extractions `loanFlag_false_of_checkOrder_zero` /
`activityAfterJoinFlag_false_of_checkOrder_zero`) — the CT03 proof leg
consumes them, and a per-module copy would have had to track the extended
alphabet anyway.

### 1.3 The measurement: not one existing pin moves

Constraint-7-style discipline applied to F02's own pins — **measured, not
assumed**:

| Pinned expectation | Before | After |
|---|---|---|
| D1/D2/D3/5040-extension trace digests (`crates/qcore-core/tests/task_trace_relation.rs`) | `1827949402977241471`, `7621727749705747350`, `9380586544780836206`, `13005161553597186085` | identical, tests green |
| Verdict digests (D1, D2, D3, 5040) | `6242011289262393457`, `3851450914538809003`, `15391469991380593011`, `6228814275926525894` | identical |
| Per-class censuses, admitted counts (1/6/18), edge counts (16/31) | pinned | identical |
| Lean D1–D3 kernel theorems (`TaskTraceRelation.lean`) | pinned | file untouched; audits green |
| CT01/CT37 Lean trace digests | `13023433800416048247`, `17239871955803798867` | identical (runner pins unmoved) |
| F02 generated-C digests (`f02_trace_relation.rs`) | `6524b41d0ccb910f`, `0719a8dcb0192e7e` | identical — emission untouched; all four native tests green |

The table's strongest form is structural, and was re-verified at the resumed
run's tip: **not one pinned constant was edited.** `git diff` over the three
pin-carrying test files (`crates/qcore-core/tests/task_trace_relation.rs`,
`compiler/tests/ct01_proof_leg.rs`, `compiler/tests/ct37_proof_leg.rs`) removes
exactly **three lines** — `assert_eq!(parsed.len(), 13);`, `parsed[12],` and
`assert_eq!(parsed[7].actor(), None);`, all three the parser round-trip
inventory's 13 → 14 index shift — and **no removed line carries a digest, a
census, an admitted count or an edge count**. Every pin therefore stands at its
pre-card value by inspection, and every test carrying one is green in the §8
run. The digest argument is likewise structural as well as proved: `encode_event`
uses codes `1`–`10` with stride `16`, so `Written => 11 + 16 * (…)` occupies a
previously unused code and no existing event's contribution to the fold moves —
which is exactly what `mixTrace_agrees_on_the_embedded_alphabet` states as a
theorem.

Two test files were **edited, deliberately and disclosed**: the three
`encode_event` transcriptions gained the `Written => 11 + …` arm their now
non-exhaustive matches require (changing nothing they encode), and the parser
round-trip test gained the `QCORE_BUFFER_WRITE` line (13 → 14 lines — it is
an alphabet inventory, not a pinned expectation table). Four new tests pin
the write-aware rule: established-value reads admitted, a stale read refused
(`PayloadMismatch`), a write outside the generation's lifetime refused
(`ReleaseOrder`/`AllocationOrder`), and a write-carrying rendering against a
write-free order refused (`EventMultisetMismatch` — conservativity's
observable face).

Constraint 15a is not engaged: nothing here touches the obs-reordering
relation, worker-body trace lines, or either premise the declaration rests on.

## 2. Half two: the CT03 proof leg — `proof/QCore/Ct03ReadShares.lean`

On the CT01/CT37 pattern, over the extended alphabet. New module (audited by
`proof/AuditCt03ReadShares.lean` together with `TaskTraceWrites`; Lean audits
123 → **124** at this card's base — the card's own contribution is one audit,
§8), containing:

* **`referenceRun`** — the transcription of CT03's nineteen-event reference
  trace (allocate payload `1`; write `42`; spawn/lend twice; three reads of
  `42` while the shares are out; both children complete and are joined, both
  shares return; write `7`; read `7`; drop; root completes `7`), proved a
  declared admitted causal order of the extended relation
  (`ct03_reference_run_is_an_admitted_causal_order`, kernel `decide` — the
  very derivation that refused before the alphabet grew), with
  `ct03_trace_digest : mixTrace referenceRun = 9638775376944872586` as the
  transcription pin the Rust runner recomputes.
* **`dischargesSharedReads : List Event → Bool`** — a total decision procedure
  for the scenario property: nineteen facts; the parent's projection is
  exactly the fifteen-fact `parentOrder` (in particular **both writes are the
  parent's**, one on each side of the lending window); each borrower's
  projection is exactly read-then-complete; exactly one allocation and one
  release of the shared generation, one completion and one join per child;
  and **`readSitsInsideTheWindow`** per borrower — the first shared read
  strictly between the loan opening and the share returning, stated
  positionally because no single actor's projection can carry it.
* **Meaning theorems**, each for an *arbitrary* accepted trace:
  `discharged_parent_facts_are_exactly_the_scenario`,
  `discharged_borrower_facts_are_exactly_the_scenario`,
  `discharged_release_is_unique`,
  `discharged_writes_sit_on_either_side_of_the_window` (the six-fact chain
  `[prepared, loanOne, loanTwo, returnOne, returnTwo, restored]` as a
  subtrace — no write while a share is out),
  `discharged_release_follows_the_restored_read`,
  `discharged_reads_report_the_written_values` (every read carries the value
  the preceding write established, and `42 ≠ 1` — the write is observable
  through the shares, which is the T05 crux),
  `discharged_shared_generation_is_the_allocations`, and
  `discharged_each_borrower_reads_inside_its_window` (loan, read, returned
  share as a subtrace, per borrower).
* **`every_admitted_rendering_discharges_the_shared_reads`** — the closure:
  for **arbitrary** traces, `relate referenceRun observed = 0 →
  dischargesSharedReads observed = true`. Projections and counts follow the
  CT01/CT37 closures; the two window clauses are the new work, extracted from
  the relation's own order admission — `loanFlag` (no shared read before the
  loan opens; the share returns exactly once, at or after the join) and
  `activityAfterJoinFlag` (a borrower's own activity precedes its join),
  addressed **by name** through the named-flag structure of the extended
  `checkOrder`, with strictness from the fact that no event is both a read
  and a loan, or both a read and a join.
* **Seven named refusals** — a read before its loan and a read after its
  share returned (projections and counts intact: only the window clause
  refuses each), a write inside the window (CT04's shape, the refusal CT03's
  acceptance is the complement of), a stale after-read (the unobservable
  write), a missing release (the leak shape), a double release, and a
  borrower that writes.

Axiom surface: every theorem's closure is within `propext`,
`Classical.choice`, `Quot.sound` (the `AuditAll` allow-list); the audit
prints each, the digest pin and both non-vacuity theorems are `propext`-only,
and the projection theorems are axiom-free. No `native_decide`, no
placeholder, no project axiom.

## 3. The Rust side — `compiler/tests/ct03_proof_leg.rs`

The proposed executor,
**`compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape`**,
is host-unconditional (no `cfg`, no Clang, no `host_gate`): it compiles
CT03's own fixture through `compile_files`, executes `execute_task_module` on
the checked module, derives the admitted causal order with the production
observer, and (1) asserts the events equal the nineteen-fact transcription of
the Lean `referenceRun` term, event for event; (2) recomputes `mixTrace` with
wrapping `u64` arithmetic and asserts the Lean-proved
`ct03_trace_digest = 9638775376944872586`; (3) runs
`discharges_shared_reads`, the clause-for-clause transcription of the Lean
procedure including both window clauses, and requires acceptance; (4) asserts
the artifact's rendering is admitted by the production `relate` against its
own declared order, so the Lean closure theorem covers it. A second test,
`the_procedure_refuses_the_lean_named_mutations`, re-runs the transcription on
the same seven mutants the Lean module proves refused. Like CT37, there is no
recorded `.trace` fixture file; the binding is artifact ↔ named events ↔
kernel-proved digest ↔ Lean term.

## 4. Exactly what is and is not covered

**Covered.** The property is proved of CT03's *checked artifact*: the
reference execution's admitted causal order under the extended relation, and
every rendering that relation admits against it. Concretely: only the parent
writes, and only outside the lending window; each borrower reads the one
shared generation exactly once, strictly inside its own window, and the value
it reads is the one the parent's preceding write established rather than the
allocation's initial payload; the generation is released exactly once, by the
parent, after both shares are back and after the restored payload is read
back — for the reference trace by kernel `decide`, for arbitrary admitted
renderings by the symbolic closure. And the alphabet extension itself: on
write-free traces the extended relation *is* the pinned relation, by theorem
and by measurement.

**Not covered — stated so the row cannot be over-read.**

* **No native refinement is claimed.** The emitted C's native execution is
  covered only by the existing `native_heap_scenarios` observations (three
  fixed Linux schedules under ASan/UBSan/LSan with the heap witness). The
  manifest's own caveats stand, including "ThreadSanitizer does not intercept
  C11 `thrd_create`, so data-race freedom rests on the checker's
  no-write-while-shared rule" — the theorems are about the reference
  execution and the admitted-rendering relation, not about what the binary
  did.
* **The Lean–Rust tie is transcription agreement, not verified extraction** —
  the F02 pattern, exactly as CT01 and CT37 record it.
* **The alphabet still excludes typed error outcomes** (`ErrorWithBuffer`
  refuses `reference_trace`; the Lean `Outcome` carries none) and
  `CreationFailed` stays unattributable. The `CT-PROOF-LEG-2` survey's
  boundary for the remaining mapped task cases is narrowed by exactly the
  write, nothing else.
* **Nothing outside this profile.** The theorems quantify over F02-alphabet
  traces; they say nothing about other programs, other cases, or the checker.

## 5. Measured results

| Check | Result |
|---|---|
| `proof/QCore/TaskTraceWrites.lean`, `proof/QCore/Ct03ReadShares.lean` under `-DwarningAsError=true` | clean |
| `AuditCt03ReadShares` axiom surface (26 theorems, measured in the §8 run) | within the `AuditAll` allow-list throughout: no printed dependency lies outside `propext` / `Classical.choice` / `Quot.sound`. `ct03_trace_digest` and the three projection theorems **depend on no axioms at all**; `ct03_reference_run_is_an_admitted_causal_order`, `ct03_reference_run_discharges_the_shared_reads`, `mixTrace_agrees_on_the_embedded_alphabet` and both non-vacuity theorems are `propext`-only; the two conservativity theorems carry all three |
| `ct03_artifact_is_the_lean_proved_shared_reads_shape` | green: artifact = `referenceRun` transcription, digest = `9638775376944872586`, procedure accepts, relation admits |
| `the_procedure_refuses_the_lean_named_mutations` | green: all seven mutants refused |
| F02 harness (23 tests incl. every digest/census/edge pin) | green, no pin moved |
| F02 native suite (4 tests, Clang, generated-C digest pins) | green, no pin moved |
| CT01/CT37 proof legs | green, digest pins unmoved |

Every row above was re-run at the resumed tip (`2492825b` + this card) inside the
§8 workspace run, which is green end to end. Raw gate logs: worktree-private
`measure/gate-*.log`. The full gate table is in §8.

## 6. The withdrawal that did not happen

The brief allowed stopping at the boundary if the digest function's domain
could not grow without moving an existing digest. It could: the digest is a
fold of per-event codes, `written` takes the previously unused code `11`, and
every pre-existing constructor keeps its code — so the domain grows and every
existing digest is unchanged, proved (`mixTrace_agrees_on_the_embedded_alphabet`)
and measured (§1.3). No obstruction; no withdrawal.

## 7. Proposed proof-stage evidence (the integrator's flip)

On the exact terms of CT01's and CT37's flips; this card edits none of these
files.

**(a) The manifest record** — CT03's `proof` row in
`docs/reevaluation/ct_manifest.json`:

```json
"proof": {
  "status": "executed",
  "executor": "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
  "artifact": "compiler/tests/fixtures/ct/CT03/case.qcore",
  "observations": 1
}
```

No `host_gate` (the runner is host-unconditional) and no `tool_gate` (the
Lean theorems are checked by the standing `check_lean_audits.py` gate, not by
this runner).

**(b) The `PROOF_GATED_CASES` row** in `compiler/tests/ct_corpus.rs` — the
pinned fail-closed table grows by exactly one entry:

```rust
// Card `CT03-PROOF-ALPHABET`: CT03's shared-reads procedure over the
// extended F02 alphabet (QCore.Ct03ReadShares over QCore.TaskTraceWrites,
// AuditCt03ReadShares.lean).
(
    "CT03",
    "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
),
```

Every case outside the table keeps `unassessed`; the existing
`derive_stage_plan` proof arm needs no edit — it already reads the table.

**(c) The stage-evidence record** — a new
`docs/reports/ct03-proof-alphabet/ct_stage_evidence.json` on the shape of the
CT01/CT37 flips, written by the integrator at the flip:
`mapped_case_stage_matrix.CT03` with
`"supersedes": "docs/reports/remaining-wave3-ct-integration/ct_stage_evidence.json"`
— **the record currently holding CT03's stages** (verified: CT03's
`verified_stages` live in the wave-3 CT-integration record, which is
superseded from the new record and never edited, constraint 21) — every
non-proof stage row carried unchanged, and the `proof` row:

```json
"proof": {
  "status": "executed",
  "executor": "compiler/tests/ct03_proof_leg.rs::ct03_artifact_is_the_lean_proved_shared_reads_shape",
  "artifact": "compiler/tests/fixtures/ct/CT03/case.qcore",
  "observations": 1,
  "note": "the artifact checked against the Lean-proved total decision procedure dischargesSharedReads (QCore.Ct03ReadShares over the write-extended alphabet QCore.TaskTraceWrites, AuditCt03ReadShares.lean); reference-run discharge and admitted renderings covered, including both per-borrower window conjuncts extracted from the relation's own loan and activity-after-join admission; the alphabet extension proved conservative (write_free_traces_relate_exactly_as_before) with every pre-existing digest pin measured unchanged; native execution deliberately not (observation only)"
}
```

plus the matrix regeneration through the standing generator with hand-kept
sections carried (constraint 25), and `validation.lean_audits` regenerated
123 → 124 (§8.1).

**(d) The headline numbers.** This card moves the proof **numerator by exactly
one** and moves the denominator **not at all** — it maps no case, and the
manifest is untouched. The parallel lane `CT-WIRE-LANE-3`, which maps **CT25**,
merged to `main` (`0ef07ba9`) while this card was being closed and **is merged
into this branch**; the tip therefore holds **43 mapped / 7 unmapped**, and
`AGENTS.md` reads `proof 2 of 43`. The flip accordingly reads **`proof 2 of 43`
→ `proof 3 of 43`**, with the mapped denominator and sanitized-native (`7 of
43`) both carried untouched.

The two lanes compose in either merge order and were verified to: they touch
disjoint manifest rows (CT25's and CT03's), this card edits neither, and the
merge of `0ef07ba9` into this branch conflicted **only** in the wave-31
increment file — a two-paragraph ordering, no shared code or evidence. CT25 is
mapped onto the `composed_epoch` route, not the task route, so it does not
reach the F02 relation at all; had it, the alphabet extension would be safe for
it by the conservativity theorem rather than by coincidence, a write-free run
relating exactly as before. Per constraint 23, a grown denominator is mapping
rather than conformance, and a grown proof numerator is evidence about one
case's artifact.

Suggested reading of the flipped row: *proof — the scenario property proved
of the checked artifact's reference run and its admitted renderings under the
write-extended relation; native execution unproved, covered by observation
only.* A grown proof numerator is evidence about this case's artifact, not
language conformance (constraint 23's spirit applies to this denominator
too).

## 8. Gate

Run at the resumed card tip on Linux, over base `0ef07ba9`, under default cargo
configuration (no test, pin or assertion edited to obtain the results; the two
disclosed test-file edits are §1.3's). The gate was measured twice — once over
`2492825b` and again in full after `CT-WIRE-LANE-3` merged into the branch
mid-close. **Every row below is the second, tip measurement**, and every row was
identical in both.

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,490 passed · 0 failed · 8 ignored**, 224 binaries, exit 0 |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | `Passed 124 Lean audits` |
| `python3 -m unittest discover -s scripts/tests` | **199 of 200 green**; the one failure is the standard expected tripwire, §8.1 |

**No constraint-16 row tripped in either run**, so no isolation re-verification
was needed; both facts are reported as required. The workspace count is the
wave-31 integration tip's own **4,484** plus exactly this card's **six** new
tests (the two `ct03_proof_leg.rs` tests and the four write-aware F02 harness
tests of §1.3) — the delta is accounted for test by test, not merely plausible.
`CT-WIRE-LANE-3` net-added **no** workspace test (its `region_joins_loan.rs`
binary reports 11 passed in both runs, its fixtures having moved rather than
grown), which is why the same 4,490 holds over both bases and the decomposition
is unaffected by the mid-close merge.

The earlier interrupted run measured this gate at base `469f6509` and recorded
`Passed 122 Lean audits` with 199 of 200 Python tests. Both counts moved with the
**base**, not with this card: `C-GATEWAY-WAVE` landed one audit of its own
(122 → 123 before this card) and the Python suite reached 200 there. This card's
own contribution to each is unchanged at **+1 audit and +0 Python tests**.

### 8.1 The one expected failure

`scripts/tests`'
`test_the_derived_facts_are_recomputed_rather_than_carried` trips on
`validation.lean_audits`, which moves 123 → **124**:

```
AssertionError: Lists differ: ['14.0.0', 123] != ['14.0.0', 124]
```

This card's brief forbids editing `docs/reviews/r0_3/acceptance_matrix.json`, so
the derived row is deliberately left stale for the wave integrator to regenerate
— the handling every audit-adding card's delta receives (`CT01-PROOF-LEG` and
`CT-PROOF-LEG-2` record the same shape). The delta is entirely this card's one
audit file, `proof/AuditCt03ReadShares.lean`; the other 199 tests are green, and
no test was edited or skipped to reach that number.

## 9. Follow-ons

* `CT03-PROOF-ROW` — the integrator flip of §7.
* `CT-PROOF-NATIVE-REFINEMENT` — unchanged; this card neither closes nor
  narrows "native refinement remains open".
* `CT-PROOF-FOURTH-CASE` — the measured boundary after this card: within the
  now-extended alphabet, CT50 remains the one mapped case whose default run
  both derives and exhibits its scenario (with the honesty caveat CT37's
  survey recorded); every other mapped task case still needs error outcomes
  or attributable failure events — a further deliberate alphabet decision,
  not a mechanical repeat of this one, because `CreationFailed`'s
  unattributability is a stated premise (P2) rather than a missing
  constructor.
* `F02-NATIVE-WRITE-RELATION` — the native F02 suite still exercises only the
  two write-free programs; a native run of a write-carrying program through
  `parse_observed_trace`/`relate` (now expressible end to end) would be new
  evidence, not a regression risk, and no claim is made here.
