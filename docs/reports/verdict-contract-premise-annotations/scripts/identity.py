#!/usr/bin/env python3
"""identity.py BEFORE AFTER OUT.json -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS:
the identity check of one route (a run directory holding
`resource-diagnostics.json` and its `.contracts.json` sidecar).

The card changes contract text only: eleven declarations gain one
`resource_premise` clause each (and the modules gain comments). The claim is
that nothing moves but the contract inputs and the premises the dependency
section reports. The check compares BEFORE and AFTER in full, path by path,
and passes only when

* the verdict rows are identical: the digest of every unit's (library, file,
  status, reason, reason_code) and every row's (function, status,
  diagnostics), in report order (the definition VERDICT-CONTRACT-DEPENDENCIES
  used), and also of every row's `contract_dependencies` field; and
* every differing path of the report and of the sidecar is a contract input:
  a module's text or digest (`/sources/`, `/contracts/`, `/manifest`,
  `module_sha256`), a lock's digest (`/contract_revision/locks/`), the
  current route's per-unit contract-source digests
  (`compile_configuration/.../resource_contract_sources`,
  `inventory_sha256`), or, for one of the eleven CHANGED declarations only,
  its `contract_sha256` and its `premises` (in `contract_revision`, in the
  dependency section's declaration record and top-20 list, and in the
  current route's `contract_premises` map).

It prints, for each changed declaration, its record in the AFTER dependency
section: mode, status, body standing, rows and Safe rows that rest on it,
and the premise names it now reports."""
import hashlib
import json
import pathlib
import sys

CHANGED = (
    "flb_output_task_singleplex_enqueue", "flb_input_collector_resume", "flb_input_chunk_get_name",
    "cJSON_New_Item", "add_item_to_array", "rd_kafka_toppar_new0", "rd_kafka_toppar_desired_add",
    "rd_kafka_q_new0", "ut_create_mock_queue", "__errno_location", "strerror",
)
CONTRACT_INPUT = ("/sources/", "/contract_revision/locks/", "/manifest", "/contracts/")
CONFIGURATION_INPUT = ("/resource_contract_sources", "/inventory_sha256")


def differences(x, y, path=""):
    if type(x) is not type(y):
        return [path]
    if isinstance(x, dict):
        out = []
        for key in sorted(set(x) | set(y)):
            if key not in x or key not in y:
                out.append(f"{path}/{key}")
            else:
                out.extend(differences(x[key], y[key], f"{path}/{key}"))
        return out
    if isinstance(x, list):
        if len(x) != len(y):
            return [path]
        return [d for i, (u, v) in enumerate(zip(x, y)) for d in differences(u, v, f"{path}[{i}]")]
    return [] if x == y else [path]


def indexed_function(document, path, prefix):
    """The `function` of the list element PATH names under PREFIX, or None."""
    if not path.startswith(prefix + "["):
        return None
    index = int(path[len(prefix) + 1:].split("]")[0])
    node = document
    for key in prefix.strip("/").split("/"):
        node = node[key]
    return node[index].get("function")


def allowed(path, after):
    if path.startswith(CONTRACT_INPUT) or path.endswith("/module_sha256"):
        return True
    if "/compile_configuration/" in path and any(f"{key}" in path for key in CONFIGURATION_INPUT):
        return True
    parts = path.split("/")
    for prefix in ("/contract_revision/contracts/", "/contract_dependencies/declarations/",
                   "/contract_premises/"):
        if path.startswith(prefix):
            name = path[len(prefix):].split("/")[0]
            tail = path[len(prefix) + len(name):]
            if name in CHANGED and (prefix == "/contract_premises/" or tail in ("/contract_sha256",)
                                    or tail.startswith("/premises")):
                return True
    for prefix in ("/contract_revision/pinned_in_force",
                   "/contract_dependencies/safe_rollup/top_declared_assumptions"):
        function = indexed_function(after, path, prefix)
        if function in CHANGED:
            tail = path.split("]", 1)[1]
            if tail in ("/contract_sha256",) or tail.startswith("/premises"):
                return True
    del parts
    return False


