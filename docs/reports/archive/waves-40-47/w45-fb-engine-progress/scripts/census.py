#!/usr/bin/env python3
"""W45-FB-ENGINE-PROGRESS: publish the loop census.

Usage: census.py INVENTORY SLICE_REPORT --out census.json [--quirin-out quirin-view.json]

Joins the mechanical inventory (`loop_inventory.py`), the hand classification
(`classification.py`) and a `quirin-audit current` slice report. Fail-closed:
it exits non-zero if any inventory loop has no classification row, if a row
names a loop the inventory lacks, if a row uses an unknown role, class,
premise or expression id, or if a loop's function is absent from the slice
report. Nothing here computes a progress verdict: the Quirin columns are the
resource audit's own status and reasons for the enclosing function.
"""

import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import classification as C  # noqa: E402

SCHEMA = "w45-fb-engine-progress/census/v1"


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def unit_variant(unit):
    words = unit["compile_configuration"]["command"].split()
    output = words[words.index("-o") + 1] if "-o" in words else ""
    return "shared" if "-shared" in output or "fluent-bit-shared" in output else "static"


def quirin_rows(report):
    """(file, function) -> list of per-unit {variant, status, diagnostics}."""
    rows = collections.defaultdict(list)
    for unit in report["units"]:
        variant = unit_variant(unit)
        for function in unit["functions"]:
            rows[(unit["file"], function["function"])].append(
                {"variant": variant, "status": function["status"], "diagnostics": function["diagnostics"]})
    return rows


def origin_line(origin, file):
    if not origin or ":" not in origin:
        return None
    path, line = origin.rsplit(":", 1)
    if not path.endswith(file) and not file.endswith(path):
        return None
    try:
        return int(line)
    except ValueError:
        return None


