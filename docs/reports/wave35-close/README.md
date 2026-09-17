# WAVE-35-CLOSE — seven cards, two arbiters, four losses found by measuring — 2026-09-18

Base `4cf9bdf9`, the `JOINED-PROVENANCE-CYCLES` merge. Evidence:
[`gate.json`](gate.json), [`reconciliation.json`](reconciliation.json),
[`scripts/`](scripts).

## Headline

Wave 35 merged **seven** cards and ran **two arbiters**, wave 31's shape. The
close gave all **78** integrator-addressed items a verdict **measured against
the tree** — the target file grepped for the proposed text or its effect, never
inferred from a report's claim that it had landed — and that distinction found
**four things**, three of them real losses:

1. **`CT-CTWIN-LANE`'s entire §9 item 1 was unapplied.** At the close's base
   `grep -c CT46 AGENTS.md` returned **0** and the ledger still read
   *45/50 mapped, 5 unmapped*, although the card merged at `cfd410d0` with
   three further cards merging after it. Landed here.
2. **`MUTEX-SHARE-CROSS-ARMS`' single ledger retirement was dropped at its
   merge.** Its four additions and its successor entry all landed; the removal
   the card's own arithmetic specifies ("one replaced, four added") did not, so
   the ledger carried `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS` **beside the
   successor entry whose own text says it discharged it**.
3. **`MUTEX-SHARE-CROSS`'s whole integrator list was still unlanded three waves
   on**, exactly as `SWEEP-BLIND-SPOTS` measured. Its standing constraint is
   landed verbatim as **15o** and both namespace rows are updated.
4. **Arbiter #1's proposed constraint `5c` contained a clause arbiter #2
   falsified later in the same wave**, so it could not land in the words
   proposed. Recorded `superseded` with both texts.

**Nothing is promoted.** No unit or function status moved at either arbiter,
the contract corpus is unchanged at 76, CT coverage grew by **mapping** with
both numerators unmoved, and full language conformance stays **false**.

## 1. Verdict distribution

| Verdict | Count |
|---|---|
| `applied` | 44 |
| `noted` | 27 |
| `superseded` | 3 |
| `declined` | 2 |
| `deferred` | 2 |
| **Total** | **78** |

`python3 scripts/reconciliation_sweep.py . --since a300a39d --check
docs/reports/wave35-close/reconciliation.json` exits **0**. That means every
swept item carries a recorded human verdict and **never** that any item was
applied — the tool's own words, and this close's 44/78 is why the distinction
matters.

### 1.1 Every non-`applied` verdict, with its ground

**Declined (2)** — both halves of one decision, taken in the card's own words.

* `c-gateway-constant-do-while#7` and `#8` — the merged-gate row's Lean figure.
  The card proposed a **multi-tip** form (`130 Lean audits (129 at the
  DOCS-COMPACT-6 tip; …)`) to avoid re-attributing a row to a tip at which it
  was not measured, and offered the decline arm itself: *"If you would rather
  the row stay a single-tip measurement, decline this and record it declined."*
  **Ground:** the close **re-measured the whole row** at its own base, so every
  figure in it is one measurement of one tree. The multi-tip parenthetical
  would have left the row carrying `DOCS-COMPACT-6`'s qualifications from two
  waves back, and the figure proposed (130) is itself two tips stale — the wave
  added **three** audits, not one. The card's reasoning was right and its
  conclusion obsolete, which is the ordinary fate of a "needs your decision"
  item a later measurement answers.

**Deferred (2)** — both because the work is a card's or the user's, not a
close's.

* `ct-ctwin-lane#11` — an owner for `CT-FIXTURE-CROSS-CASE-DUPLICATES`.
  **Ground:** the item asks for an owner *outside the lane*, no card of wave 35
  owns the CT02/CT14 or CT03/CT04 fixtures, and assigning one is work
  allocation. The entry is registered, measured and explicitly "not fixed", so
  nothing is lost by carrying it; the close adds no new information and says so
  rather than inventing a disposition.
