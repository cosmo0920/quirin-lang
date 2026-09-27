#!/usr/bin/env python3
"""Join every changed diagnostic to the side-by-side statement coordinates.

Usage: attribute_final.py CHANGED_FUNCTIONS_GZ PARITY_JSONL AFTER_DIAGNOSTICS OUTPUT_DIR
       [CORPUS_SOURCE_REPLAY_JSON].

System units expose the exact preprocessed source SHA in their compile-task
identity, so even repeated configurations bind to parity. Corpus units require
exact replay of the archived frontend request against pinned source bytes.
Other rows retain ambiguity rather than borrowing header functions from a
different unit.
"""

import collections
import gzip
import hashlib
import io
import json
import pathlib
import sys


def writer(path):
    return io.TextIOWrapper(gzip.GzipFile(filename=str(path), mode="wb", mtime=0), encoding="utf-8")


def read(path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as stream:
        return json.load(stream)


def main():
    if len(sys.argv) not in (5, 6):
        raise SystemExit(__doc__)
    changes, parity, diagnostics, out = map(pathlib.Path, sys.argv[1:5])
    replay = pathlib.Path(sys.argv[5]) if len(sys.argv) == 6 else None
    out.mkdir(parents=True, exist_ok=False)
    units = read(diagnostics)["units"]
    occurrences = collections.defaultdict(list)
    source_units = collections.defaultdict(set)
    for unit_index, unit in enumerate(units):
        source_sha = (unit.get("compile_configuration") or {}).get(
            "compile_task_identity", {}
        ).get("preprocessed_source_sha256")
        if source_sha:
            source_units[source_sha].add(unit_index)
        for function in unit["functions"]:
            occurrences[function["function"]].append(
                (unit_index, unit["library"], unit["file"])
            )
    with parity.open() as stream:
        parity_rows = [json.loads(line) for line in stream]
    source_provenance = {sha: "compile_task_identity" for sha in source_units}
    if replay:
        for record in read(replay)["units"]:
            index = record["unit_index"]
            unit = units[index]
            if (record["library"], record["file"]) != (unit["library"], unit["file"]):
                raise SystemExit(f"replay unit identity mismatch: {index}")
            for match in record["matches"]:
                if match["exit"] != 0 or match["preprocessed_sha256"] != match["source_sha256"]:
                    raise SystemExit(f"invalid replay match: {match}")
                source_units[match["source_sha256"]].add(index)
                source_provenance[match["source_sha256"]] = "clang_preprocess_replay"
    if not source_units:
        raise SystemExit("corpus attribution requires exact source replay mapping")
    direct = collections.defaultdict(set)
    direct_by_unit_function = collections.defaultdict(set)
    candidate_direct = collections.defaultdict(set)
    candidate_lines = collections.defaultdict(set)
    for row in parity_rows:
        if row["old_route"] == "old_refusal" and row["new_admitted"]:
            name, line_number, source_sha = (
                row["function"], row["line"], row["source_sha256"]
            )
            candidate_direct[(name, line_number)].add(source_sha)
            candidate_lines[name].add(line_number)
            exact_units = source_units.get(source_sha, set())
            matched = [index for index in exact_units
                       if (index, units[index]["library"], units[index]["file"])
                       in occurrences[name]]
            for index in matched:
                direct[(index, name, line_number)].add(source_sha)
                direct_by_unit_function[(index, name)].add(line_number)
    totals = collections.Counter()
    families = collections.Counter()
    rows = []
    with gzip.open(changes, "rt") as stream:
        for line in stream:
            function = json.loads(line)
            unit_index = function["unit_index"]
            if unit_index >= len(units):
                raise SystemExit(f"changed function has invalid unit index: {unit_index}")
            unit = units[unit_index]
            identity = (unit_index, function["library"], function["file"])
            if (unit["library"], unit["file"]) != identity[1:] or identity not in occurrences[function["function"]]:
                raise SystemExit(f"changed function does not match AFTER unit: {function}")
            for side in ("removed", "added"):
                for diagnostic in function[side]:
                    name, line_number = function["function"], diagnostic["line"]
                    matched_sources = sorted(direct[(unit_index, name, line_number)])
                    matched_lines = sorted(direct_by_unit_function[(unit_index, name)])
                    possible_sources = sorted(candidate_direct[(name, line_number)])
                    possible_lines = sorted(candidate_lines[name])
                    if not matched_lines and len(occurrences[name]) > 1 and possible_lines:
                        relationship = "ambiguous_function_unit"
                    elif not matched_lines and possible_lines:
                        relationship = "unresolved_source_unit"
                    elif len(matched_sources) > 1:
                        relationship = "ambiguous_source_context_same_line"
                    elif matched_sources:
                        relationship = "newly_admitted_statement_same_line"
                    elif matched_lines:
                        relationship = "same_function_downstream_or_summary"
                    else:
                        relationship = "no_new_statement_in_function"
                    row = {
                        "unit_index": unit_index,
                        "library": function["library"],
                        "file": function["file"],
                        "function": function["function"],
                        "verdict_before": function["before"],
                        "verdict_after": function["after"],
                        "side": side,
                        "diagnostic": diagnostic,
                        "relationship": relationship,
                        "matching_source_sha256": matched_sources if len(matched_sources) == 1 else [],
                        "matching_source_provenance": [source_provenance[sha] for sha in matched_sources]
                            if len(matched_sources) == 1 else [],
                        "candidate_source_sha256": possible_sources,
                        "new_statement_lines_in_function": matched_lines,
                        "candidate_statement_lines_in_function": possible_lines,
                        "candidate_units": occurrences[name] if relationship == "ambiguous_function_unit" else [],
                    }
                    rows.append(row)
                    totals[(side, diagnostic["category"], relationship)] += diagnostic["count"]
                    families[(side, diagnostic["description"], relationship)] += diagnostic["count"]
    with writer(out / "changed-leaf-attribution.jsonl.gz") as stream:
        for row in sorted(rows, key=lambda r: (r["unit_index"], r["library"], r["file"], r["function"], r["side"], r["diagnostic"]["line"], r["diagnostic"]["description"])):
            stream.write(json.dumps(row, sort_keys=True) + "\n")
    result = {
        "schema": "qcore.w46-access-path-statements-attribution.v2",
        "source_unit_bindings": dict(sorted(collections.Counter(source_provenance.values()).items())),
        "changed_leaf_rows": len(rows),
        "weighted_counts": [
            {"side": side, "category": category, "relationship": relationship, "count": count}
            for (side, category, relationship), count in sorted(totals.items())
        ],
        "families": [
            {"side": side, "description": description, "relationship": relationship, "count": count}
            for (side, description, relationship), count in sorted(families.items())
        ],
        "attribution_sha256": hashlib.sha256((out / "changed-leaf-attribution.jsonl.gz").read_bytes()).hexdigest(),
    }
    (out / "attribution.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result["weighted_counts"], indent=2))


if __name__ == "__main__":
    main()
