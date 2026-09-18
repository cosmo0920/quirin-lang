# DOCS-COMPACT-7 — the seventh firing of the three-file compaction cadence

**Card:** `DOCS-COMPACT-7`, wave 37. **Base:** `2caf7369`, the final pre-close
wave-37 `main`. **Scope:** `AGENTS.md`, `CLAUDE.md`, `README.md`, plus verbatim
moves into `docs/development/`. Documentation only — no crate source, no
`proof/`, no frozen archive (constraint 21).

**This card was written twice.** Its first pass ran against `477b733d`, the
wave-37 open. The integrator held it to merge **last**, because a compaction
union-merged against rows other cards are rewriting loses meaning. In the
meantime the wave-36 close and six wave-37 cards landed, rewriting six standing
constraints and every ledger row this card compacts. The first pass was
therefore **discarded rather than rebased**: a `git rebase` conflicted on the
same regions, and resolving those as text would have been exactly the
union-merge the hold existed to prevent. Every number below is re-read from its
source **at `2caf7369`**, and no figure from the first pass is carried. §7
records what the rebase changed about the card's own conclusions — including one
correction it no longer needs to make, because the close made it first.

**A note on the two kinds of edit.** `CLAUDE.md`'s cadence rule expects
single-digit percentages and says the rule exists to bound accretion of
*narrative*, not to promise a shrinking file. This report separates
**compaction** bytes (narrative removed, the fact preserved in history or in a
named authority) from **correction** bytes (a statement false about the tree,
repaired, which costs words), so that if the integrator decides correction work
does not belong to this cadence the two can be split cleanly. §3 gives it line
by line.

---

## 1. Numbers verified, and the source each was read from

Every figure re-read **from its source and never from the file being compacted**,
at `2caf7369`.

| Figure | Value | Source |
|---|---|---|
| CT mapped | **46 of 50**; 4 unmapped (CT10, CT24, CT41, CT42) | `docs/reevaluation/ct_manifest.json`, `mapping_status` |
| CT sanitized-native | **10 of 46** — CT03, CT07, CT09, CT17, CT22, CT25, CT26, CT34, CT46, CT49 | same, `stages.native_sanitized.status == "executed"` |
| CT proof | **3 of 46** — CT01, CT03, CT37 | same, `stages.proof.status == "executed"` |
| Contract corpus | **88** modules | `contract/qcore` on disk; pinned at 88 in `compiler/tests/qcore_contract_corpus.rs` |
| Manifest-selected | **33** | pinned in `crates/qcore-c-gateway/tests/neighbours.rs` |
| resource-v1 declarations | **59** | `docs/reports/w37-corpus-contracts-4/README.md` headline |
| Lean audits | **137** | `proof/Audit*.lean` on disk; `check_lean_audits.py` globs exactly that |
| `named_followups` | **213** at base; **216** after this card | `docs/reviews/r0_3/acceptance_matrix.json`; `dimensions` re-derived by the generator |
| Corpus units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only, **zero moves at all three wave-37 arbiters** | `docs/reports/w37-corpus-contracts-4/README.md` §1; `scalar-route-exit-edge` and `joined-provenance-round-bound` §5 |
| Corpus functions | 205 → **213 Safe**, 84 provisional Unsafe unmoved, 4,545 → **4,537 Unknown**, of 4,834 | `w37-corpus-contracts-4` §1 |
| Diagnostics | leak **272** · `unknown_alias` **4,309** · `double_release` **0** · `unsupported` **49,753** · `summary_conflict` **1** | same |
| Leaf movement | **183 removed, 15 added**, all 15 same-position substitutions | `w37-corpus-contracts-4` §5 |
| Loop-header reach | **0 of 201** `BranchOnNull` nodes take either arm; 89.6% untracked | `docs/reports/scalar-route-exit-edge/README.md` |
| Arbiter chain | **12** links, **11** archives | the reports' own `artifacts/` directories |
| Merged gate | §6 | measured by this card at its own tip |

