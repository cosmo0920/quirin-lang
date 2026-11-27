#!/usr/bin/env python3
"""sizes.py RUN [RUN...] -- CG23-RELATIONAL-POSTCONDITIONS: re-measure the
absorbed follow-ups' callees on each run (a run directory or a
resource-diagnostics.json file).

For each named callee: the leaves whose description names it in backticks,
the functions carrying one, and the Unknown functions every one of whose
non-note leaves names it (sole-blocked). Also each named function's own row
(status and leaf count), for the entries that name a function rather than a
callee.
"""

import collections
import json
import pathlib
import sys

CALLEES = {
    "CONDITIONAL-RELEASE-BY-LITERAL": ["rd_kafka_topic_partition_destroy0"],
    "W45-FB-BACKLOG-MAP-ARMS": ["flb_input_chunk_map"],
    "W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED": [
        "realpath",
        "pthread_getspecific",
        "flb_output_thread_instance_get",
    ],
    "W43-DESCRIPTOR-REFUSED-SHAPES": [
        "getaddrinfo",
        "freeaddrinfo",
        "fcntl",
        "accept4",
        "fts_open",
        "fts_close",
        "glob",
        "globfree",
        "pipe",
        "fdopen",
        "mmap",
        "munmap",
    ],
    "CJSON-DELETE-SIBLING-CHAIN": ["cJSON_Delete"],
}

ROWS = {
    "W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL": [
        "ut_sasl_oauthbearer_oidc_assertion_parse_from_file",
        "rd_kafka_oidc_assertion_read_from_file",
    ],
    "W45-FB-BACKLOG-MAP-ARMS": ["cb_queue_chunks", "flb_input_chunk_map"],
}


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def measure(data):
    out = {}
    for entry, callees in CALLEES.items():
        rows = {}
        for callee in callees:
            needle = f"`{callee}`"
            leaves = 0
            functions = set()
            sole = set()
            for unit in data["units"]:
                for function in unit.get("functions") or []:
                    diagnostics = [
                        d for d in function.get("diagnostics", []) if d.get("category") != "note"
                    ]
                    hits = [d for d in diagnostics if needle in (d.get("description") or "")]
                    if not hits:
                        continue
                    key = f"{pathlib.Path(unit['file']).name}::{function['function']}"
                    leaves += len(hits)
                    functions.add(key)
                    if function.get("status") == "Unknown" and len(hits) == len(diagnostics):
                        sole.add(key)
            rows[callee] = {"leaves": leaves, "functions": len(functions), "sole_blocked": len(sole)}
        out[entry] = rows
    named = collections.defaultdict(dict)
    for entry, names in ROWS.items():
        for unit in data["units"]:
            for function in unit.get("functions") or []:
                if function["function"] in names:
                    diagnostics = [
                        d for d in function.get("diagnostics", []) if d.get("category") != "note"
                    ]
                    named[entry][function["function"]] = {
                        "status": function.get("status"),
                        "leaves": collections.Counter(
                            d.get("category") for d in diagnostics
                        ),
                    }
    out["rows"] = named
    return out


def main():
    print(json.dumps({run: measure(load(run)) for run in sys.argv[1:]}, indent=1))


if __name__ == "__main__":
    main()
