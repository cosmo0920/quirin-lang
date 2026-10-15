#!/usr/bin/env python3
"""table.py [MODULES] -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: render the census
(`artifacts/census.json`) as Markdown rows for the report: per declaration
without the premise, its definition, the decision and its ground (the
shortest path to the log callback, the other rk_conf callbacks and
interceptor methods reached, or why none is reachable). MODULES is a comma
list (default: the five custody modules)."""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
CENSUS = HERE.parent / "artifacts/census.json"
DEFAULT = "client.qv,refcount.qv,typed_queue.qv,queue_destructor.qv,topic_partition.qv"


def ground(row):
    a = row["66910c10"]
    if not a.get("defined"):
        return "no C definition: a macro (`rd_refcnt_add0` is `rd_atomic32_add`, rd.h:351) or the model's abstract destructor"
    if row["class"] == "rk-conf":
        avoiding, sinks = a["rk_conf_sinks_avoiding_rd_kafka_crash"], a["rk_conf_sinks"]
        table = avoiding or sinks
        sink = "rk-conf:log_cb" if "rk-conf:log_cb" in table else next(iter(table))
        path = table[sink]
        call = "conf->log_cb" if sink == "rk-conf:log_cb" else sink.split(":", 1)[1]
        crash = "" if avoiding else " (only through `rd_kafka_crash`'s dump)"
        others = sorted(k.split(":", 1)[1] for k in sinks if k != sink)
        text = f"`{' → '.join(path)}` → `{call}`{crash}"
        if others:
            text += "; also " + ", ".join(f"`{o}`" for o in others)
        return text
    parts = [f"{a['reachable_functions']} function(s) reached, no rk_conf or interceptor call"]
    for call, r in a.get("resolved", {}).items():
        cands = ", ".join(f"`{c}`" for c in r["candidates"]) or "none here"
        parts.append(f"`{call}` resolved to {cands}: {r['reason']}")
    return "; ".join(parts)


def main():
    modules = (sys.argv[1] if len(sys.argv) > 1 else DEFAULT).split(",")
    rows = json.loads(CENSUS.read_text())["rows"]
    print("| Module | Declaration | Definition | Class | Premise | Ground |")
    print("| --- | --- | --- | --- | --- | --- |")
    for row in rows:
        if row["module"] not in modules or row["has_premise"]:
            continue
        a = row["66910c10"]
        site = ":".join(map(str, a["sites"][0][:2])).replace("lib/librdkafka-2.15.0/src/", "") if a.get("defined") else "—"
        give = "yes" if row["class"] in ("rk-conf", "indirect") else "no"
        print(f"| `{row['module']}` | `{row['function']}` | `{site}` | {row['class']} | {give} | {ground(row)} |")


main()
