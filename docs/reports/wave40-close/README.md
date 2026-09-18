# `WAVE-40-CLOSE` — the integrator's reconciliation and close for wave 40

Wave 40 is a **two-card cadence wave**: `STABILITY-REEVAL-4` (merged `d6a57f66`)
and `DOCS-COMPACT-8` (merged `0c7b0cea`, re-derived onto the reeval's tip). It
moves **no crate source at all** — `git diff --stat 6dc484ad 0c7b0cea --
crates/ compiler/ proof/ providers/` is empty — so **nothing is promoted**: no
corpus count, unit status or function status moves, raw units stay unpromoted,
contract corpus 88, CT 47/50 with sanitized-native 10 and proof 4, and full
language conformance stays **false**.

## Headline

* **48 integrator-addressed items verdicted** across four card reports.
  **19 `applied` · 24 `noted` · 3 `deferred` · 2 `declined`**, plus **2
  `no_section`** cards. `--check` exit **0** on this close's own record, with
  44 content claims verified at their named locations across 7 files and 19 of
  19 `applied` verdicts carrying checkable text.
* Only **27** of the 48 are the wave's own two cards. The other **21** are
  wave-39 items on two frozen reports that re-entered the sweep because this
  close appended a dated section to each (constraint 21); they carry the
  **wave-39 close's** measurement, not this one's.
* **Tracker 258 → 257**: three retirements, two registrations, six amendments.
  Every retirement named explicitly, verified registered by id, and its
  discharge condition read in full before removal (§5).
* **Two standing questions decided** (§8): the perishable `verify` claim — where
  the measured class is far wider than the residual stated — and the
  checkpoint-page split, kept.
* The **merged-gate row re-owned** under this close's provenance at its own
  **in-history** tips, its summary arithmetic now a **third** independent
  measurement agreeing exactly (§5).
* **One new finding of the cadence's own kind**: constraint 14's reserved-word
  count is one short of what the function it names as "the one place to count
  them" actually matches (§6).

## 1. Scope and method

Base **`0c7b0cea`**, confirmed equal to `origin/main` after `git fetch` and to
`main` at the start. Branch `worktree-agent-ae8c514e4518ad359`, a delegated
worktree; no git command was run against the main checkout, nothing merged,
nothing pushed. `git submodule update --init --recursive` exit 0,
`third_party/libevent` at `d82464a2`.

The sweep range is **inclusive of the previous close's own merge**:

```
python3 scripts/reconciliation_sweep.py . --since 6dc484ad
```

which is `6dc484ad^..HEAD`; the wave's own span `6dc484ad^..0c7b0cea` is
**22 commits** (`git rev-list --count`), so the wave-39 close's own branch work
is swept too (the `WAVE-BASE-UNSWEPT` seam), and this close's commits extend it
from there. Every verdict below was
measured against the tree rather than read off the card — where a card says a
figure, this close recomputed it; where a card says an amendment landed, this
close read it at the named location and the `verify` arm re-reads it
mechanically.

## 2. The sweep

`--since 6dc484ad` names six card directories and finds 48 items in four of
them:

| Card | Items | Section |
|---|---|---|
| `docs-compact-8` | 15 | 9. For the integrator |
| `ct42-cell-route` | 13 | 9. For the integrator (**wave 39's**) |
| `stability-reeval-4` | 12 | 6. Integrator reconciliation list |
| `ct07-two-thread-proof-leg` | 8 | 11. For the integrator (**wave 39's**) |
| `wave39-close` | — | `no_section` |
| `wave40-close` | — | `no_section` |

The two wave-39 card reports are in the sweep **because of this close**: it
appended a dated "Published beside" section to each (§8.3), and the sweep reads
a directory git says changed. Their 21 items were verdicted in full by the
wave-39 close at the same item ids; re-verdicting them here would put this
close's name on wave 39's measurement, which is the softening-while-reconciling
failure the rule exists to prevent. They are `noted`, each naming
`docs/reports/wave39-close/reconciliation.json` as the authority.

### 2.1 Every `declined` and `deferred`, with its ground

| Item | Verdict | Ground |
|---|---|---|
| `stability-reeval-4#…#2` — rewrite the two false TSan sentences and re-cite the retired name in three places | **deferred** | Two of three parts landed (`DOCS-COMPACT-8` rewrote constraints 15h and 15o). The third — `compiler/tests/ct07_proof_leg.rs` and `docs/reports/ct07-two-thread-proof-leg/README.md` still citing the retired `TASK-ROUTE-TSAN-SEGV` — was **not** taken here: a crate-source comment is a code card's edit, and this close deliberately did not smuggle a second, unrelated correction into the dated note it was appending to that same report for another residual. Recorded in the entry's own `why` as its standing discharge condition; `LEDGER-TSAN-BOUNDARY-SENTENCES-STALE` stays **open** — so those two AGENTS.md instances now reach a tracker, which they did not before |
| `stability-reeval-4#…#6` — register a successor for `CT24-CLEANUP-TASK-COMPOSITION` and `CT46-NOTES-PREDATE-TSAN-ROW`, or restate the CT ledger row | **deferred** | Both halves belong to `CT10-REASON-ROW`, open for a **seventh** wave and re-listed in this record's `deferred_decisions`: whether an unmapped or retired case's reason may live only in a report is that decision's question, and a close registering successors would answer it sideways. The half this close *could* measure is taken instead — `RESIDUAL-QUIET-BY-PROSE-MENTION` (§6), which names both of them and four more |
| `docs-compact-8#…#4` — take `GATE-ROW-SUMMARY-COUNT-UNCHECKED`'s mechanical discharge (two derived `--gate` keys) | **deferred** | Generator code plus the mutation control that shows the refusal red; a tooling card's work with its own gate, not a close's. The entry stays registered, named in §10 as tooling-card material. What the close did do is reduce its exposure without touching its discharge: three independent measurements now support the published arithmetic |
| `docs-compact-8#…#5` — should the repository map stop carrying countable figures | **declined** | The map's figures were **re-derived here and are all correct** — `contract/qcore` 88, `crates/qcore-c-gateway/src` 36 with ten named by standing constraints, `crates/` 9 — so no stale figure forces the decision now; and which figures a navigation table carries is a documentation-structure decision the compaction cadence owns, which a close taking it would pre-empt from `DOCS-COMPACT-9`. Recorded rather than folded in: this close found a *different* stale figure of the same family (§6) and registered it separately, because the mechanism differs |
| `docs-compact-8#…#10` — restore the "icebox has held for N waves" count if the close knows it | **declined** | The close does **not** know. The tree records the decision only as "an integrator-recorded decision of 2026-09-12" and names no wave; deriving a wave number from a date would be inference presented as a measurement. The span the enumeration supports ("across waves 26–39") is what the tree carries and is kept. A second reason: the figure "thirteen waves" also sits inside the wave-39 close's **frozen** deferred-decision ground, which constraint 21 forbids editing, so a restored count would have to agree with a frozen number nobody can re-derive either. What is added instead is wave 40's own datum — the consequence ran **zero** times and zero codes were allocated, measured |

## 3. Deferred decisions carried from the wave-39 close

`outstanding_decisions(root, 40)` returns **eleven**; all eleven are re-listed
with a ground measured at this base, and none is resolved — a two-card cadence
wave that moves no crate source produces no evidence about most of them, and
this close says so rather than letting silence read as resolution. Full grounds
are in `reconciliation.json`'s `deferred_decisions`; the new evidence each one
gained:

| Decision | New evidence at this base |
|---|---|
| `CT10-REASON-ROW` | An **eighth** wave, and a second card now asks for it: the reeval's item 6 names two untracked names whose cause is this decision's question. No new evidence about CT10 itself — no crate source moved |
| `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` | Cuts both ways this wave: the compaction card fired and did nine corrections, **and** this close still did correction work on top (a stale word count, a stale cadence bullet, a non-ancestor provenance tip) |
| `EXAMPLES-STYLE-SLICE-FANOUT` | `examples/` did not move at all (`git diff --name-only 6dc484ad 0c7b0cea -- examples/` empty); 12 family directories, unchanged |
| `F02-WINDOWS-PIN-RUN` | The reeval re-read both pin sites; the literals still disagree, so that Windows row is red for a **fifth** wave. Both gates report 10 ignored, the tenth being the `#[ignore]`d agreement test |
| `INDIRECT-CENSUS-FEATURE-UNBUILT` | No Cargo feature or CI job moved — the wave's whole diff outside `docs/` is empty |
| `MATRIX-RECORDED-BY-DECISION` | Still exactly **67** entries without `recorded_by`; every wave-40 registration carries it (23 + 2 + 2) |
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | Four more regenerations this wave; `updated` still opens "2026-09-12 (wave 27 complete", now beside a file regenerated thirteen waves later |
| `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` | `OWNED-FIELD-WINDOWS-RUN` now appears in four documents and no tracker entry; `HTTP-V1-WINDOWS-RUN` absent by id while twice in the matrix's prose — which is exactly `RESIDUAL-QUIET-BY-PROSE-MENTION` |
| `Q-NAMESPACE-DECISION` | The consequence ran **zero** times in wave 40 and zero codes were allocated; the tally stands at eighteen on ten bands across waves 26–39, and `AGENTS.md` now says wave 40 added nothing |
| `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS` | Not taken, same ground. Distinguished explicitly from this close's new registration, which is the **opposite** direction: the tool reads *too much* prose |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | A fourth repetition with one new wrinkle — see §4 |

One decision is **opened and ended in the same record**: `CHECKPOINT-PAGE-SPLIT`
(§8.2), `resolved`, because the tree resolved it.

## 4. The hand sweep of merge prose

`UNCARDED-BLIND-TO-MERGE-PROSE` (measured by the reeval) says
`reconciliation_sweep.py --uncarded` excludes every merge by construction — and
the merges are exactly where the integrator's own edits live. **This is the
first close able to act on that finding**, so the wave's merge prose is swept by
hand.

### 4.1 `--uncarded`, classified

Nine commits, every one attributable: **six** `DOCS-COMPACT-8` branch commits
(`70f1cec3`, `97166b99`, `c1eaf2a4`, `fa6b50c8`, `c97d4e5d`, `55951751`),
**two** `WAVE-39-CLOSE` branch commits (`11edf7eb`, `b1462e6d`) reachable only
through the inclusive base merge, and **one structural integrator commit outside
any merge**: `f11ccbba`, *"Open wave 40: create the increment record and move the
index pointer"*, two files, carried on `STABILITY-REEVAL-4`'s branch on the
`05bbd149` (wave 38) / `477b733d` (wave 37) convention.

**So the answer to "`--uncarded` should show no integrator commit outside
merges" is NO this wave** — and the exception matters, because that same commit
is what broke one of the wave-39 close's `verify` claims (§8.1). A structural
commit nobody reports is now measurably load-bearing; that is
`SYNC-RULE-TOOLING-COMMIT-SCOPE`'s question with a cost attached.

### 4.2 What the two merge messages assert, and what checked out

`d6a57f66` (`STABILITY-REEVAL-4`) asserts: the four-step method over waves
38–39 with no Rust, no Lean, no semantic change and nothing promoted; 8
discharged / 2 partial / 4 open of firing #3's fourteen with **none gone
quiet**; the correction of firing #3's own headline (mechanism 5/11, sentence
3/3, the discriminator being **brief-reach**); 23 findings registered; the one
retirement named explicitly; `CT-COMPOSED-NATIVE-ROWS` measured discharged and
deliberately **not** retired; "Matrix 234 → 256"; and a per-merge
"Gate at `f661b2aa` (integration tip)" line.

`0c7b0cea` (`DOCS-COMPACT-8`) asserts: 56 numbered entries before and after with
the reeval **proved** to have moved none; `AGENTS.md` 192,048 → 173,072
(−9.9 %); nine corrections; 47,344 bytes of checkpoints on a separate page; two
registrations and nothing retired, "256 → 258"; "Do not retire it at the close"
for `LEDGER-TSAN-BOUNDARY-SENTENCES-STALE`; and its own "Gate at `3cb27d80`
(integration tip)" line.

Checked by this close:

* **The arithmetic chains reproduce** against the committed file:
  234 + 23 − 1 = 256, 256 + 2 = 258, and 258 − 3 + 2 = **257** after this close.
* **Both merges' stated integration figures equal their cards' own gates** —
  4,727 workspace / 141 Lean / 303 Python in both
  `docs/reports/{stability-reeval-4,docs-compact-8}/gate.json` — so under the
  rule this close landed (§8.3) **neither owed an `integration-gate.json`**.
  Measured, not assumed.
* **Both cited integration tips are non-ancestors of `main`**: `f661b2aa` and
  `3cb27d80`, each a commit object here, each `git merge-base --is-ancestor …`
  exit 1. Two new instances of `INTEGRATION-TIP-CITATION-UNREACHABLE`, seven in
  all.
* **No merge hand-edited the merged-gate row's Lean figure.** The row at this
  base is `DOCS-COMPACT-8`'s own measurement, which is
  `MERGED-GATE-ROW-LEAN-HAND-EDIT`'s repair holding for a first wave.

Nothing in either merge's prose is unattributable to a card's report or to a
measurement this close could reproduce.

### 4.3 The integrator's own record this wave, unsmoothed

**Two tooling repairs were made mid-wave, and both are visible only in merge
prose — in no report and under no residual name.**

1. **The Lean figure is no longer hand-edited at a merge.** The merge line's own
   "Gate at `<tip>` (integration tip)" sentence carries the Lean count as *that
   run's* measurement, and merge `d6a57f66`'s line is the first under the new
   rule. Measured consequence: `MERGED-GATE-ROW-LEAN-HAND-EDIT`'s mechanism did
   **not** recur this wave. The entry stays open, because the finding was that
   nothing *prevents* it — a close is the only catch, and a close is not a gate.
2. **The follow-on-count checks were made whitespace-insensitive** after the
   compaction rewrapped that paragraph. The count and the word `entries` now
   straddle a newline in `AGENTS.md`; the literal check every merge depends on
   would have failed from here on.

Both are precisely the shape `SYNC-RULE-TOOLING-COMMIT-SCOPE` asks about:
committed-document tooling changed with no report and no residual name, indexed
by nothing but a merge message. They are written here because this is the only
document that indexes them.

## 5. The merged-gate row, re-measured and re-owned

Two detached full gates (`setsid -f`, **no** `trap '' TERM`; every exit to its
own file, no pipe; `git submodule update --init --recursive` inside the script),
each polled by a bounded foreground loop on its `DONE` marker. Both tips are
**ordinary commits of this close's own branch** and therefore reachable from
`main` once its merge lands — deliberately, because the tip the row named at
this base (`c188d3ab`) is not.

| | Base run | Final run |
|---|---|---|
| Tip | `2ba203ef` (= `0c7b0cea`'s code byte for byte plus this close's report skeleton) | this close's tip carrying every non-report change |
| Workspace | 4,727 passed / 0 failed / 10 ignored | identical |
| `test result: FAILED` lines | 0 | 0 |
| Targets / strict summaries / loose | 236 / 238 / 240 | identical |
| Doubled sections | 2 (`Doc-tests qcore_core`, `Doc-tests qcore_verifier_host`) | identical |
| fmt · clippy · probes | 0 · 0 · 0, 0 | identical |
| Lean audits · Python | 141 · 303 OK | identical |

**The summary arithmetic is now a third independent measurement.**
`DOCS-COMPACT-8` and `STABILITY-REEVAL-4` measured it independently and agreed
on 238 and two; this close ran the reeval's own shipped script
(`docs/reports/stability-reeval-4/scripts/count_summaries.py`, path
parameterised, against its own gate log) and got **236 targets, 238 strict, 240
loose, two doubled sections, zero targets emitting no summary**. Three
measurements on three trees agree exactly. The card asked that a *differing*
measurement be reported as a finding rather than silently reconciled; the
converse is reported here just as explicitly.

**The `clang` claim is this close's own.** `clang --version` was run in this
worktree with `QCORE_CLANG` unset, exit 0 captured to its own file, and the
recording published beside the gate file — not borrowed from another report.
`CLANG-RECORDING-UNBOUND-TO-ITS-RUN` says the generator's check cannot tell the
difference, so the honesty is the measurer's; `AGENTS.md`'s standing-gate
section now says that in those words (§8.4).

**Constraint 16.** Four of the constraint's five named rows live in this
workspace and all four were green in both runs; the fifth,
`real_packages_preserve_unknown_and_reject_mutations`, lives in the separate
`providers/` workspace. Nothing flaked, so no isolated re-verification was
required and none is claimed. These are full-suite runs on a machine carrying
its own build, **not** the quiet-host number `HOST-ROW-STARVATION-DESCRIPTION`
still asks for. Provider **122 is carried, not re-measured** (nothing under
`providers/` or `crates/qcore-schemas/` moved), and is a Unix figure
(`PROVIDER-CONFORMANCE-UNIX-ONLY`).

## 6. Headline numbers verified against their sources

Re-read from the sources, never from the ledger — and after a compaction pass,
which is when a ledger is most likely to have been copied rather than derived.

| Figure | Source read | Result |
|---|---|---|
| Corpus row: 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only; 4,834 functions 213 / 84 / 4,537; leak 272, `unknown_alias` 4,309, `unsupported` 49,753, `summary_conflict` 1 | `docs/reports/w37-corpus-contracts-4/artifacts/diagnostics.json.gz` (the frozen wave-37 arbiter the ledger names) | **exact**, every figure |
| CT row: 47/50, unmapped CT10/CT24/CT41, sanitized 10, proof 4 (CT01, CT03, CT07, CT37) | `docs/reevaluation/ct_manifest.json`, and `scripts/ct_stage_summary.py --check` exit 0 | **exact** |
| Sanitized-native host gate | the manifest's ten executed `native_sanitized` rows | **all ten** `["linux","macos"]` — the qualifier now published (§8.5) |
| Contract corpus 88 / 59 resource-v1 / 33 manifest-selected | the tree; `grep -rhc 'requires resource_contract_v1;'`; `contract/cycle19-resource-contracts.json` | **exact** |
| Repository map: gateway 36 modules, **ten** named by constraints; `crates/` 9 | the directory, and the row's own list | **exact** (both figures `DOCS-COMPACT-8` corrected) |
| Lean audits 141 · example families 12 | `proof/Audit*.lean`; `examples/*/` | **exact** |
| `named_followups` 257 carried / 257 derived | the committed matrix | **exact** |
| Constraint 14: reserved source words | `qcore_frontend::frontend::is_reserved_source_word` | **113, not 112 — a finding** |

### 6.1 `CONSTRAINT-14-WORD-COUNT-CASE-FILTERED` (registered)

Constraint 14 names `is_reserved_source_word` as "the one place to count them"
and published **112**. Extracting every string literal from that function's
`matches!` arms with comments stripped returns **113** distinct words. The
difference is **`FULL`** — uppercase, sitting among the grade literals `zero`,
`one`, `affine`, `relevant`, `many`, `inf`. `DOCS-COMPACT-8`'s own artifact
(`artifacts/reserved-words.txt`, 112 lines, "extracted from the `matches!`
arms") is lowercase-only; the two sets were diffed and the symmetric difference
is `{FULL}` in one direction and empty in the other.

Corrected in `AGENTS.md` to **113**, with the case-sensitivity stated so the
next extraction is honest. This is `AGENTS-MAP-FIGURES-UNDERIVED`'s family a
third time — a figure whose stated authority is a piece of Rust that nothing
derives it from — with one aggravating difference worth naming: **the
constraint's sentence claims the derivation while the artifact beside it was
filtered**. Registered separately rather than folded into that entry because the
mechanism differs (a case-filtered extraction, not a second hand-kept copy), and
the new entry says so in its own `why`.

## 7. Untracked residual-shaped names

`--residuals` over the range returns **62** names at this close's tip (60 before
its edits). Classified one by one from the source text, in the wave-37 close's
classes plus one the compaction card's inventory tables made necessary:

| Class | Count | Names |
|---|---|---|
| Card names, not residuals | 22 | `CORPUS-REPIN`, `CT-PROOF-LEG-2`, `CT-WIRE-LANE-2`, `CT-WIRE-LANE-3`, `CT01-PROOF-LEG`, `CT03-PROOF-ALPHABET`, `CT07-TWO-THREAD-REMAP`, `CT22-CT-MAPPING`, `CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`, `CT46-SENDSYNC`, `EVENT-V1-WINDOWS-AFUNIX`, `G03-F1`, `G03-F2`, `P01-F2`, `REASON-CODES`, `REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `T00-CT-SCHEMA`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES`, `WAVE-38-CLOSE` |
| **Prospective** card names — proposed, never run, absent from the tree outside the report that proposed them | 5 | `CT-NATIVE-CREDIT`, `CT-PROOF-LEG-3`, `CT24-CLEANUP-TASK-COMPOSITION`, `DOCS-COMPACT-9`, `TSAN-FAILURE-ARMS` |
| Discharged or retired, still written where they were discharged (`MAIN-GUARD-UNACTIVATED`'s mechanism) | 20 | `BASE-TYPE-READER-UNMEASURED`, `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`, **`CT-COMPOSED-NATIVE-ROWS`** (retired here), `CT42-CELL-DERIVED-PREMISE`, `CT42-CELL-UDP-QUEUE`, `CT42-CELL-UNROUTED`, `CT42-MAP`, `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE`, `JOINED-PROVENANCE-FACTS-NAME-NOTHING`, `JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED`, `LEAN-AUDIT-GATE-KEY-OPTIONAL`, `MAIN-GUARD-UNACTIVATED`, `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`, `SOCKET-CELL-QUESTION-COMPOSED-ROUTES`, `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`, `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`, `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`, `W4-RECSUM-CONSTRAINT-4`, `WAVE-34-CLOSE-UNSWEPT`, `WLT-EXERCISE-TSAN-TEST-SILENT-ARMS` |
| Rules or prospective rule names | 4 | `DOCS-COMPACT-5-EXAMPLES-HOST-GATE`, `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE`, `W4-RECSUM-ALREADY-LANDED`, `W36-W37-MUTATION-HARNESSES-UNSHIPPED` (retired this wave by the reeval; the name survives as `CLAUDE.md`'s **rule** about unshipped harnesses) |
| Deferred decisions, not followups | 5 | `VERIFIER-HOST-ROW-DISPOSITION`, `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK`, `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS`, `TASK-ROUTE-TSAN-SEGV-PRIORITY`, and this close's own **`CHECKPOINT-PAGE-SPLIT`** (opened and resolved in the same record) |
| Verdict or bucket labels | 2 | `DECIDABLE-NOW`, `RED-AS-REQUIRED` |
| Constraint and invariant labels, not names | 3 | `C-14`, `INV-1`, `INV-6` — `DOCS-COMPACT-8`'s own shorthand for constraint 14 and invariants 1 and 6 in its inventory tables |
| Open, unregistered, and named in four documents | 1 | `OWNED-FIELD-WINDOWS-RUN` — `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED`'s open decision, re-listed in §3 |
| **Total** | **62** | 22 + 5 + 20 + 4 + 5 + 2 + 3 + 1 |

**Two of this close's three retirements are already silent**, and that is §7.1's
mechanism firing on this very close: `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`
appears twice and `INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE` once inside
other entries' `why` texts this close wrote, so the scan reads them as tracked
while they are absent by id. Only `CT-COMPOSED-NATIVE-ROWS`, which no entry
mentions, shows in the listing. Recorded so no later close reads that silence as
a registration.

### 7.1 `RESIDUAL-QUIET-BY-PROSE-MENTION` (registered) — the listing's other half

The diff against the wave-39 close's listing is not what a reader expects. **Five
names that close listed as untracked are silent here while still being absent
from `named_followups` by id**: `CT46-NOTES-PREDATE-TSAN-ROW`,
`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`, `CT42-CELL-FALLBACK-ARM-DIRECTION`,
`WAVE-38-QUEUE-NAMES-UNTRACKED` and `MUTATION-HARNESS-MTIME-RESTORE`. Each now
appears once or twice **inside another entry's `why`** — the reeval registered 23
entries whose texts cite earlier names — and `--residuals` scans the tracker's
whole text rather than its ids, so a prose mention reads as tracking.

`HTTP-V1-WINDOWS-RUN` is the standing case (absent by id, twice in prose, silent
for three waves). And this close demonstrated the mechanism on **itself**:
`WAVE-40-CLOSE` was in the listing before its tracker edits and dropped out
after, because `register.py` writes `"recorded_by": "WAVE-40-CLOSE"` into two
new entries.

The wave-39 close recorded this as a one-off "quiet for the wrong reason". It is
registered here as a **property of the tool**, because registering entries is
the ordinary way a wave makes names quiet: **the listing shrinks fastest exactly
when the most work is being recorded.** Discharge: match against ids, with a
separate "mentioned in another entry's `why`" bucket so nothing is lost, shown
red by mutation; or a recorded decision that a prose mention *is* tracking, with
the six names given entries or an explicit statement that their tracker is the
entry citing them.

## 8. The decisions this close took

### 8.1 `RECONCILIATION-VERIFY-CLAIM-PERISHABLE` — decided, and the class is wider than the entry said

The residual records one instance: a `verify` claim quoting the increments
index's mutable "Current record file" pointer, which the wave-opening commit
moved. **Measured at this base, the wave-39 close's own documented command fails
16 of its 57 claims, not one.** Each was attributed by testing its
whitespace-normalised needle against `git show <rev>:<path>` at four revisions:

| Broke at | Count | Where |
|---|---|---|
| `f11ccbba` (the wave-opening commit) | **1** | `docs/development/increments/README.md` — the entry's own instance |
| `0c7b0cea` (the `DOCS-COMPACT-8` merge) | **15** | 14 in `AGENTS.md`, 1 in `CLAUDE.md` |
| `d6a57f66` (the reeval merge) | 0 | — |

**Durability is a property of the target file, not of the wording.** Every claim
into the acceptance matrix (17), a frozen increment record (4), the spec (3),
crate source (2), the CT manifest and its fixture (2) and a frozen card report
(1) survived; **15 of the 27** into `AGENTS.md`/`CLAUDE.md` did not.

So the residual's **first** discharge arm — "a rule that a `verify` claim quotes
durable text" — **is not sufficient on its own**, and that is now measured
rather than argued. Verifying an `AGENTS.md` amendment *in `AGENTS.md`* is the
point of the arm; a compaction card is chartered to rewrite exactly that prose;
the two cannot both be satisfied by a wording rule.

**Decision, landed in `CLAUDE.md`:** a `--check` is green **at its own close and
only there**. Each close `--check`s its own record against its own tree and is
never obliged to re-green a predecessor's; a later red on a frozen close's
record is **evidence of supersession, not of a defect**, and must not be
repaired by editing that frozen record (constraint 21). Where an amendment
genuinely lives in a durable artifact, prefer that location. The entry's
discharge narrows to the **second** arm alone — a `--check` mode distinguishing
"applied and later legitimately superseded" from "never applied" — and stays
open.

*This close's own record is checked under that rule and makes no claim about any
later tree.* It also demonstrated the mechanism on itself while being written:
one `verify` claim quoting an `AGENTS.md` sentence went red when the close
sharpened that same sentence a few minutes later, and the fix was to re-quote,
not to soften the sentence. A claim into prose is perishable **by the minute**,
not only by the wave.

### 8.2 The checkpoint-page split — **kept**, with both index links

`wave-40.md` is **10,970** bytes and
`wave-40-compaction-checkpoints.md` **51,145**; inlining would make one
**62,115**-byte record page, against `wave-37.md`'s **64,613** — the size that
occasioned the split, and already twice `AGENTS.md`'s own "below 32 KiB where a
complete record boundary permits it". Beyond size: a verbatim compaction
checkpoint has a different reading contract from an increment paragraph, and
separating them keeps the record page one a reader finishes.

The wave-40 index row keeps **both** links (record page and checkpoint page),
against the "one link per wave" reading the card offered to drop: the index is
the cheapest place to make that distinction, and `wave-40.md`'s own header links
the checkpoint page too, so the two routes agree. Recorded as the deferred
decision `CHECKPOINT-PAGE-SPLIT`, `resolved`, so the next compaction pass
inherits it by the mechanism that re-lists decisions rather than by reading this
paragraph.

### 8.3 The integration-gate conventions

`INTEGRATION-GATE-NOT-PUBLISHED-FOR-EVERY-MERGE` is **decided on its second arm
and retired**. The rule is in `CLAUDE.md` beside the wave-39 convention:

> a merge whose message states a figure **differing** from the card's own
> `gate.json` must publish `docs/reports/<card>/integration-gate.json`; a merge
> whose stated figure **equals** the card's need not.

Applied retrospectively, as the arm requires, by dated appended sections on
`docs/reports/ct07-two-thread-proof-leg/README.md` and
`docs/reports/ct42-cell-route/README.md` — each carrying the merge's stated
figure, the **non-ancestor tip** it names and the **in-history merge sha**
beside it. Nothing above either heading is edited (constraint 21). No
retrospective `integration-gate.json` is written for either: the runs' logs are
not in the tree, so such a file would assert a measurement nobody can reproduce.
Wave 40's own two merges stated figures **equal** to their cards' gates, so
neither owed a file — measured, not assumed.

`INTEGRATION-TIP-CITATION-UNREACHABLE` stays **open**, amended with the two new
instances (seven in all). The one instance a close can repair without touching a
frozen record **is** repaired: the merged-gate row named `c188d3ab` and now
names this close's own in-history tips. The successor mapping is published here
rather than written *into* the three frozen `integration-gate.json` files, which
is constraint 21's dual-publication shape — so the arm asking for the sha inside
each such file remains unmet.

| Cited tip | Merge, in history | Where the citation lives |
|---|---|---|
| `f1fcd14d` | `0d9e922f` | merge message; now also `ct07-two-thread-proof-leg/README.md` |
| `ecfef946` | `45a33285` | merge message; now also `ct42-cell-route/README.md` |
| `3fa88fc6` | `e80b58d6` | `aggregate-owned-field/integration-gate.json` |
| `601acde6` | `0150c904` | `tsan-rows-2/integration-gate.json` |
| `4786ae4c` | `b6d1f101` | `loops-on-composed-routes/integration-gate.json` |
| `f661b2aa` | `d6a57f66` | merge message (wave 40, new) |
| `3cb27d80` | `0c7b0cea` | merge message (wave 40, new) |
| `c188d3ab` | `0c7b0cea` | `AGENTS.md`'s merged-gate row — **replaced by this close** |

### 8.4 The `clang` key is a disclosure convention, not a check

The second arm the reeval offers for
`INTEGRATION-GATE-CLANG-CLAIM-BORROWED` / `CLANG-RECORDING-UNBOUND-TO-ITS-RUN`
is taken, in those words: `AGENTS.md`'s standing-gate section now states that
the key is a disclosure convention rather than a check, names what
`clang_recording()` actually verifies and what it does not — no re-invocation,
no host identity, no timestamp, no commit binding, and the `clang-version.exit`
arm firing only **if** that file happens to exist — and instructs that
`clang --version` be run in the worktree that took the measurement. Both entries
are amended and stay **open** on the mechanical arm.

### 8.5 Constraint 14's pointer list, and the CT row's host condition

The five card names `DOCS-COMPACT-8` added to constraint 14's evidence-pointer
list **stay**, and the convention is now stated so the next pass need not guess.
Measured: **three** of the five appear as card names in the constraint's own
compacted body (`MUTEX-SHARE-CROSS`, `AGGREGATE-OWNED-FIELD`,
`CT42-CELL-ROUTE`) and **two** do not (`WRITE-LEND-TASKS-EXERCISE`,
`LOOPS-ON-COMPOSED-ROUTES`). The reading: the pointer list is the constraint's
**bibliography** and names every card that ran a census under it, whether or not
it reserved a word — the per-card ledger those two belong to having moved
verbatim to the checkpoint page — while the **body** names only the decisions a
refactor could undo. That is what the constraint's own Method paragraph already
warns about in the other direction, so no name is removed.

The CT row's sanitized-native count now carries its host condition, which
discharges `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS` word for word, and the
entry's own "asserting nothing" wording is corrected in the same sentence rather
than carried into the retirement: the gate is a runtime `cfg!` placed **after**
the reference-run and `c_source` assertions, so the **emission** row is
genuinely exercised on another host.

## 9. Boundaries

* Nothing here is promoted. No unit status, function status, corpus count, CT
  count or conformance claim moves; wave 40 moved no crate source, no Lean and
  no provider package.
* The 24 `noted` verdicts on the two re-swept frozen reports carry the **wave-39
  close's** measurements, not this close's.
* The gate figures are full-suite runs on one machine carrying its own build —
  not the quiet-host number `HOST-ROW-STARVATION-DESCRIPTION` asks for — and
  provider 122 is carried, not re-measured.
* The retrospective integration-figure notes **transcribe** the merge messages;
  they do not re-measure. One of those lines reads "0 failed" while naming three
  failed rows, which is `GATE-SUMMARY-PARSER-OK-ONLY` and is **not** repaired
  here; the note says so.
* `LEDGER-TSAN-BOUNDARY-SENTENCES-STALE`'s third part is **not** discharged, and
  this close did not take it either.
* No Windows or Apple observation is claimed by anything in this close.

## 10. Due, not run

* **`STABILITY-REEVAL` #5** — after wave 41. Firing #4's own nomination, now in
  `CLAUDE.md`: the **transcription family**
  (`LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION`,
  `CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND`, `PROOF-LEG-DIGESTS-UNCROSSCHECKED` —
  one defect wearing three names), and **the standing constraints' own
  sentences**, this firing having found two of them simply false.
* **`DOCS-COMPACT-9`** — around wave 42 or 43, the bullet's own arithmetic,
  confirmed and not overruled.
* **The next implementation wave's three candidates**, each with firing #4's §4
  conclusion attached, because it measured all three:
  * **`CT-PROOF-LEG-3`** — CT07's ledger pattern does **not** generalise; CT50
    is the only remaining F02 candidate; the proof numerator already counts two
    binding shapes, so a third leg buys a count and not a new kind of evidence.
    The name appears nowhere in the tree outside the reeval's report and this
    record.
  * **`CT-NATIVE-CREDIT`** — the premise is **spent**. The rows already read
    `executed` (this close retired `CT-COMPOSED-NATIVE-ROWS` for exactly that),
    and the platform qualifier that remained is landed here too, so the card is
    thinner than when it was proposed. What is left is a decision about how the
    count is printed, not a measurement.
  * **`TSAN-FAILURE-ARMS`** — **breaks the harness first**: a failed spawn may
    never reach the join the forcing gate opens at, so the arms need outcome
    hooks and a gate-closed control before any catch-rate row means anything.
* **Tooling-card material**, named rather than taken:
  `GATE-ROW-SUMMARY-COUNT-UNCHECKED` (two derived `--gate` keys),
  `CLANG-RECORDING-UNBOUND-TO-ITS-RUN` (a recording bound to its run),
  `PROOF-LEG-DIGESTS-UNCROSSCHECKED` (a gate script comparing each
  `theorem ct*_digest` with its `const`), `UNCARDED-BLIND-TO-MERGE-PROSE` (a
  combined-diff mode), `RESIDUAL-QUIET-BY-PROSE-MENTION` (id matching) and
  `RECONCILIATION-VERIFY-CLAIM-PERISHABLE` (a supersession-aware `--check`).

## 11. Reproduction

```
git submodule update --init --recursive
setsid -f docs/reports/wave40-close/scripts/gate.sh <dir> < /dev/null   # detached, exits to files (edit W= to your worktree)
python3 docs/reports/wave40-close/scripts/register.py --show            # then without --show
python3 -B docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave40-close/gate.json
python3 scripts/ct_stage_summary.py --check
python3 -m unittest discover -s scripts/tests
python3 docs/reports/wave40-close/scripts/build_reconciliation.py
python3 scripts/reconciliation_sweep.py . --since 6dc484ad --check docs/reports/wave40-close/reconciliation.json
python3 scripts/reconciliation_sweep.py . --since 6dc484ad --residuals
python3 scripts/reconciliation_sweep.py . --since 6dc484ad --uncarded
```
