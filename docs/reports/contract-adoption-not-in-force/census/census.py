#!/usr/bin/env python3
"""CONTRACT-ADOPTION-NOT-IN-FORCE census.

Recomputes, from one `quirin-audit current` report and the tracked provider
contracts, every callee the report calls "uncontracted" although some tracked
provider module declares it (`extern fn NAME`), and records why no declaration
of it is in force on that route.

    python3 census.py --report REPORT.json --probe PROBE.json --out DIR

REPORT is the run's `resource-diagnostics.json`. PROBE is the output of
`target/debug/examples/provider_resource_views providers/contract-load-path.json`
(the loader's own view probe, which also prints each refused module's reason).
Run from the repository root. Reads tracked files only (`git ls-files`).
Writes DIR/census.json and DIR/census.md.
"""
import argparse
import collections
import hashlib
import json
import re
import subprocess
from pathlib import Path

UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`$")
EXTERN = re.compile(r"^\s*extern\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", re.M)
SELECTOR = re.compile(r"requires\s+(resource_contract_v1|bounds_contract_v1|resource_body_checked)\b")

# Where each declaring module's callees are tracked. A name is a ledger entry
# of docs/reviews/r0_3/acceptance_matrix.json (or the one this card registers).
OWNER = {
    "librdkafka/null_deref.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "fluent_bit/null_deref.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "chunkio/file_st_bounds.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "cmetrics/map_lifetime.qv": "CMT-MAP-OPTS-CONTRACT-ADOPTION",
    "cmetrics/opts_lifetime.qv": "CMT-MAP-OPTS-CONTRACT-ADOPTION",
    "cmetrics/metric_setters.qv": "CMT-MAP-OPTS-CONTRACT-ADOPTION",
    "cfl/intrusive_list.qv": "W45-SUB-OBJECT-STORE-FORMAL-VALUE",
    "cfl/arena_lifetime.qv": "W47-ARENA-POOL-RESULT-MODE",
    "chunkio/error_frame.qv": "W44-CALLEE-CROSS-UNIT-SUMMARY",
    "chunkio/error_state.qv": "W44-CALLEE-CROSS-UNIT-SUMMARY",
    "chunkio/counters.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "chunkio/context_destructor.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "chunkio/memory_backend.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "chunkio/stream_destructor.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "monkey/wrapper_paths.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "monkey/wait_source_policy.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "librdkafka/typed_queue.qv": "DECLARED-NAMES-RESOURCE-V1-PROJECTIONS",
    "fluent_bit/storage_backlog.qv": "W45-FB-BACKLOG-REACHABLE-RELEASE",
    "librdkafka/cjson.qv": "CLASS1-UNDECLARABLE-RETAINERS",
}


def tracked(prefix):
    out = subprocess.run(["git", "ls-files", "-z", prefix], capture_output=True, check=True).stdout
    return [p for p in out.decode().split("\0") if p]


def declaration_text(text, name):
    match = re.search(r"^\s*extern\s+fn\s+" + re.escape(name) + r"\b", text, re.M)
    return text[match.start(): text.find("};", match.start()) + 2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--probe", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    report_bytes = Path(args.report).read_bytes()
    report = json.loads(report_bytes)
    probe = json.loads(Path(args.probe).read_text())
    load_path = json.loads(Path("providers/contract-load-path.json").read_text())
    manifest = json.loads(Path("cycle19-resource-contracts.json").read_text())

    pairs = collections.defaultdict(set)
    sites = collections.Counter()
    evidence = collections.defaultdict(lambda: collections.defaultdict(set))
    for unit in report["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                match = UNCONTRACTED.match(diagnostic["description"])
                if match:
                    pairs[match.group(1)].add((unit["library"], unit["file"], function["function"]))
                    sites[match.group(1)] += 1
        body = unit.get("contract_body_evidence", {})
        for kind in ("failed", "withdrawn"):
            for name, reasons in body.get(kind, {}).items():
                evidence[name][unit["file"]].update(f"{kind}:{r}" for r in reasons)

    declared = collections.defaultdict(list)
    for path in tracked("providers"):
        if path.endswith(".qv") and "/contracts/" in path and "/tests/fixtures/" not in path:
            for match in EXTERN.finditer(Path(path).read_text()):
                declared[match.group(1)].append(path)

    crates = {}
    for record in load_path["crates"]:
        for module in record["modules"]:
            crates["providers/" + record["contracts"] + "/" + module] = record
    lock_pins = {}
    for record in load_path["crates"]:
        for kind, lock in (("bounds", "bounds-contracts.lock"), ("resource", "contracts.lock")):
            path = Path("providers") / record["name"] / lock
            if path.is_file():
                for entry in json.loads(path.read_text()).get("pinned", []):
                    revisions = sorted({r["revision"][:8] for i in entry["identities"] for r in i["reviewed_at"]})
                    lock_pins[(kind, entry["function"], entry["module"])] = revisions

    # Every resource-v1 declaration sits in its crate's resource view, and the
    # committed views are the probe's (the census's no-lookup-bug ground).
    stray = []
    for record in load_path["crates"]:
        computed = probe["crates"][record["name"]]
        assert computed["resource_view"] == record["resource_view"], record["name"]
        assert computed["bounds_view"] == record["bounds_view"], record["name"]
        assert computed["outer_view"] == record["outer_view"], record["name"]
        for module in record["modules"]:
            if module not in record["resource_view"]:
                text = Path("providers", record["contracts"], module).read_text()
                if re.search(r"requires\s+resource_contract_v1\s*;", text):
                    stray.append(f"{record['name']}:{module}")
    manifest_modules = set()
    for module in manifest["modules"]:
        if module.startswith("providers/"):
            manifest_modules.add(module)
        else:
            crate, rest = module.split(":", 1)
            manifest_modules.add("providers/" + next(r for r in load_path["crates"] if r["name"] == crate)["contracts"] + "/" + rest)
    view_modules = {"providers/" + r["contracts"] + "/" + m for r in load_path["crates"] for m in r["resource_view"]}

    rows = []
    for name in sorted(set(pairs) & set(declared), key=lambda n: (-len(pairs[n]), n)):
        declarations = []
        for path in sorted(set(declared[name])):
            record = crates[path]
            module = path[len("providers/" + record["contracts"] + "/"):]
            selectors = sorted(set(SELECTOR.findall(declaration_text(Path(path).read_text(), name))))
            pin_kind = "bounds" if "bounds_contract_v1" in selectors else "resource" if "resource_contract_v1" in selectors else None
            declarations.append({
                "crate": record["name"],
                "module": module,
                "in_module_list": True,
                "resource_view": module in record["resource_view"],
                "bounds_view": module in record["bounds_view"],
                "outer_view": module in record["outer_view"],
                "in_route_manifest": path in manifest_modules,
                "selectors": selectors,
                "pinned_reviewed_at": lock_pins.get((pin_kind, name, module)) if pin_kind else None,
                "resource_loader_refusal": probe["crates"][record["name"]]["refused"].get(module, "").split(": ", 1)[-1] or None,
            })
        in_view = [d for d in declarations if d["resource_view"]]
        if in_view:
            reasons = sorted({r for f in evidence[name].values() for r in f})
            klass = "C: resource-v1 in view and pinned, withdrawn in the calling unit (" + ", ".join(reasons) + ")"
        elif any("bounds_contract_v1" in d["selectors"] for d in declarations):
            klass = "A: bounds-v1 only (bounds view); no resource-v1 declaration"
        else:
            klass = "B: declared model / typed module the resource loader refuses; no resource-v1 declaration"
        owners = sorted({OWNER[d["module"]] for d in declarations})
        callers = sorted(pairs[name])
        rows.append({
            "callee": name,
            "pairs": len(callers),
            "sites": sites[name],
            "caller_libraries": sorted({c[0] for c in callers}),
            "class": klass,
            "owner": owners,
            "declarations": declarations,
            "body_evidence_in_calling_units": {f: sorted(v) for f, v in evidence[name].items()
                                               if any(c[1] == f for c in callers)},
            "callers": [[c[1], c[2]] for c in callers],
        })

    summary = collections.Counter()
    pair_summary = collections.Counter()
    for row in rows:
        summary[row["class"][0]] += 1
        pair_summary[row["class"][0]] += row["pairs"]
    document = {
        "card": "CONTRACT-ADOPTION-NOT-IN-FORCE",
        "report_sha256": hashlib.sha256(report_bytes).hexdigest(),
        "report_fluent_bit_revision": report["fluent_bit_revision"],
        "uncontracted_callees": len(pairs),
        "uncontracted_pairs": sum(len(v) for v in pairs.values()),
        "declared_and_uncontracted_callees": len(rows),
        "declared_and_uncontracted_pairs": sum(r["pairs"] for r in rows),
        "by_class_callees": dict(sorted(summary.items())),
        "by_class_pairs": dict(sorted(pair_summary.items())),
        "resource_v1_declarations_outside_resource_views": stray,
        "route_manifest_equals_union_of_resource_views": manifest_modules == view_modules,
        "rows": rows,
    }
    out = Path(args.out)
    (out / "census.json").write_text(json.dumps(document, indent=1) + "\n")
    lines = ["| # | Callee | Pairs | Class | Declaring module | Module's first resource-loader refusal, or the withdrawal | Owner |",
             "|---|---|---|---|---|---|---|"]
    for i, row in enumerate(rows, 1):
        modules = "; ".join(f"`{d['module']}`" for d in row["declarations"])
        if row["class"].startswith("C"):
            why = row["class"].split("(", 1)[1].rstrip(")")
        else:
            why = "; ".join(sorted({d["resource_loader_refusal"].split(" at byte")[1].split(": ", 1)[1]
                                    for d in row["declarations"]}))
        lines.append(f"| {i} | `{row['callee']}` | {row['pairs']} | {row['class'][0]} | {modules} | {why} | "
                     + ", ".join(f"`{o}`" for o in row["owner"]) + " |")
    (out / "census.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({k: v for k, v in document.items() if k != "rows"}, indent=1))


if __name__ == "__main__":
    main()
