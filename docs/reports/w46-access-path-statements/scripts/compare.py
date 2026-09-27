#!/usr/bin/env python3
"""Compare complete corpus artifacts and the frozen P2 forecast.

Usage: compare.py BEFORE AFTER PARITY OUTPUT_DIR [--no-forecast].
Inputs may be JSON or gzip.
Every changed function row is retained; no Unknown is removed from counts.
"""

import collections
import gzip
import io
import json
import pathlib
import sys


def read(path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as stream:
        return json.load(stream)


def diagnostics(row):
    return collections.Counter(json.dumps(d, sort_keys=True) for d in row["diagnostics"])


def render_diagnostics(rows):
    return [dict(json.loads(text), count=count) for text, count in sorted(rows.items())]


def gzip_writer(path):
    return io.TextIOWrapper(gzip.GzipFile(filename=str(path), mode="wb", mtime=0), encoding="utf-8")


def main():
    before_path, after_path, parity_path, out_path = map(pathlib.Path, sys.argv[1:5])
    no_forecast = "--no-forecast" in sys.argv[5:]
    out_path.mkdir(parents=True, exist_ok=False)
    before, after = read(before_path), read(after_path)
    if len(before["units"]) != len(after["units"]):
        raise SystemExit("unit denominator changed")
    moves = []
    changed = []
    before_status = collections.Counter()
    after_status = collections.Counter()
    after_by_name = {}
    before_by_name = {}
    changed_keys = set()
    for ordinal, (old_unit, new_unit) in enumerate(zip(before["units"], after["units"])):
        if (old_unit["file"], old_unit["library"]) != (new_unit["file"], new_unit["library"]):
            raise SystemExit(f"unit identity changed at {ordinal}")
        old_rows = {r["function"]: r for r in old_unit["functions"]}
        new_rows = {r["function"]: r for r in new_unit["functions"]}
        if old_rows.keys() != new_rows.keys():
            raise SystemExit(f"function denominator changed at {ordinal}")
        for function in sorted(old_rows):
            old, new = old_rows[function], new_rows[function]
            key = (new_unit["library"], new_unit["file"], function)
            after_by_name[key] = new["status"]
            before_by_name[key] = old["status"]
            before_status[old["status"]] += 1
            after_status[new["status"]] += 1
            removed = diagnostics(old) - diagnostics(new)
            added = diagnostics(new) - diagnostics(old)
            if old["status"] == new["status"] and not removed and not added:
                continue
            row = {
                "unit_index": ordinal,
                "library": key[0],
                "file": key[1],
                "function": key[2],
                "before": old["status"],
                "after": new["status"],
                "removed": render_diagnostics(removed),
                "added": render_diagnostics(added),
            }
            changed.append(row)
            changed_keys.add(key)
            if old["status"] != new["status"]:
                moves.append(row)
    forecast = []
    if not no_forecast:
        prediction_path = pathlib.Path(__file__).parents[2] / "w46-access-path-census/artifacts/prediction.json"
        with prediction_path.open() as stream:
            stream.readline()  # schema envelope
            prediction = json.load(stream)
        for row in prediction["freed"]["P2"]:
            if row["needs_cross_unit_summary"]:
                continue
            key = (row["library"], row["file"], row["function"])
            forecast.append({
                "library": key[0], "file": key[1], "function": key[2],
                "predicted": row["verdict"], "before": before_by_name.get(key),
                "observed": after_by_name.get(key),
                "changed": key in changed_keys,
                "hit": after_by_name.get(key) == row["verdict"] and key in changed_keys,
                "reason": row["reason"],
            })
    parity_totals = collections.Counter()
    parity_disagreements = collections.Counter()
    disagreements = []
    with parity_path.open() as stream:
        for line in stream:
            row = json.loads(line)
            route = row["old_route"]
            parity_totals[route] += 1
            if row["disagreement"]:
                parity_disagreements[route] += 1
                disagreements.append(json.dumps(row, sort_keys=True))
    with gzip_writer(out_path / "parity-disagreements.jsonl.gz") as output:
        for row in sorted(disagreements):
            output.write(row + "\n")
    result = {
        "schema": "qcore.w46-access-path-statements-comparison.v1",
        "before_status": dict(before_status),
        "after_status": dict(after_status),
        "before_diagnostic_counts": before["diagnostic_counts"],
        "after_diagnostic_counts": after["diagnostic_counts"],
        "changed_functions": len(changed),
        "verdict_moves": len(moves),
        "moves": moves,
        "p2_forecast": forecast,
        "p2_hits": sum(row["hit"] for row in forecast),
        "parity_rows": dict(parity_totals),
        "parity_disagreements": dict(parity_disagreements),
    }
    (out_path / "comparison.json").write_text(json.dumps(result, indent=2) + "\n")
    with gzip_writer(out_path / "changed-functions.jsonl.gz") as output:
        for row in changed:
            output.write(json.dumps(row, sort_keys=True) + "\n")
    print(json.dumps({key: value for key, value in result.items() if key not in ("moves", "p2_forecast")}, indent=2))


if __name__ == "__main__":
    main()
