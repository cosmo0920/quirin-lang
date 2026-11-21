# `STATUS-COMPACT` — `docs/STATUS.md` rewritten as a current-state page

**Base:** `origin/main` `2aefebbc1` (integrate/r48 landing record); `origin/main`
had not moved when this card finished, so `2aefebbc1` is also the main revision
folded in (§6). **Scope:** `docs/STATUS.md`, its verbatim checkpoint, the
increment index row that lists it, one note in the
[maintenance cadence](../../agents/operations.md#maintenance-cadence), this report
and its amendments. Documentation only: no crate, proof, contract, matrix or
frozen report changed. The user asked to tidy the statuses STATUS describes;
operations requires the page to be "current, not a history".

**Headline.** `docs/STATUS.md` **290,886 → 23,182 bytes** (570 → 262 lines),
under the 32 KiB target. The whole page it replaced is preserved verbatim in
eleven checkpoint pages,
[`wave-48-compaction-status-part1.md`](../../development/increments/wave-48-compaction-status-part1.md)
to `part11.md`, listed in the [increment index](../../development/increments/README.md)'s
wave-48 row. [`checkpoint.py check 2aefebbc1`](scripts/checkpoint.py) asserts
that their fenced bodies, concatenated, equal `git show 2aefebbc1:docs/STATUS.md`
byte for byte ([output](artifacts/checkpoint-check.txt)). Nothing was deleted
without a verbatim copy, and every figure on the new page was re-derived from a
primary source (§3), not copied from an old paragraph.

This is not the tenth documentation compaction: `AGENTS.md`, `CLAUDE.md`,
`README.md`, the rule topic pages and the increment records over 32 KiB were
not touched. That compaction remains due (wave 47–48), and the cadence bullet
now says STATUS was compacted separately.

## 1. What the old page held, and where it went

| Old region (`2aefebbc1`) | Bytes | Became |
|---|---:|---|
| Lines 3–203: 68 per-card status blocks, newest first (prepended by `apply_amendments.py` on each landing) | 45,053 | Checkpoint parts 1–2. Each card's evidence is also in its report and increment paragraph. Facts still current were re-derived into the sections below. |
| Line 51: the "Unpublished landings" sentence | 2,235 | Kept as one delimited sentence (§4); the 37-landing list moved to checkpoint part 1. |
| Lines 204–462, `## Coverage`: preamble paragraphs, an 84-row table with per-card chains (two rows 45 KB each) and two boundary paragraphs | 236,913 | Checkpoint parts 2–11. Current figure and boundary of each still-relevant row → *Coverage by area*; the language/gateway capability rows → a pointer to the implemented grammars and gateway rules; the boundary paragraphs ("Mapping is not conformance", "Raw units remain unpromoted") → *Evidence boundaries*, verbatim where still true. |
| `## Milestones and trust` | 2,248 | Cycle status and the trust-boundary-28 statement kept; the two retired-example paragraphs (pinned replay parity, combined gate of 6,059 Rust tests) moved. |
| `## Latest validated gate` (the event-collector gate, 8,927 / 396 / 568) plus the provider, Cycle 18 and host-race paragraphs | 6,286 | Replaced by the r48 landing gate (§3). The provider-ownership figures re-derived; Cycle 18 follow-ups and the rule 16/26 pointers kept; the host-race repair chain moved. |
| `## Where to go next` | 340 | Kept, plus a link to the new checkpoint. |

The new page's sections: **Evidence boundaries** (the invariants every figure
obeys), **Headline measurements** (published baselines; the unpublished-landings
sentence; what was measured since; conformance, proof and corpus), **Coverage by
area**, **Milestones and open trust boundaries**, **Latest validated gate**,
**Where to go next**.

## 2. Inventory before and after

[`inventory.py 2aefebbc1`](scripts/inventory.py) ([output](artifacts/inventory.txt))
exits 0:

