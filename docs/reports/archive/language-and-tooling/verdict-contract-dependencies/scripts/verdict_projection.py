#!/usr/bin/env python3
"""verdict_projection.py BEFORE AFTER OUT.json -- VERDICT-CONTRACT-DEPENDENCIES:
the verdict-only projection of a route report and its contract sidecar.

The projection of a report removes exactly what the card adds: each function
row's `contract_dependencies` key and each unit row's `contract_body_evidence`
key; of a sidecar (`*.contracts.json`), its `contract_dependencies` and
`contract_premises` keys. It is written back in the report's own form
(serde_json's pretty printer: two-space indent, `": "`, no ASCII escaping,
the report with a trailing newline), which Python reproduces byte for byte
(checked on BEFORE itself: the projection of a report without the keys is
that report's bytes). The card's claim is that the projection of AFTER is
byte-identical to BEFORE, so the SHA-256 of the projection equals BEFORE's
raw digest. BEFORE and AFTER are run directories or report paths.

The sidecar also carries the contract inputs, and the card changed one: the
served-queue premise is new text in `librdkafka/client.qv`, so that module's
text and digest, the two declarations' contract digests and the crate lock's
digest differ. On the current route (system, slice) each unit's
`compile_configuration` names the contract sources' digests, so those change
in the report too. Every differing path of both projections is listed, and the
run passes only when every difference is one of those contract-input paths
and the **verdict rows** are identical: the digest of every unit's
(library, file, status, reason, reason_code) and every row's (function,
status, diagnostics), in report order, is printed for both runs."""
import hashlib
import json
import pathlib
import sys

ROW_KEY = "contract_dependencies"
UNIT_KEY = "contract_body_evidence"
SIDECAR_KEYS = ("contract_dependencies", "contract_premises")


def report_path(path):
    path = pathlib.Path(path)
    return path / "resource-diagnostics.json" if path.is_dir() else path


def project_report(data):
    for unit in data.get("units", []):
        unit.pop(UNIT_KEY, None)
        for function in unit.get("functions", []):
            function.pop(ROW_KEY, None)
    return data


def project_sidecar(data):
    for key in SIDECAR_KEYS:
        data.pop(key, None)
    return data


def dump(data, newline):
    text = json.dumps(data, indent=2, ensure_ascii=False).encode()
    return text + (b"\n" if newline else b"")


CONTRACT_INPUT = ("/sources/", "/contract_revision/locks/", "/manifest", "/contracts/")
CONFIGURATION_INPUT = ("/resource_contract_sources", "/inventory_sha256")
CHANGED_DECLARATIONS = ("rd_kafka_q_purge0", "rd_kafka_q_wait_result")


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


def contract_input(path, after):
    """Whether a differing sidecar path is the changed contract input."""
    if path.startswith(CONTRACT_INPUT) or path.endswith("/module_sha256"):
        return True
    if "/compile_configuration/" in path and path.endswith(CONFIGURATION_INPUT):
        return True
    if path.endswith("/contract_sha256"):
        if path.startswith("/contract_revision/contracts/"):
            return path.split("/")[3] in CHANGED_DECLARATIONS
        if path.startswith("/contract_revision/pinned_in_force["):
            index = int(path.split("[")[1].split("]")[0])
            return after["contract_revision"]["pinned_in_force"][index]["function"] in CHANGED_DECLARATIONS
    return False


def compare(before, after, project, newline):
    raw_before = before.read_bytes()
    raw_after = after.read_bytes()
    projected_before = dump(project(json.loads(raw_before)), newline)
    projected_after = dump(project(json.loads(raw_after)), newline)
    sha = lambda data: hashlib.sha256(data).hexdigest()
    return {
        "before": str(before), "after": str(after),
        "before_sha256": sha(raw_before), "after_sha256": sha(raw_after),
        "before_bytes": len(raw_before), "after_bytes": len(raw_after),
        "projector_reproduces_before": projected_before == raw_before,
        "projection_after_sha256": sha(projected_after),
        "projection_after_equals_before": projected_after == raw_before,
        "added_bytes": len(raw_after) - len(raw_before),
        "differing_paths_after_projection": differences(
            json.loads(projected_before), json.loads(projected_after)),
    }


def verdict_rows(path):
    """The verdict rows of a report, as canonical JSON bytes."""
    data = json.loads(path.read_bytes())
    rows = [[unit.get("library"), unit.get("file"), unit.get("status"), unit.get("reason"),
             unit.get("reason_code"),
             [[f["function"], f["status"], f.get("diagnostics")] for f in unit.get("functions", [])]]
            for unit in data.get("units", [])]
    return json.dumps(rows, sort_keys=True, ensure_ascii=False).encode()


def main():
    before, after = report_path(sys.argv[1]), report_path(sys.argv[2])
    record = {"report": compare(before, after, project_report, True)}
    sidecar_before = before.with_suffix(".contracts.json")
    sidecar_after = after.with_suffix(".contracts.json")
    if sidecar_before.exists() and sidecar_after.exists():
        record["sidecar"] = compare(sidecar_before, sidecar_after, project_sidecar, False)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    verdicts = [hashlib.sha256(verdict_rows(path)).hexdigest() for path in (before, after)]
    record["verdict_rows_sha256"] = {"before": verdicts[0], "after": verdicts[1],
                                     "equal": verdicts[0] == verdicts[1]}
    report_other = [path for path in record["report"]["differing_paths_after_projection"]
                    if not contract_input(path, {})]
    record["report"]["differences_outside_the_contract_input"] = report_other
    ok = verdicts[0] == verdicts[1] and not report_other
    if "sidecar" in record:
        after_sidecar = json.loads(sidecar_after.read_text())
        other = [path for path in record["sidecar"]["differing_paths_after_projection"]
                 if not contract_input(path, after_sidecar)]
        record["sidecar"]["differences_outside_the_contract_input"] = other
        ok = ok and not other
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    print("verdict rows", verdicts[0], verdicts[1], "equal", verdicts[0] == verdicts[1])
    for name in ("report", "sidecar"):
        if name not in record:
            continue
        part = record[name]
        print(name, part["before_sha256"], "->", part["after_sha256"], "projection",
              part["projection_after_sha256"], "equal", part["projection_after_equals_before"],
              "projector", part["projector_reproduces_before"], "added", part["added_bytes"],
              "of", part["before_bytes"], "differing", len(part["differing_paths_after_projection"]),
              "outside-input", len(part.get("differences_outside_the_contract_input", [])))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
