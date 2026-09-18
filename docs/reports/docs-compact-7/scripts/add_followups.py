#!/usr/bin/env python3
"""Append card DOCS-COMPACT-7's registrations to the acceptance matrix.

`named_followups` is hand-maintained in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates. It then rewrites
with the generator's byte format - indent=1, ensure_ascii=False, trailing
newline (card HYGIENE-16) - so the diff stays reviewable.

Run the generator afterwards so `dimensions.named_followups` follows, or the
committed-matrix tripwire D02-DIMENSIONS-FOLLOWUP-COUNT goes red:

    python3 -B docs/reports/remaining-d02/build_matrix.py .

Idempotent: an id already present is skipped.

REBASE NOTE (onto 2caf7369, the final pre-close wave-37 main). This card's first
pass registered four names. Two of them - W36-COVER-HIDDEN-BLOCKER and
CJSON-DELETE-NESTED-DETACH - were registered by WAVE-36-CLOSE in the meantime,
as part of its sweep of all nine of W36-CORPUS-CONTRACTS-3's SS9 residuals. They
are therefore DROPPED here rather than re-registered: the close reached them
first and its entries stand. The finding that they were untracked was correct
when made; it is now discharged by someone else, which is the right outcome and
is recorded in this card's report rather than re-asserted in the matrix.

Scope note. This card owns AGENTS.md, CLAUDE.md and README.md. It registers only
residual-shaped names that appear IN THOSE THREE FILES and reach no tracker, or
that this card itself measured.
"""

import json
import pathlib
import sys

p = pathlib.Path(sys.argv[1] if len(sys.argv) > 1
                 else "docs/reviews/r0_3/acceptance_matrix.json")
m = json.loads(p.read_text(encoding="utf-8"))
nf = m["named_followups"]
have = {e["id"] for e in nf}