### 1.1 One figure that does not survive its source — for the second time

`AGENTS.md`'s merged-gate row publishes **137 Lean audits** and attributes the
row to **the wave-36 close**. That close measured **135** — stated in its README
gate table ("exit 0 — **135 audits**") and twice in its own `gate.json` `note`
("135 Lean audits"; "Lean 132 → 135 (+3)"). 137 is the count *after* wave 37's
`JOINED-PROVENANCE-ROUND-BOUND` and `SCALAR-ROUTE-EXIT-EDGE` each added one.

This is **the same field wrong the same way in two consecutive waves**. At this
card's first base the row read "135 Lean audits" attributed to the *wave-35*
close, which measured **132**. Both times every other figure in the row matched
its named source exactly; both times the Lean figure was the current file count
wearing an older close's label.

The mechanism is visible, which is why this is registered as its own residual
rather than another instance of `INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED`:
`docs/reports/wave36-close/gate.json` has **nine keys and none is a Lean count**.
The one figure in the row with no machine-readable source in the record it cites
is exactly the one that has drifted twice. Registered as
**`GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`** (§5).

---

## 2. Constraint inventory, before and after

**Count 49 → 49. Labels identical and in the same order. Group headings
identical and in the same order.** Measured by
`docs/reports/docs-compact-7/scripts/constraints.py`.

The new base has **more** constraint text than this card's first base — 107,041
bytes against 97,598, the wave having rewritten six constraints — but the same
**49** labels, so the count figure is unchanged and nothing was dropped to reach
it. What the wave itself did, for the record:

| # | at `477b733d` | at `2caf7369` | Δ | by |
|---|---|---|---|---|
| 5c | 3,674 | 5,686 | +2,012 | wave-36 close (`FOREVER-FOR-EXIT-EDGE-UNOWNED`) + `SCALAR-ROUTE-EXIT-EDGE` |
| 7 | 4,633 | 5,418 | +785 | wave-36 close, `W37-CORPUS-CONTRACTS-4` |
| 8 | 2,151 | 3,728 | +1,577 | wave-36 close, `W37-CORPUS-CONTRACTS-4` |
| 10b | 3,248 | 4,354 | +1,106 | `JOINED-PROVENANCE-ROUND-BOUND` |
| 14 | 13,064 | 14,341 | +1,277 | wave-36 close |
| 16 | 4,204 | 6,890 | +2,686 | wave-36 close (`LOAD-SENSITIVE-ROW-UNNAMED`) |

**After this card: 47 of 49 constraints are byte-identical with the new base.**
All six the wave rewrote are **untouched**, verified by hash. Two changed:

| # | before | after | Δ | why |
|---|---|---|---|---|
| 23 | 1,562 | 1,528 | −34 | compaction — a self-disclaimed stale checkpoint removed (§2.2) |
| 25 | 1,069 | 1,060 | −9 | correction — corpus 76 → 88 (§2.3) |

The constraints section as a whole: 107,041 → 106,998 (−43). **This card is
almost entirely a ledger compaction, not a constraint edit** — which is the
right shape for it, and a cleaner one than its first pass, which also touched 5c.

### 2.1 The wave's constraint text is preserved exactly

| Constraint | what the wave did | after this card |
|---|---|---|
| 5c | amended twice — the `for (;;)` correction, then `SCALAR-ROUTE-EXIT-EDGE`'s exit-edge rule | **byte-identical** (`3ab225c1912a9f74`) |
| 7 | leak-pin chain appended | **byte-identical** (`a5bd326473579078`) |
| 8 | mode-gate arm appended | **byte-identical** (`a8cdc68861054af6`) |
| 10b | round bound replaced the 64-round cap | **byte-identical** (`bcceaa7ab94660bb`) |
| 14 | census ledger extended | **byte-identical** (`9c0cfed7a39b03ca`) |
| 16 | seventh row recorded as unnamed | **byte-identical** (`1b5631321dde2ba6`) |
| 15h, 15n, 15o, 10a | unchanged this wave | **byte-identical** |

