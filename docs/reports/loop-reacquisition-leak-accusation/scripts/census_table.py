#!/usr/bin/env python3
"""Card LOOP-REACQUISITION-LEAK-ACCUSATION: the per-generation census table.

Reads a QCORE_LEAK_WITNESS stream written by the card's census binary and
prints, per reported leak of the named functions, which generations of the
acquisition site the settled exit still reports (live, not exempt, no
unresolved effect), with the witness path's re-acquisitions and whether the
site's re-acquisition is refused. A leak is a generation join exactly when no
current-generation witness is settled and some earlier generation's is.

Usage: census_table.py LEAK_WITNESS.jsonl FUNCTION [FUNCTION ...]
           [--report resource-diagnostics.json] [--json OUT]
"""
import json
import sys


def option(argv, name):
    if name not in argv:
        return None, argv
    index = argv.index(name)
    return argv[index + 1], argv[:index] + argv[index + 2:]


def origins(report_path):
    """Map (function, unit line) to the source origin the report prints."""
    mapping = {}
    if not report_path:
        return mapping
    with open(report_path) as handle:
        report = json.load(handle)
    for unit in report.get("units", []):
        for function in unit.get("functions", []):
            for diagnostic in function.get("diagnostics", []):
                mapping[(function["function"], diagnostic["line"])] = diagnostic.get("origin")
    return mapping


def main(argv):
    out, argv = option(argv, "--json")
    report, argv = option(argv, "--report")
    origin = origins(report)
    stream, functions = argv[0], set(argv[1:])
    rows = {}
    with open(stream) as handle:
        for raw in handle:
            witness = json.loads(raw)
            if witness.get("record") or witness.get("function") not in functions:
                continue
            key = (witness["function"], witness["line"], witness["acquisition_line"])
            entry = rows.setdefault(key, {})
            generation = witness["generation"]
            path = witness.get("path")
            entry.setdefault(generation, []).append({
                "invocation": [witness["unit_seq"], witness["invocation"]],
                "settled": witness["settled"],
                "status": witness["status"],
                "unresolved": witness["unresolved"],
                "reacquisition_refused": witness["reacquisition_refused"],
                "path": path is not None,
                "path_lines": [step["line"] for step in path] if path else None,
                "path_reacquisitions": witness.get("path_reacquisitions"),
                "exit_unmodeled": witness["exit_unmodeled"],
            })
    table = []
    for (function, line, acquisition_line), entry in sorted(rows.items()):
        # A generation has a witness when, in some invocation, the settled
        # exit still reports it or a path keeps it live from its producer.
        witnessed = lambda g: any(e["settled"] or e["path"] for e in entry.get(g, []))
        current = witnessed("current")
        earlier = [g for g in ("previous", "older") if witnessed(g)]
        settled_any = any(e["settled"] for records in entry.values() for e in records)
        join = not current and bool(earlier)
        refused = any(e["reacquisition_refused"] for records in entry.values() for e in records)
        statuses = {g: sorted({(e["status"], e["unresolved"]) for e in records})
                    for g, records in entry.items()}
        table.append({
            "function": function,
            "line": line,
            "origin": origin.get((function, line)),
            "acquisition_line": acquisition_line,
            "current_witness": current,
            "earlier_witnesses": earlier,
            "settled_exit_reports_it": settled_any,
            "reacquisition_refused": refused,
            "generation_join": join,
            "withdrawn_by_rule_42": join and refused,
            "statuses": statuses,
            "generations": entry,
        })
        print(
            f"{function} [{origin.get((function, line))}]: "
            f"current witness={current} earlier={earlier or '-'} "
            f"settled reproduces={settled_any} refused={refused} "
            f"statuses={statuses} "
            f"=> {'generation join' if join else 'not a join'}"
        )
    if out:
        with open(out, "w") as handle:
            json.dump(table, handle, indent=1)
            handle.write("\n")


if __name__ == "__main__":
    main(sys.argv[1:])
