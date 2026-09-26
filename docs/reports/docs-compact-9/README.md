# `DOCS-COMPACT-9` — the ninth documentation compaction (wave 45 close)

**Base:** `8c1f15eb`. **Scope:** the router files (`AGENTS.md`, `CLAUDE.md`,
`README.md`), the rule topic pages where narrative accreted after the
2026-09-19 restructuring, `docs/STATUS.md`, and the four increment records over
32 KiB. Documentation only: no crate source, proof, matrix entry or frozen
report changed. The ninth firing of the
[maintenance cadence](../../agents/operations.md#maintenance-cadence), due
around waves 42–43 and postponed until now.

**Compaction is not deletion.** Every byte that left a current page is in one
of three checkpoint pages, re-read from `git show 8c1f15eb:<file>` by
[`checkpoint.py`](scripts/checkpoint.py) and asserted to be a contiguous
substring of the base file:
[rule pages](../../development/increments/wave-45-compaction-checkpoints.md)
(32,564 bytes), [status table](../../development/increments/wave-45-compaction-status.md)
(29,054) and [status prose](../../development/increments/wave-45-compaction-status-prose.md)
(10,747). Each says it is history and not a second authority.

**Headline.** 28,230 bytes left four current pages: `docs/STATUS.md`
43,785 → 23,605, `gateway.md` 61,441 → 57,211, `diagnostics.md` 34,348 → 30,770
(back under the 32 KiB cap the page sets itself), `buffers-records.md`
32,132 → 31,891. All 69 rule blocks keep their numbers, pages and anchors; 65
are byte-identical. The six hard invariants are byte-identical. **Nine
corrections**, each measured, two of them the brief's named stale items.

## 1. Inventory before and after

[`inventory.py`](scripts/inventory.py) splits every rule page at its
`<a id="constraint-…">` anchors and hashes each block, hashes each hard
invariant, lists every anchor of every tracked Markdown file, and resolves every
local link outside code fences; [`compare.py`](scripts/compare.py) diffs two
runs ([result](artifacts/inventory-compare.txt)).

| Check | Before (`8c1f15eb`) | After |
|---|---|---|
| Rule blocks (1–29 with letters, plus the 14 and 15k amendment anchors) | 69 | 69, same ids, labels and pages |
| Byte-identical rule blocks | — | **65**; changed: 5c, 7, 8 (`gateway.md`) and the 15k amendments (`buffers-records.md`) |
| Hard invariants | 6 | 6, byte-identical |
| Anchors lost, any tracked Markdown file | — | **none** |
| Local links with a fragment into a changed page | — | 46, all resolve |
| Unresolved local links, whole tree | 30 | 30, the same set (none into a changed page) |
| Names (`UPPER-KEBAB`) no longer on a page | — | 103 on `STATUS.md`, all in the status checkpoints; none on a rule page |
| Report pointers no longer on a page | — | 6 on `STATUS.md`, all in the status checkpoints; none from any rule block |
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
| `docs/STATUS.md` | provenance paragraph; nine coverage rows' parenthetical chains | −14,606 | | |
| | condition-call row (dated, current figure added); unit-reason row (re-sourced); raw-units paragraph | | +51 | |
| | uncontracted-call row; latest gate and provider paragraph; follow-up chronicle | | | −5,625 |
| `docs/agents/gateway.md` | header pointer; constraint 8's two first-exercise paragraphs | −355 | | |
| | constraints 5c and 7 | | | −3,875 |
| `docs/agents/diagnostics.md` | five `§L21` table rows; the namespaces block's markers | −3,902 | | |
| | the amendments preamble | | +324 | |
| `docs/agents/buffers-records.md` | the 15k amendment's opening history | −241 | | |
| `docs/agents/README.md` | rule 7's index sentence | | −1 | |
| **Total −28,230** | | **−19,104** | **+374** | **−9,500** |

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
page is under 32 KiB. The other topic pages are under 16 KiB.

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
   5,995 / 0 / 11 over 333 summaries, provider 129 / 0 / 1 over 43, 207 Lean,
   364 Python, 15 package Python.
7. **STATUS, provider paragraph:** "39 shared modules … indexes all 111". The
   eleven roots of `contract/corpus.json` hold 72 + 42 = **114** tracked modules
   ([`rederive2.py`](scripts/rederive2.py)), which `qcore_contract_corpus.rs` pins.
8. **STATUS, follow-up paragraph:** "391 follow-ups are registered"; the matrix
   carries **472**, carried and derived agreeing. The three open Cycle 18
   follow-ups it mentioned are now named, each read from `named_followups`.
9. **`diagnostics.md`'s amendments preamble** said every preserved block is
   "pinned by `verify.py` and never edited in place". `verify.py` exits 1 at the
   base ([before](artifacts/structure-verify-before.txt)): eight migration rows
   were already edited in place on other pages and `operations` has no markers
   ([blocks](artifacts/preserved-blocks-before.txt)). The preamble now says
   which blocks are still preserved and where the namespaces block's migration
   text is.

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
| Follow-ups | 472, carried = derived | `docs/reviews/r0_3/acceptance_matrix.json` |
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
  "17,281" in the indirect-access chain that moved. Frozen records are not
  rewritten (integration policy).
* **Residual discovery** (`reconciliation_sweep.untracked_residuals` over a
  documents-only base root and the tree, [result](artifacts/residual-discovery.txt)):
  97 → 98 candidates, none lost. The new one, `W43-LIBC-POINTER-QUALIFIER-READER`,
  is a name W43-LIBC-RESULT-ALIAS retired; it surfaces because the status
  checkpoint is an increment page the sweep reads and `STATUS.md` was not.
* **Preserved migration blocks** ([after](artifacts/preserved-blocks-after.txt)):
  constraint 5c, 7, 8 and the namespaces block lose their markers, because
  their text is no longer the migration's; every removed line is in the rule-page
  checkpoint with its markers. The migration's `verify.py` exits 1 before and
  after, at the same first row ([after](artifacts/structure-verify-after.txt)).

## 7. Gate

Filled in from the run below; exits in [`gate/`](gate/).

## 8. For the integrator

1. **The cadence bullet.** `operations.md`'s "Due and not yet recorded as run"
   bullet is `STABILITY-REEVAL-5`'s to update for its own firing; this card
   records its firing there only after merging that version (§9).
2. **Editing preserved blocks is a decision this card took.** `diagnostics.md`
   and `buffers-records.md` stated that preserved blocks are never edited in
   place; the pin it rested on no longer holds (correction 9). This card
   compacted the namespaces block and gateway 5c, 7 and 8 with their migration
   text moved verbatim, markers included, and left every other preserved block
   untouched. Reverse it by restoring those blocks from the checkpoint if the
   convention should stand.
3. **`gateway.md` is still 57,211 bytes**, the one policy page far over
   32 KiB. What remains is operative rule text. Splitting it would move
   `gateway.md#constraint-N` anchors that reports link to, so it is a structural
   decision, not a compaction.
4. **The Q-space tally was carried, not re-derived.** The amendments put it at
   twenty-four plus one (RESOURCE-HELPER-FUNCTIONS); the compacted `§L21` rows
   keep every card, so it stays derivable.
5. **Three frozen claims flipped** (§6), by accident, not by a moved claim.

## 9. Re-derivation after `main` moved

Pending: this card merges last; this section records what the merge re-derived.