Constraint 14 is untouched, so the integrator's instruction about its "does not
engage" enumeration stopping at wave 35 needs no action from this card — but the
observation is carried to the close in §7.3.

### 2.2 Constraint 23's stale parenthetical

Constraint 23 carried "(**9 of 45 and 3 of 45** at the wave-34 close — read them
there, not here; this parenthetical is a checkpoint that has been stale
before…)". The numbers were stale again at this base (the row reads 10 of 46 and
3 of 46). Rather than re-date a checkpoint that has now gone stale twice and
disclaims its own authority in the same breath, the numbers are **removed** and
the rule kept: read them in the ledger row and **never copy them here**. Nothing
is lost — the paragraph itself named the row as the authority.

### 2.3 Constraint 25's corpus figure

"beside a live corpus (76, unmoved since wave 31)" → "(88 since wave 37)". Read
from `contract/qcore` on disk and from the pin in
`compiler/tests/qcore_contract_corpus.rs`. The `D02-B00-LEDGER-SNAPSHOT` record
this figure exists to contrast with is untouched.

---

## 3. Byte counts per file, split compaction vs correction

Against the rebase base `2caf7369`.

| File | before | after | Δ | % |
|---|---|---|---|---|
| `AGENTS.md` | 173,199 | *see §6* | | |
| `CLAUDE.md` | 22,170 | 22,999 | +829 | +3.74% |
| `README.md` | 11,970 | 13,178 | +1,208 | +10.09% |

### 3.1 Every edit, line by line

