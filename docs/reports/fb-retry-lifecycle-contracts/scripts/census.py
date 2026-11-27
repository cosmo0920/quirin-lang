#!/usr/bin/env python3
"""census.py -- FB-RETRY-LIFECYCLE-CONTRACTS: the Fluent Bit retry-lifecycle
census, written down for a human reviewer.

    python3 census.py --report SYSTEM.json --source TREE --label LABEL \
        --out-md CENSUS.md --out-json census.json [--base-report BASE.json]

Reads one fluent-bit-system route report (`quirin-audit current --selection
system-selection.json`; `resource-diagnostics.json`), the prepared Fluent Bit
tree it measured, this repository's provider contracts, the classification
table beside this script (`classification.json`, hand-written: one entry per
uncontracted callee, the reviewer-checkable judgement) and the repository's
Rust test files. Writes CENSUS.md and a compact census.json carrying the
SHA-256 of every file it read.

The lifecycle rows are every function row of the five lifecycle units
(`LIFECYCLE`): both compilation configurations of each, so a function appears
once per configuration. Rows are keyed by (file, function, occurrence), as
FB-RETRY-OR6-ELEMENT-IDENTITY's `moved.py` keys them.

With `--base-report`, the census also lists, per contract the classification
marks `written`, which leaves of the base report cleared at this report and
which rows moved status.

Everything here is a reading of the reports and of the source text: it
decides nothing and changes no verdict."""
import argparse
import collections
import hashlib
import json
import pathlib
import re
import sys