* `mutex-share-cross-arms#9` — the second reader on
  `docs/reports/c-gateway-return-call-w4/scripts/mutations.py`. **Ground:**
  reading a second card's harness for a restore-semantics defect is a
  *measurement*, and a close performing it would be doing card work inside a
  reconciliation. The deferral is made visible outside this file: CLAUDE.md's
  new bullet says in its own text that the second reader is outstanding, and
  `MUTATION-HARNESS-MTIME-RESTORE` carries it.

**Superseded (3)** — each a figure or a text a later measurement overtook.

* `c-gateway-constant-do-while#2` — the corpus baseline pointer, applied by the
  card and superseded in the same wave by arbiter #2, which is the chain rule
  working normally.
* `c-gateway-constant-do-while#12` — constraint `5c`; see §3.
* `mutex-share-cross-arms#1` — the Lean row `129 → 130`, superseded by the
  close's own whole-row re-measurement at **132**.

## 2. The 62 residual-shaped names, by class

`--residuals` returned **62** names the tracker does not carry. This is the
**first honest count**, not a wave-35 regression: `SWEEP-BLIND-SPOTS` widened
the scan past `docs/reports/` to `docs/development/increments/` and the
standing-constraints text, and narrowed `TRACKERS` to `named_followups` alone,
so it now sees names from waves 29–30 the old scan could not reach by
construction. Each was classified individually.

| Class | Count | Disposition |
|---|---|---|
| **Open work → registered** | **27** | Each with a discharge condition drawn from its own source report |
| **Card names, not residuals** | **22** | No registration owed |
| **Discharged residuals** | **6** | Discharging card named below |
| **Decisions already carried** | **3** | In `deferred_decisions`; registering them as follow-ons too would double-count |
| **Rules / prospective names** | **3** | Not open work |
| **A bucket label** | **1** | `DECIDABLE-NOW` |
| | **62** | |

**Registered (27).** The five live `CONSTANT-DO-WHILE-*`; the six
`VERIFIER-HOST-*`/`HOST-*` from this wave's host card; three
`MUTEX-SHARE-CROSS-*`; five `CT42-*`; three `CT-COMPOSED-NATIVE-*`; two
`CT-PROOF-*`; and `W31-SIZE-SIBLINGS-REMAINDER`, `WRITE-LEND-RELEND`,
`RECONCILE-SWEEP-DIMENSIONS-UNPINNED`.

**Card names (22).** Fourteen have an exactly-matching `docs/reports/`
directory. `CT46-SENDSYNC` and `EVENT-V1-WINDOWS-AFUNIX` have directories under
other names (`remaining-ct46-sendsync`, `windows-std-event-af-unix`).
`DOCS-COMPACT-1` is a past card and `DOCS-COMPACT-7` a future one. The four
`FB-*` are **queued** cards from the Fluent Bit plan — column 1 of
`STABILITY-REEVAL` #1 §5's table is headed "Queued card" — which the wave-34
close had already classified the same way.

**Discharged (6), with the discharging card named.**

| Name | Discharged by |
|---|---|
| `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` | `JOINED-PROVENANCE-CYCLES` (wave 35) — see §4 |
| `W29-CONTENT-SIZE-UNDECLARED` | `W31-CORPUS-CONTRACTS-2` (wave 31) |
| `W29-SIZE-SIBLINGS-UNDECLARED` | `W31-CORPUS-CONTRACTS-2`, *partly*; remainder registered as `W31-SIZE-SIBLINGS-REMAINDER` |
| `W4-RECSUM-CONSTRAINT-4` | `W32-RECONCILE-SWEEP` (wave 32) |
| `DOCS-COMPACT-5-MIGRATED-SECTION-CLAUSE` | `W32-RECONCILE-SWEEP` (wave 32) |
| `CT22-CT-MAPPING` | `CT22-WIRE` (wave 29) — CT22 mapped |

