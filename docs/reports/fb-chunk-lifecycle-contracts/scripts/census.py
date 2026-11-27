#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card FB-CHUNK-LIFECYCLE-CONTRACTS: the chunk-lifecycle contract census.

usage: census.py --label LABEL --revision REV --tree SIDE_TREE --checkout FLB
                 --system SYSTEM_REPORT --current CURRENT_REPORT
                 --classification classification.json --out-json census.json
                 --out-md CENSUS.md [--base-census BASE_census.json]

Reads the two measurement reports of one revision (the fluent-bit-system
route and opt-in `current --monkey-linked`) and writes, for the lifecycle
units `plugins/in_storage_backlog/sb.c` and `src/flb_input_chunk.c`:

* the contracts in force: every declaration a lifecycle row resolves
  (`contract_dependencies.declarations`), the module(s) the run loaded it
  from (the `.contracts.json` sidecar), its clauses, the rows that use it,
  and the test files of SIDE_TREE that name it;
* every non-Safe row with its leaves (description, original coordinate),
  grouped by cause, and the `call to uncontracted function X` leaves grouped
  by X and ranked;
* the class of every uncontracted callee, from the hand-written
  classification.json (the script fails if a callee is unclassified, so an
  unknown callee is never omitted), with the callee's own row status where
  its body is on the route.

