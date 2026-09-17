# STABILITY-REEVAL #2 — the cadence's second firing

**Card:** `STABILITY-REEVAL` #2, wave 36. **Base:** `2200a04e` (over `60958bf0`).
**Method:** `CLAUDE.md`'s four steps, over waves 34–35.
**Baseline:** `docs/reports/stability-reeval-1/README.md`.

This card is an **audit**. It weakens no gate, edits no frozen archive and
changes no verdict rule. Everything below is labelled **MEASURED** or
**INFERRED**; where a boundary is proved on one side and observed on the other,
this report says so in those words.

**A note on provenance.** This card's **first run** lost its isolation worktree
mid-flight — it ended a turn with a pristine tree while its background
sub-audits were still using it, and the harness auto-removes an unchanged
isolation worktree. Its sub-audits completed and reported anyway. Their results
are used here as **inputs that were re-verified cheaply**, never as findings
taken on trust; each is attributed to "this card's first run", and where
re-verification **changed** the conclusion this report says so. The first act of
this run was to commit a report skeleton, which is why the same failure did not
recur.

---

## 1. Tally over firing #1's 27 residuals

**15 discharged · 3 partial · 10 open** at the base. **The split itself is
CARRIED from this card's first run, not re-derived here**; what this run did is
spot-verify four of its rows independently, each of which held:
`CT-SHARED-FIXTURE-DISCLOSURE` still partial (`ct_manifest.json` contains the
string `digest` **zero** times), the `MUTEX-SHARE-CROSS-ARMS` rows clean (both
arms carry `// **Unreachable, and kept.**` in
`crates/qcore-frontend/src/source_mutex.rs:1332` and `:1365`),
`WAVE-33-DEFERRED-DECISIONS-UNTRACKED`'s retirement damaging (§1.2), and
`FB-PLAN-GATE-HOST-TIMEOUT`'s plan text stale (§1.3). Four of 27 is a sample,
and this report does not present it as more. The open ten:
`RETURN-POSITION-ESCAPE-UNREACHABLE`,
`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`,
`LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED`,
`AUDITALL-TRIPWIRE-CONTROLS-UNCOMMITTED`,
`CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`,
`LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`, `GATEWAY-CONSERVATIVE-UNATTACHED`,
`WINDOWS-COHERENCE-GATES-UNCARDED`, `F02-DIGEST-PIN-DUPLICATED`,
`LEAN-STATEMENT-SEMANTIC-REVIEW`. The three partial:
`CT-SHARED-FIXTURE-DISCLOSURE`, `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`,
`D02-DIMENSIONS-FOLLOWUP-COUNT`.

### 1.1 The pattern, and it is not a coincidence

**MEASURED.** Every residual of firing #1 that needed **a test, a control or a
mechanism** is open. Almost every one that needed **a sentence in `AGENTS.md`**
is discharged. The project discharges documentation obligations reliably and
verification obligations unreliably, and the gap is not narrowing. This is
offered as a finding about the *process*, not about any card: a residual whose
discharge is a sentence is cheap to close in the wave that names it, and a
residual whose discharge is a test competes with the next wave's cards.

### 1.2 The retirement pattern — a residual retired into invisibility

**MEASURED, by the project's own tool.** Firing #1 recorded that
`WAVE-33-DEFERRED-DECISIONS-UNTRACKED` was retired on a ground that removed the
only tracker entry pointing at `CT10-REASON-ROW` and
`MATRIX-UPDATED-STALE-AT-WAVE-27`. Re-verified here three ways:

```
CT10-REASON-ROW                    named_followups=0  AGENTS.md=0
MATRIX-UPDATED-STALE-AT-WAVE-27    named_followups=0  AGENTS.md=0
WAVE-33-DEFERRED-DECISIONS-UNTRACKED  named_followups=0  AGENTS.md=0
```

and `python3 scripts/reconciliation_sweep.py . --cards stability-reeval-1 … --residuals`
lists all three with **no tracker line at all**:

```
  CT10-REASON-ROW
      written in  docs/reports/stability-reeval-1/README.md
  MATRIX-UPDATED-STALE-AT-WAVE-27
      written in  docs/reports/stability-reeval-1/README.md
```

