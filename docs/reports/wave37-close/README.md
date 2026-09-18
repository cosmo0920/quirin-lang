# WAVE-37-CLOSE — seven cards, three arbiters, nothing unapplied at its merge and five things found anyway — 2026-09-18

Base `c33c7640`, the `DOCS-COMPACT-7` merge, equal to `origin/main` at the
start; the only card running. Record: `reconciliation.json` (77 verdicts,
`verify_applied: true`), `gate.json` (this close's own measurement, with the
Lean count under a key the generator now checks), and this report.

## Headline

Seven cards merged in wave 37, each through its own full gate at the
integration tip: `BASE-TYPE-READER-CENSUS` (`c1814e3c`),
`SCALAR-ROUTE-EXIT-EDGE` (`0b0b4ac1`), `JOINED-PROVENANCE-ROUND-BOUND`
(`797661f3`), `CT-LANE-5` (`e88d9917`), `TASK-ROUTE-TSAN` (`3fb52b8c`),
`W37-CORPUS-CONTRACTS-4` (`2caf7369`) and `DOCS-COMPACT-7` (`c33c7640`). All
**77** integrator-addressed items were given a verdict **measured against the
tree** — the named file read for the proposed text or its effect, never the
card's claim that it landed — and the result differs from wave 36's in one
respect that matters: **nothing was found unapplied at its merge.** Every
amendment a card proposed (5c, 7, 8, 10b, the CT-row and corpus-row sentences,
the `MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION` restatement, every registration
and retirement) is in the tree in the card's words.

What the close found instead is the next layer down, and each is an instance
of a shape `STABILITY-REEVAL` already names:

1. **A standing constraint carrying two false line numbers.** 5c cites
   `legacy_c.rs:3312` and `:5161`, re-measured by the wave-36 close; at
   `c33c7640` the code is at **3325** and **5177**, moved by this wave's own
   first arbiter. Repaired to identifiers with the numbers pinned to the commit
   they were measured at (§7.3).
2. **A standing constraint whose rewrap broke the thing it protected.** The
   `W37-CORPUS-CONTRACTS-4` merge rewrapped constraint 8 "so that no residual
   name breaks at a hyphen", kept `LOADER-ABI-INLINE-FNPTR-DECLARATOR` whole,
   and split the backticked marker `abi-unreadable-declaration` across two
   lines. The `--check` normaliser does not heal that (it keeps the inserted
   space), so the close's claim about the paragraph **failed on the merged
   text** until the whitespace was repaired (§7.2, §9).
3. **A tracker carrying two discharged entries.** `BASE-TYPE-READER-UNMEASURED`
   (its discharge met word for word by the census card; the merge dedup'd the
   card's registration against the wave-36 close's and left the entry open) and
   `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE` (both arms of its discharge taken by
   the wave-35 close and standing in constraint 14). Retired, named and checked
   (§8).
4. **A report calling a name "registered" that no tracker carries.**
   `W37-MPACK-FAMILY-FIRST`, in `W37-CORPUS-CONTRACTS-4` §10 item 9. Nothing
   between a report and the tracker reads a report's own assertion that a name
   is tracked; registered as `REPORT-REGISTERED-CLAIM-UNCHECKED` (§5.1).
5. **Constraint 14's enumeration stopped at wave 35** with the wave-36 entry
   written by the wave-36 close and no wave-37 entry at all. Brought current
   through wave 37 and, for both waves, **measured** — a `git diff` over the
   front-end crate — rather than read off the reports (§6).

Two texts landed at merges differ from their cards' section quotations and
**both differences are the cards' own**, present at each card's own tip: 5c
carries one extra clause, 10b integrates its quotation into the existing
sentence. Recorded, not smoothed (§7.1).

Counts: `named_followups` **216 → 215** (−2 +1); constraints 49 → 49, three
texts touched (5c, 8, 14) and the gate block; CT **46/50**, sanitized-native
**10 of 46**, proof **3 of 46**, unmoved; corpus **88** modules, unmoved; leak
**272**, unmoved; nothing promoted, full language conformance **false**. The
merged-gate row is **replaced** by this close's own measurement (§10).

## 1. Verdict distribution

| Verdict | Count |
|---|---|
| `applied` | 33 |
| `noted` | 38 |
| `superseded` | 4 |
| `deferred` | 2 |
| `declined` | 0 |
| **Total** | **77** |

62 `verify` claims across the 33 `applied` and 4 `superseded` entries and the
one `noted` entry that carries a checkable name; **zero** `unverifiable`
grounds — every `applied` claim this wave names text at a location. The
`noted` count is high for a structural reason the sweep's tokeniser makes
visible: blockquoted proposal bodies, "with:", "Land it in these words", and
one horizontal rule are each items to the tool, and each is verdicted `noted`
with a pointer to the item whose body it is rather than dropped.

