#!/usr/bin/env python3
"""Render complete final, unit-bound changed-leaf and Safe-move review rows.

Usage: build_final_review.py CORPUS_CENSUS_DIR SYSTEM_CENSUS_DIR
       CORPUS_PARITY_JSONL SYSTEM_PARITY_JSONL OUTPUT_JSON
"""

import collections
import gzip
import json
import pathlib
import sys


def read_rows(path):
    with gzip.open(path, "rt") as stream:
        return [json.loads(line) for line in stream]


def mechanism(row):
    side = row["side"]
    category = row["diagnostic"]["category"]
    description = row["diagnostic"]["description"]
    if side == "removed" and category == "unsupported" and description.startswith("call to uncontracted function `"):
        return "callee_call_refusal_withdrawal"
    if side == "removed" and (category == "unknown_alias" or description.startswith("nullable resource `")):
        return "downstream_binding_obligation_withdrawal"
    if side == "removed":
        return "diagnostic_reprojection"
    if side == "added" and category == "unknown_alias":
        return ("later_or_callee_alias_obligation" if row["relationship"] != "newly_admitted_statement_same_line"
                else "admitted_statement_root_obligation")
    return "retained_independent_refusal"


def classify_safe(row, statements):
    name = row["function"]
    if row["library"] == "cmetrics":
        return "no_access_pointer_declarator"
    if name == "rd_kafka_Uuid_str":
        return "indexed_scalar_store_and_field_reads"
    if name.endswith("_cmp") or "_cmp_" in name or name in {"mykey_cmp", "person_cmp"}:
        return "untracked_pointer_declarators_for_comparison"
    if any("->" in statement["text"] or " - > " in statement["text"]
           or " . " in statement["text"] for statement in statements):
        return "typed_scalar_member_or_index_path"
    return "custody_neutral_scalar_statement"


def process(scope, census, parity_path):
    comparison = json.loads((census / "comparison.json").read_text())
    attribution = read_rows(census / "final-attribution/changed-leaf-attribution.jsonl.gz")
    if scope == "corpus":
        replay = json.loads((census / "source-replay.json").read_text())
        source_by_unit = {row["unit_index"]: row["matches"][0]
                          for row in replay["units"] if len(row["matches"]) == 1}
    else:
        source_by_unit = {}
    parity = collections.defaultdict(list)
    frontend_by_source = collections.defaultdict(set)
    with parity_path.open() as stream:
        for line in stream:
            row = json.loads(line)
            frontend_by_source[row["source_sha256"]].add(row["frontend_sha256"])
            if row["old_route"] == "old_refusal" and row["new_admitted"]:
                parity[(row["function"], row["source_sha256"], row["frontend_sha256"])].append(row)
    if scope == "system":
        diagnostics = json.loads((parity_path.parent / "resource-diagnostics.json").read_text())
        contexts = {
            row["frontend_sha256"]: row["frontend_debug"]
            for row in (json.loads(line) for line in
                        (parity_path.parent / "parity.jsonl.contexts.jsonl").open())
        }
        for index, unit in enumerate(diagnostics["units"]):
            config = unit["compile_configuration"]
            sha = config["compile_task_identity"]["preprocessed_source_sha256"]
            frontends = frontend_by_source[sha]
            shared = "-Dfluent_bit_shared_EXPORTS" in config["command"]
            matching_frontends = [frontend for frontend in frontends
                                  if ('Define("fluent_bit_shared_EXPORTS")' in contexts[frontend]) == shared]
            if len(matching_frontends) != 1:
                raise ValueError(f"system unit {index}: ambiguous frontend context {matching_frontends}")
            source_by_unit[index] = {
                "source_sha256": sha,
                "frontend_sha256": matching_frontends[0],
                "source_file": config["expected_source"],
            }
    non_direct = []
    for row in attribution:
        if row["relationship"] == "newly_admitted_statement_same_line":
            continue
        index = row["unit_index"]
        source = source_by_unit.get(index)
        source_sha = source["source_sha256"] if source else None
        candidate = parity.get((row["function"], source_sha, source["frontend_sha256"]), []) if source_sha else []
        witnesses = sorted({(r["line"], " ".join(r["texts"])) for r in candidate})
        non_direct.append({
            "unit_index": index,
            "library": row["library"], "file": row["file"], "function": row["function"],
            "side": row["side"], "diagnostic": row["diagnostic"],
            "relationship": row["relationship"], "mechanism": mechanism(row),
            "verdict_before": row["verdict_before"], "verdict_after": row["verdict_after"],
            "source_file": source["source_file"] if source else None,
            "preprocessed_source_sha256": source_sha,
            "frontend_sha256": source["frontend_sha256"] if source else None,
            "same_function_admitted_statements": [
                {"line": line, "text": text} for line, text in witnesses
            ],
        })
    safe = []
    for move in comparison["moves"]:
        if move["before"] != "Unknown" or move["after"] != "Safe":
            continue
        source = source_by_unit.get(move["unit_index"])
        source_sha = source["source_sha256"] if source else None
        candidate = parity.get((move["function"], source_sha, source["frontend_sha256"]), []) if source_sha else []
        statements = [{"line": line, "text": text} for line, text in sorted({
            (r["line"], " ".join(r["texts"])) for r in candidate
        })]
        safe.append({
            "unit_index": move["unit_index"], "library": move["library"],
            "file": move["file"], "function": move["function"],
            "mechanism": classify_safe(move, statements),
            "removed": move["removed"], "added": move["added"],
            "source_file": source["source_file"] if source else None,
            "preprocessed_source_sha256": source_sha,
            "frontend_sha256": source["frontend_sha256"] if source else None,
            "admitted_statements": statements,
        })
    return {
        "comparison": str(census / "comparison.json"),
        "attribution": str(census / "final-attribution/changed-leaf-attribution.jsonl.gz"),
        "non_direct_count": sum(r["diagnostic"]["count"] for r in non_direct),
        "non_direct": non_direct,
        "safe_moves": safe,
    }


def main():
    if len(sys.argv) != 6:
        raise SystemExit(__doc__)
    corpus, system, corpus_parity, system_parity, out = map(pathlib.Path, sys.argv[1:6])
    result = {
        "schema": "qcore.w46-access-path-statements-final-mechanism-review.v1",
        "corpus": process("corpus", corpus, corpus_parity),
        "system": process("system", system, system_parity),
    }
    out.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({scope: {"non_direct_count": result[scope]["non_direct_count"],
                              "safe_moves": len(result[scope]["safe_moves"])}
                      for scope in ("corpus", "system")}, indent=2))


if __name__ == "__main__":
    main()
