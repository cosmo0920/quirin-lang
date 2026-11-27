#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: classify every IND, COND and RET leaf of the
current run by typed path, join the alias census, rank shapes, run the closure.

usage: shape_census.py RUN_DIR ALIAS_DIR OUT_DIR [--jobs N]

RUN_DIR   run_census.py output (census build; `access-path-units/`, report)
ALIAS_DIR alias_census.py output for the same run (leaves.jsonl.gz, unit-map.json)
OUT_DIR   shapes.json, leaves.jsonl.gz, unmatched.json

Reuses W46's `access_census.py` (typed AST, constructs, paths) and W47's
`alias_census.py` unit matching by import; adds no scanner. Document unit i is
analysed from the same text alias_census joined it to (its `unit-map.json`).

Every leaf of the report is put in one *family* (IND, COND, RET, ALIAS, UNC-RUN,
UNC-EXT, or the report's other categories) and carries a set of *tags*, the
smallest things one card could remove to free that leaf:

* IND / COND / RET: the typed path's refusal causes (`union`, `cast`,
  `pointer-valued-store`, ...), each a tag; with none, `expr:<construct>:<call-arg|direct>:<root>`
  (an expressible path the typed layer still refuses: its residual reason is the
  root premise, not the path); with no path at all, `none`;
