# Wave-32 CT lane — CT44 and CT47 mapped, each with its one inexpressible word published (CT-WIRE-LANE-4)

Wave 32, card `CT-WIRE-LANE-4`, on the `CT22-WIRE` / `CT-WIRE-LANE-2`
mechanics precedents, the `CT-WIRE-LANE-3` withdrawal-with-measurement
precedent, and — decisively for both decisions — the **`CT06` precedent for an
inexpressible vehicle**. Two decisions, the two the brief named cheapest:
**CT47** — *"Raw pointer is Copy but no transfer/share contract exists"*,
expected **Reject automatic cross-thread acceptance** — is **MAPPED** on the
`task` route, exactly as `docs/reports/ct47-copy-capture/README.md` §6
prescribed for a lane taking the row; **CT44** — *"Stack capture expires or
environment moves before join"*, expected **Reject** — is **MAPPED** beside it,
after a measurement that found **both** of card T00's archived blockers for it
obsolete.

CT coverage moves **43/50 → 45/50** mapped, unmapped **7 → 5**. Harness-gated
sanitized-native stays **7, now of 45**. Proof stays **3 of 45** (CT01, CT03,
CT37). A grown denominator is mapping, not conformance (constraint 23).

Base: `main` at `1c7d8e45` (the wave-31 close plus the `STABILITY-REEVAL`
commit). Branch: `worktree-agent-a37c7fabef385fed7` in an isolated worktree.

**Nothing is promoted, and nothing semantic changed.** `git diff` over
`crates/` and `compiler/src/` is empty for this card: the only Rust is harness
wiring in `compiler/tests/ct_corpus.rs`. No checker rule, no diagnostic code
(the Q-space icebox needed none — this card allocates zero and widens zero
conditions), no lowering, no model, no reserved word. **This wave ran no
arbiter**, so the Cycle 19 corpus rows and the leak pin are carried by
construction.

---

## 1. The decisions

| Case | Expected | Readiness | Decision |
|---|---|---|---|
| **CT47** | *Reject automatic cross-thread acceptance* | `CT47-COPY-CAPTURE` (wave 29) §6 | **MAPPED** — the transferability judgment refuses at the crossing construct, naming the missing contract and enumerating the three this kind carries; **all three** are supplied by accepted controls that each admit the program (§3) |
| **CT44** | *Reject* | none — card T00's archive, `ambiguous`, was the standing word | **MAPPED** — the checked task ledger's `LoanOutstanding` refuses, **naming the borrowing task**, and both of T00's blockers are measured obsolete (§4) |

Both join the `task` route's `reject` set beside CT04, CT05 and CT18, taking it
from three cases to five.

Neither mapping is a `Q000` parse failure, neither rests on a counterfactual,
and neither claims its scenario word for word: each has exactly **one** word
this language gives no referent, published in that case's own semantic profile
and in its fixture's header comment.

### 1.1 The standard both were held to

The CT22 standard: does the fixture give **every word** of the scenario a
checked referent, each carried by a **named rule of an unedited model**? Where
a word has no referent at all, the lane may still map **only** on the CT06
precedent — that case's scenario says *"Same pointer copied into differently
named variables"* and is mapped on a `{ptr, len}` view, with *"pointer values
the language gives no type remain outside this route"* published in its own
semantic profile. Read as a rule, the precedent says: **map on the nearest
expressible shape when the scenario's PROPERTY is fully carried and only its
VEHICLE is inexpressible, and publish the vehicle as a boundary.** Both
decisions below are taken under it and say so.

## 2. What was measured, before anything was written

Every shape below was run through the real pipeline on this tree
(`./target/debug/quirin check`), transcripts in `measure/` (worktree-private).

**CT47 — the three wave-26 shapes, re-measured:**

| Shape | Measured on this tree |
|---|---|
| `spawn s reader(buffer)` — no declared mode | `Q000`: *capture requires copy, move, read … share, lend … LOW..HIGH, or write* |
| `spawn s reader(copy buffer)` — the bit-copy | `Q600`: *the transferability judgment refuses this capture: no transfer or share contract admits `copy` of `own<Buffer>` across a spawn boundary; `own<Buffer>` crosses only under `move` …, `read PLACE share N` …, or `lend PLACE LOW..HIGH` …* |
| `spawn s reader(read buffer share FULL)` | admitted, `ScopedReadLoansV1` |

