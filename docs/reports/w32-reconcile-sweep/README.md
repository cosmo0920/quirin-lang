# W32-RECONCILE-SWEEP — the integrator's own reconciliation lists, measured and made mechanical (wave 32)

A defect in the *integrator's* process has now been reported three waves
running. Cards close with an explicit section addressed to the integrator —
"Integrator reconciliation list", "For the integrator", "Integrator carry
list", "Proposed amendment text for standing constraint N", "Proposed
proof-stage evidence (the integrator's flip)". At each wave close the
**headline numbers** were applied and the rest repeatedly was not.
`DOCS-COMPACT-4` filed it as `DOCS-COMPACT-4-TABLE-SYNC`; `DOCS-COMPACT-5`
filed it again as `DOCS-COMPACT-5-RECONCILIATION-LIST` with the judgment that
its predecessor's remedy had been insufficient — *"the cards wrote the lists,
the close didn't work them"* — and with a concrete casualty:
`WRITE-LEND-TASKS` §12's proposed **constraint 15h append** stood unlanded for
an entire wave beside a shipped, Lean-proved task-route write lease.
`DOCS-COMPACT-5` landed that one. **This card finds the others, repairs what is
safely repairable, and replaces the advice with a mechanism.**

## Headline

* **77 integrator-addressed items** across the **20** card reports of waves 29,
  30, 31 and 32-so-far, each given a verdict **measured against the tree at
  this card's tip** — the target file grepped for the proposed text or its
  effect — never inferred from the report's own claim. The record is
  `reconciliation-waves-29-31.json` beside this file, and a test in the gate's
  own Python suite (`python3 -m unittest discover -s scripts/tests`) holds it
  complete.
* **Verdicts: 37 applied, 38 noted, 1 superseded, 1 deferred, 0 declined.** The
  three-file surface is in much better shape than the finding implies —
  because `DOCS-COMPACT-5`, merged days earlier, had just swept it. What
  remained was **outside** those three files, and that is the finding of this
  card: the sweep after a compaction pass measures the compaction pass's reach,
  not the close's.
* **Three unapplied items found and repaired**, all documentation-side and
  unambiguous: a stale derived count in the acceptance matrix, a stale CT proof
  count in a `ct_corpus.rs` doc comment (**the same sentence, stale a second
  time**), and an increment-index clause a card had named for discharge.
* **Two residual names that describe this very defect** —
  `DOCS-COMPACT-4-TABLE-SYNC` and `DOCS-COMPACT-5-RECONCILIATION-LIST` —
  were tracked **nowhere**: not in `named_followups`, not in `AGENTS.md`, not
  in `CLAUDE.md`. The follow-on saying "the close loses follow-ons" was itself
  lost by the close. Repaired.
* **The mechanism:** `scripts/reconciliation_sweep.py` (+ 27 tests) plus a
  named close step in `CLAUDE.md`. It sweeps each merged card's
  integrator-addressed section into individually addressable items and refuses
  a wave-close record that lacks a verdict for any of them; `--residuals`
  lists residual names the wave introduced that no tracker mentions.
  **It decides nothing.** A green `--check` means every item carries a recorded
  human verdict, never that any item was applied — stated in the script, in the
  CLI's own success line, and pinned by a test.
* **Four things need an integrator decision** (§5), listed with options and
  evidence on the `CT-COMPOSED-NATIVE-ROWS` precedent rather than guessed.
* **No arbiter was run and no corpus artifact was produced or read** (this
  wave runs none). No constraint was weakened, no pinned expectation deleted,
  no diagnostic downgraded, no frozen archive edited, no `Q` code allocated.
  Nothing is promoted and no corpus count moved.

## 1. Scope and method

Cards were enumerated from the wave-close increment records
(`docs/development/increments/wave-29.md` … `wave-32.md`) and cross-checked
against `git log` merge commits since the wave-28 close (`5d8b2ff1`) and
against `git diff --name-only 5d8b2ff1 HEAD -- docs/reports`. The three sources
agree on twenty report directories; each card's own `README.md` is
authoritative for that card. The new tool's own scope selector agrees
independently: `scripts/reconciliation_sweep.py . --since 5d8b2ff1` reports
*"77 integrator-addressed items across 20 cards"*, the same twenty this card
enumerated by hand.

Every verdict is **measured**, and the measurement is recorded in the verdict's
note. Concretely: constraint amendments were checked by grepping `AGENTS.md`
for the proposed clauses; namespace-table widenings by reading the row;
proof-stage flips by reading `PROOF_GATED_CASES`, `RUNNER_REGISTRY`, the
manifest and the per-card `ct_stage_evidence.json`, with
`python3 scripts/ct_stage_summary.py . --check` green (43 mapped / 7 unmapped,
proof depth 3) as the cross-check; corpus baseline moves by reading
`AGENTS.md`'s chain paragraph; derived matrix values by recomputing them from
disk.

Two throwaway measurement scripts are worktree-private under `measure/`:
`report_paths.py` (do the reports' backticked repository paths resolve — yes,
every apparent miss is either a path relative to `compiler/tests` or
`contract/qcore`, or a `.gz` artifact, each verified by hand) and
`increment_coverage.py` (does every card report have an increment paragraph —
yes; `development-docs-structure` is recorded at `wave-31.md` line 323 and was
a false negative of the checker's uppercase match).

**Proved vs. observed** (`STABILITY-REEVAL` discipline). Nothing in this card
is proved. Every verdict is an **observation** of one tree at one commit, made
by reading; the only mechanically re-checkable part is that each item *has* a
verdict, which is what the new test pins. A verdict of `applied` means the
proposed text or its effect was found in the named file, not that the text is
correct.

## 2. The inventory

Verdict vocabulary: **applied** (landed; note says where) · **declined**
(deliberately not; note carries the ground) · **superseded** (a later card
replaced it) · **deferred** (carried, to a named owner) · **noted** (no action:
a preamble, a suggested reading, or a statement of what the card itself did).

Item ids below are abbreviated; the full ids, and every note, are in
`reconciliation-waves-29-31.json`.

### 2.1 Cards with an integrator-addressed section

| Card (wave) | Item | Verdict |
|---|---|---|
| `CT01-PROOF-LEG` (29) | §6 CT01 manifest `proof` row | **applied** — present; `ct_stage_summary --check` green |
| | §6 no `host_gate`/`tool_gate` | **applied** — the row carries neither |
| | §6 `ct_corpus.rs` `RUNNER_REGISTRY` + `PROOF_GATED_CASES` + `derive_stage_plan` arm | **applied** — lines 190–208 and 410 |
| | §6 headline `proof 0 of 41 → 1 of 41` | **applied** — `AGENTS.md` CT row (now 3 of 43) |
| | §6 suggested reading of the flipped row | **applied** — carried nearly verbatim in the CT row |
| | §6 two preamble blocks | noted |
| `CT-PROOF-LEG-2` (30) | §7 (a) CT37 manifest row · (b) `PROOF_GATED_CASES` row · (c) stage-evidence record · matrix regeneration · headline `1 of 42 → 2 of 42` · suggested reading | **applied** (six items) — `ct-proof-leg-2/ct_stage_evidence.json` exists and supersedes T00's frozen `coverage.json` **without editing it** (constraint 21) |
| | §7 preamble, "needs no edit" note | noted (two) |
| `CT03-PROOF-ALPHABET` (31) | §7 (a) CT03 manifest row · (b) `PROOF_GATED_CASES` row · (c) stage-evidence record + its `proof` note · matrix `lean_audits` · (d) headline `proof 2 of 43 → 3 of 43` · suggested reading | **applied** (eight items) — the wave-3 CT-integration record is superseded, not edited |
| | §7 preambles, sub-headings, merge-order statement | noted (five) |
| `CT22-WIRE` (29) | §9 nine "files this lane owns and changed" rows | noted (nine) — each verified present |
| | §9 **"Not changed, and left for the integrator"**: `AGENTS.md` CT row `41/50 → 42/50`, README, matrix | **applied** — superseded in place by wave 31's 43/50; `README.md` line 110 agrees; matrix regenerated at the wave-29 close (`30aee0b2`) |
| `CT-WIRE-LANE-3` (31) | §9 nine file rows | noted (nine) |
| | §9 **"left for the integrator"**: CT row `42/50 → 43/50`, sanitized-native `7 of 43`, README, matrix | **applied** — `AGENTS.md`, `README.md` line 110, matrix at `8bf7fd33` |
| `REGION-JOINS-DUTIES` (29) | §2 proposed constraint **15m** | **superseded** — `REGION-JOINS-LOAN` (30) rewrote the same constraint by design; `AGENTS.md` 15m carries the loan text, which quotes the duties text |
| `REGION-JOINS-LOAN` (30) | §2 proposed constraint **15m** | **applied** — `AGENTS.md` lines 932–987, phrase-checked (`REGION-JOINS-EPOCH-AXIS`, `REGION-JOINS-MULTI-OWNER`, the acquired/adopted two-arm rule) |
| | §9 `named_followups` reconciliation for `T08-COMPOSE-LOAN` and `REGION-JOINS-LOAN` | **applied** — `T08-SRC-O`'s `why` records the discharge in the card's own terms |
| `SLICE-PERMISSIONS-WRITE-LEND` (29) | §12.1 proposed constraint **15h** amendment | **applied** — `AGENTS.md` 15h line 767 onward; the "no write lend" sentence is gone |
| | §12.2 `Q051`–`Q057` five widenings, `Q068`–`Q079` one | **applied** — both rows carry them |
| | §12.3 derived `lean_audits` 115 → 116 | **applied** — matrix tracks the tree |
| | §12.5 CT07 mapping may be extended by a lane | **applied** — `CT07-WRITE-REMAP` (30) did exactly that |
| | §11.2 expected Python failure · §12 preamble · §12.4 API inventory | applied / noted |
| `WRITE-LEND-TASKS` (31) | §12.1 proposed constraint **15h append** | **applied LATE** — by `DOCS-COMPACT-5` (wave 32), unlanded for a whole wave; the occasioning casualty |
| | §12.2 `Q020`/`Q021` and `Q600` rows; Lean headline 123 → 124 | **applied** — by `DOCS-COMPACT-5` (its drift 8) |
| | §12.3 derived `lean_audits` | **applied** |
| | §12.5 CT07 mapping and **CT10's reason row** | **deferred** — explicitly a lane's work; CT10 stays unmapped and no lane has taken §8's readiness |
| | §11.1 expected Python failure · §12 preamble · §12.4 API inventory | applied / noted |
| `W29-CORPUS-CONTRACTS` (29) | §10 baseline move + corpus row | **applied** — superseded in place; `AGENTS.md` names it among the frozen predecessors |
| `W4-RECURSIVE-SUMMARIES` (30) | §10 baseline may move; §2.2 **constraint-4 amendment** | **applied** — `AGENTS.md` lines 212–224 carry "above the neutral element" with both premises and the prefix-preservation theorem named |
| `W31-CORPUS-CONTRACTS-2` (31) | §11 baseline move + corpus row | **applied** — `AGENTS.md` lines 104–120 and the corpus row |
| `DOCS-COMPACT-5` (32) | §4.1 the unapplied reconciliation list | **applied** — by this card |

### 2.2 Cards with no integrator-addressed section

The tool treats a swept card with no recognised section as a **complaint**, not
a pass: either the heading is a spelling it has not learned, or the card owed
nothing, and a person must say which. All seven are answered in the record's
`no_section` map; the grounds in brief:

| Card (wave) | Why nothing was owed |
|---|---|
| `CT47-COPY-CAPTURE` (29) | applied its **own** `AGENTS.md` `Q600` row and `acceptance.json` reason row — verified present. This is the one card of four that `DOCS-COMPACT-4-TABLE-SYNC` records as *not* having slipped |
| `CT07-WRITE-REMAP` (30) | §6 is the card's own follow-on list; its one integrator-relevant observation (a stale `ct_corpus.rs` doc comment) was repaired by `CT-WIRE-LANE-3` — and has since gone stale again, §3.2 |
| `CT24-CLEANUP-PREEMPTED` (30) | moved no code, widened no condition; `AGENTS.md`'s `Q600` row records exactly that |
| `CT42-PROTOCOL-CELL` (31) | §6's named follow-ons, `CT42-MAP` included, are a lane's work and live in its own report per the ledger rule |
| `C-GATEWAY-WAVE` (31) | the user's own card, landed directly on `main`; baseline and corpus figures verified in `AGENTS.md` |
| `DEVELOPMENT-DOCS-STRUCTURE` (31) | a tooling commit (`c05b2c4b`), not a delegated card; see the decision in §5.3 |
| `DOCS-COMPACT-4` (30) | a compaction pass applies its own findings; its one follow-on reached no tracker until this card |

## 3. What this card applied

Three items, each documentation-side and unambiguous. Nothing was softened
while being reconciled: an amendment lands in the words the card proposed, or
it is flagged (constraint 1, invariant 1).

### 3.1 `dimensions.named_followups` was 39 against a list of 40

`docs/reviews/r0_3/acceptance_matrix.json`'s `dimensions.named_followups` is a
**derived** field — `len(carried("named_followups"))` in
`docs/reports/remaining-d02/build_matrix.py` line 582. Commit `1c7d8e45` added
the fortieth entry (`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`) *after* the
wave-31 close regeneration (`8bf7fd33`), so the committed file carried 39
against a list of 40. `AGENTS.md` says 40, because `DOCS-COMPACT-5` counted the
list rather than reading this field.

Corrected to 40 by recomputation, not by hand-typing: the whole-file round trip
`json.dumps(matrix, indent=2) + "\n"` reproduces the committed bytes exactly
(**measured**), so the diff is the four changed lines and nothing else. The
existing tripwire `test_the_derived_facts_are_recomputed_rather_than_carried`
does not compare this field, which is why it stayed green and why the drift
survived — recorded, not repaired, as `RECONCILE-SWEEP-DIMENSIONS-UNPINNED`
(§6).

### 3.2 `ct_corpus.rs` says "proof is 2 of 43" — the same sentence, stale twice

`compiler/tests/ct_corpus.rs` line 3113's doc comment read *"proof is 2 of 43
(CT01 and CT37)"*. Proof has been **3 of 43** since `CT03-PROOF-ALPHABET`
(wave 31), which merged *after* `CT-WIRE-LANE-3` — the lane that had just
refreshed this very sentence from its previous staleness ("0 of 42"), a
staleness `CT07-WRITE-REMAP` §6 had observed in passing.

So the sentence has now gone stale twice, from the same cause both times: it is
prose that no test asserts, sitting next to a pinned table that no one doubts.
Repaired to name CT03 and its card, and the comment now **records both
stalenesses and says why it drifted** — the authority is `PROOF_GATED_CASES`
and the manifest, both of which read three. Verified not load-bearing:
`ct_mapping_progress_is_pinned_and_counted` asserts mapped/unmapped counts
only, and nothing string-matches the comment. A repository-wide grep for the
same shape (`of 4x` in `.rs`/`.py`/`.lean`) found no other instance.

### 3.3 `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`, discharged on its own terms

`DOCS-COMPACT-5` §8 filed it with the discharge condition *"drop or scope the
clause when that file is next edited"*. This card edits that file (the
increment paragraph below), so it is discharged: the increment index's
instruction to prepend "above the migrated section" is **scoped** — measured,
`wave-32.md` carries no `<!-- migrated-section -->` block while `wave-29.md`
through `wave-31.md` all do — rather than dropped, so it still describes the
files that have one.

### 3.4 The untracked residual names

`DOCS-COMPACT-O` is the docs-compaction family row in `named_followups` and
stopped at the **third** pass. Passes four and five and their four residual
names appeared in **no** tracker at all — measured with the new
`--residuals` mode against `acceptance_matrix.json`, `AGENTS.md` and
`CLAUDE.md`. The row now carries both passes and, in the cards' own words:
`DOCS-COMPACT-4-TABLE-SYNC`; `DOCS-COMPACT-5-RECONCILIATION-LIST` with its
discharge condition and its casualty; `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`
marked discharged by this card; and `DOCS-COMPACT-5-EXAMPLES-HOST-GATE` marked
standing, deliberately not reworded. `named_followups` is a `hand_maintained`
section the generator carries verbatim, so this append survives regeneration
(constraint 25) — verified by running the generator's own preservation test.

The full `--residuals` sweep over all twenty cards reports **42** untracked
names. The other 38 are in their own cards' follow-on lists, which
`AGENTS.md` names as a legitimate home ("plus each report's own follow-on
list"), so they are **candidates, not defects** — which is exactly what the
tool says about them. The list is in §6's second table for a future reader.

## 4. The mechanism

Both prior findings failed because the remedy was advice. `DOCS-COMPACT-4`
proposed "a sentence in the next wave's card template, not a rule change";
`DOCS-COMPACT-5` observed that this had not worked and set a discharge
condition instead: *"a wave close whose own record states, per card, that every
item of that card's reconciliation list was applied or explicitly declined."*
This card makes that condition checkable.

### 4.1 `scripts/reconciliation_sweep.py`

Two operations, plus a listing mode.

* **`sweep`** finds every integrator-addressed section in a card report and
  splits it into individually addressable **items** with stable ids
  (`<card>#<section-slug>#<n>`). Section numbers are stripped from the slug, so
  renumbering a report's sections does not orphan a recorded verdict.
  Recognition is by a heading vocabulary of eight substrings, because the
  twenty cards spelled one intent six ways.
* **`check`** refuses a wave-close record that lacks a verdict for any swept
  item, carries a verdict outside the vocabulary, carries a non-`applied`
  verdict with no note, or carries a verdict for an item the sweep does not
  find. A swept card with **no** recognised section is a complaint, answerable
  only by a written reason in the record's `no_section` map — and that map is
  checked in both directions, so it cannot be used to wave away a card whose
  section the sweep *did* find.
* **`--residuals`** lists `SCREAMING-KEBAB` names a report writes in backticks
  that appear in neither the acceptance matrix's `named_followups` nor
  `AGENTS.md` nor `CLAUDE.md`. This is the defect's other half, and it found
  its own occasioning residuals (§3.4).

Three design decisions, each made against a measured failure of the naive
version:

1. **Prose blocks are items, not just list entries.** `CT22-WIRE` and
   `CT-WIRE-LANE-3` put the owed half in a closing *paragraph* after a bulleted
   list of files the lane changed itself. Read list-only, those sections yield
   nine items none of which is the one thing owed. Pinned by a test.
2. **Fenced blocks attach to the block they follow.** A proposed JSON manifest
   row or Rust table entry is evidence for an item, not a list of its own.
   Pinned by a test.
3. **A fifth verdict, `noted`,** for a block that carries no action — a
   preamble, a suggested reading, a statement of what the card itself did. It
   exists so `applied` never has to stretch to cover them, and it demands a
   note for the same reason. 38 of the 77 items are `noted`; without it they
   would have been recorded as applied, and a ledger that calls a preamble
   "applied" teaches a reader to skim.

**What it does not do, said plainly because the alternative would be worse than
no tool.** It does not decide whether an item was applied, and it cannot: an
item is a paragraph of English addressed to a person, and only a person can
measure a tree against one. A green `--check` means *every item has a recorded
human verdict* and never *every item was applied*. That sentence is in the
module docstring, in the CLI's own success line, and pinned by the test
`test_green_output_disclaims_having_measured_the_tree`. A reconciliation record
that says `applied` without saying where is worth no more than silence.

### 4.2 The close rule in `CLAUDE.md`

A new working rule names the step, its two commands, its evidence file
(`docs/reports/wave<N>-close/reconciliation.json`, beside the `gate.json` each
close already writes) and the tool's limits. It also restates the discipline
this card worked under: an amendment lands **in the words the card proposed**
or is recorded `declined` with its ground — never silently softened while being
"reconciled".

`AGENTS.md`'s entry points gain a pointer to the script, so an agent reading
the briefing can find it.

### 4.3 Evidence that the mechanism works on the real corpus

`reconciliation-waves-29-31.json` is this card's own inventory in the record's
format: 20 cards, 77 verdicts, 7 `no_section` answers,
`scripts/reconciliation_sweep.py . --check` green. The Python test
`test_this_cards_own_retrospective_record_is_complete` holds it that way, and
its docstring says in terms that complete is not correct.

Had this mechanism been in place at the wave-31 close, the two items
`DOCS-COMPACT-5` had to land a wave later —
`write-lend-tasks#integrator-reconciliation-list#3` and `#4` — would have had
no verdict, and `--check` would have exited nonzero. That is an **argument, not
a measurement**: it is what the tool does on those inputs today, and this card
cannot re-run a past close.

## 5. For the integrator — decisions, not guesses

Four items are not unambiguous. Each is listed with its options and evidence,
on the `CT-COMPOSED-NATIVE-ROWS` precedent, rather than repaired on a guess.

### 5.1 The matrix's `updated` field is four waves stale

`acceptance_matrix.json`'s `updated` reads *"2026-09-12 (wave 27 complete: …)"*
while `validation.note` reads the wave-31 close. `section_ownership` labels
`updated` **`integrator_supplied`** — "the integrator's own note for this
checkpoint" — so writing it is authoring an integrator checkpoint statement in
the integrator's voice, which this card will not do.

* **(a)** The integrator writes a wave-28…31 checkpoint note.
* **(b)** Record that `validation.note` has become the live checkpoint note and
  that `updated` is a date stamp only — and say so in `section_ownership`, so
  the next reader is not misled by a four-wave-old summary.
* **(c)** Leave both, and accept two checkpoint notes of different ages in one
  file.

Evidence: `dimensions` and `validation.note` moved at the waves-29/30/31
closes; `updated` did not.

### 5.2 `STD-MODULES-O` lists a follow-on that has been discharged for three waves

`named_followups`' `STD-MODULES-O` row records
`SLICE-PERMISSIONS-WRITE-LEND` as **new and open** (from wave 27's
`CT07-SLICE-PERMISSIONS`). It landed in wave 29, carries its rules in
`AGENTS.md` 15h, and published its own successor residuals
(`WRITE-LEND-COMPUTED-INDEX`, `WRITE-LEND-RELEND`, `WRITE-LEND-NESTED-ACQUIRE`,
`WRITE-LEND-BOUND-FROM-BYTE`, `SLICE-PERMISSIONS-LOOP-WINDOW`,
`SLICE-PERMISSIONS-CROSSED-WINDOWS`, `SLICE-PERMISSIONS-COMPOSED-LEND`), which
wave 31's `WRITE-LEND-TASKS` family then extended. The ledger therefore
overstates open work in this family.

Not repaired here because the card that landed it **did not write the text**,
and `named_followups` is integrator-owned recorded analysis; authoring status
prose there is a judgment call this card's brief reserves.

* **(a)** Append a discharge sentence to `STD-MODULES-O` naming the successors
  (smallest change; the row grows again).
* **(b)** Open a `WRITE-LEND-O` family row and cross-reference, matching how
  `REGION-SRC-O` and `AGGREGATES-O` were handled.
* **(c)** Decide that family rows freeze at the card that opened them and that
  per-card follow-on lists are the live record — and say so in `AGENTS.md`'s
  "where the detail lives" paragraph, which currently implies both are live.

### 5.3 Does the sync rule bind a tooling commit that publishes a report?

`docs/reports/development-docs-structure/` was written by commit `c05b2c4b`,
the `DEVELOPMENT.md` split, which `DOCS-COMPACT-5` §5 describes as done "by a
different tool". It has an increment paragraph (`wave-31.md` line 323) but no
card-style boundary or gate section, and it publishes under `docs/reports/`,
where every other directory is a delegated card's evidence package.

* **(a)** Treat it as a card retrospectively (it already has its paragraph;
  nothing more is owed).
* **(b)** Record in `CLAUDE.md` that tooling commits may publish under
  `docs/reports/` and are exempt from the card-close discipline.
* **(c)** Move such reports to a separate directory so `docs/reports/*` stays
  one kind of thing — which is what the new sweep, and any future tool that
  walks that directory, assumes.

This is worth deciding now precisely because §4's mechanism walks
`docs/reports/`.

### 5.4 `CT10`'s reason row — the one `deferred` verdict

`WRITE-LEND-TASKS` §12.5 and §8 supply CT10's **concurrency** half (two
independent write authorities over one location, requested concurrently,
refusing at `Q600` when they share a byte) and state the honest remainder: the
sub-byte half, for which a mapping lane would have to declare a
byte-granularity encoding in its profile. It is explicitly a lane's work, not
the integrator's, and no lane has taken it. Carried as a named item for a
future CT wiring lane rather than dropped — which is the whole point of a
`deferred` verdict having to carry a note.

## 6. Named follow-ons

| Name | What is missing | Discharge condition |
|---|---|---|
| `RECONCILE-SWEEP-DIMENSIONS-UNPINNED` | `dimensions.*` is derived but no test compares the committed values against a regeneration, which is why §3.1's drift stayed green for a wave | a tripwire in `scripts/tests/test_build_matrix_sections.py` comparing the committed `dimensions` to the produced one, flipped deliberately at each close like the `lean_audits` row |
| `RECONCILE-SWEEP-VOCABULARY` | recognition is a heading substring list; a card inventing a seventh spelling contributes zero items, and only the `no_section` complaint makes that visible | a card-report template fixing one required heading, after which the substring list becomes a compatibility shim for the frozen reports |
| `RECONCILE-SWEEP-APPLIEDNESS` | `applied` is a human's word; nothing checks that the named file actually contains the named text | out of reach in general, but a record could optionally carry `evidence: {file, substring}` per item and the tool could check *that* — a real narrowing, deliberately not attempted here rather than half-done |
| `RECONCILE-SWEEP-CLOSE-REPORT` | wave closes write `gate.json` and no README, so a close leaves no account of what it reconciled beyond the numbers | the first close under §4.2's rule writes both |

`--residuals` candidates across the twenty cards (42 names, 38 of them already
in their own card's follow-on list and therefore tracked by the ledger rule):
`CT-PROOF-FOURTH-CASE`, `CT-PROOF-NATIVE-REFINEMENT`, `CT-PROOF-THIRD-CASE`,
`CT03-PROOF-ROW`, `CT37-PROOF-ROW`, `F02-NATIVE-WRITE-RELATION`,
`CT24-CLEANUP-TASK-COMPOSITION`, `CT42-CELL-DERIVED-PREMISE`,
`CT42-CELL-DUP-SOURCE`, `CT42-CELL-PENDING-PHASE`, `CT42-CELL-UDP-QUEUE`,
`CT42-MAP`, `CT25-CT-MAPPING`, `REGION-JOINS-LOAN-ADOPT-ONLY`,
`T08-COMPOSE-GUARD`, `WRITE-LEND-RELEND`, `W29-AFTER-ATTEMPT-DISCARDED`,
`W29-CONTENT-SIZE-UNDECLARED`, `W29-SIZE-SIBLINGS-UNDECLARED`,
`W31-SIZE-SIBLINGS-REMAINDER`, `C03-REACH-FNPTR-PARAM`,
`W4-RECSUM-ALREADY-LANDED`, `W4-RECSUM-CONSTRAINT-4` (discharged, §2.1),
`WRITE-LEND-TASKS-EPOCH`, `WRITE-LEND-TASKS-EXTENT`, `WRITE-LEND-TASKS-F1`,
`WRITE-LEND-TASKS-PRELUDE-FRAGMENT`, `WRITE-LEND-TASKS-VARYING-BOUND`, and the
four `DOCS-COMPACT-*` names this card tracked.

## 7. What was touched

| Path | Change |
|---|---|
| `scripts/reconciliation_sweep.py` | **new** — the sweep, the close-record check, the residual listing |
| `scripts/tests/test_reconciliation_sweep.py` | **new** — 27 tests (Python suite 200 → **227**) |
| `docs/reports/w32-reconcile-sweep/README.md` | **new** — this report |
| `docs/reports/w32-reconcile-sweep/reconciliation-waves-29-31.json` | **new** — 20 cards, 77 verdicts, 7 `no_section` answers |
| `CLAUDE.md` | the close rule (§4.2) |
| `AGENTS.md` | one entry-point pointer to the script |
| `docs/reviews/r0_3/acceptance_matrix.json` | `dimensions.named_followups` 39 → **40**; `DOCS-COMPACT-O` carries passes 4 and 5 and four residual names |
| `compiler/tests/ct_corpus.rs` | one doc comment: proof `2 of 43` → **3 of 43**, with both stalenesses recorded |
| `docs/development/increments/increments README` | the "above the migrated section" clause scoped |
| `docs/development/increments/wave-32.md` | one increment paragraph |

Deliberately **not** touched: every frozen archive (constraint 21), every
gateway, contract, audit and provider file (so no arbiter is implied), every
`.qcore` fixture, `docs/reevaluation/ct_manifest.json`, `docs/QCORE_SPEC_r0_3.md`,
and every standing constraint body except by the appends named above.

## 8. Gate

Run at the card tip on Linux with `$HOME/.elan/bin` on `PATH`. The Lean step is
read from its **text** line per that script's own behaviour
(`GATE-NO-FAIL-FAST` applies to the workspace run).

**Machine load during the measurement, disclosed.** The gate ran while **two
other worktrees were running their own test binaries concurrently** —
`agent-a18bfaba15868e7a9` and `agent-a37c7fabef385fed7`, both confirmed by
their `deps/` process paths. That is exactly the regime standing constraint 16
names, and it is disclosed rather than smoothed over. Both facts, in order, are
in the table's first row.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,510 passed, 1 failed, 8 ignored** across **223** test binaries, exit 101. The one failure is a **constraint-16 row**, re-verified in isolation below; both facts are recorded |
| ↳ constraint-16 re-verification, in isolation | `cargo test -p qcore-verifier-host --test enforced_process_limits -- --test-threads=1` → **12 passed, 0 failed, exit 0** in 4.67s, with no other worktree running. The suite total with that row counted green is **4,511** — the wave-31 close number exactly |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | clean (exit 0) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | exit 0 — **`Passed 125 Lean audits`** |
| `python3 -m unittest discover -s scripts/tests` | **227 tests, OK** (exit 0) — 200 + this card's 27 |

### 8.1 The tripped row, in full

`a_declared_process_limit_stops_the_fixture_from_forking`
(`crates/qcore-verifier-host/tests/enforced_process_limits.rs:389`) asserted
`Io(Broken pipe)` where it expected `Terminated`. This is **constraint 16's
first row**: the `RLIMIT_NPROC` conformance test, which counts every process of
the real UID and is therefore load-sensitive by construction — and it tripped
while two sibling worktrees were running their own test binaries on the same
machine. Re-run alone on a quiet machine it passes, along with the whole
twelve-test binary.

Nothing was weakened to obtain that: no assertion, pin or expectation was
edited, and the row is reported as **tripped under load and green in
isolation** rather than as either one alone. `GATE-NO-FAIL-FAST` did its job
here — under cargo's default the run would have stopped at
`qcore-verifier-host` and the 223-binary total would have been a lower bound
rather than a measurement.

No provider rebuild: `providers/package.py` inputs untouched, `qcore-schemas`
stays 14.0.0, constraint 18 not engaged; the provider row carries at 122.
**No arbiter was run and no corpus artifact was produced or read.**

## 9. Measurement hygiene

`measure/report_paths.py`, `measure/increment_coverage.py`,
`measure/apply_ledger.py`, `measure/build_record.py` and
`measure/sweep-29-32.json` are worktree-private under `measure/` (gitignored) —
never under `target/` and never in the shared session scratchpad.
