# WAVE-38-CLOSE — one card, its integrator list swept, the two gate keys made required checks, the Windows F02 pin deferred with its Linux half landed — 2026-09-18

Base `4f663934`, the `STABILITY-REEVAL-3` merge, equal to `origin/main` at the
start; the only card running. Record: `reconciliation.json` (68 verdicts,
`verify_applied: true`), `gate.json` (this close's own measurement, carrying
the two keys the generator now **requires and checks**: `lean_audits` and
`clang`), `gate/` (every exit of both detached runs, the Clang recordings),
`mutations.log` (the generator arms and the F02 agreement test), `scripts/`
(the record builder, the tracker edit script, the harness, the gate script)
and this report.

## Headline

One card merged in wave 38, `STABILITY-REEVAL-3` (`4f663934`), the cadence's
third firing — an audit, fourteen residuals registered, nothing promoted. Its
§6 addressed **11** items to the integrator; every one was given a verdict
**measured against the tree** — the named file read for the proposed text or
its effect, never the card's claim — and, as at wave 37, **nothing was found
unapplied at its merge**: the two matrix amendments the merge applied (items 2
and 3) are in the tree in the card's words, item 3 with the location deviation
the merge recorded (§2).

What the close did beyond the sweep, each on the card's own ask:

1. **The Windows F02 pin stays where it is, and the Linux half of its
   discharge is landed** (§3). `windows_threads.rs:31,36` still pins the
   fourth-move digests; re-pinning them from Linux is `37bc9004`'s shape and
   was refused. A new host-unconditional test,
   `compiler/tests/f02_windows_pin_agreement.rs`, reads both files' literals
   and asserts they agree — **red at `4f663934` by design**, so it is
   `#[ignore]`d with the residual's name in its reason and reproducible on
   demand; shown green under a temporary edit of the Windows literals that was
   restored sha256-equal and never committed. The platform run is a deferred
   decision for the user, `F02-WINDOWS-PIN-RUN`, in the `HTTP-V1-WINDOWS-RUN`
   pattern (§9).
2. **The two gate keys are now required, not optional, and one of them is
   new** (§7). `build_matrix.py` refuses a `--gate` file lacking `lean_audits`
   (discharging `LEAN-AUDIT-GATE-KEY-OPTIONAL`) and a new `clang` presence
   claim is checked against the `clang-version.txt` recorded beside the gate;
   eight tests, six arms red by mutation, and the refusal lives only on the
   regeneration path — no frozen close's `gate.json` is ever opened.
3. **Three frozen reports corrected beside themselves and four mutation counts
   restated** (§4, §6), each as a dated, labelled, appended section under
   constraint 21; `CLAUDE.md`'s card-report convention gained the one line the
   card asked for: a mutation count is cited only with its script and log in
   the report directory.
4. **`recorded_by` deferred with the fact that decides it measured** (§5):
   49 of the 67 entries name one report directory, 18 name several, 0 map to
   a `recorded_by` value already in use.
5. **Two registrations, three retirements**, each named and verified (§8):
   `STRING-CONCAT-LOOP-WINDOW` and `AGGREGATE-OWNED-FIELD` registered from
   constraints 15h and 15k in those constraints' words; `SLICE-PERMISSIONS-LOOP-WINDOW`
   deliberately **not** registered (the constraint text states it as a rule);
   `WAVE-38-QUEUE-NAMES-UNTRACKED`, `LEAN-AUDIT-GATE-KEY-OPTIONAL` and
   `CLANG-GUARDED-GREEN-FAMILY-750` retired with their discharges met.

And what the close found about itself, unsmoothed (§11): the base gate's
`fmt` step exited **1** on this close's **own** unformatted new test file (the
wave-37/38 "own unrepaired test" shape — formatted, re-checked, exit 0, both
exits archived); a stale 34-name file in the **shared scratchpad** — another
session's — was nearly used as the `--residuals` before-list and was caught by
its count, `CLAUDE.md`'s hygiene rule doing its job; the first `--check` of
this close's own record was **red with 63 complaints**, all from six frozen
reports the appended notes made the sweep re-read (§2.1); and two retirements
went quiet in `--residuals` for the wrong reason (§10).

Counts: `named_followups` **229 → 228** (+2 −3); constraints unchanged in
number, one Q-space sentence appended (§11); CT **46/50**, sanitized-native
**10 of 46**, proof **3 of 46**, unmoved; corpus **88** modules, unmoved; the
merged-gate row re-measured under this close's provenance (§12).

## 1. Scope and method

The four commands of the close rule, each run from this worktree and each
output kept under the worktree-private `private/wave38-close/`:

