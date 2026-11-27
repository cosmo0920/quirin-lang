#!/usr/bin/env python3
"""CHUNKIO-META-LEN-BOUNDS-CONTRACTS: amend CG16-BOUNDS-CONTRACT-TRANSPORT and
register CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (CHUNKIO-META-LEN-BOUNDS-CONTRACTS"

AMEND = {
    "CG16-BOUNDS-CONTRACT-TRANSPORT": (
        " AMENDED (CHUNKIO-META-LEN-BOUNDS-CONTRACTS, wave 48 round 19, base e2031f65a, gateway "
        "rule 12r): landed (a) placement and packaging: each load-path crate record carries a "
        "generated `bounds_view` (the owned modules the bounds loader accepts alone, all loading "
        "together; computed by probe_resource_views, written and checked by providers/contracts.py; "
        "the Rust reader refuses a module also in the resource view), and packages carry bounds "
        "modules through the unchanged inventory (no route reads contract modules from a package); "
        "(b) C06.2 pins: a crate's bounds-contracts.lock in C06.2's lock format keyed by the "
        "declaration's text digest (the quant clause's text included), written by `quirin-audit pin "
        "--bounds`, verified by `pin --verify-contracts`, decided at audit time by pin_standing, a "
        "mismatch holding the declaration (`contract-pin-mismatch`, Pending at most, never accusing); "
        "controls (tests/bounds_pin_hold.rs): no lock, a stale pin (changed body, source-changed), a "
        "changed clause (contract-changed, and a stale lock under --verify-contracts), a missing "
        "entry (not-in-lock), and a pin in force; (c) selection: with QCORE_BOUNDS_PROPERTY the "
        "pinned, configured and contracts routes load every bounds view on the load path their "
        "resource manifest names (record ROWS.contracts.json). First declarations: chunkio's "
        "cio_file_st_get_meta_len and cio_file_st_set_meta_len (providers/qcore-provider-chunkio/"
        "contracts/chunkio/meta_len_bounds.qv), pinned in force at 66910c10 and 046accf6; the "
        "bounds-selected corpus moved no verdict (162 Safe / 4,759 Unknown; 9 call obligations in 9 "
        "chunkio functions now name the quant check, still bounds-missing-extent on cf->map). "
        "Remaining (Size: 1 module, 2 declarations): `.qo` checkpoint transport of bounds rows and "
        "the environment; G05a inclusion and inheritance for bounds modules (an inherited marker "
        "control); the cross-unit refutation phase on the routes (the API takes the table); tip "
        "tracking for bounds pins (refused by pin --bounds); and checking a header-defined "
        "declaration's body on the routes: static inline header definitions have no row "
        "(MainSourceDefinitions), so the two chunkio bodies are checked only through the API "
        "(crates/qcore-c-gateway/tests/chunkio_meta_len_bounds.rs). DISCHARGE now: those five."
    ),
}

NEW = {
    "id": "CHUNKIO-FILE-ST-NEIGHBOUR-BOUNDS",
    "title": "The other six cio_file_st_* header accessors have no bounds contract; each needs a vocabulary step first or a reading",
    "why": (
        "Registered by CHUNKIO-META-LEN-BOUNDS-CONTRACTS (gateway rule 12r), which wrote the bounds "
        "contracts of cio_file_st_get_meta_len and cio_file_st_set_meta_len and, by the brief, not "
        "their neighbours in lib/chunkio/include/chunkio/cio_file_st.h (byte-identical at 66910c10 "
        "and 046accf6). What each needs: cio_file_st_get_hash(map) (l.70, `map + 2`): a derived "
        "result bounds_return_derived(map, 2, 4, bytes) and a read frame, writable today; "
        "cio_file_st_get_meta(map) (l.89, `map + 24`): a derived result whose length is the "
        "metadata length read from map[22..23], a symbolic length tied to a value read from the "
        "object (CG16-BOUNDS-SYMBOLIC-RANGES, relational postconditions); cio_file_st_get_content(map) "
        "(l.95, `map + 24 + get_meta_len(map)`): a symbolic offset from header content and a size "
        "relation, not expressible; cio_file_st_infer_content_len(map, size) (l.104, scalar "
        "arithmetic and one get_meta_len(map)): the same quant check `22 <= #i && #i < 24` (an "
        "escape rule 12l(4) decides inside), writable today; cio_file_st_set_content_len(map, len) "
        "(l.116, writes map[10..13] through `(uint8_t *) &map[10]`): the check `10 <= #i && #i < 14` "
        "read through a byte-view-cast local (rule 12h) in the quant body check, which follows only "
        "direct, copied and P + k accesses; cio_file_st_get_content_len(map, size, page_size, "
        "tainted) (l.129, reads map[10..13] the same way and map[24 + meta_len] under `size > "
        "content_offset`): bounds_extent(map, size, bytes), the cast-view reading and a symbolic "
        "index. Size: 6 declarations; payoff on the corpus needs CG16-BOUNDS-POINTER-EXTENTS (every "
        "corpus caller passes cf->map). DISCHARGE: each accessor read against both pinned bodies and "
        "either declared (bounds contract v1, pinned with pin --bounds at both revisions) with its "
        "measured payoff, or its missing vocabulary named on the owning entry."
    ),
    "source": "docs/reports/chunkio-meta-len-bounds-contracts/README.md",
    "recorded_by": "CHUNKIO-META-LEN-BOUNDS-CONTRACTS",
}


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    seen = 0
    for entry in followups:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        seen += 1
        if MARK not in entry["why"]:
            entry["why"] += text
    assert seen == len(AMEND), seen
    if not any(entry["id"] == NEW["id"] for entry in followups):
        followups.append(NEW)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"amended {seen}; follow-ups {len(followups)}")


if __name__ == "__main__":
    main()
