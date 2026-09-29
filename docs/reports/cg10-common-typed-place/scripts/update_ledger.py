#!/usr/bin/env python3
"""CG10-COMMON-TYPED-PLACE: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Amends CG10-COMMON-TYPED-PLACE (not every DISCHARGE clause holds), retires
W46-ACCESS-PATH-TRACKED-MIXED-PLACES and W44-AGGREGATE-PREMISE3-MEMBER-NAME
(discharged), amends W44-CHAIN-NESTED-AGGREGATE, W43-INDIRECT-UNTYPED-SINGLE-ACCESS
and REFUSED-HANDON-LINE-GRANULARITY, registers CG10-BOUND-UNTRACKED-ROOTS and
CG10-TYPED-PLACE-RESIDUALS, and keeps `dimensions.named_followups` equal to the
list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg10-common-typed-place/README.md"
CARD = "CG10-COMMON-TYPED-PLACE"

AMEND = {
    CARD: (
        " AMENDED (CG10-COMMON-TYPED-PLACE, wave 48 round 2): landed gateway rule 10t "
        "(typed_place.rs): the common typed place (DeclID root, FieldID/index/deref steps with "
        "Clang types and views), mixed paths on a tracked root lowered to the chain obligation "
        "(root only, or root plus the direct member place R->F; never-bound premise off), I04 "
        "statement identity by the consumed run's own occurrence, QCore.TypedPlace (audited, "
        "vectors bound). BEFORE reproduced ae17152b byte-identically and the M01/I04 controls; "
        "PREDICTION.md frozen; AFTER corpus 527 / 10 / 4,297 (one Unknown -> Safe, "
        "rd_kafka_anyconf_is_modified; 800 functions move, every one attributed; no Safe -> "
        "non-Safe, no leak or violation added), system and tail-pause slice 0 status moves. Of "
        "CG00's 10,186 M01/I04 leaves 1,458 are gone (1,454 replaced by an unknown_alias at the "
        "line: the place is read and its root fails the flow), 8,728 stay. REMAINS: the absorbed "
        "census classes (W43-INDIRECT-UNTYPED-SINGLE-ACCESS, W44-CHAIN-NESTED-AGGREGATE) were not "
        "re-measured under their own registering scripts (their census streams need the retired "
        "compiler example's runner); the dominant blocker is registered as "
        "CG10-BOUND-UNTRACKED-ROOTS and the refused place shapes as CG10-TYPED-PLACE-RESIDUALS."
    ),
    "W44-CHAIN-NESTED-AGGREGATE": (
        " AMENDED (CG10-COMMON-TYPED-PLACE): the Clang-typed fallback (rule 10t) now admits a `.` "
        "selection of an embedded record as a same-object step after the root and after the "
        "direct member link on a tracked root (R->in.x, R->F->in.x), with the chain obligations "
        "(QCore.TypedPlace, over QCore.IndirectChainAccess); on a never-bound root W46 already "
        "did. The lexical chain reader is unchanged, so the class's own census still names these "
        "shapes. Not re-measured under its registering script (chain_census.py needs the census "
        "streams of the retired compiler-example runner); the corpus effect is in the CG10 report "
        "(1,314 M01 leaves gone, almost all to an unknown_alias on a bound, untracked root)."
    ),
    "W43-INDIRECT-UNTYPED-SINGLE-ACCESS": (
        " AMENDED (CG10-COMMON-TYPED-PLACE): no reader repair landed for this class; rule 10t "
        "reads mixed places on tracked roots, which does not change a single access's typing. "
        "Not re-measured under its registering script (residuals.py needs the census streams of "
        "the retired compiler-example runner)."
    ),
    "REFUSED-HANDON-LINE-GRANULARITY": (
        " AMENDED (CG10-COMMON-TYPED-PLACE): unchanged by the card. Rule 10t introduces a "
        "statement-granular identity for a consumed statement (the parser's stream index and "
        "length, `typed_place_origin`) that a statement-granular hand-on table could key by; the "
        "refused-node hand-on rule still keys by line and joins by union. The approximation stands "
        "(it only withdraws); size not measurable by its own definition, which names none."
    ),
}

RETIRE = ["W46-ACCESS-PATH-TRACKED-MIXED-PLACES", "W44-AGGREGATE-PREMISE3-MEMBER-NAME"]

NEW = [
    {
        "id": "CG10-BOUND-UNTRACKED-ROOTS",
        "title": "A typed place whose root the body may bind but does not track (a bound formal, a borrowed alias, an opaque read) is refused at the flow: 2,550 unknown_alias leaves on 800 corpus functions",
        "why": (
            "Measured by CG10-COMMON-TYPED-PLACE (corpus AFTER b63c1b57 against BEFORE ae17152b): rule 10t "
            "reads 1,454 of CG00's M01/I04 leaves as tracked-root places, and the chain root reading "
            "(chain_resolves: exactly one live identity, or never bound with no binding) refuses almost "
            "all of them, adding 2,550 unknown_alias leaves (top roots rk 641, rkb 391, rktp 157, rkcg 132, "
            "rkbuf 126, sr 68) on 800 functions; one moves to Safe. System slice 19 functions and "
            "tail-pause slice 20 functions move the same way (0 status). The roots are formals or locals "
            "the custody closure cannot prove never bound (an actual at a non-Borrowed position, an "
            "assignment from a call) but that carry no tracked identity. Treating such a root as "
            "untracked is unsound where the body hands the object to a custody-taking position. "
            "DISCHARGE: a premise that proves, per path, that no custody position received the root "
            "before the access (or a measured decision that the refusal stands), with Lean parity, "
            "Rust refusal controls and a measured re-pin."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "CG10-TYPED-PLACE-RESIDUALS",
        "title": "Refused typed-place shapes and identity: array roots 366, element places 245, deeper places 177, identity 12 place decisions; I04 outside statements; 6,354 M01 statement leaves with no recorded cause",
        "why": (
            "Measured by CG10-COMMON-TYPED-PLACE (corpus AFTER, typed-place parity rows): the tracked-root "
            "route refuses by decision an array root (366), a pointer loaded from the root's element or "
            "pointee (245, R[i]->x), a pointer loaded from a place deeper than R->F (177, R->in.p->x), and "
            "a root whose spelling names another declaration or storage (12). I04: 1,024 of CG00's "
            "shared-line leaves stay (a condition, return or call-actual operand is a copy of the stream "
            "with no occurrence, so only consumed statements are told apart; 35 name-identity leaves stay). "
            "CORRECTION to CG00's M01 residue: of its 7,669 remaining M01 leaves, 6,354 are statements the "
            "typed statement reader refuses with no parity row (not a root-premise refusal: rule 10t never "
            "saw them), 121 more have no row, 919 are positions whose refusal every root premise granted would lift, and 275 are "
            "positions refused for another construct. DISCHARGE: a per-statement refusal row that splits "
            "the 6,354 by cause; element and deeper places with projection evidence (CG11's owned-field "
            "model); occurrence identity for position operands; each with Lean parity and a measured re-pin."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    path = sys.argv[1]
    matrix = json.load(open(path))
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    if any(entry["id"] in ids for entry in NEW):
        raise SystemExit("already applied")
    for name in list(AMEND) + RETIRE:
        if name not in ids:
            raise SystemExit(f"missing {name}")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    followups.extend(NEW)
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w") as out:
        json.dump(matrix, out, indent=1, ensure_ascii=False)
        out.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
