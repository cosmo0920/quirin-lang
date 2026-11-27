#!/usr/bin/env python3
"""W45-FB-SYSTEM-SELECTION: derive the published system-slice evidence.

Usage:
  measure.py --slice-report SLICE/resource-diagnostics.json
             --main-report MAIN/resource-diagnostics.json
             --census-archive docs/reports/w45-return-expression-families/integration/artifacts/diagnostics.json.gz
             --baseline-archive docs/reports/w45-sub-object-stores/integration/artifacts/diagnostics.json.gz
             [--inventory docs/reports/w45-fb-system-selection/lifetime-inventory.json]
             --out DIR

Reads only. Writes into DIR (which must not exist):
  denominator.json          every unit and function of the eight files, with verdicts,
                            Unknown reasons and blocker classes;
  input-chunk-comparison.json
                            flb_input_chunk.c function by function across the census
                            archive, the current pinned baseline, the configured
                            main-profile run at 864 and the system slice;
  slice-mechanisms.json     (with --inventory) per P1 slice of the lifetime inventory,
                            the blocker classes ranked by the number of slice functions
                            they block, and the uncontracted callees;
  inventory-audit-join.json (with --inventory) every function a site or edge of the
                            lifetime inventory names, with its audit verdict.

A blocker class is a diagnostic's category plus its description with any
backquoted name removed, so `call to uncontracted function `x`` and
`call to uncontracted function `y`` are one class. This is a measurement of
the audit's own refusals; it is not a model, a proof or a native observation.
"""

import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import re

FILES = [
    "plugins/in_storage_backlog/sb.c",
    "src/flb_input_chunk.c",
    "src/flb_storage.c",
    "src/flb_task.c",
    "src/flb_engine.c",
    "src/flb_engine_dispatch.c",
    "src/flb_scheduler.c",
    "src/flb_output.c",
]


def load(path: pathlib.Path):
    raw = path.read_bytes()
    data = gzip.decompress(raw) if path.suffix == ".gz" else raw
    return json.loads(data), hashlib.sha256(raw).hexdigest(), hashlib.sha256(data).hexdigest()


def blocker_class(diagnostic) -> str:
    description = re.sub(r"`[^`]*`", "`_`", diagnostic["description"])
    description = re.sub(r" acquired at \S+$", " acquired at _", description)
    return f"{diagnostic['category']}: {description}"


def variant(unit) -> str:
    command = (unit.get("compile_configuration") or {}).get("command") or ""
    if "fluent-bit-static.dir" in command:
        return "static"
    if "fluent-bit-shared.dir" in command:
        return "shared"
    if "flb-plugin-" in command:
        return "plugin"
    return "unconfigured" if not command else "other"


def origin_line(diagnostic):
    origin = diagnostic.get("origin") or ""
    match = re.search(r":(\d+)$", origin)
    return int(match.group(1)) if match else None


def function_summary(function):
    classes = collections.Counter(blocker_class(x) for x in function["diagnostics"])
    leaks = [
        {"description": x["description"], "origin": x.get("origin")}
        for x in function["diagnostics"]
        if x["category"] == "leak"
    ]
    first = min(
        (x for x in function["diagnostics"] if origin_line(x) is not None),
        key=origin_line,
        default=None,
    )
    return {
        "status": function["status"],
        "diagnostics": len(function["diagnostics"]),
        "blocker_classes": dict(sorted(classes.items())),
        "first_blocker": None
        if first is None
        else {"class": blocker_class(first), "origin": first.get("origin")},
        "leaks": leaks,
    }


