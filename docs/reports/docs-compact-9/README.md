# `DOCS-COMPACT-9` — the ninth documentation compaction (wave 45 close)

**Base:** `8c1f15eb`, merged with `main` `200fdcbe` after `STABILITY-REEVAL-5`
and re-derived there (§9). **Scope:** the router files (`AGENTS.md`, `CLAUDE.md`,
`README.md`), the rule topic pages where narrative accreted after the
2026-09-19 restructuring, `docs/STATUS.md`, and the four increment records over
32 KiB, and at the merge five prose defects `STABILITY-REEVAL-5` reported in
this card's region. Documentation and ledger only: no crate source, proof or
frozen report changed. The ninth firing of the
[maintenance cadence](../../agents/operations.md#maintenance-cadence), due
around waves 42–43 and postponed until now.

**Compaction is not deletion.** Every byte that left a current page is in one
of three checkpoint pages, re-read by [`checkpoint.py`](scripts/checkpoint.py)
from `git show <revision>:<file>` (`8c1f15eb`, and `200fdcbe` for the one row
`STABILITY-REEVAL-5` had rewritten) and asserted to be a contiguous substring of
that file: [rule pages](../../development/increments/wave-45-compaction-checkpoints.md)
(32,193 bytes), [status part 1](../../development/increments/wave-45-compaction-status.md)
(22,431) and [status part 2](../../development/increments/wave-45-compaction-status-prose.md)
(22,254). Each says it is history and not a second authority.

**Headline**, against the merged `main` `200fdcbe`: `docs/STATUS.md`
44,158 → 23,931, `gateway.md` 61,441 → 57,557, `diagnostics.md` 34,348 → 31,060
(back under the 32 KiB cap the page sets itself), `buffers-records.md`
32,132 → 32,193 (compacted, then grown by the corrected preamble); 27,338 bytes
net off those four. All 69 rule blocks keep their numbers, pages and anchors;
64 are byte-identical. The six hard invariants are byte-identical. **Fourteen
corrections**, each measured: nine from the draft, two of them the brief's
named stale items, and five the review's findings in this card's region.

## 1. Inventory before and after

[`inventory.py`](scripts/inventory.py) splits every rule page at its
`<a id="constraint-…">` anchors and hashes each block, hashes each hard
invariant, lists every anchor of every tracked Markdown file, and resolves every
local link outside code fences; [`compare.py`](scripts/compare.py) diffs two
runs: against the base ([result](artifacts/inventory-compare.txt)) and against the
merged `main` ([result](artifacts/inventory-compare-vs-main.txt)), which agree on
every row below.

| Check | Before (`8c1f15eb`) | After |
|---|---|---|
| Rule blocks (1–29 with letters, plus the 14 and 15k amendment anchors) | 69 | 69, same ids, labels and pages |
| Byte-identical rule blocks | — | **64**; changed: 5c, 5l, 7, 8 (`gateway.md`) and the 15k amendments (`buffers-records.md`) |
| Hard invariants | 6 | 6, byte-identical |
| Anchors lost, any tracked Markdown file | — | **none** |
| Local links with a fragment into a changed page | — | 55, all resolve |
| Unresolved local links, whole tree | 30 | 30, the same set (none into a changed page) |
| Names (`UPPER-KEBAB`) no longer on a page | — | 106 on `STATUS.md` (against `200fdcbe`), all in the status checkpoints; none on a rule page |
| Report pointers no longer on a page | — | 7 on `STATUS.md`, all in the status checkpoints; none from any rule block |
| `§L21` table rows and `Q` codes on `diagnostics.md` | 31 rows | 31 rows, same first column; no code or name lost |

Each changed rule keeps every report pointer and every name it had: the three
residuals of constraint 7's chain (`AOM-DESTROY-JOINED-PROVENANCE`,
`AOM-SIBLINGS-BINDING-POISON`, `CONSTRUCTOR-RESIDUALS-LEAK-271`, all still in
`named_followups`) are named in the compacted rule.

## 2. What was compacted, and bytes

Per block by [`deltas.py`](scripts/deltas.py) ([table](artifacts/byte-deltas.txt));
"both" blocks were compacted and corrected in one rewrite.

