# WAVE-39-CLOSE — five cards swept, one uncounted tally corrected, the integrator's own record kept, two retirements and three registrations — 2026-09-18

Base `b6d1f101`, the `LOOPS-ON-COMPOSED-ROUTES` merge, equal to `origin/main`
at the start; the compiler-side wave of five cards, the only card running at
the close. Record: `reconciliation.json` (106 verdicts, `verify_applied: true`),
`gate.json` (this close's own measurement at its own tip, carrying the two keys
the generator requires and checks), `gate/` (every exit of both detached runs,
the Clang recording, the regeneration and check exits), `scripts/` (the record
builder, the tracker edit script, the CT46 citation script, the gate script),
`register-show.log` / `register.log` / `cite_ct46.log` and this report.

## Headline

Five cards merged in wave 39, in this order and each through its own full gate
at the integration tip: `CT07-TWO-THREAD-PROOF-LEG` (`0d9e922f`, proof 3 → 4),
`AGGREGATE-OWNED-FIELD` (`e80b58d6`, owner paths in the IR and the `bytes`
field on the record route), `TSAN-ROWS-2` (`0150c904`, forced-schedule TSan
rows and the CT46 decision), `CT42-CELL-ROUTE` (`45a33285`, CT42 mapped,
47/50) and `LOOPS-ON-COMPOSED-ROUTES` (`b6d1f101`, loan windows computed over
iterations). Their integrator-addressed sections gave this close **49** items;
every one was given a verdict **measured against the tree** — the named file
read for the proposed text or its effect, never the card's claim (§2):
**applied 31, superseded 5, declined 2, noted 11**. Nothing was found
unapplied at its merge. One thing was found **uncounted** at its merge: the
Q-space note's hand-kept tally had missed `CT42-CELL-ROUTE`'s five condition
widenings (§6). A further **57** items belong to five frozen reports inside
the inclusive range and are carried `noted` citing the wave-36/37 records that
verdicted them (§2.1). `--check` on this close's own record: **exit 0** on the
second run (the first had one complaint, this close's own quotation broken
across a `///` line).

What the close did beyond the sweep, each on a card's own ask or the brief's:

1. **The integrator's own record, unsmoothed** (§4): a summary-parser
   transcription defect in merge `0d9e922f`'s measured line (registered,
   `GATE-SUMMARY-PARSER-OK-ONLY`); the deferred-regeneration convention and its
   three `integration-gate.json` files — **accepted**, written into `CLAUDE.md`,
   swept here, and its borrowed Clang claim registered
   (`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`); one prose union in constraint 14
   verified; two rebases; one intermittent Python row, now named.
2. **The mutation-harness restore hazard registered with a survey** (§5):
   `MUTATION-HARNESS-STALE-ARTIFACTS`, after measuring all seventeen shipped
   harnesses (none mtime-preserving today); `MUTATION-HARNESS-MTIME-RESTORE`
   retired on both halves of its discharge, the second reader done.
3. **CT46's two texts cited** (§3): the two-text TSan citation on the CT07
   precedent, `CT46-NOTES-PREDATE-TSAN-ROW` retired with its discharge met word
   for word.
4. **The `ct_corpus.rs` doc comment decided** (§3): its hand-kept figures
   ("7 of 43", "3 of 43", against a ledger at 10 of 47 and 4 of 47 — the third
   drift of the same sentence) removed rather than refreshed a fourth time.
5. **Two hand-kept ledgers corrected and completed** (§6): the Q-space tally
   (`CT42-CELL-ROUTE` the seventeenth, `LOOPS-ON-COMPOSED-ROUTES` the
   eighteenth, eighteen on ten bands) and constraint 14's wave line, measured
   against `git diff` over the front end; `CLAUDE.md`'s cadence lines updated
   from "#3 is due now" and "next due around wave 39".

Counts: `named_followups` **233 → 234** (+3 −2, one amendment); constraints
unchanged in number (15p landed by a card; none by this close); CT **47/50**,
sanitized-native **10 of 47**, proof **4 of 47**, verified against the manifest
and unmoved by this close; corpus **88** modules, unmoved; the merged-gate row
re-measured under this close's provenance (§7). Wave 39 closed at **one
reserved word** (`disjoint`), the first since wave 26.

## 1. Scope and method