So the names raised by the stability card are now in exactly the untracked set
that `--residuals` exists to surface — which is the same place `DOCS-COMPACT-4`'s
and `DOCS-COMPACT-5`'s own findings sat for two waves. **The defect the cadence
was created to catch has now happened to the cadence's own output.** Recorded as
`STABILITY-REEVAL-1-RESIDUALS-ORPHANED`.

*Discharge:* register `CT10-REASON-ROW` and `MATRIX-UPDATED-STALE-AT-WAVE-27` as
`named_followups` entries in their own right (this card registers them), and add
to the close rule that **retiring a tracker entry requires re-homing every name
it was the sole tracker for** — checked by `--residuals` before the retirement
lands, not after.

### 1.3 One stale accusation, re-verified

**MEASURED.** `docs/plans/fluent-bit-system-coverage/README.md:244` still reads
"**FB-PLAN-GATE-HOST-TIMEOUT** is an unresolved validation finding" and asserts
that `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
**failed**. This card's first run measured that test green 4/4. The plan text is
stale in the **accusing** direction — it asserts a live red test that is green.
Named `FB-PLAN-STALE-RED-ASSERTION`; *discharge:* re-measure and either record
the discharge beside the finding or restate it as an environment boundary.

---

## 2. New findings

### 2.1 `JPC-LEDGER-44-21-STALE` — the ledger carries figures the card forbade

**MEASURED, four ways.** `AGENTS.md`'s Cycle 19 raw corpus row reads:

> "…is exercised on **44** corpus functions, and is blocked at `release_site`'s
> nullable-site and naming premises at every one of the **21** sites that reach
> them."

The card's own report, §11 item 2, says **"Apply in these words"** and the words
are **40** and **14** (`docs/reports/joined-provenance-cycles/README.md:415-416`).
`docs/development/increments/wave-35.md:117` also carries 40/14.

The **21** is not a typo. `docs/reports/joined-provenance-cycles/README.md:251`:

> "(At `a300a39d` the same census read 21 sites with the same two blockers
> universal…)"

— i.e. **21 is the superseded pre-rebase census**, the figure the card's §2
exists to stop anyone quoting, and the one the ledger now quotes.

`CLAUDE.md` is explicit: "An amendment lands **in the words the card proposed**
or is recorded `declined` with its ground — never silently softened while being
'reconciled'."

*Discharge:* correct the row to 40/14, and state in the wave-36 record which
tree each figure was measured on.

### 2.2 `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED` — the close rule's remaining hole

**MEASURED.** `docs/reports/wave35-close/reconciliation.json`, verdict
`joined-provenance-cycles#for-the-integrator#3`:

```json
{ "verdict": "applied",
  "note": "Verified against the card's report, as it asked. … the zero-movement
           sentence with its 40 exercised functions and 14 blocked sites is present.",
  "where": "AGENTS.md, Cycle 19 raw corpus row." }
```

The verdict is `applied`. It says **where**. It says it **verified against the
card's report**. And the fact it asserts is **not in the tree** — that row says
44 and 21.