### 1.1 Every non-`applied` verdict that carries a decision

* **`superseded` (4).** `base-type-reader-census #6` — "wave 36's nine
  residuals are still unregistered" was true at the card's base and false at
  this one; the wave-36 close registered all nine, re-checked by id here.
  `joined-provenance-round-bound #10`, `w37-corpus-contracts-4 #14` and `#15` —
  each asked for a fold or a regeneration that a later card or this close
  redid under its own provenance.
* **`deferred` (2).** `ct-lane-5 #8` — `CT07-TWO-THREAD-REPLAY-ORDER` can be
  closed by statement, and the card declined to be the one who writes the
  sentence a later reader relies on; so does this close. Recorded as a new
  deferred decision for the user (§11). `task-route-tsan #10` — whether
  `--residuals` should see prose-only open items. Not widened: a widening
  needs a definition of the shape that a test can make red by mutation, and
  none was designed or measured here; recorded as a new deferred decision
  (`SWEEP-RESIDUALS-PROSE-ONLY-ITEMS`, §11).
* **`declined` (0).** Nothing a card proposed was refused.

### 1.2 The `noted` entries that are decisions, not bodies

`base-type-reader-census #7` is `applied` rather than `noted` — the second
measured value of `RECORD-BODY-TOTAL-OR-REJECT` now stands in the entry's
`why` — but the *disposition* it asks to revisit is a wave-38 cut decision and
is deliberately not changed by a close. `w37-corpus-contracts-4 #12` (the
`mpack/` family) is `noted` and carries the false-"registered" finding (§5.1).
`joined-provenance-round-bound #5` (the 1,272 → 1,273 transcription) is
`noted` after a grep: no tracker or brief carries the wrong figure.

## 2. The three integrator-only items the brief assigned to the close

* **Constraint 14** (`ct-lane-5 #5`, `task-route-tsan #9`,
  `w37-corpus-contracts-4 #11`, `docs-compact-7 #2`): §6.
* **Record heads** (`docs-compact-7 #4`): §7.4.
* **The merged-gate row and the Lean key** (`docs-compact-7 #1`,
  `joined-provenance-round-bound #10`, `w37-corpus-contracts-4 #15`): §10.

## 3. What `--check` found on this close's own record

The content-verifying arm was run on this record before the gate.json, the
regeneration and the merged-gate row existed, deliberately, so that the pending
claims would be seen failing rather than assumed:

```
reconciliation record incomplete (7):
  remaining-d02: no integrator-addressed section found …
  ct-lane-5#integrator-reconciliation-list#10 verify[1]: … does NOT carry
      'integrator_supplied (docs/reports/wave37-close/gate.json)'
  docs-compact-7#for-the-integrator-and-for-the-close#1 verify[1]: no file at
      docs/reports/wave37-close/gate.json
  joined-provenance-round-bound#…#10 verify[1]: AGENTS.md does NOT carry
      'wave37-close/gate.json'
  joined-provenance-round-bound#…#11, w37-corpus-contracts-4#…#14, #15: same
```

Two things in that list are findings rather than pending work. First,
**`remaining-d02` entered the swept set** because this close changed the
matrix generator under `docs/reports/remaining-d02/` — a directory the tool
treats as a card because a `README.md` lives there. It owes nothing, and the
record says so in `no_section` with the reason (the tool checks that map in
both directions, so a card that *does* have a section cannot be waved away the
same way). Second, **the other 56 claims passed on the first run**, including
the four constraint texts — which is the measurement behind "nothing unapplied
at its merge" in the headline. The constraint-8 claim passed only *because*
the split marker had already been repaired (§7.2); on the merged text it fails,
and that failure was reproduced deliberately (§9).

After the gate: `--check` exit 0, 0 complaints (§13).

## 4. The 21 seam commits, accounted by eye

`--uncarded` returned 21 commits writing no card report. Each is a card's
**own** commit: `TASK-ROUTE-TSAN` 4, `W37-CORPUS-CONTRACTS-4` 3,
`JOINED-PROVENANCE-ROUND-BOUND` 3, `WAVE-36-CLOSE` 3 (reachable only through
`0f6eb0a6`, the merge that opens the range), `DOCS-COMPACT-7` 2, `CT-LANE-5` 2,
`BASE-TYPE-READER-CENSUS` 2, `SCALAR-ROUTE-EXIT-EDGE` 2. 4+3+3+3+2+2+2+2 = 21.
**Zero integrator commits outside merges** — wave 36 had three, wave 35 four.
The whole range `0f6eb0a6^..c33c7640` is 63 commits: 8 merges and 55 branch
commits, every one attributable (the per-card counts are in the record's
`hand_sweep`).

