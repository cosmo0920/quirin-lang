#!/usr/bin/env python3
"""verdict_projection.py BEFORE AFTER OUT.json -- DECLARED-ASSUMPTION-BODY-CHECKS
(copied unchanged from VERDICT-BODY-STANDING-CLAUSE-SCOPE, adapted from
VERDICT-CONTRACT-DEPENDENCIES; for this card the expected unit-field moves are
the `conflict` failures rule 13m replaces by `13m` evidence): the projection of a route report
and its contract sidecar without what this card changes.

The card changes two things: each unit row's `contract_body_evidence` (the
per-clause coverage it adds, and the W4 evidence it now records for a
declaration with container or counted-user clauses) and the sidecar's
`contract_dependencies` section (the standing labels and the roll-up). The
projection of a report removes exactly the unit key, **keeping every row's
`contract_dependencies` field**; of a sidecar, exactly its
`contract_dependencies` section. It is written back in the report's own form
(serde_json's pretty printer: two-space indent, `": "`, no ASCII escaping, the
report with a trailing newline), which Python reproduces byte for byte (checked
on BEFORE itself). The claim is that the projection of AFTER equals the
projection of BEFORE byte for byte on every route (no contract input changed,
so nothing else may differ), and that the **verdict rows** are identical: the
digest of every unit's (library, file, status, reason, reason_code) and every
row's (function, status, diagnostics), in report order, printed for both runs.

The unit field itself is compared too (`unit_field`): AFTER's field without its
`clauses` key against BEFORE's field, listing every declaration that moved
between `validated`, `unvalidated` and `failed` (expected: only declarations
newly validated by W4 because their container or counted-user clauses no
longer stop the W4 record)."""
import hashlib
import json
import pathlib
import sys

UNIT_KEY = "contract_body_evidence"
SIDECAR_KEYS = ("contract_dependencies",)


def report_path(path):
    path = pathlib.Path(path)
    return path / "resource-diagnostics.json" if path.is_dir() else path


def project_report(data):
    for unit in data.get("units", []):
        unit.pop(UNIT_KEY, None)
    return data


def project_sidecar(data):
    for key in SIDECAR_KEYS:
        data.pop(key, None)
    return data


def dump(data, newline):
    text = json.dumps(data, indent=2, ensure_ascii=False).encode()
    return text + (b"\n" if newline else b"")


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
        "projector_reproduces_before": dump(json.loads(raw_before), newline) == raw_before,
        "projection_before_sha256": sha(projected_before),
        "projection_after_sha256": sha(projected_after),
        "projections_equal": projected_after == projected_before,
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


def unit_field(before, after):
    """Per declaration, its unit outcomes before and after where they differ."""
    def outcomes(path):
        data = json.loads(path.read_bytes())
        out = {}
        for index, unit in enumerate(data.get("units", [])):
            field = unit.get(UNIT_KEY, {})
            key = f"{unit.get('library')}/{unit.get('file')}#{index}"
            for name, by in field.get("validated", {}).items():
                out[(key, name)] = ("validated", tuple(by))
            for name in field.get("unvalidated", []):
                out[(key, name)] = ("unvalidated", ())
            for name, why in field.get("failed", {}).items():
                out[(key, name)] = ("failed", tuple(why))
        return out
    b, a = outcomes(before), outcomes(after)
    moved = {}
    for key in sorted(set(b) | set(a)):
        if b.get(key) != a.get(key):
            name = key[1]
            moved.setdefault(name, []).append([key[0], b.get(key), a.get(key)])
    return moved


def main():
    before, after = report_path(sys.argv[1]), report_path(sys.argv[2])
    record = {"report": compare(before, after, project_report, True)}
    sidecar_before = before.with_suffix(".contracts.json")
    sidecar_after = after.with_suffix(".contracts.json")
    if sidecar_before.exists() and sidecar_after.exists():
        record["sidecar"] = compare(sidecar_before, sidecar_after, project_sidecar, False)
    verdicts = [hashlib.sha256(verdict_rows(path)).hexdigest() for path in (before, after)]
    record["verdict_rows_sha256"] = {"before": verdicts[0], "after": verdicts[1],
                                     "equal": verdicts[0] == verdicts[1]}
    moved = unit_field(before, after)
    record["unit_field_moves"] = moved
    ok = verdicts[0] == verdicts[1] and record["report"]["projections_equal"]
    if "sidecar" in record:
        ok = ok and record["sidecar"]["projections_equal"]
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    print("verdict rows", verdicts[0], verdicts[1], "equal", verdicts[0] == verdicts[1])
    for name in ("report", "sidecar"):
        if name not in record:
            continue
        part = record[name]
        print(name, part["before_sha256"], "->", part["after_sha256"], "projection",
              part["projection_before_sha256"], part["projection_after_sha256"],
              "equal", part["projections_equal"], "projector", part["projector_reproduces_before"],
              "added", part["added_bytes"], "of", part["before_bytes"],
              "differing", len(part["differing_paths_after_projection"]))
    print("unit-field moves", len(moved), sorted(moved))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
