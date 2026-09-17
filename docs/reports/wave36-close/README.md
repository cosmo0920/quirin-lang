# WAVE-36-CLOSE — seven cards, two arbiters, four amendments found unapplied — 2026-09-18

Base `fa48211a`, the `WRITE-LEND-TASKS-EXERCISE` merge. Evidence:
[`gate.json`](gate.json), [`reconciliation.json`](reconciliation.json),
[`scripts/`](scripts).

## Headline

Wave 36 merged **seven** cards and ran **two arbiters**. All **89**
integrator-addressed items were given a verdict **measured against the tree** —
the target file grepped for the proposed text or its effect, never inferred from
a report's claim that it had landed. That distinction found **five things**:

1. **`W36-CORPUS-CONTRACTS-3`'s constraint 7 amendment was unapplied.** At the
   close's base, `grep -c "Card \`W36-CORPUS-CONTRACTS-3\` (wave 36) moved the
   number"` over `AGENTS.md` returned **0**. Landed here verbatim.
2. **Its constraint 8 amendment was unapplied too** — the same measurement, the
   same card, the same merge. Landed here verbatim.
3. **`STABILITY-REEVAL` #2's constraint 5c amendment was unapplied**, and 5c was
   therefore still asserting something the card had measured **false about the
   tree**: that the do-while rule "does not generalise to `while (1)` or
   `for (;;)`", when a pre-existing route already removes the exit edge for
   `for (;;)`. Landed, with two corrections recorded rather than made silently
   (§2.3).
4. **Constraint 14 had no wave-36 entry at all** — not the "does not engage"
   list, not `WRITE-LEND-TASKS-EXERCISE`'s `+0` censused row. **No card owed
   the whole entry**, which is why nobody's item named it: two cards asked to be
   listed and the other five were never anyone's task. Written here.
5. **`W36-CORPUS-CONTRACTS-3`'s constraint 16 text could not land as written.**
   It says the host row "joins this list as its **sixth** row"; the row has
   *been* the sixth since the wave-35 close. Landing it verbatim would have put
   a false statement into a standing constraint and given constraint 16 the same
   member twice. Split: the wording superseded, the **measurement** landed (§6).

The one code change this close makes is `STABILITY-REEVAL` #2's own request —
the `--check` content arm — and running this close's record through it found
**five more** problems, in this close's own work (§4).

**Nothing is promoted.** No corpus count, unit status or function status moves
at this close; the contract corpus stands at **82**, CT coverage at **46/50**
with sanitized-native **10** and proof **3**, and full language conformance
stays **false**.

## 1. Verdict distribution

| Verdict | Count |
|---|---|
| `applied` | 50 |
| `noted` | 31 |
| `superseded` | 4 |
| `deferred` | 4 |
| `declined` | 0 |
| **Total** | **89** |

```
python3 scripts/reconciliation_sweep.py . --since 60958bf0 \
    --check docs/reports/wave36-close/reconciliation.json
```

exits **0**, and now says two things rather than one:

> every swept item carries a recorded verdict. This says nothing about whether
> any item was applied - read the notes.
> 112 content claims (98 present, 14 absent) verified at their named locations
> across 9 files; 46 of 50 'applied' verdicts carry checkable text, 4 declared
> unverifiable with a ground.

**Zero `declined`.** That is worth stating rather than passing over: a close
with no refusals is usually a close that did not look. This one refused *half*
of one item (§6) and recorded it `superseded` rather than `declined`, because
what overrode it was another card's earlier landing rather than this close's
judgement. If any item here deserved a flat "no", it is that one, and it is
recorded with both texts so a reader can disagree.

### 1.1 Every non-`applied` verdict that carries a decision

**Superseded (4)** — each by something that happened, not by an opinion.

* `card-scope-guard#12` — "Count: 121 → 128". Right at its merge, and itself
  landed late by the integrator's own correction commit `b5cfc2cb`. Five merges
  later the figure was 171 and this close takes it to 187. Superseded by the
  wave's own chain, which `AGENTS.md` now carries merge by merge so no single
  figure has to stand for the wave.
* `c-gateway-indirect-census-2#3` — the matrix regeneration as a byte-identical
  fixed point with `named_followups` carried at 121. Correct for a card that
  adds no audit, no contract module and no provider package. Superseded because
  the count has since moved 121 → 187 and this close regenerates with **its own**
  gate, restoring an `integrator_supplied` provenance naming wave 36.
