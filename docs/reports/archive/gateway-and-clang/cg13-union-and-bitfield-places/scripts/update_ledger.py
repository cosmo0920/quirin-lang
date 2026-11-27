#!/usr/bin/env python3
"""CG13-UNION-AND-BITFIELD-PLACES: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON PUNNED_JSON

Retires CG13-UNION-AND-BITFIELD-PLACES and the absorbed W46-UNION-MEMBER-PATH,
amends W44-CHAIN-INCOMPLETE-RECORD (its union part), W43-LIBC-GETPEERNAME-
SOCKADDR-UNION (the decision), CG11-POINTER-VALUED-STORES,
CG11-UNREAD-POINTER-PLACES and CG18-FOOTPRINT-CONNECTION, registers the card's residuals with their measured
sizes (PUNNED_JSON is `punned.py`'s output), and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg13-union-and-bitfield-places/README.md"
BY = "CG13-UNION-AND-BITFIELD-PLACES"


def register(punned):
    return [
        {
            "id": "CG13-UNION-PUNNING-FLOW-INSENSITIVE",
            "title": (
                f"A union pointer read is refused whenever the function stores through another member of that union "
                f"anywhere ({punned['consultations']} refused consultations over {punned['functions']} function names)"
            ),
            "why": (
                "Found by CG13-UNION-AND-BITFIELD-PLACES (gateway rule 10w): the punning check "
                "(`union_bitfield::punned`) is flow- and tag-insensitive. A pointer value read through a union member "
                "(or a pointer loaded through one to read through) is refused when the function stores through, "
                "increments or takes the address of another member of a union of the same record, or writes a whole "
                "object of its type, wherever that store is. Tagged unions written in one arm and read in another "
                "(the OpenTelemetry `value_case`/`data_case` initialisers, `initialize_metric`, "
                "`otlp_any_value_initialize`) therefore stay refused. Measured on the corpus arbiter at the card's "
                f"final tip from its parity rows (`position: union-bitfield`, outcome `refused-punned`, deduplicated "
                f"by function, node offset and outcome): {punned['consultations']} consultations over {punned['functions']} "
                "function names. A callee's store through another member is not visible at all: the premise "
                "`union-live-member-view` (a pointer so read is still only a target-free borrow). DISCHARGE: a "
                "live-member reading (a store of the member read, or a tag test, dominating the read with no "
                "intervening store through another member or call that may write the object), with Lean parity and "
                "a measured re-pin."
            ),
        },
        {
            "id": "CG13-UNION-LEAVES-OTHER-CAUSES",
            "title": (
                "Union-path leaves rule 10w reads past but another cause keeps refused: 891 union-only leaves "
                "(1,201 M06 leaves in all) over 405 Unknown functions"
            ),
            "why": (
                "Measured by CG13-UNION-AND-BITFIELD-PLACES on the corpus arbiter at its final tip with CG00's leaf "
                "attribution (scripts/population.py): M06 leaves 1,629 -> 1,201 over 467 -> 405 Unknown functions "
                "(428 union-only leaves read). The remaining leaves carry a second cause CG00's first-cause "
                "attribution hid behind `union`; scripts/residuals.py classes them by their source line (a reading "
                "aid, not a causal attribution): a call on the line 343 (calls in a return, call results stored "
                "into a union member: CG03/CG05), a CG00 pointer-valued store 229 (CG11), an address `&` 219 "
                "(`&rko->rko_u.x.y` actuals deeper than one member: rule 10n), a release of a pointer read through "
                "a union 131 (a borrow, never the owner: stays refused), an indirect call 35 (CG22), a cast 10 "
                "(CG08), other 234 (punned reads, `?:` values, aggregate copies, pointer returns on bound roots). "
                "DISCHARGE: each owning card measures its share on these leaves; this entry closes when none is "
                "left that a union step alone blocks."
            ),
        },
    ]


AMEND = {
    "W44-CHAIN-INCOMPLETE-RECORD": (
        " AMENDED (CG13-UNION-AND-BITFIELD-PLACES): the union part is absorbed. Rule 10w reads a union-member step "
        "in the Clang-typed reader and in the common typed place (scalar reads neutral, pointer reads a borrow "
        "unless punned, stores with the refused-write frame, pointer stores refused), so a typed position no longer "
        "refuses a chain for a union on it; the lexical reader's whole-record refusal is unchanged and still "
        "answers first. The non-union part (bit-field, function-pointer member, nested anonymous aggregate, two "
        "definitions) is not re-measured under this entry's own census (its registering script needs the census "
        "build); CG13's re-measurement of the union leaves is in CG13-UNION-LEAVES-OTHER-CAUSES."
    ),
    "W43-LIBC-GETPEERNAME-SOCKADDR-UNION": (
        " DECISION (CG13-UNION-AND-BITFIELD-PLACES, reported, not taken): the `__SOCKADDR_ARG` transparent union is a "
        "declared-ABI reader class (a parameter's typedef), not a place; rule 10w reads union-member places and "
        "does not change `legacy_declared_abi.rs`, so no libc contract changes and the provider gate is not due. "
        "Re-measured on the corpus arbiter at CG13's base and tip: 1 leaf (`call to getpeername whose declared "
        "contract was withdrawn ... (abi-unreadable-declaration)`) in `mk_socket_ip_str` (monkey mk_socket.c), "
        "Unknown with other blockers, unchanged. The discharge condition stands."
    ),
    "CG11-POINTER-VALUED-STORES": (
        " AMENDED by CG13-UNION-AND-BITFIELD-PLACES (rule 10w): a pointer-valued store through a union member is "
        "refused by rule 10w's table as well (no contract fixes the view of overlapping storage), and no route "
        "binds a pointer place or field link through a union member; that is what keeps a sibling store's "
        "invalidation sound. 229 of CG11's leaves are through a union (CG00 `path:pointer-valued-store,union`, "
        "unchanged by CG13); admitting one needs a view contract or a live-member reading, not only a store event."
    ),
    "CG18-FOOTPRINT-CONNECTION": (
        " AMENDED by CG13-UNION-AND-BITFIELD-PLACES (rule 10w): bit-field storage units are connected to "
        "`disjoint_footprint`. A bit-field's unit is read from its record's declaration order (C11 3.14) and a "
        "layout contract from Clang's own `-fdump-record-layouts-complete`; when the elaborator refuses an "
        "unordered pair of two bit-field places of one object, the core model names it `SharedBitfieldUnit` "
        "(Q661). A pair the model proves disjoint (two units) stays refused: admitting it, and any other field "
        "pair, is this card's."
    ),
}


def main():
    path = sys.argv[1]
    punned = json.load(open(sys.argv[2]))
    matrix = json.load(open(path))
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    retired = ("CG13-UNION-AND-BITFIELD-PLACES", "W46-UNION-MEMBER-PATH")
    for entry_id in retired:
        if entry_id not in ids:
            raise SystemExit(f"{entry_id} missing (already applied?)")
    followups[:] = [entry for entry in followups if entry["id"] not in retired]
    union_stores = punned["step_outcomes"].get("union pointer-store refused-pointer-store", 0)
    AMEND["CG11-UNREAD-POINTER-PLACES"] = (
        " AMENDED by CG13-UNION-AND-BITFIELD-PLACES (rule 10w): `typed_place::read_place` now reads a union-member "
        "and a bit-field step, so a pointer-typed store through a union member reaches rule 10v's value reading, "
        "and rule 10w's table then refuses it whatever its value (no contract fixes the view of overlapping "
        f"storage): {union_stores} such consultations on the corpus at CG13's merged tip (parity "
        "`position: union-bitfield`, step `union pointer-store refused-pointer-store`, deduplicated). The union "
        "part of this entry is therefore decided (refused by rule), not unread; the cast, array, call and "
        "volatile parts stand."
    )
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    found = {entry["id"] for entry in followups} & set(AMEND)
    if found != set(AMEND):
        raise SystemExit(f"missing amendment targets: {set(AMEND) - found}")
    for entry in register(punned):
        followups.append(dict(entry, source=REPORT, recorded_by=BY))
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(followups))


if __name__ == "__main__":
    main()