LIFECYCLE = [
    "src/flb_task.c",
    "src/flb_output.c",
    "src/flb_scheduler.c",
    "src/flb_engine_dispatch.c",
    "src/flb_engine.c",
]
UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`(?: in branch condition)?$")
TICK = re.compile(r"`[^`]*`")
HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CONTRACT_ROOTS = [REPO / "providers"]
TEST_ROOTS = [REPO / "crates", REPO / "compiler" / "tests", REPO / "providers"]
READ = {}


def sha(path):
    path = pathlib.Path(path)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    try:
        key = str(path.resolve().relative_to(REPO))
    except ValueError:
        key = str(path)
    READ[key] = digest
    return digest


def rows(report):
    data = json.load(open(report))
    out = collections.OrderedDict()
    for unit in data["units"]:
        if unit.get("file") not in LIFECYCLE:
            continue
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            key = (unit["file"], function["function"], seen[function["function"]])
            while key in out:  # the second configuration of the same file
                key = (key[0], key[1], key[2] + 1)
            out[key] = function
    return out


def leaves(function):
    return [(d.get("description", ""), d.get("origin") or d.get("line")) for d in function.get("diagnostics", [])]


def cause(description):
    match = UNCONTRACTED.match(description)
    if match:
        return "call to uncontracted function"
    return TICK.sub("`…`", description)


def merged(blocked):
    """Collapse configurations with identical status and leaves."""
    out = collections.OrderedDict()
    for (file, name, _), function in blocked:
        group = tuple((d, str(ln)[len(file) + 1:] if str(ln).startswith(file + ":") else ln)
                      for d, ln in leaves(function))
        key = (file, name, function["status"], group)
        out[key] = out.get(key, 0) + 1
    return list(out.items())


def contract_modules():
    """Every `extern fn NAME` in the provider contract trees: name -> [(module, clauses)]."""
    out = collections.defaultdict(list)
    for root in CONTRACT_ROOTS:
        for path in sorted(root.glob("**/contracts/**/*.qv")):
            text = path.read_text()
            for match in re.finditer(r"^extern fn (\w+)\(.*?^\};", text, re.M | re.S):
                body = match.group(0)
                clauses = [c.strip().rstrip(";") for c in re.findall(
                    r"^\s+((?:requires|ensures|quant)\b[^;]*;)", body, re.M)]
                out[match.group(1)].append((str(path.relative_to(REPO)), clauses))
    return out


def test_index(names, modules):
    """name -> [file] for each Rust test file that exercises the contract: it
    states the declaration itself (`extern fn NAME(` in its own contract
    text, inline or in a `tests/**/fixtures` file it includes) or loads a
    module declaring it (`include_str!` of that module). A lexical reading:
    a file that loads the module and never calls the function is listed too,
    so a listed file is a candidate the reviewer opens, not a proof."""
    found = collections.defaultdict(set)
    for root in TEST_ROOTS:
        for path in sorted(root.glob("**/tests/**/*")):
            if "/target/" in str(path) or not path.is_file() or path.suffix not in (".rs", ".qv", ".qr", ".qcore"):
                continue
            text = path.read_text(errors="replace")
            where = str(path.relative_to(REPO))
            for name in names:
                if f"extern fn {name}(" in text:
                    found[name].add(where)
                    continue
                for module, _ in modules.get(name, []):
                    tail = module.split("/contracts/", 1)[-1]
                    if path.suffix == ".rs" and re.search(r'include_str!\(\s*"[^"]*' + re.escape(tail) + '"', text):
                        found[name].add(where)
                        break
    return {k: sorted(v) for k, v in found.items()}


def clip(text, width=110):
    text = " ".join(text.split())
    return text if len(text) <= width else text[: width - 1] + "…"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--base-report")
    parser.add_argument("--source", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--out-md", required=True)
    parser.add_argument("--out-json", required=True)
    args = parser.parse_args()

    sha(args.report)
    if args.base_report:
        sha(args.base_report)
    source = pathlib.Path(args.source)
    sources = {}
    for name in LIFECYCLE:
        sources[name] = hashlib.sha256((source / name).read_bytes()).hexdigest()
    classification_path = HERE / "classification.json"
    sha(classification_path)
    classification = json.load(open(classification_path))
    callees_table = classification["callees"]

    table = rows(args.report)
    base_table = rows(args.base_report) if args.base_report else None
    modules = contract_modules()

    status = collections.Counter(f["status"] for f in table.values())
    per_file = collections.defaultdict(collections.Counter)
    for (file, _, _), function in table.items():
        per_file[file][function["status"]] += 1

    # Contracts in force: each declaration a lifecycle row consulted in force.
    in_force = collections.defaultdict(set)
    summaries = collections.defaultdict(set)
    for (file, name, occ), function in table.items():
        deps = function.get("contract_dependencies") or {}
        for decl in deps.get("declarations", []):
            in_force[decl].add((file, name))
        for summary in deps.get("summaries", []):
            summaries[summary].add((file, name))

    # Blocking reasons.
    blocked = [(k, f) for k, f in table.items() if f["status"] != "Safe"]
    by_cause = collections.defaultdict(lambda: [0, set()])
    uncontracted = collections.defaultdict(lambda: [0, set()])
    for key, function in blocked:
        for description, _line in leaves(function):
            group = by_cause[cause(description)]
            group[0] += 1
            group[1].add(key)
            match = UNCONTRACTED.match(description)
            if match:
                entry = uncontracted[match.group(1)]
                entry[0] += 1
                entry[1].add(key)
    ranked = sorted(uncontracted.items(), key=lambda kv: (-len(kv[1][1]), -kv[1][0], kv[0]))

    tests = test_index(list(in_force) + [c for c, _ in ranked], modules)

    missing = [c for c, _ in ranked if c not in callees_table]
    if missing:
        print("unclassified uncontracted callees: " + ", ".join(missing), file=sys.stderr)
        return 3

    # Movement against the base report, per written contract.
    movement = {}
    if base_table is not None:
        for callee, entry in callees_table.items():
            if entry.get("class") != "a" or not entry.get("written"):
                continue
            cleared, moved = [], []
            leaf = f"call to uncontracted function `{callee}`"
            for key, before in base_table.items():
                after = table.get(key)
                hits = [ln for d, ln in leaves(before) if d == leaf]
                if not hits:
                    continue
                left = [ln for d, ln in leaves(after or {}) if d == leaf]
                for line in hits:
                    if line not in left:
                        cleared.append({"file": key[0], "function": key[1],
                                        "occurrence": key[2], "line": line})
                if after and after["status"] != before["status"]:
                    moved.append({"file": key[0], "function": key[1], "occurrence": key[2],
                                  "move": f"{before['status']} -> {after['status']}"})
            movement[callee] = {"cleared": cleared, "moved": moved}
        all_moves = []
        for key in sorted(set(base_table) | set(table)):
            a = base_table.get(key, {}).get("status", "absent")
            b = table.get(key, {}).get("status", "absent")
            if a != b:
                all_moves.append({"file": key[0], "function": key[1], "occurrence": key[2],
                                  "move": f"{a} -> {b}"})
    else:
        all_moves = None

    # census.json (compact).
    doc = {
        "schema": "fb-retry-lifecycle-census/1",
        "label": args.label,
        "lifecycle_units": LIFECYCLE,
        "sources_sha256": sources,
        "read_sha256": READ,
        "status": dict(sorted(status.items())),
        "status_by_file": {f: dict(sorted(c.items())) for f, c in sorted(per_file.items())},
        "contracts_in_force": {
            name: {
                "modules": [m for m, _ in modules.get(name, [])],
                "rows": len(rs),
                "functions": sorted({n for _, n in rs}),
                "tests": tests.get(name, [])[:6],
                "tests_total": len(tests.get(name, [])),
            } for name, rs in sorted(in_force.items())
        },
        "blocking_causes": {
            c: {"leaves": n, "rows": len(rs)}
            for c, (n, rs) in sorted(by_cause.items(), key=lambda kv: (-len(kv[1][1]), kv[0]))
        },
        "uncontracted": [
            {"callee": c, "rows": len(rs), "leaves": n,
             "class": callees_table[c]["class"],
             "written": bool(callees_table[c].get("written"))}
            for c, (n, rs) in ranked
        ],
        "non_safe_rows": [
            {"file": file, "function": name, "rows": occs, "status": status,
             "leaves": [f"{line}: {text}" for text, line in group]}
            for (file, name, status, group), occs in merged(blocked)
        ],
    }
    if base_table is not None:
        doc["movement"] = movement
        doc["moves"] = all_moves
    pathlib.Path(args.out_json).write_text(json.dumps(doc, indent=1, sort_keys=True) + "\n")

    # CENSUS.md.
    out = []
    w = out.append
    w(f"# FB-RETRY-LIFECYCLE-CONTRACTS: retry-lifecycle census ({args.label})")
    w("")
    w("Generated by [scripts/census.py](scripts/census.py) from one fluent-bit-system")
    w("route report; do not edit by hand. The classification column is")
    w("[scripts/classification.json](scripts/classification.json), the reviewed")
    w("judgement. Source digests and every file read are in")
    w("[census.json](census.json). Lines are the source `origin` each leaf")
    w("carries (`file:line` of the 046accf6 tree).")
    w("")
    w("## Rows")
    w("")
    w("| Unit | Safe | Pending | Unknown | Unsafe |")
    w("|---|---|---|---|---|")
    for file in LIFECYCLE:
        c = per_file.get(file, collections.Counter())
        w(f"| `{file}` | {c['Safe']} | {c['Pending']} | {c['Unknown']} | {c['Unsafe']} |")
    w(f"| **all** | {status['Safe']} | {status['Pending']} | {status['Unknown']} | {status['Unsafe']} |")
    w("")
    w("Both compilation configurations of each unit are rows. Unsafe rows are")
    w("provisional violation candidates, never upstream defects.")
    w("")
    w("## Contracts in force")
    w("")
    w("Each declaration a lifecycle row consulted in force at a call")
    w("(`contract_dependencies.declarations`), with its module and clauses, the")
    w("functions whose rows use it, and the Rust tests whose body names it (a")
    w("lexical index: up to three named, then the count; \"none\" where none does).")
    w("")
    w("| Contract | Module | Clauses | Rows | Functions | Tests |")
    w("|---|---|---|---|---|---|")
    for name, rs in sorted(in_force.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        mods = modules.get(name, [])
        mod_text = "<br>".join(f"`{m.split('contracts/', 1)[-1]}`" for m, _ in mods) or "(not in a provider tree)"
        resource = [cl for _, cl in mods if any("resource_" in c for c in cl)]
        clauses = resource[0] if resource else (mods[0][1] if mods else [])
        clause_text = "<br>".join(f"`{clip(c)}`" for c in clauses if not c.startswith("requires resource_contract_v1")) or "-"
        funcs = sorted({n for _, n in rs})
        func_text = ", ".join(f"`{n}`" for n in funcs[:6]) + (f" (+{len(funcs) - 6})" if len(funcs) > 6 else "")
        t = tests.get(name, [])
        test_text = "<br>".join(f"`{x}`" for x in t[:3]) + (f"<br>(+{len(t) - 3})" if len(t) > 3 else "") if t else "none"
        w(f"| `{name}` | {mod_text} | {clause_text} | {len(rs)} | {func_text} | {test_text} |")
    w("")
    if summaries:
        w("In-unit derived summaries the rows rested on (not contracts; listed for")
        w("completeness): " + ", ".join(f"`{s}` ({len(r)})" for s, r in sorted(summaries.items())) + ".")
        w("")
    w("## Blocking reasons")
    w("")
    w(f"{len(blocked)} non-Safe rows. Causes (backticked names folded), ranked by rows:")
    w("")
    w("| Cause | Rows | Leaves |")
    w("|---|---|---|")
    for c, (n, rs) in sorted(by_cause.items(), key=lambda kv: (-len(kv[1][1]), -kv[1][0], kv[0])):
        w(f"| {c} | {len(rs)} | {n} |")
    w("")
    w("### Uncontracted callees, ranked, with their classification")
    w("")
    w("Classes: **(a)** statable now from the callee's source; **(b)** needs")
    w("vocabulary or a rule the schema lacks; **(c)** the callee's own body is the")
    w("blocker; **(d)** out of scope (the sibling card's file, an upstream library).")
    w("")
    w("| Callee | Rows | Leaves | Class | Reason | Owner / hand-off |")
    w("|---|---|---|---|---|---|")
    for c, (n, rs) in ranked:
        e = callees_table[c]
        cls = e["class"] + (" (written)" if e.get("written") else "")
        reason = e["reason"] + (f". Clauses: `{e['clause']}`" if e.get("clause") else "")
        w(f"| `{c}` | {len(rs)} | {n} | {cls} | {reason} | {e.get('owner', '-')} |")
    w("")
    w("### Every non-Safe row and its leaves")
    w("")
    w("One entry per function; `rows` counts the configurations whose row has")
    w("exactly these leaves (a configuration that differs is its own entry).")
    w("Lines are source lines of the row's own unit.")
    w("")
    for (file, name, status, group), occs in merged(blocked):
        w(f"- `{file}` `{name}` ({occs} row{'s' if occs > 1 else ''}): **{status}**")
        for text, line in group:
            w(f"  - {line}: {text}")
    w("")
    if base_table is not None:
        w("## Movement against the base report")
        w("")
        w("Per contract this card wrote (classification `written`): each base leaf")
        w("`call to uncontracted function` naming it that is gone here, and each row")
        w("that moved status. Then every status move of the lifecycle rows.")
        w("")
        for callee, m in sorted(movement.items()):
            w(f"- `{callee}`: {len(m['cleared'])} leaves cleared, {len(m['moved'])} rows moved")
            for c in m["cleared"]:
                w(f"  - cleared {c['file']} `{c['function']}` #{c['occurrence']} line {c['line']}")
            for c in m["moved"]:
                w(f"  - moved {c['file']} `{c['function']}` #{c['occurrence']}: {c['move']}")
        w("")
        w("Every status move:" if all_moves else "No lifecycle row moved status.")
        for mv in all_moves:
            w(f"- {mv['file']} `{mv['function']}` #{mv['occurrence']}: {mv['move']}")
        w("")
    pathlib.Path(args.out_md).write_text("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