def denominator(report):
    units = []
    functions = collections.OrderedDict()
    for unit in report["units"]:
        if unit["file"] not in FILES:
            raise SystemExit(f"unexpected unit {unit['file']}")
        kind = variant(unit)
        not_projected = unit.get("not_projected", [])
        units.append({
            "file": unit["file"],
            "variant": kind,
            "status": unit["status"],
            "reason_code": unit.get("reason_code"),
            "reason": unit.get("reason"),
            "source_functions": len(unit["source_functions"]),
            "analysed_functions": len(unit["functions"]),
            "function_status": dict(sorted(collections.Counter(f["status"] for f in unit["functions"]).items())),
            "not_projected": not_projected,
            "enumeration_completeness": (unit.get("enumeration") or {}).get("completeness"),
            "inventory_sha256": (unit.get("compile_configuration") or {}).get("inventory_sha256"),
            "compile_task_input_sha256": ((unit.get("compile_configuration") or {}).get("compile_task_identity") or {}).get("preprocessed_source_sha256"),
        })
        for function in unit["functions"]:
            key = (unit["file"], function["function"])
            functions.setdefault(key, {})[kind] = function_summary(function)
        for name in not_projected:
            key = (unit["file"], name)
            functions.setdefault(key, {})[kind] = {
                "status": "not-projected",
                "reason": "raw-scan definition absent from this configuration's preprocessed text (report not_projected)",
            }
    rows = []
    variants_equal = 0
    for (file, name), by_variant in functions.items():
        statuses = sorted({summary["status"] for summary in by_variant.values()})
        summaries = list(by_variant.values())
        same = all(summary == summaries[0] for summary in summaries)
        variants_equal += same
        row = {"file": file, "function": name, "variants": sorted(by_variant), "status": statuses[0] if len(statuses) == 1 else statuses,
               "variants_identical": same}
        if same:
            row.update({k: v for k, v in summaries[0].items() if k != "status"})
        else:
            row["by_variant"] = by_variant
        rows.append(row)
    per_file = collections.OrderedDict()
    for row in rows:
        per_file.setdefault(row["file"], collections.Counter())[row["status"] if isinstance(row["status"], str) else "variant-dependent"] += 1
    unit_status = collections.Counter(unit["status"] for unit in units)
    function_status = collections.Counter(row["status"] if isinstance(row["status"], str) else "variant-dependent" for row in rows)
    return {
        "units": units,
        "functions": rows,
        "totals": {
            "selected_files": len(FILES),
            "files_with_units": len({unit["file"] for unit in units}),
            "units": len(units),
            "unit_status": dict(sorted(unit_status.items())),
            "distinct_functions": len(rows),
            "function_status": dict(sorted(function_status.items())),
            "functions_identical_across_variants": variants_equal,
            "per_file": {file: dict(sorted(counts.items())) for file, counts in per_file.items()},
            "blocker_classes_over_distinct_functions": dict(collections.Counter(
                cls for row in rows if "blocker_classes" in row for cls in row["blocker_classes"]
            ).most_common()),
        },
    }


def chunk_rows(report, library=None):
    rows = []
    for unit in report["units"]:
        if unit["file"].endswith("flb_input_chunk.c") and (library is None or unit["library"] == library):
            rows.append(unit)
    return rows


def signature(function):
    return sorted(
        (x["category"], x["description"], origin_line(x)) for x in function["diagnostics"]
    )


def compare_chunk(sources):
    table = {}
    for label, units in sources.items():
        for index, unit in enumerate(units):
            column = label if len(units) == 1 else f"{label}#{variant(unit)}"
            table[column] = {
                "unit_status": unit["status"],
                "reason_code": unit.get("reason_code"),
                "source_functions": len(unit["source_functions"]),
                "analysed": len(unit["functions"]),
                "not_projected": unit.get("not_projected", []),
                "functions": {f["function"]: f for f in unit["functions"]},
            }
    columns = list(table)
    reference = columns[0]
    names = sorted(set().union(*(set(table[c]["functions"]) for c in columns)))
    differences = []
    for name in names:
        present = {c: table[c]["functions"].get(name) for c in columns}
        statuses = {c: (f["status"] if f else "absent") for c, f in present.items()}
        sigs = {c: (signature(f) if f else None) for c, f in present.items()}
        if len(set(statuses.values())) > 1 or any(sigs[c] != sigs[reference] for c in columns):
            reference_classes = collections.Counter(
                (x["category"], x["description"], origin_line(x)) for x in (present[reference] or {"diagnostics": []})["diagnostics"])
            delta = {}
            for c, f in present.items():
                if c == reference or f is None:
                    continue
                mine = collections.Counter((x["category"], x["description"], origin_line(x)) for x in f["diagnostics"])
                if mine != reference_classes:
                    delta[c] = {
                        "only_here": [list(k) + [v] for k, v in sorted((mine - reference_classes).items(), key=str)],
                        "only_in_reference": [list(k) + [v] for k, v in sorted((reference_classes - mine).items(), key=str)],
                    }
            differences.append({
                "function": name,
                "status": statuses,
                "diagnostic_multiset_equal_to_reference": {c: sigs[c] == sigs[reference] for c in columns},
                "diagnostic_counts": {c: (len(f["diagnostics"]) if f else None) for c, f in present.items()},
                "delta_against_reference": delta,
            })
    return {
        "reference": reference,
        "columns": {
            c: {k: v for k, v in table[c].items() if k != "functions"}
            | {"function_status": dict(sorted(collections.Counter(f["status"] for f in table[c]["functions"].values()).items()))}
            for c in columns
        },
        "functions_compared": len(names),
        "comparison_key": "per function: status and the multiset of (category, description, original source line) of its diagnostics; preprocessed line numbers and file-name spelling are excluded because the pinned route spells the origin by basename and preprocesses different headers",
        "differences": differences,
    }