```
python3 scripts/reconciliation_sweep.py . --since e69083a6                 # 11 items, 4 cards
python3 scripts/reconciliation_sweep.py . --since e69083a6 --residuals     # 44 names before, 45 after the tracker edits
python3 scripts/reconciliation_sweep.py . --since e69083a6 --uncarded      # 2 commits
python3 scripts/reconciliation_sweep.py . --cards stability-reeval-3 --residuals
python3 scripts/reconciliation_sweep.py . --since e69083a6 --check docs/reports/wave38-close/reconciliation.json
```

`--since` is inclusive, so the range opens at the wave-37 close merge
`e69083a6` and its four swept cards at the start were `stability-reeval-3`,
`wave37-close`, `wave38-close` (this report's skeleton) and `remaining-d02`
(the generator, touched by the wave-37 close and by this one). The tool
decides nothing; every verdict is a person's judgment against the tree, and
every `applied` verdict carries the text at its location (`verify`).

## 2. The sweep of `stability-reeval-3` §6

Eleven items. Verdicts: **applied 6** (items 2, 3, 4, 7, 8, 9), **deferred 2**
(items 1, 6), **noted 3** (items 5, 10, 11). No `declined`, no `superseded`.

| # | Item | Verdict | Where |
|---|---|---|---|
| 1 | `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` re-pin | **deferred** — a Windows run is the condition; the Linux-side agreement test landed `#[ignore]`d | §3, §9 |
| 2 | Amend `CT07-TWO-THREAD-PROOF-LEG`'s discharge | **applied at the merge** in the card's words (ASCII hyphen for the em-dash, the matrix's convention); "unusually cheap" deleted from *this* entry, verified by an id-scoped read | `reconciliation.json` |
| 3 | Amend `CT-FIXTURE-EXTERNAL-READERS`'s `why` | **applied at the merge**; the anchor sentence is in the entry's **title**, so the measurement was appended before the DISCHARGE clause — the location deviation the merge recorded, no wording change | `reconciliation.json` |
| 4 | Publish beside three frozen reports | **applied**, three dated appended sections | §4 |
| 5 | Retirements — the card retires nothing | **noted**; the three entries it names kept were verified registered by id | §8 |
| 6 | Decide `recorded_by` | **deferred** for the user, with the cheap fact measured | §5, §9 |
| 7 | Mutation harnesses | **applied** — four restatements beside the reports, one `CLAUDE.md` line | §6 |
| 8 | Gate keys | **applied** — `lean_audits` required, `clang` added and checked, tests red by mutation | §7 |
| 9 | Amend `MAIN-GUARD-REFUSES-PACK-REFS`'s discharge | **applied** — the second transaction and the main-checkout refusal named; the pin half stays open under `MAIN-GUARD-PACK-REFS-UNPINNED` | `reconciliation.json` |
| 10 | Lines for the queued cards' briefs | **noted**, carried for the record | §14 |
| 11 | Next firing | **noted**, carried for the record | §14 |

The id-scoped read behind item 2, so it is reproducible: the entry's `why`
contains `binding CT07's checked \`ScopedTasks\` module's lease set to the
Lean \`Reachable\` term` and does **not** contain `unusually cheap`; a
file-wide `absent` claim is impossible because
`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`'s `why` quotes the phrase it retired.

### 2.1 What `--check` found on this close's own record

The **first** `--check` was red: **63 complaints**. None was about the eleven
items. This close appended notes to six frozen reports (items 4 and 7), so
`--since` saw their directories move and swept their integrator sections
again — 62 item complaints across `ct-lane-5`, `joined-provenance-round-bound`,
`task-route-tsan`, `mutex-share-cross-emission` and
`write-lend-tasks-exercise`, plus `ct42-protocol-cell` with no section. Two
things were wrong, and they were repaired differently:

* **Five of the 62 were this close's own artifact.** Each appended note opened
  with a `---` rule, which the sweep read as one more item of the section
  above it. The six rules were stripped (my text, not the archive's; the
  `## Published beside …` heading already ends the previous section).
* **The remaining 57 had all been verdicted by the wave-36 or wave-37
  close** — measured by loading both records and matching every key
  (`private/wave38-close/prior_verdicts.py`: 14 + 12 + 10 from wave 37, 10 +
  11 from wave 36). They are carried as **`noted`**, each citing the prior
  record and its verdict, not re-measured; and `ct42-protocol-cell` (no
  integrator-addressed section; a wave-33 report) is in `no_section` with its
  ground.

The **second** `--check` exited **0**: 68 verdicts (6 applied, 2 deferred, 60
noted), **16 content claims verified at their named locations across 10
files**, 0 declared unverifiable. The lesson is recorded rather than
generalised: a publish-beside note is exactly the edit that makes the sweep
re-read a frozen report, and the record must say why each re-read item owes
nothing new.

## 3. Item 1 — the Windows F02 pin (deferred; Linux-side agreement test)

**Not re-pinned.** `compiler/tests/windows_threads.rs:31,36` still reads
`6df369991465940e` / `0da423be2f16b8bb` (the fourth-move pair) where
`f02_trace_relation.rs:96-97` reads `bfa572914cb96413` / `9cbe67a70a420d52`
(the fifth). The item's own condition for moving them is a card that records a
Windows run of
`windows_f02_renderings_satisfy_the_incoming_trace_relation_and_emission_pins`;
a Windows expectation re-pinned from Linux without a run is `37bc9004`'s
shape. That run is a user-validated platform run and is carried as the
deferred decision **`F02-WINDOWS-PIN-RUN`** (§9), in the `HTTP-V1-WINDOWS-RUN`
pattern.

**Landed: `compiler/tests/f02_windows_pin_agreement.rs`**, the Linux-side half
`F02-DIGEST-PIN-DUPLICATED` asked for. It `include_str!`s both sibling test
files and extracts the first 16-hex literal after each anchor (`const
ROUND_TRIP_C_DIGEST: &str = ` / `const CONCURRENT_C_DIGEST: &str = `;
`"round-trip",` / `"concurrent",`), so it runs on every host regardless of the
Windows file's `#![cfg]`. Two tests:

| Test | In the gate? | Result |
|---|---|---|
| `both_f02_pin_sites_are_still_where_this_test_reads_them` | yes | green: both extractions reach a 16-hex literal, both files spell `0xcbf2_9ce4_8422_2325` and `0x0000_0100_0000_01b3` |
| `the_windows_f02_pins_agree_with_the_linux_ones` | **`#[ignore]`**, reason names `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` | **red on `--include-ignored`** — the finding |

Measured (`mutations.log`, `private/wave38-close/mutations.py f02`):

```
F0 at the committed literals (--include-ignored): RED (RED is the finding); failing tests: the_windows_f02_pins_agree_with_the_linux_ones
F1 both Windows literals TEMPORARILY set to the Linux pair (never committed): GREEN (as required); failing tests: -
# restored: windows_threads.rs sha256 3dfabb8c…6aa4cd (equal)
```

So the test is non-vacuous in both directions and the Windows file is
byte-identical to the base. It is ignored so the standing gate stays green
while the red is reproducible on demand (`cargo test -p qcore-compiler --test
f02_windows_pin_agreement -- --ignored`); when the Windows-run card re-pins,
the `#[ignore]` comes off and `F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` and
`F02-DIGEST-PIN-DUPLICATED` retire together. The registered residual's `why`
now names the decision and the test (§8). Cost in the gate: one more ignored
test (8 → 9, §13).

## 4. Item 4 — publish-beside notes on three frozen reports

Each is a dated section headed `## Published beside by \`WAVE-38-CLOSE\`
(2026-09-18) — …`, appended after the frozen text with the sentence "Appended,
not edited (constraint 21)"; no archived line changed (`git diff` of each file
is additions only). Line numbers were **re-read at `4f663934`** before writing.

* **`docs/reports/ct42-protocol-cell/README.md`** —
  `CT42-CELL-FALLBACK-ARM-DIRECTION`. §3's last bullet calls the two
  "unreachable for admitted programs" arms "the direction that *refuses more*";
  `socket_footprint.rs:184` (`continue`) and `:211` (`None => acquire(entries,
  binding, None)`) give the binding a fresh singleton cell (`:167`) that the
  unedited T09 checker (`disjoint_footprint.rs:714-722`) judges **disjoint** —
  the admitting direction, harmless only because `:149` refuses first,
  load-bearing when `CT42-CELL-ROUTE` routes the deriver.
* **`docs/reports/task-route-tsan/README.md`** —
  `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`. §4.1's "two races" is what the
  card's un-halted run saw; the committed harness runs
  `TSAN_OPTIONS=halt_on_error=1` (`thread_sanitizer.rs:183`) and asserts
  `>= 1`. "CT46's emitted crossing" is `fork_join_trace()`
  (`mutex_c_emit.rs:843`), a hand-built twin nothing equates with the CT46
  fixture's lowering; `DISJOINT_EXERCISE` (`task_route_tsan.rs:65`) is the
  logger twin, and CT07's fixture has no TSan row.
* **`docs/reports/ct-lane-5/README.md`** — §2.4's `mutations.json` is the
  `mutations` key of `ct_stage_evidence.json` (line 237), and
  `measure/ct-lane-5/mutate.py` is on a gitignored path — so the note doubles
  as item 7's restatement for this card.

## 5. Item 6 — `recorded_by`: deferred with the cheap measurement

Not decided, and **no entry back-filled by guess**. The decision is carried as
**`MATRIX-RECORDED-BY-DECISION`** (§9) with the fact that decides it measured
(`private/wave38-close/recorded_by_measure.py`, log
`recorded_by_measure.log`, read-only over the matrix at the base):

| Of the 67 entries without `recorded_by` | Count |
|---|---|
| `source` names **exactly one** `docs/reports/<dir>/README.md` | **49** (26 distinct directories; 28 of the 49 are the five waves-36/37 cards' entries) |
| `source` names **several** report directories | **18** (2 to 10 each — e.g. `P04-NEXT-O` names ten, `O04-ARMS-O` nine) |
| `source` names no report directory | 0 |
| single-directory entries whose `DIR.upper()` is already some entry's `recorded_by` | **0** |

So a back-fill is an **inference** for 49 (the directory name is a card name
by convention, but no existing `recorded_by` value confirms any of them) and
**impossible by rule** for 18. Either arm of the card's discharge — back-fill
plus a five-key generator check, or a recorded decision that `source` is the
provenance key — is a decision, not an edit. The residual's `why` now carries
the measurement and names the decision; the decision names the residual
(`DECISION-ALSO-REGISTERED-AS-FOLLOWUP`'s discharge shape, so ending either is
visibly half the job).

## 6. Item 7 — the four mutation harnesses

Measured first: this worktree has no `measure/`, its `private/` holds only
this close's own files, and each of the four report directories was listed at
`4f663934` — **none of the four harnesses is in the tree.** So each count is
restated beside its report, in a dated appended section, as *measured once by
the card, harness not shipped*:

| Report | Count restated | Where the card said the harness was |
|---|---|---|
| `joined-provenance-round-bound` §7 | 6 of 7, one expected survivor | `measure/…/mutations.py`, no log |
| `ct-lane-5` §2.4 | 7 of 7 | `measure/ct-lane-5/mutate.py`; results in a file that is not there (§4) |
| `write-lend-tasks-exercise` §8 | 10 of 11, M5d's survival asserted | `private/mutate.py`; no catching test named per row |
| `mutex-share-cross-emission` §4 | a, a2, b, c, d and the later arms | no script named |

And the convention line, in `CLAUDE.md`'s "Delegated cards…" bullet: *a
mutation count is cited in a report only with its script and its per-mutation
log in that report directory; a count whose harness lives under `measure/` or
`private/` is written as "measured once, harness not shipped"*, with the
close's publish-beside restatement as the repair for an archived report. This
close holds itself to it: `mutations.log` and `scripts/mutations.py` are in
this directory.

## 7. Item 8 — the generator's `lean_audits` and `clang` gate keys

`docs/reports/remaining-d02/build_matrix.py`, on the `--gate` path only:

* **`lean_audits` is required.** `CHECKED_GATE_KEYS = ["lean_audits", "clang"]`
  and a `--gate` file lacking either is refused with a message naming
  `LEAN-AUDIT-GATE-KEY-OPTIONAL` / `CLANG-GUARDED-GREEN-FAMILY-750`. The
  wave-37 comparison against `proof/Audit*.lean` is unchanged and re-confirmed
  red by mutation (M6).
* **`clang` is a presence claim, checked against a recording.** The value is
  `present: <banner>` or `absent: <banner>`; the generator finds
  `clang-version.txt` beside the gate file (its directory, then a `gate/`
  subdirectory — `STABILITY-REEVAL-3`'s layout) and refuses when the banner
  is not that file's first line, when no recording exists, when
  `clang-version.exit` is recorded and disagrees with the claimed presence, or
  when the value is not of that form. Neither key is copied into the published
  block (`validation.lean_audits` stays derived; `clang` is not published).
* **The refusal applies to a regeneration, not to a frozen archive.** Every
  earlier close's `gate.json` lacks the keys; the generator opens only the one
  file named by `--gate`, so those archives are never read and never fail.
  `update_acceptance.py` passes no gate (checked).

Tests (`scripts/tests/test_build_matrix_sections.py`, 27 in the module, exit
0), with the `write_gate` helper writing the gate and its recording together:
`test_a_gate_file_lacking_lean_audits_is_refused`,
`test_a_gate_file_lacking_clang_is_refused`,
`test_a_clang_claim_agreeing_with_its_recording_is_accepted` (both layouts),
`test_an_absent_clang_claim_agreeing_with_its_recording_is_accepted`,
`test_a_clang_claim_disagreeing_with_its_recording_is_refused`,
`test_a_clang_claim_with_no_recording_beside_it_is_refused`,
`test_a_clang_presence_claim_contradicted_by_the_recorded_exit_is_refused`,
`test_a_clang_value_that_is_not_a_presence_claim_is_refused`; the three
pre-existing `--gate` tests were moved onto the helper.

Mutations (`mutations.log`; restore by rewriting, sha256 asserted equal):

| # | Arm mutated | Red? | By |
|---|---|---|---|
| M1 | the required-checked-keys refusal never fires | red | `…lacking_lean_audits…`, `…lacking_clang…` |
| M2 | a clang claim is never compared with its recording | red | `…disagreeing_with_its_recording…` |
| M3 | a missing clang recording is not refused | red | `…no_recording_beside_it…` |
| M4 | the presence claim is not checked against the recorded exit | red | `…contradicted_by_the_recorded_exit…` |
| M5 | any clang value is accepted as a presence claim | red | `…not_a_presence_claim…` |
| M6 | (wave-37 arm) the lean count is never compared with disk | red | `…disagrees_with_the_files_on_disk…` |

Six of six, `build_matrix.py` restored sha256-equal (`4a357a03…dc00b9`). This
close's own `gate.json` carries both keys, with `gate/clang-version.txt` and
`gate/clang-version.exit` beside it, and the matrix was regenerated through
that check (§12).

## 8. Registrations and the three retirements

**Registered (2)**, each with all five keys, from the constraint text that
names it and with the discharge in that constraint's words plus the card's
step-4 measurement:

* **`STRING-CONCAT-LOOP-WINDOW`** — constraint 15h: "the interval is exact
  because the route is one flat statement sequence, and a loop construct
  invalidates that premise — which is why the composed route **refuses** a
  `for` today"; discharge: revisit the window's computation for a loop
  construct (`last_view_uses` walking `Repeat` bodies), refusal kept in front
  until then, 15h amended when it lands — `LOOPS-ON-COMPOSED-ROUTES`' target.
* **`AGGREGATE-OWNED-FIELD`** — constraint 15k: "do not give a field custody
  without the four rules that do not exist"; discharge: the four rules landed
  in the IR first (an IR card before a route card), `Q055`'s lift stated
  deliberately, 15k amended in the same change; under the re-affirmed Q-space
  icebox it allocates no code.

**Not registered, deliberately:** `SLICE-PERMISSIONS-LOOP-WINDOW`. Constraint
15h states it as one of "three refusals that must survive refactors" — a
**rule**, exactly the class `CONSTRAINT-NAMED-BOUNDARIES-NOT-OPEN-WORK`
records — and the constraint text supports no other reading; "a card that
lifts one of those refusals must name it then." The new
`STRING-CONCAT-LOOP-WINDOW` entry says so.

**Retired (3)**, each printed in full by `scripts/register.py --show` and its
discharge read before removal:

1. `WAVE-38-QUEUE-NAMES-UNTRACKED` — discharge "register `STRING-CONCAT-LOOP-WINDOW`
   and `AGGREGATE-OWNED-FIELD` with the discharge conditions above": done.
2. `LEAN-AUDIT-GATE-KEY-OPTIONAL` — discharge "from the wave-38 close on, a
   `--gate` file lacking `lean_audits` is refused … with a test red by
   mutation": done, M1 red.
3. `CLANG-GUARDED-GREEN-FAMILY-750` — discharge "a clang presence key … checked
   by the generator the way `lean_audits` now is … **and** the 14-row entry
   re-scoped to the family or retired into this one": both done —
   `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN`'s `why` is amended to make it the
   family's tracker for the defect the key does not repair (750 guards report
   `ok` on a Clang-less host), so nothing is orphaned.

**Amended (5)** `why` texts: `MAIN-GUARD-REFUSES-PACK-REFS` (item 9),
`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36` (§3), `MATRIX-RECORDED-BY-ABSENT-67`
(§5), `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN` (above),
`GATE-ROW-FIGURE-ATTRIBUTION-DRIFT` (still partial: the "any other figure the
row publishes" arm and the row-level check remain unlanded).

`named_followups` **229 → 228**; `AGENTS.md`'s count line kept in step;
`dimensions.named_followups` re-derived by the regeneration (§12).

## 9. Deferred decisions re-listed

Twelve carried from the wave-37 close, **none resolved by the tree this wave**
(a one-card audit wave), the three already ended kept for accounting; three
added. Grounds are in `reconciliation.json`; the substance:

| Decision | Verdict | Ground, in short |
|---|---|---|
| `CT10-REASON-ROW` | deferred | sixth wave; no card touched CT10; still double-carried |
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | deferred | regenerated twice this wave, `updated` prose still wave 27's |
| `EXAMPLES-STYLE-SLICE-FANOUT` | deferred | `git diff --name-only e69083a6 HEAD -- examples/` is empty (measured) |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | deferred | wave 37's shape repeated: 2 uncarded commits, both a card's/close's own; the integrator's edits live inside `4f663934` |
| `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` | deferred | more partial: `F02-WINDOWS-PIN-RUN` joins the family; `HTTP-V1-WINDOWS-RUN` itself still absent from the tracker (measured by id) |
| `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` | deferred | data point: no compaction card fired and this close still corrected three frozen readings beside them |
| `VERIFIER-HOST-ROW-DISPOSITION` | resolved (wave 35) | accounting |
| `items` | withdrawn (wave 35) | accounting |
| `INDIRECT-CENSUS-FEATURE-UNBUILT` | deferred | nothing builds either feature still; both arms stand |
| `TASK-ROUTE-TSAN-SEGV-PRIORITY` | resolved (wave 37) | accounting; the remainder is registered |
| `CT07-TWO-THREAD-REPLAY-ORDER` | deferred | new evidence: CT07 has no TSan row at all and no F02 proof-leg route; the closing sentence still declined on wave 37's ground |
| `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS` | deferred | not taken; no shape definition designed |
| **`Q-NAMESPACE-DECISION`** | **deferred (re-affirmed, open)** | an integrator-recorded decision, landed as one appended sentence in `AGENTS.md`'s Q-space note: re-affirmed 2026-09-18 for wave 39 — condition extensions only; no card allocates a new `Q` code, including the aggregate-custody card, which widens the existing `StandardBuffer` and `Ownership` bands with each widening stated in §L21 |
| **`F02-WINDOWS-PIN-RUN`** | **deferred (new)** | a user-validated Windows run of the F02 row at or after the re-pin, in the `HTTP-V1-WINDOWS-RUN` pattern (§3) |
| **`MATRIX-RECORDED-BY-DECISION`** | **deferred (new)** | which arm; the 49/18/0 measurement (§5) |

## 10. Untracked names, classified

`--residuals` returned **44** names at the base (this close's own run; a
34-name `sweep-residuals.txt` found in the shared scratchpad was another
session's and was **not** used — its count gave it away) and **45** after the
tracker edits, both listings diffed under `LC_ALL=C`
(`private/wave38-close/residuals-{before,after}.names`):

* **44 at the base** = the wave-37 close's final **43**, each carried in the
  class that close gave it (`wave37-close/README.md` §5: 20 card names, 14
  discharged, 2 rules, 4 deferred decisions, 2 verdict labels, 1 named
  premise) and re-read rather than copied, **plus one new name**:
  `TSAN-ROWS-2`, a wave-39 card name written in the card's §4 and in
  `wave-38.md` — the `DOCS-COMPACT-8` future-card shape. 43 + 1 = 44.
* **+1 after the tracker edits:** `LEAN-AUDIT-GATE-KEY-OPTIONAL`, retired here
  and still written in the report that discharged it — the
  `MAIN-GUARD-UNACTIVATED` mechanism, expected.
* **Quiet for the wrong reason (2):** `WAVE-38-QUEUE-NAMES-UNTRACKED` and
  `CLANG-GUARDED-GREEN-FAMILY-750`, also retired here, do **not** appear —
  each is cited inside other entries' `why` texts (the two registrations';
  `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN`'s) and the scan reads the whole
  tracker text. Recorded so no later close reads the silence as a
  registration (the wave-37 close's `W37-MPACK-FAMILY-FIRST` mechanism).

After this report and the regenerated matrix were in place the listing was
re-run a third time (`private/wave38-close/residuals-final.names`): **45,
identical to the after-tracker listing — empty diff both ways.** The names
this report itself introduces move nothing, and the reason was measured
rather than assumed (each grepped in the matrix text): the three decision
names of §9 (`Q-NAMESPACE-DECISION`, `F02-WINDOWS-PIN-RUN`,
`MATRIX-RECORDED-BY-DECISION`), the queued card names of §14
(`CT42-CELL-ROUTE`, `LOOPS-ON-COMPOSED-ROUTES`), the platform-run name
`HTTP-V1-WINDOWS-RUN` and the standing-rule names this report cites
(`GATE-NO-FAIL-FAST`, `GATE-EXIT-SWALLOWED-BY-PIPE`,
`MUTATION-HARNESS-MTIME-RESTORE`, `HOST-ROW-STARVATION-DESCRIPTION`,
`PROVIDER-CONFORMANCE-UNIX-ONLY`) are **every one cited inside some tracker
entry's `why`** — the deferred decisions inside the entries this close
amended to name them — so the whole-text scan counts them as tracked: the
"quiet for the wrong reason" mechanism, this time on names that were never
meant to be registered (decisions must not be; card names and rules are not
work). `DOCS-COMPACT-8` and `TSAN-ROWS-2` are already in the 45 as card
names. So the final listing is the 44 of the base plus
`LEAN-AUDIT-GATE-KEY-OPTIONAL`, classified as above; **zero** names were
registered from any listing, and no third-party name was orphaned by the
three retirements.

## 11. The integrator's own record this wave

Step-3 material, not smoothed:

* **Edits at `4f663934`.** The merge's body says its integration went no
  further than the merge script's computed resolutions and the two matrix
  amendments of items 2 and 3 (with item 3's location deviation), the count
  line 215 → 229 and the regeneration with the wave-37 close's gate. This
  close verified each of those in the tree (§2) and **`--uncarded` agrees**: 2
  commits in the range, `05bbd149` (the card's own wave-opening commit) and
  `91e064c7` (the wave-37 close's WIP, reachable through the inclusive
  `e69083a6`) — zero integrator commits outside merges. The whole range is 5
  commits, every one attributable.
* **This close's mid-course addition.** One sentence appended to `AGENTS.md`'s
  Q-space note (the icebox re-affirmed 2026-09-18 for wave 39), recorded as an
  integrator-recorded decision and carried open in §9; no constraint number
  moved.
* **The base gate's `fmt` exit 1.** The detached run's `cargo fmt --check`
  found only `compiler/tests/f02_windows_pin_agreement.rs` — this close's own
  file, written after the gate's start (06:03Z) and before its fmt step
  (06:17Z). Formatted with `rustfmt --edition 2024`, re-checked: exit 0, empty
  diff. Both exits are in `gate/base/` (`fmt.exit` = 1, `fmt-post.exit` = 0).
  The final gate (§13) ran on the formatted file.
* **The shared-scratchpad near-miss.** A `sweep-residuals.txt` with 34 names
  sat in the session scratchpad from another agent; a diff against it would
  have reported 12 added and 1 removed. Caught by the count (this close's own
  listing had 44), re-derived from this close's own run, and every listing
  this close cites now lives under `private/wave38-close/` — `CLAUDE.md`'s
  hygiene rule, observed once more to be load-bearing.
* **The first `--check` red** (§2.1): 63 complaints, all from re-swept frozen
  reports, repaired by stripping six rules of this close's own text and by
  carrying 57 prior verdicts as `noted`.

## 12. Merged-gate row, re-measured

Written under this close's provenance from `gate.json` (§13's figures), the
Lean count and the Clang presence under the two keys the generator now
requires and checks; the row in `AGENTS.md` replaced with these numbers and
this attribution. Provider 122 is carried, not re-measured (`git diff
--name-only e69083a6 HEAD -- providers/ crates/qcore-schemas/` is empty;
`qcore-schemas` still 14.0.0; a Unix figure).

The row now reads **4,639 passed / 0 failed / 9 ignored, 234 result
summaries over 232 test targets** (223 `Running` + 9 doc-test sections) ·
**137** Lean · **303** Python · 122 provider (carried) · both probes 0 ·
fmt/clippy clean · Clang **present** (21.1.8, recorded and checked), measured
by the wave-38 close at `19c61952`; the tripwire sentence updated to this
wave's `229 != 228`; the attribution sentence now names both required keys.
Deltas against the wave-37 close's row: workspace **+2 passed** (the card's
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`, this
close's `both_f02_pin_sites_are_still_where_this_test_reads_them`), **+1
ignored** (this close's, red by design), Python **+8** (the generator tests),
Lean **+0**, one more `Running` target (the new test file). The other
headline numbers were re-read from their sources and are unmoved: 228
followups (matrix file and derived dimension agree after the regeneration),
88 contract modules, CT 46/50 with 10 and 3, the corpus row.

## 13. Gate

Two detached runs, each `setsid -f … < /dev/null`, no `trap '' TERM`, every
exit to its own file with no pipe in the command (`GATE-NO-FAIL-FAST`,
`GATE-EXIT-SWALLOWED-BY-PIPE`), `git submodule update --init --recursive`
first (exit 0, `third_party/libevent` at `d82464a2`). Exits under `gate/base/`
and `gate/final/`; the Clang recording under `gate/`.

| Step | Base run `78bc2c0e` (= `4f663934`'s code) | Final run `19c61952` (every code change of this close) |
|---|---|---|
| started / cargo end / end | 06:03:18Z / 06:17:36Z / 06:18:53Z (14m18s / 15m35s) | 06:22:39Z / 06:37:42Z / 06:38:03Z (15m03s / 15m24s) |
| `cargo test --workspace --no-fail-fast` | exit 0 · **4,638 / 0 / 8** over 233 summaries (222 + 9; loose grep 235) — the merge's own integration figure exactly | exit 0 · **4,639 / 0 / 9** over 234 summaries (223 + 9; loose grep 236) · `test result: FAILED` 0 times |
| `cargo fmt --all -- --check` | **exit 1** — the diff was this close's own new, unformatted `f02_windows_pin_agreement.rs` (§11); formatted, re-checked **exit 0** (`fmt-post.exit`) | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 (the new file already present) | exit 0 |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | 0 / 0 |
| `check_lean_audits.py` | exit 0 · 137 | exit 0 · **137** |
| Python (`unittest discover -s scripts/tests`) | exit 0 · 303 OK (before the tracker edits) | **exit 1** · 303 run, **1 failure**: `test_each_derived_dimension_matches_the_section_it_counts`, `229 != 228` — the `D02-DIMENSIONS-FOLLOWUP-COUNT` tripwire against the not-yet-regenerated matrix, wave 36's and 37's shape; **re-run after the regeneration: 303 OK, exit 0** (`gate/python-post.exit`) |
| Clang | present, 21.1.8, `QCORE_CLANG` unset, exit 0 | the same, recorded as `gate/clang-version.txt` and checked by the regeneration |
| disk | 310 → 266 GB | 266 → 266 GB |

The final run's cargo step was not interrupted: the wait this close had
started on its `DONE` marker was not a tracked child of the session, and
while checking for the process by hand this close briefly renamed the run's
output directory (seconds; restored before any later step wrote; the log's
open descriptor followed the inode) — recorded here because it is the kind of
thing a gate report must say; every step's exit file was written to the
intended directory, as the table shows. The `--check` of this close's own
record ran three times: red (63, §2.1), then 0, then **0 again after the
regeneration** (`gate/check-3.log`): 68 verdicts, 16 content claims verified
across 10 files. Constraint 16's rows: all five named rows in this workspace
green in both runs, and the unnamed seventh; a full-suite figure, not the
quiet-host number. Provider 122 carried (§12). Nothing promoted.

## 14. Next wave, for the record

Not briefed and not run by this close. Wave 39 is the five compiler cards,
each with a paragraph in `STABILITY-REEVAL-3` §4 naming the claim that is
load-bearing today only because a construct is unreachable, and each with the
line the card's §6 item 10 asks the brief to carry:

* **`CT42-CELL-ROUTE`** — make both fallback arms `Err` (or prove pairing
  agreement) and take the T09 edit first; the frozen report's "refuses more"
  is corrected beside it (§4). `CT42-CELL-FALLBACK-ARM-DIRECTION`,
  `CT42-CELL-DERIVED-PREMISE`.
* **`AGGREGATE-OWNED-FIELD`** — an IR card before a route card; the owner
  vocabulary is `String` everywhere; `Q055` is the deliberate lift; now a
  registered entry (§8); under the re-affirmed icebox, condition extensions of
  the `StandardBuffer` and `Ownership` bands only, each stated in §L21.
* **`CT07-TWO-THREAD-PROOF-LEG`** — ledger-side binding only
  (`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`); the discharge was amended at the
  merge (item 2).
* **`TSAN-ROWS-2`** — two cards, not one: a `QCORE_TASK_CREATE` harness card
  for the task route, and a recorded decision for CT46 (impossible by
  construction); `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`' discharge is its
  material (§4).
* **`LOOPS-ON-COMPOSED-ROUTES`** — rewrite `last_use`/`last_view_uses` over
  `Repeat` bodies under a runs-many-times semantics, refusals kept in front;
  its three names are now classified — `STRING-CONCAT-LOOP-WINDOW` registered,
  `SLICE-PERMISSIONS-LOOP-WINDOW` a rule to be named when lifted,
  `WRITE-LEND-RELEND` present.

`DOCS-COMPACT-8` is due around wave 40. **`STABILITY-REEVAL` #4** is due after
wave 39; the card's §6 item 11 names its highest-value grounds — the Windows
arm (this wave's one live red row now carried as `F02-WINDOWS-PIN-RUN`) and
the tracker-only residuals (18 mechanism-needing entries, zero discharged in
two waves).

## 15. Boundaries

* Nothing is promoted: no corpus count, unit status or function status moves;
  raw units stay unpromoted; CT 46/50 with sanitized-native 10 and proof 3;
  contract corpus 88; full language conformance **false**.
* The Windows F02 row is **still red on every Windows host**; this close made
  the disagreement visible on Linux and did not repair it. No Windows or Apple
  run of anything is claimed.
* The `clang` key records presence at the gate's host; it does not make the
  750 guards report `ignored` — that defect is `RETURN-CALL-TESTS-CLANG-GUARDED-GREEN`'s,
  re-scoped to the family (§8).
* The 57 `noted` verdicts on re-swept frozen reports carry the prior closes'
  measurements, not this close's; the 49/18/0 `recorded_by` measurement is a
  fact about `source` strings, not a back-fill.
* The gate figures are full-suite runs on a machine carrying its own build,
  not the quiet-host number `HOST-ROW-STARVATION-DESCRIPTION` asks for.

## 16. Reproduction

```
git submodule update --init --recursive
setsid -f docs/reports/wave38-close/scripts/gate.sh <dir> < /dev/null       # detached, exits to files (edit W= to your worktree)
python3 docs/reports/wave38-close/scripts/recorded_by_measure.py .
python3 docs/reports/wave38-close/scripts/register.py --show                  # then without --show
python3 docs/reports/wave38-close/scripts/mutations.py generator mutations.log
python3 docs/reports/wave38-close/scripts/mutations.py f02 mutations.log
python3 -B docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave38-close/gate.json
python3 docs/reports/wave38-close/scripts/build_reconciliation.py
python3 scripts/reconciliation_sweep.py . --since e69083a6 --check docs/reports/wave38-close/reconciliation.json
```
