#!/usr/bin/env python3
"""distinct.py PARITY DIAGNOSTICS [--json OUT] -- CG22's census by distinct
site (function, line, callee tokens), builtins apart, restricted to functions
the run reports, with their status and the leaves on each site's line."""
import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity")
    parser.add_argument("diagnostics")
    parser.add_argument("--json")
    args = parser.parse_args()
    rows, regs = {}, {}
    for line in open(args.parity, encoding="utf-8"):
        r = json.loads(line)
        if r.get("position") == "indirect-call":
            rows.setdefault((r["function"], r["line"], tuple(r["texts"])), r)
        elif r.get("position") == "callback-registration":
            regs.setdefault((r["function"], r["line"], tuple(r["designators"])), r)
    d = json.load(open(args.diagnostics, encoding="utf-8"))
    reported = collections.defaultdict(list)
    for u in d["units"]:
        for f in u["functions"]:
            reported[f["function"]].append(f)

    def builtin(r):
        return r["open"] == "unread form implicit BuiltinFnToFnPtr"

    out = {"distinct_sites": len(rows), "builtin": sum(builtin(r) for r in rows.values())}
    real = {k: r for k, r in rows.items() if not builtin(r)}
    out["indirect_distinct"] = len(real)
    out["by_shape_open"] = dict(
        collections.Counter(f'{r["shape"]}: {r["open"]}' for r in real.values()).most_common()
    )
    out["by_rewrite"] = dict(collections.Counter(r["rewrite"] for r in real.values()).most_common())
    rep = {k: r for k, r in real.items() if k[0] in reported}
    out["indirect_in_reported_functions"] = len(rep)
    out["reported_by_shape_open"] = dict(
        collections.Counter(f'{r["shape"]}: {r["open"]}' for r in rep.values()).most_common()
    )
    out["reported_functions"] = len({k[0] for k in rep})
    status = collections.Counter()
    for name in {k[0] for k in rep}:
        for f in reported[name]:
            status[f["status"]] += 1
    out["reported_function_status"] = dict(status)
    leaves = collections.Counter()
    for k in rep:
        for f in reported[k[0]]:
            for g in f.get("diagnostics", []):
                if g.get("line") == k[1]:
                    leaves[f'{g.get("category")}: {(g.get("description") or "")[:80]}'] += 1
    out["leaves_on_site_lines"] = dict(leaves.most_common(15))
    out["single_name_callees"] = sorted({k[2][0] for k in rep if len(k[2]) == 1})
    out["registrations_distinct"] = len(regs)
    out["registrations_reported"] = sum(1 for k in regs if k[0] in reported)
    out["registration_relation_reported"] = dict(
        collections.Counter(
            v for k, r in regs.items() if k[0] in reported for v in r["context_relation"]
        )
    )
    out["registration_unsafe_rows"] = sorted(
        (k[0], k[1], r["callee"]) for k, r in regs.items() if "Unsafe" in r["context_relation"]
    )
    text = json.dumps(out, indent=1)
    if args.json:
        pathlib.Path(args.json).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
