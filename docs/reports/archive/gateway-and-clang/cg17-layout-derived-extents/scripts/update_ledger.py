#!/usr/bin/env python3
"""CG17-LAYOUT-DERIVED-EXTENTS: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG17-LAYOUT-DERIVED-EXTENTS (every DISCHARGE clause holds: the cited
refusals reproduced at the base, PREDICTION.md frozen, AFTER measured on all
three routes with nothing moving, the absorbed follow-up re-registered with
its re-measured state, Rust refusal controls and audited Lean
`QCore.LayoutExtents`), amends W45-FB-OR6-ELEMENT-IDENTITY with the
re-measured state, registers the card's residuals, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes: the AFTER corpus arbiter's parity census at the card's tip over main
`b6a3dc42` (private/cg17le/after-parity.jsonl, scripts/census.py, de-duplicated
by function, line and fields) and the AFTER system route.
"""

import json
import sys

REPORT = "docs/reports/cg17-layout-derived-extents/README.md"
BY = "CG17-LAYOUT-DERIVED-EXTENTS"

RETIRE = ["CG17-LAYOUT-DERIVED-EXTENTS"]

AMEND = {
    "W45-FB-OR6-ELEMENT-IDENTITY": (
        " AMENDED by CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u, base b6a3dc42): container_of now has a "
        "base/subobject provenance rule, but it names the enclosing object only for a pointer established as "
        "`&x->m` of the same body; OR-6's `queued_task = mk_list_entry_first(&queue->pending, ...)` (and "
        "flush_next's `ended_task`) is a list lookup over `(H)->next`, refused by that rule as "
        "`not-a-member-address` by design: its identity is rule 10f's container premise (P-CRR-MEMBER), "
        "unchanged. Re-measured on the system route at b6a3dc42 (unchanged by the card): "
        "flb_output_task_queue_flush_one Unknown (counted-user operation on `queued_task->task`, "
        "flb_output_task_flush's refuted contract, unresolved `queued_task`, an indirect access, the "
        "uncontracted flush_next), flb_output_task_singleplex_flush_next Unknown (the uncontracted "
        "flush_one: the pair is mutually recursive). Size unchanged: 1 obligation, 2 functions. The "
        "blocker stays W45-FOREIGN-UNLINK-SUMMARY (a summary stating the released element)."
    ),
}

REGISTER = [
    {
        "id": "CG17-FLEXIBLE-EXTENT-UNESTABLISHED",
        "title": "Flexible-array extents never established: 16 corpus accesses over 12 functions read size-not-established",
        "why": (
            "Registered by CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u). layout_extents::flexible_extent derives "
            "(size - offsetof(member)) / sizeof(element) from Clang's offset and width, but only for an "
            "established allocation size, and no current contract establishes one: on the corpus arbiter "
            "(66910c10, the card's tip over main b6a3dc42) all 16 de-duplicated flexible-array accesses (12 "
            "functions) are `size-not-established`; their lines carry 11 BEFORE leaves (6 indirect access, 3 "
            "increment, 1 return, 1 unknown_alias). DISCHARGE: CG16's reviewed extent vocabulary (C09.1) "
            "establishing an allocation's byte size at its acquisition (malloc-family size actuals), consumed by "
            "flexible_extent, with the bounds obligation checked per access and Lean parity; re-measure."
        ),
    },
    {
        "id": "CG17-ALIGNMENT-UNDECIDED",
        "title": "Alignment obligations of derived addresses undecided: 400 of 441 corpus conversions (232 functions); layout.alignment.v1 has no quirin-audit consumer",
        "why": (
            "Registered by CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u). Every BitCast of a derived address "
            "(pointer arithmetic or &x) to a non-character referent carries C11 6.3.2.3p7's obligation, decided "
            "from Clang's _Alignof/sizeof. On the corpus arbiter (66910c10, tip over b6a3dc42): 441 de-duplicated "
            "conversions over 232 functions, 41 aligned, 0 misaligned, 400 undecided (a char/void base or a base "
            "the reader does not name: the container_of subtraction's `(char *)&x->m` or `(char *)__mptr`, a "
            "`(char *)buf + off` cursor, an index whose element width is not a multiple). The property is "
            "published only by layout_extents::layout_report (library), not selected by any quirin-audit route. "
            "DISCHARGE: base alignments for allocation results (an allocator alignment contract, max_align_t), "
            "for member addresses (Clang's field offset and record alignment), and a quirin-audit consumer that "
            "selects layout.alignment.v1 with its assumptions, each with Lean parity and a measured re-pin."
        ),
    },
    {
        "id": "CG17-CONTAINER-OF-LIST-LOOKUPS",
        "title": "container_of over a list lookup has no member provenance: 355 corpus shapes over 189 functions",
        "why": (
            "Registered by CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u). container_of is the enclosing object "
            "only when its pointer is established as `&x->m` of the same body; on the corpus arbiter (66910c10, "
            "tip over b6a3dc42) every one of the 355 de-duplicated shapes (189 functions; 380 offsetof expressions "
            "over 192 functions) is `unestablished: not-a-member-address` (mk_list_entry/cfl_list_entry over a "
            "cursor or `(H)->next`), and none reaches the hook (they stand inside statement expressions whose "
            "inner statements the older routes refuse first: 46 indirect-access, 38 `__builtin_offsetof`, 19 "
            "braced leaves on their lines at BEFORE). Their element identity is rule 5l/10f's container premise, "
            "not member provenance. DISCHARGE: a rule connecting rule 10f's foreign-unlink element (P-CRR-MEMBER) "
            "to the enclosing object's identity where the list's element type and link member are Clang's, "
            "coordinated with W45-FB-OR6-ELEMENT-IDENTITY, with Lean parity and a measured re-pin; or accepted "
            "as a measured boundary."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    for amended in AMEND:
        if amended not in ids:
            raise SystemExit(f"{amended} is not registered")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {len(ids)} -> {len(followups)}")


if __name__ == "__main__":
    main()
