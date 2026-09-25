#!/usr/bin/env python3
"""Join every published leak leaf with its leak-witness census records.

Usage: join_witnesses.py DIAGNOSTICS(.json|.json.gz) LEAK_WITNESS.jsonl OUT.json

A published leaf is identified, as every pinned comparison identifies it, by
(unit, function, preprocessed exit line, description). The census writes one
record per leak an analysis *invocation* reported (W4 derivation, validation
and the published analysis each run the same body), keyed by function, exit
line and the preprocessed acquisition line. A leaf is joined to the records of
the invocations whose reported leak lines equal the published function's leak
lines exactly (as multisets); within one function the acquisitions are paired
in (exit line, acquisition line) order, since the preprocessed and original
lines of one body are monotone in each other.

Every published leaf must join, and every joined invocation must agree on the
settled-exit fields; the script exits 1 otherwise. Witness paths may differ
between invocations (a different entry custody binds formals differently), so
for each of `path`, `clean_path` and `strict_path` the output records whether
one exists in every joined invocation and in some; a class that rests on a
path's existence or absence reads those flags.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

FIELDS = ("settled", "status", "unresolved", "exempt", "exit_unmodeled", "exit_widened")
PATHS = ("path", "clean_path", "strict_path")


def load(path: str):
    raw = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        raw = gzip.decompress(raw)
    return json.loads(raw)


def acquisition_origin(description: str) -> int:
    match = re.search(r"acquired at \S+:(\d+)$", description)
    return int(match.group(1)) if match else 0


def steps(path):
    if path is None:
        return None
    return [
        {
            "line": step["line"],
            "instruction": step["instruction"],
            "refused": step["refused"],
            "refused_read": step["refused_read"],
            "refused_call": step.get("refused_call", False),
            "edge_unknown": step.get("edge_unknown", False),
            "bound": step["bound"],
        }
        for step in path
    ]


def summary(record):
    return {
        **{field: record[field] for field in FIELDS},
        "bound_at_exit": record["bound_at_exit"],
        **{key: steps(record.get(key)) for key in PATHS},
    }


def main() -> int:
    diagnostics = load(sys.argv[1])
    records = [json.loads(line) for line in pathlib.Path(sys.argv[2]).read_text().splitlines()]
    by_function = collections.defaultdict(list)
    for record in records:
        by_function[record["function"]].append(record)
    joined = []
    failures = []
    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            leaks = [d for d in function.get("diagnostics") or [] if d["category"] == "leak"]
            if not leaks:
                continue
            origins = {d["line"]: d.get("origin") for d in function.get("diagnostics") or []}
            published = collections.Counter(d["line"] for d in leaks)
            invocations = collections.defaultdict(list)
            for record in by_function[function["function"]]:
                invocations[(record["unit_seq"], record["invocation"])].append(record)
            matching = [
                sorted(group, key=lambda r: (r["line"], r["acquisition_line"]))
                for group in invocations.values()
                if collections.Counter(line for line, _ in group[0]["reported"]) == published
            ]
            if not matching:
                failures.append(f"{unit['file']}:{function['function']}: no matching invocation")
                continue
            ordered = sorted(leaks, key=lambda d: (d["line"], acquisition_origin(d["description"])))
            for index, leaf in enumerate(ordered):
                evidence = [summary(group[index]) for group in matching]
                agree = all({k: e[k] for k in FIELDS} == {k: evidence[0][k] for k in FIELDS} for e in evidence)
                if not agree:
                    failures.append(
                        f"{unit['file']}:{function['function']}:{leaf['line']}: settled fields disagree"
                    )
                chosen = dict(evidence[-1])
                for key in PATHS:
                    present = [e[key] is not None for e in evidence]
                    chosen[f"{key}_in_every_invocation"] = all(present)
                    chosen[f"{key}_in_some_invocation"] = any(present)
                    if chosen[key] is None and any(present):
                        chosen[key] = next(e[key] for e in evidence if e[key] is not None)
                    for step in chosen[key] or []:
                        step["origin"] = origins.get(step["line"])
                joined.append(
                    {
                        "library": unit["library"],
                        "unit": unit["file"],
                        "function": function["function"],
                        "line": leaf["line"],
                        "origin": leaf.get("origin"),
                        "description": leaf["description"],
                        "acquisition_line": matching[-1][index]["acquisition_line"],
                        "invocations": len(matching),
                        "paths_agree": all(
                            {k: e[k] is not None for k in PATHS}
                            == {k: evidence[0][k] is not None for k in PATHS}
                            for e in evidence
                        ),
                        "witness": chosen,
                    }
                )
    document = {"leaves": len(joined), "failures": failures, "joined": joined}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(document, indent=1) + "\n")
    print(json.dumps({"leaves": len(joined), "failures": failures}, indent=1))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
