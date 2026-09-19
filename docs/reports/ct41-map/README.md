# Card `CT41-MAP` — reduced onto the landed CT41 mapping

**CT41 was mapped twice, independently, from the same base.** The mapping that
landed is card `CT41-PUBLICATION-MAPPING` (merge `2910c1e0`), and this report is
what remains of the other one after everything duplicating it was dropped. It is
**not** a second mapping, moves **no** coverage count, and adds **no**
`ct_stage_evidence.json` — CT41's row, its retirement and its stage evidence all
belong to `docs/reports/ct41-publication-mapping/`.

What it carries: two controls the landed fixture does not have, one
reconciliation of the two attribution grounds, and seven residuals re-verified
against main rather than inherited from the pre-merge measurement.

## 1. The two mappings, and the agreement that matters

Both cards were built from `39376a5f`. They reach the **same row** — route
`atomic_publication`, artifact `compiler/tests/fixtures/ct/CT41/case.c`, `parse`
and `atomic_publication_check` executed under a Clang tool gate, `proof`
`unassessed`, `native_sanitized` `not_applicable`, coverage **50/50** with proof
**5/50** and gated sanitized-native **10/50** — and they agree on the fact that
made it reachable at all: the wave-35 obstruction was structural **on the legacy
resource projection**, whose qualifier-invariance card `CTWIN-PUBLISH` §5
*decided* is correct, and a judgment that models synchronization is a different
judgment. Neither reads a verdict of `analyze_unrestricted_c11`.

More importantly, **both measured that the wave-9 attribution rule's clause (b)
cannot be met literally for this case**, separately and before deciding what to
do about it. That agreement is the substance; the rest is what each did next
(§3).

The landed mapping is the better one, and the reduction is not grudging. Three
of its properties this card's version did not have:

* **The qualifier refusal is attributed only after the full bounded shape and
  access inventory are checked**, never from the flag declaration alone. The
  unlanded version refused at the flag's type spelling before reading either
  entry body — so it would have said "this flag establishes no synchronization"
  about a `volatile` object in a program that is not a publication at all. The
  landed ordering makes the sentence true of the program it is said about.
* **`canonical_memory_orders` now requires evaluated integer constants** for the
  four orders (`relaxed` 0, `acquire` 2, `release` 3, `seq_cst` 5), because
  `AtomicExpr` executes Clang's numeric builtin order and not the spelling of an
  enum member. A translation unit defining `memory_order_release = 0` would have
  been read as a release store by the route this card measured against. That is
  a soundness fix this card did not make and would not have caught.
* **The controls are pinned as derivations of the case**, not merely as files:
  the runner asserts `case.c` with `"volatile "` removed *is* `plain.c`, byte for
  byte, and that the accepting control is the plain one with exactly the flag
  protocol substituted. The anti-manufacture property is thereby checked rather
  than argued.

## 2. What this card adds

Two controls, each **measured against the landed route** — whose qualifier arms
and memory-order reader both differ from the route the original measurement was
taken on — and each derived from the accepting control by a single declaration
substitution, which the runner now asserts in the landed file's own style.

| File | Measured on the landed route | What it bounds |
|---|---|---|
| `qualified_atomic.c` — `static volatile _Atomic int done` | `Source(UnsupportedAtomicFlag)` | The flag-type arm **neither qualifier refusal reaches**. A `volatile _Atomic int` *is* a synchronization primitive, so reporting it under the volatile arm would state something false of it; and this route models exactly `_Atomic(int)`, so admitting it would widen the shape. It refuses as the fail-closed third answer — which is what bounds the volatile arm to a flag that is **not** atomic. |
| `volatile_atomic_ops.c` — `release_acquire.c` with `_Atomic` written `volatile` | **not valid C11**: Clang refuses with `address argument to atomic operation must be a pointer to _Atomic type` | The **language boundary**: the one-token edit toward the admitted side is not a program, so no admitted one-token neighbour of `case.c` exists. This is the measurement behind the landed card's decision (§3) rather than an attribution. |

The landed control set is `{plain, release_acquire, relaxed_store,
relaxed_load}` — a refusing twin, an accepting twin, and the two order arms.
Neither addition duplicates one: the first reaches the `_` fallthrough of the
landed `match flag_type`, which no other fixture exercises and which the landed
change made an *exact* match (`"volatile int" => …`), so it is a live regression
pin; the second is not C at all.

`volatile_atomic_ops.c` needed one structural change to the runner. The control
loop's first act is
`validate_unrestricted_c11(&unit).expect("CT41 artifact must be valid C11")` —
a good property, deliberately kept — so a control that is *not* valid C11 cannot
ride it. It gets its own `language_boundary` block in `expected.json`, checked
separately, asserting both that C refuses it and that it refuses **for the stated
reason**.

### Why the second control matters more after the merge, not less