* `w36-corpus-contracts-3#3` — the baseline pointer. Applied at that card's
  merge and superseded **within the same wave** by the second arbiter, whose
  fresh BEFORE reproduced this card's `diagnostics.json` and
  `chunk-contracts.json` byte for byte and which moved zero leaves, zero
  functions and zero units. That is the chain rule working, not a loss.
* `w36-corpus-contracts-3#15` — constraint 16's proposed wording. §6.

**Deferred (4)** — each because the decision belongs to a card or to the user.

* `c-gateway-indirect-census-2#7` — `INDIRECT-CENSUS-FEATURE-UNBUILT`. The card
  says in terms that it is a decision and not a task, and declines to take it.
  **Ground:** a close that added a CI job would be chartering wave 37's work by
  the back door, and a close that declared the instrumentation disposable would
  be discarding a measured artifact on no evidence. Both arms are written down
  and the name is registered, so nothing expires.
* `c-gateway-indirect-census-2#13` — "do not cut a wave-37 arbiter on this
  family". **Ground:** chartering is not a close's decision. Carried with its
  reasoning intact (§10), because the reasoning is the valuable part: the
  family's leaf mass and its function reach are nearly disjoint.
* `write-lend-tasks-exercise#8` — `TASK-ROUTE-TSAN-SEGV`'s priority. **Ground:**
  the card declined to set it and said the choice is the user's; promoting a
  residual to card work is work allocation. Recorded as a deferred decision, and
  §9.2 states the route-wide fact behind it plainly.
* `write-lend-tasks-exercise#9` — CT07's remap. **Ground:** lane work, and no
  lane of wave 36 owns it. `CT07-TWO-THREAD-REMAP` is registered and the card
  explains why it does not remap the row itself; the close adds no information
  by inventing a disposition.

## 2. The three unapplied amendments

### 2.1 Constraint 7 and constraint 8

Both proposed by `W36-CORPUS-CONTRACTS-3` §10 items 5 and 6, both in the words
"please add, in these words", and **neither was in `AGENTS.md` at this close's
base**. Landed verbatim. Constraint 8 additionally gains the evidence pointer
(`w36-corpus-contracts-3` §5.2) that the constraint's own convention wants.

This is the third consecutive wave in which a card's *prose* items were applied
less completely than its *numbers*. `W36-CORPUS-CONTRACTS-3` opened its own
integrator section with exactly that warning — "please work these as items, not
as headline numbers — the wave-32 `W32-RECONCILE-SWEEP` finding is that the
numbers get applied and the prose does not" — and then had two of its prose
items dropped at its own merge. The warning was right and did not help, which is
the argument for the mechanism rather than the advice.

### 2.2 The wave-35 record is corrected beside itself, not edited