Unchanged from card `CT47-COPY-CAPTURE`'s published measurement but for one
thing, which is an improvement for this lane rather than drift: **both messages
now name a third contract**, `lend PLACE LOW..HIGH`, added by
`CT07-SLICE-PERMISSIONS` in wave 27. The case therefore supplies three accepted
controls rather than the two the readiness statement offered.

**CT44 — the scenario's shapes, measured first, per the brief:**

| Shape | Measured on this tree |
|---|---|
| `drop buffer;` between spawn and join, share out | `Q600: LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2] }`, attributed to the `drop buffer;` statement |
| a second `spawn s taker(move buffer)` while the share is out | the **same** `Q600 LoanOutstanding`, attributed to the moving statement |
| a bare `return` out of the thread scope, share out | `Q600: ScopeBusy(5): cleanup preempted — this exit would run the frame's cleanup over resource 1 while task(s) [2] still hold a read share of it, destroying the owner before the child's join` |
| the same `drop buffer;` in the same position, scalar `copy` capture instead | **admitted**, `ScopedOwnedTasksV1` |
| the same program with the drop **after** the join | **admitted**, `ScopedReadLoansV1` |

## 3. CT47 — the honesty question, answered in the affirmative

### 3.1 Word by word

* ***but no transfer/share contract exists*** — carried by the judgment itself.
  The refusal is `qcore_core::transfer`'s own `TransferRefusal`, emitted at
  `SemanticPass::Concurrency` and attributed to `capture.span`, with a message
  that names the missing contract **and enumerates the contracts the kind does
  carry**, read from the judgment's own classification rather than restated by
  hand. This is a semantic judgment about why the value may not cross — which
  is exactly what the wave-26 lane found missing when the same shape fell
  through to the C backend's `Q500` executable-subset catch-all, the "the
  profile cannot express this at all" vacuity the wave-9 attribution rule
  excludes.
* ***is Copy*** — carried at the **mode** level, and stated as such. `copy` is
  this route's bit-duplication mode, and the program asks for exactly the
  bit-copy the scenario's premise names. The refusal is **for the mode, not the
  kind**: `own<Buffer>` is classified *sendable*, so this is not a vacuous
  "this kind never crosses". That is precisely what card `CT47-COPY-CAPTURE`'s
  audited theorem
  `QCore.Transferability.copyOfTheOwnedBufferIsRefusedForItsMode` pins, naming
  it CT47's own cell, with `theBoundaryIsDecidedExactlyOnce` showing refusal
  and admission partition the whole 13 × 4 × 13 boundary matrix.
* ***Reject automatic cross-thread acceptance*** — carried in **two measured
  halves**. The explicit request is refused by the judgment (the case). The
  *automatic* form is not spellable: a capture with no declared mode does not
  parse, so there is no acceptance for the checker to grant automatically and
  none to withdraw. Pinned as `modeless.qcore`, a refusing control at `Q000`,
  deliberately **not** as the case — a parse failure is a statement about the
  surface, and the case's evidence is the semantic judgment one layer in.
* ***Raw pointer*** — **no referent.** §3.3.

### 3.2 The attribution, which is complete rather than illustrative

The refusal **enumerates** the contracts this kind carries. All three are
supplied, and every one of them admits the program:

| Control | Role | Measured |
|---|---|---|
| `move_transfer.qcore` | the first named contract | **admitted**, `ScopedOwnedTasksV1` |
| `read_share.qcore` | the second named contract (byte-identical to the case but for the mode) | **admitted**, `ScopedReadSharesV1` |
| `write_lease.qcore` | the third named contract | **admitted**, `ScopedWriteLeasesV1` |
| `modeless.qcore` | the *automatic* half | **refused**, exact cascade `["Q000"]` |

So the `Q600` is the **absence of a contract for `copy`**, not an inability of
this value to cross a spawn boundary. The wave-9 rule asks that removing
exactly the construct the scenario names move the verdict; here supplying any
one of the named contracts — and nothing else about the program — does.

