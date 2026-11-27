#!/usr/bin/env python3
"""classify_census.py WITNESS.jsonl [--json OUT] -- SUP07-REFUSED-TEST-ACCUSATION's
census reading of one leak-witness stream (census.sh).

Each leak witness is classified by its paths (legacy_leak_witness.rs):

* `no_path`: no producer -> exit path keeps the site live in the settled
  states (an iterate-only report, or the census could not rebuild it);
* `refused_test_only`: a path exists, but none avoids the null (invalid) edge of
  a refused test whose tested binding may denote the site: every witness keeps
  the site live only because a refused test did not decide it;
* `tied`: of those, none avoids the edges where joined provenance also ties the
  site to the tested name: exactly the claims rule 12e settles;
* `independent`: a path avoids every such edge: the claim does not rest on a
  refused test.

`refused_test_edge` records (every refused test's null edge whose binding
denotes a site, leaks or none) are counted and listed with their tied sites.
"""
import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("stream", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    witnesses, edges = [], []
    if args.stream.exists():
        for line in args.stream.read_text().splitlines():
            row = json.loads(line)
            (edges if row.get("record") == "refused_test_edge" else witnesses).append(row)
    classes = collections.Counter()
    rows = []
    for w in witnesses:
        if not w["path"]:
            kind = "no_path"
        elif w["untested_path"] is None:
            kind = "tied" if w["untied_path"] is None else "refused_test_only"
        else:
            kind = "independent"
        classes[kind] += 1
        rows.append({
            "function": w["function"],
            "line": w["line"],
            "acquisition_line": w["acquisition_line"],
            "settled": w["settled"],
            "class": kind,
            "refused_null_steps": [s["line"] for s in (w["path"] or []) if s["edge_refused_null"]],
            "tied_steps": [s["line"] for s in (w["path"] or []) if s["edge_tied"]],
        })
    tied_edges = [e for e in edges if e["tied"]]
    summary = {
        "leak_witnesses": len(witnesses),
        "classes": dict(sorted(classes.items())),
        "rests_on_refused_test": [r for r in rows if r["class"] in ("tied", "refused_test_only")],
        "path_through_refused_null_edge": [r for r in rows if r["refused_null_steps"]],
        "refused_test_edges": len(edges),
        "refused_test_edges_functions": len({e["function"] for e in edges}),
        "tied_edges": len(tied_edges),
        "tied_edges_live": sum(1 for e in tied_edges if e["tied_live"]),
        "tied_edge_rows": [
            {k: e[k] for k in ("function", "line", "source", "targets", "tied", "tied_live")}
            for e in tied_edges
        ],
    }
    text = json.dumps(summary, indent=1)
    if args.json:
        args.json.write_text(text + "\n")
    print(json.dumps({k: v for k, v in summary.items() if not isinstance(v, list)}))
    for key in ("rests_on_refused_test", "path_through_refused_null_edge"):
        print(key, json.dumps(summary[key]))
    seen = collections.Counter(e["function"] for e in tied_edges)
    print("tied edge functions", len(seen), dict(seen))


if __name__ == "__main__":
    main()
