#!/usr/bin/env python3
"""attribute.py BEFORE AFTER BEFORE_PARITY AFTER_PARITY [--json OUT] -- card
CG10-BOUND-UNTRACKED-ROOTS: attribute every function whose status or leaves
move between two runs.

A moved function is attributed to

* `premise` when an AFTER `bound-untracked-root` row names it whose chain
  resolves with the premise and not without it (rule 11d admitted a root the
  flow refused before; a rule-10k(3) actual left to the flow carries such a
  chain too);
* `deferred-actual` when an AFTER `bound-untracked-root` row names it at a
  line where the BEFORE stream has none: a pointer read rule 10k(3) now leaves
  to the flow, whose refusal (the root is tracked, or the path refused) takes
  the place of the classifier's;
* `callee-summary` when a removed leaf names a callee whose own function moved
  to Safe (a validated summary now in force);
* `read` for a function in the `READ` table below: attributed by reading the
  source, with the reason recorded (a condition or statement calling a callee
  whose body rule 11d admitted, whose leaf names no callee);
* `unattributed` otherwise (a finding to read).

Prints the counts and the status moves; writes every row to OUT.
"""
import argparse
import collections
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent

# Attributed by reading (corpus AFTER at f477d6c0). Each names the callee whose
# body rule 11d now admits, so its validated summary is in force at the call.
READ = {
    "flb_input_chunk_drop_task_route": "callee-summary: the branch condition "
    "`flb_task_get_route_data(task, output_plugin, &route_records, &route_bytes) == 0` "
    "(flb_input_chunk.c:883) calls the static inline flb_task_get_route_data, whose "
    "`*records`/`*bytes` stores rule 11d admits",
    "flb_input_chunk_release_space": "callee-summary: the branch condition "
    "`if (flb_input_chunk_drop_task_route(old_input_chunk->task, ...))` "
    "(flb_input_chunk.c:607) calls flb_input_chunk_drop_task_route, now Safe",
    "rd_kafka_handle_Produce_metadata_update": "callee-summary: "
    "`rd_kafkap_leader_discovery_metadata_init(mdi, nodeid)` (rdkafka_request.c:3900), "
    "whose body rule 11d admits (9 lifted rows), no longer refuses `mdi`",
    # System route (flb_engine.c, flb_engine_dispatch.c, flb_output.c).
    "handle_output_event": "callee-summary: the branch condition "
    "`flb_task_get_route_data(task, ins, &effective_records, ...)` (flb_engine.c:625) "
    "calls the static inline body rule 11d admits",
    "record_retry_failure_metrics": "callee-summary: the branch condition "
    "`flb_task_get_route_data(task, output, ...) != 0 && ...` (flb_engine_dispatch.c:54) "
    "is admitted, so the refused statement no longer poisons `task`'s binding; its reads "
    "at :58-59 resolve",
    "flb_output_task_flush_error": "callee-summary: `flb_task_set_route_data(task, "
    "out_ins, task->event_chunk->total_events, ...)` (flb_output.c:439), whose static "
    "inline body rule 11d admits, is contracted, so its actuals are read",
    "mk_fifo_worker_read": "callee-summary then deferred-actual: `mk_fifo_queue_get` "
    "(now Safe) is contracted at mk_fifo.c:443, and its actual `fw->fifo`, left to the "
    "flow by rule 10k(3), is refused there (`fw`'s path was refused)",
}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("before_parity")
    parser.add_argument("parity")
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    sys.path.insert(0, str(HERE))
    import compare  # noqa: E402

    _, before = compare.load(pathlib.Path(args.before))
    _, after = compare.load(pathlib.Path(args.after))
    lifted = collections.defaultdict(set)
    with open(args.parity) as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("position") == "bound-untracked-root" and row["resolves_with"] \
                    and not row["resolves_without"]:
                lifted[row["function"]].add(row["root"])
    def root_lines(path):
        lines = collections.defaultdict(set)
        with open(path) as stream:
            for line in stream:
                row = json.loads(line)
                if row.get("position") == "bound-untracked-root":
                    lines[row["function"]].add(row["line"])
        return lines

    before_lines = root_lines(args.before_parity)
    after_lines = root_lines(args.parity)
    safe_now = {name for (unit, name), (status, _) in after.items()
                if status == "Safe" and before.get((unit, name), ("absent",))[0] != "Safe"}
    rows = []
    for key in sorted(set(before) | set(after), key=str):
        b_status, b_leaves = before.get(key, ("absent", collections.Counter()))
        a_status, a_leaves = after.get(key, ("absent", collections.Counter()))
        if b_status == a_status and b_leaves == a_leaves:
            continue
        removed = b_leaves - a_leaves
        added = a_leaves - b_leaves
        unit, name = key
        if name in lifted:
            cause = "premise"
        elif after_lines.get(name, set()) - before_lines.get(name, set()):
            cause = "deferred-actual"
        elif any(f"`{callee}`" in (label[1] or "") for label in removed
                 for callee in safe_now | set(lifted)):
            cause = "callee-summary"
        elif name in READ:
            cause = "read"
        else:
            cause = "unattributed"
        rows.append({
            "unit": unit,
            "function": name,
            "status": [b_status, a_status],
            "cause": cause,
            "roots": sorted(lifted.get(name, [])),
            "reading": READ.get(name),
            "removed": {f"{k[0]}: {k[1]}": v for k, v in removed.items()},
            "added": {f"{k[0]}: {k[1]}": v for k, v in added.items()},
        })
    summary = {
        "functions": len(rows),
        "by_cause": dict(collections.Counter(row["cause"] for row in rows)),
        "status_moves": [row for row in rows if row["status"][0] != row["status"][1]],
        "unattributed": [row for row in rows if row["cause"] == "unattributed"],
    }
    print(json.dumps(summary, indent=1, ensure_ascii=False))
    if args.json:
        args.json.write_text(json.dumps({"summary": summary, "rows": rows}, indent=1,
                                        ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
