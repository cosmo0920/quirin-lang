#!/usr/bin/env python3
"""compare.py BEFORE_DIR BEFORE_NAME AFTER_DIR AFTER_NAME OUT.json -- OUTER-CONTRACT:
the static identity and the runtime projection of one measurement.

BEFORE is main's binary (OUTER-CONTRACT-DESIGN's census run, private/ocd/main*,
no projection); AFTER is this card's binary with the null property and the
runtime projection (private/oc/NAME*). Per route (corpus, system, slice):

* **Custody report.** BEFORE and AFTER `resource-diagnostics.json`: the digest
  of each, and whether they are equal once every row's new dependency keys
  (`run_set_members`, `stated_members`) are removed; how many rows carry each
  key; every row's `status` compared (must be identical).
* **Contract sidecar.** Equal byte for byte, or the keys that differ.
* **Null rows.** The JSON lines byte-identical, and the summary equal once the
  bounds selection record's load-path digest is removed (the load-path
  manifest gained the outer views).
* **Projection.** The static tallies against the report, every scenario's
  tallies and moves, and every moved row.

Writes OUT.json and prints the tallies."""
import collections
import hashlib
import json
import pathlib
import sys

SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
NEW_KEYS = ("run_set_members", "stated_members")


def paths(directory, name, route):
    directory = pathlib.Path(directory)
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    if route == "slice":
        tag = "ocd" if directory.name == "ocd" else "oc"
        report = SLICE / f"{tag}-{name}" / "resource-diagnostics.json"
    else:
        report = directory / (name if route == "corpus" else f"{name}-system") / "resource-diagnostics.json"
    return {
        "report": report,
        "sidecar": report.with_suffix(".contracts.json"),
        "null": directory / f"{name}{suffix}-null.jsonl",
        "summary": directory / f"{name}{suffix}-null.summary.json",
        "outer": directory / f"{name}{suffix}-outer.json",
    }


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def strip_report(report):
    counts = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            deps = function.get("contract_dependencies")
            if isinstance(deps, dict):
                for key in NEW_KEYS:
                    if key in deps:
                        counts[key] += 1
                        del deps[key]
    return counts


def statuses(report):
    return [(u["file"], f["function"], f["status"]) for u in report["units"] for f in u["functions"]]


def strip_summary(summary):
    summary = json.loads(json.dumps(summary))
    def walk(value):
        if isinstance(value, dict):
            value.pop("load_path_sha256", None)
            for v in value.values():
                walk(v)
        elif isinstance(value, list):
            for v in value:
                walk(v)
    walk(summary)
    return summary


def route(before_dir, before_name, after_dir, after_name, name):
    b, a = paths(before_dir, before_name, name), paths(after_dir, after_name, name)
    out = {"before": {k: str(v) for k, v in b.items()}, "after": {k: str(v) for k, v in a.items()}}
    br, ar = json.loads(b["report"].read_text()), json.loads(a["report"].read_text())
    out["report_sha256"] = {"before": sha(b["report"]), "after": sha(a["report"])}
    out["statuses_identical"] = statuses(br) == statuses(ar)
    out["rows_with_new_keys"] = dict(strip_report(ar))
    strip_report(br)
    out["report_identical_without_new_keys"] = br == ar
    bs, as_ = json.loads(b["sidecar"].read_text()), json.loads(a["sidecar"].read_text())
    out["sidecar_identical"] = bs == as_
    out["sidecar_differing_keys"] = sorted(k for k in set(bs) | set(as_) if bs.get(k) != as_.get(k))
    out["null_sha256"] = {"before": sha(b["null"]), "after": sha(a["null"])}
    out["null_identical"] = b["null"].read_bytes() == a["null"].read_bytes()
    sb, sa = json.loads(b["summary"].read_text()), json.loads(a["summary"].read_text())
    out["null_summary_identical"] = sb == sa
    out["null_summary_identical_without_load_path_digest"] = strip_summary(sb) == strip_summary(sa)
    projection = json.loads(a["outer"].read_text())
    out["projection_sha256"] = sha(a["outer"])
    out["static"] = projection["static"]
    report_tally = collections.Counter(s for _, _, s in statuses(ar))
    out["static_custody_matches_report"] = all(
        projection["static"]["custody"].get(k, 0) == report_tally.get(k, 0)
        for k in ("Safe", "Pending", "Unknown", "Unsafe"))
    out["contracts"] = projection["contracts"]
    out["loaders"] = [{"loader": l["loader"], "pin_hold": l["pin_hold"],
                       "reach": (l.get("reach") or {}).get("reach")} for l in projection["loaders"]]
    out["scenarios"] = {}
    for scenario, value in projection["scenarios"].items():
        out["scenarios"][scenario] = {
            "custody": value["custody"], "null": value["null"], "combined": value["combined"],
            "moves": value["moves"], "moved": value["moved"],
        }
    return out


def main():
    before_dir, before_name, after_dir, after_name, out = sys.argv[1:6]
    result = {r: route(before_dir, before_name, after_dir, after_name, r)
              for r in ("corpus", "system", "slice")}
    pathlib.Path(out).write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    for r, v in result.items():
        print(r, json.dumps({k: v[k] for k in (
            "statuses_identical", "report_identical_without_new_keys", "rows_with_new_keys",
            "sidecar_identical", "sidecar_differing_keys", "null_identical",
            "null_summary_identical", "null_summary_identical_without_load_path_digest",
            "static_custody_matches_report")}))
        print("  static", json.dumps(v["static"]))
        for s, sv in v["scenarios"].items():
            print("  ", s, json.dumps({k: sv[k] for k in ("custody", "null", "combined", "moves")}))


if __name__ == "__main__":
    main()