The commands of the close rule, each run from this worktree through a wrapper
script so every exit lands in a file, outputs kept under the session
scratchpad's `w39/` directory and re-run rather than copied:

```
python3 scripts/reconciliation_sweep.py . --since e878b362                 # 106 items, 14 cards swept
python3 scripts/reconciliation_sweep.py . --since e878b362 --residuals     # 56 names before, 56 after the tracker edits
python3 scripts/reconciliation_sweep.py . --since e878b362 --uncarded      # 15 commits (17 with this close's own)
python3 scripts/reconciliation_sweep.py . --cards ct07-two-thread-proof-leg aggregate-owned-field tsan-rows-2 ct42-cell-route loops-on-composed-routes --residuals   # 48 names
python3 scripts/reconciliation_sweep.py . --since e878b362 --check docs/reports/wave39-close/reconciliation.json
```

`--since` is inclusive, so the range opens at the wave-38 close merge
`e878b362` (44 commits by `git rev-list --count e878b362^..b6d1f101`: 39 after
the base — five merges and 34 branch commits — plus `e878b362` and the four
wave-38 close branch commits reachable only through it). Fourteen cards were
swept: the five of the wave; five frozen reports the wave-38 close and
`TSAN-ROWS-2` appended notes to; and four with no integrator-addressed section
(`ct42-protocol-cell`, `remaining-d02`, `wave38-close`, this report), each with
its ground in `no_section`. The tool decides nothing; every verdict is a
person's judgment against the tree, and every `applied` verdict carries the
text at its location (`verify`: 57 claims, 50 present and 7 absent, across 10
files).

## 2. The sweep of the five cards' integrator sections

49 items. Verdicts: **applied 31**, **superseded 5**, **declined 2**,
**noted 11**. The full notes are in `reconciliation.json`; the substance:

| Card (section) | Items | Applied | Superseded | Declined | Noted | What the close itself did |
|---|---|---|---|---|---|---|
| `ct07-two-thread-proof-leg` (§11) | 7 | 5 | 0 | 0 | 2 | item 6: the `ct_corpus.rs` doc comment decided (§3) |
| `aggregate-owned-field` (§12) | 6 | 5 | 1 | 0 | 0 | item 6: `OWNED-FIELD-WINDOWS-RUN` named in `CLAUDE.md`'s platform list, the `Q103` sentence kept and its alternative deferred with `Q-NAMESPACE-DECISION`, the `AGGREGATES-V1` §3.1 note declined |
| `tsan-rows-2` (§8 "left registered", §11) | 3 + 9 | 1 + 6 | 0 + 1 | 0 + 1 | 2 + 1 | item 6: the CT46 citation (§3); item 7: a standing rule **declined** with its ground; item 8: the wave paragraph says executed-schedule coverage |
| `ct42-cell-route` (§9) | 12 | 7 | 1 | 0 | 4 | item 12: the Q-space tally corrected (§6); item 11: `CT-CTWIN-DESCRIPTOR` located inside `CT-CTWIN-O`'s roll-up (§5) |
| `loops-on-composed-routes` (§12) | 12 | 7 | 2 | 1 | 2 | item 8: `MUTATION-HARNESS-STALE-ARTIFACTS` registered with the survey (§5); item 7: two decisions declined with grounds; item 4: renumbered eighteenth |

The two `declined` verdicts, with their grounds:

* **`tsan-rows-2` §11 item 7** — a standing constraint for "a forced-schedule
  harness must be invisible to the race detector it serves". Declined: one
  forcing header rides one seam; the rule is enforced by a shipped mutation
  that goes red by *missing* the race when the header is instrumented
  (`mutate.sh`'s `hook_visible_to_tsan`); `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`'s
  `why` is indexed and carries the rule with its discharge. A numbered
  constraint restating a tracker entry for a surface with one instance is the
  accretion the compaction rule bounds; the condition for landing one is a
  second forcing harness or a second seam, at which point the entry's text is
  the constraint's, as the card said.
* **`loops-on-composed-routes` §12 item 7** — (a) whether the composed route
  should accept an unrolling licence before a `for`: not taken, a design choice
  for the card that lifts `LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`, where the
  card recorded it; a close deciding a route's grammar without a card is the
  shape invariant 6 forbids. (b) Whether `SLICE-PERMISSIONS-LOOP-WINDOW`'s
  lifting should be recorded as a retired rule somewhere other than 15h:
  declined — the constraint text is the authority for a rule, and 15h quotes
  the superseded sentence in place with the lifting card named, which is the
  wave-38 close's own decision; a second record would be a second authority.

The five `superseded`: three per-merge matrix regenerations that could not use
the card's `gate.json` and used an integration gate (§4), superseded again by
this close's regeneration; `ct42-cell-route`'s Lean count 140 (141 after
`LOOPS`); and `loops-on-composed-routes`' "sixteenth" (seventeenth at the
merge, eighteenth here, §6).

