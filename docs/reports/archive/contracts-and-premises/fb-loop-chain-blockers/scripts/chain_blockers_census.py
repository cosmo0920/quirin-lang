#!/usr/bin/env python3
"""Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: the access and identity
blockers on the event-loop registration and teardown chains.

Read-only. For each route report (system, slice), every row whose function is
on a chain the FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME census listed (its
README §1.4 R1-R6, the collector setters and the loop-operation functions of
its loop-member census) is listed with each

* access leaf (`indirect memory access is outside the resource projection`,
  or a nested or unmodeled call, as the ledger entry counts them), and
* `unresolved resource identity `NAME`` leaf,

with the leaf's original coordinate (`origin`), the source line at that
coordinate in the prepared tree, and, when a census stream of an
`indirect-census` build is given, the analyzer's own token view of the
refused statement (its chains: base, base kind, tag, member class, steps).

The classification (a)-(d) is applied by `classify.py` from the hand-read
table in that file; this script only gathers.

usage: chain_blockers_census.py ROUTE REPORT SOURCE_ROOT OUT.json [CENSUS.jsonl]
"""
import collections
import hashlib
import json
import sys

INDIRECT = "indirect memory access is outside the resource projection"
IDENTITY = "unresolved resource identity `"

# chain -> functions: exactly the ledger entry's scope, the chain list of
# FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME's scripts/chain_blockers.py (its
# README §1.4 R1-R6 and §7), so the sizes here compare with the entry's.
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


def sha256(path):
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def main():
    route, report, root, out = sys.argv[1:5]
    census_path = sys.argv[5] if len(sys.argv) > 5 else None
    document = json.load(open(report))
    census = collections.defaultdict(list)
    if census_path:
        with open(census_path) as handle:
            for raw in handle:
                record = json.loads(raw)
                census[(record["function"], record["line"])].append(record)
    sources = {}

    def text_at(origin):
        path, _, line = origin.rpartition(":")
        if path not in sources:
            try:
                with open(f"{root}/{path}", encoding="utf-8", errors="replace") as handle:
                    sources[path] = handle.read().split("\n")
            except OSError:
                sources[path] = None
        lines = sources[path]
        if lines is None or not line.isdigit() or int(line) < 1 or int(line) > len(lines):
            return None
        return lines[int(line) - 1].strip()

    rows = []
    for unit_index, unit in enumerate(document["units"]):
        for function in unit["functions"]:
            name = function["function"]
            if name not in CHAIN_OF:
                continue
            leaves = []
            others = collections.Counter()
            for leaf in function["diagnostics"]:
                description = leaf["description"]
                if description == INDIRECT or description.startswith(
                        ("nested call", "unmodeled call")):
                    kind = "access"
                elif description.startswith(IDENTITY):
                    kind = "identity"
                else:
                    others[description] += 1
                    continue
                entry = {
                    "kind": kind,
                    "line": leaf["line"],
                    "origin": leaf.get("origin"),
                    "source": text_at(leaf["origin"]) if leaf.get("origin") else None,
                }
                if kind == "identity":
                    entry["name"] = description[len(IDENTITY):-1]
                views = census.get((name, leaf["line"]), [])
                if kind == "access" and views:
                    view = views[0]
                    entry["census"] = {
                        "site": view["site"],
                        "position": view["position"],
                        "call_contracted": view["call_contracted"],
                        "statement": " ".join(view["texts"]),
                        "chains": [
                            {k: c[k] for k in ("base", "base_kind", "tag", "steps",
                                               "member_class", "member", "base_cause",
                                               "tag_lost_step", "base_never_bound")}
                            for c in view["chains"]
                        ],
                    }
                leaves.append(entry)
            rows.append({
                "unit_index": unit_index,
                "unit": unit["file"],
                "function": name,
                "chain": CHAIN_OF[name],
                "status": function["status"],
                "access_leaves": sum(1 for l in leaves if l["kind"] == "access"),
                "identity_leaves": sum(1 for l in leaves if l["kind"] == "identity"),
                "other_leaves": sum(others.values()),
                "other": dict(sorted(others.items())),
                "leaves": leaves,
            })
    found = {r["function"] for r in rows}
    result = {
        "card": "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS",
        "route": route,
        "report": report,
        "report_sha256": sha256(report),
        "census_stream": census_path,
        "census_sha256": sha256(census_path) if census_path else None,
        "chains": CHAINS,
        "chain_functions_absent_from_route": sorted(set(CHAIN_OF) - found),
        "totals": {
            "rows": len(rows),
            "rows_with_access_or_identity": sum(
                1 for r in rows if r["access_leaves"] or r["identity_leaves"]),
            "access_leaves": sum(r["access_leaves"] for r in rows),
            "identity_leaves": sum(r["identity_leaves"] for r in rows),
            "status": dict(collections.Counter(r["status"] for r in rows)),
        },
        "rows": rows,
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(json.dumps(result["totals"]))
    print("absent:", result["chain_functions_absent_from_route"])


if __name__ == "__main__":
    main()
