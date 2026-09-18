#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: tabulate the polarity/arm census."""
import collections
import json
import pathlib
import sys

M = pathlib.Path(__file__).resolve().parent


def main():
    rows = [json.loads(line) for line in (M / "census.jsonl").read_text().splitlines() if line]
    print(f"census rows (BranchOnNull edges, settled states): {len(rows)}")
    print(f"distinct BranchOnNull nodes: {len({(r['function'], r['node']) for r in rows})}")
    print(f"distinct functions with a BranchOnNull: {len({r['function'] for r in rows})}")

    headers = {(r["function"], r["node"]) for r in rows if r["loop_header"]}
    print(f"distinct LOOP HEADER BranchOnNull nodes: {len(headers)}")
    print()

    print("== edges by role x result ==")
    by = collections.Counter((r["role"], r["result"]) for r in rows)
    for (role, result), n in sorted(by.items()):
        print(f"  {role:<7} {result:<11} {n:>7}")
    print()

    print("== edges by role x arm ==")
    by = collections.Counter((r["role"], r["arm"]) for r in rows)
    for (role, arm), n in sorted(by.items()):
        print(f"  {role:<7} {arm:<17} {n:>7}")
    print()

    print("== LOOP HEADER edges: polarity x role x arm x result ==")
    loops = [r for r in rows if r["loop_header"]]
    by = collections.Counter(
        (r["true_is_null"], r["role"], r["arm"], r["result"]) for r in loops
    )
    for (pol, role, arm, result), n in sorted(by.items(), key=lambda kv: (-kv[1], str(kv[0]))):
        print(f"  true_is_null={str(pol):<5} {role:<5} {arm:<17} {result:<11} {n:>6}")
    print()

    deleted = [r for r in loops if r["result"] == "infeasible"]
    print(f"== loop-header edges the value route calls INFEASIBLE: {len(deleted)} ==")
    exits = [r for r in deleted if r["role"] == "exit"]
    bodies = [r for r in deleted if r["role"] == "body"]
    print(f"   of which EXIT edges (this card's hazard): {len(exits)}")
    print(f"   of which BODY edges (the untouched sibling): {len(bodies)}")
    print()
    if exits:
        print("   EXIT-edge deletions, by function:")
        for (fn, line), n in sorted(
            collections.Counter((r["function"], r["line"]) for r in exits).items()
        ):
            print(f"     {fn} @ internal L{line}  x{n}")
    print()
    print("== branch (if) edges called infeasible — the precision this card keeps ==")
    branch = [r for r in rows if r["role"] == "branch" and r["result"] == "infeasible"]
    print(f"   {len(branch)}")
    by = collections.Counter(r["arm"] for r in branch)
    for arm, n in sorted(by.items()):
        print(f"     {arm:<17} {n:>7}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
