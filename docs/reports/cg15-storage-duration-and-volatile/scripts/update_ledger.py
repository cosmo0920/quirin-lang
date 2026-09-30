#!/usr/bin/env python3
"""CG15-STORAGE-DURATION-AND-VOLATILE: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG15-STORAGE-DURATION-AND-VOLATILE (every DISCHARGE clause holds) and
the two follow-ups it absorbs, GATEWAY-VOLATILE-PREMISE and
CT41-VOLATILE-COMPLEMENT-UNMEASURED (each discharge met: README sections 7
and 8); registers the two residual classes with their sizes; keeps
`dimensions.named_followups` equal to the list length (it was 658 against 659
entries on the base). Writes the matrix back in its own form (indent=1,
ensure_ascii=False, trailing newline).

Sizes are the card's AFTER2 corpus run (private/cg15/after2, corpus arbiter
--jobs 4, base c374de96), read with scripts/population.py and
scripts/leaves_of.py.
"""

import json
import sys

REPORT = "docs/reports/cg15-storage-duration-and-volatile/README.md"
BY = "CG15-STORAGE-DURATION-AND-VOLATILE"

RETIRE = [
    "CG15-STORAGE-DURATION-AND-VOLATILE",
    "GATEWAY-VOLATILE-PREMISE",
    "CT41-VOLATILE-COMPLEMENT-UNMEASURED",
]

REGISTER = [
    {
        "id": "CG15-STATIC-TABLE-RESIDUALS",
        "title": "Reads and writes of static-duration tables and buffers the typed routes still refuse: 52 corpus leaves on 17 Unknown functions",
        "why": (
            "Measured by CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h) on its AFTER corpus run (base c374de96, "
            "scripts/population.py with CG00's attribution): of M24's 132 leaves on 67 Unknown functions, 52 on 17 remain, "
            "all path leaves (every static declaration is now no event). By reading: 29 read an element of an untagged "
            "static table (`static const struct { ... } t[]`), which the typed place refuses as an untagged aggregate, "
            "mostly in loop conditions with increments (unit tests of rdbuf, rdstring, rdkafka_msg, "
            "rd_kafka_get_legacy_ApiVersions); 3 write through or step a static formatter buffer (`*ret[reti] = ...`, "
            "`ret[reti] + of`, a memcpy destination; rd_kafka_features2str), pointer arithmetic being CG16's; 18 sit in "
            "debug-macro actuals, sub-object stores and nested calls whose other refusal the static root hid. Two "
            "functions are freed by this class together with CG07. Route T refuses every static-duration local and "
            "global reference by name: no typed-source consumer decides a lifetime longer than the frame. DISCHARGE: "
            "untagged-aggregate element reads in the typed place (or a tag the reader synthesises from Clang's "
            "RecordDecl id), and static-buffer stores and offsets with CG16's extents, each measured with Lean parity; "
            "a T consumer that decides static lifetime, if one is wanted."
        ),
    },
    {
        "id": "CG15-STATIC-REGISTRY-CONTRACT",
        "title": "A tracked identity stored into static storage is refused, never discharged: no contract states a registry with a release protocol (1 corpus leaf)",
        "why": (
            "Recorded by CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h, rule 10 unchanged): a pointer stored into "
            "a global, a file-scope static or a function-local static escapes the frame's custody to no named party, so "
            "it is refused (never Safe, never a per-call leak); a function-local static holding an allocation across "
            "calls is this form. The contract model (crates/qcore-schemas/src/contract.rs, Q29) has no registry, static "
            "owner or release-protocol vocabulary, so nothing can decide it. Measured on the card's AFTER corpus run: "
            "1 leaf, rdunittest_fetcher.c::unittest_fetcher_share_filter_forward storing a created handle into the "
            "file-scope `ut_mock_rk`. DISCHARGE: a product decision on a registry contract form (which static object, "
            "which release protocol, where the obligation is discharged), its checker and Lean model, and a measured "
            "re-pin; or a recorded decision that static publication stays refused."
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