### 3.3 The boundary, published rather than papered over

**`Raw pointer` has no referent.** This language has no pointer type;
`DeclaredTypeTag::Pointer` is `Unmodeled`, and a declared pointer parameter
refuses long before a capture is considered. The bit-copy of an owned handle is
the nearest expressible shape.

The sharper half, and this lane's own finding rather than the readiness
statement's: **no QCore type is both freely duplicable and resource-bearing**,
because resource kinds are linear. The dangerous *combination* CT47 names —
a value the type system treats as freely duplicable that nonetheless denotes a
resource — is therefore **prevented by the type system**, not caught by a
checker. What the mapped fixture demonstrates is the rule that makes the
combination impossible, at the boundary where it would matter. That is why
*"is Copy"* is recorded above as carried at the mode level and **not** at the
type level: `own<Buffer>` is not `Copy` in the type system, and saying it is
would be the overclaim.

Taken together with §3.1 this is the CT06 shape exactly: the property the
expectation names (*reject automatic cross-thread acceptance*) is fully
carried; only the vehicle (*raw pointer*) is inexpressible; the vehicle is
published.

## 4. CT44 — measured first, and both of T00's blockers found obsolete

Card T00's archived triage bucketed CT44 `ambiguous` on two blockers. This lane
measured before deciding, per the brief, and **both are obsolete**:

