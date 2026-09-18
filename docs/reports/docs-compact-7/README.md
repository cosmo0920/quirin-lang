# DOCS-COMPACT-7 — the seventh firing of the three-file compaction cadence

**Card:** `DOCS-COMPACT-7`, wave 37. **Base:** `477b733d` (the wave-37 open, over
`fa48211a`, the `WRITE-LEND-TASKS-EXERCISE` merge). **Scope:** `AGENTS.md`,
`CLAUDE.md`, `README.md`, plus verbatim moves into `docs/development/`.
Documentation only — no crate source, no `proof/`, no frozen archive
(constraint 21).

This card weakens no gate, deletes no pinned expectation and downgrades no
diagnostic. It moves **no constraint's meaning**: 46 of 49 standing constraints
are byte-identical across it, and the three that changed are named corrections,
each with its source, published in §2.

**A note on the two kinds of edit.** `CLAUDE.md`'s cadence rule expects
single-digit percentages and says the rule exists to bound accretion of
*narrative*, not to promise a shrinking file; `DOCS-COMPACT-6` grew `CLAUDE.md`
by 11.87% correcting understatements, and whether correction work belongs to
this cadence at all is an open question for the integrator. This report
therefore separates the two everywhere: **compaction** bytes (narrative removed,
the fact preserved in history or in a named authority) and **correction** bytes
(a statement that was false about the tree, repaired, which costs words). §3
gives the split line by line.

---

## 1. Numbers verified, and the source each was read from

Every figure below was re-read **from its source and never from the file being
compacted**, per the cadence rule.