### 2.1 What `--check` found on this close's own record

The first `--check` (`gate/`-adjacent scratch log `check-1`) was red with
**one** complaint: this close's own `verify` quotation for the `ct_corpus.rs`
comment spanned two `///` lines, and the sweep's whitespace normalisation does
not strip comment leaders. The claim was narrowed to one line's text; the
second `--check` exited **0**: 106 verdicts, 57 content claims verified at
their named locations across 10 files, 31 of 31 `applied` verdicts carrying
checkable text, 0 declared unverifiable. The 57 re-swept items — `ct-lane-5`
(14), `joined-provenance-round-bound` (12), `task-route-tsan` (10),
`mutex-share-cross-emission` (10), `write-lend-tasks-exercise` (11) — were each
verdicted by the wave-36 or wave-37 close and carried `noted` by the wave-38
close; they are re-swept here because the wave-38 close's appended notes sit
inside the inclusive range and `TSAN-ROWS-2` appended a further note to
`task-route-tsan`. Each is carried `noted` citing the original record, not
re-measured.

## 3. Taken at the close on a card's ask

* **The CT46 citation** (`tsan-rows-2` §11 item 6; `scripts/cite_ct46.py`,
  `cite_ct46.log`). On the CT07 precedent (`tsan-rows-2/scripts/cite_ct07.py`)
  one sentence is appended to the manifest's CT46 `semantic_profile` and to
  `compiler/tests/fixtures/ct/CT46/expected.json`'s `notes`, naming the row
  (`task_route_tsan.rs::ct46s_own_case_runs_clean_bound_and_the_forcing_header_finds_no_seam`)
  and the recorded decision in the merge's own words. Neither text is
  byte-pinned; no stage moves. The manifest's three other changed lines are an
  escaping difference only (`—` → `—`), shown semantically identical after
  parsing (`gate/manifest-semantic-compare.log`).
  `CT46-NOTES-PREDATE-TSAN-ROW`'s discharge is met in its own words and the
  entry is retired (§5).
* **The `ct_corpus.rs` doc comment** (`ct07-two-thread-proof-leg` §11 item 6).
  Above `ct_mapping_progress_is_pinned_and_counted` the prose read "7 of 43"
  and "3 of 43" against a ledger at 10 of 47 and 4 of 47 — the third drift of
  a sentence that already recorded two. Decided: the figures are **removed**,
  the sentence now points at the authority (`PROOF_GATED_CASES`, the runner
  registry, the manifest through `ct_stage_summary.py --check`) and records the
  three drifts. A comment edit only; `cargo fmt --check` and the whole
  workspace are green on it (§7).