**Rules / prospective names (3), not open work.**
`CT24-CLEANUP-TASK-COMPOSITION` is explicitly the name a **future** card
*would* have ("would be its name if a future card takes…").
`DOCS-COMPACT-5-EXAMPLES-HOST-GATE` was marked **standing** and deliberately
not reworded by `W32-RECONCILE-SWEEP`. `W4-RECSUM-ALREADY-LANDED` records a
stale brief that its own card then verified.

### 2.1 After-state, reported honestly

Re-running `--residuals` returns **37**, and that is **not** 62 − 27 = 35: the
delta is measured, not assumed.

* **32 left** the untracked set — 27 registered as their own entries, and
  **5 because they are now named inside another entry's prose**, which
  `ledger_text` counts as tracking *by design* (`WAVE-33-DEFERRED-DECISIONS-
  UNTRACKED` deliberately registered four names as one entry). Those five are
  `VERIFIER-HOST-CONTROL-BUDGET`, `W31-CORPUS-CONTRACTS-2`,
  `W29-CONTENT-SIZE-UNDECLARED`, `W29-SIZE-SIBLINGS-UNDECLARED` and
  `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED`. **Prose tracking is weaker than
  an entry**, and saying so is the point of measuring the delta.
* **7 joined** it, all as a consequence of this close's own retirements: five
  are the retired entries themselves, still written in reports and constraints
  (`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`, `WAVE-33-DEFERRED-DECISIONS-UNTRACKED`,
  `WAVE-34-CLOSE-UNSWEPT`, `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE`,
  `FB-PLAN-GATE-HOST-TIMEOUT`), which is correct — a discharged name is not
  open work. The other two, **`CT10-REASON-ROW` and
  `MATRIX-UPDATED-STALE-AT-WAVE-27`, were tracked only by the prose of the
  entry this close retired**, and they are *deferred decisions*, now carried by
  the re-listing mechanism instead. That is the right home for a decision and a
  weaker one for a name, and it is recorded rather than smoothed over.

## 3. The two constraint-text decisions

### 3.1 Constraint `5c` — landed, `superseded`, with both texts published

The rule landed whole. **Two clauses of the card's text did not**, and
AGENTS.md's own `5c` says so in its body rather than only here.

* **The stale clause.** The card wrote that removing a back edge makes a
  function acyclic *"and `legacy_joined_provenance::derive` returns no
  conditional-provenance facts at all for a cyclic CFG (constraint 10b)"*.
  Wave 35's **second** arbiter falsified that later in the same wave.
  **Verified in source, not inferred:**
  `crates/qcore-c-gateway/src/legacy_joined_provenance.rs:151` returns
  `fixed_point(cfg, &reachable)`, and constraint 10b was itself rewritten by
  that card to say so. Landing the sentence verbatim would have put a statement
  **contradicting 10b into the same file**. It reads *"and that changes which
  route `legacy_joined_provenance::derive` takes for it (constraint 10b)"*.
  **The rule is unchanged**: a widening of the key still moves two subsystems
  and must re-measure both.
* **The script path.** The card wrote `measure/.../acyclicity.py`, its own
  gitignored worktree-private path; replaced by the committed copy the card
  archived, `docs/reports/c-gateway-constant-do-while/scripts/acyclicity.py`,
  verified to exist.

Neither substitution changes what the rule forbids, and the verdict is
`superseded` rather than `applied` so the difference is never read as a silent
softening (invariant 1).

### 3.2 Constraint `15o` — landed verbatim, three waves late

