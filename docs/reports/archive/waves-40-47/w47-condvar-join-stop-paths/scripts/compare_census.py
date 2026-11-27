#!/usr/bin/env python3
"""compare_census.py BEFORE.json AFTER.json OUT.json -- attribute every
condvar/join census change (card W47-CONDVAR-JOIN-STOP-PATHS).

Keys: a C5 release by (root, file, line, what); a C4 site by (file, line,
function, callee, kind, object). For each key present in either run it
records the verdict before and after and the reason (or the first reason),
and classifies the change:

* `verdict`  -- the verdict label moved (a Safe-direction move would be a
  discharge; the report names every one);
* `reason`   -- same verdict, reworded reason;
* `evidence` -- same verdict and reason, the per-path join evidence changed;
* `added` / `removed` -- the key is in one run only.

Tallies of both runs are copied, and every release whose evidence lost the
`!ctx->config` path (`returned from flb_stop at src/flb_lib.c:1063`) or gained
a repeated join is counted. Read-only; standard library only.
"""
import json
import sys
from collections import Counter


def label(result):
    return result["verdict"] if result["verdict"] != "refused" else "refused (%s)" % result["refusal"]


def reason(result):
    if "reason" in result:
        return result["reason"]
    if "reasons" in result:
        return result["reasons"][0] if result["reasons"] else ""
    return ""


def load(path):
    with open(path) as handle:
        return json.load(handle)


def main():
    before, after, out = sys.argv[1:4]
    old, new = load(before), load(after)

    def releases(run):
        return {
            (r["root"], r["file"], r["line"], r["what"]): r for r in run["releases"]
        }

    def sites(run):
        return {
            (s["file"], s["line"], s["function"], s["callee"], s["kind"], s["object"] or ""): s
            for s in run["sites"]
        }

    changes = {"releases": [], "sites": []}
    counts = {"releases": Counter(), "sites": Counter()}
    moves = Counter()
    for kind, a, b in (("releases", releases(old), releases(new)), ("sites", sites(old), sites(new))):
        for key in sorted(set(a) | set(b), key=lambda k: tuple(map(str, k))):
            x, y = a.get(key), b.get(key)
            if x is None or y is None:
                counts[kind]["added" if x is None else "removed"] += 1
                changes[kind].append({"key": key, "change": "added" if x is None else "removed"})
                continue
            lx, ly = label(x["result"]), label(y["result"])
            rx, ry = reason(x["result"]), reason(y["result"])
            if lx != ly:
                change = "verdict"
                moves["%s -> %s" % (lx, ly)] += 1
            elif rx != ry:
                change = "reason"
            elif kind == "releases" and x.get("live") != y.get("live"):
                change = "evidence"
            else:
                counts[kind]["unchanged"] += 1
                continue
            counts[kind][change] += 1
            entry = {
                "key": key,
                "change": change,
                "before": {"verdict": lx, "reason": rx},
                "after": {"verdict": ly, "reason": ry},
            }
            if kind == "releases":
                gone = sorted(set(x["live"].get("flb_lib_worker", [])) - set(y["live"].get("flb_lib_worker", [])))
                came = sorted(set(y["live"].get("flb_lib_worker", [])) - set(x["live"].get("flb_lib_worker", [])))
                entry["flb_lib_worker_paths_removed"] = gone
                entry["flb_lib_worker_paths_added"] = came
            changes[kind].append(entry)
    config_path = "returned from `flb_stop` at src/flb_lib.c:1063 without joining"
    lost_config = sum(
        1
        for key, x in releases(old).items()
        if any(config_path in p for paths in x["live"].values() for p in paths)
        and key in releases(new)
        and not any(config_path in p for paths in releases(new)[key]["live"].values() for p in paths)
    )
    kept_config = sum(
        1
        for x in new["releases"]
        if any(config_path in p for paths in x["live"].values() for p in paths)
    )
    repeated = sum(
        1 for x in new["releases"] if any(p.startswith("Repeated") for paths in x["live"].values() for p in paths)
    )
    report = {
        "before_tally": old["tally"],
        "after_tally": new["tally"],
        "counts": {k: dict(v) for k, v in counts.items()},
        "verdict_moves": dict(moves),
        "releases_that_lost_the_config_branch": lost_config,
        "releases_that_keep_the_config_branch": kept_config,
        "releases_with_a_repeated_join": repeated,
        "null_tests": len(new.get("null_tests", [])),
        "null_tests_excluded": [
            "%s:%s %s" % (t["file"], t["line"], t["test"])
            for t in new.get("null_tests", [])
            if t["excluded"].startswith("Ok")
        ],
        "changes": changes,
    }
    with open(out, "w") as handle:
        json.dump(report, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({k: report[k] for k in report if k != "changes"}, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
