#!/usr/bin/env python3
"""project.py NAME OUT.json -- OUTER-CONTRACT-DESIGN: the predicted runtime
projection of each alternative over one measurement's rows (measure.sh NAME).

A row's closed-world facts (README section 2.1) are read from the rows:

* `fbp`: a null row naming `stable:flb_{input,output,filter,processor}_instance.p`;
* `rk`: a null row listing the premise `rk_no_plugin_library`, or a custody
  row resting on a declaration that carries it;
* `cl`: a custody row resting on a run-set declaration (rule 12n / 13u);
* `k3:NAME`: a custody row resting on a declaration carrying one of the
  closed-world premises `assignor_callback`, `broker_monitor_callback`,
  `conf_property_writer`, `cjson_allocator_hooks`.

Scenarios (each maps a fact to `Pending`, `Unknown` or nothing):

* `static`: nothing loaded; no fact contributes (every alternative).
* `A`: both contracts loaded, facts cited as in Alternative A / C (the four
  `p` declarations cite `flb_external_plugin`; the 163 `rk_no_plugin_library`
  clauses cite `rk_plugin_library`): `fbp`, `rk` -> Pending; the rest is not
  cited, so nothing (A's omission risk: `cl` and `k3:*` rows without `rk`
  are not detected).
* `B-fb`: only Fluent Bit plugins loaded, Alternative B with the drafted
  envelope (the four `p` lines): `fbp` -> Pending; `rk`, `cl`, `k3:*` ->
  Unknown (reachable from `config` through `void *` contexts, not covered).
* `B-rk`: only an rk plugin library loaded, Alternative B with the drafted
  envelope (rk_conf members, interceptor lists, `rkb_rk`, `rko_op_cb` kept,
  `#premise` lines for assignor_callback, broker_monitor_callback,
  conf_property_writer): `rk`, `cl`, those `k3` -> Pending; `fbp` -> Unknown
  (reachable through the conf's opaque); `cjson_allocator_hooks` not
  reachable through the public API -> nothing.
* `B-both`: both loaded, each envelope as drafted: the join of B-fb and B-rk.
* `B-both-cross`: both loaded, each envelope also vouching for the other's
  facts (the Fluent Bit envelope states librdkafka's lines and vice versa):
  `fbp`, `rk`, `cl`, those `k3` -> Pending; `cjson_allocator_hooks` ->
  Unknown (a Fluent Bit plugin reaches every exported symbol's global only if
  its statement does not exclude non-public symbols; it is kept Unknown here,
  the cautious reading).

The runtime verdict is the join `Safe < Pending < Unknown < Unsafe` of the
static verdict and the contributions. Prints and writes, per route and
scenario, the null, custody and combined tallies and the Safe rows that move.
"""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/ocd"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
FBP = {f"stable:flb_{k}_instance.p" for k in ("input", "output", "filter", "processor")}
K3_RK = {"assignor_callback", "broker_monitor_callback", "conf_property_writer"}
K3_ALL = K3_RK | {"cjson_allocator_hooks"}
CLOSURE = re.compile(r"resource_callback_(member_invoked|registry_invoked|invoked|member_registered)\(")


def join(a, b):
    return a if ORDER[a] >= ORDER[b] else b


def closure_names():
    names = set()
    for path in (REPO / "providers").rglob("*.qv"):
        for m in re.finditer(r"extern fn (\w+)\(.*?\ncontract \{(.*?)\n\};", path.read_text(), re.S):
            if CLOSURE.search(m.group(2)):
                names.add(m.group(1))
    return names


def contributions(facts, scenario):
    out = []
    if scenario == "static":
        return out
    if scenario == "A":
        if "fbp" in facts or "rk" in facts:
            out.append("Pending")
        return out
    fb = scenario in ("B-fb", "B-both")
    rk = scenario in ("B-rk", "B-both")
    if scenario == "B-both-cross":
        for f in facts:
            out.append("Unknown" if f == "k3:cjson_allocator_hooks" else "Pending")
        return out
    for f in facts:
        if fb:
            out.append("Pending" if f == "fbp" else "Unknown")
        if rk:
            if f in ("rk", "cl") or (f.startswith("k3:") and f[3:] in K3_RK):
                out.append("Pending")
            elif f == "fbp":
                out.append("Unknown")
    return out


def project(verdict, facts, scenario):
    for c in contributions(facts, scenario):
        verdict = join(verdict, c)
    return verdict


SCENARIOS = ("static", "A", "B-fb", "B-rk", "B-both", "B-both-cross")


