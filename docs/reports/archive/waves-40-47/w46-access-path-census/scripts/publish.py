#!/usr/bin/env python3
"""Card W46-ACCESS-PATH-CENSUS: write the small published evidence.

usage: publish.py CENSUS_OUT PREDICT_OUT CENSUS_JSON CLASSES_JSON REPORT_DIR

* `census-leaves.jsonl.gz` - one row per leaf group (every leaf of one family
  at one line of one function), schema line first: coordinates, family,
  leaf count, scope (`P2`, `P3` or `refused`), refusal causes and each path
  as root kind, root name, steps with their Clang types, value, access and
  position. The private `leaves.jsonl.gz` keeps the constructs' remaining
  facts.
* `prediction.json` - schema line, then every function a scope frees, with
  its predicted verdict, reason, obligations and closure round, per scope.
* `census.json` and `classes.json` - copied with their schema lines.
"""

import gzip
import json
import pathlib
import shutil
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import predict  # noqa: E402

SCHEMA = "qcore.w46-access-path-census.v1"


def main():
    census = pathlib.Path(sys.argv[1])
    predicted = pathlib.Path(sys.argv[2])
    census_json = pathlib.Path(sys.argv[3])
    classes_json = pathlib.Path(sys.argv[4])
    out = pathlib.Path(sys.argv[5])
    artifacts = out / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)
    rows = predict.jsonl(census / "leaves.jsonl.gz")
    with gzip.GzipFile(artifacts / "census-leaves.jsonl.gz", "wb", mtime=0) as raw:
        raw.write((json.dumps({"schema": SCHEMA, "kind": "leaf-groups",
                               "path": "root_kind, root, steps [op, member, clang type], value, access, "
                                       "position, causes"}) + "\n").encode())
        for row in rows:
            group, causes = predict.group_scope(row)
            paths = []
            for construct in row["constructs"]:
                for p in construct["paths"]:
                    paths.append([p["root_kind"], p["root"],
                                  [[s["op"], s["member"], s["type"]] for s in p["steps"]],
                                  p["value"], p["access"],
                                  "call-argument" if p["call_argument"] else construct["kind"],
                                  p["causes"]])
            raw.write((json.dumps({
                "library": row["library"], "file": row["file"], "function": row["function"],
                "status": row["status"], "line": row["line"], "origin": row["origin"],
                "family": row["family"], "condition": row["condition"], "leaves": row["leaves"],
                "scope": group or "refused", "causes": causes,
                "constructs": [c["kind"] + ":" + c["clause"] for c in row["constructs"]],
                "calls": sorted({name for c in row["constructs"] for name in c["calls"]}),
                "paths": paths,
            }, separators=(",", ":")) + "\n").encode())
    records = predict.jsonl(predicted / "prediction.jsonl.gz")
    freed = {"P2": [], "P3": []}
    for record in records:
        for scope in ("P2", "P3"):
            hit = record[scope]["same-unit"]
            cross = record[scope]["cross-unit"]
            if hit is None and cross is None:
                continue
            chosen = hit or cross
            freed[scope].append({
                "library": record["library"], "file": record["file"], "function": record["function"],
                "verdict": chosen["verdict"], "reason": chosen["reason"], "round": chosen["round"],
                "needs_cross_unit_summary": hit is None,
                "obligations": chosen["obligations"],
            })
    for scope in freed:
        freed[scope].sort(key=lambda r: (r["library"], r["file"], r["function"]))
    summary = predict.load(predicted / "prediction-summary.json")
    (artifacts / "prediction.json").write_text(
        json.dumps({"schema": predict.SCHEMA}) + "\n"
        + json.dumps({"summary": summary["scopes"], "freed": freed}, indent=1) + "\n")
    shutil.copyfile(census_json, artifacts / "census.json")
    classes = json.loads(classes_json.read_text())
    (artifacts / "classes.json").write_text(json.dumps({"schema": SCHEMA, "kind": "registered-classes"}) + "\n"
                                            + json.dumps(classes, indent=1) + "\n")
    print({scope: len(v) for scope, v in freed.items()})


if __name__ == "__main__":
    main()
