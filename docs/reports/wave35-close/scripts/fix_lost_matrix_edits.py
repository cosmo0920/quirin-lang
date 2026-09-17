#!/usr/bin/env python3
"""Wave-35 close: two matrix edits that a card asked for and a merge did not keep.

BOTH WERE FOUND BY MEASURING THE TREE RATHER THAN BY READING A REPORT, which is
the whole method the close rule exists to enforce. Both are the same shape as
the two the integrator already had to repair by hand this wave (`8d158c4e`,
restoring PYTHON-EVIDENCE-SUITE-INTERMITTENT-ROW) and in wave 32 (`3bbec4d0`,
losing the DOCS-COMPACT-O row): a hand-maintained JSON section resolved at a
merge, where a REMOVAL is silently dropped because a union-style resolution
keeps both sides.

1. MUTEX-SHARE-CROSS-UNEXERCISED-ARMS is retired.
   Card MUTEX-SHARE-CROSS-ARMS §11 item 3 says the residual "is DISCHARGED and
   is replaced in named_followups by a successor entry", and its own arithmetic
   is explicit: "80 -> 84 (one replaced, four added)" - five entries in, one
   out. All five successors ARE present at the tip. The removal is NOT: both
   the discharged entry and its successor MUTEX-SHARE-CROSS-ARM-C-REVIVAL stand
   side by side, and that successor's own `why` opens by stating it discharged
   the entry sitting beside it. So the ledger simultaneously carries a residual
   and its retirement notice. Retiring it here is applying the card's item, not
   a judgment of the close's own.

2. DOCS-COMPACT-4-TABLE-SYNC is registered, and
   W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE is retired.
   Card SWEEP-BLIND-SPOTS §6.3 gives the discharge as a disjunction - restore
   the pass-4 and pass-5 residual names to the DOCS-COMPACT-O row, OR register
   DOCS-COMPACT-4-TABLE-SYNC as its own entry - and says explicitly that the
   card did not do it because "named_followups prose for another card's row is
   integrator-owned recorded analysis, and the e0052b14 precedent is the
   integrator amending it". The close takes the SECOND arm, because the first
   would restore prose into a row describing a different card's finding while
   leaving the name itself untracked, which is the defect rather than its
   repair. Its sibling DOCS-COMPACT-5-RECONCILIATION-LIST is deliberately NOT
   registered: CLAUDE.md records it as already discharged by W32-RECONCILE-SWEEP
   ("card W32-RECONCILE-SWEEP, wave 32, discharging
   DOCS-COMPACT-5-RECONCILIATION-LIST"), so registering it would put closed work
   in the open-work ledger.
   The entry's `why` also asks a second question - whether a close should
   re-verify the `where` of verdicts recorded by an earlier close - and this
   close answers it by DOING it: every one of the 78 verdicts was measured
   against the tree, which is what found CT-CTWIN-LANE's unapplied CT row and
   both losses repaired here.

Net: 122 - 1 - 1 + 1 = 121.
"""

import json
import pathlib

MATRIX = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")

