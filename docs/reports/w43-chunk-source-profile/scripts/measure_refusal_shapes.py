#!/usr/bin/env python3
"""Measure the ABI shape of every selected chunk function at the pinned build.

Card W43-CHUNK-SOURCE-PROFILE. For each function the W42 lifecycle inventory
selects, run Clang with that unit's own compile-database flags, filter the JSON
AST by the function's name exactly as ``qcore_c_gateway::typed_c_ast::function_ast``
does (one ``FunctionDecl`` with that exact name and a ``CompoundStmt`` body), and
record its full ``qualType``, result type and parameter types.

It also records each body's direct callees, so a residual such as
``input_chunk_get`` can be explained from its own pinned body.

The script decides nothing about lifecycle. It transports identities: it refuses
an inventory whose recorded artifact digest is not the pinned one, a source file
whose digest differs from the value recorded here, and a compile database that
does not name a selected unit. Where the database names a unit more than once
(``flb_input_chunk.c`` is compiled for the static and the shared library), the
first entry is used, exactly as the pinned audit's exact-unit selection does.
"""
import argparse
import hashlib
import json
import pathlib
import shlex
import subprocess
import sys
from collections import Counter

PINNED_REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
PINNED_ARTIFACT_SHA256 = "c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980"
UNITS = {
    "cio_chunk.c": "lib/chunkio/src/cio_chunk.c",
    "flb_input_chunk.c": "src/flb_input_chunk.c",
}
# cio_chunk.c is independently pinned by contract/profiles/fluent-bit/profile.json;
# flb_input_chunk.c is pinned here from the measured checkout.
UNIT_SHA256 = {
    "cio_chunk.c": "8f6064c742b3fcc53d75b66dbd07d5c7f03d2f245ec7716f8605a2c3a6efd6c8",
    "flb_input_chunk.c": "a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e",
}
PROFILE_REASON = "outside single borrowed chunk / int-or-void-result source profile"
CHUNK_POINTERS = ("struct cio_chunk *", "struct flb_input_chunk *")


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def frontend_arguments(entry):
    words = entry.get("arguments") or shlex.split(entry["command"])
    words = words[1:]
    out, skip = [], False
    for word in words:
        if skip:
            skip = False
            continue
        if word in ("-o", "-MF", "-MT", "-MQ"):
            skip = True
            continue
        if word in ("-c", "-MD", "-MMD") or word == entry["file"]:
            continue
        out.append(word)
    return out


def definitions(clang, entry, function):
    command = [clang, *frontend_arguments(entry), "-fsyntax-only", "-Xclang", "-ast-dump=json",
               "-Xclang", "-ast-dump-filter", "-Xclang", function, entry["file"]]
    result = subprocess.run(command, cwd=entry["directory"], capture_output=True, text=True)
    if result.returncode:
        raise SystemExit(f"clang failed for {function}: {result.stderr[:2000]}")
    decoder = json.JSONDecoder()
    text, index, found = result.stdout, 0, []
    while True:
        while index < len(text) and text[index] != "{":
            index += 1
        if index >= len(text):
            break
        node, index = decoder.raw_decode(text, index)
        if (node.get("kind") == "FunctionDecl" and node.get("name") == function
                and any(child.get("kind") == "CompoundStmt" for child in node.get("inner", []))):
            found.append(node)
    return found


def shape(node):
    qual = node["type"]["qualType"]
    params = [child["type"]["qualType"] for child in node.get("inner", [])
              if child.get("kind") == "ParmVarDecl"]
    result = qual.split("(", 1)[0].strip()
    return qual, result, params


def callees(node):
    """Direct callees named in the body, in first-call order."""
    names = []

    def referenced(expression):
        while expression is not None:
            if expression.get("kind") == "DeclRefExpr":
                decl = expression.get("referencedDecl", {})
                return decl.get("name") if decl.get("kind") == "FunctionDecl" else None
            inner = expression.get("inner") or []
            expression = inner[0] if inner else None
        return None

    def walk(current):
        if current.get("kind") == "CallExpr" and current.get("inner"):
            name = referenced(current["inner"][0])
            if name and name not in names:
                names.append(name)
        for child in current.get("inner", []) or []:
            walk(child)

    for child in node.get("inner", []):
        if child.get("kind") == "CompoundStmt":
            walk(child)
    return names