That zero is not clean. Every integrator edit of wave 37 — the compile fix, the
constraint rewraps, the recomputed count lines, the folded Lean figure — lives
**inside a merge commit**, indexed by nothing but the merge message. The
question `SYNC-RULE-TOOLING-COMMIT-SCOPE` asks did not go away; it moved into a
place `--uncarded` cannot see (§11).

## 5. The 34 residual-shaped names, classified one by one

`--residuals` returned **34** names the tracker does not carry. Each was
classified from its own source text.

| Class | Count |
|---|---|
| **Card names**, not residuals | 13 |
| **Discharged**, with the discharging card named | 13 |
| **Rules or prospective names**, not open work | 2 |
| **Deferred decisions**, not followups | 2 |
| **Verdict or bucket labels** | 2 |
| **A named premise of a superseded arm** | 1 |
| **A convention a report falsely called "registered"** | 1 |
| **Total** | **34** |

**Card names (13)** — `CT-PROOF-LEG-2`, `CT-WIRE-LANE-2`, `CT-WIRE-LANE-3`,
`CT01-PROOF-LEG`, `CT03-PROOF-ALPHABET`, `CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`,
`CT46-SENDSYNC`, `EVENT-V1-WINDOWS-AFUNIX`, `REGION-CREDIT-JOINS`,
`REGION-JOINS-DUTIES`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES`. The
tool's known false-positive class, classified the same way by the wave-35 and
wave-36 closes.

**Discharged (13)** — five by **this wave's** cards:
`CT07-TWO-THREAD-REMAP` by `CT-LANE-5` (retired at `e88d9917`);
`JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE` and
`JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED` by
`JOINED-PROVENANCE-ROUND-BOUND` (retired at `797661f3`);
`MAIN-GUARD-UNACTIVATED` and `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, the
wave-36 pair still written in the history that discharged them (the wave-36
close said to expect them). And eight carried from earlier classifications,
re-read rather than copied: `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`
(`JOINED-PROVENANCE-CYCLES`), `CT22-CT-MAPPING` (`CT22-WIRE`),
`JOINED-PROVENANCE-FACTS-NAME-NOTHING` (`JOINED-PROVENANCE-CALL-FRAMES`),
`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` (`SWEEP-BLIND-SPOTS`),
`W4-RECSUM-CONSTRAINT-4` and `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`
(`W32-RECONCILE-SWEEP`), `WAVE-34-CLOSE-UNSWEPT` and
`W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE` (the wave-35 close).

**Rules or prospective names (2)** — `DOCS-COMPACT-5-EXAMPLES-HOST-GATE`
(standing) and `W4-RECSUM-ALREADY-LANDED` (a stale brief its card verified).

**Deferred decisions (2)** — `VERIFIER-HOST-ROW-DISPOSITION` (resolved at wave
35) and `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` (open, §11); neither may be
registered as a followup as well.

