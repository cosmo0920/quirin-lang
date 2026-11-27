#!/usr/bin/env python3
"""FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME: chain_blockers.py OUT.json ROUTE=REPORT...

Read-only. Sizes the ledger entry FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS
(USER DECISION 2026-10-17 D6, "Blockers first"): for every function of the
registration and teardown chains R1-R6 (README section 1.4) on each route,
its status and its leaves by class:

* `access`    the access gate and projection refusals: "indirect memory
              access is outside the resource projection", "nested call ...
              inside a modeled call", "unmodeled call, aggregate or indirect
              access in ...";
* `identity`  "unresolved resource identity ...";
* `callee`    "call to uncontracted function ..." and other callee refusals
              (declared-obligation and review refusals included);
* `lifetime`  rule 12t's retention leaves;
* `other`     everything else.

A row whose leaves are all `access` / `identity` is one the blockers card
alone could free; a lifetime model can show movement on a row only after its
`access` and `identity` leaves are cleared or named. Nothing is piped.
"""
import collections
import gzip
import json
import pathlib
import re
import sys

CHAINS = {
    "R1/R2 collectors and instances": [
        "collector_create", "collector_start", "flb_input_set_collector_time",
        "flb_input_set_collector_event", "flb_input_set_collector_socket",
        "flb_input_collector_start", "flb_input_collectors_signal_start",
        "flb_input_collectors_start", "flb_input_thread_collectors_start",
        "flb_input_collector_pause", "flb_input_collector_resume",
        "flb_input_collector_delete", "flb_input_collector_destroy",
        "flb_input_instance_destroy", "flb_input_instance_exit", "flb_input_exit_all",
        "flb_input_thread_instance_exit", "flb_input_thread_instance_init",
        "input_thread", "input_thread_instance_create", "input_thread_instance_destroy",
        "input_instance_channel_events_init"],
    "R3 engine and library": [
        "flb_engine_start", "flb_engine_shutdown", "flb_create", "flb_destroy",
        "flb_stop", "flb_config_exit"],
    "R4 output workers": ["flb_output_thread_pool_create", "flb_output_thread_pool_destroy",
                          "output_thread"],
    "R6 scheduler": [
        "flb_sched_create", "flb_sched_destroy", "flb_sched_timer_create",
        "flb_sched_timer_cb_create", "flb_sched_timer_coro_cb_create",
        "flb_sched_timer_cb_disable", "flb_sched_timer_cb_destroy", "flb_sched_timer_destroy",
        "flb_sched_timer_invalidate", "flb_sched_timer_cleanup", "flb_sched_request_create",
        "flb_sched_request_destroy", "flb_sched_request_invalidate", "schedule_request_now",
        "flb_sched_retry_now", "flb_task_retry_destroy"],
}
CHAIN_OF = {f: c for c, fs in CHAINS.items() for f in fs}


def classify(description):
    d = description or ""
    if re.search(r"retention|retained", d):
        return "lifetime"
    if re.search(r"^indirect memory access|^nested call|^unmodeled call", d):
        return "access"
    if d.startswith("unresolved resource identity"):
        return "identity"
    if re.search(r"^call to ", d):
        return "callee"
    return "other"


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        gz = path / "resource-diagnostics.json.gz"
        path = gz if gz.exists() else path / "resource-diagnostics.json"
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main():
    out = sys.argv[1]
    result = {"chains": CHAINS, "routes": {}}
    for spec in sys.argv[2:]:
        route, report = spec.split("=", 1)
        data = load(report)
        rows = []
        seen = collections.Counter()
        for unit in data["units"]:
            ordinal = seen[unit["file"]]
            seen[unit["file"]] += 1
            name = unit["file"] if ordinal == 0 else f"{unit['file']}#{ordinal}"
            for function in unit.get("functions", []):
                if function["function"] not in CHAIN_OF:
                    continue
                counts = collections.Counter(classify(d.get("description"))
                                             for d in function.get("diagnostics", []))
                first = sorted(function.get("diagnostics", []), key=lambda d: d.get("line") or 0)
                rows.append({
                    "unit": name, "function": function["function"],
                    "chain": CHAIN_OF[function["function"]], "status": function.get("status"),
                    "leaves": dict(counts),
                    "blockers": counts["access"] + counts["identity"],
                    "only_blockers": bool(counts) and set(counts) <= {"access", "identity"},
                    "first_leaf": (first[0].get("description") if first else None),
                })
        rows.sort(key=lambda r: (r["chain"], r["unit"], r["function"]))
        units_ordinal0 = [r for r in rows if "#" not in r["unit"]]
        result["routes"][route] = {
            "rows": rows,
            "summary": {
                "rows": len(rows),
                "rows_first_configuration": len(units_ordinal0),
                "rows_with_blockers": sum(1 for r in rows if r["blockers"]),
                "rows_only_blockers": sum(1 for r in rows if r["only_blockers"]),
                "access_leaves": sum(r["leaves"].get("access", 0) for r in rows),
                "identity_leaves": sum(r["leaves"].get("identity", 0) for r in rows),
                "statuses": dict(collections.Counter(r["status"] for r in rows)),
            },
        }
        print(route, json.dumps(result["routes"][route]["summary"]))
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, ensure_ascii=False)
        f.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
