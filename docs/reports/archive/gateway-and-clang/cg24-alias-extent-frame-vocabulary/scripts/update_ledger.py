#!/usr/bin/env python3
"""CG24-ALIAS-EXTENT-FRAME-VOCABULARY: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG24-ALIAS-EXTENT-FRAME-VOCABULARY: every DISCHARGE clause holds (the
BEFORE reproduced CG00's K04/K06 controls, PREDICTION.md was frozen before any
implementation code, the AFTER moved no function on any route, the absorbed
entries are re-registered with re-measured sizes, and the frame and relation
decisions have Rust controls and audited Lean, QCore.IndexExtentFrames); the
scope it did not reach is registered as two residuals with sizes. Amends the
two absorbed follow-ups, whose DISCHARGE clauses are custody vocabulary this
card did not add. Keeps `dimensions.named_followups` equal to the list length
and writes the matrix in its own form (indent=1, ensure_ascii=False, newline).

Sizes: the BEFORE runs (private/cg24/before*, census-before-*.json by
scripts/census.py), equal to the AFTER runs (byte-identical custody and bounds
artifacts on all three routes).
"""

import json
import sys

REPORT = "docs/reports/cg24-alias-extent-frame-vocabulary/README.md"
BY = "CG24-ALIAS-EXTENT-FRAME-VOCABULARY"

RETIRE = ["CG24-ALIAS-EXTENT-FRAME-VOCABULARY"]

AMEND = {
    "W46-SDS-CAT-SAFE-IN-PLACE-REALLOC": (
        " AMENDED (CG24-ALIAS-EXTENT-FRAME-VOCABULARY, wave 48 round 11, base 6cab1ad9, gateway rule 11e): the "
        "bounds half landed: bounds contract v1 states the in-place reallocation through a slot "
        "(`buf: ptr<mut, ptr<mut, u8>>`, `ensures normal bounds_frame_moves(buf)`), which withdraws the facts of "
        "the slot's local and of every alias of the old base (tests/cg24_frames_and_relations.rs, the sds "
        "control). The custody half is unchanged: resource v1 has no reallocation-through-an-out-slot item, and "
        "the call stays uncontracted for custody. Re-measured (corpus 66910c10, custody byte-identical "
        "ceefb6c4 before and after): 170 leaves in 36 non-Safe functions, 3 as their only obstacle "
        "(format_metric_type, append_bytes_value, sds_cat_safe); 0 on the system route and the tail-pause slice. "
        "The custody DISCHARGE stands."
    ),
    "W45-FB-BACKLOG-REFERENCE-VOCABULARY": (
        " AMENDED (CG24-ALIAS-EXTENT-FRAME-VOCABULARY, wave 48 round 11, gateway rule 11e): the card's derived "
        "relations are bounds relations of a result to its argument bases (bounds_return_derived, multi-base), "
        "not the custody relation this entry needs (wrapper refers to chunk; close requires no outstanding "
        "reference), and it adds no return-conditioned clause or per-element identity. Re-measured on base "
        "6cab1ad9: the storage-backlog declarations' leaves sit in 7 non-Safe system-route functions (17 leaves: "
        "the five sb.c bodies whose declared specification is not established, flb_engine_start and "
        "flb_input_chunk_release_space_compound), none of them as the only obstacle; 1 corpus leaf, not sole; 0 "
        "on the tail-pause slice. The reach of a relation clause alone stays 0. The DISCHARGE stands."
    ),
}

REGISTER = [
    {
        "id": "CG24-CALLBACK-SPANNING-RELATIONS",
        "title": "Borrow and extent relations that span a callback (a view valid while a callback runs) are not stated",
        "why": (
            "Registered by CG24-ALIAS-EXTENT-FRAME-VOCABULARY (rule 11e). Entry K06's callback-spanning relations "
            "need the indirect callees and callback relations of CG22-INDIRECT-CALLS-AND-CALLBACKS: a frame or "
            "relation is applied only at a direct call to a named callee (an indirect call is CG16's unstated "
            "call). Size: 0 functions on the corpus, system and slice routes, which run the bounds property with "
            "no bounds declaration (CG16-BOUNDS-CONTRACT-TRANSPORT). DISCHARGE: a relation clause scoped to a "
            "callback's invocations, applied at each candidate callee CG22 resolves, with Rust/Lean parity and "
            "refusal controls."
        ),
    },
    {
        "id": "CG24-FRAME-AND-SLOT-PRECISION",
        "title": "Bounds frames are declared and refuted, never summarised; loops ignore read frames; a reallocated slot gets no extent",
        "why": (
            "Registered by CG24-ALIAS-EXTENT-FRAME-VOCABULARY (rule 11e). (1) An analysed body only refutes a "
            "declared frame or relation; no frame is derived from a body, so an in-unit callee with no bounds "
            "declaration stays an unstated call. (2) A loop or switch containing any call withdraws every fact on "
            "an object the source did not establish at entry (CG16's rule), whatever the frames. (3) A held frame "
            "tracks no local (Unknown, never Pending). (4) After bounds_frame_moves on a slot, the slot's new "
            "object has no extent (cfl_sds_cat_safe's grows by an argument-dependent length). (5) A closed frame's "
            "'touches nothing else' is a Declared premise: writes to globals are not checked against it. Size: 0 "
            "functions on the audit routes (no bounds declaration is selected there); every limit is a precision "
            "limit, never a wrong Safe. DISCHARGE: validated bounds frame summaries of in-unit bodies, frame-aware "
            "loop entry, a slot extent clause, and a closure check, each with controls and Lean parity, then a "
            "measured re-pin once declarations are transported."
        ),
    },
]


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        matrix = json.load(handle)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    before = len(followups)
    for retired in RETIRE:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    for name, text in AMEND.items():
        entry = next((e for e in followups if e["id"] == name), None)
        if entry is None:
            raise SystemExit(f"{name} is not registered")
        if f"AMENDED ({BY}" in entry["why"]:
            raise SystemExit(f"{name} already amended")
        entry["why"] += text
    for entry in REGISTER:
        if any(existing["id"] == entry["id"] for existing in followups):
            raise SystemExit(f"{entry['id']} already registered")
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(f"named_followups: {before} -> {len(followups)}")


if __name__ == "__main__":
    main()