| Page | Blocks | Compaction | Correction | Both |
|---|---|---:|---:|---:|
| `docs/STATUS.md` | provenance paragraph; nine coverage rows' parenthetical chains (row 48 measured against `STABILITY-REEVAL-5`'s version) | −14,777 | | |
| | condition-call row; unit-reason row; native descriptor row; raw-units paragraph | | +132 | |
| | uncontracted-call row; latest gate and provider paragraph; follow-up chronicle | | | −5,582 |
| `docs/agents/gateway.md` | header pointer; constraint 8's two first-exercise paragraphs | −324 | | |
| | constraint 5l's element-custody sentence (against 10f) | | +315 | |
| | constraints 5c and 7 | | | −3,875 |
| `docs/agents/diagnostics.md` | five `§L21` table rows; the namespaces block's markers | −3,902 | | |
| | the amendments preamble | | +614 | |
| `docs/agents/buffers-records.md` | the 15k amendment's opening history | −241 | | |
| | the amendments preamble, made the same statement as `diagnostics.md`'s | | +302 | |
| `docs/agents/operations.md` | the integration-gate path; the cadence's page list and firing record | | +229 | |
| `docs/development/audit-checkpoints.md` | the two-phase resume sentence | | +278 | |
| `docs/agents/README.md` | rule 7's index sentence | | −1 | |
| **Total −26,832** | | **−19,244** | **+1,869** | **−9,457** |

What each compacted block keeps is stated beside its verbatim copy in the
checkpoint. In short: every STATUS row keeps its current figure, the card that
last moved it and its source, and the projected-functions row keeps every
card's premise boundary (index traversal's "no bounds claim" included); the
chains of earlier figures moved. Constraint 5c keeps its key, the three
survivals, the value route's exit-edge rule, the Lean names and the acyclicity
boundary, and sheds the line-number history and the wave-35 close's integrator
clauses. Constraint 7 keeps the attribution duty and the two directions a move
can take, now stated as rules rather than as a chain. The five `§L21` rows keep
every extending card, every widened, narrowed or unwidened code and every
constraint cross-reference; `§L21` carries the full condition text.

**Not compacted, and why.** `AGENTS.md` (5,547), `CLAUDE.md` (511) and
`README.md` (2,491) are router-sized already, every link resolves and no figure
in them is stale, so they are unchanged. Gateway rules 4, 4a, 5–5b, 5d–5l, 6 and
9–10h are operative rules, not chronicles; 5d–5l and 10c–10h are wave-44/45
authorities. `buffers-records.md`'s preserved 15h and 15k blocks are dense
must-survive lists (`DOCS-COMPACT-8` §3.5 reached the same view of 15h) and the
page is under 32 KiB. The other topic pages are under 16 KiB. **`gateway.md`
stays one page at 57,557 bytes** (integrator decision): what remains is
operative rule text, and a split would move the `gateway.md#constraint-N`
anchors that reports link to, so it would need an anchor-redirect plan first.

## 3. Corrections, each with its measured source

1. **Gateway constraint 7: 272 → 34.** `diagnostic_counts.leak` of the baseline
   `docs/reports/w45-container-root-reach/integration/artifacts/diagnostics.json.gz`
   is 34, and so is W45-FB-BACKLOG-CUSTODY's archive; 272 last held at the wave-43
   libc and return-expression cards (their `attribution-summary.json`). The
   archived arbiters give the chain since ([statuses](artifacts/archived-arbiter-statuses.txt)):
   271 (W44 bases) → 270 → 21 (W44-LEAK-PIN-AUDIT) → 19 → 34 → 35 → 37 → 34.
   The rule index's opening sentence for rule 7 is corrected with it.
2. **Gateway constraint 5c: 32 → 28 do-while-condition leaves.**
   `blocker_census.py` of W45-SUB-OBJECT-STORES on the baseline
   ([census](artifacts/blocker-census-baseline.txt)); the 32 is kept as the
   card's re-pin figure.