**Verdict or bucket labels (2)** — `DECIDABLE-NOW` (card T00's CT bucket) and
`RED-AS-REQUIRED` (the verdict word in `task-route-tsan/artifacts/mutations.json`,
nine times).

**A named premise of a superseded arm (1)** — `SCALAR-ROUTE-POISON-DEPENDENCE`:
`SCALAR-ROUTE-EXIT-EDGE` §7's name for the second premise the *old* value route
rested on (`a_spared_write_leaves_the_fact_standing`,
`no_write_leaves_the_fact_standing`), which the card's repair stops consulting
(`retention_does_not_read_the_unchecked_premise`). Explained by its card, not
open work.

**A convention a report falsely called "registered" (1)** —
`W37-MPACK-FAMILY-FIRST`, §5.1.

**Zero** of the 34 were registered. The one registration this close makes is
its own finding about the last of them.

### 5.1 `W37-MPACK-FAMILY-FIRST`, and what checks a report's word "registered"

`W37-CORPUS-CONTRACTS-4` §10 item 9 ends "(`W37-MPACK-FAMILY-FIRST`,
registered)". The name is in **none** of the 216 `named_followups` entries at
`c33c7640`, checked by id; it is not in the card's own §9 table of ten either.
What it records — `mpack/` is a different library from `msgpack/` and the
pinned table's comment says so at the point of confusion
(`qcore_contract_corpus.rs`, "the corpus's first `mpack` family … two different
libraries vendored") — is a corpus-layout convention already written where a
reader will meet it, with no discharge to state. It is **not registered** here.

The shape is registered: **`REPORT-REGISTERED-CLAIM-UNCHECKED`**. `--check`
verifies text an `applied` verdict names; `--residuals` lists names no tracker
carries; **neither reads a report's own assertion that a name *is* tracked**,
and that assertion is exactly the sentence that makes a reader stop looking.
Discharge, both arms written down: an arm that reads each swept report for a
backticked name followed in the same sentence by "registered" and refuses when
the name is absent from `named_followups`, with a test shown red by mutation;
or a recorded decision that the word carries no weight.

### 5.2 `--residuals` after the close's edits

Re-run after the two retirements and the one registration, as the close rule
requires (a retirement is named, checked, and swept after): **34 → 43**, with
the two name sets diffed under `LC_ALL=C` (private `residuals-before.names` /
`residuals-after.names`). Every movement is accounted:

* **Added (10).** Six historical card names — `CORPUS-REPIN`, `G03-F1`,
  `G03-F2`, `P01-F2`, `REASON-CODES`, `T00-CT-SCHEMA` — because
  `docs/reports/remaining-d02/README.md` entered the swept set when this close
  changed the generator beside it (§3); they are the tool's known card-name
  class. `DOCS-COMPACT-8`, a **future** card this report names (the
  `DOCS-COMPACT-7`-at-wave-36 shape). `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS` and
  `TASK-ROUTE-TSAN-SEGV-PRIORITY`, **deferred decisions** written by this
  close, which must not be registered as followups. And
  `BASE-TYPE-READER-UNMEASURED`, retired here and still written in the report
  that discharged it — the discharged class, the `MAIN-GUARD-UNACTIVATED`
  mechanism the wave-36 close said to expect.
* **Removed (1).** `W37-MPACK-FAMILY-FIRST` — and it went quiet **for the
  wrong reason**: it is not registered, it is *cited* inside
  `REPORT-REGISTERED-CLAIM-UNCHECKED`'s `why`, and the scan reads the whole
  tracker text. The classification (a convention, not a residual) is right; a
  future close must not read the drop as a registration.
* **Absent when expected present (1).** `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`,
  retired here, does not appear either — the same wrong reason: two other
  entries' `why` texts cite it (the matrix carries the string twice and the id
  zero times).

**No third-party name was orphaned** by the retirements. Final state: **43
names, none unclassified** — 20 card names (13 + the six historical + one
future), 14 discharged with the discharging card named, 2 standing rules, 4
deferred decisions, 2 verdict labels, 1 named premise; 20+14+2+4+2+1 = 43.

## 6. Constraint 14 — waves 36 and 37, measured rather than read

The "cards that do not engage this constraint at all" enumeration stopped at
wave 35 plus the wave-36 entry the wave-36 close wrote. Three cards asked for
the wave-37 entry (`CT-LANE-5`, `TASK-ROUTE-TSAN`, `W37-CORPUS-CONTRACTS-4`) and
`TASK-ROUTE-TSAN` §9 item 5 refused to write its own line in front of the gap,
on the ground that adding one wave-37 card ahead of a missing wave-36 entry
"would make the list read as current when it is not". That was the right
refusal and the close honours it by closing the whole gap at once.

**The text appended to the bullet**, after "the `MUTEX-SHARE-CROSS`
distinction, applied rather than restated.":

> And **wave 37's seven of seven** — `BASE-TYPE-READER-CENSUS`,
> `SCALAR-ROUTE-EXIT-EDGE`, `JOINED-PROVENANCE-ROUND-BOUND`, `CT-LANE-5`,
> `TASK-ROUTE-TSAN`, `W37-CORPUS-CONTRACTS-4` and `DOCS-COMPACT-7`, the first
> wave of this ledger with no card in the `+0` family at all: none reserves a
> word, moves the lexer or allocates a `Q` code, and each says so in its own
> report (`JOINED-PROVENANCE-ROUND-BOUND` §12 item 6, `CT-LANE-5` §11 item 4,
> `TASK-ROUTE-TSAN` §6.2, `W37-CORPUS-CONTRACTS-4` §10 item 8, `DOCS-COMPACT-7`
> §11; `SCALAR-ROUTE-EXIT-EDGE` §10 item 4 states the `Q`-space half, its
> refusal being a gateway-internal `Unsupported` description, and
> `BASE-TYPE-READER-CENSUS` is a gateway measurement feature that states
> neither). The wave-37 close therefore **measured** the seam rather than
> reading the reports: `git diff --name-only 0f6eb0a6 c33c7640 --
> crates/qcore-frontend/src/` is empty, so no wave-37 commit touched the lexer,
> the parser or any front-end source. The wave-36 entry above was re-verified
> the same way at that close: over `60958bf0..fa48211a` the only front-end file
> that moved is `source_tasks.rs`, in `WRITE-LEND-TASKS-EXERCISE`'s own two
> commits — the one card the entry places in the `+0` family — and `lexer.rs`
> and `parser.rs` did not move (`docs/reports/wave37-close/README.md` §6).