def inventory_function(item):
    """(file, function) of a lifetime-inventory slice entry: the name before any
    parenthetical, the file before the line range of `at`."""
    name = item["name"].split(" (")[0].strip()
    file = item["at"].split(":")[0].strip()
    return file, name


def slice_mechanisms(slices, rows, report):
    by_key = {(row["file"], row["function"]): row for row in rows}
    diagnostics = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            diagnostics.setdefault((unit["file"], function["function"]), function["diagnostics"])
    result = {}
    for name, spec in slices.items():
        members = []
        missing = []
        callees = collections.Counter()
        for item in spec["functions"]:
            file, function = inventory_function(item)
            item = {"file": file, "function": function, "inventory_entry": item["name"]}
            for x in diagnostics.get((file, function), []):
                match = re.fullmatch(r"call to uncontracted function `([^`]*)`", x["description"])
                if match:
                    callees[match.group(1)] += 1
            row = by_key.get((file, function))
            if row is None:
                missing.append(item)
            else:
                members.append(row)
        blocks = collections.Counter()
        sole = collections.Counter()
        statuses = collections.Counter()
        for row in members:
            statuses[row["status"] if isinstance(row["status"], str) else "variant-dependent"] += 1
            classes = row.get("blocker_classes", {})
            for cls in classes:
                blocks[cls] += 1
            non_leak = [c for c in classes if not c.startswith("leak")]
            if len(non_leak) == 1:
                sole[non_leak[0]] += 1
        # Greedy cover: repeatedly add the class whose addition leaves the most
        # slice functions with no remaining refusal class (leaks excluded, since a
        # leak is an accusation, not a refusal). The cumulative count is how many
        # functions would carry no refusal once those classes were all decided;
        # it is an upper bound on reach, never a prediction of Safe.
        blocked = {
            (row["file"], row["function"]): {c for c in row.get("blocker_classes", {}) if not c.startswith("leak")}
            for row in members
        }
        chosen, cover = [], []
        remaining = set(blocks) - {c for c in blocks if c.startswith("leak")}
        while remaining:
            def cleared(extra):
                allowed = set(chosen) | {extra}
                return sum(1 for classes in blocked.values() if classes and classes <= allowed)
            best = max(sorted(remaining), key=lambda c: (cleared(c), blocks[c]))
            chosen.append(best)
            remaining.discard(best)
            cover.append({"add": best, "functions_without_refusal": cleared(best)})
        result[name] = {
            "greedy_cover": cover,
            "uncontracted_callees": dict(callees.most_common()),
            "functions": [
                {"file": row["file"], "function": row["function"], "status": row["status"],
                 "blocker_classes": row.get("blocker_classes", {}), "leaks": row.get("leaks", [])}
                for row in members
            ],
            "absent_from_audit": missing,
            "function_status": dict(sorted(statuses.items())),
            "ranking": [
                {"class": cls, "functions_blocked": count, "functions_where_sole_non_leak_class": sole.get(cls, 0)}
                for cls, count in sorted(blocks.items(), key=lambda item: (-item[1], item[0]))
            ],
        }
    return result


FUNCTION_AT = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*)(?![A-Za-z0-9_.])")


def function_reference(text):
    """The C function a site or edge endpoint names first, or None for prose
    ("callers of ...", "output worker loop"), a macro constant or a member
    (`sb_out_queue.chunks`)."""
    match = FUNCTION_AT.match(text or "")
    if not match:
        return None
    name = match.group(1)
    if "_" not in name or name.isupper():
        return None
    return name


