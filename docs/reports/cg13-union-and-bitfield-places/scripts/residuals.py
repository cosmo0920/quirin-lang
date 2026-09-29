#!/usr/bin/env python3
"""CG13: the union leaves still present after the card, by a lexical class of
their original source line (a reading aid, not a causal attribution).

usage: residuals.py DIAGNOSTICS OUT_JSON [--show CLASS]

A remaining leaf is a current unsupported leaf that keeps CG00's M06
attribution (population.py's matching). Its class is the first that matches
its original source line, read from the pinned checkout by the leaf's
`origin`: `release` (a free/destroy-like callee), `pointer-store` (CG00's own
pointer-valued-store cause), `indirect-call`, `cast`, `address` (`&`), `call`
(any other call on the line), `other`.
"""

import collections
import gzip
import json
import os
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
LEAVES = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"
ROOT = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
RELEASE = re.compile(r"\b\w*(free|destroy|FREE|release|unref)\w*\s*\(")
CALL = re.compile(r"\b[A-Za-z_]\w*\s*\(")
KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "unlikely", "likely"}


def main():
    diagnostics = json.load(open(sys.argv[1]))
    show = sys.argv[4] if len(sys.argv) > 4 and sys.argv[3] == "--show" else None
    with gzip.open(LEAVES, "rt") as handle:
        rows = [json.loads(line) for line in handle][1:]
    old = collections.defaultdict(list)
    for row in rows:
        old[(row["file"], row["function"], row["line"], row["description"])].append(row)
    index = {}
    for dirpath, _, filenames in os.walk(ROOT):
        rel = os.path.relpath(dirpath, ROOT)
        if rel.startswith("build") or "/.git" in dirpath:
            continue
        for name in filenames:
            index.setdefault(name, []).append(os.path.join(dirpath, name))
    texts = {}

    def source(origin):
        name, line = origin.rsplit(":", 1)
        paths = index.get(name, [])
        if len(paths) != 1:
            return ""
        if paths[0] not in texts:
            with open(paths[0], errors="replace") as handle:
                texts[paths[0]] = handle.read().split("\n")
        return texts[paths[0]][int(line) - 1]

    classes = collections.Counter()
    functions = collections.defaultdict(set)
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            used = collections.Counter()
            for leaf in function["diagnostics"]:
                if leaf["category"] != "unsupported":
                    continue
                lookup = (unit["file"], function["function"], leaf["line"],
                          re.sub(r"`[^`]*`", "`X`", leaf["description"]))
                candidates = old.get(lookup, [])
                if used[lookup] >= len(candidates):
                    continue
                row = candidates[used[lookup]]
                used[lookup] += 1
                if row["entry"] != "M06":
                    continue
                text = source(leaf["origin"])
                reason = row["reason"]
                if RELEASE.search(text):
                    kind = "release"
                elif "pointer-valued-store" in reason:
                    kind = "pointer-store"
                elif "indirect-call" in reason:
                    kind = "indirect-call"
                elif "cast" in reason:
                    kind = "cast"
                elif "&" in text.replace("&&", ""):
                    kind = "address"
                elif any(m.group(0).split("(")[0].strip() not in KEYWORDS for m in CALL.finditer(text)):
                    kind = "call"
                else:
                    kind = "other"
                key = f"{kind} ({row['family']})"
                if show == kind:
                    print(row["family"], function["function"], "|", text.strip()[:120])
                classes[key] += 1
                functions[key].add((unit["file"], function["function"]))
    result = {
        "schema": "qcore.cg13-union-residuals.v1",
        "leaves": sum(classes.values()),
        "classes": {k: {"leaves": n, "functions": len(functions[k])} for k, n in classes.most_common()},
    }
    with open(sys.argv[2], "w") as handle:
        json.dump(result, handle, indent=1, sort_keys=True)
        handle.write("\n")
    if show is None:
        print(json.dumps(result, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
