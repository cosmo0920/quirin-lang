#!/usr/bin/env python3
"""Count plain (non-safe) list iterations whose body unlinks or frees, in the
units of an arbiter report, by source reading.

Usage: plain_unlink_census.py REPORT SOURCE_ROOT OUT_JSON

For every `mk_list_foreach(` / `cfl_list_foreach(` (not `_safe`) in a
translation unit the report names, the braced body is read (brace matching
on the original text) and classified by whether it contains a list unlink
call (`mk_list_del`, `cfl_list_del`) or a release call (`free`, `flb_free`,
`mk_mem_free`, `cfl_sds_destroy`, `flb_sds_destroy`), and whether such a
statement is followed by `break`/`return`/`goto` in the same block. Source
reading for the report's prediction check only.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

PLAIN = re.compile(r"\b(mk_list_foreach|cfl_list_foreach)\s*\(")
UNLINK = re.compile(r"\b(mk_list_del|cfl_list_del)\s*\(")
RELEASE = re.compile(r"\b(free|flb_free|mk_mem_free|cfl_sds_destroy|flb_sds_destroy)\s*\(")
LEAVE = re.compile(r"\b(break|return|goto)\b")


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main():
    report = load(sys.argv[1])
    root = pathlib.Path(sys.argv[2])
    by_name = collections.defaultdict(list)
    for path in root.rglob("*.c"):
        by_name[path.name].append(path)
    counts = collections.Counter()
    sites = []
    for unit in report["units"]:
        for path in by_name.get(pathlib.Path(unit["file"]).name, []):
            text = path.read_text(errors="replace")
            for match in PLAIN.finditer(text):
                brace = text.find("{", match.end())
                if brace < 0:
                    continue
                depth = 0
                end = brace
                for index in range(brace, len(text)):
                    if text[index] == "{":
                        depth += 1
                    elif text[index] == "}":
                        depth -= 1
                        if depth == 0:
                            end = index
                            break
                body = text[brace:end]
                unlink = bool(UNLINK.search(body))
                release = bool(RELEASE.search(body))
                leaves = bool(LEAVE.search(body))
                key = ("unlink" if unlink else "") + ("+release" if release else "") or "neither"
                counts[key] += 1
                if unlink or release:
                    counts["with-" + ("leave" if leaves else "no-leave")] += 1
                    line = text.count("\n", 0, match.start()) + 1
                    sites.append({"file": str(path.relative_to(root)), "line": line, "unlink": unlink, "release": release, "leaves": leaves})
    result = {"counts": dict(counts), "sites": sites}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result["counts"], indent=1))


if __name__ == "__main__":
    main()