`MUTEX-SHARE-CROSS`'s §13.1 block quote, reproduced by `SWEEP-BLIND-SPOTS` §6.1
and landed **word for word**, including its own opening and closing citations.
The card's measurement was reproduced before acting on it: `grep -c
'MUTEX-SHARE-CROSS' AGENTS.md` returned **0** at the base, while
`docs/QCORE_SPEC_r0_3.md:1004` — the authority AGENTS.md calls "a summary of it,
not a second allocation" — carried the widening in full. The summary had drifted
from its authority for one card.

**One thing is the integrator's and is labelled as such in the file:** the
constraint **number**. The card left it unassigned; `15a`–`15n` were taken.

## 4. `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` — answered, not registered

Five of arbiter #1's six residuals are registered; the sixth deliberately is
not, which is a better outcome than six. Its discharge condition was that
arbiter #2 quote its pre-pass coverage against **this card's tree** rather than
`a300a39d`, and state whether any of the 654 newly-acyclic functions changes an
answer. It did, clause by clause: coverage quoted against `148eef79`, the
654-function flip reproduced independently (1,927 − 654 = 1,273 cyclic), and
the effect stated — **zero**, at a cost of six functions and 20 leaves of
servable population moving to the topological pass, *reach that moved, not
reach that was lost*. Registering an answered residual would put closed work in
the open-work ledger.

## 5. The ledger

`named_followups` **103 → 121**: **28 added** (27 from the classification, plus
`DOCS-COMPACT-4-TABLE-SYNC`) and **10 retired**. Every retirement names a
discharging card and was measured at this close, not read off a report:

| Retired | Measured evidence |
|---|---|
| `WAVE-32-CLOSE-UNSWEPT` | `wave32-close/reconciliation.json` now exists with **67** verdicts over all seven of wave 32's cards; before, that directory held only `gate.json` |
| `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` | The widened scan returned 62 names including whole families from waves 29–30 increments |
| `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` | 11 of the 62 came back annotated "a RULE, not a tracker", and **every one proved to be a card name** — the distinction the repair exists to force |
| `WAVE-BASE-UNSWEPT` | `--uncarded` returned four commits and the wave-open commit `a300a39d` was among them — the exact commit the old exclusive range could not see |
| `WAVE-33-DEFERRED-DECISIONS-UNTRACKED` | `--check` re-listed all seven deferred decisions; this record restates each |
| `WAVE-34-CLOSE-UNSWEPT` | `wave34-close/` now holds README, gate and reconciliation, the last with **25** verdicts |
| `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` | Discharged by option **(c)** — investigation *plus* repair; control passes 38/38 |
| `FB-PLAN-GATE-HOST-TIMEOUT` | "Closes when that one does", in its own text |
| `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS` | The removal `MUTEX-SHARE-CROSS-ARMS` asked for and the merge dropped |
| `W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE` | Both arms answered; see below |

Derived and confirmed a **byte-identical generator fixed point**, regenerated
through `docs/reports/remaining-d02/build_matrix.py . --gate
docs/reports/wave35-close/gate.json` — **this close's own gate**, which is what
`CT-CTWIN-LANE` §9 item 3 asks for and what restores an `integrator_supplied`
provenance naming this wave. Previous closes carried the **wave-33** gate
deliberately, because wave 34 had no close directory; that reason is gone.
`scripts/tests/test_build_matrix_sections.py` green at **17/17**.

> **Note for the next close.** The generator **refuses** a `gate.json` carrying
> keys outside its fixed nine. This close's first attempt added `measured_at`,
> `lean_audits` and split pass/fail/ignored counts and was rejected with exit 1;
> those figures were folded into the note instead.

### 5.1 Removals lost at merges — a pattern, named

`named_followups` is hand-maintained (constraint 25), and at a merge resolution
its **additions survive while its removals are dropped**. Three instances in
three waves: wave 32's `DOCS-COMPACT-O` row lost 2,208 bytes at `3bbec4d0`;
wave 34's close lost `PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW` and the
integrator restored it by hand at `8d158c4e`; and this wave
`MUTEX-SHARE-CROSS-ARMS`' single retirement was dropped while its five
additions landed. **No standing rule is proposed here** — that is a card's work,
and `RECONCILE-SWEEP-DIMENSIONS-UNPINNED` (registered) already asks for the
tripwire that would catch a drifted derived count. What is recorded is that a
close must diff this section in **both directions**, which is how all three
were found.

### 5.2 A shape defect in the wave-34 deferred-decisions block

Measured here, repaired beside the record rather than in it (constraint 21).
The wave-34 close wrote its `deferred_decisions.items` as a **map** of
name → sentence; `reconciliation_sweep.decisions_in()` reads `items` only when
it is a **list** of `{name, verdict, ground}` objects. Measured:
`_decisions_of(pathlib.Path('docs/reports/wave34-close'))` returned exactly
`['items']` and **none** of that close's seven decisions. So the three
decisions wave 34 **raised** reached the re-listing mechanism nowhere and would
have expired in precisely the silence that mechanism was landed in the same
wave to prevent; the four inherited from wave 33 stayed visible only because
the wave-33 record happens to use the list shape.

All seven are registered in the new
[`docs/reports/wave34-close/deferred_decisions.json`](../wave34-close/deferred_decisions.json)
— the sibling file the tool provides for exactly this case — and the wave-34
record is **unedited**. The bogus `items` pseudo-decision is `withdrawn` in this
close's record with its ground.

## 6. Deferred decisions — all seven re-listed

Wave 34 carried seven; **four are wave 33's**, now carried for a third wave.
**These are the user's and this close takes none of them.** One is recorded
`resolved` only because a *card* was assigned to it and discharged it.

| Decision | Verdict | New evidence this wave |
|---|---|---|
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | deferred | The matrix has now been **regenerated at every wave-35 merge** — by three cards passing the wave-33 gate deliberately, and by this close with its own — while `updated` still carries wave 27's prose. The gap is **wider and more visible**, not narrower |
| `CT10-REASON-ROW` | deferred | `CT-CTWIN-LANE` re-measured it and recorded CT10 staying unmapped "on half a scenario" — a lane that could have moved it did not |
| `EXAMPLES-STYLE-SLICE-FANOUT` | deferred | **None.** No card touched `examples/`. Re-listed so it does not expire in silence |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | deferred | `--uncarded` returned four commits, three of them exactly the shapes the decision is about; each accounted for **by eye**, which is what the absence of a rule costs every wave |
| `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` | deferred | The three files **grew again here**, and every addition was a correction landing **late** — `5c`, `15o`, a rewritten constraint 16, a CLAUDE.md bullet. Arguably evidence for the second reading, since here corrections *did* land outside a compaction pass |
| `VERIFIER-HOST-ROW-DISPOSITION` | **resolved** | By the card it was assigned to, not by this close. Its question — whether option (a) *alone* retires the entry — no longer arises: the card took option **(c)** |
| `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` | deferred | The split the decision calls "neither" has **widened**: this wave added `VERIFIER-HOST-READINESS-APPLE-UNRUN` and `CT-COMPOSED-NATIVE-APPLE` to the registered side, while the six CLAUDE.md names that prompted it still are not |

## 7. Uncarded commits

`--uncarded` returned **four**, each accounted for by eye:

| Commit | What it is |
|---|---|
| `a300a39d` | "Open wave 35" — the wave-open commit, the exact shape `WAVE-BASE-UNSWEPT` names and now visible |
| `b6ea42df` | Integrator fix: a pre-existing `--since HEAD` test rewritten to `--range HEAD..HEAD` under the new inclusive base |
| `8d158c4e` | Integrator correction: restoring `PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW`, lost at the wave-34 close's merge |
| `b954b753` | `MUTEX-SHARE-CROSS-ARMS`' own code commit, its report landing in a later commit of the same card |

**None needs a card.** The tool's caveat holds: writing no report is *necessary*
for being uncarded, not sufficient.

## 8. Gate

Measured at `4cf9bdf9` with a clean tree before any of this close's edits.
`git submodule update --init --recursive` first, **exit 0**,
`third_party/libevent` at `d82464a2`. **Every exit captured to its own file
with no pipe in the command** — a `| tail` swallowed a gate exit for the
integrator earlier in this wave.

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,580 passed / 0 failed / 8 ignored**, 230 result summaries, 228 executables, **exit 0** |
| `cargo fmt --all -- --check` | **exit 0** |
| `cargo clippy --all-targets -- -D warnings` | **exit 0** |
| `reevaluation_probe` | **exit 0** |
| `reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **`Passed 132 Lean audits`**, **exit 0** |
| `python3 -m unittest discover -s scripts/tests` | **247 tests, OK**, **exit 0** |