The landed card supersedes the control clause. The obvious objection to a
supersession is "you could have found a control that satisfies it and did not
look hard enough". `volatile_atomic_ops.c` answers that objection **with a
compiler error rather than an argument**: the clause cannot be satisfied at token
level because C11 does not admit an atomic operation on a volatile non-atomic
object, so the one-token neighbour the clause asks for does not exist to be
found. It is shipped as a fixture so a reader can check that without reproducing
the measurement — and it is recorded as a language boundary and **never** as
CT41's attribution, a parser rejection being exactly the "the projection cannot
express this at all" answer the clause excludes.

## 3. Reconciling the two grounds

Both cards hit the same wall: clause (b) of the wave-9 attribution rule asks for
"a control that removes **exactly** that construct and moves the verdict", and
for CT41 no such control exists — measured, in both directions, by both cards.

**The landed ground: supersede the clause, for this case, out loud.**
`docs/agents/evidence.md` now carries a CT41 section stating that protocol
replacement is the attribution — volatile and plain both refuse, a matching
release/acquire control checks, relaxed arms refuse, and removing `volatile`
must never confer ordering — and that this "explicitly supersedes the historical
removal-must-change-verdict criterion **for CT41 only**".

**The unlanded ground: read the clause at the mechanism level.** "`volatile` used
as a completion flag" names a *mechanism*, not a token; the isolating control
removes that mechanism and moves the verdict across the classes; clause (b) is
satisfied as written.

**Which the project should carry: the superseding form.** Plainly, it is the
more honest of the two, and the reasons are not close.

1. **It is visible.** A supersession is a declaration in a routed constraint
   page with a case name attached. The mechanism-level reading is a
   *reinterpretation* of an existing rule's word "exactly", recorded in a report
   nobody re-reads when the rule is next applied. Two readings of one clause,
   with no declaration distinguishing them, is how a rule quietly stops
   constraining anything.
2. **It is bounded.** "For CT41 only" cannot be inherited by accident. A
   mechanism-level reading is general by construction: once "exactly that
   construct" may mean "the mechanism that construct implements", any
   multi-site control can be called minimal, and the next card need not say so
   or even notice it is relying on it.
3. **It fails in the safe direction.** If the supersession is wrong, one case's
   attribution is wrong and its declaration says where to look. If the
   mechanism-level reading is wrong, every future application of clause (b) is
   weaker and nothing records when that started.

The mechanism-level reading is not *unreasonable* — a scenario that names a
mechanism plausibly names a mechanism — and this card's own measurement is what
makes the supersession defensible rather than convenient. But a rule should be
changed where it is written, not re-read where it is applied.

**For a future case of the same shape**, the two grounds differ concretely. Under
the landed form, the next case declares its **own** supersession and does not
inherit CT41's; the reviewer sees a new declaration and asks whether it is
earned. Under the mechanism-level reading, the next case simply asserts its
control is minimal and no declaration appears anywhere. What is **undecided** is
the general rule — whether clause (b) is read at token level with per-case
supersessions, or at construct level generally — and that decision belongs in the
constraint page rather than in either report. Registered as
`CT41-ATTRIBUTION-CLAUSE-SUPERSEDED-FOR-ONE-CASE`.

## 4. Residuals: what survived re-verification

Seven registered, `named_followups` **262 → 269**. Each was re-measured against
main, not carried across the rebase.

| Id | Re-verification |
|---|---|
| `CT41-ROUTE-HAS-NO-PRODUCTION-CALLER` | **Holds.** Re-measured on the landed tree: `check_atomic_publication_source` is called from the wave-41 focused test and from `compiler/tests/support/ct_atomic_publication.rs`, and from nothing else. No `quirin` invocation reports this refusal — a weaker position than `legacy_c`, whose analyzer `quirin-audit` really drives. |
| `CT41-PROOF-NOT-BOUND-TO-THE-CASE-ARTIFACT` | **Changed by the merge, and the change is the point.** See below. |
| `CT41-ATTRIBUTION-CLAUSE-SUPERSEDED-FOR-ONE-CASE` | §3. Re-worded from a draft entry about this card's own clause reading into the open question about the general rule. |
| `CT41-BOUNDS-ARE-CALLER-DECLARED-PREMISES` | **Holds.** The landed runner asserts both bounds on the certificate; pinning a premise is not discharging it. Re-verified that the case's refusal does not depend on them, so the exposure is entirely on the admitted control. |
| `CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING` | **Holds.** `typed_source.rs:522` still refuses `volatile \|\| _Atomic \|\| restrict` in one branch, untouched by the merge. This card's `qualified_atomic.c` is what measures that the atomic-publication route *does* draw the distinction. |
| `CT41-VOLATILE-COMPLEMENT-UNMEASURED` | **Holds, and sharpened**: since the landed route attributes the qualifier refusal only after validating the publication shape, it now says nothing at all about a volatile object that is not a completion flag. |
| `CT-UNMAPPED-LABEL-MACHINERY-HAS-NO-LIVE-INSTANCE` | **Holds as a finding; the fix is main's.** §5. |

