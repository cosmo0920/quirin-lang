# `SWEEP-BLIND-SPOTS` — the reconciliation sweep's measured blind spots, and wave 32 retro-swept

**Card:** `SWEEP-BLIND-SPOTS` (wave 35, process tooling).
**Base:** `a300a39d` ("Open wave 35: create the increment record and move the index
pointer before any card launches").
**Owns:** `scripts/reconciliation_sweep.py`, `scripts/tests/test_reconciliation_sweep.py`,
`docs/reports/wave32-close/reconciliation.json` (new), `docs/reports/sweep-blind-spots/`,
`docs/reviews/r0_3/acceptance_matrix.json` (append only), `docs/development/increments/wave-35.md`.
**Touches no crate source and no frozen archive.**

## 1. Headline

Four blind spots that `STABILITY-REEVAL` #1 measured in `scripts/reconciliation_sweep.py`
are repaired, each with a Python test built from a fixture the **old behaviour would have
missed**, and each test **falsified by mutating the arm it claims to cover** before being
believed. The tool's one rule is unchanged and restated in its own docstring: **it decides
nothing**; a green `--check` means every item carries a recorded human verdict, never that
any item was applied.

Then the mechanism was pointed at the wave it was built in and skipped in.
`docs/reports/wave32-close/reconciliation.json` now exists — **67 items across all seven
wave-32 cards, every verdict measured against the tree at wave 35**, discharging
`WAVE-32-CLOSE-UNSWEPT`. `docs/reports/wave32-close/gate.json` was not edited; this is a new
file beside it (constraint 21).

**The retro-sweep was not a formality.** It found three things, none of which is a code
defect and all of which are the class of loss an unswept close produces:

1. **`MUTEX-SHARE-CROSS`'s entire integrator list is unlanded** — its proposed standing
   constraint, its `Q600` namespace row and its `Q610`–`Q627` row — while the **authority
   those rows summarise**, spec §L21, *does* carry the widening. `AGENTS.md` contains the
   string `MUTEX-SHARE-CROSS` **zero times**.
2. **A claim of exhaustiveness now stands over an incomplete list**: constraint 14 was
   rewritten at `DOCS-COMPACT-6` to say "**every card of waves 32, 33 and 34**" and then
   enumerates ten, omitting `MUTEX-SHARE-CROSS`, `PROOF-LEG-BATCH` and `DOCS-COMPACT-5`.
3. **A recorded `applied` verdict is false in half its evidence.** `W32-RECONCILE-SWEEP`'s
   own matrix amendment was **resolved away at its own merge** (`3bbec4d0`, −2,208 bytes,
   the only `named_followups` row to change across that merge), and the verdict that cites
   it still reads `applied`.

And, surfaced rather than fixed: **wave 34 has no close directory at all**
(`WAVE-34-CLOSE-UNSWEPT`) — the same defect recurring in the **very wave whose own card
found it**, since `STABILITY-REEVAL` #1 ran in wave 34 and is one of the three cards left
unswept by it.

## 2. The four blind spots

Each row names the residual it discharges, the change, the test, and the mutation that
falsified the test. Mutation runs are in §2.5.

### 2.1 `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` — the scan reads four kinds of document now

`--residuals` read only `docs/reports/`, so a residual introduced anywhere else was
invisible **by construction** — the reason the wave-33 close ran the tool, got an answer,
and did not see `CT-SHARED-FIXTURE-DISCLOSURE`.

`residual_sources()` now returns the named card reports, **every**
`docs/development/increments/*.md`, and the **standing-constraints text** of `AGENTS.md`
and `CLAUDE.md`. `constraint_text()` extracts the section whose heading contains "standing
constraint"; a file with no such heading is read **whole** and the reader says so — that is
`CLAUDE.md`, whose *Platform records* name residuals exactly as a constraint does.

* `test_a_residual_introduced_in_an_increment_record_is_seen` — the name is written in **no
  card report at all**, so the reports-only scan could not have returned it however the
  ledger was arranged.
* `test_the_constraint_scan_reads_the_constraints_section_only` — a name in a *Status
  ledger* heading and one in a *Standing per-change gate* heading are **not** returned; the
  one inside the constraints section is.
* `test_a_file_with_no_constraints_heading_is_read_whole`.

### 2.2 `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` — one tracker, and it is the ledger

`TRACKERS` counted an `AGENTS.md` or `CLAUDE.md` mention as tracking. It no longer does:

```python
MATRIX = "docs/reviews/r0_3/acceptance_matrix.json"
TRACKERS = (f"{MATRIX}:named_followups",)
MENTION_ONLY = ("AGENTS.md", "CLAUDE.md")
```

**A constraint paragraph states a rule that must survive refactors; the ledger states work
that is open.** A name can be in the first and still be untracked as the second — which is
exactly how `RETURN-POSITION-ESCAPE-UNREACHABLE` and
`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS` sat in constraint 5b with no ledger entry
while this tool stayed silent about both. A name found only in those files is now reported
**untracked, with where it was found**, because "untracked, but `AGENTS.md` states a rule
about it" and "untracked, and nothing anywhere mentions it" want different human answers.

The tracker surface is the **serialized `named_followups`**, not the id list: a residual is
legitimately tracked by being named inside another entry's prose, which is how
`WAVE-33-DEFERRED-DECISIONS-UNTRACKED` deliberately registers four names as one entry.

* `test_a_name_only_a_constraint_mentions_is_untracked_work` — **the old test, inverted.**
  The previous suite asserted `untracked_residuals(...) == {}` for exactly this fixture. That
  assertion *was* the defect, so the repair is visible as a test whose expectation moved from
  "silent" to "reported, with `mentions == ("AGENTS.md",)`".

### 2.3 `WAVE-BASE-UNSWEPT` — the seam is inclusive, explicit, and listable

Git's `A B` diff is exclusive of `A`'s own commit, so a wave whose declared base is the
commit that landed some change never swept that change. `37bc9004` is the live instance:
the wave-33 base, landing after the wave-32 close, described by neither wave's increment
record, indexed by no report — while moving two pinned F02 digests and removing four tests
from the Windows provider gate.

* `--since REF` is now **inclusive of `REF`** (`REF^..HEAD`).
* `--range A..B` takes a seam explicitly.
* `--uncarded` lists commits in a range that write no card report at all.
* `cards_changed_since` **keeps its exclusive meaning** as the primitive, so the change is a
  new capability rather than a silent redefinition.

Tests: `test_the_base_commits_own_report_is_swept_only_when_the_base_is_included` asserts
**both** ends — `exclusive == []` (the old behaviour, pinned so the repair stays visible)
and `inclusive == ["base-card"]`; `test_cards_changed_since_keeps_its_exclusive_meaning`;
`test_an_explicit_range_sweeps_between_two_closes`; `test_a_commit_that_writes_no_report_is_listed`;
`test_parse_range_wants_two_ends`; plus two CLI tests. The fixtures build a real
three-commit git repository with the seam shape in it.

**On the live tree**, `--range 474f27af..37bc9004 --uncarded` returns **exactly one
commit**: `37bc9004`. The seam the residual described is now one command away.

### 2.4 `WAVE-33-DEFERRED-DECISIONS-UNTRACKED` — a deferral is re-listed until it ends

`check()` refuses a verdict for an item the sweep cannot find, so a deferred decision could
only ever live in the one JSON file that recorded it, and expired in silence.

**The shape and the vocabulary were read from the wave-33 close, not invented here.** That
close already wrote a `deferred_decisions` block with a `note`, an `items` list and a
`ground` per entry, having reasoned correctly that a decision is not a swept item and so
cannot go in `verdicts`. It got everything right except that nothing would ever read it
again. So `DECISION_VERDICTS = ("deferred", "resolved", "withdrawn")` with `deferred`
the open state, and `docs/reports/wave33-close/reconciliation.json` is **valid unchanged and
was not edited** (constraint 21).

`outstanding_decisions(root, before_wave)` walks every `docs/reports/wave<N>-close/`,
takes each decision's state from the **latest** close to mention it, and returns those still
`deferred`. `check()` then complains if the record omits one. A `deferred_decisions.json`
**beside** a record is read too, so a close whose record is already written can have its
deferrals registered retrospectively in a new file rather than by editing a frozen one.

Measured on the live tree: `outstanding_decisions(root, 34)` returns the wave-33 close's
four — `CT10-REASON-ROW`, `EXAMPLES-STYLE-SLICE-FANOUT`, `MATRIX-UPDATED-STALE-AT-WAVE-27`,
`SYNC-RULE-TOOLING-COMMIT-SCOPE` — so **every close from wave 34 onwards must now restate or
end each of them.** The residual is discharged without a single byte of the wave-33 record
moving.

Tests: `test_a_later_close_that_ignores_an_open_deferral_is_refused`,
`test_re_stating_the_deferral_answers_it`, `test_resolving_it_ends_the_carry`,
`test_a_deferral_restated_without_a_ground_is_refused`,
`test_a_decision_verdict_outside_the_vocabulary_is_refused`,
`test_a_sibling_file_registers_a_closed_records_deferrals`,
`test_a_plain_map_is_read_as_well_as_the_items_list`.

### 2.5 The tests were falsified before they were believed

Six mutations, each reverting one repaired arm, run against the whole suite:

| Mutation | Tests that went red |
|---|---|
| `--residuals` reads only `docs/reports/` (the old scan) | 3 |
| `AGENTS.md`/`CLAUDE.md` count as trackers again (the old `TRACKERS`) | 3 |
| the sweep range is exclusive of its base again | 1 |
| an uncarded commit is not looked for | 1 |
| an earlier close's open deferral is not re-listed | 2 |
| a deferral may be restated without a ground | 1 |

Every mutation was caught; the module was then **restored byte-identically** (asserted by
the mutation script itself). Suite: **27 → 45 tests**, all green.

### 2.6 One heading spelling added

`SECTION_MARKERS` gained `"integrator flip"`. `PROOF-LEG-BATCH`'s owed section is headed
*"5. No integrator flip is proposed"*, which matched nothing, so that card would have needed
a `no_section` excuse for a section it actually has. The docstring already describes a new
spelling as a one-line addition here. Regression checked: both pre-existing live records
(`wave33-close` and `w32-reconcile-sweep`) stay green after the change.

## 3. The wave-32 retro-sweep

```sh
python3 scripts/reconciliation_sweep.py . --range 8bf7fd33..474f27af
python3 scripts/reconciliation_sweep.py . --check docs/reports/wave32-close/reconciliation.json
```

`8bf7fd33` is the wave-31 close; `474f27af` is the wave-32 close. The explicit range is the
capability this card adds, used for the purpose it was added for. It returns **all seven
wave-32 cards and 67 items**; `--check` is green.

**How the verdicts were taken.** Every one is measured against the tree — a `grep` of the
named file, a `git show` of the named commit, a JSON read of the named record — and never
inferred from a report's own claim. Because three waves have passed, a verdict here answers
*"is this in the tree now"*, which is a different and in some ways better question than the
close would have answered; where a later card moved an item, the verdict says so.

### 3.1 Distribution

| Verdict | Count |
|---|---|
| `noted` | 27 |
| `applied` | 18 |
| `deferred` | 16 |
| `superseded` | 4 |
| `declined` | 2 |
| **Total** | **67** |

| Card | Items | Breakdown |
|---|---|---|
| `ct-composed-native` | 17 | 10 applied, 6 noted, 1 superseded |
| `ct-wire-lane-4` | 11 | 10 noted, 1 applied |
| `docs-compact-5` | 1 | 1 applied (with a finding — §4.3) |
| `mutex-share-cross` | 8 | 4 deferred, 2 applied, 2 noted |
| `proof-leg-batch` | 3 | 3 noted |
| `trace-alphabet-lease-boundary` | 6 | 3 superseded, 2 noted, 1 applied |
| `w32-reconcile-sweep` | 21 | 12 deferred, 4 noted, 3 applied, 2 declined |

The `noted` count is high for an honest reason: `CT-WIRE-LANE-4` §10 and
`CT-COMPOSED-NATIVE` §4 are largely *statements of what the card itself changed*, and `noted`
exists so `applied` never has to stretch to cover them. Each `noted` still carries what was
checked.

### 3.2 What landed cleanly

* **`CT-COMPOSED-NATIVE`'s coupled edit — fully applied**, at `a0cbf143`, and the tree
  carries an integrator comment at `ct_corpus.rs:1510` saying so. `RUNNER_REGISTRY` names
  `run_composed_native` (`:445`); `SANITIZED_GATED_CASES` is 9 entries including `CT22` and
  `CT25` (`:456`); the `derive_stage_plan` block is at `:1509`; `ct_manifest.json` gives CT22
  and CT25 `executed` on all three rows with that executor, and **CT23 is unchanged**
  (`not_applicable`/`not_applicable`/`unassessed`), which is the card's own boundary held.
* **`CT-WIRE-LANE-4`'s one owed item — applied** at `1423b6dc`: `AGENTS.md:131` reads
  `45/50 mapped` and `sanitized-native 9 of 45`.
* **`W32-RECONCILE-SWEEP` §5.2 (`STD-MODULES-O`) — applied** at `e0052b14`, and **option (a)
  is the option that was taken**: the row's `why` now names both successors. Options (b) and
  (c) are recorded `declined` with the ground that (a) was taken, which is what the verdict
  vocabulary's refusal arm is for.
* **`MUTEX-SHARE-CROSS`'s Lean-audit reconciliation — applied**: the matrix reads
  `lean_audits` 129 with `AuditMutexShareCross.lean` present, and the expected Python
  tripwire failure the card left is green (247 tests, 0 failures in this card's run).
* **`TRACE-ALPHABET-LEASE-BOUNDARY` §5.4** is three `superseded` and one `applied`: the
  card's per-card numbers (4,516 tests / 126 audits / 200 Python) were correctly replaced by
  the close's own tip measurement in `wave32-close/gate.json` (4,534 / 122 provider / 227
  Python), and its "provider 122 and every corpus row carried — no arbiter ran" is exactly
  what that gate file records.

### 3.3 The three things the missing record cost

**(1) `MUTEX-SHARE-CROSS`'s integrator list is unlanded.** Measured item by item:

| §13 item | Verdict | Measurement |
|---|---|---|
| 1 — scope statement | `noted` | Correct, and why everything below is owed elsewhere |
| 2, 3 — the proposed standing constraint | `deferred` | `AGENTS.md` contains `MUTEX-SHARE-CROSS` **zero times**; no constraint-15 append, no new constraint |
| 4a — the `Q600` namespace row | `deferred` | `AGENTS.md:1441` still reads "**Four** later cards move this row" and names four |
| 4b — the `Q610`–`Q627` row | `deferred` | `AGENTS.md:1442` is one unchanged line |
| 4c — the constraint-14 census row | `deferred` | Absent — and now load-bearing, see (2) |
| 4d — Lean headline 125 → 126 | (in 4) | **Superseded**: 129 at the wave-34 close |
| 5 — derived `lean_audits` | `applied` | Matrix 129, `AuditMutexShareCross.lean` listed |
| 6 — public API notice | `noted` | `transfer.rs:161,167`; `qcore-schemas` untouched, so constraint 18's rebuild correctly **not** owed |
| 7 — CT46 mapping | `deferred` | `ct_manifest.json` gives CT46 `mapping_status: "unmapped"` — correctly still a lane's |

The drift is visible precisely because the **authority** carries what the summary does not:
`docs/QCORE_SPEC_r0_3.md:1004` reads *"Card `MUTEX-SHARE-CROSS` (wave 32) widens `Q600`'s
boundary condition and the `Q610`-`Q627` sub-band's route"*. `AGENTS.md` calls its own
namespace table "a summary of it, not a second allocation" — so for one card the summary and
its authority disagree. **No code is challenged**: the card's Rust, its Lean
(`AuditMutexShareCross.lean`) and its spec text are all in the tree. Registered as
`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`.

**(2) An exhaustiveness claim over an incomplete list.** `DOCS-COMPACT-6` (wave 34) rewrote
constraint 14's "Cards that do not engage this constraint at all" bullet to read "**every
card of waves 32, 33 and 34**" and then enumerates ten. Wave 32 merged **seven** cards
(`wave32-close/gate.json` names them); `MUTEX-SHARE-CROSS`, `PROOF-LEG-BATCH` and
`DOCS-COMPACT-5` are missing from the enumeration. "Waves 28 through 34 each closed at zero
reserved words" rests on it. The omission is not neutral for `MUTEX-SHARE-CROSS`: that card
claimed a **new position for already-reserved words** (`thread scope NAME share PLACE under
MUTEX`) and asked in writing for its `+0 / +0 / +0` census row. Registered as
`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`.

**(3) A recorded `applied` verdict whose "where" does not hold.**
`w32-reconcile-sweep/reconciliation-waves-29-31.json` gives
`docs-compact-5#the-unapplied-integrator-reconciliation-list#1` the verdict `applied` with
the note *"… and `DOCS-COMPACT-O` in the matrix now carries the passes 4 and 5 residual
names"*. Measured by `git show` at three points:

| Ref | `DOCS-COMPACT-O` row | Carries pass-4/5 names |
|---|---|---|
| `16070450` (the card's own commit) | 5,473 bytes | **yes** |
| `3bbec4d0` (its merge) | 3,265 bytes | **no** |
| `HEAD` | byte-equal to the post-merge row | **no** |

It is the **only** `named_followups` row that changed across that merge (−2,208 bytes),
whose parents are `1423b6dc` and `16070450` — so this is an edit lost in a merge resolution,
not a later deliberate retirement. Consequence today: `DOCS-COMPACT-4-TABLE-SYNC` reaches the
ledger **nowhere**, and `DOCS-COMPACT-5-RECONCILIATION-LIST` reaches it only through a
wave-34 entry. The two findings that named the reconciliation defect were, once again, the
untracked ones.

The item's **substance** did land — `AGENTS.md:878` carries the 15h append and `:1402` the
`Q020`/`Q600` rows for `WRITE-LEND-TASKS` — so the verdict here is `applied` with the loss
recorded in its note and registered as `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`. The general
point is about the tool this card maintains: **a green `--check` says a verdict was
recorded, never that its "where" still holds.** This is the first measured instance of the
gap, and it corroborates that wave 32's merges were lossy — `9f90fc42` in the same wave is
titled *"Repair wave-32.md: resolve conflict markers committed by a failed resolution"*.

### 3.4 Deferred decisions in this record

The wave-32 record declares none, and that is correct rather than convenient:
`outstanding_decisions(root, 32)` is empty because no close before wave 32 registered one.
The four decisions `W32-RECONCILE-SWEEP` surfaced were carried by the **wave-33** close in
its own block; the twelve `deferred` verdicts here point at that block, so each decision now
has both a verdict at the wave that raised it and a tracker that re-lists it.

## 4. Uncarded commits in the seam

```sh
python3 scripts/reconciliation_sweep.py . --range 8bf7fd33..474f27af --uncarded
```

**15 of the wave-32 range's 38 commits** write no `docs/reports/<card>/README.md`. **The
tool classifies none of them** — writing no report is *necessary* for being uncarded, not
*sufficient*, and a card's own code commits land in the list too. Read by a human, the list
separates into three kinds:

* **Card commits** (their subject names the card): `MUTEX-SHARE-CROSS: …` ×3,
  `CT-WIRE-LANE-4: map CT44 and CT47 …`, `TRACE-ALPHABET-LEASE-BOUNDARY: prove and exercise …`.
  Not uncarded; each card's report indexes them.
* **Integrator ledger and close commits**: `474f27af`, `e0052b14`, `1423b6dc`, `a0cbf143`,
  `619b2245`, `6caea680`. Each is claimed by a close or by a card's report.
* **Genuinely unindexed**, worth a human eye:
  * **`1c7d8e45`** — *"Record the trace-alphabet union as unproven; institute the
    STABILITY-REEVAL cadence"*. Four files: `CLAUDE.md`, **`crates/qcore-core/src/task_trace_relation.rs`**,
    `docs/development/increments/wave-31.md`, the matrix. It **instituted a standing cadence
    rule and touched crate source**, and owns no `docs/reports/` entry of its own. Stated
    fairly, because it is easy to overread: several wave-32 card reports *do* cite
    `1c7d8e45`, but only as the **base commit** they measured against, never as a description
    of what it changed; and the cadence itself is attributed in `CLAUDE.md`:64 to a **user
    direction of 2026-09-13**, so its provenance is recorded even though the commit is not
    indexed. The observation is narrow: the commit that instituted the `STABILITY-REEVAL`
    cadence is itself of the shape that cadence's method item 3 tells a later card to look
    for.
  * **`9f90fc42`** — *"Repair wave-32.md: resolve conflict markers committed by a failed
    resolution"*: conflict markers reached a committed increment record and were repaired.
  * **`3722d705`** — *"Fix the newline glitch"*, touching `compiler/tests/region_joins_loan.rs`.

**This card cards none of them** and makes no claim that any needed a card — that is exactly
the question `SYNC-RULE-TOOLING-COMMIT-SCOPE` defers. The seam is now visible, which is what
`WAVE-BASE-UNSWEPT` asked for.

On the **wave-33** seam the same command returns exactly one commit, `37bc9004`
(`WINDOWS-COHERENCE-GATES-UNCARDED`). **This card does not card it either**, per its brief.

## 5. The widened `--residuals`, run

```sh
python3 scripts/reconciliation_sweep.py . --cards sweep-blind-spots --residuals
```

**40 residual-shaped names the ledger does not carry.** Of those, **27 are written only in
`docs/development/increments/*.md`** — structurally invisible to the old reports-only scan,
however the ledger was arranged. This is the repair measured on the live tree rather than on
a fixture:

```
CT-COMPOSED-NATIVE-APPLE            CT42-CELL-PENDING-PHASE       RETURN-CALL-NULLABLE-WRAPPER
CT-COMPOSED-NATIVE-ROWS             CT42-CELL-UDP-QUEUE           RETURN-CALL-SYNTHETIC-PLACE-PRESENTED
CT-COMPOSED-NATIVE-SCRIPT           CT46-SENDSYNC                 RETURN-CALL-W4-REACH-BLOCKED
CT-PROOF-CONTROL-ARM-TIER           DOCS-COMPACT-5-EXAMPLES-HOST-GATE   W29-CONTENT-SIZE-UNDECLARED
CT-PROOF-CREATION-FAILURE-ALPHABET  DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE  W29-SIZE-SIBLINGS-UNDECLARED
CT-WIRE-LANE-2                      EVENT-V1-WINDOWS-AFUNIX       W31-SIZE-SIBLINGS-REMAINDER
CT22-CT-MAPPING                     MUTEX-SHARE-CROSS-TASK-BUFFER W4-RECSUM-ALREADY-LANDED
CT42-CELL-DERIVED-PREMISE           RECONCILE-SWEEP-DIMENSIONS-UNPINNED   W4-RECSUM-CONSTRAINT-4
CT42-CELL-DUP-SOURCE                RETURN-CALL-INCOMPLETE-LINE-455       WRITE-LEND-RELEND
```

A further **12 name-sightings are in the standing-constraints text** of `AGENTS.md` or
`CLAUDE.md` and are now reported *untracked, with where they were found* — the §2.2 repair,
also measured on the live tree rather than only on its fixture.

**This card classifies none of them and takes none of them.** Several are plainly card names
rather than residuals (`CT-WIRE-LANE-2`, `CT46-SENDSYNC`, `W4-RECURSIVE-SUMMARIES`), which is
exactly why the tool's contract is "candidates for a human eye". Several others read as
genuine open work no ledger carries — `MUTEX-SHARE-CROSS-TASK-BUFFER` (the half of CT46 that
card left open), `WRITE-LEND-RELEND`, the four `CT42-CELL-*` and the four `RETURN-CALL-*`.
Sorting the list is a human's job and a card's worth of it; it is offered to the integrator
in §6 as a measurement, not a verdict.

One name comes from **this card's own report**: `MUTEX-SHARE-CROSS-EMISSION`, which appears
inside §6.1's verbatim quotation of another card's proposed constraint. It is untracked, and
reporting it is correct rather than noise — it is named in a constraint the integrator is
being asked to land.

## 6. For the integrator

Measured items this card **cannot** land itself: `AGENTS.md` is the integrator's, and
`CLAUDE.md` says an amendment lands **in the words the card proposed** or is recorded
`declined` with its ground — never silently softened (invariant 1).

### 6.1 `MUTEX-SHARE-CROSS`'s proposed standing constraint, verbatim

This is `docs/reports/mutex-share-cross/README.md` §13.1's block quote, reproduced **word
for word**. It is three waves old and was never swept. Land it or decline it with a ground:

> **(card `MUTEX-SHARE-CROSS`.)** A `protected N` payload crosses a task
> boundary only through `thread scope NAME share PLACE under MUTEX { … }` on
> `source.mutex.guard.v1`, and the decision is in **three** halves that must
> not be merged: classification is `qcore_core::transfer`'s (the fifth row;
> each sharing mode requests **one** protocol and is admitted exactly when
> the kind's discipline is that protocol — do not restore the arm that took
> whatever protocol the kind named); the **absent** and **mismatched**
> protocol gates are the crossing's, because the judgment cannot see a
> generation's declared footprint; and **unverified** is the *unedited*
> `CheckedMutexInvariantV1` checker's own `Q611`. Do not re-implement a
> guard rule beside the judgment. The clause is **opt-in**: a child without
> it lowers byte-identically, and the load-bearing reach rule
> (`Frame::admits_reach`) applies only to a declaring child and only in the
> refusing direction. `MUTEX_SHARING_GAP` stays `false` — the *task* route's
> Buffer still has no mutex — and the crossing is **checked, never
> executed** (`MUTEX-SHARE-CROSS-EMISSION`); do not claim native or
> data-race evidence for it. `mutex-share-cross`.

### 6.2 Three `AGENTS.md` rows the same card asked for

1. **`Q600` namespace row** (`AGENTS.md:1441`): reads "Four later cards move this row" and
   names four. Spec §L21 (`docs/QCORE_SPEC_r0_3.md:1004`) already carries
   `MUTEX-SHARE-CROSS`'s widening in full; the summary row should follow its authority.
2. **`Q610`–`Q627` row** (`:1442`): should record that the `unverified` refusal reaches this
   sub-band's `Q611` from the crossing clause.
3. **Constraint 14 census** (`:654` bullet): add `MUTEX-SHARE-CROSS`, `PROOF-LEG-BATCH` and
   `DOCS-COMPACT-5` to the wave-32 enumeration, **or** narrow "every card of waves 32, 33 and
   34" to the cards actually listed. As written the sentence is false.

### 6.3 The lost matrix edit

`DOCS-COMPACT-O` lost `W32-RECONCILE-SWEEP`'s amendment at `3bbec4d0`. Restoring the pass-4
and pass-5 residual names (or registering `DOCS-COMPACT-4-TABLE-SYNC` as its own
`named_followups` entry) closes `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`. This card **did not
do it**: `named_followups` prose for another card's row is integrator-owned recorded
analysis, and the `e0052b14` precedent is the integrator amending it.

### 6.4 Wave 34 has no close record

`docs/reports/wave34-close/` does not exist; every reconciliation record on disk was
enumerated and none names `stability-reeval-1`, `c-gateway-return-call-w4` or
`docs-compact-6`. A sweep of those three returns **25 items with no verdict anywhere**. Not
taken here — out of this card's scope, and 25 items measured against the tree is a card's
work. `WAVE-34-CLOSE-UNSWEPT`.

### 6.5 Retirement candidates

Five residuals this card discharges, for retirement at the wave-35 close with the evidence
above: `WAVE-32-CLOSE-UNSWEPT` (§3), `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` (§2.1),
`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` (§2.2), `WAVE-BASE-UNSWEPT` (§2.3),
`WAVE-33-DEFERRED-DECISIONS-UNTRACKED` (§2.4). This card does not retire them itself.

### 6.6 A note on the brief

The card brief named `scripts/build_matrix.py`. **No such file exists** — verified by
`find . -name "build_matrix*.py"`, which returns exactly one path. The generator is
`docs/reports/remaining-d02/build_matrix.py`, as `section_ownership.generator` in the matrix
itself already records. The matrix here was regenerated through it with
`--gate docs/reports/wave33-close/gate.json` re-supplied so
`validation.measurement_provenance` was not downgraded (the `DOCS-COMPACT-6` precedent), and
`scripts/tests/test_build_matrix_sections.py` is green at 17 tests. Worth correcting in
whatever template the brief came from, since the wrong path would send the next card hunting.

## 7. Boundaries — what this card does not claim

* **The tool still decides nothing.** All four repairs widen what a human is *shown*; none
  adds a judgment. `--residuals` returns candidates, `--uncarded` returns a human-eye list,
  and `--check` remains a completeness gate on the record, not on the repository.
* **A green `--check` does not re-verify a past verdict's "where".** §3.3(3) is the first
  measured instance of that gap, and it is recorded as a residual rather than repaired —
  repairing it would mean the tool measuring English against a tree.
* **The uncarded rule is approximate** (`SWEEP-UNCARDED-RULE-APPROXIMATE`): "writes a card
  report README" is necessary for being carded, not sufficient, and its precision is coupled
  to `SYNC-RULE-TOOLING-COMMIT-SCOPE`.
* **One extractor artifact is recorded, not fixed** (`SWEEP-SEPARATOR-ITEM-ARTIFACT`): a
  `---` rule inside an integrator section becomes an item with no content. It was given a
  `noted` verdict rather than suppressed, because filtering it means the extractor deciding
  what is and is not content.
* **The wave-32 verdicts answer "is this in the tree at wave 35"**, not "was this applied at
  the wave-32 close". Where the two differ the note says so (the four `superseded` rows are
  exactly those cases).