One uninterrupted run of 15m54s. `grep -c "test result: FAILED"` over the log is
**0**, so no isolated re-verification was required and **none is claimed**.

**Which contested row this run saw**, as constraint 16 requires:
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
**passed**. This is the **first full-suite observation after**
`VERIFIER-HOST-CONTROL-BUDGET`'s `poll(2)` repair and it agrees with that card's
38/38 — **but it is not the quiet-host isolated number
`HOST-ROW-STARVATION-DESCRIPTION` still asks for, and is not offered as one**: a
full-suite run at load ≈6.5 is not a quiet account. Constraint 16's other
workspace rows were green too.

Provider **122 carried, not re-measured**: `git diff --name-only a300a39d^ HEAD
-- providers/ crates/qcore-schemas/` is **empty**, so constraint 18 is not
engaged. It stays a Unix figure, and its `RLIMIT_NPROC` row was separately
measured **unverifiable under this wave's conditions**
(`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`).

Deltas: workspace 4,559 → **4,580** (+21), Lean 129 → **132** (+3, one per
arbiter plus the arms card), Python 229 → **247** (+18, wholly
`SWEEP-BLIND-SPOTS`').

## 9. Two cards, one test row — reconciled

`CT-CTWIN-LANE` and `VERIFIER-HOST-CONTROL-BUDGET` described the **same row in
one wave** and neither could see the other. The lane proposed **host
starvation**, explicitly not vouching for it; the other card **measured the
mechanism** and it was a **transport defect** — the row failed when the host was
**least** loaded, the opposite sign — an 8 KiB pipe capacity against a 1 ms
backoff quantum, repaired with `poll(2)`, **test file untouched** (verified:
`git diff --stat cc7dbe78^ cc7dbe78` names `src/lib.rs` and no test).

`HOST-ROW-STARVATION-DESCRIPTION` is therefore **amended in place, not
retired**, because its two discharge arms have different fates: the first
("replace the standing description") **is** discharged, in constraint 16, in the
other card's own words; the second ("re-measure on a quiet machine") **is not** —
**no unloaded-host number exists anywhere in the tree**. The entry is narrowed
to exactly that.

## 10. What this close does not claim

* **A green `--check` is not a green repository.** It says 78 items carry
  verdicts. Thirty-four of them are not `applied`.
* **The close decided no deferred decision.** The one `resolved` was discharged
  by a card, and the close records that rather than doing it.
* **No unit, function or verdict moved.** Corpus counts stand as arbiter #1 left
  them and arbiter #2 confirmed; `double_release` 1 → 0 was a **withdrawal**,
  not a discharge.
* **CT 46/50 is mapping, not conformance** (constraint 23). Sanitized-native
  stays **9** and proof stays **3**: both denominators grew and neither
  numerator did.
* **Prose tracking is weaker than an entry**, and five names left the untracked
  set that way rather than by registration (§2.1).

## 11. For the integrator and the next wave

* **`STABILITY-REEVAL` #2 is due after this wave** — every two waves, and #1 ran
  in wave 34. **Noted, not performed.** Three of this close's measurements are
  the kind it consumes: `HOST-BACKOFF-FALLBACK-ARMS-UNDRIVEN` is a fresh
  "defensive, not verified" arm in a subsystem repaired **this wave** (method
  item 1); `HOST-ROW-STARVATION-DESCRIPTION` is a boundary with one arm
  discharged and the other unmeasured (method item 2); and §5.1's
  removals-lost-at-merges is a semantic edit made with no card and no residual
  name, three times over (method item 3).
* **`DOCS-COMPACT-7` falls around wave 37**, and
  `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` sets its rule. This close adds one
  data point and does not take the decision.
* **The constraint count moved**: `5c` and `15o` are new, so the next compaction
  must see them.