With --base-census it adds, per route, the rows whose status or leaves
moved and the leaves cleared per callee. It records the SHA-256 of every
file it read. Deterministic: no timestamps; sorted output."""
import argparse
import collections
import hashlib
import json
import os
import re
import sys

UNITS = ("plugins/in_storage_backlog/sb.c", "src/flb_input_chunk.c")
SOURCES = ("plugins/in_storage_backlog/sb.c", "src/flb_input_chunk.c", "src/flb_input.c",
           "lib/chunkio/src/cio_chunk.c", "lib/chunkio/src/cio_file.c")
UNCONTRACTED = re.compile(r"call to uncontracted function `([^`]+)`")
CLASS_NAMES = {
    "a": "statable now from the callee's source (written by this card)",
    "b": "needs vocabulary or a rule the schema lacks",
    "c": "the callee's own body is the blocker",
    "d": "out of scope for this card's files (hand-off)",
}


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def cause(description):
    return re.sub(r"`[^`]*`", "`X`", description)


def lifecycle_rows(report):
    """{(unit, function): row} over the lifecycle units; identical rows of a
    unit analysed under several configurations are kept once, and a
    difference between configurations is recorded, never merged."""
    rows, differing, units = {}, [], {}
    for unit in report["units"]:
        if not unit["file"].endswith(UNITS):
            continue
        name = next(u for u in UNITS if unit["file"].endswith(u))
        units.setdefault(name, set()).add(unit.get("status"))
        for function in unit.get("functions", []):
            leaves = sorted(
                (d["description"], d.get("origin") or "", d["line"])
                for d in function.get("diagnostics", [])
            )
            deps = function.get("contract_dependencies") or {}
            row = {
                "status": function["status"],
                "leaves": [list(leaf) for leaf in leaves],
                "declarations": sorted(deps.get("declarations", [])),
                "withdrawn": sorted(deps.get("withdrawn", [])),
            }
            key = (name, function["function"])
            if key in rows and rows[key] != row:
                differing.append("::".join(key))
                continue
            rows.setdefault(key, row)
    return rows, sorted(set(differing)), {k: sorted(v) for k, v in units.items()}


def body_status(report):
    """{function: [status, ...]} of every function row on the route (the
    callee's own body, where the route analyses it)."""
    out = collections.defaultdict(set)
    for unit in report["units"]:
        for function in unit.get("functions", []):
            out[function["function"]].add(function["status"])
    return {k: sorted(v) for k, v in out.items()}


def declared_modules(contracts_sidecar):
    """{function: [(module, clauses)]} from the modules the run loaded."""
    sources = json.load(open(contracts_sidecar))["contracts"]
    out = collections.defaultdict(list)
    for path, text in sources.items():
        module = path.split("/providers/", 1)[-1] if "/providers/" in path else os.path.basename(path)
        for match in re.finditer(r"extern fn ([A-Za-z_]\w*)\s*\(", text):
            start = text.find("contract {", match.end())
            semi = text.find(";", match.end())
            if start == -1 or (semi != -1 and semi < start and "contract" not in text[match.end():semi]):
                clauses = []
            else:
                end = text.find("};", start)
                clauses = [
                    " ".join(line.split())
                    for line in text[start + len("contract {"):end].splitlines()
                    if line.strip() and not line.strip().startswith("//")
                ]
            out[match.group(1)].append((module, clauses))
    return out


def tests_naming(tree, names):
    """{name: ["path::fn", ...]} over SIDE_TREE's Rust test sources."""
    found = collections.defaultdict(set)
    pattern = re.compile(r"\b(" + "|".join(sorted(map(re.escape, names))) + r")\b") if names else None
    roots = [os.path.join(tree, "crates"), os.path.join(tree, "providers")]
    for root in roots:
        for directory, _, files in os.walk(root):
            if "/target" in directory or "/fixtures" in directory and False:
                continue
            rel = os.path.relpath(directory, tree)
            if "tests" not in rel.split(os.sep) and not rel.endswith("src"):
                continue
            for file in files:
                if not file.endswith(".rs"):
                    continue
                path = os.path.join(directory, file)
                text = open(path, errors="replace").read()
                if pattern is None or not pattern.search(text):
                    continue
                current = None
                pending_test = False
                for line in text.splitlines():
                    stripped = line.strip()
                    if stripped.startswith("#[test]"):
                        pending_test = True
                        continue
                    m = re.match(r"(?:pub\s+)?fn\s+([A-Za-z_]\w*)", stripped)
                    if m:
                        current = m.group(1) if pending_test else None
                        pending_test = False
                    if current:
                        for hit in pattern.findall(line):
                            found[hit].add(f"{os.path.relpath(path, tree)}::{current}")
    return {k: sorted(v) for k, v in found.items()}


def route_census(rows, classification, bodies):
    nonsafe = {k: v for k, v in rows.items() if v["status"] != "Safe"}
    causes = collections.Counter()
    cause_rows = collections.defaultdict(set)
    callee_leaves = collections.Counter()
    callee_rows = collections.defaultdict(set)
    for (unit, function), row in nonsafe.items():
        for description, origin, _ in row["leaves"]:
            match = UNCONTRACTED.search(description)
            if match:
                callee_leaves[match.group(1)] += 1
                callee_rows[match.group(1)].add(f"{unit.split('/')[-1]}::{function}")
            else:
                causes[cause(description)] += 1
                cause_rows[cause(description)].add(f"{unit.split('/')[-1]}::{function}")
    missing = sorted(c for c in callee_leaves if c not in classification)
    callees = [
        {
            "callee": callee,
            "leaves": count,
            "rows": sorted(callee_rows[callee]),
            "class": classification.get(callee, {}).get("class", "UNCLASSIFIED"),
            "callee_body_status": bodies.get(callee, ["not on this route"]),
        }
        for callee, count in sorted(callee_leaves.items(), key=lambda kv: (-kv[1], kv[0]))
    ]
    return {
        "status_counts": dict(sorted(collections.Counter(
            f"{k[0].split('/')[-1]}:{v['status']}" for k, v in rows.items()).items())),
        "causes": [
            {"cause": c, "leaves": n, "rows": len(cause_rows[c])}
            for c, n in sorted(causes.items(), key=lambda kv: (-kv[1], kv[0]))
        ],
        "uncontracted": callees,
        "unclassified": missing,
    }


def contracts_in_force(routes_rows, modules, tests):
    users = collections.defaultdict(set)
    for route, rows in routes_rows.items():
        for (unit, function), row in rows.items():
            for name in row["declarations"]:
                users[name].add(f"{route}:{unit.split('/')[-1]}::{function}")
    out = []
    for name in sorted(users):
        # The same module is keyed by its provider path on one route and by
        # its load-path name on the other: keep the longest spelling.
        names = sorted({m for m, _ in modules.get(name, [])}, key=len, reverse=True)
        kept = [m for i, m in enumerate(names) if not any(n.endswith("/" + m) for n in names[:i])]
        out.append({
            "function": name,
            "modules": sorted(kept) or ["(not in a loaded module)"],
            "clauses": (modules.get(name) or [(None, [])])[0][1],
            "rows": sorted(users[name]),
            "tests": (tests.get(name, []) or ["none"])[:5],
            "tests_count": len(tests.get(name, [])),
        })
    return out


def moved(base, tip):
    out = {}
    for route in tip:
        b, t = base.get(route, {}).get("all_rows", {}), tip[route]["all_rows"]
        rows = []
        for key in sorted(set(b) | set(t)):
            if b.get(key) == t.get(key):
                continue
            bl = collections.Counter(tuple(x) for x in (b.get(key) or {}).get("leaves", []))
            tl = collections.Counter(tuple(x) for x in (t.get(key) or {}).get("leaves", []))
            rows.append({
                "row": key,
                "status": [(b.get(key) or {}).get("status"), (t.get(key) or {}).get("status")],
                "cleared": [list(x) for x in sorted((bl - tl).elements())],
                "added": [list(x) for x in sorted((tl - bl).elements())],
            })
        out[route] = rows
    return out


def short(clause):
    return clause if len(clause) <= 100 else clause[:97] + "..."


def nonsafe(route):
    return {k: v for k, v in route["all_rows"].items() if v["status"] != "Safe"}


def markdown(census, classification):
    lines = []
    w = lines.append
    w(f"# Chunk-lifecycle contract census ({census['label']}, `{census['revision'][:9]}`)")
    w("")
    w("Generated by `scripts/census.py`; do not edit by hand. Units: "
      "`plugins/in_storage_backlog/sb.c` and `src/flb_input_chunk.c` at Fluent Bit "
      f"`{census['fluent_bit_revision'][:12]}`. Leaves are `description` at the original "
      "`file:line` (`origin`; the report's preprocessed `line` is kept in census.json). "
      "Rows identical across a unit's configurations are listed once.")
    w("")
    for route, data in census["routes"].items():
        w(f"Route `{route}`: " + ", ".join(f"{k} {v}" for k, v in data["status_counts"].items())
          + f"; unit statuses {data['unit_status']}"
          + (f"; rows differing by configuration: {data['differing']}" if data["differing"] else "") + ".")
        w("")
    if census.get("moved") is not None:
        w("## What moved against the base census")
        w("")
        for route, rows in census["moved"].items():
            w(f"### `{route}`: {len(rows)} row(s) changed")
            w("")
            for row in rows:
                w(f"* `{row['row']}`: {row['status'][0]} -> {row['status'][1]}")
                for leaf in row["cleared"]:
                    w(f"  * cleared: {leaf[0]} ({leaf[1]})")
                for leaf in row["added"]:
                    w(f"  * added: {leaf[0]} ({leaf[1]})")
            if not rows:
                w("No row moved.")
            w("")
    w("## 1. Contracts in force")
    w("")
    w("Every declaration a lifecycle row resolves at a call (`contract_dependencies."
      "declarations`), on either route. *Tests* are the test functions of this tree "
      "that name the function (\"none\" where none does).")
    w("")
    w("| Function | Module(s) | Clauses | Rows | Tests |")
    w("|---|---|---|---|---|")
    for c in census["contracts_in_force"]:
        rows = sorted({r.split(":", 1)[1] for r in c["rows"]})
        tests = c["tests"][:3] + ([f"... {c['tests_count'] - 3} more"] if c["tests_count"] > 3 else [])
        w(f"| `{c['function']}` | {'<br>'.join(c['modules'])} | "
          f"{'<br>'.join('`' + short(x).replace('|', '&#124;') + '`' for x in c['clauses'] if not x.startswith('requires resource_contract_v1')) or '-'} | "
          f"{len(rows)}: {', '.join(rows[:6])}{' ...' if len(rows) > 6 else ''} | "
          f"{'<br>'.join(tests)} |")
    w("")
    w("## 2. Blocking reasons")
    w("")
    for route, data in census["routes"].items():
        w(f"### Route `{route}`")
        w("")
        w("Causes other than an uncontracted call, by leaf count:")
        w("")
        w("| Cause | Leaves | Rows |")
        w("|---|---|---|")
        for c in data["causes"]:
            w(f"| {c['cause'].replace('|', '&#124;')} | {c['leaves']} | {c['rows']} |")
        w("")
        w("`call to uncontracted function X`, by X, ranked:")
        w("")
        w("| X | Leaves | Rows | Class | X's own row |")
        w("|---|---|---|---|---|")
        for c in data["uncontracted"]:
            w(f"| `{c['callee']}` | {c['leaves']} | {len(c['rows'])}: {', '.join(c['rows'])} | "
              f"({c['class']}) | {', '.join(c['callee_body_status'])} |")
        w("")
    w("### Every non-Safe row and its leaves (system route)")
    w("")
    system = census["routes"].get("fluent-bit-system")
    for row, data in (nonsafe(system) if system else {}).items():
        w(f"* `{row}` ({data['status']}, {len(data['leaves'])} leaves)")
        for description, origin, _ in data["leaves"]:
            w(f"  * {description} ({origin})")
    w("")
    current = census["routes"].get("current")
    if current and system:
        system_rows = nonsafe(system)
        extra = {k: v for k, v in nonsafe(current).items() if system_rows.get(k) != v}
        w(f"### Rows whose leaves differ on the current route ({len(extra)})")
        w("")
        for row, data in extra.items():
            w(f"* `{row}` ({data['status']}, {len(data['leaves'])} leaves)")
            for description, origin, _ in data["leaves"]:
                w(f"  * {description} ({origin})")
        if not extra:
            w("None: every non-Safe row and leaf is the system route's.")
        w("")
    w("## 3. Classification of each uncontracted callee")
    w("")
    for key in "abcd":
        members = sorted(k for k, v in classification.items() if v["class"] == key)
        w(f"### ({key}) {CLASS_NAMES[key]}: {len(members)}")
        w("")
        for callee in members:
            entry = classification[callee]
            w(f"* `{callee}` ({entry.get('defined', '?')}): {entry['reason']}"
              + (f" Owner: {entry['owner']}." if entry.get("owner") else ""))
        w("")
    w("## Inputs (SHA-256)")
    w("")
    for path, digest in sorted(census["inputs"].items()):
        w(f"* `{path}` `{digest}`")
    w("")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    for flag in ("label", "revision", "tree", "checkout", "system", "current",
                 "classification", "out-json", "out-md"):
        parser.add_argument("--" + flag, required=True)
    parser.add_argument("--base-census")
    args = parser.parse_args()
    classification = json.load(open(args.classification))["callees"]
    inputs = {}
    routes, all_rows, modules = {}, {}, collections.defaultdict(list)
    revision = None
    for route, path in (("fluent-bit-system", args.system), ("current", args.current)):
        report = json.load(open(path))
        revision = report.get("fluent_bit_revision", revision)
        inputs[f"{route}:{os.path.basename(path)}"] = sha256(path)
        sidecar = path[:-len(".json")] + ".contracts.json"
        inputs[f"{route}:{os.path.basename(sidecar)}"] = sha256(sidecar)
        for name, entries in declared_modules(sidecar).items():
            for entry in entries:
                if entry not in modules[name]:
                    modules[name].append(entry)
        rows, differing, unit_status = lifecycle_rows(report)
        all_rows[route] = rows
        routes[route] = route_census(rows, classification, body_status(report))
        routes[route]["differing"] = differing
        routes[route]["unit_status"] = unit_status
        routes[route]["all_rows"] = {
            f"{k[0].split('/')[-1]}::{k[1]}": {"status": v["status"], "leaves": v["leaves"]}
            for k, v in sorted(rows.items())
        }
    for source in SOURCES:
        inputs[f"fluent-bit:{source}"] = sha256(os.path.join(args.checkout, source))
    inputs["classification.json"] = sha256(args.classification)
    names = set()
    for rows in all_rows.values():
        for row in rows.values():
            names.update(row["declarations"])
    tests = tests_naming(args.tree, names)
    census = {
        "schema": "fb-chunk-lifecycle-census/1",
        "label": args.label,
        "revision": args.revision,
        "fluent_bit_revision": revision or "",
        "routes": routes,
        "contracts_in_force": contracts_in_force(all_rows, modules, tests),
        "inputs": inputs,
    }
    unclassified = sorted({c for r in routes.values() for c in r["unclassified"]})
    if args.base_census:
        census["moved"] = moved(json.load(open(args.base_census))["routes"], routes)
        inputs["base-census"] = sha256(args.base_census)
    with open(args.out_json, "w") as handle:
        json.dump(census, handle, indent=1, sort_keys=True)
        handle.write("\n")
    with open(args.out_md, "w") as handle:
        handle.write(markdown(census, classification))
    if unclassified:
        print("unclassified callees: " + ", ".join(unclassified), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