def paths(name, route):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    null = PRIVATE / f"{name}{suffix}-null.jsonl"
    if route == "slice":
        report = SLICE / f"ocd-{name}" / "resource-diagnostics.json"
    else:
        report = PRIVATE / (name if route == "corpus" else f"{name}-system") / "resource-diagnostics.json"
    return null, report


def custody_facts(report_path, closures):
    report = json.loads(report_path.read_text())
    sidecar = json.loads(report_path.with_suffix(".contracts.json").read_text()).get("contract_dependencies", {})
    premised = collections.defaultdict(set)
    for decl, record in sidecar.get("declarations", {}).items():
        for p in record.get("premises") or []:
            premised[p.get("name") if isinstance(p, dict) else str(p)].add(decl)
    rows = {}
    seen = collections.Counter()
    for unit in report["units"]:
        for entry in unit["functions"]:
            names = set((entry.get("contract_dependencies") or {}).get("declarations", []))
            facts = set()
            if names & premised.get("rk_no_plugin_library", set()):
                facts.add("rk")
            if names & closures:
                facts.add("cl")
            for k in K3_ALL:
                if names & premised.get(k, set()):
                    facts.add(f"k3:{k}")
            key = (unit["file"], entry["function"])
            seen[key] += 1
            rows[key + (seen[key],)] = (entry["status"], facts)
    return rows


def route_projection(name, route, closures):
    null_path, report_path = paths(name, route)
    if not null_path.exists() or not report_path.exists():
        return None
    custody = custody_facts(report_path, closures)
    by_function = collections.defaultdict(list)
    by_unit_function = collections.defaultdict(list)
    for (unit, function, _), value in custody.items():
        by_function[function].append(value)
        by_unit_function[(unit, function)].append(value)
    result = {}
    for scenario in SCENARIOS:
        null_t, comb_t, cust_t = collections.Counter(), collections.Counter(), collections.Counter()
        moving = []
        for (unit, function, occ), (status, facts) in custody.items():
            after = project(status, facts, scenario)
            cust_t[after] += 1
            if after != status and status == "Safe":
                moving.append({"part": "custody", "unit": unit.split("/")[-1], "function": function,
                               "to": after, "facts": sorted(facts)})
        for line in null_path.read_text().splitlines():
            if not line.strip():
                continue
            row = json.loads(line)
            null = row["null"]
            nfacts = set()
            deps = set(null.get("contract_dependencies") or [])
            if deps & FBP:
                nfacts.add("fbp")
            if any((p.get("name") if isinstance(p, dict) else p) == "rk_no_plugin_library"
                   for p in null.get("premises") or []):
                nfacts.add("rk")
            nv = project(null["verdict"], nfacts, scenario)
            null_t[nv] += 1
            # Combined: the custody rows of the same unit and function (every
            # configuration's facts, a union), joined with the projected null
            # verdict.
            cust = row.get("custody")
            cfacts = set()
            same = by_unit_function.get((row["file"], row["function"])) or by_function.get(row["function"], [])
            for status, facts in same:
                cfacts |= facts
            cv = project(cust, cfacts, scenario) if cust in ORDER else cust
            comb = join(cv, nv) if cv in ORDER else row.get("combined")
            comb_t[comb] += 1
            if row.get("combined") == "Safe" and comb != "Safe":
                moving.append({"part": "combined", "file": row["file"].split("/")[-1],
                               "function": row["function"], "to": comb,
                               "facts": sorted(nfacts | cfacts)})
            if null["verdict"] == "Safe" and nv != "Safe":
                moving.append({"part": "null", "file": row["file"].split("/")[-1],
                               "function": row["function"], "to": nv, "facts": sorted(nfacts)})
        result[scenario] = {
            "custody": dict(sorted(cust_t.items(), key=lambda kv: ORDER.get(kv[0], 9))),
            "null": dict(sorted(null_t.items(), key=lambda kv: ORDER.get(kv[0], 9))),
            "combined": dict(sorted(comb_t.items(), key=lambda kv: ORDER.get(str(kv[0]), 9))),
            "safe_moves": dict(collections.Counter(f"{m['part']} -> {m['to']}" for m in moving)),
            "moving": moving,
        }
    return result


def main() -> int:
    name, out = sys.argv[1], pathlib.Path(sys.argv[2])
    closures = closure_names()
    result = {route: route_projection(name, route, closures) for route in ("corpus", "system", "slice")}
    out.write_text(json.dumps(result, indent=1, ensure_ascii=False) + "\n")
    for route, scen in result.items():
        if scen is None:
            print(route, "missing")
            continue
        for s, v in scen.items():
            print(route, s, "custody", v["custody"], "null", v["null"], "combined", v["combined"], "moves", v["safe_moves"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