def inventory_join(inventory, rows):
    """Every function named by a site (`function`) or an edge endpoint (the first
    identifier of `from`/`to`) of the lifetime inventory, with its audit verdict
    when it is a function of the eight files."""
    by_name = collections.defaultdict(list)
    for row in rows:
        by_name[row["function"]].append(row)
    named = collections.OrderedDict()
    for identity in inventory["identities"]:
        for kind, sites in identity.get("sites", {}).items():
            for site in sites:
                name = function_reference(site.get("function", ""))
                if name:
                    named.setdefault(name, set()).add(f"{identity['id']}:{kind}")
    for edge in inventory["edges"]:
        for end in ("from", "to"):
            name = function_reference(edge.get(end, ""))
            if name and not edge.get(end, "").startswith("outside_selection"):
                named.setdefault(name, set()).add(f"edge:{edge['id']}:{end}")
    functions = []
    status = collections.Counter()
    for name, uses in named.items():
        matches = by_name.get(name, [])
        if not matches:
            verdict = "outside-the-eight-files"
        elif len({row["status"] if isinstance(row["status"], str) else "variant-dependent" for row in matches}) == 1:
            verdict = matches[0]["status"] if isinstance(matches[0]["status"], str) else "variant-dependent"
        else:
            verdict = "differs-by-file"
        status[verdict] += 1
        functions.append({
            "function": name,
            "files": sorted({row["file"] for row in matches}),
            "audit_status": verdict,
            "inventory_uses": sorted(uses),
        })
    return {"functions": functions, "totals": {"functions_named": len(functions), "audit_status": dict(sorted(status.items()))}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slice-report", type=pathlib.Path, required=True)
    parser.add_argument("--main-report", type=pathlib.Path, required=True)
    parser.add_argument("--census-archive", type=pathlib.Path, required=True)
    parser.add_argument("--baseline-archive", type=pathlib.Path, required=True)
    parser.add_argument("--inventory", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    slice_report, slice_raw, _ = load(args.slice_report)
    main_report, main_raw, _ = load(args.main_report)
    census, census_gz, census_raw = load(args.census_archive)
    baseline, baseline_gz, baseline_raw = load(args.baseline_archive)
    inputs = {
        "slice_report": {"path": str(args.slice_report.resolve()), "sha256": slice_raw, "revision": slice_report["fluent_bit_revision"]},
        "main_report": {"path": str(args.main_report.resolve()), "sha256": main_raw, "revision": main_report["fluent_bit_revision"]},
        "census_archive": {"path": str(args.census_archive), "gzip_sha256": census_gz, "raw_sha256": census_raw, "revision": census["fluent_bit_revision"]},
        "baseline_archive": {"path": str(args.baseline_archive), "gzip_sha256": baseline_gz, "raw_sha256": baseline_raw, "revision": baseline["fluent_bit_revision"]},
    }
    scope = "measurement of Quirin audit rows; Unsafe is provisional; not a model, proof, native observation or upstream defect claim"
    result = denominator(slice_report)
    (args.out / "denominator.json").write_text(json.dumps({"schema_version": 1, "inputs": inputs, "scope": scope, **result}, indent=1) + "\n")
    comparison = compare_chunk({
        "baseline-archive@66910c10(pinned)": chunk_rows(baseline, "fluent-bit-chunk"),
        "census-archive@66910c10(pinned)": chunk_rows(census, "fluent-bit-chunk"),
        "main-profile@864(current)": chunk_rows(main_report, "fluent-bit-chunk"),
        "system-slice@864(current)": chunk_rows(slice_report),
    })
    (args.out / "input-chunk-comparison.json").write_text(json.dumps({"schema_version": 1, "inputs": inputs, "scope": scope, **comparison}, indent=1) + "\n")
    summary = {"totals": result["totals"], "chunk_differences": len(comparison["differences"])}
    if args.inventory:
        inventory = json.loads(args.inventory.read_text())
        mechanisms = slice_mechanisms(inventory["slices"], result["functions"], slice_report)
        join = inventory_join(inventory, result["functions"])
        record = inputs | {"inventory": {"path": str(args.inventory), "sha256": hashlib.sha256(args.inventory.read_bytes()).hexdigest()}}
        (args.out / "slice-mechanisms.json").write_text(json.dumps({"schema_version": 1, "inputs": record, "scope": scope, "slices": mechanisms}, indent=1) + "\n")
        (args.out / "inventory-audit-join.json").write_text(json.dumps({"schema_version": 1, "inputs": record, "scope": scope, **join}, indent=1) + "\n")
        summary["inventory_join"] = join["totals"]
        summary["slices"] = {name: {"functions": len(value["functions"]), "absent": len(value["absent_from_audit"]), "status": value["function_status"], "top": value["ranking"][:5], "cover": value["greedy_cover"][:8], "callees": list(value["uncontracted_callees"].items())[:15]} for name, value in mechanisms.items()}
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