* **`OWNED-FIELD-WINDOWS-RUN`** (`aggregate-owned-field` §12 item 6): named in
  `CLAUDE.md`'s Windows platform list on the family's existing footing
  (`SLICE-PERMISSIONS-WINDOWS-RUN`'s shape), not registered, until
  `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` is decided.

## 4. The integrator's own record this wave

Step-3 material (`CLAUDE.md`'s `STABILITY-REEVAL` method: semantic edits made
without a card), not smoothed:

1. **A transcription defect in a merge message.** `0d9e922f`'s measured line
   reads "workspace 4620 passed / 0 failed / 9 ignored" and in the same
   sentence names three constraint-16 verifier-host rows that **failed** under
   four sibling builds (re-verified 12/12 and 12/12 in isolation). A parser
   summing only `test result: ok.` summaries drops every binary whose summary
   reads `FAILED`. The brief to this close states the true totals as
   **4,641 / 3 / 9**; 4,641 + 3 = 4,644 is the card's own figure at
   `81f783ce`, the corroboration available here (the run's log is not in the
   tree and this close did not re-derive the triple). No ledger figure carried
   it — the merged-gate row at the base is the card's own 4,644 / 0 / 9 and this
   close re-measured — so the defect lives in a permanent merge message and in
   whatever produced its line. Registered: **`GATE-SUMMARY-PARSER-OK-ONLY`**,
   discharge a parser that counts every `test result:` line and is shown to
   reproduce the right triple from a log carrying a `FAILED` summary.
2. **The deferred-regeneration convention — three `integration-gate.json`
   files.** At `e80b58d6`, `0150c904` and `b6d1f101` no card's `gate.json`
   could agree with the merged tree (two sibling cards had added Lean audits and
   the generator refuses a `lean_audits` figure that disagrees with disk), so
   the integrator measured each tip and wrote the file beside the card's
   report, saying so in the merge message:

   | File | Tip | Workspace | Lean | Python | Clang claim |
   |---|---|---|---|---|---|
   | `aggregate-owned-field/integration-gate.json` | `3fa88fc6` | 4,677 / 0 / 9 | 139 | 303 (re-run after the regeneration) | taken from the card's `gate/clang-version.txt` |
   | `tsan-rows-2/integration-gate.json` | `601acde6` | 4,683 / 0 / 10 | 139 | 303 | the card's |
   | `loops-on-composed-routes/integration-gate.json` | `4786ae4c` | 4,727 / 0 / 10 | 141 | 303 | the card's |

   **Verdict: the convention is accepted** and now written in `CLAUDE.md`'s
   delegated-cards bullet — an integrator who cannot regenerate with any
   card's gate measures the tip, writes the file, says so in the merge
   message, and the close sweeps every such file into its own record (this
   table) and re-measures the row regardless. **The generator is deliberately
   not taught a second layout**: its `--gate` path already accepts the file,
   and a code change to the generator needs tests red by mutation, which is a
   card's work, not a close's. What the generator's check cannot see is
   registered: each file's Clang claim was checked against the **card's**
   recording beside it — same directory, likely the same host, but a recording
   of a sibling run — so the wave-38 check was met vacuously three times
   (**`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`**, discharge an integration
   gate's own `clang-version.txt`/`.exit` in a layout the generator reaches, or
   a recorded decision stating the claim is the card's in the file's note).
3. **One prose union.** Constraint 14's ledger at `b6d1f101` carries CT42's and
   LOOPS' bullets after AGGREGATE's with CT42's re-worded run sentence kept —
   read in full and correct; this close added the wave line after them (§6).
4. **The merged-gate row's provenance.** Resolved to main's at every merge and
   re-measured at each amend, the row at the base still names the proof-leg
   card's own tip `81f783ce` as its measurement — replaced by this close's
   figure under its own provenance (§7).
5. **Two rebases ordered** (CT42 twice, LOOPS once). `ct42-cell-route` §7
   records its two superseded runs ("kept only in the worktree-private
   directories") and the union proved on the final tip before its gate; its
   session was cut once by a rate limit mid-rebase and resumed (the brief's
   record; the report's gate note is the citation). `loops-on-composed-routes`
   §12 item 9 records its union one line per file.
6. **One intermittent Python row at the `TSAN-ROWS-2` amend** —
   `test_every_audit_runs_once_with_the_sequential_command`
   (`scripts/tests/test_check_lean_audits.py:141`), green on re-run; the
   brief's record, no log in the tree. `PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW`'s
   `why` is amended: a row is named for the first time, and what it does is
   measured — it patches `run_step` and asserts the command list
   `check_audits` issues through its `ThreadPoolExecutor`
   (`check_lean_audits.py:160`), a concurrency-shaped row, not the wall-clock
   one the wave-34 hypothesis named. Discharge (a) is half met; whether it
   joins constraint 16 or is repaired is still undecided.
7. **`--uncarded`**: 15 commits in the range write no report and every one is a
   card's own branch commit by prefix and files (CT07 3, AGGREGATE 5, TSAN 1,
   CT42 3, LOOPS 3); **zero integrator commits outside merges**. The
   integrator's edits live inside the five merges — the retirements and
   registrations (one the integrator's own, at `0150c904`), the count lines,
   the regenerations, the prose union — indexed by nothing but the merge
   messages, which is again the only reason this close could verify them
   (`SYNC-RULE-TOOLING-COMMIT-SCOPE`, §8).

## 5. Registrations, retirements, and the untracked names

**Registered (3)**, each with all five keys and a discharge condition
(`scripts/register.py`, `register.log`):

* **`MUTATION-HARNESS-STALE-ARTIFACTS`** — `loops-on-composed-routes` §12 item 8's
  proposal, the `mv`-of-a-`cp`-backup route to the mtime hazard. Measured over
  every shipped harness in `docs/reports/` (seventeen, by `find docs/reports
  -name 'mutat*'`): **twelve** restore by rewriting (`write_text`/`write_bytes`;
  `ct07-two-thread-proof-leg` and `stability-reeval-3` also `os.utime` into the
  future), **four** by `cp` from a backup, which writes a fresh mtime
  (`c-gateway-return-position` without a `touch`; `task-route-tsan`'s two and
  `tsan-rows-2`'s with one), **one** by `mv` plus `touch` (the loops card's own,
  repaired). None is mtime-preserving today; the `CLAUDE.md` rule now names the
  mechanism. Discharge: the survey made mechanical — a script red on an
  mtime-preserving restore, with a test red by mutation, run at each close.
* **`INTEGRATION-GATE-CLANG-CLAIM-BORROWED`** — §4 item 2.
* **`GATE-SUMMARY-PARSER-OK-ONLY`** — §4 item 1.

**Retired (2)**, each printed in full by `register.py --show`
(`register-show.log`) and its discharge read before removal:

1. `CT46-NOTES-PREDATE-TSAN-ROW` — "a citation appended to both CT46 texts on
   the CT07 precedent, naming the row and the recorded decision": done, §3.
2. `MUTATION-HARNESS-MTIME-RESTORE` — "read the other harness, and land the
   proposed `CLAUDE.md` measurement-hygiene bullet": the bullet is in
   `CLAUDE.md` (it said the second reader was not done), and
   `docs/reports/c-gateway-return-call-w4/scripts/mutations.py` was read by this
   close — it restores by `path.write_text(text)` (lines 120, 131), a rewrite.
   Both halves met; the family goes forward under the new entry.

**Amended (1)**: `PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW` (§4 item 6).
`named_followups` **233 → 234**; `AGENTS.md`'s count line rewritten as the
wave's chain (228 → 227 → 232 → 234 → 231 → 233 → 234, each step attributed)
and kept in step; `dimensions.named_followups` re-derived by the regeneration.

**The untracked names.** `--residuals` returned **56** at the base and **56**
after the tracker edits and the `CLAUDE.md` amendments (the before listing is
this close's first run, captured from its output; the after run overwrote the
file, so the before set was written back from that output and the two diffed
in Python — added `CT46-NOTES-PREDATE-TSAN-ROW`, removed
`MUTATION-HARNESS-STALE-ARTIFACTS`). Classified one by one from the source
text, in the wave-37 close's classes:

| Class | Count | Names |
|---|---|---|
| Card names, not residuals | 21 | the wave-37 close's 13 (`CT-PROOF-LEG-2`, `CT-WIRE-LANE-2`, `CT-WIRE-LANE-3`, `CT01-PROOF-LEG`, `CT03-PROOF-ALPHABET`, `CT22-WIRE`, `CT24-CLEANUP-PREEMPTED`, `CT46-SENDSYNC`, `EVENT-V1-WINDOWS-AFUNIX`, `REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `W29-CORPUS-CONTRACTS`, `W4-RECURSIVE-SUMMARIES`), the six historical (`CORPUS-REPIN`, `G03-F1`, `G03-F2`, `P01-F2`, `REASON-CODES`, `T00-CT-SCHEMA`, from `remaining-d02`), `DOCS-COMPACT-8` (future) and `WAVE-38-CLOSE` (the previous close, written in the reports it appended to) |
| Discharged, still written where they were discharged (`MAIN-GUARD-UNACTIVATED`'s mechanism) | 24 | the wave-37 close's 14; wave 38's `LEAN-AUDIT-GATE-KEY-OPTIONAL` and `WAVE-38-QUEUE-NAMES-UNTRACKED`; **wave 39's eight**: `CT07-PROOF-LEG-PATTERN-UNAVAILABLE` (`CT07-TWO-THREAD-PROOF-LEG`), `CT42-CELL-UNROUTED`, `CT42-MAP`, `CT42-CELL-FALLBACK-ARM-DIRECTION`, `CT42-CELL-DERIVED-PREMISE`, `CT42-CELL-UDP-QUEUE` (`CT42-CELL-ROUTE`; the last two also named in constraint 15p as rules), `TSAN-SUBJECTS-ARE-TWINS-NOT-FIXTURES`, `WLT-EXERCISE-TSAN-TEST-SILENT-ARMS` (`TSAN-ROWS-2`) |
| Rules or prospective names | 2 | `DOCS-COMPACT-5-EXAMPLES-HOST-GATE`, `W4-RECSUM-ALREADY-LANDED` |
| Deferred decisions, not followups | 4 | `VERIFIER-HOST-ROW-DISPOSITION`, `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK`, `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS`, `TASK-ROUTE-TSAN-SEGV-PRIORITY` |
| Verdict or bucket labels | 2 | `DECIDABLE-NOW`, `RED-AS-REQUIRED` |
| Acted on by this close | 3 | `MUTATION-HARNESS-STALE-ARTIFACTS` (registered); `OWNED-FIELD-WINDOWS-RUN` (named in `CLAUDE.md`, §3); `SOCKET-CELL-QUESTION-COMPOSED-ROUTES` (withdrawn by `CT42-CELL-ROUTE` itself before merge on its superseded HTTP measurement — the merge message says so — and still written in its report) |
| **Total** | **56** | 21 + 24 + 2 + 4 + 2 + 3 |

After the edits: +`CT46-NOTES-PREDATE-TSAN-ROW`, retired here and still
written in the report and record that discharged it (expected, the discharged
class); −`MUTATION-HARNESS-STALE-ARTIFACTS`, registered. **Quiet for the wrong
reason (1)**: `MUTATION-HARNESS-MTIME-RESTORE`, also retired here, does not
appear — it is cited inside the new entry's `why` and the scan reads the whole
tracker text; recorded so no later close reads the silence as a registration.
`CT-CTWIN-DESCRIPTOR` (`ct42-cell-route` §9 item 10, "left open") is likewise
silent because it lives inside `CT-CTWIN-O`'s roll-up `why`; the roll-up is
its tracker and it was not registered twice. `HTTP-V1-WINDOWS-RUN` is still
absent from the tracker by id (§8). Zero names were registered from any
listing without a discharge condition; no third-party name was orphaned by
the two retirements. The listing was re-run a third time after this report and
the final regeneration (§7).

## 6. Headline numbers verified against their sources

Re-read from the sources, never from the ledger:

* **CT row.** `scripts/ct_stage_summary.py --check` exit 0
  (`gate/ct-stage-check.exit`): mapped 47 / unmapped 3, proof 4,
  `native_sanitized` 8 stage rows plus the two harness-gated rows the row counts
  as sanitized-native 10 (the manifest's authority, unchanged by this close);
  `AGENTS.md`'s row reads exactly `47/50 mapped, 3 unmapped; sanitized-native
  10 of 47; proof 4 of 47` with both wave-39 sentences. A grep for `46/50`,
  `of 46`, `45/50`, `of 45`, `3 of 4x` and `9 of 4x` over `AGENTS.md`,
  `README.md`, `DEVELOPMENT.md` and the increments index returns nothing: no
  stale figure remains outside checkpoint-labelled history.
* **Constraint 14's ledger.** The three family bullets (`AGGREGATE-OWNED-FIELD`
  +0, `CT42-CELL-ROUTE` +1 `disjoint`, `LOOPS-ON-COMPOSED-ROUTES` +0) read
  correctly after the prose union. The two cards not in either family are
  added to the "does not engage" bullet with their reports' and merge messages'
  own words, and the seam is **measured** rather than read: `git diff
  --name-only e878b362 b6d1f101 -- crates/qcore-frontend/src/` names six files
  and `git log --name-only` attributes every one — `frontend.rs` (the one
  reserved word) and `source_socket.rs` to `CT42-CELL-ROUTE` (`2d1f8fa9`);
  `source_record.rs`, `whole_program.rs`, `source_std_buffer.rs` to
  `AGGREGATE-OWNED-FIELD` (`e60ad0c1`, `2f27c652`, `c960a263`);
  `source_std_buffer.rs`, `source_std_string.rs` to `LOOPS-ON-COMPOSED-ROUTES`
  (`39cc9848`, `6adaf1ef`). `lexer.rs` and `parser.rs` did not move; no commit
  of the proof-leg or TSan card touched the front end. The wave line: **wave 39
  closed at one reserved word**, the first since `LOOP-V1`'s `for` at wave 26.
* **The Q-space note's tally.** At the base the note counted
  `AGGREGATE-OWNED-FIELD` sixteenth and `LOOPS-ON-COMPOSED-ROUTES` seventeenth
  and did **not** name `CT42-CELL-ROUTE`, whose own rows in the table below the
  note say "widening three conditions" (`Q080`, `Q081`, `Q085`) and "widens
  `Q095`'s condition", and whose report §4.5 and merge message say "five
  conditions widened, zero codes allocated" — an instance of the rule by the
  note's own criterion (it excludes wave 31's `CT42-PROTOCOL-CELL` precisely
  because that card widened nothing). Corrected: `CT42-CELL-ROUTE` is the
  **seventeenth**, `LOOPS-ON-COMPOSED-ROUTES` the **eighteenth** (its report and
  merge message say "seventeenth", named in the note as that checkpoint), the
  count sentence reads eighteen on ten bands, the icebox thirteen waves. The
  brief to this close carried the note's figure ("seventeen"), so the
  correction is a finding against the brief as well; it is exactly what
  `Q-SPACE-EXTENSION-TALLY-HAND-KEPT` records, now with a live instance where a
  card stated its widenings everywhere but the tally.
* **`named_followups`**: 234 in the matrix file, `dimensions.named_followups`
  derived to 234 by the regeneration; the count line's chain re-summed from the
  five merge messages (228 → 227 → 232 → 234 → 231 → 233) and this close's
  +3 −2.
* **Contract corpus** 88: `git diff --name-only e878b362 b6d1f101 --
  contract/qcore/` is empty. **Corpus row** unmoved: no arbiter ran.
  **Provider** 122 carried: the same diff over `providers/` and
  `crates/qcore-schemas/` is empty and `qcore-schemas` is still 14.0.0.
* **Merged gate**: replaced by this close's own measurement (§7).

## 7. Gate

Two detached runs, each `setsid -f docs/reports/wave39-close/scripts/gate.sh
<dir> < /dev/null`, no `trap '' TERM`, `git submodule update --init
--recursive` first inside the script (exit 0, `third_party/libevent` at
`d82464a2`), every exit to its own file with no pipe in the command
(`GATE-NO-FAIL-FAST`, `GATE-EXIT-SWALLOWED-BY-PIPE`), each polled by a bounded
loop on its `DONE` marker. Exits under `gate/base/` and `gate/final/`; the
Clang recording under `gate/`; disk never below 260 GB free.

GATE-TABLE-PLACEHOLDER

## 8. Deferred decisions re-listed

Fifteen carried from the wave-38 close, **one resolved by this wave's tree**,
three already ended kept for accounting, one added for accounting; grounds
with their measurements are in `reconciliation.json`. The substance:

| Decision | Verdict | Ground, in short |
|---|---|---|
| `CT10-REASON-ROW` | deferred | seventh wave; no card touched CT10 (`git diff` over its fixture directory empty); still double-carried |
| `MATRIX-UPDATED-STALE-AT-WAVE-27` | deferred | regenerated at least nine times this wave; `updated` still opens "wave 27 complete" |
| `EXAMPLES-STYLE-SLICE-FANOUT` | deferred | `examples/` moved for the first time since wave 33 (three new programs, four README rows); no existing example respelled, so the decision is untouched |
| `SYNC-RULE-TOOLING-COMMIT-SCOPE` | deferred | wave 37/38's shape a third time at five times the volume; one in-merge edit carried a transcription defect no report indexes (§4) |
| `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` | deferred | `OWNED-FIELD-WINDOWS-RUN` joins the `CLAUDE.md` list unregistered; `HTTP-V1-WINDOWS-RUN` still absent by id; more partial again |
| `DOCS-COMPACT-CADENCE-DOES-CORRECTION-WORK` | deferred | data point: no compaction fired and this close corrected a hand-kept tally and two stale cadence lines; `DOCS-COMPACT-8` due around wave 40 |
| `VERIFIER-HOST-ROW-DISPOSITION` | resolved (wave 35) | accounting; the proof-leg merge's three red host rows under load were green in isolation — constraint 16's shape |
| `items` | withdrawn (wave 35) | accounting |
| `INDIRECT-CENSUS-FEATURE-UNBUILT` | deferred | no `Cargo.toml` or CI file moved (measured); both arms stand |
| `TASK-ROUTE-TSAN-SEGV-PRIORITY` | resolved (wave 37) | accounting; `TSAN-ROWS-2` built on it |
| **`CT07-TWO-THREAD-REPLAY-ORDER`** | **resolved (this wave)** | retired by `TSAN-ROWS-2` by statement on its second arm; merge `0150c904` records "the card's retirement of `CT07-TWO-THREAD-REPLAY-ORDER` by statement (the deferred decision the wave-37 close left to the user) accepted as a user decision; the restated boundary is `TSAN-FORCED-BODY-OVERLAP-NOT-FORCED`" — absent by id, the restated boundary present; the double-carry ends |
| `SWEEP-RESIDUALS-PROSE-ONLY-ITEMS` | deferred | not taken; the one prose-named item this wave (LOOPS §12 item 8) was in backticks and the tool returned it |
| `Q-NAMESPACE-DECISION` | deferred (open; the wave-39 re-affirmation held) | three instances, zero codes, tally re-derived as eighteen; a re-affirmation for wave 40 is not this close's to make |
| `F02-WINDOWS-PIN-RUN` | deferred | no Windows or Apple run claimed by any wave-39 card; the literals unmoved; the agreement test still `#[ignore]`d and red by design |
| `MATRIX-RECORDED-BY-DECISION` | deferred | still exactly 67 entries without `recorded_by`; every wave-39 registration carries it |
| **`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`** | **resolved (added for accounting)** | never a deferred decision; retired at `0150c904` on its own second arm, which the merge records as "a DECISION, recorded as such, that this route's native evidence stays serial by construction" — the wording now cited in both CT46 texts |

## 9. Due, not run

* **`STABILITY-REEVAL` #4** is due after this wave. `CLAUDE.md`'s cadence line
  now says so (it read "#3 is due now, after wave 37"); #3's own §6 item 11
  names the Windows arm (`F02-WINDOWS-PIN-RUN`, the one live red row) and the
  tracker-only residuals as the highest-value ground. Candidate grounds this
  close adds, offered rather than briefed: the hand-kept tallies (the Q-space
  count went wrong within one merge of being written; constraint 14's
  enumeration is re-measured by each close because nothing derives it); the
  three integration gates' borrowed Clang claims; and the
  `WRITE-LEND-TASKS-EXERCISE`-shaped question for the forced-schedule rows —
  what a later card that gives a worker body a synchronization edge does to
  `TSAN-FORCED-HARNESS-SYNC-INVISIBILITY`.
* **`DOCS-COMPACT-8`** around wave 40 (`CLAUDE.md` now reads so; it read "next
  due around wave 39").

## 10. Boundaries

* Nothing is promoted: no corpus count, unit status or function status moves;
  raw units stay unpromoted; CT 47/50 with sanitized-native 10 and proof 4;
  contract corpus 88; full language conformance **false**.
* No Windows or Apple run of anything is claimed; the Windows F02 row is still
  red on every Windows host.
* The 4,641 / 3 / 9 triple in §4 is the brief's statement corroborated by a
  sum, not a re-derivation from the integration run's log.
* The 57 `noted` verdicts on re-swept frozen reports carry the prior closes'
  measurements, not this close's.
* The gate figures are full-suite runs on a machine carrying its own build,
  not the quiet-host number `HOST-ROW-STARVATION-DESCRIPTION` asks for.

## 11. Reproduction

```
git submodule update --init --recursive
setsid -f docs/reports/wave39-close/scripts/gate.sh <dir> < /dev/null        # detached, exits to files (edit W= to your worktree)
python3 docs/reports/wave39-close/scripts/cite_ct46.py                       # idempotent
python3 docs/reports/wave39-close/scripts/register.py --show                 # then without --show
python3 -B docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave39-close/gate.json
python3 scripts/ct_stage_summary.py --check
python3 docs/reports/wave39-close/scripts/build_reconciliation.py
python3 scripts/reconciliation_sweep.py . --since e878b362 --check docs/reports/wave39-close/reconciliation.json
```
