#!/usr/bin/env python3
"""VERDICT-CONTRACT-PREMISE-ANNOTATIONS: retire the card's entry and register
the residual with its size. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "VERDICT-CONTRACT-PREMISE-ANNOTATIONS"
SOURCE = "docs/reports/verdict-contract-premise-annotations/README.md"
RETIRE = {CARD}

NEW = [
    {
        "id": "VERDICT-PREMISES-NON-RESOURCE-CONTRACTS",
        "title": "Caller premises of the non-resource-v1 contract modules are stated only in comments",
        "why": "Registered by VERDICT-CONTRACT-PREMISE-ANNOTATIONS, whose sweep of every tracked .qv under providers/ and contract/ (215 comment lines naming a premise or an assumption, 71 modules; docs/reports/verdict-contract-premise-annotations/sweep.json) found 15 lines in 14 modules that are not resource-v1 and so cannot carry C04.18's requires resource_premise: chunkio counters, error_frame, error_state, file_close, file_close_delete, filesystem_delete, filesystem_outcomes, list_unlink, memory_backend, memory_close, parent_teardown, teardown_traversal (qcore-provider-chunkio) and monkey composition, event_payload (qcore-provider-libevent). Each states caller premises of a lifecycle, source-frame or composition contract (live exclusive record provenance, cross-unit ABI identity, framed external-call effects, storage extent and write authority); C06.5's dependency record covers resource-v1 declarations only, so no published row names them. Size: 15 comment lines, 14 modules, two crates. DISCHARGE: a machine-readable premise form in those contract families (or a decision that their routes' own reports name them), recorded in the contract specification, with the premises attached and measured with no verdict movement.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
