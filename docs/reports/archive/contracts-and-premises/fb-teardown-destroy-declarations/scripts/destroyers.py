#!/usr/bin/env python3
"""destroyers.py ROUTE REPORT_JSON SOURCE_ROOT OUT_JSON [UNIT] -- FB-TEARDOWN-DESTROY-DECLARATIONS.

The destroyer census on one measured route: every `call to uncontracted
function NAME` leaf whose NAME is a teardown callee of the census
(FB-QUANT-CONTRACT-CENSUS's P3 list, scripts/classify.py `DESTROY`), and every
`unresolved resource identity` leaf on a line calling one, with the row it
stands in, the row's status and leaf count, whether every leaf of the row is
a destroyer leaf ("alone"), and the source line (read from the prepared
export, read only). Also the per-callee totals: leaves, rows, distinct
functions. A census of where the calls stand, not a proof of what any
declaration decides.
"""
import collections
import json
import re
import sys

DESTROY = re.compile(
    r"\b(flb_input_chunk_destroy|flb_task_destroy|flb_task_retry_destroy|flb_input_instance_destroy|"
    r"flb_output_instance_destroy|flb_upstream_conn_release|flb_upstream_destroy|flb_event_chunk_destroy|"
    r"flb_sched_request_destroy|flb_sched_timer_destroy|flb_output_flush_destroy|flb_input_coro_destroy|"
    r"flb_coro_destroy|flb_input_collector_destroy|flb_input_collectors_signal_exit|flb_task_retry_clean|"
    r"flb_task_queue_destroy|flb_input_chunk_release_route|flb_task_users_release|flb_task_retry_remove|"
    r"flb_sched_request_invalidate|flb_input_exit_all|flb_output_exit|flb_engine_destroy_tasks|"
    r"flb_tail_file_remove|flb_tail_file_remove_all|flb_tail_config_destroy|flb_input_thread_instance_exit)\s*\(")
UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")
UNRESOLVED = re.compile(r"unresolved resource identity `([^`]+)`")


def main():
    route, report, root, out_path = sys.argv[1:5]
    only = sys.argv[5] if len(sys.argv) > 5 else None
    cache = {}

    def line(origin):
        if not origin or ":" not in origin:
            return ""
        path, _, n = origin.rpartition(":")
        if path not in cache:
            try:
                cache[path] = open(f"{root}/{path}", encoding="utf-8", errors="replace").read().split("\n")
            except OSError:
                cache[path] = []
        lines = cache[path]
        n = int(n) if n.isdigit() else 0
        return lines[n - 1] if 0 < n <= len(lines) else ""

    rep = json.load(open(report))
    rows = []
    per_callee = collections.defaultdict(lambda: {"leaves": 0, "rows": set(), "functions": set(),
                                                  "unresolved_leaves": 0})
    for unit in rep["units"]:
        if only and unit["file"] != only:
            continue
        for function in unit["functions"]:
            diagnostics = function.get("diagnostics", []) or []
            mine = []
            for d in diagnostics:
                desc = d.get("description", "") or ""
                text = line(d.get("origin"))
                match = UNCONTRACTED.search(desc)
                callee = None
                kind = None
                if match and DESTROY.search(match.group(1) + "("):
                    callee, kind = match.group(1), "uncontracted"
                elif UNRESOLVED.search(desc) and DESTROY.search(text):
                    callee, kind = DESTROY.search(text).group(1), "unresolved"
                if callee:
                    mine.append({"callee": callee, "kind": kind, "origin": d.get("origin"),
                                 "description": desc, "source": text.strip()[:160]})
                    entry = per_callee[callee]
                    entry["leaves" if kind == "uncontracted" else "unresolved_leaves"] += 1
                    entry["rows"].add(f"{unit['file']}::{function['function']}")
                    entry["functions"].add(function["function"])
            if mine and function["status"] != "Safe":
                rows.append({"unit": unit["file"], "function": function["function"],
                             "status": function["status"], "leaves": len(diagnostics),
                             "destroyer_leaves": len(mine), "alone": len(mine) == len(diagnostics),
                             "sites": mine})
    out = {
        "route": route, "report": report, "source_root": root,
        "callees": {name: {"uncontracted_leaves": v["leaves"], "unresolved_leaves": v["unresolved_leaves"],
                           "rows": len(v["rows"]), "functions": sorted(v["functions"])}
                    for name, v in sorted(per_callee.items(), key=lambda kv: -kv[1]["leaves"])},
        "rows": rows,
    }
    json.dump(out, open(out_path, "w"), indent=1)
    for name, v in out["callees"].items():
        print(f"{name}: {v['uncontracted_leaves']} uncontracted + {v['unresolved_leaves']} unresolved leaves, "
              f"{v['rows']} rows, functions {', '.join(v['functions'])}")
    print(f"rows {len(rows)}; alone {sum(r['alone'] for r in rows)}: "
          + ", ".join(sorted({r['function'] for r in rows if r['alone']})))


if __name__ == "__main__":
    main()
