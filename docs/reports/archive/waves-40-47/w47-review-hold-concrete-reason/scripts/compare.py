#!/usr/bin/env python3
"""compare.py BASE RUN [--json OUT] [--alias FROM=TO ...]

Card W47-REVIEW-HOLD-CONCRETE-REASON. BASE and RUN are measurement
directories (raw files, or an archive of `.gz` files) of one scope: corpus,
system or slice. Units are aligned by position (two configurations of one
file are two units). Reports:

* raw identity of `resource-diagnostics.json` and its `.contracts.json`
  sidecar, after replacing each alias FROM (a run's own root path) by TO;
* every unit or function status move and every diagnostic-leaf move;
* the function rows that differ, and whether each differs only by the
  `review_holds` member this card adds (listed with its holds);
* the JSON paths that differ in the report and in the sidecar;
* the console lines (`console.log`, or `audit.log` for the corpus) present
  on one side only.

Exit 0 when no unit or function status and no leaf moves, 1 otherwise.
"""
import gzip
import hashlib
import json
import pathlib
import sys
from collections import Counter


def raw(directory: pathlib.Path, name: str):
    path = directory / name
    if path.is_file():
        return path.read_bytes()
    packed = directory / (name + ".gz")
    if packed.is_file():
        return gzip.decompress(packed.read_bytes())
    return None


def aliased(data, aliases):
    if data is None:
        return None
    text = data.decode("utf-8")
    for source, target in aliases:
        text = text.replace(source, target)
    return text.encode("utf-8")


def walk(x, y, path, out):
    if isinstance(x, dict) and isinstance(y, dict):
        for key in sorted(set(x) | set(y)):
            walk(x.get(key), y.get(key), f"{path}/{key}", out)
    elif isinstance(x, list) and isinstance(y, list) and len(x) == len(y):
        for u, v in zip(x, y):
            walk(u, v, f"{path}[]", out)
    elif x != y:
        out.add(path)


def leaves(function):
    return Counter(
        (d.get("category"), d.get("description"), d.get("line"))
        for d in function.get("diagnostics", [])
    )


def statuses(report):
    functions = Counter(f.get("status") for u in report["units"] for f in u.get("functions", []))
    units = Counter(u.get("status") for u in report["units"])
    return dict(sorted(functions.items())), dict(sorted(units.items()))


def console(directory):
    for name in ("console.log", "audit.log"):
        data = raw(directory, name)
        if data is not None:
            return name, data.decode("utf-8").splitlines()
    return None, []


def main() -> int:
    args = sys.argv[1:]
    out = None
    aliases = []
    if "--json" in args:
        index = args.index("--json")
        out = args[index + 1]
        del args[index : index + 2]
    while "--alias" in args:
        index = args.index("--alias")
        source, target = args[index + 1].split("=", 1)
        aliases.append((source, target))
        del args[index : index + 2]
    base_dir, run_dir = pathlib.Path(args[0]), pathlib.Path(args[1])
    base_bytes = aliased(raw(base_dir, "resource-diagnostics.json"), aliases)
    run_bytes = aliased(raw(run_dir, "resource-diagnostics.json"), aliases)
    base, run = json.loads(base_bytes), json.loads(run_bytes)
    base_side = aliased(raw(base_dir, "resource-diagnostics.contracts.json"), aliases)
    run_side = aliased(raw(run_dir, "resource-diagnostics.contracts.json"), aliases)
    report_paths, sidecar_paths = set(), set()
    walk(base, run, "", report_paths)
    walk(json.loads(base_side), json.loads(run_side), "", sidecar_paths)
    unit_moves, function_moves, leaf_moves = [], [], []
    only_review_holds, other_row_changes = [], []
    if len(base["units"]) != len(run["units"]):
        unit_moves.append(f"unit count {len(base['units'])} -> {len(run['units'])}")
    for index, (bu, ru) in enumerate(zip(base["units"], run["units"])):
        where = f"#{index} {ru.get('library')}/{ru['file']}"
        if (bu.get("library"), bu["file"]) != (ru.get("library"), ru["file"]):
            unit_moves.append(f"{where}: unit identity differs")
            continue
        if bu.get("status") != ru.get("status"):
            unit_moves.append(f"{where}: {bu.get('status')} -> {ru.get('status')}")
        bfs = {f["function"]: f for f in bu.get("functions", [])}
        rfs = {f["function"]: f for f in ru.get("functions", [])}
        for name in sorted(set(bfs) | set(rfs)):
            bf, rf = bfs.get(name), rfs.get(name)
            if bf is None or rf is None:
                function_moves.append(f"{where}::{name}: row present on one side only")
                continue
            if bf.get("status") != rf.get("status"):
                function_moves.append(f"{where}::{name}: {bf.get('status')} -> {rf.get('status')}")
            if leaves(bf) != leaves(rf):
                leaf_moves.append(f"{where}::{name}")
            if bf == rf:
                continue
            stripped = {k: v for k, v in rf.items() if k != "review_holds"}
            base_stripped = {k: v for k, v in bf.items() if k != "review_holds"}
            if stripped == base_stripped:
                only_review_holds.append({
                    "row": f"{where}::{name}",
                    "status": rf.get("status"),
                    "base_review_holds": bf.get("review_holds"),
                    "review_holds": [
                        {
                            "declaration": hold["declaration"],
                            "scope": hold["scope"],
                            "clause": hold["clause"],
                            "category": hold["category"],
                            "reason": hold["reason"],
                        }
                        for hold in rf.get("review_holds") or []
                    ],
                })
            else:
                other_row_changes.append(f"{where}::{name}")
    base_console_name, base_console = console(base_dir)
    run_console_name, run_console = console(run_dir)
    base_lines, run_lines = Counter(base_console), Counter(run_console)
    result = {
        "resource_diagnostics_identical": base_bytes == run_bytes,
        "resource_diagnostics_sha256": {
            "base": hashlib.sha256(base_bytes).hexdigest(),
            "run": hashlib.sha256(run_bytes).hexdigest(),
        },
        "contracts_sidecar_identical": base_side == run_side,
        "report_differing_json_paths": sorted(report_paths),
        "sidecar_differing_json_paths": sorted(sidecar_paths),
        "unit_status_moves": unit_moves,
        "function_status_moves": function_moves,
        "function_leaf_moves": leaf_moves,
        "rows_differing_only_by_review_holds": only_review_holds,
        "rows_differing_otherwise": other_row_changes,
        "console": {
            "files": [base_console_name, run_console_name],
            "only_in_base": sorted((base_lines - run_lines).elements()),
            "only_in_run": sorted((run_lines - base_lines).elements()),
        },
        "base": dict(zip(("functions", "units"), statuses(base)),
                     diagnostic_counts=base.get("diagnostic_counts")),
        "run": dict(zip(("functions", "units"), statuses(run)),
                    diagnostic_counts=run.get("diagnostic_counts")),
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    print(text)
    if out:
        pathlib.Path(out).write_text(text + "\n", encoding="utf-8")
    moved = unit_moves or function_moves or leaf_moves
    return 1 if moved else 0


if __name__ == "__main__":
    raise SystemExit(main())
