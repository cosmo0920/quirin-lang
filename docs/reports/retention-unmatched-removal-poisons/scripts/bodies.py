#!/usr/bin/env python3
"""RETENTION-UNMATCHED-REMOVAL-POISONS (from FB-EVENT-LOOP-RETENTION-AND-REGISTRATION):
read the declared loop create/destroy bodies, and the removal whose status reset
is the defect, at the audited revisions and report, per function, its line and body sha256 at each.
Read-only over the prepared exports. Output: artifacts/bodies.json."""
import hashlib
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
ROOTS = {
    "66910c10": "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/clang/66910c10a4d7eafa810b84229cf2dcf7b0f26f97/fluent-bit-clang-source",
    "046accf6": "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
    "90978ac6": "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source",
}
FUNCTIONS = {
    "lib/monkey/mk_core/mk_event.c": ["mk_event_loop_create", "mk_event_loop_destroy", "mk_event_del"],
    "lib/monkey/mk_core/mk_event_epoll.c": ["_mk_event_loop_create", "_mk_event_loop_destroy", "_mk_event_del"],
}


def bodies(text, name):
    out = []
    for m in re.finditer(r"^[^\s#].*\b%s\s*\(" % re.escape(name), text, re.M):
        start = m.start()
        brace = text.find("{", m.end())
        semi = text.find(";", m.end())
        if brace < 0 or (0 <= semi < brace):
            continue
        depth = 0
        for i in range(brace, len(text)):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    out.append((text.count("\n", 0, start) + 1, text[start:i + 1]))
                    break
    return out


def main():
    result = {}
    for rel, names in FUNCTIONS.items():
        for name in names:
            row = {}
            for label, root in ROOTS.items():
                with open(os.path.join(root, rel), encoding="utf-8", errors="replace") as f:
                    text = f.read()
                found = bodies(text, name)
                row[label] = [{"line": line, "sha256": hashlib.sha256(" ".join(body.split()).encode()).hexdigest()}
                              for line, body in found]
            digests = {tuple(d["sha256"] for d in v) for v in row.values()}
            result[f"{rel}::{name}"] = {"revisions": row, "identical": len(digests) == 1}
    dest = os.path.join(REPORT, "artifacts", "bodies.json")
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1)
        f.write("\n")
    for key, value in result.items():
        lines = {k: [d["line"] for d in v] for k, v in value["revisions"].items()}
        print(("identical " if value["identical"] else "DIFFERS   ") + key, lines)
    return 0


if __name__ == "__main__":
    sys.exit(main())