| Figure | Value | Source it was read from |
|---|---|---|
| CT cases mapped | **46 of 50**; 4 unmapped (CT10, CT24, CT41, CT42) | `docs/reevaluation/ct_manifest.json`, counting `mapping_status` |
| CT sanitized-native | **10 of 46** — CT03, CT07, CT09, CT17, CT22, CT25, CT26, CT34, CT46, CT49 | same file, `stages.native_sanitized.status == "executed"` |
| CT proof | **3 of 46** — CT01, CT03, CT37 | same file, `stages.proof.status == "executed"` |
| Contract corpus | **82** modules | `find contract/qcore -name '*.qcore' | wc -l`; pinned at 82 in `compiler/tests/qcore_contract_corpus.rs` |
| Manifest-selected modules | **27** | pinned in `crates/qcore-c-gateway/tests/neighbours.rs` |
| resource-v1 declarations | **49** | `docs/reports/w36-corpus-contracts-3/README.md` §1 |
| Lean audits | **135** `Audit*.lean` | `find proof -name 'Audit*.lean' | wc -l`, and `scripts/check_lean_audits.py` enumerates exactly `proof.glob("Audit*.lean")` |
| `named_followups` | **171** at base; **174** after this card's three registrations | `docs/reviews/r0_3/acceptance_matrix.json`, and `dimensions.named_followups` re-derived by the generator |
| Corpus units | 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only | `docs/reports/w36-corpus-contracts-3/README.md` §1; unmoved at `joined-provenance-call-frames` |
| Corpus functions | 205 Safe / 84 provisional Unsafe / 4,545 Unknown of 4,834 | same |
| Diagnostics | leak 272 · `unknown_alias` 4,308 · `double_release` 0 · `unsupported` 49,922 · `summary_conflict` 1 | `docs/reports/joined-provenance-call-frames/README.md` (its AFTER is byte-identical to its BEFORE, so these are the baseline's) |
| Arbiter chain | **9** links | enumerated in the headline paragraph and cross-checked against the report directories |
| Example families | **12** | `ls -d examples/*/` |
| Merged gate | see §6 | measured by this card at its own tip |

### 1.1 A figure that did not survive its source

`AGENTS.md`'s merged-gate row attributed **135 Lean audits** to the *wave-35
close*. `docs/reports/wave35-close/gate.json` records that the wave-35 close
measured **132**. The 135 is today's file count, not that close's measurement —
one row carrying a number from one place and a label from another. This is the
hazard `INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED` (registered by
`STABILITY-REEVAL` #2) already names, arriving as a live defect rather than a
hazard. It is repaired by §6, which replaces the row with a single measurement
this card actually ran.

---

## 2. Constraint inventory, before and after

**Count 49 → 49. Labels identical and in the same order. Group headings
identical and in the same order.** Measured by
`docs/reports/docs-compact-7/scripts/constraints.py`, which splits the standing
constraints section on a column-0 `N.` / `Na.` label and hashes each body.

Labels, both sides: `1 2 3 4 5 5a 5b 5c 6 7 8 9 10 10a 10b 11 12 12a 13 14 15
15a 15b 15c 15d 15e 15f 15g 15h 15i 15j 15k 15l 15m 15n 15o 16 17 18 19 20 21
22 23 24 25 26 27 28`.

Groups, both sides: *Diagnostics and codes* · *W4 custody inference and the C
gateway* · *Compile-task identity and source transport* · *Concurrency source
routes* · *Providers, packages and limits* · *Pins, archives and measurement* ·
*Trust boundaries that are true statements, not TODOs*.

**46 of 49 constraints are byte-identical.** Three changed, all deliberately:

| # | before | after | Δ | byte-identical | why |
|---|---|---|---|---|---|
| 5c | 3,674 | 4,689 | +1,015 | **no** | correction — `FOREVER-FOR-EXIT-EDGE-UNOWNED` (§2.2) |
| 23 | 1,562 | 1,528 | −34 | **no** | compaction — a self-disclaimed stale checkpoint removed (§2.3) |
| 25 | 1,069 | 1,060 | −9 | **no** | correction — corpus 76 → 82 (§2.4) |

### 2.1 The wave-36 constraint text is preserved exactly

The card names five constraints whose text wave 36 corrected or replaced and
requires them inventoried byte for byte. Measured:

| Constraint | wave-36 change | after this card |
|---|---|---|
| 10b | replaced verbatim by `JOINED-PROVENANCE-CALL-FRAMES` | **byte-identical** (sha `b8f5548352d469f4`) |
| 15h | appendix by `WRITE-LEND-TASKS-EXERCISE` | **byte-identical** (sha `a3cc590f478914b5`) |
| 15n | the `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` decision | **byte-identical** (sha `bf1101828fff35f3`) |
| 15o | replaced verbatim by `MUTEX-SHARE-CROSS-EMISSION` | **byte-identical** (sha `cb09943f1f63b0b4`) |
| 5c | the superseded form | **changed, by the one named amendment below and nothing else** |

For 5c the amendment is additive plus one narrowing phrase; every other sentence
of the constraint is carried unchanged. Before and after of the changed region:

**Before**

> And the rule does **not** generalise to `while (1)` or `for (;;)`: modelling
> those exactly would remove an *exit* edge, which is the accusing direction and
> needs its own Lean statement and re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`).

**After**

> And the rule does **not** generalise to `while (1)`: modelling that exactly
> would remove an *exit* edge, which is the accusing direction and needs its own
> Lean statement and re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`) —
> `Statement::While` sets `successors = vec![body_entry, continuation]`
> unconditionally, so a `while` header's exit edge is never removed. A
> pre-existing route already removes the **exit** edge for `for (;;)`:
> `condition.is_empty()` (`legacy_c.rs`, the `Statement::For` construction) sets
> `condition_always_true`, which the CFG builder uses to drop `continuation`
> from the header's successors. It is exact (C11 §6.8.5.3) and `break` still
> reaches `continuation`, but it carries neither a Lean statement nor a pin, and
> it is not owned by card `C-GATEWAY-CONSTANT-DO-WHILE`
> (`FOREVER-FOR-EXIT-EDGE-UNOWNED`). **Until this paragraph was corrected in
> wave 37 it asserted that step was untaken**, which the tree has contradicted
> since Cycle 19 (`490f5151`); the sentence above is card `STABILITY-REEVAL`
> #2's §6 item 3 in its own words, with its two line citations replaced by the
> identifiers they named, because the lines had already moved between that
> card's base and this one.

### 2.2 `FOREVER-FOR-EXIT-EDGE-UNOWNED` — landed, with one recorded deviation

**Still open at this card's base.** No `docs/reports/wave36-close/` exists, so no
close had taken a verdict on it; `AGENTS.md` 5c still carried the false
sentence. The finding **is** registered in `named_followups` and **is** narrated
in `docs/development/increments/wave-36.md`; what was missing was the edit
itself.

**Re-verified in the tree at this card's HEAD**, independently of the card that
found it: `crates/qcore-c-gateway/src/legacy_c.rs` sets
`condition_always_true: condition.is_empty()` in the `Statement::For`
construction, and the CFG builder's successor assignment reads
`if *condition_always_true { vec![body_entry] } else { vec![body_entry,
continuation] }` — the exit edge is removed. The `while` arm assigns
`vec![body_entry, continuation]` unconditionally, so the sentence is true of
`while (1)` and false of `for (;;)`, exactly as measured. `break` still reaches
`continuation` because `break_targets` is pushed before the body is compiled.
`proof/QCore/ConstantDoWhile.lean` says nothing about it, and no fixture or test
anywhere in the repository drives a `for (;;)` loop — so the "no pin" half holds
too.

**The deviation, recorded rather than taken silently.** `CLAUDE.md`'s close rule
says an amendment lands **in the words the card proposed** or is recorded
`declined` with its ground. The proposed words embed two line citations,
`legacy_c.rs:3277` and `legacy_c.rs:5056`. Those were exact at that card's base
`2200a04e`; at this card's HEAD the same code stands at `:3312` and `:5161`.
Landing the words verbatim would have written two **stale line numbers** into a
standing constraint — which is the same defect class the amendment exists to
fix. The words are therefore landed **unchanged except that the two line
citations are replaced by the identifiers they named**
(`condition.is_empty()`, the `Statement::For` construction, the CFG builder's
successor assignment), which do not drift, and the substitution is disclosed in
the constraint text itself. Verdict: `applied`, with this deviation named. It is
not a softening — the sentence is strictly more durable and says the same thing.

**Not discharged by this card:** the pinning test the finding also asks for
(`for(;;)` with and without a `break`) is code work and stays open under the
same residual name.

### 2.3 Constraint 23's stale parenthetical

Constraint 23 carried "(**9 of 45 and 3 of 45** at the wave-34 close — read them
there, not here; this parenthetical is a checkpoint that has been stale
before…)". The numbers were stale again at this card's base (the row reads 10 of
46 and 3 of 46). Rather than re-date a checkpoint that has now gone stale twice
and disclaims its own authority in the same breath, the numbers are **removed**
and the rule kept: read them in the ledger row and **never copy them here**.
Nothing is lost — the paragraph itself named the row as the authority.

