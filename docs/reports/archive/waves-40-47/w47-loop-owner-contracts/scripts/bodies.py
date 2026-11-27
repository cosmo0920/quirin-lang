#!/usr/bin/env python3
"""bodies.py: SHA-256 (first 16 hex) of each loop-owner operation's definition
text in src/flb_input.c at the three audited source roots, and whether they
agree. A column-0 definition line through its column-0 closing brace."""
import hashlib
import json
import re
import sys

ROOTS = {
    "66910c10": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source",
    "046accf6": "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
    "90978ac6": "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source",
}
FUNCTIONS = ["flb_input_collector_pause", "flb_input_collector_resume",
             "flb_input_collector_running", "flb_input_collector_delete",
             "get_collector", "flb_input_pause", "flb_input_resume",
             "flb_input_plugin_pause", "flb_input_plugin_resume"]


def body(path, name):
    lines = open(path, errors="replace").read().splitlines()
    for index, line in enumerate(lines):
        if re.match(r"^[A-Za-z].*\b" + name + r"\s*\(", line) and not line.rstrip().endswith(";"):
            end = index
            while not lines[end].startswith("}"):
                end += 1
            return "\n".join(lines[index:end + 1]), index + 1
    return None, None


def main():
    out = {}
    for name in FUNCTIONS:
        row = {}
        for revision, root in ROOTS.items():
            text, line = body(root + "/src/flb_input.c", name)
            row[revision] = None if text is None else {
                "line": line, "sha256": hashlib.sha256(text.encode()).hexdigest()[:16]}
        row["identical"] = len({v["sha256"] for v in row.values() if v}) == 1
        out[name] = row
    json.dump(out, sys.stdout, indent=1)
    print()


if __name__ == "__main__":
    main()