and the closing sentence's range moved from "Waves 28 through 35" to **"Waves
28 through 37"**.

**Why measured.** Two wave-37 reports carry no constraint-14 sentence at all
(`BASE-TYPE-READER-CENSUS`; `SCALAR-ROUTE-EXIT-EDGE` states only the `Q`-space
half). Citing a section per card, as the brief asks, is therefore possible for
five of seven; for all seven, and for wave 36's re-verification, the census
standard the constraint itself sets is "run rather than assumed", so the close
ran one: the diff of the whole seam over `crates/qcore-frontend/src/` is empty
(rc 0, zero lines), and the diff of the whole wave-36 range names exactly one
file, `source_tasks.rs`, whose two commits are `WRITE-LEND-TASKS-EXERCISE`'s
(`e3a2168a`, `9df2c4b3`) — the card the wave-36 entry already places in the
`+0` family. The wave-36 entry stands as written.

## 7. Constraint-text decisions

### 7.1 Two landed texts that differ from their proposals — both the cards' own

* **5c** (`SCALAR-ROUTE-EXIT-EDGE` §10 item 2). The tree reads "That neighbour
  is now owned by card `SCALAR-ROUTE-EXIT-EDGE` (wave 37), **and "conservatively
  wherever measured" was measured only on positively spelled headers.** The
  value route decides …"; the §10 quotation has no such clause. It is present at
  the card's own tip `da532616:AGENTS.md`, so it is the card's — a boundary
  added in the refusing direction — and the merge carried the card's tree, as
  its message says. Verdict `applied`, the difference recorded.
