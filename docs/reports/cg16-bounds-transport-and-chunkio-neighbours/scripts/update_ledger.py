#!/usr/bin/env python3
"""CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the ledger change (run once
from the repository root). Amends CG16-BOUNDS-CONTRACT-TRANSPORT,
CG16-BOUNDS-SYMBOLIC-RANGES and CG16-BOUNDS-POINTER-EXTENTS, retires
CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS and registers BOUNDS-BODY-EVIDENCE-UNCOVERED-CLAUSES."""
import json
import pathlib
import sys

PATH = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
CARD = "CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS"
REPORT = "docs/reports/cg16-bounds-transport-and-chunkio-neighbours/README.md"

TRANSPORT = (
    " AMENDED (" + CARD + ", wave 48 round 21, base 84ec54d1d, gateway rules 14f-14j): three of the five landed."
    " (1) Header bodies: phase one's check of every declared body a unit defines (static inline header definitions"
    " included) is published as evidence for the declaration in the bounds selection record (`body_evidence`, rules"
    " 13g-13j's fold per clause, a bounds Safe roll-up), never as a row (C09.1 keeps header definitions outside the"
    " routes' denominator); corpus: {evidence}. (2) Cross-unit refutation: the routes' second phase runs when the"
    " custody or the bounds table is not empty and withdraws a refuted bounds declaration from every unit's callers"
    " (control: a caller in another unit moves Safe -> Unknown); corpus table empty. (3) Tips: `pin --bounds"
    " --track-tip/--untrack-tip`, a tip in force a named assumption (`tip-tracked:`) with its notice, otherwise held."
    " Not landed, each now refused rather than dropped (rule 14j): `.qo` checkpoint transport (a checkpointed run"
    " refuses QCORE_BOUNDS_PROPERTY: no section carries bounds rows, body evidence or the environment, and the unit"
    " identity names neither the environment digest nor the bounds refutation table) and G05a inclusion and"
    " inheritance (the bounds loader refuses both forms; the probe leaves such a module out of every view; a bounds"
    " agreement gate over renamed quant predicates and counts and an evidence replay are missing). Size: 0 bounds"
    " modules use either; 0 checkpointed bounds runs. DISCHARGE now: those two, each with its parity and negative"
    " controls (a stale checkpoint under a changed environment or table; an inherited review marker; a changed parent"
    " taking the child out of its pin)."
)

SYMBOLIC = (
    " AMENDED (" + CARD + ", wave 48 round 21, gateway rule 14g): chunkio's `cio_file_st_get_meta` (`map + 24`, a"
    " length read from map[22..23]), `cio_file_st_get_content` (`map + 24 + get_meta_len(map)`, an offset read from"
    " the object) and `cio_file_st_get_content_len` (`bounds_extent(map, size, bytes)`, the byte view, and"
    " `map[24 + meta_len]` under `size > content_offset`) need a length or offset read from fixed bytes of the"
    " object. Not closed narrowly: the consumer's facts live on locals and parameters, never on object contents, so"
    " such a value would need content facts invalidated by every write through an alias or call; and every corpus"
    " caller passes `cf->map`, which has no extent to compare it with (CG16-BOUNDS-POINTER-EXTENTS). Size: 3"
    " accessors, 0 corpus payoff today. DISCHARGE additionally: a content-read length term (e.g. the big-endian"
    " 16-bit value at a constant offset of a covered extent) with content facts and their invalidation, Lean parity."
)

POINTER = (
    " AMENDED (" + CARD + ", wave 48 round 21): five chunkio header accessors are now declared and pinned in force;"
    " {calls} corpus calls to them pass `cf->map` (a member with no extent) and stay `bounds-missing-extent` Unknown."
)

UNCOVERED = {
    "id": "BOUNDS-BODY-EVIDENCE-UNCOVERED-CLAUSES",
    "title": "Bounds declaration clauses no body evidence covers: derived relations' length, return extents, preserves, global extents, and frames the syntactic walk cannot confirm",
    "why": (
        "Registered by " + CARD + " (gateway rules 14f-14g). Rule 14f's evidence covers a quant check or a"
        " precondition by a Safe, unrefuted body and a frame by rule 14g's positive frame check; nothing covers a"
        " `bounds_return_derived` (its LEN units are a fact about the caller's object), a `bounds_return_extent`,"
        " `bounds_preserves_extents()` or a `bounds_global_extent`, and the frame walk follows no copy, byte view or"
        " derived pointer. Corpus: {uncovered}. DISCHARGE: a body check that establishes each kind (a returned"
        " pointer's extent from a covered base, a preserved-extents check over the body's calls, a frame walk over"
        " tracked copies and byte views) with Lean parity, re-measured."
    ),
    "source": REPORT,
    "recorded_by": CARD,
}


def main():
    values = json.loads(sys.argv[1])
    matrix = json.loads(PATH.read_text())
    entries = matrix["named_followups"]
    by_id = {e["id"]: e for e in entries}
    by_id["CG16-BOUNDS-CONTRACT-TRANSPORT"]["why"] += TRANSPORT.format(**values)
    by_id["CG16-BOUNDS-SYMBOLIC-RANGES"]["why"] += SYMBOLIC
    by_id["CG16-BOUNDS-POINTER-EXTENTS"]["why"] += POINTER.format(**values)
    retired = "CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS"
    assert retired in by_id
    entries = [e for e in entries if e["id"] != retired]
    new = dict(UNCOVERED)
    new["why"] = new["why"].format(**values)
    assert new["id"] not in by_id
    entries.append(new)
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    PATH.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(by_id), "->", len(entries))


main()