def quirin_view(entries, file, line, end_line):
    statuses = sorted({(e["variant"], e["status"]) for e in entries})
    first = entries[0]
    counts = collections.Counter(d["category"] for d in first["diagnostics"])
    in_loop = []
    for d in first["diagnostics"]:
        ln = origin_line(d.get("origin"), file)
        if ln is not None and line <= ln <= end_line:
            in_loop.append({"line": ln, "category": d["category"], "description": d["description"]})
    loop_condition = [d for d in in_loop if "loop condition" in d["description"]]
    same = all(sorted((d["category"], d["description"], d.get("origin")) for d in e["diagnostics"]) ==
               sorted((d["category"], d["description"], d.get("origin")) for d in first["diagnostics"]) for e in entries)
    specification = [d["description"] for d in first["diagnostics"] if "declared specification" in d["description"]]
    reasons = collections.Counter(d["description"] for d in first["diagnostics"])
    return {
        "status_by_variant": [{"variant": v, "status": s} for v, s in statuses],
        "variants_identical": same,
        "leaves": dict(sorted(counts.items())),
        "top_reasons": [{"description": k, "count": n} for k, n in reasons.most_common(3)],
        "leaves_inside_loop": len(in_loop),
        "loop_condition_leaves": loop_condition,
        "checked_specification": specification[0] if specification else None,
        "progress_verdict": None,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("inventory", type=pathlib.Path)
    parser.add_argument("report", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    args = parser.parse_args()
    inventory = load(args.inventory)
    report = load(args.report)
    qrows = quirin_rows(report)
    rows = {(f, fn, ln): row for f, fn, ln, row in C.ROWS}
    if len(rows) != len(C.ROWS):
        sys.exit("duplicate classification keys")
    errors = []
    loops = []
    seen = set()
    counter = 0
    for f in inventory["files"]:
        for fn in f["functions"]:
            for loop in fn["loops"]:
                key = (f["file"], fn["function"], loop["line"])
                row = rows.get(key)
                if row is None:
                    errors.append(f"unclassified loop {key}")
                    continue
                seen.add(key)
                for field in ("role", "entry", "cls", "bound", "premises", "progress", "express"):
                    if field not in row:
                        errors.append(f"{key}: missing {field}")
                if row.get("role") not in C.ROLES:
                    errors.append(f"{key}: unknown role {row.get('role')}")
                if row.get("cls") not in C.CLASSES:
                    errors.append(f"{key}: unknown class {row.get('cls')}")
                if row.get("express") not in C.EXPRESS:
                    errors.append(f"{key}: unknown express {row.get('express')}")
                for p in row.get("premises", []):
                    if p not in C.PREMISES:
                        errors.append(f"{key}: unknown premise {p}")
                entries = qrows.get((f["file"], fn["function"]))
                if not entries:
                    errors.append(f"{key}: function absent from the slice report")
                    continue
                counter += 1
                loops.append({
                    "id": f"EP-L{counter:03d}",
                    "file": f["file"],
                    "function": fn["function"],
                    "line": loop["line"],
                    "end_line": loop["end_line"],
                    "site": f"{f['file']}:{loop['line']}-{loop['end_line']}",
                    "kind": loop["kind"] + (" (macro)" if loop["macro"] else ""),
                    "depth": loop["depth"],
                    "role": row["role"],
                    "entry": row["entry"],
                    "class": row["cls"],
                    "bound": row["bound"],
                    "premises": row["premises"],
                    "decreasing_under": row.get("decreasing_under"),
                    "progress": row["progress"],
                    "batch_limit": row["bound"] if row.get("express") == "measure-const" else None,
                    "blocking_calls": row.get("blocking", []),
                    "retry_scheduling": row.get("retry"),
                    "exits_inside": {
                        "return_lines": sorted(set(loop["returns"])),
                        "break_lines": sorted(set(loop["breaks"])),
                        "continue_lines": sorted(set(loop["continues"])),
                        "goto_lines": sorted(set(loop["gotos"])),
                    },
                    "yield_points": row.get("yield_points", []),
                    "calls_in_body": sorted({c["callee"] for c in loop["calls"]}),
                    "expressible_as": row["express"],
                    "note": row.get("note"),
                    "quirin": quirin_view(entries, f["file"], loop["line"], loop["end_line"]),
                })
    for key in rows:
        if key not in seen:
            errors.append(f"classification row names no inventory loop: {key}")
    recursion = inventory.get("recursion", [])
    if isinstance(recursion, dict):
        errors.append(f"call-graph name collisions: {recursion.get('name_collisions')}")
        recursion = recursion.get("cycles", [])
    found = [sorted(c["functions"]) for c in recursion]
    declared = [sorted(r["functions"]) for r in C.RECURSION]
    if sorted(found) != sorted(declared):
        errors.append(f"recursion rows {declared} do not match the inventory's cycles {found}")
    recursion_rows = []
    for r in C.RECURSION:
        entries = [e for fn in r["functions"] for e in qrows.get((r["file"], fn), [])]
        views = {fn: quirin_view(qrows[(r["file"], fn)], r["file"], 0, 0)["status_by_variant"]
                 for fn in r["functions"] if (r["file"], fn) in qrows}
        recursion_rows.append(dict(r, quirin_status=views))
    if errors:
        for e in errors:
            print(e, file=sys.stderr)
        sys.exit(1)
    summary = {
        "loops": len(loops),
        "functions_with_loops": len({(l["file"], l["function"]) for l in loops}),
        "by_class": dict(collections.Counter(l["class"] for l in loops)),
        "by_role": dict(collections.Counter(l["role"] for l in loops)),
        "by_role_and_class": {f"{r}/{c}": n for (r, c), n in sorted(collections.Counter((l["role"], l["class"]) for l in loops).items())},
        "by_expressible_as": dict(collections.Counter(l["expressible_as"] for l in loops)),
        "enclosing_status_static": dict(collections.Counter(
            next(s["status"] for s in l["quirin"]["status_by_variant"] if s["variant"] == "static") for l in loops)),
        "loops_with_a_loop_condition_leaf": sum(1 for l in loops if l["quirin"]["loop_condition_leaves"]),
        "recursion_cycles": len(recursion_rows),
        "revisits": len(C.REVISITS),
    }
    out = {
        "schema": SCHEMA,
        "fluent_bit_revision": report.get("fluent_bit_revision") or "046accf6aded7906f66a1baf0202da349e521736",
        "inventory": {"path": str(args.inventory), "sha256": hashlib.sha256(args.inventory.read_bytes()).hexdigest(),
                      "compile_database_sha256": inventory.get("compile_database_sha256")},
        "slice_report": {"path": str(args.report), "sha256": hashlib.sha256(args.report.read_bytes()).hexdigest()},
        "scope": "every for/while/do loop (macro loops at their use site) in every function defined in the eight system-selection files, plus the slice's direct-call recursion cycles and the cross-invocation revisit sites read by hand",
        "no_progress_verdict": "Quirin proves resource safety, not progress. The quirin fields are the resource audit's status and reasons for the enclosing function; no progress verdict is computed or implied.",
        "roles": C.ROLES,
        "classes": C.CLASSES,
        "premises": C.PREMISES,
        "expressible_as": C.EXPRESS,
        "summary": summary,
        "loops": loops,
        "recursion": recursion_rows,
        "revisits": C.REVISITS,
    }
    args.out.write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