| # | File / region | Δ | Kind | Justification |
|---|---|---|---|---|
| 1 | `AGENTS.md` headline paragraph | −544 | compaction | Per-link prose dropped; the **chain list is preserved in full** (12 links, newest first), as are the byte-identity rule, the archiving-tool assertion and the two-coordinates sentence. The two link *kinds* the wave introduced — declarative and zero-movement — are kept as definitions rather than as three retellings. **Corrects** "those seven" against a list of twelve, and states that **eleven** of the twelve published an archive (`joined-provenance-round-bound` published none, having moved nothing), which the old text's count could not express. |
| 2 | `AGENTS.md` *Cycle 19* + *CT* + *Contract corpus* rows | −8,814 | compaction + correction | The per-arbiter delta narrative (waves 31–37) and the waves-28-to-37 mapping chronicle removed — each already a paraphrase of two other authorities the rows themselves named. Kept: every current count, the leak pin's pointer to constraint 7, the coverage boundaries, `input_chunk_get`, the proof-leg method, `PROOF_GATED_CASES`, constraint 21's supersession rule, the four unmapped cases' current grounds, both contract pins and the declaration-choosing method. **Three corrections inside**, §4.2–§4.4. |
| 3 | `AGENTS.md` follow-on count paragraph | −854 | compaction + correction | The per-merge changelog removed to the increment records. **Corrects** a live internal inconsistency: the paragraph opened "**213** entries at the wave-36 close" and then derived "171 → 189 → 190 → **187**" — 213 is the current figure, 187 was the close's, and 26 entries had been added since without the chain being updated. The operative rule is kept in full, and the `171 != 187` tripwire the close hit is kept as the reason the rule exists. |
| 4 | `AGENTS.md` *Merged gate* row | §6 | compaction + correction | §1.1 and §6. |
| 5 | `AGENTS.md` constraint 23 | −34 | compaction | §2.2. |
| 6 | `AGENTS.md` constraint 25 | −9 | correction | §2.3. |
| 7 | `AGENTS.md` gate section, submodule comment | +23 | correction | "once per checkout" is false for a delegated worktree, which is a fresh checkout every time; the doc half of `WORKTREE-SUBMODULE-UNGATED`. |
| 8 | `AGENTS.md` gate section, "five load-sensitive rows" | −5 | correction | Constraint 16 now carries six named rows plus an unnamed seventh (`LOAD-SENSITIVE-ROW-UNNAMED`); the count is dropped rather than re-pinned, since it is the constraint's to state. |
| 9 | `AGENTS.md` gate section, new paragraph | +593 | preservation | Not new rules. `GATE-EXIT-SWALLOWED-BY-PIPE`'s ground lived **only** in the merged-gate row this card compacts, so dropping the row without rehoming it would have been deletion. Both residuals are named as registered with their mechanical discharge **open**. See §7.2. |
| 10 | `AGENTS.md` Q-space note | +895 | correction | §4.1. |
| 11 | `CLAUDE.md` cadence roster | +80 | compaction (net grew) | The dated roster of six firings replaced by a pointer to the seven reports; the **dated roster moves verbatim** to the wave-37 record. Grew because naming seven directories costs more than six dates — what it removes is the accretion that would have added a seventh date this wave and an eighth next. |
| 12 | `CLAUDE.md` `STABILITY-REEVAL` pointer | +187 | correction | Said "#1 ran in wave 34 … next due after wave 35". #2 ran in **wave 36**, and #3 is **due now**, after wave 37, by that card's own §6 item 7 — which also names the next firing's highest-value ground. |
| 13 | `CLAUDE.md` main-guard bullet | +561 | correction | §5.2 — a measured false positive in the mechanism this bullet describes, re-verified at `2caf7369`. |
| 14 | `README.md` `std.concurrency` row | +183 | correction | The declared mutex crossing was absent from the capability table entirely. |
| 15 | `README.md` `std.buffer` row | +174 | correction | "no worker operation exercises it yet" was falsified in wave 36. |
| 16 | `README.md` boundary paragraph | +236 | correction | Adds the crossing's own boundary in constraint 15o's words — it witnesses a thread boundary and says **nothing** about contention, a data race or the C memory model, and no TSan row exists in this repository — and narrows "general task/mutex composition remains open" to the measured fact behind it. |
| 17 | `README.md` corpus paragraph | +250 | correction | Adds the corpus **baseline pointer**, re-derived at this tip as `docs/reports/w37-corpus-contracts-4/artifacts/`, and the one sentence that makes the chain mean something to a human reader. |
| 18 | `README.md` CT counts | +1 | correction | 45/50 → **46/50**, 5 → 4 unmapped, 9 → **10** sanitized. |
| 19 | `README.md` main guard | +363 | correction | The guard exists, is now **active**, and a human contributor could not discover it from `README.md`. Stated with both boundaries `CLAUDE.md` insists on: inert in a fresh clone until installed, bypassable by intent, a guard against a mistake and not against an attacker. |

---

## 4. Corrections, each with its source

### 4.1 The Q-space note's false universal about wave 32