* ALIAS: `alias:<discharge group>`;
* anything else: `fix:<family>` (never removed by this census).
"""

import collections
import gzip
import importlib.util
import json
import multiprocessing
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


AC = load_module("access_census", "docs/reports/w46-access-path-census/scripts/access_census.py")
AC.CLANG = "/usr/lib/llvm-21/bin/clang"
UNC = re.compile(r"call to uncontracted function `([^`]*)`")


def jsonl_gz(path):
    with gzip.open(path, "rt") as handle:
        handle.readline()
        return [json.loads(line) for line in handle]


def leaf_family(leaf, defined):
    category = leaf["category"]
    if category == "unknown_alias":
        return "ALIAS", None
    if category != "unsupported":
        return category.upper(), None
    fam, sub = AC.family(leaf["description"])
    if fam:
        return fam, sub
    match = UNC.match(leaf["description"])
    if match:
        return ("UNC-RUN" if match.group(1) in defined else "UNC-EXT"), match.group(1)
    description = leaf["description"]
    for key, name in (("callback", "CALLBACK"), ("slot", "SLOT"), ("out-parameter", "OUTPARAM"),
                      ("retention", "RETAIN"), ("retain", "RETAIN"), ("reference", "REFCOUNT"),
                      ("nullable", "NULLABLE"), ("va_", "VA")):
        if key in description:
            return name, None
    return "SYNTAX", None


def path_tags(constructs):
    """Tags of an IND/COND/RET leaf from the paths of its constructs."""
    paths = [(c, p) for c in constructs for p in c["paths"]]
    if not paths:
        return {"none"}, {}
    causes = set()
    for _, p in paths:
        causes.update(p["causes"])
    facts = {
        "roots": sorted({p["root_kind"] for _, p in paths}),
        "values": sorted({p["value"] for _, p in paths}),
        "accesses": sorted({p["access"] for _, p in paths}),
        "positions": sorted({("call-arg" if p["call_argument"] else "direct") for _, p in paths}),
        "constructs": sorted({c["kind"] if c["kind"] != "condition" else "cond-" + str(c["clause"]) for c in constructs}),
        "max_hops": max(p["hops"] for _, p in paths),
    }
    if causes:
        return {c if not c.startswith("root:") else "root-other" for c in causes}, facts
    top = paths[0][1]
    kinds = "+".join(facts["roots"])
    pos = "+".join(facts["positions"])
    return {f"expr:{facts['constructs'][0]}:{pos}:{kinds}"}, facts


def main():
    arguments = sys.argv[1:]
    jobs = 8
    if "--jobs" in arguments:
        index = arguments.index("--jobs")
        jobs = int(arguments[index + 1])
        del arguments[index:index + 2]
    run, alias_dir, out = (pathlib.Path(a) for a in arguments)
    out.mkdir(parents=True, exist_ok=True)
    report = json.load(open(run / "resource-diagnostics.json"))
    unit_map = json.load(open(alias_dir / "unit-map.json"))["unit_of"]
    alias_rows = jsonl_gz(alias_dir / "leaves.jsonl.gz")
    directory = run / "access-path-units"
    records = [json.loads(line) for line in (directory / "units.jsonl").read_text().splitlines()]
    needed = set(unit_map.values())
    with multiprocessing.Pool(jobs) as pool:
        extracted = dict(pool.map(AC.extract_unit, [(str(directory), r) for r in records if r["unit_seq"] in needed],
                                  chunksize=1))
    defined = set()
    storage = {}
    for index, document in enumerate(report["units"]):
        seq = unit_map.get(f"{document['file']}#{index}")
        if seq is None or "definitions" not in extracted.get(seq, {}):
            continue
        for name, value in extracted[seq]["definitions"].items():
            defined.add(name)
            storage.setdefault(name, []).append((document["file"], value["storage"]))

    alias_by = collections.defaultdict(list)
    for row in alias_rows:
        alias_by[(row["file"], row["function"], row.get("unit_seq"))].append(row)

    unmatched = {"leaves": [], "functions": []}
    leaves_out = []
    functions = []  # one per (unit index, function)
    for index, document in enumerate(report["units"]):
        seq = unit_map.get(f"{document['file']}#{index}")
        definitions = extracted[seq]["definitions"] if seq in extracted and "definitions" in extracted[seq] else {}
        for function in document["functions"]:
            name = function["function"]
            definition = definitions.get(name)
            items = []  # (family, tags, detail)
            groups = collections.defaultdict(list)
            alias_used = alias_by.get((document["file"], name, seq), [])
            aliases = [r for r in alias_used if r["function"] == name]
            for leaf in function["diagnostics"]:
                fam, sub = leaf_family(leaf, defined)
                if fam == "ALIAS":
                    continue  # taken from the alias census, below
                if fam in ("IND", "COND", "RET"):
                    groups[(fam, sub, leaf["line"])].append(leaf)
                elif fam in ("UNC-RUN", "UNC-EXT"):
                    items.append((fam, {fam + ":" + sub}, sub))
                else:
                    items.append((fam, {"fix:" + fam}, None))
            for (fam, sub, line), group in groups.items():
                if definition is None:
                    items.extend((fam, {"unjoined"}, None) for _ in group)
                    unmatched["leaves"].append({"file": document["file"], "function": name, "line": line, "family": fam})
                    continue
                at_line = [c for c in definition["constructs"] if line in c["lines"]]
                constructs = [c for c in at_line if AC.construct_matches(c, fam, sub)]
                if not constructs and fam == "IND":
                    constructs = [c for c in at_line if c["kind"] == "return" and c["paths"]] \
                        or [c for c in at_line if c["kind"] == "condition" and c["paths"]] \
                        or [c for c in at_line if c["kind"] == "return"]
                if not constructs:
                    items.extend((fam, {"unjoined"}, None) for _ in group)
                    unmatched["leaves"].append({"file": document["file"], "function": name, "line": line,
                                                "family": fam, "count": len(group)})
                    continue
                tags, facts = path_tags(constructs)
                for _ in group:
                    items.append((fam, tags, facts))
                leaves_out.append({"file": document["file"], "function": name, "line": line, "family": fam,
                                   "sub": sub, "leaves": len(group), "tags": sorted(tags), "facts": facts})
            for row in aliases:
                items.append(("ALIAS", {"alias:" + row["group"]}, row["class"]))
            functions.append({"i": index, "file": document["file"], "library": document.get("library"),
                              "function": name, "status": function["status"], "items": items,
                              "unit_status": document["status"]})
    with gzip.open(out / "leaves.jsonl.gz", "wt") as handle:
        handle.write(json.dumps({"schema": "qcore.current-access-alias-census.v1"}) + "\n")
        for row in leaves_out:
            handle.write(json.dumps(row) + "\n")
    (out / "unmatched.json").write_text(json.dumps(unmatched, indent=1) + "\n")
    with gzip.open(out / "functions.jsonl.gz", "wt") as handle:
        for f in functions:
            handle.write(json.dumps({**f, "items": [[a, sorted(b), c if isinstance(c, str) or c is None else None]
                                                    for a, b, c in f["items"]]}) + "\n")
    print(f"functions {len(functions)}, leaf groups {len(leaves_out)}, unjoined groups {len(unmatched['leaves'])}")


if __name__ == "__main__":
    main()