def result_class(result):
    """A descriptive dimension, not a verdict."""
    if result.endswith("*"):
        return "pointer"
    if result in ("int", "void"):
        return result
    return "other-scalar"


def parameter_shape(params):
    if not params:
        return "no parameter"
    if params == ["void"]:
        return "no parameter"
    head = "chunk" if params[0] in CHUNK_POINTERS else "non-chunk"
    return f"{head} first, {len(params)} parameter(s)"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=pathlib.Path, required=True)
    parser.add_argument("--source-root", type=pathlib.Path, required=True)
    parser.add_argument("--compile-database", type=pathlib.Path, required=True)
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()

    inventory = json.loads(args.inventory.read_text())
    if inventory["artifact_sha256"] != PINNED_ARTIFACT_SHA256:
        raise SystemExit("inventory is not bound to the pinned sidecar digest")
    if inventory["source_revision"] != PINNED_REVISION:
        raise SystemExit("inventory is not at the pinned source revision")
    database = json.loads(args.compile_database.read_text())
    entries = {}
    for unit, relative in UNITS.items():
        path = (args.source_root / relative).resolve(strict=True)
        digest = sha256(path)
        if digest != UNIT_SHA256[unit]:
            raise SystemExit(f"{relative} digest {digest} is not the pinned digest")
        matches = [e for e in database if pathlib.Path(e["file"]).resolve() == path]
        if not matches:
            raise SystemExit(f"compile database does not name {relative}")
        # The pinned audit's exact-unit selection takes the first entry in
        # database order (`select_exact_unit`); mirror it rather than choose.
        entries[unit] = matches[0]

    rows = []
    for row in inventory["functions"]:
        found = definitions(args.clang, entries[row["file"]], row["function"])
        contract = row["field_contract"]
        record = {
            "file": row["file"],
            "function": row["function"],
            "pinned_projection": contract.get(
                "reason", f"checked field contract over {contract.get('states')} states"),
            "definitions_after_preprocessing": len(found),
        }
        if len(found) == 1:
            qual, result, params = shape(found[0])
            record.update({"abi": qual, "result": result, "parameters": params,
                           "result_class": result_class(result),
                           "parameter_shape": parameter_shape(params),
                           "direct_callees": callees(found[0])})
        rows.append(record)

    refused = [r for r in rows if r["pinned_projection"] == PROFILE_REASON]

    def tally(selector):
        counts = Counter(selector(r) for r in refused)
        return dict(sorted(counts.items(), key=lambda kv: (-kv[1], str(kv[0]))))

    summary = {
        "profile_refusals": len(refused),
        "by_result_class": tally(lambda r: r.get("result_class", "no definition")),
        "by_parameter_shape": tally(lambda r: r.get("parameter_shape", "no definition")),
        "by_result_class_and_parameter_shape": tally(
            lambda r: f"{r.get('result_class')} / {r.get('parameter_shape')}"),
        "one_chunk_parameter_only": sum(1 for r in refused if r.get("parameters") and
                                        len(r["parameters"]) == 1 and
                                        r["parameters"][0] in CHUNK_POINTERS),
        "admitted_boolean_abi": sum(1 for r in refused if r.get("abi") in (
            "int (struct cio_chunk *)", "void (struct cio_chunk *)",
            "int (struct flb_input_chunk *)", "void (struct flb_input_chunk *)")),
    }
    report = {
        "card": "W43-CHUNK-SOURCE-PROFILE",
        "source_revision": PINNED_REVISION,
        "inventory": str(args.inventory),
        "inventory_artifact_sha256": inventory["artifact_sha256"],
        "unit_sha256": UNIT_SHA256,
        "clang": subprocess.run([args.clang, "--version"], capture_output=True,
                                text=True).stdout.splitlines()[0],
        "summary": summary,
        "functions": rows,
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    json.dump(summary, sys.stdout, indent=2)
    print()


if __name__ == "__main__":
    main()