### 2.4 Constraint 25's corpus figure

"beside a live corpus (76, unmoved since wave 31)" → "(82 since wave 36)". Read
from `contract/qcore` on disk and from the pin in
`compiler/tests/qcore_contract_corpus.rs`. The `D02-B00-LEDGER-SNAPSHOT` record
this figure exists to contrast with is untouched.

---

## 3. Byte counts per file, split compaction vs correction

Base is `d44e3dc1` (this card's report skeleton, identical in these three files
to `477b733d`).

| File | before | after | Δ | % |
|---|---|---|---|---|
| `AGENTS.md` | 156,747 | 152,146 | **−4,601** | **−2.94%** |
| `CLAUDE.md` | 17,667 | 18,436 | +769 | +4.35% |
| `README.md` | 11,970 | 13,181 | +1,211 | +10.12% |
| **total** | **186,384** | **183,763** | **−2,621** | **−1.41%** |

### 3.1 Every edit, line by line

Measured by a line-level diff of base against HEAD; each hunk is classified as
**compaction** (narrative removed, fact preserved elsewhere) or **correction**
(statement false about the tree, repaired). Mixed hunks are split and the split
is stated.

| # | File / region | Δ bytes | Kind | Justification |
|---|---|---|---|---|
| 1 | `AGENTS.md` headline paragraph (arbiter chain) | −296 | compaction | Per-link prose dropped; the **chain list is preserved in full** (9 links, newest first), as is the byte-identity rule, the archiving-tool assertion, the declarative-link definition, the frozen-archive list and the two-coordinates sentence. Also **corrects** "those seven" → "those **nine**": wave 36 added two links and the count was not updated. |
| 2 | `AGENTS.md` *Cycle 19 raw corpus* row | −3,783 | compaction | The per-arbiter delta narrative (waves 31–36, six arbiters) removed. It was already a paraphrase of two other authorities, which the row itself named: each arbiter's report §5 and that wave's increment record. Current counts, the current pre-pass reach measurement (40 functions, `release_site` 6 of 14 / nullable-site 0 of 14), the leak-pin pointer to constraint 7, the coverage boundary and the `input_chunk_get` residual are all kept. Also repairs a run-on where two sentences had been concatenated without a stop. |
| 3 | `AGENTS.md` *CT01–CT50* row | −2,454 | compaction + correction | The waves-28-to-35 mapping chronicle removed (it is in the increment records and each lane's report). Method, gate (`PROOF_GATED_CASES`), constraint-21 supersession rule, the four unmapped cases' current grounds and the untranslatable-words rule all kept. **Two corrections inside it**, §4. |
| 4 | `AGENTS.md` *Contract corpus* row | −51 | compaction | The 73→74→76→82 chain and the 39→41→43→49 / 18→19→21→27 chains removed to the increment records; in exchange the row now **names the two pinned expectations and where they live**, which it did not before. |
| 5 | `AGENTS.md` follow-on count paragraph | −206 | compaction | The per-merge changelog (171 / 165 / 161 / 159 / 128 / 121, and one card's 28-registered / 62-swept / 10-retired) removed. The current number and the whole operative rule — carried vs derived, checked against each other, add through the generator, constraint 25 — are kept verbatim. |
| 6 | `AGENTS.md` *Merged gate* row | −230 | compaction + correction | The wave-35 close's host-row narrative, the three-wave `HOST-ROW-STARVATION-DESCRIPTION` story and the "prior figures, for the delta only" list all removed to history. **Corrects** the row's "135 Lean audits" attributed to a close that measured 132 (§1.1), and replaces every figure with one this card actually measured (§6). Both boundaries the old row carried — the host row and provider-carried-Unix — are kept, and a third is added, that this is not a close's measurement. |
| 7 | `AGENTS.md` constraint 23 | −34 | compaction | §2.3. |
| 8 | `AGENTS.md` constraint 25 | −9 | correction | §2.4. |
| 9 | `AGENTS.md` constraint 5c | +1,015 | correction | §2.2. An understatement-style defect: the file claimed less had been done than had been done, and saying so costs words. |
| 10 | `AGENTS.md` gate section, submodule comment | +23 | correction | "once per checkout" is false for a delegated worktree, which is a fresh checkout every time; the doc half of `WORKTREE-SUBMODULE-UNGATED`. |
| 11 | `AGENTS.md` gate section, new paragraph | +593 | preservation | Not new rules. `GATE-EXIT-SWALLOWED-BY-PIPE`'s ground lived **only** in the merged-gate row this card compacts, so dropping the row without rehoming it would have been deletion. Both residuals are named as registered with their mechanical discharge **open**, and the text says the instruction is the interim. See §7.1 for the decision not to promote either to a rule. |
| 12 | `AGENTS.md` Q-space note | +854 net | correction | §4.1. |
| 13 | `CLAUDE.md` cadence roster | +80 | compaction (net grew) | The dated roster of six firings replaced by a pointer to the seven reports, which are the real evidence; the **dated roster moves verbatim** to the wave-37 record. Grew because naming seven report directories costs more than six dates — the accretion it removes is the one that would have added a seventh date this wave and an eighth the next. |
| 14 | `CLAUDE.md` `STABILITY-REEVAL` pointer | +175 | correction | Said "#1 ran in wave 34 … next due after wave 35". #2 ran in **wave 36**; #3 is due after wave 37 by that card's own §6 item 7, which also names the next firing's highest-value ground. |
| 14a | `CLAUDE.md` main-guard bullet | +514 | correction | §5.2 — a measured false positive in the mechanism this bullet describes. |
| 15 | `README.md` CT counts | +1 | correction | 45/50 → 46/50, 5 → 4 unmapped, 9 → 10 sanitized. |
| 16 | `README.md` `std.buffer` row | +174 | correction | "no worker operation exercises it yet" was falsified by `WRITE-LEND-TASKS-EXERCISE` in wave 36. |
| 17 | `README.md` `std.concurrency` row | +183 | correction | The declared mutex crossing (wave 32, emitting real threads since wave 36) was absent from the capability table entirely. |
| 18 | `README.md` boundary paragraph | +236 | correction | Adds the crossing's own boundary in the words constraint 15o uses — it witnesses a thread boundary and says nothing about contention, a data race or the C memory model, and no TSan row exists in this repository — and narrows "general task/mutex composition remains open" to the measured fact behind it (`MUTEX_SHARING_GAP` is false; the task route's buffer has no mutex). A README that advertises real threads without that sentence overstates. |
| 19 | `README.md` corpus paragraph | +257 | correction | Adds the corpus **baseline pointer** and the one sentence that makes the chain mean something to a human reader. |
| 20 | `README.md` main guard | +359 | correction | The guard exists and a human contributor cannot discover it from `README.md`. Stated with both boundaries `CLAUDE.md` insists on: inert until installed, bypassable by intent, a guard against a mistake and not against an attacker. |

### 3.2 The split

| | compaction | correction / preservation |
|---|---|---|
| `AGENTS.md` | **−7,054** (rows 1, 2, 4, 5, 6, 7) | **+2,453** (5c +1,015, Q-space +868, gate section +616, submodule +23, constraint 25 −9, minus the −60 of corrections folded into rows 1, 3 and 6) |
| `CLAUDE.md` | +80 (row 13) | +689 (rows 14, 14a) |
| `README.md` | 0 | +1,210 |
| **total** | **−6,974** | **+4,352** |

`AGENTS.md`'s compaction alone is **−4.50%** of the file, and the file's net
change is **−2.94%** — inside the single-digit range the cadence rule expects.
Across the three files the compaction removes **6,974 bytes of narrative** and
the corrections add **4,352**, for a net **−2,621 (−1.41%)**. Every correction
is itemised above with its source so the integrator can judge each on its own
and, if the answer is that correction work does not belong to this cadence,
separate them cleanly.

Where the compaction went, by region: the *Cycle 19* and *CT* rows together
account for **−6,237** of it, 88% — those two table cells held **11,576** bytes
between them at this card's base and hold **5,339** after, while keeping every
current number, boundary and pin. With the *Contract corpus* row beside them the
three adjacent cells go 12,434 → 6,146.

---

## 4. Corrections, each with its source

Beyond §2's constraint corrections:

### 4.1 The Q-space note's false universal about wave 32

**MEASURED.** The note stated: *"Waves 32–34 allocated no code and ran no
condition extension either, so the count before it stood at thirteen"*, and
called `WRITE-LEND-TASKS-EXERCISE` the fourteenth. The same file's own
diagnostics table contradicts that twice:

* the `Q600` row: "`MUTEX-SHARE-CROSS` (wave 32) is the fifth, widening `Q600`'s
  boundary condition and the `Q610`–`Q627` sub-band's route and allocating
  nothing, on the same precedent";
* the `Q610`–`Q627` row: "Card `MUTEX-SHARE-CROSS` (wave 32) reaches `Q611`
  `UnguardedProtectedAccess` from a **new route** and allocates nothing".

So a wave-32 card did run the consequence; the universal was false and the tally
one short. This is the **same shape** as
`CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE`, which card `SWEEP-BLIND-SPOTS`
measured in the neighbouring constraint-14 bullet, about the same wave — and
the wave-35 close's handling of that one is the precedent followed here: narrow
the quantifier to the waves actually enumerated **and** give the missing card its
row, because replacing a false universal with a false member would be no better.

Totals re-derived from the note's own enumeration plus those two rows: wave 26
three, wave 27 four, wave 28 one, wave 29 three, wave 30 one, wave 31 one
(thirteen, nine bands), **wave 32 one** — `MUTEX-SHARE-CROSS`, which takes the
**tenth** band, `Q610`–`Q627` — and wave 36 one. **Fifteen firings, ten bands.**
Waves 33 and 34 remain at zero, for the reason the note already gave.

Nothing derives this tally mechanically, and its two halves live in two places
no check compares. Registered as **`Q-SPACE-EXTENSION-TALLY-HAND-KEPT`** (§5).

### 4.2 CT row — CT46's stages

The row said the `checked_mutex_crossing` verdict mode "is why four stages read
`unassessed` and none reads `executed`". Read from
`docs/reevaluation/ct_manifest.json` at this card's base, CT46's `c_emission`,
`native_execution`, `native_sanitized` and `mutex_reference_execution` all read
**`executed`**: wave 36's `MUTEX-SHARE-CROSS-EMISSION` landed the emission and
took the sanitized-native count from 9 to 10. The row now records the boundary
as **retired** and names the card that retired it.

### 4.3 CT row — the forward question

The row said CT03's forward question "stays open as
`TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`". Constraint 15n's own wave-36 appendix
says the opposite in bold: `WRITE-LEND-TASKS-EXERCISE` **DECIDES** it. One file,
two statements, one of them wrong. The row now defers to 15n, which is the
authority, and does not restate the decision.

### 4.4 The arbiter chain's length

"All earlier archives remain frozen (constraint 21): **those seven** plus …"
against a list of **nine** links. Wave 36 added two links and updated the list
but not the count. Corrected to nine; the frozen set is unchanged.

---

## 5. Names registered

`named_followups` **171 → 175**, appended by
`docs/reports/docs-compact-7/scripts/add_followups.py` (idempotent by id) and
then `dimensions.named_followups` re-derived by
`docs/reports/remaining-d02/build_matrix.py`, which reports `named followups
175`, `lean audits 135`, `contract modules 82`.

The sweep (`docs/reports/docs-compact-7/scripts/residuals.py`): at this card's
base, **160** residual-shaped names appear in the three files and **14** are
neither a `named_followups` id nor mentioned anywhere in the matrix. Of those
14, **eleven are card names**, not residuals — `CT01-PROOF-LEG`,
`CT03-PROOF-ALPHABET`, `CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`, `CT-PROOF-LEG-2`,
`CT-WIRE-LANE-2`, `CT-WIRE-LANE-3`, `REGION-CREDIT-JOINS`,
`REGION-JOINS-DUTIES`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES` — each
with its own report directory, and a card name is not open work. Three are
genuine residuals:

| Name | Verdict | Ground |
|---|---|---|
| `W36-COVER-HIDDEN-BLOCKER` | **registered** | Named in `W36-CORPUS-CONTRACTS-3` §9's own follow-on table and carried in the ledger row this card compacts; reached no tracker. |
| `CJSON-DELETE-NESTED-DETACH` | **registered** | Same table, same row, same absence. |
| `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` | **not registered, deliberately** | The wave-35 close already dispositioned it: `docs/reports/wave35-close/README.md` §4 is titled "answered, not registered", and constraint 5c records the same. Registering it now would reopen a decision already taken and recorded. |

Plus two names this card itself introduces:

| Name | Verdict | Ground |
|---|---|---|
| `Q-SPACE-EXTENSION-TALLY-HAND-KEPT` | **registered** | §4.1. The residual is the method, not the arithmetic: the tally is prose, nothing derives it, it has been wrong once, and no check compares it against the diagnostics rows it summarises. |
| `MAIN-GUARD-REFUSES-PACK-REFS` | **registered** | §5.2. A measured false positive in the mechanism `CLAUDE.md` describes. |

**After this card**: 164 residual-shaped names in the three files, **11** absent
from the matrix entirely — the ten card names above that remain cited (`CT-WIRE-LANE-2`
left the files with the CT row's chronicle) plus
`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`, whose non-registration is the
recorded decision above.

### 5.1a Retirements — named explicitly, as the brief requires

**This card retires nothing.** No `named_followups` entry is removed, renamed or
marked discharged by it. Four entries have a *documentation half* landed here and
**stay open** under their own names — `FOREVER-FOR-EXIT-EDGE-UNOWNED`,
`WORKTREE-SUBMODULE-UNGATED`, `GATE-EXIT-SWALLOWED-BY-PIPE` and
`INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED` (whose hazard §1.1 shows
arriving as a live defect) — because in each case the mechanical or code arm of
the discharge is untaken. Landing half a discharge is not a retirement, and the
count moves in one direction only: **171 → 175**.

### 5.2 `MAIN-GUARD-REFUSES-PACK-REFS` — measured incidentally, and fail-closed

While committing, `git`'s background `gc` failed behind a successful commit with
the guard's banner:

```
QCore-C main-ref guard: REFUSED an update to refs/heads/main
  ground: delegated card-worktree provenance (…)
fatal: ref updates aborted by hook
error: failed to run pack-refs
error: task 'gc' failed
```

**Reproduced deterministically**: with `main` at `477b733d`, `git pack-refs
--all` from this worktree prints the same refusal, and
`.git/qcore-main-guard.log` records
`refused-provenance:… refs/heads/main 0000000000000000000000000000000000000000->477b733d…`
— old is the null OID and new is the ref's **existing** value, so the refused
transaction advanced nothing. `pack-refs` re-writes every ref into
`packed-refs`; the `reference-transaction` hook sees that as an update to
`refs/heads/main` and refuses it on delegated-worktree provenance.

This is **not** a soundness problem: the guard fails closed, in the refusing
direction, and no delegated commit is lost — mine landed. It is a false positive
with two costs: repository maintenance is broken in every delegated worktree,
and a card is shown an alarming guard banner attached to an operation that was
never a main update, which trains readers to ignore the banner — the one thing a
guard cannot afford. This card owns no hook, so only the documentation half is
landed: `CLAUDE.md`'s guard bullet now says to expect it. The code half is the
residual.

### 5.1 For the integrator — ten wave-36 residuals outside this card's scope

This card registers only names that appear **in the three files it owns**. The
same scan over the wave-36 card reports finds **ten** further residual-shaped
names that reach no tracker at all and are named in none of the three files, so
they are the wave-36 close's reconciliation business rather than this card's:
`ADD-NAME-COLLISION`, `CJSON-DELETE-SIBLING-CHAIN`,
`JOINED-PROVENANCE-FACTS-NAME-NOTHING`, `KAFKA-MEM-FREE-SUMMARY-CONFLICT`,
`LIBC-MEM-RESULT-ALIAS`, `MK-PRINT-TAIL-ACTUALS`,
`STANDALONE-HOST-CLOSED-INPUT-LOAD`, `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`,
`W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`, `W36-CTRACES-FAMILY-FIRST`. Seven of the
ten are `W36-CORPUS-CONTRACTS-3`'s §9 table, which is a complete, well-formed
follow-on table that simply has not been swept yet. Note also that
`STANDALONE-HOST-CLOSED-INPUT-LOAD` proposes itself as constraint 16's *sixth*
load-sensitive row, a slot the wave-35 close already filled from
`VERIFIER-HOST-CONTROL-BUDGET` — the two are about the same test and need
reconciling into one row rather than two.

---

## 6. The merged-gate row, and this card's gate

The card's brief allows either `docs/reports/wave36-close/gate.json` once that
close lands, or `fa48211a`'s figure. **No wave-36 close exists** at this card's
base or at its final fetch — `main` is still `477b733d`. Rather than carry a
per-card branch-tip figure into a row labelled *merged*, this card **ran the
full standing per-change gate itself** at the wave-37 open tip, which is the
wave-36 integration tip plus one documentation commit, and the row now carries
that single measurement. Disk was 258 GB free, far above the 60 GB threshold, so
the Python-only subset was not used.

**Full gate, all eight steps exit 0.** Started 22:57:40Z, finished 23:21:11Z.
Every step's exit captured to its own file under the worktree-private
`measure/`, with **no pipe in any gate command**.

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a2` |
| `cargo test --workspace --no-fail-fast` | 0 | **4,611 passed / 0 failed / 8 ignored**, 231 result summaries over 220 test binaries and 9 doc-test targets, 0 `FAILED` summaries |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `reevaluation_probe` | 0 | |
| `reevaluation_followup` | 0 | |
| `scripts/check_lean_audits.py` | 0 | **Passed 135 Lean audits** |
| `python3 -m unittest discover -s scripts/tests` | 0 | **Ran 274 tests … OK** |

Per-command exits in `docs/reports/docs-compact-7/gate.json`.

The matrix's `named_followups` was appended to once **while** the gate was
running (§5.2's residual), and two Python tests read the committed matrix, so
`python3 -m unittest discover -s scripts/tests` was **re-run afterwards**: **Ran
274 tests … OK**, exit 0. No `.rs` file in the tree mentions
`acceptance_matrix`, so the `cargo test` figure does not depend on it and was
not re-run.

### 6.1 Which load-sensitive rows this gate saw

Constraint 16 requires a gate report to **state which it saw**. Five of its six
rows live in this workspace and **all five were green**:
`a_declared_process_limit_stops_the_fixture_from_forking`,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`, the confined-fixture
write test,
`enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`,
and
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`.
The sixth, `real_packages_preserve_unknown_and_reject_mutations`, lives in the
separate `providers/` workspace and was not among those measured here.

The fifth is worth its own sentence: `W36-CORPUS-CONTRACTS-3` saw it **red** once
in-gate at load ≈31–41 on 16 cores and green 12/12 in isolation, and proposed it
as constraint 16's sixth row under the name
`STANDALONE-HOST-CLOSED-INPUT-LOAD` — a slot the wave-35 close had already
filled from `VERIFIER-HOST-CONTROL-BUDGET`. This run is a **second independent
green under full-suite load**. It is **not** the quiet-host isolated number
`HOST-ROW-STARVATION-DESCRIPTION` still asks for, and is not offered as one; and
it does not settle whether the row needs one entry or two, which is the wave-36
close's reconciliation (§5.1).

### 6.2 What the row says now

The row reports one measurement at one tip with three boundaries: the host rows
above; provider **122 carried, not re-measured, and Unix-only** (nothing under
`providers/` or `crates/qcore-schemas/` changed, so constraint 18 is not
engaged); and, stated plainly, that this is **a documentation card's measurement
of the wave-36 integration tip rather than a close's**, so the wave-36 close
should replace it with its own `--gate` numbers. The three-wave host-row
narrative and the "prior figures, for the delta only" list moved verbatim to
history.

Since this card changed no Rust, no `proof/` and no frozen archive, these
numbers are **not evidence about a code change**; they are the integration tip's
current state, measured because nothing else had measured it.

---

## 7. Decisions taken, and one not taken

### 7.1 `GATE-EXIT-SWALLOWED-BY-PIPE` was **not** promoted to a rule

Its registered discharge offers two arms: a gate runner that captures exits, or
`set -o pipefail` mandated in the standing gate section. Mandating it here would
be writing a **new standing rule** without a card, which `STABILITY-REEVAL`
method step 3 exists to catch and which this card is not entitled to do. What
this card did instead is narrower and forced: the residual's *ground* lived only
in the merged-gate row being compacted, so it is rehomed to the gate section as
a stated measurement with the residual named and its mechanical discharge
explicitly still open. The distinction matters and is recorded so a later reader
does not mistake the paragraph for an adopted rule.

### 7.2 What was **not** compacted, and why

* **Constraint 7's leak chain** (4,633 bytes) and **constraint 14's keyword
  ledger** (13,064 bytes) are the two largest constraints and both read as
  chronicles. Neither was touched. Constraint 7's chain is the *attribution* of
  a pinned number across every move it has made, which is the pin's evidence and
  not narrative about it; constraint 14's ledger is the census record that makes
  "censused rather than assumed" checkable per card. Compacting either would
  move meaning, which this cadence forbids. They are flagged here so the next
  firing does not have to rediscover the judgement.
* **The diagnostics namespace table** (19,227 bytes) is a summary of spec §L21
  by its own first line, and its per-row condition-widening histories are the
  §L21 process working. Not touched.
* **`docs/development/increments/`** pages were not edited except by the
  prescribed prepend and the verbatim moves.

---

## 8. Verbatim moves into `docs/development/`

Seven blocks, **18,029 bytes**, moved verbatim into
`docs/development/increments/wave-37.md` under a heading that labels them
checkpoints and states in terms that they are **not** a second current rule
authority:

1. the headline paragraph (arbiter chain) as it stood after wave 36 — 2,264 B
2. the *Cycle 19 raw corpus* row — 5,752 B
3. the *CT01–CT50 conformance* row — 5,826 B
4. the *Contract corpus* row — 859 B
5. the *Merged gate* row — 2,146 B
6. the follow-on count paragraph — 770 B
7. `CLAUDE.md`'s dated cadence roster and `STABILITY-REEVAL` pointer — 412 B

Item 7 is the one block whose content lived **only** in the file being
compacted: the six firing dates appear nowhere else in the tree. The other six
are paraphrases of material that also lives in the arbiters' reports and the
wave records; they are moved anyway, because the card asks for it and because a
verbatim checkpoint costs less than an argument about whether a paraphrase was
unique.

---

## 9. Theorem-shaped citations, resolved and dangling

Every backticked identifier in the three files matching `QCore.…` or a
snake_case name with two or more underscores was extracted and resolved against
`proof/` (135 `Audit*.lean` plus the `proof/QCore/` modules) and then against
the rest of the tree. **76 candidates. 0 dangling.**

* **16** are `QCore.`-namespaced citations. **All 16 resolve** into `proof/`,
  including wave 36's three new module pairs —
  `QCore.JoinedProvenanceCallFrames.a_call_never_adds_a_name`,
  `QCore.WriteLendTasksExercise.no_byte_is_written_by_two_parties` and
  `QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law`
  — and `QCore.MutexShareCrossArms`.
* **40** are bare Lean theorem names and all resolve to a `proof/` file
  (`fixed_point_is_sound`, `acyclic_solution_is_unique`,
  `the_empty_frame_is_the_old_blanket_forget`, `some_index_is_always_outside`,
  `weakening_stays_sound`, `constant_do_while_is_its_body`, and so on).
* **20** matched the snake_case shape but are **not** theorem citations: Rust
  functions (`analyze_error_regions`, `check_whole_program`,
  `observations_match_contract`, `parse_observed_trace`, …), Rust test names
  (`a_modeled_scalar_condition_is_a_separate_pre_existing_route`,
  `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
  `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too`,
  `real_packages_preserve_unknown_and_reject_mutations`,
  `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`), C symbols of the
  pinned corpus (`cio_chunk_get_content_size`, `flb_input_chunk_total_size`,
  `mk_mem_free`, `rd_kafka_mem_free`) and record keys
  (`retired_unmapped_reasons`, `checked_mutex_crossing`). Each was located in
  the tree; **none is a claim about `proof/` that `proof/` does not carry**, and
  in particular the two that read most like theorems —
  `a_modeled_scalar_condition_is_a_separate_pre_existing_route` and
  `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too` — are
  described in `AGENTS.md` as Rust pins, which is what they are.

The scan tool is `docs/reports/docs-compact-7/scripts/theorems.py`.

---

## 10. Residuals this card leaves

| Name | State |
|---|---|
| `Q-SPACE-EXTENSION-TALLY-HAND-KEPT` | **new, registered.** Needs code (a derivation or a cross-check) or a decision to restate the note as totalless narrative. |
| `MAIN-GUARD-REFUSES-PACK-REFS` | **new, registered.** Documentation half landed in `CLAUDE.md`; the hook fix is open and belongs to whoever owns `.githooks/`. |
| `FOREVER-FOR-EXIT-EDGE-UNOWNED` | **doc half discharged here.** The pinning test driving `for(;;)` with and without a `break`, and any Lean statement, remain open under the same name. |
| `WORKTREE-SUBMODULE-UNGATED` | **doc half discharged here** (the gate comment no longer says "once per checkout"). The mechanical arm — a gate that self-checks the submodule, or worktree creation doing the init — is open. |
| `GATE-EXIT-SWALLOWED-BY-PIPE` | **ground rehomed here, not discharged.** §7.1. |
| `W36-COVER-HIDDEN-BLOCKER`, `CJSON-DELETE-NESTED-DETACH` | **registered here**, undischarged, with the discharge conditions their own card wrote. |
| Ten wave-36 report residuals | **untracked**, §5.1, for the wave-36 close. |

---

## 11. Gate

Full standing per-change gate, **all eight steps exit 0** — see §6 for the table
and §6.1 for the load-sensitive rows this run saw. `docs/reports/docs-compact-7/gate.json`
carries the per-command exits.

**+0 reserved words, +0 tokens, +0 `Q` codes** — this card engages constraint 14
not at all and allocates nothing in the `Q` space; it only *counts* the
allocations others made (§4.1).
