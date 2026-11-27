#!/usr/bin/env python3
"""CG22-INDIRECT-CALLS-AND-CALLBACKS: the census of indirect calls and callback
registrations, from a run's parity stream (`position: "indirect-call"` and
`"callback-registration"` rows of `legacy_indirect_calls.rs`).

Usage: census.py PARITY.jsonl [DIAGNOSTICS.json] [--json OUT]

Rows are de-duplicated by (source digest, function, line, callee tokens or
registration line): every analysis entry over a unit (W4 derivation, the
review settlement, the certificate recheck) records its own copy. With the
run's resource-diagnostics.json, each site is joined to the leaves its
function carries on its line, and to its function's status.
"""

import argparse
import collections
import json
import pathlib


def load_rows(path):
    sites, registrations = {}, {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            position = row.get("position")
            if position == "indirect-call":
                key = (row["source_sha256"], row["function"], row["line"], tuple(row["texts"]))
                sites.setdefault(key, row)
            elif position == "callback-registration":
                key = (row["source_sha256"], row["function"], row["line"], tuple(row["designators"]))
                registrations.setdefault(key, row)
    return sites, registrations


def load_functions(path):
    data = json.loads(pathlib.Path(path).read_text(encoding="utf-8"))
    functions = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit["functions"]:
            functions[function["function"]].append(function)
    return functions


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity")
    parser.add_argument("diagnostics", nargs="?")
    parser.add_argument("--json")
    args = parser.parse_args()
    sites, registrations = load_rows(args.parity)
    functions = load_functions(args.diagnostics) if args.diagnostics else {}
    out = {}
    out["sites"] = len(sites)
    out["site_functions"] = len({(k[0], k[1]) for k in sites})
    out["by_shape"] = dict(collections.Counter(r["shape"] for r in sites.values()).most_common())
    out["by_form"] = dict(collections.Counter(r["form"] for r in sites.values()).most_common())
    out["closed"] = sum(1 for r in sites.values() if r["open"] is None)
    out["by_open_reason"] = dict(
        collections.Counter(r["open"] for r in sites.values() if r["open"]).most_common()
    )
    out["by_rewrite"] = dict(collections.Counter(r["rewrite"] for r in sites.values()).most_common())
    out["shape_x_closed"] = dict(
        collections.Counter(
            f'{r["shape"]}:{"closed" if r["open"] is None else "open"}' for r in sites.values()
        ).most_common()
    )
    closed = [r for r in sites.values() if r["open"] is None]
    out["closed_candidate_standing"] = dict(
        collections.Counter(
            "all-declared" if all(c["declared"] for c in r["candidates"])
            else "all-declared-or-defined" if all(c["declared"] or c["defined"] for c in r["candidates"])
            else "some-neither"
            for r in closed
        ).most_common()
    )
    out["closed_set_sizes"] = dict(
        collections.Counter(len(r["candidates"]) for r in closed).most_common()
    )
    out["closed_rows"] = sorted(
        (
            r["function"],
            r["line"],
            " ".join(r["texts"]),
            r["shape"],
            r["form"],
            ",".join(c["name"] + ("[D]" if c["declared"] else "") + ("[d]" if c["defined"] else "") for c in r["candidates"]),
            r["rewrite"],
        )
        for r in closed
    )
    if functions:
        status = collections.Counter()
        for key in {(k[1]) for k in sites}:
            for function in functions.get(key, []):
                status[function["status"]] += 1
        out["site_function_status"] = dict(status)
        closed_status = collections.Counter()
        for key in {r["function"] for r in closed}:
            for function in functions.get(key, []):
                closed_status[function["status"]] += 1
        out["closed_function_status"] = dict(closed_status)
    out["registrations"] = len(registrations)
    out["registration_functions"] = len({(k[0], k[1]) for k in registrations})
    out["registration_callee"] = dict(
        collections.Counter(
            "indirect" if r["callee"] is None
            else "declared" if r["callee_declared"]
            else "uncontracted"
            for r in registrations.values()
        ).most_common()
    )
    out["registration_with_context"] = sum(1 for r in registrations.values() if r["contexts"])
    out["registration_declared_with_context"] = sorted(
        (r["function"], r["line"], r["callee"], ",".join(r["designators"]))
        for r in registrations.values()
        if r["callee_declared"] and r["contexts"]
    )
    out["registration_relation"] = dict(
        collections.Counter(
            verdict for r in registrations.values() for verdict in r["context_relation"]
        ).most_common()
    )
    out["registration_top_callees"] = dict(
        collections.Counter(r["callee"] for r in registrations.values()).most_common(15)
    )
    text = json.dumps(out, indent=1, ensure_ascii=False)
    if args.json:
        pathlib.Path(args.json).write_text(text + "\n", encoding="utf-8")
    summary = {k: v for k, v in out.items() if k not in ("closed_rows", "registration_declared_with_context")}
    print(json.dumps(summary, indent=1, ensure_ascii=False))
    print("closed rows:", len(out["closed_rows"]))
    for row in out["closed_rows"][:80]:
        print("  ", row)
    print("declared registrations with a context:", len(out["registration_declared_with_context"]))
    for row in out["registration_declared_with_context"][:30]:
        print("  ", row)


if __name__ == "__main__":
    main()