| T00's blocker (2026-09-07) | Measured on this tree |
|---|---|
| *"the available rejection is about the handle, not about a capture whose environment expires"* | **Obsolete.** The rejection this case is mapped on is `LoanOutstanding { resource: 1, outstanding: 1024, borrowers: [2] }` — it names the resource, the outstanding amount and the **borrowing task**. That is the capture, not the handle. (T00's "available rejection" was the `ScopeBusy` one, which this lane pins as a control, §4.2.) |
| *"captures are copied into a checked environment, so `environment moves before join` has no source form"* | **Obsolete.** True at T00's checkpoint — its own CT03 row records share captures refused at `Q500`, *"only copy i32 and move own<Buffer> captures are executable"*, so no loan could be outstanding at all. Here `move buffer` handing custody to a second worker while the first worker's share is out is refused at the same code, attributed to the moving statement. Pinned as `move_before_join.qcore`. |

No card between T00 and this lane re-examined CT44; the capability that
dissolved both blockers is the read-share capture itself (card `T05`) and the
checked ledger around it.

### 4.1 Word by word

* ***capture*** — the checked task IR's own loan, derived at the spawn, named
  in the refusal by its borrower.
* ***expires*** — the written `drop buffer;`, whose executed meaning on this
  route is a real destruction: the identical program with that statement after
  the join is **admitted**.
* ***before join*** — that relative position is exactly what moves the verdict,
  pinned in **both** directions (§4.2).
* ***or environment moves*** — the second disjunct, `move buffer`, refused at
  the same code.
* ***Stack*** — **no referent.** §4.3.

### 4.2 The controls, which separate the construct from its position both ways

| Control | Role | Measured |
|---|---|---|
| `drop_after_join.qcore` | the position, false direction | **admitted**, `ScopedReadLoansV1` — byte-identical to the case but for one statement's position |
| `scalar_capture_drop.qcore` | the capture, false direction | **admitted**, `ScopedOwnedTasksV1` — the identical destruction in the identical position, a child still live and unjoined, but capturing a scalar instead of the buffer's share |
| `move_before_join.qcore` | the scenario's second disjunct | **refused**, exact cascade `["Q600"]`, the same `LoanOutstanding`, attributed to the moving statement |
| `frame_exit_before_join.qcore` | the frame's own expiry | **refused**, exact cascade `["Q600"]`, `ScopeBusy` naming the frame |

The second control is what this verdict mode does not require and the case
needs anyway: with only the first, "the refusal is keyed to destroying a
resource while some child runs" would be an untested alternative reading. With
both, the refusal is keyed to the **capture**, and to its position relative to
the join, and to nothing else.

### 4.3 The boundary, and why the frame-exit shape is a control and not the case

**`Stack` has no referent.** The captured storage is a **heap** buffer
(`qcore_buffer_new`) bound to a parent-frame local, and this language has no
stack-allocated capturable resource. What carries the hazard is that
`thread scope` **is** the scoped-thread construct — the same discipline that
makes stack captures safe elsewhere — and the ledger refuses exactly the expiry
or transfer of the captured resource before the join. Whether the storage is
stack or heap is below this model's resolution. CT06 precedent, as above.

**The objection this is weakest against, stated and answered.** One can read
the refusal as a *custody* rule ("you may not destroy a resource someone
borrowed") where CT44 asks for a *lifetime* rule ("a capture may not outlive
its environment"), and call the mapping a conflation. The answer is that in a
scoped-thread discipline these are the same rule reached from two directions:
the obligation being enforced is that the captured thing outlives every capture
until the join, and QCore discharges it with a counted-share ledger where
another language would use a lifetime. The measurement supports that reading
rather than merely permitting it — the verdict moves with the capture
(`scalar_capture_drop.qcore`) and with the position relative to the join
(`drop_after_join.qcore`), which are the two quantities a lifetime rule is
about, and not with anything else. What the mapping does **not** claim is a
storage-class distinction: there is none to make here, and the residual case
CT44 would still name — a capture of storage that dies with the frame *itself*,
distinct from a resource the frame owns — is not expressible and is not claimed.

The `frame_exit_before_join.qcore` shape is the one with a *frame* in it, and
its message says so. It is nonetheless a **control**, for a stated reason: card
`CT-WIRE-LANE-3` (wave 31) examined that exact refusal for CT24 and recorded
that the checked task route **executes no destroy at scope exit**, so the
"cleanup" its message invokes is a counterfactual of the message rather than a
fact of the tree. That objection is about the word **CT24** names
("cleanup destroys an owner") and not about the word **CT44** names
("capture expires") — a frame really does end at that `return`, and its ending
really is what the ledger refuses — but the case is written instead on the
statement whose executed meaning an **admitting control demonstrates**, which
is the stronger form and needs no such distinction. Both readings are pinned,
and neither rests on the other.

### 4.4 Coherence with the CT24 withdrawal

Lane 3 withdrew CT24 on two grounds. Neither bites here, and that was checked
rather than assumed:

* **Attribution.** Lane 3's ground was that removing the construct CT24 names
  does not move the verdict, so no control can isolate it. For CT44 both
  removals move the verdict (§4.2), measured.
* **Coherence.** Lane 3's ground was that mapping CT24 on the task-route exit
  would give *"cleanup destroys owner"* a different referent from CT25's
  adjacent cell. CT44's words are different words; no cell is contested. CT44's
  case shape is also **not** the shape lane 3 declined — it is the explicit
  written `drop`, which lane 3 itself pinned as CT25's `live_share_drop.qcore`
  control on the same rule. One rule carrying CT25's premise-false direction
  and CT44's case is coherence, not conflict.

**CT24 stays unmapped**, its `DECIDABLE-NOW` bucket and its residual
`CT24-CLEANUP-TASK-COMPOSITION` untouched by this card.

## 5. Proved vs. observed (`STABILITY-REEVAL`)

Stated in those words, per the discipline `CLAUDE.md` instituted after
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`. For every claim these two semantic
profiles make:

| Claim | Driven by a named test, or inference? |
|---|---|
| Both cases refuse with the pinned exact ordered code vector | **Driven.** `ct_corpus.rs::ct_manifest_and_fixture_set_agree_and_mapped_cases_execute` compiles each case's own fixture and compares the whole vector, plus the `message_contains` substring |
| All eight controls behave as pinned | **Driven.** `run_source_controls` compiles every one; accepted controls additionally have their `SourceExecutionProfile` compared, so a control cannot silently pass through another route |
| The four executed stages, and only those | **Driven**, and *derived* rather than asserted: `derive_stage_plan` computes the plan from the harness's own dispatch and the codes the compiler really produced, and refuses any disagreement with the manifest |
| CT47's refusal is for the **mode**, the kind still sendable | **Inference from proof, not from this fixture.** Carried by card `CT47-COPY-CAPTURE`'s audited `QCore.Transferability` theorems, which are about the **model's** judgment. No test in this lane distinguishes a mode refusal from a kind refusal on this artifact, and no proof stage is read from those theorems — `proof` stays 3 of 45, `T00.7` stays open for both cases |
| CT44's `drop buffer;` has an executed destroying meaning | **Driven, indirectly and deliberately.** No runner executes either fixture (both are refusals). What is driven is that the *admitting* control checks, which is what makes the statement's meaning a fact of the route rather than of the message |
| The emitted C of either case | **Neither.** Not claimed at all; `c_emission` and `native_execution` are `not_applicable` because the refusal provably never reaches the backend |
| `native_sanitized` / `proof` on these cases | **Open items, not claims.** Both read `unassessed`, by this corpus's counting rules (native_sanitized is `not_applicable` only on a route with no `c_emission` stage, and the task route has one; `proof` is never `not_applicable`) |

One unexercised arm was found while doing this and is **stated as a boundary,
not repaired**: `write_capture` — the fourth capture mode — still parses and
still has no lowering, and this lane adds no fixture driving it. Its refusal
moved to the judgment in wave 29; nothing here changes or exercises it.
Recorded as `CT47-WRITE-CAPTURE-UNEXERCISED` (§8).

## 6. The constraint-25 measurement

CT47 is one of the two keys of the **derived**
`acceptance_matrix.json :: ct_conformance.newly_decidable_not_mapped_reasons`
map, so mapping it moves a field this lane may not hand-carry. Measured with
this lane's own probe before anything was published —
`measure/constraint25_probe.py`, worktree-private, on lane 3's precedent — in
five stages against scratch copies, the committed tree read-only throughout:

| Stage | Derived key set | `ct_stage_summary.py --check` |
|---|---|---|
| baseline | `["CT24", "CT47"]` | exit 0, `mapped 43 / unmapped 7` |
| CT44 staged mapped | `["CT24", "CT47"]` — **unmoved** | the ordinary derived-summary drift a `--write` repairs; **no matrix row** |
| CT47 staged mapped | `["CT24"]` | three further rows red: acceptance.json's hand-kept list, its hand-kept reasons map, and **the committed matrix's** |
| both, then `--write` + the two hand-kept edits + `derived_from` grown | `["CT24"]` | exactly **one** row left — the committed matrix's |
| … then regenerated through the standing generator | `["CT24"]` | **exit 0**, `mapped 45 / unmapped 5` |

So CT44 was free of the trap entirely (it is not in card T00's newly-decidable
list), and CT47's repair path is complete and was walked end to end before
being walked for real.

### 6.1 The clobber question, measured rather than assumed

Constraint 25 forbids a whole-file regeneration clobbering later
reconciliations. Measured: a **bare** regeneration moves
`validation.measurement_provenance` from
`integrator_supplied (docs/reports/wave31-close/gate.json)` to the
carried-and-unmeasured warning — a downgrade of a published provenance in a
section this lane does not own (`validation.*` is `INTEGRATOR_SUPPLIED`).

That is preventable through the standing mechanism itself, with no hand edit:
passing the **committed gate file, by its relative spelling**, from the
repository root — `--gate docs/reports/wave31-close/gate.json` — reproduces the
committed provenance string byte for byte, because the generator prints the
option string verbatim into that field. The regeneration then asserts nothing
new: the gate numbers are wave 31's either way, and now say so in the same
words the committed file already used. (The relative spelling is load-bearing:
an absolute path produces a different string and the section moves.)

Measured outcome of the regeneration actually committed
(`measure/verify_matrix.log`):

```
top-level sections moved: ['ct_conformance', 'dimensions', 'named_followups']
  header / requirements / unverified_specs / corpus
  / validation / versions / workspace     all unchanged: True
dimensions, field by field:
  MOVED   named_followups: 39 -> 42
named_followups row count on disk: 42
```

Three movers, each accounted for:

* `ct_conformance` — derived, and this lane's own.
* `named_followups` — this lane **appends its two residuals** (§9). That section
  is `HAND_MAINTAINED` and carried, and its own ownership note says *"waves add
  entries to the committed matrix and this script carries them"*, so appending
  is the documented mechanism. No existing row is edited or removed.
* `dimensions.named_followups` **39 → 42** — a **derived** count
  (`SECTION_OWNERS`: *"counted from the emitted rows"*). Two of the three are
  this lane's new rows. The **third is a pre-existing defect this regeneration
  repairs**: the committed count read 39 against **40** rows already on disk,
  because commit `1c7d8e45` added the
  `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` row to the carried section without
  moving the derived count beside it, leaving the committed matrix internally
  inconsistent between regenerations with nothing checking the pair. Correcting
  it is a repair, not a clobber, and it is published as such rather than
  absorbed: **`D02-DIMENSIONS-FOLLOWUP-COUNT`**.

Measured both ways, so the repair is separable from this lane's own additions:
regenerating the *unmodified* committed `named_followups` produced
`39 → 40` (`measure/constraint25_probe.py` stage 5 and the first committed
regeneration), and appending the two rows then produced `40 → 42`.

### 6.2 Why the matrix's line diff is large, and what that itself measured

The committed matrix's **serialization** also changes, so `git diff` reports
roughly the whole file. This is a **restoration, not a reformat by this lane**,
and tracing it produced a second finding, which is why it is recorded here
rather than waved past. The generator has written `indent=1, ensure_ascii=False`
since card `HYGIENE-16` changed it from `indent=2, ensure_ascii=True`
(`d31a8dc3`). Measured across the history of the committed file:

| Revision | Matches generator format? |
|---|---|
| `d31a8dc3` (HYGIENE-16) | **yes** — `indent=1` |
| `b5838915` (*"Matrix: regenerate derived rows at the five-card tip"*) | **yes** |
| `8bf7fd33` (wave-31 close) | **yes** |
| `1c7d8e45` (this card's base) | **no** — `indent=2`, the pre-HYGIENE-16 format |

So the file was written by its own generator through the wave-31 close, and
commit `1c7d8e45` rewrote it **outside** the generator — which is the same
commit that added a `named_followups` row and left `dimensions.named_followups`
stale at 39. One hand edit, two consequences: a desynchronised derived count and
a whole-file reformat away from the generator's output. Both are repaired by
this lane's regeneration, and both are folded into
**`D02-DIMENSIONS-FOLLOWUP-COUNT`** (§9), whose statement is therefore *"a hand
edit to this file drifts it from its generator"* rather than only *"a count can
go stale"*. The semantic scope is unaffected and was verified on **parsed JSON**,
not on lines: `measure/verify_matrix.py` compares the committed baseline
(byte-identical to `1c7d8e45`'s file, checked) against the regenerated one
section by section, and reports exactly the three movers above.

## 7. Pinned tables, stage evidence and the derived numbers

Grown deliberately (invariant 5), each stated in the commit message:

* `MAPPED_CASES` 43 → **45** (+ CT44, CT47);
* `SOURCE_REJECT_CASES` = `["CT04", "CT05", "CT18", "CT44", "CT47"]`;
* `EVIDENCE_FILES` 18 → **19** (this lane's record), in **both** derivations —
  `compiler/tests/ct_corpus.rs` (whose array length is itself pinned) and
  `scripts/ct_stage_summary.py` — and in `acceptance.json`'s `derived_from`,
  the three-way tie card `CT-LABELS` built;
* one new arm in the harness's `execution_profile` label map,
  `ScopedWriteLeasesV1`, for CT47's third accepted control;
* the manifest's CT44 and CT47 rows (route, fixture, the semantic profile
  stating which rule carries each word and the boundary verbatim, stage
  records, `harness_verdict_mode`, `evidence_target`) and the note's wave-32
  paragraph;
* `SANITIZED_GATED_CASES`, `PROOF_GATED_CASES`, `SANITIZER_TRANSCRIPT_CASES`,
  the route table and the runner registry are **unchanged** — this lane adds no
  runner, no sanitizer claim and no proof claim.

**No retirement is declared, and that is the mechanism working, not an
omission.** Both cases were archive-held —
`ct_stage_summary.py::unmapped_labels` resolved CT44 (`ambiguous`) and CT47
(`DECIDABLE-NOW (coarser reason)`) to `t00-archive` before this lane — so no
lane record ever published an `unmapped_reasons` row for either, and both
derivations refuse a `retired_unmapped_reasons` entry naming a record that
holds no row (stale) or the archive (vacuous). Mapping removes them from the
unmapped set and their archive labels stop resolving, exactly as CT21's, CT06's
and CT25's did in earlier waves. This lane publishes **no** labelled
`unmapped_reasons` row for any case.

Two things the manifest rows got **wrong on the first pass and the harness
caught**, recorded because both are the derivation doing its job: the two cases
carry `evidence_target: "linux"` (a `source_reject` case with an accepted
control is target-scoped, that control compiling through the host-gated
emission path), and their `native_sanitized` and `proof` stages read
`unassessed`, not `not_applicable` — the task route has a `c_emission` stage,
and a theorem could be bound to any case. Both were then read off **CT04**, the
same route under the same verdict mode, rather than retyped.

Three Python **tree-fact** pins updated on the `CT22-WIRE` §5.1 precedent, no
assertion weakened: the scratch bucket tally sum 7 → **5**, and three tests'
example ids moved from CT44/CT47 to **CT10/CT24**, the two cases still unmapped
and still archive-held. Each carries a comment saying it is a tree fact and
which card moved it.

Both derivations agree, independently
(`ct_corpus.rs::ct_acceptance_summary_is_derived_from_the_manifest` and
`python3 scripts/ct_stage_summary.py . --check`, both green):

```
ct_mapped 45   ct_unmapped 5
depth_profile     elaborate 4, epoch_bound_check 4, legacy_c_twin 3,
                  mutex_invariant_check 8, native_execution 7,
                  native_sanitized 6, proof 3, task_ledger_check 9,
                  task_lowering 1
native_sanitized  gated 7 of 45 · proof executed 3 (CT01, CT03, CT37)
unmapped_label_sources   3 of 5 lane-measured (CT41/CT42 wave 17, CT46 wave 13);
                         2 still t00-archive (CT10, CT24)
```

`acceptance.json` is regenerated through the recorded mechanism
(`ct_stage_summary.py --write`), with `derived_from` grown by this record, the
two hand-kept halves edited (CT47's rows removed from
`newly_decidable_not_mapped` and its reasons map), the wave-32 paragraph
appended, and **two stale current-state sentences refreshed by re-reading the
manifest**: the sanitized/proof sentence still read *"2 of 43 (CT01, CT37)"*
after the wave-31 integrator flipped CT03's proof stage, and the label-source
sentence still read *"3 of the 7 measured … the other 4"*. Both are now read off
the manifest and say in the text that they were.

## 8. What this does not establish

* **Mapping is not conformance.** Two more cases execute their own fixtures
  through real machinery. Full language conformance stays `false`.
* **Neither scenario is claimed word for word.** Each has one word with no
  referent — `raw pointer` and `stack` — published in §3.3, §4.3, each case's
  semantic profile and each fixture's header.
* **CT47's scenario is prevented, not merely refused.** No QCore type is both
  freely duplicable and resource-bearing; the fixture demonstrates the rule at
  the boundary, not the combination.
* **No native, sanitizer or proof evidence for either case.** Both fixtures are
  refusals that reach no backend. Sanitized-native stays 7 of 45, proof 3 of
  45, `T00.7` open for both.
* **The audited Lean proves the model, not these fixtures.** §5.
* **`write` captures stay unexercised.** §5, `CT47-WRITE-CAPTURE-UNEXERCISED`.
* **CT24's classification did not move**, and neither did any other unmapped
  case's.
* **Nothing is promoted.** The Cycle 19 corpus is untouched: no gateway,
  contract, audit or provider file is in this card's diff, and this wave ran no
  arbiter, so the corpus rows and the leak pin are carried.

## 9. Named follow-ons

| Id | What |
|---|---|
| `D02-DIMENSIONS-FOLLOWUP-COUNT` | **New, and repaired here rather than only named.** Commit `1c7d8e45` edited `acceptance_matrix.json` **outside its generator**, with two measured consequences: `dimensions.named_followups` read 39 against 40 rows actually on disk, and the whole file was reserialized to the pre-`HYGIENE-16` `indent=2` format (§6.2 traces both). This lane's regeneration repairs both. The follow-on is the general shape, which stands after the repair: `dimensions` is derived while `named_followups`, `requirements` and the rest are carried, so **any** hand edit to a carried section can silently desynchronise a derived count *and* drift the file's format, with nothing checking either between regenerations — and the next regenerating lane then inherits an unreviewably large diff, as this one did. Discharge: a check comparing each derived dimension against the section it counts (HYGIENE-14's repair of the bucket tally, one field over) plus a format assertion, or a stated decision that hand edits to this file are expected and the counts advisory between regenerations. |
| `CT47-WRITE-CAPTURE-UNEXERCISED` | **New** (`STABILITY-REEVAL` §1). The `write` capture mode parses and has no lowering; its refusal moved from the `Q500` catch-all to the judgment's `WriteCaptureUnimplemented` arm in wave 29, and no fixture in this corpus drives that arm. Discharge: a fixture that produces the shape, or a statement that the keyword is withdrawn. |
| `CT24-CLEANUP-TASK-COMPOSITION` | Unchanged, lane 3's. CT24 still needs a cleanup/task composition in which an executed scope-exit destroy meets a live loan. A lane taking it inherits lane 3's §4.1 measurement **and** this lane's §6.1 finding that the regeneration must pass the committed gate file relatively. |
| `T00.7` | Unchanged and open for both new cases: no theorem is bound to either CT fixture. Proof stays 3 of 45. |

## 10. Integrator carry list

Files this lane owns and changed:

* `docs/reevaluation/ct_manifest.json` — CT44 and CT47 mapped; the note's
  wave-32 paragraph. No route-vocabulary change.
* `compiler/tests/ct_corpus.rs` — the pinned tables of §7 (including the
  `EVIDENCE_FILES` array length 18 → 19) and one `execution_profile` arm.
* `compiler/tests/fixtures/ct/CT44/` and `compiler/tests/fixtures/ct/CT47/` —
  new: two cases, eight controls, two `expected.json`.
* `scripts/ct_stage_summary.py` — this lane's record added to `EVIDENCE_FILES`.
* `scripts/tests/test_ct_stage_summary.py` — the three tree-fact pins of §7.
* `docs/reviews/r0_3/acceptance.json` — regenerated (`--write`), plus
  `derived_from`, the two hand-kept halves, the wave-32 paragraph and the two
  refreshed current-state sentences.
* `docs/reviews/r0_3/acceptance_matrix.json` — **regenerated by this lane**,
  which earlier lanes left to the integrator. The reason is §6: mapping CT47
  moves a derived field the standing check compares, so leaving it would put
  this card's own gate red. Regenerated with `--gate
  docs/reports/wave31-close/gate.json`, and §6.1 records the measured scope —
  `validation` byte-identical, every hand-maintained section unchanged except
  `named_followups`, to which this lane **appends** its two residuals (§9)
  through that section's own documented mechanism, editing no existing row.
* `docs/reports/ct-wire-lane-4/` — this report and its stage-evidence record.
* `docs/development/increments/wave-32.md` — **new file**, the wave-32
  increment paragraph; `docs/development/increments/README.md`'s current-record
  pointer and wave table updated per that file's own instruction.

**Not** changed, and left for the integrator: `AGENTS.md`'s headline numbers
(CT row `43/50 → 45/50` mapped, sanitized-native `7 of 43 → 7 of 45`, proof
`3 of 43 → 3 of 45`, gate numbers) and `README.md`.

## 11. Gate

Run in this worktree on `main` at `1c7d8e45` plus this card, with
`$HOME/.elan/bin` on `PATH` for the Lean row.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | ok — `third_party/libevent` at `d82464a2`, before the workspace run |
| `cargo test --workspace --no-fail-fast` | see §11.1 |
| `cargo fmt --all -- --check` | see §11.1 |
| `cargo clippy --all-targets -- -D warnings` | see §11.1 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | see §11.1 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | see §11.1 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | see §11.1 |
| `python3 -m unittest discover -s scripts/tests` | see §11.1 |
| `python3 scripts/ct_stage_summary.py . --check` | see §11.1 |
| `cargo test -p qcore-compiler --test ct_corpus` | see §11.1 |