def verdict_rows(report):
    rows = [[u.get("library"), u.get("file"), u.get("status"), u.get("reason"), u.get("reason_code"),
             [[f["function"], f["status"], f.get("diagnostics")] for f in u.get("functions", [])]]
            for u in report.get("units", [])]
    return json.dumps(rows, sort_keys=True, ensure_ascii=False).encode()


def dependency_rows(report):
    rows = [[u.get("file"), [[f["function"], f.get("contract_dependencies")] for f in u.get("functions", [])]]
            for u in report.get("units", [])]
    return json.dumps(rows, sort_keys=True, ensure_ascii=False).encode()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    before, after, out = (pathlib.Path(a) for a in sys.argv[1:4])
    record, ok = {}, True
    for name, suffix in (("report", "resource-diagnostics.json"),
                         ("sidecar", "resource-diagnostics.contracts.json")):
        raw_b, raw_a = (before / suffix).read_bytes(), (after / suffix).read_bytes()
        b, a = json.loads(raw_b), json.loads(raw_a)
        diffs = differences(b, a)
        outside = [p for p in diffs if not allowed(p, a)]
        record[name] = {"before_sha256": sha(raw_b), "after_sha256": sha(raw_a),
                        "differing_paths": diffs, "outside_the_contract_input": outside}
        ok = ok and not outside
        if name == "report":
            vb, va = sha(verdict_rows(b)), sha(verdict_rows(a))
            db, da = sha(dependency_rows(b)), sha(dependency_rows(a))
            record["verdict_rows_sha256"] = {"before": vb, "after": va, "equal": vb == va}
            record["dependency_rows_sha256"] = {"before": db, "after": da, "equal": db == da}
            ok = ok and vb == va and db == da
            totals = {}
            for u in a.get("units", []):
                for f in u.get("functions", []):
                    totals[f["status"]] = totals.get(f["status"], 0) + 1
            record["totals_after"] = dict(sorted(totals.items()))
        else:
            section = a.get("contract_dependencies", {})
            declarations = section.get("declarations", {})
            record["changed_declarations"] = {
                d: {k: declarations[d].get(k) for k in ("mode", "status", "body_standing", "rows", "safe_rows")}
                | {"premises": [p["name"] for p in declarations[d].get("premises", [])]}
                if d in declarations else None
                for d in CHANGED}
            premise_safe = {}
            for d, entry in record["changed_declarations"].items():
                for p in (entry or {}).get("premises", []):
                    if p == "literal_argument":
                        continue
                    premise_safe.setdefault(p, {"declarations": 0, "rows": 0, "safe_rows": 0})
                    premise_safe[p]["declarations"] += 1
                    premise_safe[p]["rows"] += entry["rows"]
                    premise_safe[p]["safe_rows"] += entry["safe_rows"]
            record["by_premise"] = premise_safe
            record["safe_rollup"] = {k: v for k, v in section.get("safe_rollup", {}).items()
                                     if k != "top_declared_assumptions"}
    record["ok"] = ok
    out.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print("verdict rows equal", record["verdict_rows_sha256"]["equal"], record["verdict_rows_sha256"]["after"][:12],
          "dependency rows equal", record["dependency_rows_sha256"]["equal"])
    for name in ("report", "sidecar"):
        part = record[name]
        print(name, part["before_sha256"][:12], "->", part["after_sha256"][:12],
              "differing", len(part["differing_paths"]), "outside", len(part["outside_the_contract_input"]))
        for path in part["outside_the_contract_input"][:20]:
            print("  OUTSIDE", path)
    print("totals", record["totals_after"])
    print("by premise", json.dumps(record["by_premise"]))
    print("changed", json.dumps(record["changed_declarations"]))
    sys.exit(0 if ok else 1)


main()