* **10b** (`JOINED-PROVENANCE-ROUND-BOUND` §12 item 2). The tree integrates the
  quoted paragraph into the existing sentence ("That bound is **this graph's own
  proved bound** …") rather than appending it. The whole 10b paragraph at
  `c33c7640` is byte-identical to the card's tip `0579369d` (`diff` exit 0), and
  item 7 says the card amended it in its own tree. Verdict `applied`.

### 7.2 Constraint 8 — applied in the card's words, one whitespace repair

The `2caf7369` merge message: "both paragraphs rewrapped after the gate,
whitespace only, so that no residual name breaks at a hyphen". The rewrap kept
`LOADER-ABI-INLINE-FNPTR-DECLARATOR` on one line and produced

```
   the declaration is withdrawn at every call site with `abi-unreadable-
   declaration`. The withdrawal is **leaf-neutral**: all eleven rows are same-
   position substitutions and no function's leaf count changed. This is the
```

— the backticked marker string a reader would grep for, and a hyphenated
word, each split at a hyphen. `--check`'s normaliser collapses runs of
whitespace to one space and nothing else, so `abi-unreadable- declaration` does
not match `abi-unreadable-declaration`: the close's claim about this paragraph
**fails on the merged text** (§9 reproduces it). Rewrapped so both are whole,
no word changed; constraint 7's paragraph needed nothing.

### 7.3 Constraint 5c — the line citations, and why they are now pinned

`DOCS-COMPACT-7` §7.3 item 5: "5c has been rewritten in two consecutive waves
and now carries two line-number citations into `legacy_c.rs`. Line citations in
a standing constraint drift; identifiers do not." Measured before acting:

```
legacy_c.rs:3312   },                                   (cited: condition.is_empty())
legacy_c.rs:5161   let instruction = branch_instruction( (cited: the successor drop)
actual: 3325  condition_always_true: condition.is_empty(),
        5177  self.nodes[header].successors = if *condition_always_true {
```

Both had already drifted — by `SCALAR-ROUTE-EXIT-EDGE`, this wave's first
arbiter, which added `loop_exit_edge` handling above them — so the wave-36
close's re-measured numbers were false at this base. The constraint now cites
the two identifiers and gives the lines **as of `c33c7640`**, so the number
names the commit it was measured at and cannot go silently false.

### 7.4 The three record heads

`wave-37.md` carried three head shapes. Normalised, head lines only, bodies
verbatim:

| Before | After |
|---|---|
| ``**Wave 37 · `W37-CORPUS-CONTRACTS-4` (2026-09-18).**`` | ``**`W37-CORPUS-CONTRACTS-4` (2026-09-18).**`` |
| ``**Wave 37 · `JOINED-PROVENANCE-ROUND-BOUND` (2026-09-18).**`` | ``**`JOINED-PROVENANCE-ROUND-BOUND` (2026-09-18).**`` |
| ``## SCALAR-ROUTE-EXIT-EDGE (2026-09-18, wave 37's first arbiter)`` + blank line | ``**`SCALAR-ROUTE-EXIT-EDGE` (2026-09-18, wave 37's first arbiter).**`` joined to the body's first line |

The third kept its parenthetical: wave 36's file carries both `(2026-09-18)`
and `(2026-09-18, wave 36)` heads, and the body of this entry does not restate
"first arbiter", so dropping it would drop a fact. The blank line between the
`##` heading and its body was removed to make the head inline; no body
character changed (`git diff` of the file shows the three head lines and that
one blank line only, beside the close paragraph).

### 7.5 The standing gate's command block

`DOCS-COMPACT-7` §7.3 item 3: the block listed seven commands and every close
runs an eighth. Decided and landed: `python3 -m unittest discover -s
scripts/tests` is now in the block, with a sentence saying it runs **after** any
matrix regeneration because `CommittedMatrixIsSelfConsistent` reads the
committed file.

## 8. Retirements — two, each named and checked first

* **`BASE-TYPE-READER-UNMEASURED`** — registered by the wave-36 close (source
  `c-gateway-indirect-census-2`), discharge "instrument the base-type reader
  the same way … and publish the sole-blocked reach of repairing each, BEFORE
  any card is cut on it". `BASE-TYPE-READER-CENSUS` did exactly that
  (`c1814e3c`; its §10 says "DISCHARGED by this card") and registered the name
  "with its discharge"; the merge dedup'd that against the existing entry
  (187 + 5 = 192, not 193) and the entry stayed open. Retired.
* **`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`** — registered by
  `SWEEP-BLIND-SPOTS`, discharge "add the three cards' census rows, or narrow
  the exhaustiveness claim to the cards actually enumerated". The wave-35 close
  took both arms, and constraint 14's own text says so; §6 brings the same
  enumeration current through wave 37. Retired.

`named_followups` **216 → 215**: −2, +1 (`REPORT-REGISTERED-CLAIM-UNCHECKED`).
Applied by editing the carried array and regenerating through
`build_matrix.py --gate docs/reports/wave37-close/gate.json`, never by hand-setting
the derived count.

## 9. The integrator's own record this wave, unsmoothed

Each of these is step-3 material for `STABILITY-REEVAL` #3 — a semantic-adjacent
edit no card owns — and each is cited to the merge commit that carries it.

1. **A compile failure at integration, fixed by one fixture line**
   (`797661f3`). `JOINED-PROVENANCE-ROUND-BOUND`'s new test fixture
   `long_cyclic_cfg` built a `LegacyCfgNode` without the `loop_exit_edge` field
   `SCALAR-ROUTE-EXIT-EDGE` (merged first) had added; both branches green on
   their own bases, no textual conflict, first integration gate red. Fixed with
   `loop_exit_edge: None,` — the value the file's sibling fixtures carry — and
   recorded in the merge message. **Verdict: fully discharged by the message and
   the integration gate; no residual name.** No semantics moved: the field is a
   test initializer, the theorem `the_exit_edge_is_never_removed` reads no
   fixture, and the gate that caught it is the mechanism working. What it *is*
   evidence of is that "both branches green alone" is not a property of the
   merge, which the failure-gated chain already assumes.
2. **Two text-wrapping incidents at `2caf7369`.** One caught before push (a
   residual name split at a hyphen, rewrapped). One **not caught**: the rewrap
   split `abi-unreadable-declaration` instead (§7.2). Found here by the
   `--check` arm's failure on the close's own claim, and repaired. **Verdict:
   discharged by the repair; the mechanism that found it is the one wave 36
   built, and the lesson is that a rewrap "so that no residual name breaks" is
   a claim about uppercase names and nothing else.**
3. **A merge that committed itself with a placeholder message** (`c33c7640`,
   the compaction merge), collapsed by `reset --soft` + `--amend` before the
   gate. Verified: `git log --format=%p` shows two parents (`2caf7369`
   `a2ee7227`) and one commit; the message is the one the gate is reported
   against. **Verdict: discharged by verification.**
4. **A merge script's `checkout --ours AGENTS.md` step**, which would have
   dropped a card's auto-merged constraint amendment, found and replaced before
   it bit. Verified at `c33c7640`: 5c, 10b, 7 and 8 are all present in their
   cards' words (the `--check` claims of §3). **Verdict: no text lost; no
   residual.** The failure it *would* have caused is the one wave 36 found four
   times, so the script step is worth the sentence.
5. **`W37-CORPUS-CONTRACTS-4` rebased mid-wave** onto `797661f3` with BEFORE and
   AFTER re-run there (its §1/§5/§8; merge message with digests), which
   independently confirmed both code arbiters' zero movement from outside them.
   **Verdict: a strength, recorded as one.**
6. **This close's own:** the merged-gate row's Lean figure had drifted for two
   waves because a close's `gate.json` had no key to hold it. This close's
   `gate.json` carries `lean_audits`, and — because the generator's `--gate`
   refused any key outside its nine, so the brief's two demands (a `lean_audits`
   key *and* a regeneration with this close's gate) could not both hold — the
   generator now takes `lean_audits` as an optional **checked** key: accepted
   when it equals the `proof/Audit*.lean` count on disk, refused otherwise,
   never copied into the published block. Two tests; the refusal test shown red
   by mutating the comparison to `False` (`AssertionError: 0 == 0`, exit 1)
   and green after the restore (private logs `matrix-mutation.log`,
   `matrix-tests-2.log`). The drift is now a refusal where it would have been
   written.

## 10. The merged-gate row, re-measured

`DOCS-COMPACT-7` measured the row at `2caf7369` and said the close should
replace it with its own `--gate` numbers. Done: the row in `AGENTS.md` now
reads this close's measurement at its own base, with its own provenance, and
the Lean count is in `gate.json` under a key the generator checks (§9 item 6).
Figures in §13.

## 11. Deferred decisions

Ten entries carried from the wave-36 close, re-listed with grounds in
`reconciliation.json` → `deferred_decisions.items`; two new. In brief:

| Name | Verdict | Ground (short) |
|---|---|---|
| `CT10-REASON-ROW` | deferred | fifth wave; new evidence `CT10-BYTE-ENCODING-UNSOUND` changes what any row would say |
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | deferred | regenerated at seven merges and this close; `updated` still wave-27 prose |
| `EXAMPLES-STYLE-SLICE-FANOUT` | deferred | no wave-37 card touched `examples/`; no new evidence |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | deferred | new shape: zero integrator commits outside merges, every integrator edit inside one (§4) |
| `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` | deferred | `TASK-ROUTE-TSAN-WINDOWS-APPLE-RUN` registered; more partially tracked |
| `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` | deferred | data point: `DOCS-COMPACT-7` did both, and this close found two more corrections it did not |
| `VERIFIER-HOST-ROW-DISPOSITION` | resolved | at wave 35; carried for accounting |
| `items` | withdrawn | reader artifact; carried for accounting |
| `INDIRECT-CENSUS-FEATURE-UNBUILT` | deferred | **restated**: sibling `provenance-rounds-census` (round-bound card) and a second payer (`BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT`); three cards' instrumentation behind two features nothing builds |
| `TASK-ROUTE-TSAN-SEGV-PRIORITY` | **resolved by the tree** | `TASK-ROUTE-TSAN` merged at `3fb52b8c`; `TASK-ROUTE-TSAN-SEGV` retired there |
| `CT07-TWO-THREAD-REPLAY-ORDER` | **new, deferred** | closable by statement; the user's call (`CT-LANE-5` §11 item 7) |
| `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS` | **new, deferred** | whether `--residuals` should see prose-only open items (`TASK-ROUTE-TSAN` §9 item 6) |

## 12. Carried to wave 38

* **`STABILITY-REEVAL` #3 is due now** (CLAUDE.md's every-2-waves rule; #2 ran
  in wave 36). Not run here. Nominated grounds carried from wave 36 (the corpus
  artifacts pinned by no test; the remaining Lean docstrings; the logged-line
  comparison hazard), plus this wave's step-3 material (§9) and two new
  kind-1 candidates: the `checked_gate_keys` refusal (driven by a test, red by
  mutation) and the constraint-14 enumeration, which is now a measured claim
  about a `git diff` and should be re-measured, not re-read, next time.
