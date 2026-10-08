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
raw digest. BEFORE and AFTER are run directories or report paths."""
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
    }


def main():
    before, after = report_path(sys.argv[1]), report_path(sys.argv[2])
    record = {"report": compare(before, after, project_report, True)}
    sidecar_before = before.with_suffix(".contracts.json")
    sidecar_after = after.with_suffix(".contracts.json")
    if sidecar_before.exists() and sidecar_after.exists():
        record["sidecar"] = compare(sidecar_before, sidecar_after, project_sidecar, False)
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps(record, indent=1))
    ok = all(part["projection_after_equals_before"] for part in record.values())
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
