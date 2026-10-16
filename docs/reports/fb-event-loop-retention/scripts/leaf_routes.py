#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: attribute each P5a leaf site of
FB-QUANT-CONTRACT-CENSUS (artifacts/patterns.json, binary 7bf6e937 from
d708641bc, whose system and slice figures equal round 30's) to the operation
on its line and to what would decide it under this card. Read-only: the
prepared exports and the census card's artifact. Output:
artifacts/leaf-routes.json.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
CENSUS = os.path.join(REPORT, "..", "fb-quant-contract-census", "artifacts", "patterns.json")
ROOTS = {
    "system": ("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/"
               "w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"),
    "slice": ("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/"
              "fluent-bit-clang-source"),
}
# What decides a call of each operation under this card (the census's P5a
# operations), keyed by callee.
DECIDER = {
    "mk_event_add": "A registration: retained_by(data, loop); the holder actual is a path",
    "mk_event_timeout_create": "A registration: retained_by(data, loop); the holder actual is a path",
    "mk_event_channel_create": "A registration: retained_by(data, loop) beside two out fds",
    "mk_event_del": "B removal of one registration: needs a removal word",
    "mk_event_timeout_destroy": "B removal of one registration: needs a removal word",
    "mk_event_channel_destroy": "B removal of one registration (and two closes): needs a removal word",
    "mk_event_wait": "C plain: borrowed loop",
    "mk_event_wait_2": "C plain: borrowed loop",
    "mk_event_foreach": "C macro over loop->events (a member read, not a call)",
    "flb_input_set_collector_time": "D registration forwarded through an id",
    "flb_input_set_collector_event": "D registration forwarded through an id",
    "flb_input_set_collector_socket": "D registration forwarded through an id",
    "flb_sched_timer_cb_create": "E timer: retains data in a timer linked into sched, registers cb",
    "flb_input_collector_start": "F start by id: retains a collector reached by id (no parameter names it)",
    "flb_input_collector_pause": "F by id",
    "flb_input_collector_resume": "F by id",
    "flb_input_collector_delete": "F by id",
    "flb_input_collector_fd": "G dispatcher: invokes cb_collect (P1, rule 10p)",
    "flb_input_pause": "H public pause: dispatches a plugin callback (rule 10j undeclarable; P1)",
    "flb_input_resume": "H public resume: dispatches a plugin callback (rule 10j undeclarable; P1)",
    "flb_input_pause_all": "H public pause: dispatches plugin callbacks (rule 10j undeclarable; P1)",
}
CALL = re.compile(r"\b(" + "|".join(sorted(DECIDER, key=len, reverse=True)) + r")\s*\(")


def line(root, site):
    path, _, n = site.rpartition(":")
    with open(os.path.join(root, path), encoding="utf-8", errors="replace") as f:
        lines = f.read().split("\n")
    return lines[int(n) - 1]


def main():
    census = json.load(open(CENSUS))["P5a-event-loop-registration"]
    out = {}
    for route in ("system", "slice"):
        root = ROOTS[route]
        seen = set()
        rows = []
        tally = {}
        for row in census[route]["rows"]:
            if row["row"] in seen:
                continue
            seen.add(row["row"])
            sites = []
            for site in row["sites"]:
                text = line(root, site)
                callees = CALL.findall(text) or ["(member actual or other)"]
                for callee in callees:
                    decider = DECIDER.get(callee, "I other: an indirect access at a member actual")
                    tally[decider[0]] = tally.get(decider[0], 0) + 1
                    sites.append({"site": site, "callee": callee, "decider": decider,
                                  "text": " ".join(text.split())[:140]})
            rows.append({"row": row["row"], "status": row["status"], "leaves": row["leaves"],
                         "pattern_leaves": row["pattern_leaves"], "sites": sites})
        out[route] = {"rows": rows, "sites_by_decider_class": dict(sorted(tally.items())),
                      "alone": sorted({r if isinstance(r, str) else r["row"] for r in census[route]["rows_alone"]})}
    dest = os.path.join(REPORT, "artifacts", "leaf-routes.json")
    with open(dest, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
        f.write("\n")
    for route, data in out.items():
        print(route, data["sites_by_decider_class"])
        for r in data["rows"]:
            print("  ", r["row"], r["leaves"], [(s["site"].rsplit(":", 1)[1], s["callee"], s["decider"][0]) for s in r["sites"]])
    return 0


if __name__ == "__main__":
    sys.exit(main())