* **`DOCS-COMPACT-8`** is due around wave 39 (CLAUDE.md's roster).
* **A wave-38 gateway cut** should read `BASE-TYPE-READER-CENSUS` §11.3 and
  `RECORD-BODY-TOTAL-OR-REJECT`'s second measured value before choosing;
  `JOINED-PROVENANCE-ROUND-BOUND` §12 item 5's note (a second consumer of the
  relation inherits ten un-exercised sites) belongs in that card's brief.
* `CT-FIXTURE-EXTERNAL-READERS` asks for a close-rule or test-level check, not
  a card; not built here.

## 13. Gate

Full standing per-change gate, run **detached** (`setsid -f`, no `trap ''
TERM`) at this close's own base — `c33c7640` plus the documentation-only
skeleton commit `6a330b31`, so the cargo, fmt, clippy, probe and Lean steps
measured `c33c7640`'s code byte for byte. Every step's exit was written to its
own file under the worktree-private `private/wave37-close/gate-base/`
(gitignored, never under `target/`, never in the shared scratchpad); no pipe in
any gate command. `git submodule update --init --recursive` ran first and
checked out `third_party/libevent` at `d82464a2`, the pinned revision. Disk 310
GB free at start, 266 GB at end. Started 04:27:39Z, cargo test 04:27:45Z →
04:44:25Z (**16m40s**), finished 04:45:55Z (**18m16s** wall).