`STABILITY-REEVAL` #2 item 1 asked that the correction of
`docs/reports/wave35-close/reconciliation.json`'s verdict note be recorded
**beside** that record rather than applied to it. Honoured: this close does not
touch `docs/reports/wave35-close/` at all (constraint 21). The correction is
that wave 35 recorded a verdict `applied` whose asserted text was not at the
location it named — the finding
`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, whose repair is §4.

The ledger row itself was corrected in `STABILITY-REEVAL` #2's own merge and is
verified here rather than assumed: `AGENTS.md` reads "exercised on **40** corpus
functions" and the superseded **44** and **21** figures occur **zero** times.

### 2.3 Constraint 5c, and two corrections recorded rather than made silently

The card's proposed text carried two line numbers. **Both had moved**: the tree
has `condition.is_empty()` at `legacy_c.rs:3312` (not 3277) and the successor
drop at `:5161` (not 5056). The mechanism was verified in source before landing:

* an empty `for` condition sets `condition_always_true`
  (`legacy_c.rs:3312`, the `Statement::For` construction);
* the header's successors then become `vec![body_entry]`, `continuation`
  dropped (`legacy_c.rs:5161`);
* `break_targets.push(continuation)` still precedes the body, so `break` reaches
  the continuation exactly as the card says.

Landing the card's stale numbers would have put a **false statement** into a
standing constraint, which is the failure this whole process exists to prevent —
so the numbers were re-measured and the correction is recorded here rather than
absorbed. The second correction is one word: the card's own deletion of
`for (;;)` left "modelling **those**" with no plural referent, so "those" →
"that". It softens nothing and it is recorded because an unrecorded
grammatical follow-through is how a softening gets its cover.

`FOREVER-FOR-EXIT-EDGE-UNOWNED` **stays open**: the route is still unowned, still
carries no Lean statement and no pin. What landed is the description, not the
discharge.

## 3. The 44 residual names, classified one by one

`--residuals` returned **44** names the tracker did not carry. Each was
classified individually, from its own source text.

| Class | Count |
|---|---|
| **Registered** as open work | 18 |
| **Card names**, not residuals | 15 |
| **Discharged**, with the discharging card named | 8 |
| **Rules or prospective names**, not open work | 2 |
| **A deferred decision**, not a followup | 1 |
| **Total** | **44** |

**Registered (18)** — all nine of `W36-CORPUS-CONTRACTS-3` §9 and all nine of
`C-GATEWAY-INDIRECT-CENSUS-2` §9. **Both cards left them to the close
deliberately and said so**, which is the correct behaviour and the reason the
sweep exists. Each `why` is drawn from its own report's §9 text and carries that
report's own DISCHARGE clause — including the two the census card marked **"NOT
a discharge"** (`RECORD-BODY-TOTAL-OR-REJECT`: citing the 2,713 leaves;
`LOAD-SENSITIVE-ROW-UNNAMED`: re-running until green), which is the half most
easily lost in a summary.

**Card names (15)** — `C-GATEWAY-INDIRECT-CENSUS-2`, `CT-PROOF-LEG-2`,
`CT-WIRE-LANE-2`, `CT-WIRE-LANE-3`, `CT01-PROOF-LEG`, `CT03-PROOF-ALPHABET`,
`CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`, `REGION-CREDIT-JOINS`,
`REGION-JOINS-DUTIES`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES`,
`CT46-SENDSYNC` and `EVENT-V1-WINDOWS-AFUNIX` (directories under other names),
and `DOCS-COMPACT-7`, a **future** card due around wave 37. The wave-35 close
classified the same shapes the same way; this is the tool's known and accepted
false-positive class.

