#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: sync the two DERIVED Lean rows only.

`validation.lean_audits` and `validation.lean_audit_files` are derived by
`docs/reports/remaining-d02/build_matrix.py` from `proof/Audit*.lean` on disk.
This card adds one audit file, so both rows move by one.

A whole-file regeneration is deliberately NOT used: without `--gate` it would
rewrite `validation.measurement_provenance` from the integrator-supplied
wave-35 gate to a carried placeholder, which is exactly the clobbering
constraint 25 forbids. The rows are taken from the generator's own output so
they are derived rather than hand-counted.
"""
import json
import pathlib

WT = pathlib.Path(__file__).resolve().parents[2]
COMMITTED = WT / "docs/reviews/r0_3/acceptance_matrix.json"
GENERATED = WT / "measure/matrix-check.json"


def main():
    matrix = json.loads(COMMITTED.read_text())
    generated = json.loads(GENERATED.read_text())
    for key in ("lean_audits", "lean_audit_files"):
        before = matrix["validation"][key]
        matrix["validation"][key] = generated["validation"][key]
        if key == "lean_audits":
            print(f"validation.{key}: {before} -> {matrix['validation'][key]}")
    assert "AuditScalarRouteExitEdge.lean" in matrix["validation"]["lean_audit_files"]
    # Everything else must already agree with the generator.
    stale = [
        key
        for key in set(matrix) | set(generated)
        if key != "validation" and matrix.get(key) != generated.get(key)
    ]
    assert not stale, f"unexpected derived drift: {stale}"
    drift = [
        key
        for key in set(matrix["validation"]) | set(generated["validation"])
        if key != "measurement_provenance"
        and matrix["validation"].get(key) != generated["validation"].get(key)
    ]
    assert not drift, f"validation drift beyond the Lean rows: {drift}"
    COMMITTED.write_text(json.dumps(matrix, indent=2, sort_keys=True) + "\n")
    print("measurement_provenance left as the integrator supplied it")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