RETIRE = {
    "MUTEX-SHARE-CROSS-UNEXERCISED-ARMS": (
        "Discharged by card MUTEX-SHARE-CROSS-ARMS (wave 35), whose §11 item 3 asked for exactly "
        "this removal ('one replaced, four added') and whose successor entry "
        "MUTEX-SHARE-CROSS-ARM-C-REVIVAL was landed while the removal was not - a merge-resolution "
        "loss the wave-35 close measured, not a deliberate carry. All four arms the STABILITY-REEVAL "
        "#1 cadence measured are dispositioned: arms A and B are DRIVEN by "
        "crates/qcore-frontend/tests/mutex_share_cross_arms.rs with non-vacuity shown by three "
        "mutations, arms C and D are PROVED UNREACHABLE and kept as backstops justified by "
        "QCore.MutexShareCrossArms rather than by the suite, which still cannot reach either. NONE "
        "was retired. What stays open is carried by three successors: "
        "MUTEX-SHARE-CROSS-ARM-C-REVIVAL, MUTEX-SHARE-CROSS-ARM-D-REVIVAL and "
        "MUTEX-SHARE-CROSS-UNMODELED-PLACE-UNMEASURED."
    ),
    "W32-SWEEP-MATRIX-EDIT-LOST-AT-MERGE": (
        "Discharged by the wave-35 close, taking the second arm of the disjunction card "
        "SWEEP-BLIND-SPOTS §6.3 offered: DOCS-COMPACT-4-TABLE-SYNC is now registered as its own "
        "named_followups entry, so the name reaches the ledger rather than only a lost prose row. Its "
        "sibling DOCS-COMPACT-5-RECONCILIATION-LIST is deliberately NOT registered, CLAUDE.md "
        "recording it as already discharged by W32-RECONCILE-SWEEP. The entry's SECOND question - "
        "whether a close should re-verify the `where` of verdicts recorded by an earlier close - is "
        "answered by this close's practice rather than by a rule: all 78 integrator-addressed items "
        "were measured against the tree, and that is what found CT-CTWIN-LANE's entirely unapplied "
        "CT ledger row, the lost MUTEX-SHARE-CROSS-UNEXERCISED-ARMS retirement, and the stale clause "
        "in arbiter #1's proposed constraint 5c. The general lesson is recorded in the close's README "
        "rather than as a standing rule, because CLAUDE.md already says the tool decides nothing and "
        "only a person can measure the tree against a paragraph of English."
    ),
}

ADD = [
    {
        "id": "DOCS-COMPACT-4-TABLE-SYNC",
        "title": "The compaction pass's own finding about table sync reaches no tracker, and was lost at a merge twice over",
        "why": (
            "DOCS-COMPACT-4 (wave 30) filed this finding: a card's integrator-addressed table is "
            "applied for its HEADLINE NUMBERS while the rest of the table is left, so amendments "
            "silently do not land. It is one of the two findings that NAMED the reconciliation defect "
            "and, with grim symmetry, both were themselves in the untracked set - the observation that "
            "motivated W32-RECONCILE-SWEEP. Card W32-RECONCILE-SWEEP (wave 32) amended the matrix to "
            "carry both names in the DOCS-COMPACT-O row, and that amendment was RESOLVED AWAY at its "
            "own merge 3bbec4d0: card SWEEP-BLIND-SPOTS (wave 35) measured the row at 5,473 bytes "
            "before the merge and 3,265 after, the ONLY named_followups row to change across it, "
            "-2,208 bytes, with HEAD byte-equal to the post-merge version. Registered here as its own "
            "entry by the wave-35 close, which is the second arm of the discharge SWEEP-BLIND-SPOTS "
            "§6.3 proposed and the arm that puts the NAME in the ledger rather than prose in another "
            "card's row. DISCHARGE: a compaction or hygiene pass that measures, over a named range of "
            "waves, how many integrator-addressed items were applied beyond their headline numbers - "
            "the quantity this finding asserts is low - and either repairs the shortfall or records "
            "that the reconciliation-sweep mechanism has closed it. Note that the mechanism now exists "
            "and did not in wave 30, so the honest measurement is of the waves SINCE it landed. NOT a "
            "discharge: observing that recent closes look thorough."
        ),
        "source": "docs/reports/sweep-blind-spots/README.md",
        "recorded_by": "WAVE-35-CLOSE",
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    before = len(followups)

    removed, kept = [], []
    for entry in followups:
        eid = entry.get("id") if isinstance(entry, dict) else None
        if eid in RETIRE:
            removed.append(eid)
        else:
            kept.append(entry)

    present = {e.get("id") for e in kept if isinstance(e, dict)}
    added = []
    for entry in ADD:
        if entry["id"] in present:
            continue
        kept.append(entry)
        added.append(entry["id"])

    matrix["named_followups"] = kept
    matrix["dimensions"]["named_followups"] = len(kept)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")

    print(f"before: {before}")
    print(f"retired ({len(removed)}): {', '.join(sorted(removed))}")
    print(f"added   ({len(added)}): {', '.join(added)}")
    print(f"after:  {len(kept)}")


if __name__ == "__main__":
    main()