| Check | Before | After |
|---|---|---|
| Checkpoint reproduces the old page | — | yes, 11 parts, 290,886 bytes |
| `UPPER-KEBAB` names | 636 | 21 kept on the page; the other 615 are in the checkpoint; **0 lost** |
| Report pointers (`reports/<dir>`) | 223 | 30 kept; 193 checkpoint only; **0 lost**; 9 new on the page |
| Backticked revisions | 127 | 8 kept; 119 checkpoint only; **0 lost** |
| Bold figures | 386 | 14 kept; 372 checkpoint only (dated chains); **0 lost** |
| Invariant statements (the list in `inventory.py`, 55 entries: the verdict order and Pending definition, provisional Unsafe, unknowns in the denominator, projection ≠ safety, raw units unpromoted, lifecycle ≠ function safety, separate properties, link-map assumption, plugin Pending-only, independence of evidence classes, trust boundary 28, the CT mapping paragraph, frozen archives, native-observation qualifiers, the Cycle 18 follow-ups, Cycle 20's refusal, the halted adoption, the failed OOM run, the contributions link, the unpublished-landings shape) | — | all 55 present verbatim |
| Re-derived headline figures (§3) | — | all on the page |
| Headings | 5 | 11; `## Coverage` and `## Milestones and trust` renamed |
| Inbound `STATUS.md#…` links, whole tree | 0 | 0, so no anchor dependency |
| Local links on the page | — | 69, all resolve |

**Corrections, each measured.** The old page had drifted from its own sources:

1. *Latest validated gate* named the event-collector gate (8,927 Rust, 396 Lean,
   568 Python); the newest landing gate is r48's (§3). `STATUS-LATEST-GATE-STALE`
   is amended (§5).
2. *Projected functions* read 952 / 9 / 3,873 (W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT);
   the published `eight-home` baseline is 1,079 / 10 / 3,745 and the newest
   integrator measurement 1,119 / 12 / 3,703.
3. *Fluent Bit system slice* read 97 / 4 / 347 and the *tail-pause slice* 29 / 1 / 239 / 1
   Pending (an older 270-function selection); the baselines are 103 / 4 / 341 and
   58 / 1 / 341 / 4 Pending over 448 and 404 rows.
4. *Cycle 19 units* read 6 Safe / 8 / 167 / 4; the baseline is 9 Safe / 8 / 164 / 4,
   and the "unit Safe count is 6" sentence is replaced by the boundary alone.
5. *Lean toolchain* read 334 audits; the tree and the r48 gate have 474.
6. *Contract corpus* read 137 modules (table) and 140 (gate paragraph, with 86
   provider-owned / 54 shared); the eleven inventory locks hold **150** (91 in
   the five `qcore-provider-*` crates, 59 in the six `quirin-provider-*`
   crates), plus 25 user-crate modules outside the locks.
7. *Registered follow-ups* read 794; the matrix carries 1,044.

## 3. Headline figures, re-derived

From primary sources only: [`rederive.py`](scripts/rederive.py)
([output](artifacts/figures-rederived.json)) and [`landings.py`](scripts/landings.py)
([output](artifacts/landing-comparisons.txt)).

| Figure | Value | Source |
|---|---|---|
| `eight-home` baseline | 4,834 rows 1,079 / 0 / 3,745 / 10; units 9 / 164 / 8 / 4; null 2,549 / 2,355 / 17 | `contract/evaluations/eight-home/baseline.json` (`556460e31`) |
| `fluent-bit-system` baseline | 448 rows 103 / 0 / 341 / 4; null 211 / 243 | its `baseline.json` (`556460e31`) |
| `fluent-bit-tail-pause` baseline | 404 rows 58 / 4 / 341 / 1; null 190 / 216 | its `baseline.json` (`777fba16e`) |
| `twelve-home` baseline | 4,704 rows 1,083 / 0 / 3,615 / 6 (the card's own preparation) | its `baseline.json` (`974434463`) |
| Newest integrator route measurement | `2c07390b6`: corpus and twelve-home 1,119 / 0 / 3,703 / 12, system 112 / 0 / 332 / 4, slice 63 / 4 / 336 / 1 | `docs/reports/sp1-symbolic-openssl-premises/integration/native-comparison.json.gz` |
| Pinned routes after it | not measured (r41–r48 gate records: "Pinned-route parity … was not run") | each landing's `integration/gate.json` |
| Latest gate | r48 on `8e7e8d509`: Rust 9,835 / 0 / 21 (671 units, 716 summaries); Lean 474; Python 779 (two stale-matrix checks, rerun after regeneration); provider 168 / 1 ignored (49 summaries); Clang 21.1.8 | `docs/reports/cg10-indirect-access-sole-residuals-r4/integration/gate.json` |
| CT | 50/50 mapped; 48 distinct fixtures, 9 shared groups; proof 5 (CT01, CT03, CT07, CT37, CT50); sanitized-native 10 | `docs/reevaluation/ct_manifest.json` |
| Lean | `leanprover/lean4:v4.35.0`, 474 `proof/Audit*.lean` | `proof/lean-toolchain` |
| Contract corpus | 150 locked modules, 992 declarations, 8,749 clauses; 175 tracked `.qv` | `providers/*/contract-inventory.lock`, `git ls-files` |
| Follow-ups | 1,044 | `acceptance_matrix.json` `named_followups` |
| Full conformance | false | `acceptance.json`, matrix header |

The integrator comparisons also show a corpus figure that moved between
measured landings with no committed comparison of its own: 1,082 Safe at
CURRENT-CONTEXT-NEW-TRIAGE D3's tip `8e190aa83`, 1,078 at the CG25 bases
(`36b2cf931`, `5c824e443`), 1,079 at the AG bases (`a9c6dcf44`). The page states
this; it is registered as `LIVE-ROUTE-MOVES-BETWEEN-MEASURED-LANDINGS` (§5).

## 4. The unpublished-landings sentence: its shape for later landings

The integrator's landing step extends this sentence with `sed`. It is kept as
**one line**, delimited by two HTML comments,
`<!-- unpublished-landings: … -->` before and `<!-- unpublished-landings:end -->`
after, under `### Unpublished landings`. It begins `Unpublished landings (user
decision 2026-11-16):` as before, and the extendable tail is unchanged:

> … and r48 (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4) completed steps 1–7 of `scripts/land.py` …

A later landing makes the same edit as before: replace
`and r48 (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4) completed steps 1–7` with
`r48 (CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4) and r49 (CARD) completed steps 1–7`
(adding the comma). The 37 earlier landings, r10 to r47, are no longer spelled
out. The sentence names them as "the 37 landings from integrate/r10
(`458bd804d`) to r47 (CG10-READER-GAPS)", listed with their cards in checkpoint
part 1 (the old line 51 verbatim). A publication (`land.py publish`) replaces
the whole sentence.

## 5. Ledger (`integration-amendments.json`)

`status_paragraph` is `null`: a prepended paragraph would start the regrowth
again. The increment paragraph is the one record of this card.

| id | operation | what it records | discharge |
|---|---|---|---|
| `STATUS-REGROWTH-BY-PREPEND` | register | `apply_amendments.py` prepends a status paragraph per landing and the landing step lengthens the unpublished list; the page went 23,931 → 290,886 bytes in eight weeks. Proposal for the user: drop `status_paragraph` from schema `quirin-amendments/1` (or make it a `status_edits` list naming the section a fact replaces), have `land.py check` fail when `docs/STATUS.md` exceeds 32 KiB, and generate the unpublished-landings sentence from the landing gate records | a user decision, then the tooling change |
| `STATUS-COMPACT-RECONCILIATION-CLAIMS` | register | 9 frozen reconciliation `contains` claims on STATUS flip to failing ([before](artifacts/reconciliation-claims-before.txt), [after](artifacts/reconciliation-claims-after.txt); 16 → 25 failing of 27). Six held real text now only in the checkpoint (three `contract-cycle4-integration` sentences, `symbolic-decomposition-cycle3`'s batch-5 sentence, `w46-contract-access-expansion`'s card name twice). Three are the `281` claims, vacuous matches already registered as `W45-RECONCILIATION-SUBSTRING-CLAIMS`. No gate or CI step runs these claims. | the sweep resolves a STATUS claim against the indexed status checkpoint, or the claims are marked historical in a way the check honours |
| `LIVE-ROUTE-MOVES-BETWEEN-MEASURED-LANDINGS` | register | the corpus moves of §3 with no committed comparison | a comparison of `8e190aa83` against `36b2cf931` attributing the four rows and the one back |
| `STATUS-LATEST-GATE-STALE` | amend | the section now names the r48 landing gate | still open: no check ties STATUS's figures to a gate file |

## 6. Main revision folded in

`git fetch origin` before finishing found `origin/main` still at `2aefebbc1`, so
there was nothing to merge and the checkpoint is of `2aefebbc1:docs/STATUS.md`. If r49–r51 land
first, their STATUS edits conflict with this rewrite in `docs/STATUS.md`. The
resolution: rerun `checkpoint.py write <new main>` (it regenerates every part
from that revision), fold each new status paragraph's current facts into the
sections, and extend the unpublished sentence as §4 says.

## 7. Gate

Scoped card gate (`scripts/gate_scope.py --base 2aefebbc1`) through
`private/toolchain/env.sh`; exits and counts in [`gate.json`](gate.json), logs
under `private/status-compact/`. Docs only, so no Rust or Lean parity work.