| Step | Exit | Figure |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `d82464a2` |
| `cargo test --workspace --no-fail-fast` | 0 | **4,637 passed / 0 failed / 8 ignored**, 233 summaries over 231 targets (222 `Running` + 9 doc-test sections; `qcore_core` and `qcore_verifier_host` doc-tests print two each); `test result: FAILED` 0 times |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` | 0 | — |
| `reevaluation_followup` | 0 | — |
| `scripts/check_lean_audits.py` | 0 | **137 Lean audits** |
| `python3 -m unittest discover -s scripts/tests` (detached run) | **1** | `Ran 295 tests`, 1 failure: `test_each_derived_dimension_matches_the_section_it_counts`, `216 != 215` |
| the same, re-run after the regeneration (`python-2.exit`) | 0 | **`Ran 295 tests`, OK** |

The one red row is `D02-DIMENSIONS-FOLLOWUP-COUNT`'s tripwire doing its job:
the detached run's Python step ran last, against a working tree in which this
close had already hand-edited `named_followups` (216 → 215) and not yet
regenerated, and the message says what to do ("regenerate through
build_matrix.py rather than repairing the count by hand"). The repair was
`build_matrix.py . --gate docs/reports/wave37-close/gate.json` (exit 0; the
checked `lean_audits = 137` accepted against 137 files on disk; `named
followups 215`; `contract modules 88`), and the recorded Python figure is the
re-run after it. This is the identical shape the wave-36 close recorded (`171
!= 187`), and it is reported rather than smoothed. A loose `grep -c '^test
result'` on the log reads **235**, because two test *names* begin `result_` —
the wave-36 close's own trap; the 233 above attributes every summary line to
its section.

Constraint 16 rows, stated because a gate report must say which it saw: all
five named rows living in this workspace were green, the unnamed seventh
(`a_forced_override_never_weakens_an_execution`) too; a full-suite run on a
machine carrying its own build, **not** the quiet-host number
`HOST-ROW-STARVATION-DESCRIPTION` asks for. Provider **122 is carried**: nothing
under `providers/` or `crates/qcore-schemas/` moved across the wave
(`git diff --name-only 477b733d HEAD -- providers/ crates/qcore-schemas/` is
empty) and `qcore-schemas` is 14.0.0, so constraint 18 is not engaged; a Unix
figure. The figures reproduce `DOCS-COMPACT-7`'s at `2caf7369` exactly
(4,637 / 0 / 8, 137 Lean), as they should — no Rust moved between that tip and
this base. Deltas against the wave-36 close's 4,611 / 293 / 135: workspace
+26 (the wave's own tests; this close adds none), Lean +2 (one audit per code
arbiter), Python +2 (this close's checked-gate-key tests).

`--check` on this record after the gate: **exit 0** — 62 content claims (50
present, 12 absent) verified across 6 files, 33 of 33 `applied` verdicts with
checkable text, 0 declared unverifiable.

Nothing is promoted by this close: no corpus count, unit status or function
status moves; raw units stay unpromoted; the contract corpus stands at 88; CT
coverage at 46/50 with sanitized-native 10 and proof 3; full language
conformance **false**.

## 14. Reproduction

```sh
python3 scripts/reconciliation_sweep.py . --since 0f6eb0a6
python3 scripts/reconciliation_sweep.py . --since 0f6eb0a6 --residuals
python3 scripts/reconciliation_sweep.py . --since 0f6eb0a6 --uncarded
python3 scripts/reconciliation_sweep.py . --cards base-type-reader-census scalar-route-exit-edge joined-provenance-round-bound ct-lane-5 task-route-tsan w37-corpus-contracts-4 docs-compact-7 --residuals
python3 docs/reports/wave37-close/scripts/build_reconciliation.py
python3 scripts/reconciliation_sweep.py . --since 0f6eb0a6 --check docs/reports/wave37-close/reconciliation.json
python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave37-close/gate.json
python3 -m unittest discover -s scripts/tests
git diff --name-only 0f6eb0a6 c33c7640 -- crates/qcore-frontend/src/      # empty
git diff --name-only 60958bf0 fa48211a -- crates/qcore-frontend/src/      # source_tasks.rs only
```