* **No crate source, no frozen archive, no `AGENTS.md` and no `CLAUDE.md` edit.**
  `wave32-close/gate.json` is untouched.

## 8. Gate

Run in this card's worktree at base `a300a39d`, `git submodule update --init --recursive`
first, `df -h .` showing **307 GB free** (well above the brief's 60 GB threshold), so the
full standing gate was run rather than the Python-only subset.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | **exit 0** — `third_party/libevent` at `d82464a2`, the pinned revision (constraint 5) |
| `cargo test --workspace --no-fail-fast` | **4,559 passed / 0 failed / 8 ignored**, 230 result summaries, **exit 0** |
| `cargo fmt --all -- --check` | **exit 0** |
| `cargo clippy --all-targets -- -D warnings` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **`Passed 129 Lean audits`**, no warning, **exit 0** |
| `python3 -m unittest discover -s scripts/tests` | **247 tests, 0 failures**, exit 0 |

**Every step of the standing gate was run**, not a subset: the brief's disk guard set a
60 GB floor and `df -h .` read **168 GB free** after the submodule checkout, so nothing was
skipped. Logs are worktree-private under `measure/` (gitignored), never under `target/` and
never in the shared scratchpad.

### 8.1 What moved, and what did not

This card changes **no crate source**, and the numbers show it:

* **Workspace 4,559 — unmoved** from the wave-34 close's published figure. This card adds
  zero Rust tests, so an equal number is the expected result rather than a coincidence.
* **Lean 129 — unmoved.** No audit added; no matrix `lean_audits` reconciliation owed.
* **Python 229 → 247, +18 exactly.** `scripts/tests/test_reconciliation_sweep.py` goes 27 →
  45 (counted at the base commit `a300a39d` and at this tip), and 229 + 18 = 247. The delta
  is wholly this card's, and no other Python row moved.

### 8.2 Machine load during the measurement, disclosed

Constraint 16 names five load- or parallelism-sensitive rows, so the conditions are reported
rather than smoothed over. **Up to nine `cargo test` processes were running concurrently**
for the whole workspace measurement — this card's plus the five sibling wave-35 cards' —
confirmed by inspecting running processes, with elapsed times spanning 12 to 23 minutes. The
run advanced through the gateway route-parity targets slowly under that contention.

**Both facts, in order.** Under that load **not one of constraint 16's five rows tripped**:
the run above is a single uninterrupted `cargo test --workspace --no-fail-fast` ending
`0 failed` with exit 0, and `grep -c "test result: FAILED"` over the log is **0**. **No
isolated re-verification was required and none is claimed.** `--no-fail-fast` was used as
`GATE-NO-FAIL-FAST` requires, so 4,559 is a measurement rather than a lower bound.

The brief's known host row — `qcore-verifier-host`'s
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`, red in isolation
and green under a full suite — was **green in this full-suite run**, matching the wave-34
close's observation. It was not touched.

## 9. Named follow-ons

| Name | What is open |
|---|---|
| `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE` | `DOCS-COMPACT-O` lost `W32-RECONCILE-SWEEP`'s amendment at merge `3bbec4d0` (−2,208 bytes, the only row to change); `DOCS-COMPACT-4-TABLE-SYNC` reaches no tracker, and a recorded `applied` verdict still cites the lost text |
| `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE` | Constraint 14 claims "every card of waves 32, 33 and 34" over a list omitting `MUTEX-SHARE-CROSS`, `PROOF-LEG-BATCH` and `DOCS-COMPACT-5` |
| `MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED` | The proposed standing constraint and two namespace rows are absent from `AGENTS.md` while spec §L21 carries the widening |
| `WAVE-34-CLOSE-UNSWEPT` | No `docs/reports/wave34-close/` exists; wave 34's three cards' 25 integrator items have no verdict anywhere |
| `SWEEP-UNCARDED-RULE-APPROXIMATE` | "Writes a card report README" is necessary, not sufficient; precision coupled to `SYNC-RULE-TOOLING-COMMIT-SCOPE` |
| `SWEEP-SEPARATOR-ITEM-ARTIFACT` | A `---` rule inside an integrator section becomes a contentless item demanding a verdict |

All six are registered in `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups`
(80 → 86, `dimensions.named_followups` moved with it, generator re-run as a fixed point) —
which is, after §2.2, the only thing that counts as tracking them.
