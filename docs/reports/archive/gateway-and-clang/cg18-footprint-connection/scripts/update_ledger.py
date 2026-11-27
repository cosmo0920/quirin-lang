#!/usr/bin/env python3
"""CG18-FOOTPRINT-CONNECTION: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG18-FOOTPRINT-CONNECTION (every DISCHARGE clause holds) and the
absorbed W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE (its DISCHARGE's
second arm, recorded in gateway rule 11k (4)); registers the card's residuals
by owner with their sizes; keeps `dimensions.named_followups` equal to the
list length. Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's runs (private/cg18fc/{before,after}*, corpus arbiter
--jobs 4, system and tail-pause slice), read from the parity streams'
`footprint-member-pair` rows and the elaboration rows' `new_refusal`.
"""

import json
import sys

REPORT = "docs/reports/cg18-footprint-connection/README.md"
BY = "CG18-FOOTPRINT-CONNECTION"

RETIRE = [
    "CG18-FOOTPRINT-CONNECTION",
    "W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE",
]

REGISTER = [
    {
        "id": "CG18-CROSS-OBJECT-PATH-PAIRS",
        "title": "An unsequenced pair of members of two different objects stays refused: no route takes P-DISTINCT-ALLOCATIONS-DISJOINT",
        "why": (
            "Measured by CG18-FOOTPRINT-CONNECTION (gateway rule 11k) on its AFTER corpus run: 5 corpus functions keep "
            "the elaborator's `not proved disjoint` refusal on a pair whose two bases are different pointers "
            "(rd_kafka_send `rkb->rkb_corrid`/`rkbuf->rkbuf_corrid`, rd_kafka_toppar_enq_msg, "
            "rd_kafka_mock_sharegroup_target_assignment_set, mk_sched_register_thread, decode_static_field); system and "
            "tail-pause slice 0. The core model decides two allocation identities disjoint only when both live "
            "generations were declared fresh, and names P-DISTINCT-ALLOCATIONS-DISJOINT on its certificate; the custody "
            "gateway publishes no per-verdict premise list, so it takes no such premise and two formals may alias. "
            "DISCHARGE: either a per-verdict assumption record that can carry the premise, with a route that proves both "
            "bases are distinct fresh acquisitions (tracked identities of two allocator results, both live), or an alias "
            "argument for the pair; each with Lean parity and a measured re-pin."
        ),
    },
    {
        "id": "CG18-FOOTPRINT-CONSUMERS",
        "title": "The core footprint premises have no gateway consumer: CG12's type view, a byte-safety property, C mutex evidence, flow generations",
        "why": (
            "Registered by CG18-FOOTPRINT-CONNECTION (gateway rule 11k). The card added the core events and premises "
            "(DeclareFreshAllocation / P-DISTINCT-ALLOCATIONS-DISJOINT, DeriveView / P-EFFECTIVE-TYPE with punned views "
            "refused, ByteCopy/ByteFill / P-BYTE-REPRESENTATION with an overlapping memcpy refused) and the "
            "mutex-invariant composition (qcore_core::footprint_mutex, reached from source.mutex.guard.v1), and connected "
            "rule 11a's generations by a checked replay. Four consumers do not exist yet, each a refusal today: (1) M22: "
            "CG12-VOID-VIEW-READBACK's type-view check had not landed; when it does, a post-cast view should be a "
            "DeriveView (EffectiveType carrying its premise, Punned refused) rather than its own rule; (2) M26: no route "
            "claims byte-level safety, so memcpy/memmove/memset keep their custody contracts and the byte events have no "
            "source caller; (3) W08: no C route produces a mutex-invariant trace, so a C field update claims no lock "
            "invariant; (4) M20: the custody flow publishes no footprint, so rule 11a's identities reach the core only in "
            "the replay test. DISCHARGE per item: a consumer that emits the core event from source, with its premise in "
            "the published assumptions, Lean parity and a measured re-pin; or a recorded decision that the property is "
            "not claimed."
        ),
    },
]


def main() -> int:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for name in RETIRE:
        assert name in ids, f"{name} is not registered"
    for entry in REGISTER:
        assert entry["id"] not in ids, f"{entry['id']} already registered"
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in REGISTER:
        entries.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
