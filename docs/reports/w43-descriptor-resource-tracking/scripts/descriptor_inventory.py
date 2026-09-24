#!/usr/bin/env python3
"""Inventory the descriptor and handle callees of a corpus diagnostics artifact.

Usage: descriptor_inventory.py DIAGNOSTICS_JSON_GZ [OUT_JSON]

For every callee in CALLEES, counts the `call to uncontracted function NAME`
leaves, the functions carrying one, and the Unknown functions whose *only*
distinct diagnostic description is that leaf (sole-blocked, as
`w43-return-expression/scripts/sole_callees.py` counts them). It also reports
every other diagnostic description that names one of the callees in backticks,
so refusals that already reach the call through another route are visible.
"""

import collections
import gzip
import json
import re
import sys

CALLEES = """
open openat open64 creat close close_range socket socketpair accept accept4 fcntl dup dup2 dup3
pipe pipe2 fopen fdopen freopen fclose fileno popen pclose tmpfile opendir fdopendir closedir
dlopen dlclose getaddrinfo freeaddrinfo fts_open fts_close glob globfree epoll_create
epoll_create1 eventfd timerfd_create signalfd inotify_init inotify_init1 mkstemp mkostemp
mkstemps mmap munmap shm_open memfd_create kqueue if_nameindex if_freenameindex getifaddrs
freeifaddrs scandir setmntent endmntent iconv_open iconv_close regcomp regfree
""".split()

UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")


def main() -> int:
    data = json.load(gzip.open(sys.argv[1]))
    leaves = collections.Counter()
    functions = collections.defaultdict(set)
    sole = collections.Counter()
    status = collections.defaultdict(collections.Counter)
    other = collections.Counter()
    for unit in data["units"]:
        for entry in unit["functions"]:
            diagnostics = entry.get("diagnostics") or []
            reasons = {d["description"] for d in diagnostics}
            for diagnostic in diagnostics:
                match = UNCONTRACTED.fullmatch(diagnostic["description"])
                if match and match.group(1) in CALLEES:
                    callee = match.group(1)
                    leaves[callee] += 1
                    key = (unit["file"], entry["function"])
                    if key not in functions[callee]:
                        status[callee][entry.get("status")] += 1
                    functions[callee].add(key)
                elif any(f"`{c}`" in diagnostic["description"] for c in CALLEES):
                    other[diagnostic["description"]] += 1
            if entry.get("status") == "Unknown" and len(reasons) == 1:
                match = UNCONTRACTED.fullmatch(next(iter(reasons)))
                if match and match.group(1) in CALLEES:
                    sole[match.group(1)] += 1
    rows = []
    for callee in CALLEES:
        if leaves[callee]:
            rows.append(
                {
                    "callee": callee,
                    "leaves": leaves[callee],
                    "functions": len(functions[callee]),
                    "sole_blocked": sole[callee],
                    "function_status": dict(status[callee]),
                    "function_list": sorted(f"{u}:{f}" for u, f in functions[callee]),
                }
            )
    all_functions = set().union(*functions.values()) if functions else set()
    summary = {
        "source": sys.argv[1],
        "callees_present": len(rows),
        "leaves": sum(leaves.values()),
        "functions": len(all_functions),
        "sole_blocked": sum(sole.values()),
        "absent": [c for c in CALLEES if not leaves[c]],
        "rows": rows,
        "other_descriptions": dict(other.most_common()),
    }
    print(f"{'callee':16} {'leaves':>6} {'funcs':>5} {'sole':>4}  status")
    for row in rows:
        print(
            f"{row['callee']:16} {row['leaves']:6} {row['functions']:5} {row['sole_blocked']:4}  "
            f"{row['function_status']}"
        )
    print(
        f"total: {summary['leaves']} leaves, {summary['functions']} functions, "
        f"{summary['sole_blocked']} sole-blocked, {summary['callees_present']} callees present"
    )
    print("absent:", " ".join(summary["absent"]))
    for description, count in other.most_common(40):
        print(f"  other {count}: {description[:140]}")
    if len(sys.argv) > 2:
        with open(sys.argv[2], "w", encoding="utf-8") as handle:
            json.dump(summary, handle, indent=1, sort_keys=True)
            handle.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
