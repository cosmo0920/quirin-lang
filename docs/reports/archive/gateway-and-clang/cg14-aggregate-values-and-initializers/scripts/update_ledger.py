#!/usr/bin/env python3
"""CG14-AGGREGATE-VALUES-AND-INITIALIZERS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends CG14-AGGREGATE-VALUES-AND-INITIALIZERS (not every DISCHARGE clause
holds: pointer-bearing copies, aggregate values in actuals, returns and
conditions, and route T stay open); registers the three residual classes
with their sizes; keeps `dimensions.named_followups` equal to the list
length. Writes the matrix back in its own form (indent=1, ensure_ascii=False,
trailing newline).

Sizes are the card's AFTER run (private/cg14/after2*, corpus arbiter
--jobs 4, base c374de96, tip binary 7ebbdc45), read with scripts/residuals.py
(first parity row per statement) and scripts/entries.py (CG00's leaves found
again by their stable identity).
"""

import json
import sys

REPORT = "docs/reports/cg14-aggregate-values-and-initializers/README.md"
BY = "CG14-AGGREGATE-VALUES-AND-INITIALIZERS"

REGISTER = [
    {
        "id": "CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES",
        "title": "A whole copy of a record with a pointer member, and an initializer storing a value that may carry an identity, stay refused: 174 corpus statements",
        "why": (
            "Measured by CG14-AGGREGATE-VALUES-AND-INITIALIZERS (gateway rule 11f) on its AFTER corpus run (base "
            "c374de96, scripts/residuals.py, first parity row per statement): 75 whole-record copies on 61 functions "
            "refuse `pointer-members` (e.g. `rkbuf->rkbuf_replyq = replyq`, `*dst = *src` of `rd_kafka_AdminOptions_t`, "
            "`LZ4F_CustomMem` members), and 99 braced initializers refuse a leaf that may carry an identity (an address "
            "30, a pointer-bearing record value 27, a formal's pointer 22, a call 13, a local's pointer 4, a pointer "
            "read 3), plus 22 whose scalar leaf the typed reader refuses. A copied pointer member is an alias, never a "
            "second owner, but nothing states which copy owns or that the source's members own nothing "
            "(`QCore.AggregateValue.releasing_both_copies_is_a_double_release`; the owned-field model makes such a "
            "record uncopyable). DISCHARGE: a field-wise alias transfer in the flow (each copied pointer member bound "
            "as the source member's alias, the source's owner premise carried: CG10-BOUND-UNTRACKED-ROOTS' untracked "
            "reading or a tracked field link), or contract/owned-field evidence naming the owning copy, with Lean "
            "parity and a measured re-pin; an initial alias of a tracked identity bound as a place binding of the "
            "local with the same premises."
        ),
    },
    {
        "id": "CG14-AGGREGATE-VALUE-POSITIONS",
        "title": "Record values and compound literals outside a statement copy or a borrowed call actual stay refused: 221 M08 and 97 M10 corpus leaves",
        "why": (
            "Measured by CG14-AGGREGATE-VALUES-AND-INITIALIZERS on its AFTER corpus run (scripts/entries.py, CG00's "
            "leaves found again by unit, function, origin and description): of CG00's M08 leaves 221 remain (282 on "
            "the base), a record passed by value (`LZ4F_free(p, cdict->cmem)`), returned "
            "(`rd_kafka_toppar_fetch_decide_next_fetch_start_pos`), compared in a condition "
            "(`RD_KAFKA_UUID_IS_ZERO(...)`, `rd_kafka_pid_eq(...)`) or copied from a call or conditional (34 "
            "`copy-operand:source` statements); all 97 of CG00's M10 leaves remain: 40 literal actuals on 31 "
            "functions reach an uncontracted callee (`literal-callee-uncontracted`, e.g. `RD_KAFKA_NO_REPLYQ` at "
            "`rd_kafka_toppar_op_pause_resume`), 8 sit inside an actual or a non-call statement, and the rest are "
            "in `return`, conditions and compound assignments (`fails += f(..., (const char[]){...})`), which this "
            "rule does not consult. DISCHARGE: the custody-free record value read at a borrowed or data formal and "
            "in a scalar-result return/condition through the rule-10u acceptance, and the literal read at those "
            "consumers under decide_literal, each measured with Lean parity; the uncontracted callees are the "
            "callee-closure cards' (W48-CALLEE-CLOSURE-R3)."
        ),
    },
    {
        "id": "CG14-DECLARATION-FORMS",
        "title": "Declarations the aggregate fallback does not read: several declarators, static storage, no single initializer: 291 corpus statements",
        "why": (
            "Measured by CG14-AGGREGATE-VALUES-AND-INITIALIZERS on its AFTER corpus run (scripts/residuals.py): a "
            "refused declaration is read by rule 11f only as one automatic local with one initializer. 96 statements "
            "on 84 functions declare several objects, 102 on 85 are not automatic (static or thread storage: "
            "CG15-STORAGE-DURATION-AND-VOLATILE's), and 93 on 70 carry no single initializer (not every one of them "
            "is an aggregate: the checks precede the type's). DISCHARGE: per-declarator lowering with each "
            "declarator read by the rule, and CG15's storage-duration identity for the static ones, measured."
        ),
    },
]

AMEND = {
    "CG14-AGGREGATE-VALUES-AND-INITIALIZERS": (
        " AMENDED by CG14-AGGREGATE-VALUES-AND-INITIALIZERS itself (wave 48 round 7, gateway rule 11f, "
        "QCore.AggregateValue): route G reads a whole copy of a custody-free record as the source's typed read and "
        "the destination's typed store, a braced initializer of an automatic record or array whose every leaf "
        "carries no identity (zero-fill, pointers null; a local that can hold a pointer through rule 10v's flow "
        "guard, publishing no summary), a compound literal only as a whole actual at a Borrowed formal of a "
        "contracted call, and a character array's string initializer with its extent recorded and no bound "
        "claimed. Corpus 608/10/4,216 -> 616/10/4,208 (+8 Unknown -> Safe: LZ4_isLittleEndian and "
        "XXH_isLittleEndian directly, six callers through their derived summaries), no Safe -> non-Safe, no leak "
        "or violation added; system 41/2/191 and tail-pause slice 28/1/240/1 Pending unmoved. CG00's leaves "
        "found again: M08 282 -> 221, M09 427 -> 129, M10 97 -> 97, M11 15 -> 13. Remaining: "
        "CG14-POINTER-BEARING-COPIES-AND-INITIAL-ALIASES, CG14-AGGREGATE-VALUE-POSITIONS, "
        "CG14-DECLARATION-FORMS; route T's initializer is unchanged (no T consumer decides one). "
        "Report: " + REPORT + "."
    ),
}


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    followups = matrix["named_followups"]
    ids = {entry["id"] for entry in followups}
    for name, text in AMEND.items():
        [entry] = [entry for entry in followups if entry["id"] == name]
        if text not in entry["why"]:
            entry["why"] += text
    for entry in REGISTER:
        if entry["id"] in ids:
            raise SystemExit(f"{entry['id']} is already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        stream.write(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
