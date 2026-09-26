#!/usr/bin/env python3
"""Rank the uncontracted callees with no definition in any corpus unit.

Usage: rank_external.py ARTIFACT_DIR [--owners OWNERS_JSON] [--json OUT] [--top N]

Reads `diagnostics.json.gz` from ARTIFACT_DIR (a corpus-arbiter run). A callee is
*external* when no unit of the run lists a function of that name, either among
its analysed functions or its `source_functions`.

For every external callee NAME it computes:

* `alone`: Unknown functions whose every diagnostic, in every category, is
  `call to uncontracted function `NAME`` (NAME is the only blocker left);
* `co_blocked`: Unknown functions whose every diagnostic is a call to an
  uncontracted function, with NAME among two or more distinct callees
  (NAME is one of the last `UNC` blockers alongside others);
* `co_blocked_external`: the subset of `co_blocked` whose other callees are all
  external too, so contracts alone could clear the function;
* `sites`: the number of `call to uncontracted function` leaves naming NAME.

The ranking is by `alone`, then `co_blocked`, then `sites`. A count is an upper
bound at this layer: a contract's own effects and the function's remaining
obligations still decide its verdict.

OWNERS_JSON, if given, maps a callee name to its owner class and the evidence
for it (see `classify_owners.py`); the ranking then carries both.
"""

import argparse
import collections
import gzip
import json
import pathlib
import re
import sys

UNC = re.compile(r"call to uncontracted function `([^`]+)`")


def rank(data):
    defined = set()
    for unit in data["units"]:
        for entry in unit["functions"]:
            defined.add(entry["function"])
        for name in unit.get("source_functions") or []:
            defined.add(name)
    sites = collections.Counter()
    units_of = collections.defaultdict(set)
    alone = collections.defaultdict(list)
    co = collections.defaultdict(list)
    co_ext = collections.defaultdict(list)
    external_sets = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            callees = []
            only_unc = True
            for diag in entry.get("diagnostics") or []:
                match = UNC.fullmatch(diag["description"])
                if match and diag["category"] == "unsupported":
                    callees.append(match.group(1))
                    if match.group(1) not in defined:
                        sites[match.group(1)] += 1
                        units_of[match.group(1)].add(unit["file"])
                else:
                    only_unc = False
            if entry.get("status") != "Unknown" or not only_unc or not callees:
                continue
            distinct = sorted(set(callees))
            key = f"{unit['file']}::{entry['function']}"
            if len(distinct) == 1:
                if distinct[0] not in defined:
                    alone[distinct[0]].append(key)
                continue
            all_external = all(name not in defined for name in distinct)
            if all_external:
                external_sets[key] = distinct
            for name in distinct:
                if name in defined:
                    continue
                co[name].append(key)
                if all_external:
                    co_ext[name].append(key)
    rows = []
    for name in sites:
        rows.append(
            {
                "callee": name,
                "alone": len(alone[name]),
                "co_blocked": len(co[name]),
                "co_blocked_external": len(co_ext[name]),
                "sites": sites[name],
                "units": len(units_of[name]),
                "alone_functions": sorted(alone[name]),
                "co_blocked_functions": sorted(co[name]),
            }
        )
    rows.sort(key=lambda r: (-r["alone"], -r["co_blocked"], -r["sites"], r["callee"]))
    return defined, rows, external_sets


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("artifacts")
    parser.add_argument("--owners")
    parser.add_argument("--json")
    parser.add_argument("--top", type=int, default=60)
    args = parser.parse_args()
    data = json.load(gzip.open(pathlib.Path(args.artifacts) / "diagnostics.json.gz"))
    defined, rows, external_sets = rank(data)
    owners = json.loads(pathlib.Path(args.owners).read_text()) if args.owners else {}
    for row in rows:
        if row["callee"] in owners:
            row["owner"] = owners[row["callee"]]["owner"]
            row["owner_evidence"] = owners[row["callee"]]["evidence"]
    total_sites = sum(r["sites"] for r in rows)
    print(f"external callees: {len(rows)}; external call sites: {total_sites}")
    print(f"functions blocked alone by one external callee: {sum(r['alone'] for r in rows)}")
    by_owner = collections.Counter()
    for row in rows:
        by_owner[row.get("owner", "unclassified")] += 1
    if owners:
        print("callees by owner: " + ", ".join(f"{k} {v}" for k, v in sorted(by_owner.items())))
    print(f"{'rank':>4} {'alone':>5} {'co':>4} {'coext':>5} {'sites':>5}  callee  [owner]")
    for index, row in enumerate(rows[: args.top], 1):
        owner = f"  [{row['owner']}]" if "owner" in row else ""
        print(
            f"{index:4} {row['alone']:5} {row['co_blocked']:4} {row['co_blocked_external']:5} "
            f"{row['sites']:5}  {row['callee']}{owner}"
        )
    if args.json:
        pathlib.Path(args.json).write_text(json.dumps(rows, indent=1, sort_keys=True) + "\n")
    print("functions whose every blocker is an uncontracted external callee, two or more of them:")
    for key, names in sorted(external_sets.items()):
        print(f"  {key}: {', '.join(names)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
