#!/usr/bin/env python3
"""Append card DOCS-COMPACT-7's registrations to the acceptance matrix.

`named_followups` is hand-maintained in the generator's `section_ownership`
(`docs/reports/remaining-d02/build_matrix.py` carries it verbatim from the
committed matrix), so this appends rather than regenerates. It then rewrites
with the generator's byte format - indent=1, ensure_ascii=False, trailing
newline (card HYGIENE-16) - so the diff stays reviewable.

Run the generator afterwards so `dimensions.named_followups` follows:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/wave35-close/gate.json

Idempotent: an id already present is skipped.

Scope note. This card owns AGENTS.md, CLAUDE.md and README.md. It registers
only residual-shaped names that appear IN THOSE THREE FILES and reach no
tracker. Ten further wave-36 report residuals reach no tracker either -
ADD-NAME-COLLISION, CJSON-DELETE-SIBLING-CHAIN, JOINED-PROVENANCE-FACTS-NAME-
NOTHING, KAFKA-MEM-FREE-SUMMARY-CONFLICT, LIBC-MEM-RESULT-ALIAS,
MK-PRINT-TAIL-ACTUALS, STANDALONE-HOST-CLOSED-INPUT-LOAD,
SWEEP-RESIDUALS-SCANS-REPORTS-ONLY, W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE,
W36-CTRACES-FAMILY-FIRST - but none is named in the three files, so they are
the wave-36 close's reconciliation business and are enumerated in this card's
report rather than registered here.
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
        "id": "W36-COVER-HIDDEN-BLOCKER",
        "title": "A sole-blocker cover is an upper bound on function moves, never a forecast",
        "why": "Named in card W36-CORPUS-CONTRACTS-3's own follow-on table (docs/reports/w36-corpus-contracts-3/README.md SS9) and carried in AGENTS.md's Cycle 19 raw corpus row, but reaching no entry in this ledger until card DOCS-COMPACT-7 (wave 37) swept the three authoritative files for residual-shaped names absent from named_followups. MEASURED: a cover that counts the functions whose SOLE remaining blocker a declaration would decide is an upper bound on the function moves that declaration buys, because a refusal reachable only once the first is removed is invisible to the cover. It cost that card 3 of 23 predicted moves - the cJSON_Delete trio, which exchanged its decided leaf for a `nested call ... inside a modeled call` refusal that was unreachable while the outer call was uncontracted. This is a property of the prediction method, not a defect in any verdict: every published count is the measured AFTER, and the cover is used to CHOOSE declarations, not to assert results. Discharge: a cover that models the projection's reachability rather than only its emitted leaves, or acceptance as a stated boundary on how a corpus-contracts card may present a prediction.",
        "source": "docs/reports/w36-corpus-contracts-3/README.md SS9 and SS5.1; AGENTS.md Cycle 19 raw corpus row",
        "recorded_by": "DOCS-COMPACT-7",
    },
    {
        "id": "CJSON-DELETE-NESTED-DETACH",
        "title": "Three cJSON functions exchanged a decided leaf for a nested-call-inside-a-modeled-call refusal",
        "why": "Named in card W36-CORPUS-CONTRACTS-3's own follow-on table (docs/reports/w36-corpus-contracts-3/README.md SS9, measured in SS5.1) and carried in AGENTS.md's Cycle 19 raw corpus row, reaching no entry in this ledger until card DOCS-COMPACT-7 (wave 37). MEASURED: declaring librdkafka/cjson.qcore decided the outer call's leaf for the three cJSON_Delete functions and thereby EXPOSED a second refusal underneath it - a nested call in argument position inside a modeled call - which the projection could not reach while the outer call was uncontracted. The three functions therefore did not move to Safe. Nothing is promoted and nothing regressed: the exchange is refusal for refusal and is the fail-closed direction (invariant 1). It is the occasioning case for W36-COVER-HIDDEN-BLOCKER. Discharge: admit a nested call in argument position inside a modeled call, or derive W4 summaries for the three cJSON_DetachItemFrom* callees, which are defined in the same translation unit - subject to constraint 4's measured caveat that being defined in the unit is not being W4-derivable.",
        "source": "docs/reports/w36-corpus-contracts-3/README.md SS9 and SS5.1; AGENTS.md Cycle 19 raw corpus row",
        "recorded_by": "DOCS-COMPACT-7",
    },
    {
        "id": "Q-SPACE-EXTENSION-TALLY-HAND-KEPT",
        "title": "The Q-space note's condition-extension tally is hand-kept prose and carried a false universal about wave 32",
        "why": "MEASURED by card DOCS-COMPACT-7 (wave 37). AGENTS.md's Q-NAMESPACE-DECISION note stated 'Waves 32-34 allocated no code and ran no condition extension either', putting the count before WRITE-LEND-TASKS-EXERCISE at thirteen and calling that card the fourteenth. The same file's own diagnostics table contradicts it twice: the Q600 row says MUTEX-SHARE-CROSS (wave 32) 'is the fifth' card widening that band's boundary condition while allocating nothing, and the Q610-Q627 row says the same card reaches Q611 from a new route and allocates nothing. So a wave-32 card DID run the consequence, the universal was false, and the tally was one short - the same shape as CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE, which card SWEEP-BLIND-SPOTS measured in the neighbouring constraint-14 bullet in wave 35, about the same wave. DOCS-COMPACT-7 corrected the note by narrowing the quantifier to the waves actually enumerated and giving MUTEX-SHARE-CROSS its row, re-deriving the totals from the note's own enumeration plus the two table rows: fifteen firings across ten bands. The residual is the METHOD, not the arithmetic: nothing derives this tally from the spec's SSL21 allocation table or from the diagnostics rows, so it is prose that has now been wrong once and can drift again silently, and its two halves live in two places that no check compares. Discharge: derive the tally (or at least cross-check the note against the table rows) in the matrix generator or a test, or restate the note as narrative that names no total.",
        "source": "AGENTS.md Q-space note and the Q600 / Q610-Q627 diagnostics rows; docs/reports/docs-compact-7/README.md",
        "recorded_by": "DOCS-COMPACT-7",
    },
    {
        "id": "MAIN-GUARD-REFUSES-PACK-REFS",
        "title": "The main-ref guard refuses git pack-refs in a delegated worktree, a transaction that changes no ref value",
        "why": "MEASURED by card DOCS-COMPACT-7 (wave 37), incidentally, while committing. CARD-SCOPE-GUARD's reference-transaction hook sees LOCAL ref updates from every worktree and refuses any update to refs/heads/main from a delegated agent-* worktree. git pack-refs - which git's automatic `gc` runs - re-writes every ref into packed-refs, and the resulting transaction for refs/heads/main carries old = the null OID and new = the ref's EXISTING value. The guard sees an update to refs/heads/main and refuses it. Reproduced deterministically: with main at 477b733d, `git pack-refs --all` from the card worktree prints the guard's refusal, and .git/qcore-main-guard.log records `refused-provenance:cwd-in-card-worktree,ancestor-in-card-worktree,git-dir-is-card-worktree refs/heads/main 0000000000000000000000000000000000000000->477b733d...` - old and new naming the SAME value, so nothing was being advanced. Observed consequence: `git commit` succeeds and then its background `gc` task fails with `error: failed to run pack-refs` / `error: task 'gc' failed`, so a card sees an alarming guard banner attached to an operation that was not a main update at all. NOT a soundness problem for the guard - it fails closed, in the refusing direction, and no delegated commit is lost - but it is a false positive that trains readers to ignore the banner, which is the one thing a guard cannot afford, and it leaves repository maintenance broken in every delegated worktree. Discharge: exempt a transaction whose new value equals the ref's current value (a no-op repack) or whose old value is the null OID under pack-refs, shown non-vacuous by a test that still refuses a real advance; or record the behaviour in CLAUDE.md as intended, with the maintenance cost stated. DOCS-COMPACT-7 landed only the documentation half - it owns no hook.",
        "source": "docs/reports/docs-compact-7/README.md; .git/qcore-main-guard.log; docs/reports/card-scope-guard/README.md",
        "recorded_by": "DOCS-COMPACT-7",
    },
]

added = [e for e in new if e["id"] not in have]
nf.extend(added)
p.write_text(json.dumps(m, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("added:", [e["id"] for e in added])
print("named_followups now:", len(nf))
print("dimensions.named_followups (stale until regeneration):",
      m["dimensions"]["named_followups"])