This is the close rule defeating itself. `CLAUDE.md` already anticipated half of
it ("a reconciliation record that says `applied` without saying where is worth no
more than silence") and this record *does* say where. The remaining hole is that
nothing checks an `applied` verdict's **asserted content** against its **named
location**; `--check` is green whenever every item carries a verdict.

**MEASURED, on how hard this is to see:** a naive scan of all 103 wave-34/35
verdicts for `applied` + a named `AGENTS.md` location + a quoted figure flags
**one** candidate, and that one is a false positive (a spec line number). The
defect is invisible to any check that does not read the named location. That is
the argument for mechanising it rather than asking for more care.

*Discharge:* an arm of `scripts/reconciliation_sweep.py --check` that, for every
`applied` verdict whose `where` names a file and whose `note` quotes a figure or
a sentence, asserts that text is present at that location — shown **red** by
mutating one verdict's figure before it is believed.

### 2.3 `JPC-OFF-CYCLE-THEOREM-ACYCLIC-ONLY` — a theorem vacuous exactly where the card operates

**MEASURED.** `proof/QCore/JoinedProvenanceCycles.lean:153`:

```lean
theorem an_off_cycle_node_does_not_return (g : Graph) (rank : Node → Nat)
    (increasing : ∀ p n, n ∈ g.succ p → rank p < rank n) (n : Node) :
    ¬ Steps g n n
```

The hypothesis `increasing` ranges over **every** edge of `g`, so it asserts the
whole graph is acyclic — under which no node returns to itself, trivially, for
every node. The theorem therefore says nothing about an off-cycle node **in a
graph that has cycles**, which is the only case this card exists for.

The shipped code tests exactly that excluded case.
`crates/qcore-c-gateway/src/legacy_joined_provenance.rs:218` `acyclic_sites` is a
**per-node** test — `on_cycle(node)` is "node is reachable from itself" — run over
a cyclic graph, inside `fixed_point`, whose own doc comment says it is "computed
as the **greatest fixed point** over a cyclic graph".

**A second layer, also MEASURED.** Even granting acyclicity, `Steps` (line 136) is
pure graph reachability. The property `acyclic_sites`' docstring and the report
claim — "runs at most once **per call**" — is about *execution*. The file *has* an
execution relation, `Reach` (line 48, carrying a `World`), and the off-cycle
theorem does not use it.

**The ledger is clean:** `grep -c an_off_cycle_node_does_not_return AGENTS.md` = 0.
The overreach lives in the Lean docstring (lines 148–152) and in
`docs/reports/joined-provenance-cycles/README.md:154-156` ("a node on no cycle
cannot return to itself, which is what makes a per-site fact a statement about
one object and justifies `acyclic_sites`").

*Discharge:* either prove the per-node property over `Reach` for a graph that may
contain cycles elsewhere — the statement the code needs — or amend the docstring
and report line to say the theorem covers wholly acyclic graphs and that
`acyclic_sites`' restriction is **observed, not proved**. The report must not be
edited in place if it is treated as frozen; publish the correction beside it
(constraint 21's pattern).

### 2.4 `JPC-TRANSFERSOUND-UNNAMED-PREMISE` — the soundness theorem described by the hypothesis it lacks

**MEASURED.** `AGENTS.md` (constraint 10b region) says "soundness on a cycle is
`fixed_point_is_sound`, proved by induction on the arrival **with no acyclicity
hypothesis**". True. But the theorem carries three hypotheses, and the principal
one is:

```lean
(transfers : ∀ p n, TransferSound g site p n)
```

— the abstract transfer is sound on **every** edge. `rg TransferSound` over the
whole tree returns hits in **one file only**, the Lean module itself: the premise
is named nowhere in the report, the increment record or `AGENTS.md`.

The card does carry a general caveat (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`,
report:133-137) naming three premises — extraction, CFG construction, declared
release effects. The abstract interpretation's own correctness obligation is a
fourth, and it is the one that does the work. This is method step 2 exactly: the
theorem is **proved**; its premise about the shipped transfer is **observed**.

*Discharge:* name `TransferSound` in the report's premise list beside the other
three, or discharge it against the shipped transfer.

### 2.5 `JPC-UNIQUENESS-ENTRY-AGREEMENT-UNSTATED`

**MEASURED.** `AGENTS.md`: "the fixed-point equation has exactly one solution on
an acyclic graph (`acyclic_solution_is_unique`), so no acyclic body's verdict can
move." The theorem (line 101) additionally requires

```lean
(root : a g.entry = b g.entry)
```

so it proves uniqueness only **among solutions agreeing at the entry**. As
written the ledger sentence is false (solutions differing at the entry exist);
what is true is "exactly one solution per entry value". The load-bearing step —
that the topological pass and the fixed point seed the entry identically — is
unstated in both the report and `AGENTS.md`.

*Discharge:* state the entry-agreement condition, and state (or prove) that the
two routes agree at the entry.

### 2.6 `FOREVER-FOR-EXIT-EDGE-UNOWNED` — a standing constraint is false about the shipped tree

**MEASURED.** `AGENTS.md` constraint 5c:

> "And the rule does **not** generalise to `while (1)` or `for (;;)`: modelling
> those exactly would remove an *exit* edge, which is the accusing direction and
> needs its own Lean statement and re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`)."

That is true of `while (1)`: `Statement::While` sets
`successors = vec![body_entry, continuation]` **unconditionally**
(`legacy_c.rs:5027`), so a `while` header's exit edge is never removed.

It is **false of `for (;;)`**. `legacy_c.rs:3277` sets
`condition_always_true: condition.is_empty()`, and `legacy_c.rs:5056` then makes
the header's successors `vec![body_entry]` — the exit edge **is** removed. Dated
by `git log -S`: introduced in `490f5151 Complete Cycle 19 control-flow
foundation`, untouched by wave 35, and `proof/QCore/ConstantDoWhile.lean`
contains no statement about it. So the shipped tree has taken the step 5c
describes as untaken, without either of the two things 5c says it would need.

**This is not a soundness claim.** An omitted `for` condition is replaced by a
nonzero constant (C11 §6.8.5.3), so removing the exit edge is exact, and `break`
still reaches `continuation` because the builder pushes `break_targets` before
compiling the body. The finding is that a **standing constraint asserts a
property of the code the code contradicts**, and that the neighbour it
misdescribes is unowned — the same shape as
`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`, which *was* recorded.

*Discharge:* narrow 5c's sentence to `while (1)`, and record the pre-existing
`for (;;)` route as a second unowned neighbour with a pinning test driving
`for(;;)` with and without a `break`.

### 2.7 `DO-WHILE-ACYCLICITY-SCRIPT-PREMISE-STALE`

**MEASURED.** `AGENTS.md` 5c records that the integrator caught the card's stale
sentence: the card wrote that `derive` "returns no conditional-provenance facts
at all for a cyclic CFG", which wave 35's second arbiter falsified later in the
same wave, and "landing that sentence verbatim would have put a statement
contradicting 10b into this file."

That sentence **is** landed, verbatim, in the committed script the same
constraint sends readers to for re-measurement —
`docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py:4-6`:

> "`legacy_joined_provenance::derive` topologically sorts the CFG and, when the
> sort fails … **returns no conditional provenance facts at all** for that whole
> function."

Merge order confirms the mechanism: `3ead460b` (do-while, arbiter #1) merged
**before** `4cf9bdf9` (cycles, arbiter #2), so the script measured against a
`derive` its own wave-sibling then replaced. `AGENTS.md` says arbiter #2's §8
reproduced the 654 independently, so the *number* is not in doubt; what is stale
is the committed instrument's stated premise.

*Discharge:* publish a provenance note **beside** the script (constraint 21 — do
not edit the archive) recording that its docstring's premise was falsified by
`JOINED-PROVENANCE-CYCLES` in the same wave, and that the 654 was re-derived in
that card's §8.

### 2.8 `GATE-EXIT-SWALLOWED-BY-PIPE` — greenness that was never measured

**MEASURED**, verbatim from commit `b6ea42df`'s own message:

> "Merge 56a97db0 reached origin with the Python suite red - the integrator's
> gate command piped unittest into tail, so the pipe's exit code was tail's and
> the push proceeded. That is the second time this wave a pipe swallowed a gate"

This is method item 1 at the level of the gate itself: for at least one wave-35
merge, the reported greenness was an artefact of measurement technique. It
happened **twice in one wave**, and the project already carries the rule (this
card's own brief restates it). Advice is skippable; the step must be mechanical.

*Discharge:* a gate runner that captures each step's exit code to a file and
refuses to emit a green `gate.json` unless every captured exit is 0 — or
`set -o pipefail` mandated in `AGENTS.md`'s standing per-change gate section,
with the close's `gate.json` recording the captured exits rather than a
narrated verdict.

### 2.9 Findings carried from the sub-audits (each re-verified or attributed)

* **`JPC-40-UNARCHIVED`** — the "40 deriving functions" and the 14-site premise
  table are sentences in the report with **no** archived script, log or test;
  `census.py`/`census.txt` compute neither, and §10's reproduction recipe does
  not invoke the instrumentation that could. *Discharge:* archive the two
  reducers and their logs, or restate both numbers as measured-once-not-archived.
* **`JPC-PREMISE-CENSUS-BLIND-TO-CYCLICITY`** — the archived premise
  instrumentation emits no function name and no cyclicity flag, so it cannot be
  said whether any of the 14 gate-reaching sites is in a cyclic function; the
  corpus evidence therefore does not establish that the **cyclic** arm's output
  has ever reached a consumer. *Discharge:* re-run with `function` and `cyclic`
  per line and publish the split.
* **`JPC-CONSUMPTION-ONLY-IN-EXIT-CUSTODY`** — a cyclic-derived fact **is**
  consumed, and non-vacuously: `finish_release` mutates
  `state.resources`/`unresolved_effect_sites`, read by `report_leaks`,
  `site_disposition`, `observe_exit` and W4 at `legacy_summary.rs:705`; the test
  `a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release` drives it
  and the card's own mutation log shows three mutations turn it red. But on the
  **corpus** the `release_site` gate passes 0 times in 14, so nothing is consumed
  there, and no test drives a cyclic CFG to a changed diagnostic, leak row or
  status. *Discharge:* a test in which a cyclic body's settled release changes a
  derived parameter mode, or an explicit statement that the consumption channel
  is exit custody only.
* **`RETURN-CALL-WHOLE-EXPRESSION-SINGLE-WITNESS`** — under mutation,
  `a_call_that_is_not_the_whole_return_expression_is_refused` stayed **green**:
  its six shapes are refused by `classify_simple`'s pre-existing guard
  (`legacy_c.rs:4023`), not by the whole-expression premise. Exactly one test,
  with two bodies, separates a false `Safe` from the tree. *Discharge:* a second
  independent row, or a docstring naming the sole witness.
* **`RETURN-CALL-TESTS-CLANG-GUARDED-GREEN`** — all 14 rows early-return when
  `clang_available()` is false, so on a Clang-less machine the file reports 14
  green having measured nothing. (It did **not** fire in this run: clang 21.1.8
  present, 0 skip lines.) *Discharge:* report Clang absence as `ignored`, or
  record the rows as a Clang-conditional measurement.
* **`RETURN-CALL-PIN-CONTROL-UNANCHORED`** — the one `Unknown → Safe` pin move in
  the range is disclosed everywhere required, and its control (the statement form
  asserted `Safe`) detects a regression of that form but **not a co-movement of
  both legs**. *Discharge:* cite the pre-card commit and archived verdict, or
  state the boundary.
* **`CORPUS-TOTALS-UNPINNED-BY-ANY-TEST`** — no test asserts the corpus headline
  counts; waves 34–35 moved four of them and no test literal moved, because none
  exists. *Discharge:* a test asserting the four counts, or a recorded decision
  that the archive-comparison scripts are the intended pin.
* **`INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED`** — the `named_followups`
  figure in `AGENTS.md` was hand-written at three wave-35 merges (values in
  neither parent). All three were **correct**; this is a hazard, not a live
  defect. *Discharge:* a check asserting the figure equals both
  `len(named_followups)` and `dimensions.named_followups`, shown red by mutation.
* **`SINCE-EMPTY-WINDOW-PROPERTY-REHOMED`** and
  **`CT-STAGE-TALLY-LITERAL-UNDISCLOSED`** — two low-severity disclosure gaps,
  both already swept by the wave-35 close. Carried for completeness.

### 2.10 Method step 4 — what a later card does to these claims

The two queued wave-36 arbiters are `JOINED-PROVENANCE-CALL-FRAMES` and
`W36-CORPUS-CONTRACTS-3`.

**`WAVE-36-CARDS-UNCHARTERED` (MEASURED).** Neither has a brief in the tree.
`wave-36.md` was an empty skeleton; `JOINED-PROVENANCE-CALL-FRAMES` appears only
as a residual *name*, three times; `W36-CORPUS-CONTRACTS-3` appears **nowhere in
the repository** — zero hits across `docs/`, `AGENTS.md`, `CLAUDE.md` and the
matrix. Both are chartered only outside the tree. This compounds §2.1: an agent
sizing `CALL-FRAMES` ("the 14 sites") from the **ledger** rather than the report
sizes it against **21** sites that no longer exist, and there is no brief in the
tree to correct it.

**`RELEASE-SITE-PREMISES-PARTIALLY-INSTRUMENTED` (MEASURED).** `release_site`
(`legacy_joined_provenance.rs:358-364`) is a **seven-way** disjunction; the
published census counts **three**. The disjuncts `contained` (:362),
`field_links` (:363) and `conditional_replacements` (:364) have never been the
blocker at any corpus site and were never instrumented. `CALL-FRAMES` removes
`:358` at some subset of the 14, after which `:359-364` are the live gate — and a
post-card census could not distinguish "now blocked by `contained`" from "still
blocked by naming". **A prediction the card should commit to before measuring:**
because `nullable_site` is *independently* universal at 14/14, `CALL-FRAMES`
**alone moves zero corpus sites**, however well it attributes the naming loss;
its payoff is conditional on `JOINED-PROVENANCE-RELEASE-NULLABLE` landing too.

**`LOADER-ABI-MODES-CORPUS-PROPERTY` (MEASURED rule, INFERRED threat).** The
purest instance of the pattern this step hunts. Constraint 8 does not withdraw on
a duty refinement **"because every derivable comparison in the pinned corpus is a
nullability refinement"** — sound only because the corpus contains no such shape.
`W36-CORPUS-CONTRACTS-3` adds declarations, each creating new derivable
comparisons; the first non-nullability duty inequality fires the withdrawal arm,
which constraint 8 itself says can turn a `Safe` unit into a provisional
`Unsafe`, moving the histogram and possibly leak 272.

**`JPC-CENSUS-CONTRACT-SET-DEPENDENT` (MEASURED).** `acyclic_sites` counts
`ContractCall { produced_sites }` as well as `Acquire`, so the whole population
census is a function of the contract corpus — the card's own §2 records
population 2 first measuring **zero** for exactly this reason. Growing the corpus
invalidates 67 / 354 / 40 / 14 as published.

Also recorded: `RETURN-POSITION-PREMISE-CONTRACT-ROUTE` (a `Stored`/`Removed`
declaration is a plausible second route to falsifying `TrackedBasesAreBound`,
where only the member-typing route is named).

### 2.11 The 64-round bound, and the `Alias` read

**`JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED` (MEASURED, five ways).** How
many of the **1,273** cyclic functions exhaust `FIXED_POINT_ROUNDS = 64` and
silently derive nothing is **not measured and not instrumented**: the shipped
code counts nothing; the archived instrumentation has no `rounds_used` or
`converged` field; `census.txt` has no such row; `rg FIXED_POINT_ROUNDS` returns
six hits — three in the module, three in the archived copy, **zero in any test**;
and the only cyclic fixture settles in a handful of rounds. The project's own
convention in the same subsystem is the opposite: the sibling bound
`MAX_ALIAS_TARGETS` sets `state.widened` and `legacy_c.rs:6317-6323` turns that
into an `Unsupported` diagnostic that keeps the function non-`Safe`. Declining is
fail-closed and proved safe, so this is **not unsoundness** — it is an unmeasured
refusal rate behind a green suite, the `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`
shape, and it makes the report's headline ("1,273 run the fixed point and 40
derive") readable as a measurement of derivability when it may be partly a
measurement of the bound.

**`JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE` (MEASURED).** `fixed_point` binds
`let declined = …` at `:258` and returns **that same value** both at `:261`
("nothing to derive") and at `:309` ("did not converge"). No consumer and no
future census can separate them — which is why the rate above is unmeasurable
without a code change.

**The `legacy_c.rs:6826` `Alias` read — answered NO, by exhaustive enumeration.**
An `AbstractPointer` with `targets` empty, `unknown == false` and
`may_be_null == false` **cannot** reach `state.pointers`. The invariant is
`targets.is_empty() ⟹ (unknown || may_be_null)`, and it holds at all six struct
literals, the clone insertion, all three join arms and every write to the two
booleans; the one non-literal `unknown = false` (`:7887`) immediately sets
`may_be_null` (`:7888`), and the load-bearing `may_be_null = false` (`:7890`) is
guarded at `:7880` by `unknown || targets.len() > 1`, so an empty-targets binding
reaching it had `unknown == true`. Two fragility residuals follow:
**`POINTER-EMPTY-TARGETS-INVARIANT-UNSTATED`** (the invariant is asserted
nowhere — no doc comment, no `debug_assert`, no Lean — and rests on two conjuncts
a plausible "completeness fix" would remove) and
**`ABSTRACT-POINTER-DEFAULT-IS-THE-UNSOUND-TRIPLE`** (the struct derives
`Default`, and its `Default` *is* the forbidden triple, so a future
`entry(name).or_default()` — already the idiom six times in this file on
neighbouring maps — would insert it silently).

---

## 3. Measurements completed, and what they settled

| Lead | Result |
|---|---|
| §2 Lead 4 — are the 40 real, and is a fact **consumed**? | Consumed **yes**, with a full call chain and a non-vacuous test; on the corpus **never** (14 gate reaches, 0 passes). The 40 itself is unarchived. |
| §2 Lead 5 — mutate the `Reallocated` arm and the whole-expression premise | **Both go red**, exactly one test each; 14/14 green at HEAD and after restoration, tree byte-identical (md5 equal). Neither claim is vacuous. Both mutations also produce a **false `Safe`**, confirming the premise is load-bearing against invariant 1. |
| §2 — the three unread merge resolutions | **All clean.** Zero sentences dropped, zero constraints softened, at `cfd410d0`, `3ead460b`, `ebc0fd49` and `proof/QCore.lean`. The only hand-written content is one `named_followups` count per merge, each correct. `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`'s failure mode did **not** recur. |
| §2 — the CT46 / CT-CTWIN-LANE ledger question | Settled independently: the lane never wrote the line (its brief deferred ledger numbers to the integrator), so the close's "unapplied" verdict is right in kind and the merge-loss reading is wrong. Classified as an **integrator obligation missed at merge**. |
| §2 Job 2.4 — weakened tests in waves 34–35 | **Zero.** `#[ignore]` 8→8 byte-identical; `cfg` attribute multiset identical; assertion-bearing lines 20,097→20,217 Rust and 493→529 Python with **no file decreasing**; three pinned expectations moved, all three disclosed in **both** report and commit message. My specific lead did **not** hold. |
| §2 — the fused `wave-35.md` entries | Repaired on `main`. `git diff --numstat 60958bf0 2200a04e` = **3 added / 0 deleted**, every added line blank; at HEAD every card paragraph is blank-line separated. **Discharged.** |

---

## 4. Postponed, and why

* **The 64-round decline rate itself** is postponed and **cannot** be measured
  without a code change (§2.11): the two outcomes return the same value. What is
  delivered here is the demonstration that it is unmeasured and the smallest
  change that would make it measurable — not the number.
* **Whether the 40 deriving functions and the 14 gate-reaching sites intersect**
  is undecided (`JPC-PREMISE-CENSUS-BLIND-TO-CYCLICITY`): the archived
  instrumentation records neither function name nor cyclicity, so the corpus
  evidence cannot establish that the **cyclic** arm's output has ever reached a
  consumer. The test suite does; the corpus cannot be asked.
* **`LEAN-STATEMENT-SEMANTIC-REVIEW`** (open from firing #1) is **partly
  advanced** here: §2.3–§2.5 are three instances of exactly that review, all in
  one module. That is evidence the residual is real and worth a card, not
  evidence it is discharged. It stays **open**.
* No attempt was made to re-derive firing #1's ledger/citation sweep (nine
  headline numbers, 37 theorem-shaped identifiers, 98 path citations). It is
  **carried**, attributed, and not re-measured — except `named_followups` = 121,
  re-counted here.

---

## 5. Gate

Run at `ab45406d` on this card's tree by `measure/gate.sh`, which **captures
every step's exit code to its own file and pipes nothing**. That is not
ceremony: §2.8 records a wave-35 merge reaching origin with the Python suite red
because a gate command took a pipe's status instead of the command's, twice in
one wave. A card reporting on that defect must not commit it.

| Step | Result | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **4,580 passed · 0 failed · 8 ignored** | **0** |
| `cargo fmt --all -- --check` | clean | **0** |
| `cargo clippy --all-targets -- -D warnings` | clean | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | JSON emitted | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | JSON emitted | **0** |
| `python3 scripts/check_lean_audits.py` | **132 Lean audits passed** | **0** |
| `python3 -m unittest discover -s scripts/tests` | **247 tests, OK** | **0** |

`grep -c '^test result: FAILED' measure/test.log` is **0**, so the totals are a
measurement of the whole run rather than of everything before a first failure —
which is what `--no-fail-fast` is in the gate for (`GATE-NO-FAIL-FAST`).
`git submodule update --init --recursive` ran first and exited 0; `df -h .`
showed 258 GB free at the start and 166 GB at the gate, so every step ran and
none was skipped for space. The Python suite is reported because this card
**changed `acceptance_matrix.json`**, where the derived-vs-carried checks live;
247 matches the wave-35 close's own figure.

**This card changed no Rust, so these numbers are not evidence about a code
change** — they are evidence that the documentation, matrix and increment edits
left the tree green. The 4,580 is above the wave-32 ledger's 4,534 because of
waves 33–35's cards, not this one.

---

## 6. Integrator reconciliation list

1. **Correct `AGENTS.md`'s Cycle 19 raw corpus row** from "44 corpus functions"
   and "21 sites" to **40** and **14**, the words card `JOINED-PROVENANCE-CYCLES`
   §11 item 2 asked to be applied (§2.1). Record the correction of
   `docs/reports/wave35-close/reconciliation.json`'s verdict note **beside** it rather than
   editing that record silently.
2. **Adopt the `--check` content arm** of §2.2, or record `declined` with its
   ground. This is the one finding here that mechanises a rule the project has
   already been bitten by twice.
3. **Amendment to standing constraint 5c**, proposed in these words: after the
   sentence "And the rule does **not** generalise to `while (1)` or `for (;;)`",
   replace "`while (1)` or `for (;;)`" with "`while (1)`", and append: "A
   pre-existing route already removes the **exit** edge for `for (;;)`:
   `condition.is_empty()` at `legacy_c.rs:3277` sets `condition_always_true`,
   which `legacy_c.rs:5056` uses to drop `continuation` from the header's
   successors. It is exact (C11 §6.8.5.3) and `break` still reaches
   `continuation`, but it carries neither a Lean statement nor a pin, and it is
   not owned by card `C-GATEWAY-CONSTANT-DO-WHILE`
   (`FOREVER-FOR-EXIT-EDGE-UNOWNED`)." Land it in these words or record
   `declined` with the ground.
4. **RETIREMENTS — named explicitly, as the brief requires.** This card retires
   **nothing**. No residual of firing #1 is retired here; the three the first run
   found re-described are **re-registered**, not retired (§1.2).
5. **Re-home the orphans:** `CT10-REASON-ROW` and
   `MATRIX-UPDATED-STALE-AT-WAVE-27` are registered by this card. Add to the
   close rule that retiring a tracker entry requires re-homing every name it was
   the sole tracker for, checked by `--residuals` **before** the retirement lands.
6. **`FB-PLAN-STALE-RED-ASSERTION`** (§1.3): `docs/plans/fluent-bit-system-coverage/README.md:244`
   asserts a red test that measures green. Correct or restate as a boundary.
7. **Next firing.** `STABILITY-REEVAL` #3 is due after wave 37. The highest-value
   unexamined ground, on this firing's evidence: the **corpus artifacts**, which
   §2.9 shows are pinned by no test at all, and the **remaining Lean docstrings**,
   where a 3-for-3 hit rate in one module (§2.3–§2.5) suggests the sample is not
   exhausted.