3. **STATUS, uncontracted-call row: 9,142 / 2,616 / 428 → 9,157 / 2,619 / 436.**
   The row had not moved since W45-SUB-OBJECT-STORES. The census's own script,
   `w44-corpus-callee-summaries/scripts/callee_census.py`, reproduces
   9,142 / 2,616 / 428 on the W45-SUB-OBJECT-STORES and W45-PARENTHESISED-CALLEE
   archives and gives 9,157 / 2,619 / 436 from W45-FB-BACKLOG-CUSTODY on, the
   baseline included ([baseline](artifacts/callee-census-baseline.txt), and one
   file per archive). The by-callee split is now the baseline run's (the old
   split was W44-CORPUS-CALLEE-SUMMARIES', with 586 `__bswap_*` leaves now 0).
4. **STATUS, condition-call row** stated W44-BRANCH-CONDITION-CALLS' own figures
   as if current. They are dated to that card, and the baseline's branch-condition
   refusal is added: 7,678 leaves on 2,161 Unknown functions, 70 sole (census as in 2).
5. **STATUS, raw-units paragraph:** "the unit Safe count rose to 3" (it is 5) and
   "`flb_input_chunk_create` is function-level Safe" (it is `Unknown` in the
   baseline, since W45-FB-RETRY-LIFETIME, as the functions row itself said).
