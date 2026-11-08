#!/usr/bin/env python3
"""MEMBER-EFFECT-COMPONENT-SCOPE census on the user's `current` run.

The run's report on disk predates rule 38i, so its leaves carry the gate's
text, not rule 38i's names. This reads MEMBER-ACCESS-PROJECTION's census of
that run (`premises.jsonl.gz`) and selects the release leaves with
MEMBER-EFFECT-SUMMARY's estimate (`estimate_step1.py`'s `name_of`, the same
selection that report's step-1 table used), then classifies each leaf's base
and member spelling with `census.py`'s lexical reader over the run's source
tree. An estimate, as that table was.

usage: census_current.py PREMISES_JSONL_GZ PROVIDERS_DIR SOURCE_ROOT
"""

import collections
import gzip
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


census = load("census", HERE / "census.py")
estimate = load("estimate_step1", HERE.parent.parent / "member-effect-summary/scripts/estimate_step1.py")

RELEASES = ("member-release-through-formal", "member-address-release-through-formal")


def main(argv):
    rows = [json.loads(line) for line in gzip.open(argv[1], "rt")]
    releases, owned = estimate.contracts(pathlib.Path(argv[2]))
    tree = census.Tree(pathlib.Path(argv[3]))
    by_function = collections.defaultdict(list)
    for row in rows:
        by_function[(row["file"], row["function"])].append(row)
    leaves = []
    for (file, function), function_rows in sorted(by_function.items()):
        names = [estimate.name_of(row, releases, owned) for row in function_rows]
        if not any(name in RELEASES for name in names):
            continue
        families = function_rows[0]["families"]
        sole = set(families) <= {estimate.IND} and all(name in RELEASES for name in names)
        path = tree.root / file
        text = tree.text(path) if path.is_file() else ""
        for row, name in zip(function_rows, names):
            if name not in RELEASES:
                continue
            step = row["paths"][0]
            base = step["root"]
            member = step["steps"][0]["member"]
            b_kind, tag = census.base_class(tree, census.parameter_spelling(text, function, base))
            leaves.append({
                "file": file,
                "function": function,
                "status": row["status"],
                "origin": row["origin"],
                "name": name,
                "leaves": row["leaves"],
                "base": b_kind,
                "member": census.member_class(tree, tag, member) if member else "unparsed",
                "sole": sole,
            })
    print(json.dumps({
        "premises": argv[1],
        "source_root": argv[3],
        "leaves": sum(leaf["leaves"] for leaf in leaves),
        "functions": len({(l["file"], l["function"]) for l in leaves}),
        "table": census.table(leaves),
        "rows": leaves,
    }, indent=1))


if __name__ == "__main__":
    main(sys.argv)
