#!/usr/bin/env python3
"""MEMBER-EFFECT-SUMMARY step 1: an estimate, from MEMBER-ACCESS-PROJECTION's
census of the 046accf6 `current` run, of the indirect-access leaves rule 38i
re-attributes to each of its three names.

Not the analyzer: the census has no contract modes and no route answers, so a
callee's release positions and owned results are read here from every
provider's `.qv` text (`resource_released(P, ...)`, `resource_return_owned` /
`resource_return_reallocated`), by name, whatever the profile's view. A leaf
counts under a name when every one of its typed paths is the premise's one
path (so no second access of the statement can decide), the root is a formal,
and:

* release: the path is `P->F`, pointer-valued, a call actual at a position its
  callee's contract releases;
* store: the path is a store into `P->F`, pointer-valued, its right-hand side a
  plain name or a call whose contract returns custody;
* address: the path is `&P->F[.G...]` (an embedded record), a call actual at a
  position its callee's contract releases.

usage: estimate_step1.py PREMISES_JSONL_GZ PROVIDERS_DIR
"""

import collections
import gzip
import json
import pathlib
import re
import sys

IND = "indirect memory access is outside the resource projection"


def contracts(root: pathlib.Path):
    releases = collections.defaultdict(set)
    owned = set()
    for path in root.glob("*/contracts/**/*.qv"):
        text = path.read_text(errors="replace")
        for match in re.finditer(r"extern\s+fn\s+(\w+)\s*\((.*?)\)\s*->[^\n]*\ncontract\s*\{(.*?)\};",
                                 text, re.S):
            name, params, body = match.groups()
            names = [p.split(":")[0].strip() for p in re.split(r",(?![^<]*>)", params) if ":" in p]
            for released in re.findall(r"resource_released\(\s*(\w+)", body):
                if released in names:
                    releases[name].add(names.index(released))
            if re.search(r"resource_return_(owned|reallocated)\(", body):
                owned.add(name)
    return releases, owned


def name_of(leaf, releases, owned):
    paths = leaf["paths"]
    if len(paths) != 1:
        return None
    path = paths[0]
    if path["root_kind"] != "formal":
        return None
    steps = path["steps"]
    if not steps or steps[0]["op"] != "->":
        return None
    if path["access"] == "read" and path["value"] == "pointer" and len(steps) == 1 \
            and path.get("call_argument") and path.get("arg") in releases.get(path.get("callee"), ()):
        return "member-release-through-formal"
    if path["access"] == "store" and path["value"] == "pointer" and len(steps) == 1:
        rhs = path.get("rhs") or ""
        if rhs.split(":")[0] in ("variable", "formal") or (rhs.startswith("call:") and rhs[5:] in owned):
            return "member-store-through-formal"
    if path["access"] == "address" and path["value"] == "address" \
            and all(step["op"] == "." for step in steps[1:]) \
            and steps[-1]["value"] == "record" \
            and path.get("call_argument") and path.get("arg") in releases.get(path.get("callee"), ()):
        return "member-address-release-through-formal"
    return None


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    releases, owned = contracts(pathlib.Path(sys.argv[2]))
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    by_function = collections.defaultdict(list)
    for row in rows:
        key = (row["file"], row["function"])
        name = name_of(row, releases, owned)
        by_function[key].append(name)
        if name:
            leaves[name] += row["leaves"]
            functions[name].add(key)
    for key, names in by_function.items():
        row_families = next(r for r in rows if (r["file"], r["function"]) == key)["families"]
        if set(row_families) <= {IND} and len(set(names)) == 1 and names[0]:
            sole[names[0]].add(key)
    print("| name | leaves | functions | functions whose every access leaf it names (no other family) |")
    print("|---|---:|---:|---:|")
    for name in ("member-release-through-formal", "member-store-through-formal",
                 "member-address-release-through-formal"):
        print(f"| `{name}` | {leaves[name]} | {len(functions[name])} | {len(sole[name])} |")
    print(f"| total | {sum(leaves.values())} | {len(set().union(*functions.values()))} | "
          f"{sum(len(v) for v in sole.values())} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