6. **STATUS, latest gate:** it named the task-context bundle gate (5,043 Rust,
   165 Lean, 336 Python). The latest merged gate is
   [W45-AUDIT-SUBCOMMAND-PLUGINS' integration gate](../w45-audit-subcommand-plugins/integration/gate.json):
   5,995 / 0 / 11 over 331 summaries (the record said 333 until `33558c67`
   corrected it on `main`), provider 129 / 0 / 1 over 43, 207 Lean, 364 Python,
   15 package Python.
7. **STATUS, provider paragraph:** "39 shared modules … indexes all 111". The
   eleven roots of `contract/corpus.json` hold 72 + 42 = **114** tracked modules
   ([`rederive2.py`](scripts/rederive2.py)), which `qcore_contract_corpus.rs` pins.
8. **STATUS, follow-up paragraph:** "391 follow-ups are registered"; the matrix
   carried **472**, carried and derived agreeing (513 after `STABILITY-REEVAL-5`'s
   forty and this card's ledger edits, §8). The three open Cycle 18
   follow-ups it mentioned are now named, each read from `named_followups`.
9. **The amendments preambles of `diagnostics.md` and `buffers-records.md`**
   said the preserved blocks are "pinned by `verify.py`" and that `verify.py`
   "reconstructs the pre-restructure guides from them byte for byte". It exits 1
   at the base, on its first row, `AGENTS.md` → `docs/development/architecture.md`
   ([before](artifacts/structure-verify-before.txt)): eight migration rows were
   already edited in place and `operations` has no markers
   ([blocks](artifacts/preserved-blocks-before.txt)). Both preambles now make one
   statement: the markers pin nothing mechanically
   (`W45-DOCS-PRESERVATION-VERIFIER-DECAYED`), and a preserved block is edited
   only to correct or compact the current rule, with its migration text moved
   verbatim, markers included, into a checkpoint.

Five more at the merge, the prose defects `STABILITY-REEVAL-5` registered in
this card's region:

10. **STATUS, native descriptor row** (`NATIVE-OBSERVATION-UPSTREAM-WORDING`):
    "two source-confirmed upstream double closes" now reads "two double closes
    read against the Fluent Bit source: local observations, provisional
    candidates, not upstream defect claims".
11. **STATUS, latest gate** (`STATUS-LATEST-GATE-STALE`): correction 6, confirmed
    at the merge against the gate file: 5,995 / 0 / 11 over 331 summaries,
    207 Lean audits, 364 Python tests.
12. **Gateway 5l** (`GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK`): "whose
    custody then refuses on its own: an element of a borrowed container is
    untracked" was stale since rule 10f, whose foreign unlink binds the element
    at a fresh site (`take_foreign_custody`). The sentence now says the unlink is
    10f's, and the premise that an element is not tracked holds only until a 10f
    unlink binds it, a case its lexical guard does not see. This corrects the
    text to the code; no rule changed.
13. **`operations.md`'s integration rule** (`INTEGRATION-GATE-PATH-RULE-DRIFT`)
    named `docs/reports/<card>/integration-gate.json`; every integration gate
    since wave 43 is `integration/gate.json`, and the rule now says so.
14. **`docs/development/audit-checkpoints.md`**
    (`CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES`) said "a resumed second phase
    reuses only groups sealed under the same table". Both phases seal to
    `<dir>/<label>.qo` (`checkpoint_run.rs`, the path built per group;
    `current_route.rs` calls `checkpoint_run::run` twice with the same options),
    so a resumed run under a non-empty table reuses no group; the page now says
    that, and that only a run with an empty table reuses.

Two sentences were re-sourced rather than corrected: the unit-reason row
(167 / 1 / 17 reproduce from the baseline's `reason_code`) and the indirect-access
and return-expression rows, whose figures reproduce exactly from the baseline
blocker census.

## 4. Headline figures, re-derived

From primary sources only ([`rederive.py`](scripts/rederive.py),
[output](artifacts/figures-rederived.json)); each matched the page after the
corrections above.

| Figure | Value | Source |
|---|---|---|
| Units | 5 Safe / 8 provisional Unsafe / 168 Unknown / 4 data-only (185) | baseline `diagnostics.json.gz` |
| Functions | 398 / 9 / 4,427 (4,834); 4,921 definitions, 87 not projected | same |
| Diagnostics | leak 34, unknown_alias 3,571, unsupported 41,314, summary_conflict 1 | same, `diagnostic_counts` |
| CT | 50/50 mapped; proof 5 (CT01, CT03, CT07, CT37, CT50); sanitized-native 10 | `docs/reevaluation/ct_manifest.json` |
| Follow-ups | 472 at the base; 512 at `main` `200fdcbe`; **513** after §8 (carried = derived) | `docs/reviews/r0_3/acceptance_matrix.json` |
| Contract corpus | 114 modules; 50 manifest-selected; 174 elaborated v1 declarations | tracked provider sources; `cycle19-resource-contracts.json`; `contract_loader.rs` pin |
| Lean audits | 207; toolchain `leanprover/lean4:v4.34.0` | `proof/Audit*.lean`; `proof/lean-toolchain` |

The baseline is verdict-bearing identical at the two later integrations
(W45-REFUTED-CONTRACT-WITHDRAWAL, W45-AUDIT-SUBCOMMAND-PLUGINS), by their own
identity files.

## 5. Increment records split

Each file's oldest tail moved verbatim, at a paragraph boundary, into a
`-history.md` page ([`split.py`](scripts/split.py) asserts head + tail equals
the base file); the parent gains one sentence naming the moved cards, and the
[index](../../development/increments/README.md) lists every page.

| Record | Before | After | New page |
|---|---:|---:|---|
| `wave-45.md` | 41,339 | 30,089 | `wave-45-history.md`, 12,181 (7 cards from W45-PARENTHESISED-CALLEE back) |
| `wave-44.md` | 33,095 | 29,571 | `wave-44-history.md`, 4,307 (W44-CORPUS-CALLEE-SUMMARIES, W44-INDIRECT-CHAIN-ACCESS) |
| `wave-41.md` | 33,809 | 29,529 | `wave-41-history.md`, 5,021 (CT-PROOF-LEG-3) |
| `wave-41-continuation.md` | 36,333 | 28,855 | `wave-41-continuation-history.md`, 8,470 (six Cycle 18 and review integrations) |

The split points keep every reconciliation claim on these records green: the
three `wave-41.md` claims sit in its head, and the two
`wave-41-continuation.md` claims name cards the parent's split sentence names.

## 6. Checks beyond the gate

* **Reconciliation claims** on every touched file ([before](artifacts/reconciliation-claims-before.txt),
  [after](artifacts/reconciliation-claims-after.txt)): 157 claims, 125 → 128
  failing. The three flips are `canonical-contract-locations` and
  `cycle18-priority1-integration` (twice) claiming STATUS "contains `281`",
  their follow-up count of wave 41. It had matched only by accident, inside
  "17,281" in the indirect-access chain that moved, so they were vacuous
  before this card. Frozen records are not rewritten (integration policy);
  registered as `W45-RECONCILIATION-SUBSTRING-CLAIMS` (§8). No gate or CI step
  runs them: `.github/workflows/` runs no reconciliation check, the standing
  gate does not, and `scripts/tests` exercises the sweep only on temporary
  fixtures. They run only when a close checks its own record by hand.
* **Residual discovery** (`reconciliation_sweep.untracked_residuals` over a
  documents-only base root and the tree, [result](artifacts/residual-discovery.txt)):
  97 → 97 candidates. `DOCS-COMPACT-9` left the list, now tracked as the
  `recorded_by` of §8's entries; the one new candidate, `W43-LIBC-POINTER-QUALIFIER-READER`,
  is a name W43-LIBC-RESULT-ALIAS retired; it surfaces because the status
  checkpoint is an increment page the sweep reads and `STATUS.md` was not.
* **Preserved migration blocks** ([after](artifacts/preserved-blocks-after.txt)):
  constraint 5c, 7, 8 and the namespaces block lose their markers, because
  their text is no longer the migration's; every removed line is in the rule-page
  checkpoint with its markers. The other 59 rows stay byte-identical to the
  migration. The migration's `verify.py` exits 1 before and after, at the same
  first row ([after](artifacts/structure-verify-after.txt)).
* **After the integrator's decisions** (§8): `python3 -m unittest discover -s
  scripts/tests` and `python3 scripts/build_docs_index.py --check` re-run on the
  final tree, exits in [`gate/`](gate/) as `python-post.exit` and
  `docs-index-post.exit`.

## 7. Gate

One detached run on the draft tree (`026caf65`), `CARGO_INCREMENTAL=0`, each
step's exit written to its own file and nothing piped; exits, the totals line
and the Clang recording are in [`gate/`](gate/), logs private under
`private/dc9-gate/` and `private/test-binaries/20260926T163033Z-1790014/`.

| Step | Exit | Result |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a2` |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 319 units, 331 summaries; **5,995 passed, 0 failed, 11 ignored** |
| `cargo clean --profile dev` | 0 | after the test step, as briefed |
| `python3 scripts/check_lean_audits.py` | 0 | Passed 207 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 364 tests, OK |
| `python3 scripts/build_docs_index.py --check` | 0 | index matches all report directories |
| `python3 scripts/task_context.py --base 8c1f15eb …` | 0 | packet generated; the report's scripts are unknown paths, so every topic is selected |
| `clang --version` | 0 | Ubuntu clang 21.1.8, run in this worktree |

The pass, fail and ignored counts equal the W45-AUDIT-SUBCOMMAND-PLUGINS
integration gate's on the same code, and so do the 331 summaries, which is the
count `33558c67` corrected that record to. Constraint 16's rows in this workspace all
passed on the first run (`a_declared_process_limit_stops_the_fixture_from_forking`,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths`,
`memory_growth_beyond_the_declared_limit_is_bounded`, and beside them
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
`a_forced_override_never_weakens_an_execution`); the provider workspace was not
run, since no provider file changed. The Python suite's task-context tests read
`STATUS.md` and the rule pages through the packet builder, and passed; no test
pins a heading or anchor this card moved, so no test was adjusted.

## 8. Integrator decisions, and names registered

Decided by the integrator on the draft, and applied:

1. **Preserved blocks: the edits stand.** `verify.py` had already decayed and
   protects nothing, and a false figure in a current rule is worse than an
   edited migration block. The migration text of every edited block stays
   verbatim, markers included, in the rule-page checkpoint, and both pages that
   stated the convention now say the same thing (correction 9).
2. **`gateway.md` stays one page** (§2): anchors take precedence.
3. **The ledger, re-applied on `main`'s matrix at the merge**:
   `named_followups` 512 → **513**, by [`update_ledger.py`](scripts/update_ledger.py)
   (two entries from [`followups.json`](followups.json) registered, one retired
   into [`retired-followups.json`](retired-followups.json), four amended), then
   the matrix regenerated with `build_matrix.py . --gate
   docs/reports/stability-reeval-5/integration/gate.json`, the latest integration
   gate at equal code, so the validation block stays an integrator measurement.
   This card's own [`gate.json`](gate.json) records its pre-merge run.

| id | what it records | discharge |
|---|---|---|
| `W45-DOCS-PRESERVATION-VERIFIER-DECAYED` | `verify.py` exits 1 at `8c1f15eb` on its architecture row; eight rows were edited before this card and four by it; the script is a frozen report's (constraint 21) | a live check verifying each still-preserved block against its checkpoint copy, or a recorded retirement of the pin listing the blocks it covered |
| `W45-RECONCILIATION-SUBSTRING-CLAIMS` | the three frozen "STATUS contains `281`" claims of §6, vacuous before this card | the sweep's containment check anchored to a token or field, or the claims marked historical |

**Retired:** `INTEGRATION-GATE-PATH-RULE-DRIFT`. Its discharge asked the rule
text and any close tooling to name the path in use; the rule now does
(correction 13), and no tooling names either path (`git grep integration-gate`
over `scripts/`, `providers/`, `*.rs` and `*.sh` finds nothing).

**Amended, not retired** (an appended `AMENDED (DOCS-COMPACT-9, wave-45 close)`
sentence saying what was done and what stays open):

| id | done here | still open |
|---|---|---|
| `NATIVE-OBSERVATION-UPSTREAM-WORDING` | the STATUS row | the frozen W43 report's sentence and the wave-45 record's `mk_http_error` wording |
| `STATUS-LATEST-GATE-STALE` | the section names the latest integration gate | a check tying STATUS's figures to a gate file |
| `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` | 5l's text | the pinned fixture |
| `CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES` | the document | the code defect, kept open at the integrator's direction |

Still open:

* **The Q-space tally was carried, not re-derived.** The amendments put it at
  twenty-four plus one (RESOURCE-HELPER-FUNCTIONS); the compacted `§L21` rows
  keep every card, so it stays derivable.

## 9. Re-derivation after `main` moved

`main` moved from `8c1f15eb` to `200fdcbe`: `33558c67` (the W45-AUDIT-SUBCOMMAND-PLUGINS
gate record corrected to 331 summaries) and `STABILITY-REEVAL-5` (its report,
40 registered findings, a STATUS follow-up row, a wave-45 paragraph and the
cadence bullet). `git merge main` ran; three files conflicted, all resolved by
re-deriving rather than by union:

| Region | What `main` had | Re-derived as |
|---|---|---|
| `acceptance_matrix.json` | 512 follow-ups, the SR5 gate's validation block | `main`'s file, then this card's ledger script and a regeneration with the SR5 integration gate: 513, carried = derived |
| STATUS follow-up row | 512 with the SR5 chain | 513 and the last moving card; SR5's row moved verbatim to status part 2, as its own block at `200fdcbe` |
| STATUS latest gate | (unchanged by SR5; its finding pointed here) | 331 summaries, from the gate file as `33558c67` left it |
| `wave-45-continuation.md` | SR5's paragraph | `main`'s file with this card's paragraph above it (newest first), asserted to differ from `main`'s only by that paragraph |
| `operations.md` cadence bullet | SR5's firing-five line | kept, with this compaction's firing and the next due (wave 47–48); the cadence's first bullet names the pages it now covers |
| `INDEX.md` | SR5's report entry | regenerated |

Nothing SR5 changed touched a rule block, an anchor or an invariant: the
inventory against `200fdcbe` ([result](artifacts/inventory-compare-vs-main.txt))
gives the same 69 blocks, the same labels and pages, 64 byte-identical (the
draft's 65 less 5l, corrected at the merge), six identical invariants, no anchor
lost and the same 30 unresolved links. Every figure in §4 was re-derived again
on the merged tree and none moved but the follow-up count.

**Checks after the merge**, all exit 0 (`gate/python-merge.exit`, 364 tests;
`gate/docs-index-merge.exit`; `gate/task-context-merge.exit`, `--base 200fdcbe`). Only documentation and the matrix changed
since the full gate of §7; the integrator runs the integration gate.