**Two entries were dropped**, because their subject was this card's row rather
than the landed one:

* `CT41-STAGE-DEPTH-RANK-IS-A-JUDGMENT-CALL` — it recorded that
  `atomic_publication_check` was ranked at the check tier (3) while the
  comparable C-artifact stage `legacy_c_twin` sits at 4. **Main ranks it 4**, so
  the inconsistency it named does not exist. Dropped rather than re-worded.
* `CT41-ISOLATING-CONTROL-REPLACES-A-MECHANISM` — subsumed by
  `CT41-ATTRIBUTION-CLAUSE-SUPERSEDED-FOR-ONE-CASE`, which states the same
  measurement as an open question about the rule instead of as a defence of one
  card's reading.

### The proof residual changed, and it is now nearly dischargeable

Before the merge the honest statement was that the audited
`QCore.AtomicPublication` theorems were reached by the **admitted control** and
not by the case. The landed mapping added
**`non_atomic_completion_has_no_payload_path`**, audited in
`AuditAtomicPublication.lean`, proving that a program whose `flagKind` is
`.plain` or `.volatile` has **no** payload happens-before path — a theorem about
CT41's own **case class**.

So the gap is no longer a missing theorem; it is the **binding**. Nothing takes
`case.c`, derives the Lean `Program` it corresponds to and checks that theorem
against it the way `ct01_proof_leg.rs` binds CT01 through the F02 digest
relation. The Rust extractor's `VolatileFlagHasNoSynchronization` and the Lean
`flagKind = .volatile` are related **by review**, which is precisely constraint
28's Lean-model-to-Rust correspondence premise. CT41's `proof` stage therefore
correctly reads `unassessed` on main and the proof numerator correctly stays
**5 of 50**. The practical consequence is worth stating: a CT41 proof leg is now
a realistic next card rather than a research problem, because the theorem is
already written and already audited.

## 5. The label machinery, and one hazard worth keeping

A full denominator left the unmapped-bucket label machinery with **zero live
instances on disk**. **Main already repaired this** — `unmap_ct41()` in
`scripts/tests/test_ct_stage_summary.py`, called at eight sites, plus a direct
`assertEqual(summary.unmapped_labels(ROOT), {})`. This card reached the same
repair independently on its own branch and **dropped it as duplicate**; every one
of its ten Python edits is gone.

What is kept is the finding, and one measurement main's version does not record.
On a tree where the mapping card's retirement of CT41 was the only one,
`test_retiring_a_still_unmapped_case_fails_closed` could pass through the
**wrong rule** — "two evidence records retire this case" instead of the
live-label rule it names — because the card's own retirement supplied the second.
Main's `unmap_ct41()` withdraws that retirement and so reaches the right rule;
verified here by reading `ct_stage_summary._lane_row_retirements`, whose
unmapped check precedes its staleness check. **The hazard is generic**: a test
that reaches its refusal is not thereby testing its own rule, and a suite cannot
report that about itself.

The standing consequence: the mechanism is now verified against scratch trees
only, never against the repository disagreeing with itself. A regression
returning an empty map is invisible to the committed-tree arm except through
that one direct assertion, which is why the residual asks for the tally-sum and
`ct_unmapped` zero arms beside it.

## 6. Gate

[`gate.json`](gate.json), with [`clang-version.txt`](clang-version.txt) recorded
by running `clang --version` **in this worktree**; it is not borrowed.

## 7. Integrator reconciliation list

1. **This card claims no coverage.** CT41's row, its stage evidence and its
   bucket retirement are `ct41-publication-mapping`'s. This report adds no
   `ct_stage_evidence.json` and touches no coverage count; a second retirement
   of CT41 would be refused by the derivation ("two evidence records retire this
   case's bucket"), which is the mechanism working.
2. **The two added controls are the deliverable.** `qualified_atomic.c` pins the
   `_` fallthrough of the landed `match flag_type`, which no other fixture
   reaches; `volatile_atomic_ops.c` is the language boundary that makes the
   landed card's supersession checkable rather than assertable. Keep the
   `language_boundary` block separate from `controls`: it exists because that
   file is deliberately not valid C11, and merging it would force the runner to
   drop the C-admission assertion it applies to every other artifact.
3. **§3 asks for one decision that is not this card's to take**: whether clause
   (b) of the wave-9 attribution rule is read at token level with per-case
   supersessions declared (the landed form, which this card endorses) or at
   construct level generally. It belongs in `docs/agents/evidence.md` beside the
   landed CT41 section, not in a report.
4. **`CT41-PROOF-NOT-BOUND-TO-THE-CASE-ARTIFACT` is the entry to read next.**
   The theorem CT41's proof leg needs now exists and is audited; only the
   artifact binding is missing.
5. **Follow-ups 262 → 269**, seven registered and two dropped by name (§4). The
   `docs/STATUS.md` count moves with it. `CT41-TYPED-SOURCE-UNDISCRIMINATING`
   was already retired on main and is **not** retired again here.