**MEASURED, and re-measured unchanged on the new base.** The note states *"Waves
32–34 allocated no code and ran no condition extension either, so the count
before it stood at thirteen"*, and calls `WRITE-LEND-TASKS-EXERCISE` the
fourteenth. The same file's own diagnostics table contradicts it twice — the
`Q600` row ("`MUTEX-SHARE-CROSS` (wave 32) is the fifth, widening `Q600`'s
boundary condition and the `Q610`–`Q627` sub-band's route") and the
`Q610`–`Q627` row ("reaches `Q611` … from a **new route**"). A wave-32 card did
run the consequence; the universal was false and the tally one short. Nobody had
corrected it in the intervening wave.

This is the **same shape** as `CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`, which
`SWEEP-BLIND-SPOTS` measured in the neighbouring constraint-14 bullet, about the
same wave — and the wave-35 close's handling of that one is the precedent
followed here: narrow the quantifier to the waves actually enumerated **and**
give the missing card its row, because replacing a false universal with a false
member would be no better.

Re-derived from the note's own enumeration plus those two rows: **fifteen
firings, ten bands**, `MUTEX-SHARE-CROSS` the fourteenth and the one that took
the tenth band. Waves 33, 34 **and 37** ran none — wave 37 verified here, its
gateway arbiter `SCALAR-ROUTE-EXIT-EDGE` moving gateway diagnostics, which are
categories rather than `Q` codes. The icebox has held twelve waves, not eleven.

Nothing derives this tally mechanically. Registered as
**`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`**.

### 4.2 CT row — CT46's stages

The row still said the `checked_mutex_crossing` verdict mode "is why four stages
read `unassessed` and none reads `executed`". Read from the manifest at this
tip, CT46's `c_emission`, `native_execution`, `native_sanitized` and
`mutex_reference_execution` all read **`executed`**: wave 36's
`MUTEX-SHARE-CROSS-EMISSION` landed the emission and took sanitized-native from
9 to 10. The row now records the boundary as **retired** and names the card that
retired it.

### 4.3 CT row — the forward question

The row still said CT03's forward question "stays open as
`TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`". Constraint 15n's wave-36 appendix says
in bold that `WRITE-LEND-TASKS-EXERCISE` **DECIDES** it. One file, two
statements, one wrong. The row now defers to 15n and does not restate the
decision.

### 4.4 The arbiter chain's length

"All earlier archives remain frozen (constraint 21): **those seven** plus …"
against a list of **twelve** links. The chain grew by two in wave 36 and three
in wave 37 and the count was never updated. Corrected — and sharpened, because
twelve links published **eleven** archives: `joined-provenance-round-bound`
published none, having reproduced its predecessor byte for byte.

---

## 5. Names registered

`named_followups` **213 → 216**, appended by
`docs/reports/docs-compact-7/scripts/add_followups.py` (idempotent by id) and
`dimensions.named_followups` re-derived by
`docs/reports/remaining-d02/build_matrix.py`, which reports `named followups
216`, `lean audits 137`, `contract modules 88`.

### 5.1 Two registrations withdrawn — the close got there first

This card's first pass registered `W36-COVER-HIDDEN-BLOCKER` and
`CJSON-DELETE-NESTED-DETACH`, both cited in the ledger and tracked nowhere. The
**wave-36 close registered both** in the meantime, as part of sweeping all nine
of `W36-CORPUS-CONTRACTS-3`'s §9 residuals. They are therefore **dropped, not
re-registered**: the close reached them first and its entries stand. The finding
was correct when made and is now discharged by someone else, which is the right
outcome. Recorded here rather than re-asserted in the matrix.

Likewise `FOREVER-FOR-EXIT-EDGE-UNOWNED`, which the first pass would have
registered had `STABILITY-REEVAL` #2 not already done so.

### 5.2 Three registered

| Name | Ground |
|---|---|
| `Q-SPACE-EXTENSION-TALLY-HAND-KEPT` | §4.1. The residual is the **method**: the tally is prose, nothing derives it, it has been wrong once, and its two halves live in two places no check compares. |
| `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` | §1.1. Measured twice, in consecutive waves, on the same field, with a visible mechanism — the close's `gate.json` schema cannot hold the figure the row publishes. |
| `MAIN-GUARD-REFUSES-PACK-REFS` | §5.4. A measured false positive in the mechanism `CLAUDE.md` describes. |

### 5.3 Retirements — named explicitly, as the brief requires

**This card retires nothing.** No entry is removed, renamed or marked
discharged. Four entries have a *documentation half* landed here and **stay
open** under their own names — `WORKTREE-SUBMODULE-UNGATED`,
`GATE-EXIT-SWALLOWED-BY-PIPE`, `LOAD-SENSITIVE-ROW-UNNAMED` (cited, not
discharged) and `INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED` (whose hazard
§1.1 shows arriving as a live defect for the second time) — because in each case
the mechanical arm is untaken. Landing half a discharge is not a retirement, and
the count moves in one direction only: **213 → 216**.

### 5.4 `MAIN-GUARD-REFUSES-PACK-REFS`

Measured incidentally while committing, on both bases. `git pack-refs --all`
from this worktree is refused by the guard, and `.git/qcore-main-guard.log`
records the transaction as `refs/heads/main 0000…0000 -> <the ref's current
value>` — old and new naming the **same** value, so nothing was being advanced.
`pack-refs` is what git's automatic `gc` runs, so a delegated card sees a
successful commit followed by `error: task 'gc' failed` under the guard's
banner.

Not a soundness problem — the guard fails closed and no commit is lost. It is a
false positive with two costs: repository maintenance is broken in every
delegated worktree, and a card is shown an alarming banner for an operation that
was never a main update, which trains readers to ignore it. This card owns no
hook, so only the documentation half is landed.

### 5.5 The sweep

`docs/reports/docs-compact-7/scripts/residuals.py` over the three files after
this card: **181** residual-shaped names, **12** neither a `named_followups` id
nor mentioned anywhere in the matrix. Ten are **card names**, not residuals
(`CT01-PROOF-LEG`, `CT03-PROOF-ALPHABET`, `CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`,
`CT-PROOF-LEG-2`, `CT-WIRE-LANE-3`, `REGION-CREDIT-JOINS`,
`REGION-JOINS-DUTIES`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES`), each
with its own report directory. The other two are both **correctly** absent:

* `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` — the wave-35 close dispositioned
  it "answered, not registered" (`docs/reports/wave35-close/README.md` §4), and
  constraint 5c records the same. Registering it would reopen a decision already
  taken.
* `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED` — **retired** by the wave-36
  close, discharged by its own `--check` content arm, and still named in
  `CLAUDE.md` as the reason that arm exists. This is precisely the case the
  close's new retirement rule predicts in its own words: "Expect the retired
  names themselves to appear in that later run: a discharged name stays written
  in the history that discharged it." The sweep finding it is the rule working,
  not a defect.

---

## 6. The merged-gate row, and this card's gate

*(filled in below)*

---

## 7. What the rebase changed, and three decisions

### 7.1 One correction this card no longer needs to make

Its first pass landed `STABILITY-REEVAL` #2's constraint-5c amendment — the
finding that 5c asserted modelling `for (;;)` exactly was untaken when
`condition.is_empty()` has dropped that exit edge since Cycle 19. **The wave-36
close landed it first**, in the card's proposed words.

Worth recording, because the first pass argued about exactly this: the close's
text cites `legacy_c.rs:3312` and `legacy_c.rs:5161`, not the card's proposed
`:3277` and `:5056`. Those are the **current** lines — the same values this card
independently re-measured when it judged the card's citations stale. So the
close and this card reached the same measurement independently, and the close's
text is correct at `2caf7369`. The line-pinned form remains drift-prone, and
5c has now been rewritten twice in two waves; that is noted for the close in
§7.3 rather than edited, because changing text the close deliberately landed is
not this card's to do.

### 7.2 `GATE-EXIT-SWALLOWED-BY-PIPE` was **not** promoted to a rule

Its registered discharge offers a gate runner that captures exits, or
`set -o pipefail` mandated in the standing gate section. Mandating it here would
be writing a **new standing rule** without a card, which `STABILITY-REEVAL`
method step 3 exists to catch. What this card did is narrower and forced: the
residual's *ground* lived only in the merged-gate row being compacted, so it is
rehomed to the gate section with the residual named and its mechanical discharge
explicitly still open.

### 7.3 For the integrator, and for the close

1. **`GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`** (§1.1) is the one finding here that
   mechanises something the project has now been bitten by twice. The cheapest
   discharge is a `lean_audits` key in the close's `gate.json`.
2. **Constraint 14's "does not engage" enumeration stops at wave 35.** This card
   does not touch constraint 14 — it is byte-identical — so the enumeration is
   left exactly as found, per the integrator's instruction. Waves 36 and 37 are
   unlisted there.
3. **The standing gate's command block omits the Python suite.** Every close
   runs `python3 -m unittest discover -s scripts/tests` and publishes its count,
   and the merged-gate row carries it, but the fenced block in *Standing
   per-change gate* lists seven commands and that is not one of them. Adding it
   is a rule change and therefore not this card's; it is reported so the close
   can decide.
4. **`wave-37.md`'s record head lines come in three shapes** —
   ``**`NAME` (date).**``, ``**Wave 37 · `NAME` (date).**`` and
   ``## NAME (date, …)``. This card's compaction rules do not reach the
   increment file's internal formatting, so it used the dominant shape for its
   own paragraph and normalised nothing, per the integrator's instruction.
5. **5c has been rewritten in two consecutive waves** and now carries two
   line-number citations into `legacy_c.rs`. Line citations in a standing
   constraint drift; identifiers do not.

### 7.4 What was **not** compacted, and why

* **Constraint 7's leak chain** (5,418 bytes) and **constraint 14's keyword
  ledger** (14,341) are the two largest constraints and both read as chronicles.
  Neither was touched. Constraint 7's chain is the *attribution* of a pinned
  number across every move it has made, which is the pin's evidence rather than
  narrative about it; constraint 14's ledger is the census record that makes
  "censused rather than assumed" checkable per card. Compacting either would
  move meaning.
* **Constraint 16** grew by 2,686 bytes this wave and is now 6,890. It is a
  live, twice-revised measurement record about load-sensitive rows, including
  one newly recorded as unnamed. Left alone deliberately.
* **The diagnostics namespace table** is a summary of spec §L21 by its own first
  line. Only the Q-space note's arithmetic was touched, and only to correct it.

---

## 8. Verbatim moves into `docs/development/`

*(listed below, with the gate row)*

---

## 9. Theorem-shaped citations, resolved and dangling

Every backticked identifier in the three files matching `QCore.…` or a
snake_case name with two or more underscores, resolved against `proof/` (137
`Audit*.lean` plus the `proof/QCore/` modules) and then the rest of the tree.
**79 candidates. 0 dangling.**

* **18** are `QCore.`-namespaced. **All 18 resolve** into `proof/`, including
  wave 37's two new modules, `QCore.ScalarRouteExitEdge` and
  `QCore.JoinedProvenanceRoundBound`.
* **39** are bare Lean theorem names and all resolve to a `proof/` file.
* **22** matched the snake_case shape but are **not** theorem citations: Rust
  functions (`analyze_error_regions`, `report_retained_loop_exit_edges`,
  `condition_always_true`, `observations_match_contract`, …), Rust test names
  (`a_forced_override_never_weakens_an_execution`,
  `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
  `memory_growth_beyond_the_declared_limit_is_bounded`, …), C symbols of the
  pinned corpus (`rd_kafka_topic_partition_list_sort`, `mk_mem_free`, …) and
  record keys. Each was located in the tree; **none is a claim about `proof/`
  that `proof/` does not carry.**

The four identifiers the wave newly introduced into these files
(`condition_always_true`, `report_retained_loop_exit_edges`,
`a_forced_override_never_weakens_an_execution`,
`rd_kafka_topic_partition_list_sort`) all resolve, so the wave introduced no
dangling citation either.

---

## 10. Residuals this card leaves

| Name | State |
|---|---|
| `Q-SPACE-EXTENSION-TALLY-HAND-KEPT` | **new, registered.** Needs a derivation or cross-check, or a decision to restate the note without a total. |
| `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` | **new, registered.** Discharge: a `lean_audits` key in the close's `gate.json`, or derive the row from it. |
| `MAIN-GUARD-REFUSES-PACK-REFS` | **new, registered.** Documentation half landed; the hook fix belongs to whoever owns `.githooks/`. |
| `WORKTREE-SUBMODULE-UNGATED` | **doc half discharged here.** Mechanical arm open. |
| `GATE-EXIT-SWALLOWED-BY-PIPE` | **ground rehomed here, not discharged.** §7.2. |
| `W36-COVER-HIDDEN-BLOCKER`, `CJSON-DELETE-NESTED-DETACH` | **registered by the wave-36 close**; this card's duplicate registrations withdrawn (§5.1). |

---

## 11. Gate

*(see §6)*