new = [
    {
        "id": "Q-SPACE-EXTENSION-TALLY-HAND-KEPT",
        "title": "The Q-space note's condition-extension tally is hand-kept prose and carries a false universal about wave 32",
        "why": "MEASURED by card DOCS-COMPACT-7 (wave 37), and re-measured unchanged on the wave-37 tip 2caf7369. AGENTS.md's Q-NAMESPACE-DECISION note states 'Waves 32-34 allocated no code and ran no condition extension either', putting the count before WRITE-LEND-TASKS-EXERCISE at thirteen and calling that card the fourteenth. The same file's own diagnostics table contradicts it twice: the Q600 row says MUTEX-SHARE-CROSS (wave 32) 'is the fifth' card widening that band's boundary condition while allocating nothing, and the Q610-Q627 row says the same card reaches Q611 from a new route and allocates nothing. So a wave-32 card DID run the consequence, the universal was false, and the tally was one short - the same shape as CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE, which card SWEEP-BLIND-SPOTS measured in the neighbouring constraint-14 bullet in wave 35, about the same wave. DOCS-COMPACT-7 corrected the note by narrowing the quantifier to the waves actually enumerated and giving MUTEX-SHARE-CROSS its row, re-deriving the totals from the note's own enumeration plus the two table rows: fifteen firings across ten bands (wave 37 adds none - verified that no wave-37 card allocates a code or widens a condition). The residual is the METHOD, not the arithmetic: nothing derives this tally from the spec's SSL21 allocation table or from the diagnostics rows, so it is prose that has now been wrong once and can drift again silently, and its two halves live in two places that no check compares. Discharge: derive the tally (or at least cross-check the note against the table rows) in the matrix generator or a test, or restate the note as narrative that names no total.",
        "source": "AGENTS.md Q-space note and the Q600 / Q610-Q627 diagnostics rows; docs/reports/docs-compact-7/README.md",
        "recorded_by": "DOCS-COMPACT-7",
    },
    {
        "id": "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT",
        "title": "The merged-gate row's Lean figure has disagreed with the close it names in two consecutive waves, and the close's gate.json schema cannot hold it",
        "why": "MEASURED TWICE by card DOCS-COMPACT-7. (1) At this card's first base 477b733d, AGENTS.md's merged-gate row read '135 Lean audits' attributed to the WAVE-35 CLOSE; docs/reports/wave35-close/gate.json records that close measuring 132. (2) After rebasing onto the wave-37 tip 2caf7369, the row reads '137 Lean audits' attributed to the WAVE-36 CLOSE; that close measured 135 - stated in its README SS gate table ('exit 0 - 135 audits') and three times in its own gate.json `note` ('135 Lean audits'; 'Lean 132 -> 135 (+3)') - and 137 is the count AFTER wave 37's JOINED-PROVENANCE-ROUND-BOUND added two modules. Both times the figure is the CURRENT FILE COUNT wearing an older close's label, and both times every other figure in the row matched its named source. The mechanism is visible and is the reason this is a distinct residual rather than an instance of INTEGRATOR-HEADLINE-COUNT-HAND-WRITTEN-UNCHECKED: docs/reports/wave36-close/gate.json has NINE keys and NONE of them is a Lean count, so the one figure in the row with no machine-readable source in the record it cites is exactly the one that has drifted twice. Nothing is unsound - the audits do pass, at whatever the current count is - but a row whose label and figure come from different places is not a measurement. Discharge: give the close's gate.json a lean_audits key (and any other figure the row publishes) so the row can be checked against it, or derive the row from that file; failing either, drop the attribution and label the row with the tip it was measured at.",
        "source": "AGENTS.md merged-gate row; docs/reports/wave35-close/gate.json; docs/reports/wave36-close/gate.json and README.md; docs/reports/docs-compact-7/README.md",
        "recorded_by": "DOCS-COMPACT-7",
    },
    {
        "id": "MAIN-GUARD-REFUSES-PACK-REFS",
        "title": "The main-ref guard refuses git pack-refs in a delegated worktree, a transaction that changes no ref value",
        "why": "MEASURED by card DOCS-COMPACT-7 (wave 37), incidentally, while committing. CARD-SCOPE-GUARD's reference-transaction hook sees LOCAL ref updates from every worktree and refuses any update to refs/heads/main from a delegated agent-* worktree. git pack-refs - which git's automatic `gc` runs - re-writes every ref into packed-refs, and the resulting transaction for refs/heads/main carries old = the null OID and new = the ref's EXISTING value. The guard sees an update to refs/heads/main and refuses it. Reproduced deterministically: `git pack-refs --all` from the card worktree prints the guard's refusal, and .git/qcore-main-guard.log records `refused-provenance:cwd-in-card-worktree,ancestor-in-card-worktree,git-dir-is-card-worktree refs/heads/main 0000000000000000000000000000000000000000->477b733d...` - old and new naming the SAME value, so nothing was being advanced. Observed consequence: `git commit` succeeds and then its background `gc` task fails with `error: failed to run pack-refs` / `error: task 'gc' failed`, so a card sees an alarming guard banner attached to an operation that was not a main update at all. NOT a soundness problem for the guard - it fails closed, in the refusing direction, and no delegated commit is lost - but it is a false positive that trains readers to ignore the banner, which is the one thing a guard cannot afford, and it leaves repository maintenance broken in every delegated worktree. Discharge: exempt a transaction whose new value equals the ref's current value (a no-op repack) or whose old value is the null OID under pack-refs, shown non-vacuous by a test that still refuses a real advance; or record the behaviour in CLAUDE.md as intended, with the maintenance cost stated. DOCS-COMPACT-7 landed only the documentation half - it owns no hook.",
        "source": "docs/reports/docs-compact-7/README.md; .git/qcore-main-guard.log; docs/reports/card-scope-guard/README.md",
        "recorded_by": "DOCS-COMPACT-7",
    },
]

added = [e for e in new if e["id"] not in have]
nf.extend(added)
p.write_text(json.dumps(m, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("added:", [e["id"] for e in added])
print("skipped (already present):", [e["id"] for e in new if e["id"] in have])
print("named_followups now:", len(nf))
print("dimensions.named_followups (stale until regeneration):",
      m["dimensions"]["named_followups"])
