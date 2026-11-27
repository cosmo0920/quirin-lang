#!/usr/bin/env python3
"""Card REFUSED-STORE-OF-CALL-RESULT-ESCAPE: record the token-level census.

Usage: token_census_record.py CENSUS_DIR OUT.json

CENSUS_DIR holds `cmt_map.i` (preprocess_unit.py), `explore.log` (the
harness's printed per-function diagnostics over it, census binary,
`QCORE_LEAK_WITNESS=CENSUS_DIR/explore-witness.jsonl`) and
`control-witness.jsonl` (the same harness over the positive control). The
witness stream is opened lazily on its first record, so an absent
`explore-witness.jsonl` means zero records.
"""

import hashlib
import json
import os
import re
import sys

REPORT_SOURCE_IDENTITY = "3416ae7a908fb4fa86334009324127bfe24fbadf549ca726715aa00bd162fcaf"
PATH_KEYS = ("path", "clean_path", "strict_path", "handed_path", "untested_path", "untied_path")


def main() -> int:
    census = sys.argv[1]
    functions = {}
    current = None
    for line in open(os.path.join(census, "explore.log"), encoding="utf-8").read().splitlines():
        match = re.match(r"== (\S+) (\S+)$", line)
        if match:
            current = match.group(1)
            functions[current] = {"status": match.group(2), "diagnostics": []}
            continue
        match = re.match(r"   (\d+) (.*)$", line)
        if match and current:
            functions[current]["diagnostics"].append(
                {"line": int(match.group(1)), "kind": match.group(2)}
            )
    source = open(os.path.join(census, "cmt_map.i"), "rb").read()
    witness = os.path.join(census, "explore-witness.jsonl")
    witness_lines = (
        len(open(witness, encoding="utf-8").read().splitlines()) if os.path.exists(witness) else 0
    )
    control = open(os.path.join(census, "control-witness.jsonl"), encoding="utf-8").read().splitlines()
    first = json.loads(control[0])
    for key in PATH_KEYS:
        if key in first:
            first[key] = "(elided)" if first[key] is not None else None
    leaks = {
        name: [d for d in record["diagnostics"] if d["kind"].startswith("Leak")]
        for name, record in functions.items()
        if any(d["kind"].startswith("Leak") for d in record["diagnostics"])
    }
    document = {
        "schema": "refused-store-census/1",
        "card": "REFUSED-STORE-OF-CALL-RESULT-ESCAPE",
        "what": (
            "Token-level leak-witness census of the unit's own preprocessed text: "
            "qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit over cmt_map.c "
            "-E -P with cycle19-resource-contracts.json, built at 82ec0e80b with "
            "--features indirect-census and QCORE_LEAK_WITNESS set."
        ),
        "base": "82ec0e80b",
        "unit": "lib/cmetrics/src/cmt_map.c (Fluent Bit 046accf6)",
        "preprocessed_sha256": hashlib.sha256(source).hexdigest(),
        "report_source_identity": REPORT_SOURCE_IDENTITY,
        "preprocessed_lines": source.count(b"\n"),
        "witness_lines": witness_lines,
        "functions_with_leaks": leaks,
        "cmt_map_metric_get": functions.get("cmt_map_metric_get"),
        "metric_prepare_storage": functions.get("metric_prepare_storage"),
        "positive_control": {
            "source": (
                "void *calloc(unsigned long n, unsigned long s); int w(int k) { char *p = "
                "calloc(1, 1); if (p == ((void*)0)) { return -1; } return 0; } "
                "(one token group per line, 8 lines)"
            ),
            "w": "Unsafe, Leak at exit line 7, acquisition line 3",
            "witness_lines": len(control),
            "first_witness": first,
        },
    }
    with open(sys.argv[2], "w", encoding="utf-8") as out:
        json.dump(document, out, indent=1)
        out.write("\n")
    print(json.dumps(document["cmt_map_metric_get"], indent=1))
    print(leaks)
    print("identity", document["preprocessed_sha256"] == REPORT_SOURCE_IDENTITY, "witness", witness_lines)
    return 0


if __name__ == "__main__":
    sys.exit(main())
