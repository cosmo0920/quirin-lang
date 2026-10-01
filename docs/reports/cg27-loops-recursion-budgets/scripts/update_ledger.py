#!/usr/bin/env python3
"""CG27-LOOPS-RECURSION-BUDGETS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON SIZES_JSON   (artifacts/ledger-sizes.json)

Retires W45-COUNTED-USERS-LOOPS (its DISCHARGE holds: a bounded loop rule, a
body whose loop touches no tracked count or opaque call is skipped, with Lean
parity `QCore.LoopInvariants.CountedLoop`, measured on the retry route: the
system and tail-pause slices). Amends CG27-LOOPS-RECURSION-BUDGETS,
W43-COLLECTION-LOOP-SHAPES, JOINED-PROVENANCE-CYCLE-SITES and C03-NFR-REACH
with this card's reading, and registers the card's residual. Keeps
`dimensions.named_followups` equal to the list length and writes the matrix
back in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: private/cg27lrb/{before,after}* (corpus, system, slice) through
scripts/census.py, and the C03-REACH census (scripts/c03_census.sh).
"""

import json
import sys

REPORT = "docs/reports/cg27-loops-recursion-budgets/README.md"
BY = "CG27-LOOPS-RECURSION-BUDGETS"

RETIRE = ["W45-COUNTED-USERS-LOOPS"]

AMEND = {
    "CG27-LOOPS-RECURSION-BUDGETS": (
        " AMENDED by CG27-LOOPS-RECURSION-BUDGETS itself (gateway rule 11x; report "
        "docs/reports/cg27-loops-recursion-budgets/README.md): landed on route G the counted-user loop summary "
        "(invariant: every count as the loop found it, checked flow-insensitively over every token of the loop; "
        "W45-COUNTED-USERS-LOOPS retired), the signed threshold rank for recursion (one int-or-wider signed control, "
        "sign guards n <= c / n > c at one literal threshold, actuals n - k with 1 <= k <= 1024 or 0, several ranks "
        "at once, mutual components; consulted after every other route), and budget tests (alias widening, the "
        "counted-user path bound, route T's path limit) with Lean QCore.LoopInvariants. Corpus "
        "__CORPUS__. The entry stays open for what the card did not land, sized in CG27-LOOP-RECURSION-RESIDUALS."
    ),
    "W43-COLLECTION-LOOP-SHAPES": (
        " RE-READ by CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): unchanged. CG27's loop summary is the "
        "counted-user pass's, not the collection pass's; no decrementing sweep, `while` header or member bound is "
        "read, and the collection-only zero fact stays. Its custody counterpart, the work list's settled loop-head "
        "state, is now stated as an inductive invariant (QCore.LoopInvariants.Induction."
        "postfixpoint_bounds_every_iterate), which is what a vacuity-aware shared zero fact would have to preserve."
    ),
    "JOINED-PROVENANCE-CYCLE-SITES": (
        " RE-READ by CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): unchanged and not lifted. Since "
        "CG04-LOOP-SITE-GENERATIONS (rule 11a) the custody flow does carry a per-iteration identity (current / "
        "previous / older generations), which is the first half of this entry's DISCHARGE; the provenance pre-pass "
        "still names a site, not a generation, so a fact on a site created inside a loop would have to be keyed to "
        "the current generation and invalidated at the shift. Not attempted here: the soundness change is the "
        "pre-pass's and needs its own Lean parity and re-measure."
    ),
    "C03-NFR-REACH": (
        " RE-READ by CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x): the C03-REACH census re-run at the card's tip "
        "(scripts/c03_census.sh) finds __C03__. The signed threshold rank, consulted after the neutral route, "
        "admits __C03_RANKED__ corpus components: the blockers stay the containment gate's retained instructions, "
        "not the rank."
    ),
}

REGISTER = [
    {
        "id": "CG27-LOOP-RECURSION-RESIDUALS",
        "title": "Loop and recursion shapes CG27 leaves refused: stated loop invariants, route-T back-edges, counting loops, recursion without a literal signed rank",
        "why": (
            "Registered by CG27-LOOPS-RECURSION-BUDGETS (gateway rule 11x). Each stays fail-closed: (1) stated "
            "loop-invariant contracts (a source or contract syntax naming a loop and its invariant, checked "
            "inductively) do not exist; the only invariants checked are the custody work list's settled head state "
            "and the counted-user pass's count-neutral summary. (2) Route T refuses every back-edge "
            "(CG09-CONTROL-SHAPE-RESIDUALS' consumers read finite path sets; a loop summary needs an admission class "
            "every reader of the summary decides). (3) A counted-user loop whose body performs a counted-user "
            "operation stays undecided (a per-iteration net count times an unknown iteration count needs a bound). "
            "(4) Recursion whose rank is not a literal signed threshold over one int-or-wider control: unsigned "
            "multi-rank (n - 2 wraps), char/short/_Bool controls, mixed control widths, non-literal or different "
            "thresholds, several integer formals, structural recursion over strings or heap (corpus: "
            "cio_os_mkpath, whose only non-Safe leaf is its own recursive call through dirname(dup_dir); system: "
            "flb_output_task_singleplex_flush_next <-> flb_output_task_queue_flush_one, whose rank would be the "
            "length of queue->pending, flush_next's only non-Safe leaf). (5) C08's "
            "budget improvements by shared DAGs or summaries (no budget changed). Sizes at the card's tip: "
            "__SIZES__. DISCHARGE: per item, the admission with its premise stated, Lean parity and a measured "
            "re-pin."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(sys.argv[2], encoding="utf-8") as handle:
        sizes = json.load(handle)
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            text = AMEND[entry["id"]]
            for key, value in sizes.items():
                text = text.replace(key, value)
            entry["why"] += text
            if REPORT not in entry.get("source", ""):
                entry["source"] = entry.get("source", "") + ", " + REPORT
    for registered in REGISTER:
        if registered["id"] in ids:
            raise SystemExit(f"{registered['id']} is already registered")
        entry = dict(registered)
        for key, value in sizes.items():
            entry["why"] = entry["why"].replace(key, value)
        entry["source"] = REPORT
        entry["recorded_by"] = BY
        followups.append(entry)
    touched = set(AMEND) | {registered["id"] for registered in REGISTER}
    for entry in followups:
        if entry["id"] in touched and any(key in entry["why"] for key in
                                          ("__CORPUS__", "__C03__", "__C03_RANKED__", "__SIZES__")):
            raise SystemExit(f"unfilled size in {entry['id']}")
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