**Discharged (8)** — `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` by
`JOINED-PROVENANCE-CYCLES`; `CT22-CT-MAPPING` by `CT22-WIRE`;
`W4-RECSUM-CONSTRAINT-4` and `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE` by
`W32-RECONCILE-SWEEP`; `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` by
`SWEEP-BLIND-SPOTS`; `WAVE-34-CLOSE-UNSWEPT` and
`W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE` by the **wave-35 close** (the latter
verified in history: registered by `SWEEP-BLIND-SPOTS` at `7d2f9fe2`, removed at
`584b1bf6` by that close's own named script); and
`JOINED-PROVENANCE-FACTS-NAME-NOTHING` by **this wave's**
`JOINED-PROVENANCE-CALL-FRAMES`.

**Rules or prospective names (2)** — `DOCS-COMPACT-5-EXAMPLES-HOST-GATE`, marked
*standing* and deliberately not reworded by `W32-RECONCILE-SWEEP`; and
`W4-RECSUM-ALREADY-LANDED`, which records a stale brief its own card then
verified.

**A deferred decision (1)** — `VERIFIER-HOST-ROW-DISPOSITION`, carried in
`deferred_decisions` and resolved at wave 35. It must not be registered as a
followup as well.

### 3.1 One name went quiet for the wrong reason

`--residuals` fell **44 → 25** after registration. **Nineteen** names dropped,
not eighteen: `C-GATEWAY-INDIRECT-CENSUS-2` also went quiet, because a
registered entry's `source` path contains that card's own name and the scan is
case-insensitive. The classification is right — it *is* a card name — but it
went quiet for the wrong reason, and a future close must not read that drop as a
decision anyone took.

### 3.2 And two more appeared, which is what a retirement costs

After the three retirements (§8), `--residuals` rose **25 → 27**:
`MAIN-GUARD-UNACTIVATED` and `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED` now
reach no tracker, because they are discharged and still written in the history
that discharged them. **No third-party name was orphaned** — that is the check
`STABILITY-REEVAL` #2 item 5 asked for, run *after* the removals and compared
with the run before them, and it is now a written close rule. The two join the
"discharged, with the discharging card named" class rather than becoming open
work, and the rule says to expect them.

## 4. The `--check` content arm — and what it found in this close's own record

`STABILITY-REEVAL` #2 item 2 asked for an arm that verifies an `applied`
verdict's quoted text at the location the verdict names, **shown red by mutation
before it is believed**. Adopted, and built differently from the proposal.

**Why differently.** The item proposed reading the prose `where` field and the
`note`'s quoted figures. That would have made the check's own reliability a
*parsing* question — a close could fail the gate for phrasing its `where`
unusually, and a check that produces false reds is a check that gets switched
off. Instead an entry carries a **structured** `verify` list:

```json
{"verdict": "applied",
 "verify": [{"file": "AGENTS.md", "contains": "…"},
            {"file": "AGENTS.md", "absent": "…"}]}
```

Each claim is opened and read at check time. **Whitespace is normalised on both
sides** — `AGENTS.md` wraps its amendments at 76 columns, and a literal
substring search would call a landed amendment missing purely because a newline
fell inside the quotation. **Nothing else is normalised**, so a *softened*
amendment is still a changed claim, which is invariant 1's hazard and is pinned
by `test_one_changed_word_is_a_changed_claim` (`no` → `some`). The `absent` arm
is the other half: "this stale sentence now occurs zero times" is exactly as
checkable as its replacement's presence, and a close that only ever asserts
presence cannot catch a duplicated amendment.

From wave 36 an `applied` verdict must carry `verify` **or** `unverifiable` — a
written ground for why the claim has no quotable text (a regenerated artifact, a
command that ran, a deletion with no successor sentence). The escape hatch is
counted and printed, so discharging evidence by declaring it uncheckable is said
out loud. Carrying both is refused; `unverifiable` on a non-`applied` verdict is
refused.

**The demand is gated by wave number and the verification is not.** An
unconditional demand would have left exactly two moves against the frozen
wave-32..35 records — edit an archive (constraint 21) or weaken the live test
that pins one of them green — and both are the failure this repository names.
History keeps the standard it was written to; no archive is touched.

### 4.1 Shown red by mutation, twice over

**Nineteen new tests** (45 → 64), each red by mutating the fixture in one place:
a sentence absent, one word softened, a file that does not exist, a stale
sentence still standing, a bare `applied`, a `where` with no `verify` (the
wave-35 shape itself), an empty ground, both fields at once, a malformed claim,
a path escaping the root, a second claim in a list.

**And the implementation was mutated too**, because tests that only ever see the
green case are evidence about nothing. Eight mutations, all caught:

| Mutation | Result |
|---|---|
| `contains`-always-passes | RED, 3 tests |
| `absent`-never-refuses | RED, 1 test |
| demand switched off | RED, 3 tests |
| missing file is a pass | RED, 1 test |
| normalisation eats the words | RED, 5 tests |
| only the first claim is read | RED, 2 tests |
| both fields accepted | RED, 1 test |
| path escape allowed | RED, 1 test |

Mutations that survived: **none**. Reproduce with
[`scripts/mutate_check_arm.py`](scripts/mutate_check_arm.py).

### 4.2 What it found here — five things, all in this close's own work

This is the discharge: the close ran its own record through the arm.

1. **A false `absent` claim.** This close asserted that constraint 15h no longer
   carries "no operation admits a lease in a worker body". It **does** — as a
   quotation naming what `WRITE-LEND-TASKS-EXERCISE`'s appendix supersedes,
   which is the correct "publish beside, do not silently delete" discipline. The
   claim was wrong, not the tree. Rewritten to assert the appendix's own
   supersession sentence.
2. **A wrong evidence-pointer quotation** — constraint 10b's list reads
   ``` `joined-provenance-cycles`, `joined-provenance-call-frames`. ```, not the
   pair this close guessed.
3. **A capitalisation** — `gate.json` says "Wave-36 close", the claim said
   "wave-36 close". The arm is deliberately case-sensitive.
4. **A missing claim** — `w36-corpus-contracts-3#13` was recorded `applied` with
   no evidence at all. That is precisely the wave-35 shape, made by this close,
   caught by this close's own arm.
5. **The close's own report needed a `no_section` reason** — it is swept as a
   card because it is a `docs/reports/<card>/README.md` in the range, and it has
   no integrator section and must not: a close that addressed items to itself
   would be the reconciliation defect in miniature.

The fixed record verifies **112 content claims (98 present, 14 absent) across 9
files**; 46 of 50 `applied` verdicts carry checkable text and 4 are declared
unverifiable with a ground (the guard activation, its installer command, the
guard-log reading, and the absence-of-a-pinned-category-list measurement — all
four things no committed file asserts).

## 5. The 26 seam commits, accounted by eye

| Class | Count | Commits |
|---|---|---|
| Cards' own code commits | 22 | `write-lend-tasks-exercise` 5, `mutex-share-cross-emission` 6, `joined-provenance-call-frames` 4, `w36-corpus-contracts-3` 3, `card-scope-guard` 2, `stability-reeval-2` 1, `c-gateway-indirect-census-2` 1 |
| Integrator corrections | 3 | `b5cfc2cb` (the stated follow-on count, 121 → 128), `e4619e37` and `2200a04e` (re-separating three wave-35 increment entries a union merge had fused, and rebuilding it when the first attempt inserted nine blank lines instead of three) |
| The wave's opening | 1 | `60958bf0` |
| **Total** | **26** | |

**Zero are uncarded work.** Every commit is attributable to a card, to the
integrator, or to the wave's opening. Cross-read against the guard log (§7):
every one sits under an authorised advance of `main` from the main checkout.

Two of the three integrator corrections change a committed document's content
with no card report, which is exactly what `SYNC-RULE-TOOLING-COMMIT-SCOPE`
asks about. Wave 35 saw four such commits; wave 36 has four again. That is a
**steady state**, not an anomaly, and steady states are what rules are for.

## 6. Constraint-text decisions

### 6.1 Constraint 7 — applied verbatim
### 6.2 Constraint 8 — applied verbatim, plus the evidence pointer

Both in §2.1.

### 6.3 Constraint 16 — the wording superseded, the measurement landed

The card proposed:

> `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
> … **joins this list as its sixth row**, measured by card
> `W36-CORPUS-CONTRACTS-3` (wave 36) …

The row **has been the sixth since the wave-35 close**, which landed it in card
`VERIFIER-HOST-CONTROL-BUDGET`'s own words. Landing this sentence verbatim would
have asserted something false and given the list the same member twice. The card
saw the alternative itself — *"If the integrator would rather treat `e3d3a2ce`'s
repair as incomplete than add a sixth row, that is a legitimate alternative
reading of the same measurement — but the measurement should land somewhere
either way"* — so this close takes **neither** extreme. It does not add a
duplicate row and it does not reopen `e3d3a2ce`'s repair, which 38/38 supports.

What landed, in the card's own numbers, is the measurement as a **second and
independent observation of the same row**: a one-second wall-clock bound over an
8 MiB pipe write and a process spawn against a 40 ms declared timeout, failing
once in-gate at load ≈40 on 16 cores and passing **12 of 12** in isolation at
load 23.7–27.6. Its value is that it is the row's **first measurement at the
middle of the load range** — wave 35 measured ≈100 and ≈6.5 — and the first
*after* the `poll(2)` repair. What it shows is that the repair **narrows the
failure window without closing it**.

And the list is now recorded **known incomplete** rather than assumed complete:
`LOAD-SENSITIVE-ROW-UNNAMED` names a seventh test,
`a_forced_override_never_weakens_an_execution`, with the same signature in the
same crate, measured red by a *different* card under two concurrent corpus
audits. It was **green** in this close's own gate, which is why it is named
rather than promoted: constraint 16's rows each carry a measured load figure and
one observation is not one.

### 6.4 Constraint 14 — the whole wave-36 entry, which no card owed

Two cards asked to be listed (`W36-CORPUS-CONTRACTS-3` item 7,
`MUTEX-SHARE-CROSS-EMISSION` item 5). The other five were nobody's item, which
is why the omission was invisible: **an entry no card owns is an entry no
reconciliation list mentions**. Six of the seven cards go to "do not engage";
`WRITE-LEND-TASKS-EXERCISE` does **not**, because it ran a census (514 fixtures,
zero occurrences of either new arity) and takes a `+0` row — the
`MUTEX-SHARE-CROSS` distinction applied rather than restated.

### 6.5 The `pkill` rule — landed in the card's words

`MUTEX-SHARE-CROSS-EMISSION` ran `pkill -f "cargo test --workspace"` and killed
a **sibling** wave-36 card's standing-gate run mid-flight. It disclosed the kill
in full, touched no repository state, and deliberately did not restart the other
card's job on the ground that guessing at another card's intent is worse than
reporting the fact. It proposed the rule rather than landing it, because
`CLAUDE.md` is a standing rule authority.

**Landed, in its words**: never `pkill`/`pgrep` on a pattern that names a tool
invocation, because **the process table is shared across worktrees exactly as
the stash stack is**. The card *had* the PID and still reached for the pattern,
which is the argument for a rule rather than care. The sibling is
`JOINED-PROVENANCE-CALL-FRAMES`; its gate was re-run and its report §8 carries a
completed measurement, so the damage is repaired as well as disclosed.

### 6.6 The push-token note

A gap found **in use**, not in review: the installer's message names merges
only, and `git push` of `main` needs the token too. The log carries
`refused-push-unauthorised` twice before it was carried on the push command.
Landed in `CLAUDE.md` beside the guard bullet.

### 6.7 A new close rule: a retirement is named, checked, and swept after

`STABILITY-REEVAL` #2 item 5's other half. Landed: confirm a name is **actually
a registered entry** before counting it retired, and re-home every name the
entry was the sole tracker for, checked by `--residuals` run *after* the removal
and compared with the run before it. The rule also says to expect the retired
names themselves in that later run (§3.2), so a future close does not mistake
them for a loss.

## 7. The guard log, read for the first time

`MAIN-GUARD-LOG-UNCONSUMED` records that the guard logs every authorised and
refused update of `main` and **nothing reads it**. This close read it — as a
file, not by running any git command against the main checkout.

| | Count |
|---|---|
| `authorised-main-update` and `authorised-push` | 21 |
| `refused-provenance` (the self-test's two arms) | 2 |
| `refused-unauthorised` | 1 |
| `refused-push-unauthorised` | 2 |

The self-test's two entries stand at `2026-09-17T19:21:00Z`, one refusing the
main checkout's own `pwd` marker and one refusing a synthetic
`agent-mainguard-selftest` worktree — both arms of the mechanism refusing on a
transaction that touched no reference. Every card merge of the wave appears as
an authorised advance from the main checkout.

**The residual stays open.** One close reading a log by hand is not the
mechanism it asks for; what is discharged is the *question* — "was this uncarded
commit made under authorisation, and from where" — not the automation. `--uncarded`
still cannot ask it.

## 8. Retirements — three, each named and each checked first

The brief's rule and `STABILITY-REEVAL` #2's: **check that a name is actually a
registered entry before counting it retired.** [`scripts/retire.py`](scripts/retire.py)
refuses to remove a name that is not present, rather than reporting a no-op as a
retirement.

1. **`CARD-SCOPE-SELF-MERGE`** — discharged. Its condition was "a mechanism that
   makes a delegated card unable to advance `main` — not an instruction telling
   it not to". The mechanism exists and the integrator activated it. The card
   offered retirement-on-activation as the integrator's choice and asked that it
   be a **deliberate** removal noted in the close; this is that removal and this
   is that note.
2. **`MAIN-GUARD-UNACTIVATED`** — discharged by the same fact. Its condition is
   "a wave close noting the command ran", which is §7 exactly.
3. **`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`** — discharged by §4.

**Two "retirements" in the wave's own cards were no-ops, and are recorded as
such** rather than counted:

* `JOINED-PROVENANCE-CALL-FRAMES` (from its own item 4) was **never a matrix
  entry** — measured at `913fe7b2^`, where `JOINED-PROVENANCE-FACTS-NAME-NOTHING`
  *was* registered and this one was not. One real removal, one no-op; the card
  was right to name both explicitly, and naming them is what made the
  distinction visible. The arithmetic checks: 159 at the parent, +3 −1 = 161.
* `WRITE-LEND-TASKS-EXERCISE` (from its own item 2) likewise lived only in
  constraint 15h's text. Its discharge is the appendix, and it moves no count.

`named_followups`: **171 → 189 → 190 → 187**. Registered +18 (the sweep's) and
+1 (this close's own finding), retired −3.

## 9. Deferred decisions

**Fifteen entries** were carried in across two files —
`docs/reports/wave34-close/deferred_decisions.json` (7) and
`docs/reports/wave35-close/reconciliation.json` (8) — over **eight distinct
names**, of which **six were still open**. All are restated in
`reconciliation.json`, and **two new ones** are raised by this wave, for **ten**
entries in this close's record.

Restated, not resolved: `CT10-REASON-ROW`, `MATRIX-UPDATED-STALE-AT-WAVE-27`,
`EXAMPLES-STYLE-SLICE-FANOUT`, `SYNC-RULE-TOOLING-COMMIT-SCOPE`,
`PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED`,
`DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK`. Recorded as already ended:
`VERIFIER-HOST-ROW-DISPOSITION` (resolved at wave 35) and the bogus `items`
(withdrawn at wave 35) — kept so the 15-entry carry is accounted rather than
silently reduced to six.

Three of the six gained **new evidence** this wave and the grounds say so:
`PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` (wave 36 added another platform
residual of the same family, `WRITE-LEND-EXERCISE-WINDOWS-RUN`, and that one
*is* registered, so the family is now *more* partially tracked);
`SYNC-RULE-TOOLING-COMMIT-SCOPE` (§5); and
`DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` (§11).

### 9.1 New: `INDIRECT-CENSUS-FEATURE-UNBUILT`

Raised by `C-GATEWAY-INDIRECT-CENSUS-2` §12.2. Both arms written down, the name
registered so the work is tracked either way.

### 9.2 New: `TASK-ROUTE-TSAN-SEGV`'s priority — and the fact behind it

Raised by `WRITE-LEND-TASKS-EXERCISE` §11 item 7, which declines to set the
priority and says the choice is the user's. The close carries it, and states the
fact plainly because it is larger than the decision: **the whole checked
concurrency route has no thread-sanitizer evidence at all, and no TSan row
exists anywhere in this repository.** Constraint 15o says the same from the
other direction — the mutex emission card's Lean-proved boundary is that the
creator performs no event while its child runs, so that backend witnesses a real
thread boundary and **nothing** about contention, a data race or the C memory
model. Two independent routes now record the same gap.

## 10. Carried to wave 37

* **`DOCS-COMPACT-7`** is due around wave 37; **`STABILITY-REEVAL` #3** after
  it. Firing #2's nominated grounds are carried so #3 need not re-derive them:
  the **corpus artifacts**, pinned by no test at all, and the **remaining Lean
  docstrings**, where a 3-for-3 hit rate in one module suggests the sample is
  not exhausted. `MUTEX-SHARE-CROSS-EMISSION` §9 item 8 adds a third, offered
  rather than claimed: **any** route whose native comparison is driven by the
  typed diagnostic sink is comparing *logged* lines, so a mutation that moves
  the operation but not its log is invisible to it — measured and repaired for
  the mutex route, unmeasured for the sequential, task, epoch, socket, event and
  HTTP routes, and no claim is made about them here.
  `JOINED-PROVENANCE-CALL-FRAMES` §9 item 8 offers a worked example of step 1
  applied *by the card that created the arm* (`CALL-FRAMES-ALLOWLIST-LOCAL-ONLY`).
* **Do not cut a wave-37 arbiter on the indirect-access family** without first
  running `BASE-TYPE-READER-UNMEASURED`'s census. The card's ground, carried
  intact: the family's leaf mass and its function reach are nearly disjoint,
  every route measured decides between 2 and 17 functions against a
  3,239-function population, and **78.5%** of the unresolved-member leaves carry
  no record tag at all because a *different* reader — one nobody has measured —
  never committed to the base's type. Constraint 5b should be re-read by any
  such card before it starts, for the reason 5b itself gives.
* **`JPC-OFF-CYCLE-THEOREM-ACYCLIC-ONLY`** and
  **`DO-WHILE-ACYCLICITY-SCRIPT-PREMISE-STALE`** are registered and **not
  repaired here**. Both repairs touch a card's published evidence — a Lean
  docstring and a frozen report's script — and the real fix for the first is a
  proof over `Reach` for a graph that may contain cycles, which is card work.
  The ledger is clean in both cases (`grep -c an_off_cycle_node_does_not_return
  AGENTS.md` = 0), so nothing false stands in a current rule authority; what is
  wrong is a docstring's reach, recorded beside it rather than edited into it
  (constraint 21).

## 11. The integrator's own record this wave, unsmoothed

Step 3 of the `STABILITY-REEVAL` method asks for semantic edits made without a
card. These are this close's, and each is in a commit message.

* **Three pushes were refused by the new guard** until `push` carried the token
  (`refused-push-unauthorised`, twice in the log plus one
  `refused-unauthorised`). The guard worked; the *instruction* was incomplete,
  which is now §6.6.
* **One gate run was red for a reason that had nothing to do with the tree.**
  The detached wrapper carried `trap '' TERM`; an ignored TERM disposition is
  **inherited by every child** and turns a Python signal-handling row red.
  Reproduced both ways and re-measured green. `measure/run-gate.sh` now carries
  a comment forbidding it, and the brief carried the warning that made it
  findable.
* **Three merge-resolution attempts aborted by their own assertions before
  writing** — a wrapped continuation line matching the head pattern, a
  never-registered retirement, an evidence pointer already extended. Each
  refusal was the script declining to write rather than a reviewer catching it
  afterwards, which is the shape this close also gave
  [`retire.py`](scripts/retire.py) and
  [`merged_gate_row.py`](scripts/merged_gate_row.py): both refuse rather than
  guess when the tree is not what the edit was written against.
* **Two gates were launched against a mid-merge tree and killed unread.** A gate
  measured on a tree that is halfway through a merge measures nothing, and the
  correct response was to discard rather than to interpret.
* **One deferred-count line was missed on the first merge after the rule was
  written** — the rule being new is not a defence, and the miss is the reason
  §9's carry is stated as *fifteen entries over eight names*, in both numbers,
  rather than as a single total.

To which this close adds, from §4.2: **five defects in its own reconciliation
record**, four of them evidence claims and one a missing claim entirely. They
were found by the mechanism this close landed, on its first use, against its own
work. That is the argument for the mechanism, and it is also the reason not to
read a green `--check` as a green close.

## 12. `main` moved during this close

At the close's start `main` was `fa48211a`, the brief's base. It is now
`477b733d` — *"Open wave 37: create the increment record and move the index
pointer before any card launches"*, the integrator's wave-37 opening, taken
ahead of this close exactly as wave 35's was. It is not wave-36 work.

One consequence to know at merge time: the increment index will already name
`wave-37.md`, so a **literal** reading of the sync rule would misfile this
close's paragraph. It is in `wave-36.md`, which is where the brief and
`477b733d`'s own commit message both say it belongs. No `wave-37.md` was
created by this close.

## 13. Gate

Measured at this close's own integration tip, so the one code change it makes is
**inside** the measurement rather than beside it.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0, `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | exit 0 — **4,611 passed / 0 failed / 8 ignored**, 233 result summaries, 229 executables, 22m31s |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `reevaluation_probe` | exit 0 |
| `reevaluation_followup` | exit 0 |
| `scripts/check_lean_audits.py` | exit 0 — **135 audits** |
| Python (`scripts/tests`) | exit 0 — **293 tests** |
| Provider | **122 carried**; constraint 18 not engaged |

Every exit was captured to its own file **with no pipe in the command**
(`GATE-EXIT-SWALLOWED-BY-PIPE`). Deltas against the wave-35 close's 4,580 / 247 /
132: workspace **+31**, wholly the wave's own cards, this close adding no cargo
test; Lean **+3**; Python **+19**, wholly this close's `test_reconciliation_sweep.py`
going 45 → 64.

**The Python run went red once, informatively.** After this close edited
`named_followups` by hand,
`CommittedMatrixIsSelfConsistent::test_each_derived_dimension_matches_the_section_it_counts`
failed with `171 != 187` — `D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire doing
exactly its job. The repair was to **regenerate** through `build_matrix.py`,
never to touch the derived count. The figure above is the re-run.

Constraint 16's rows, stated because that constraint requires a gate report to
say which it saw: the sixth row, `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
and `memory_growth_beyond_the_declared_limit_is_bounded` were **all green**, as
was the unnamed seventh (§6.3). `grep -c 'test result: FAILED'` over the log is
**0** across all 233 summaries, so no isolated re-verification was required and
none is claimed. This is a full-suite run on a machine carrying its own build —
**not** the quiet-host isolated number `HOST-ROW-STARVATION-DESCRIPTION` still
asks for, and not offered as one.

## 14. Reproduction

```sh
python3 scripts/reconciliation_sweep.py . --since 60958bf0
python3 scripts/reconciliation_sweep.py . --since 60958bf0 --residuals
python3 scripts/reconciliation_sweep.py . --since 60958bf0 --uncarded
python3 scripts/reconciliation_sweep.py . --since 60958bf0 \
    --check docs/reports/wave36-close/reconciliation.json

python3 docs/reports/wave36-close/scripts/register_followups.py
python3 docs/reports/wave36-close/scripts/register_close_findings.py
python3 docs/reports/wave36-close/scripts/retire.py
python3 docs/reports/wave36-close/scripts/merged_gate_row.py
python3 docs/reports/wave36-close/scripts/write_reconciliation.py
python3 docs/reports/wave36-close/scripts/mutate_check_arm.py .

python3 docs/reports/remaining-d02/build_matrix.py . \
    --gate docs/reports/wave36-close/gate.json
```

The registration and retirement scripts are idempotent and refuse rather than
guess: `retire.py` will not remove a name that is not registered, and
`merged_gate_row.py` will not overwrite a row that is not the one it was written
against.
