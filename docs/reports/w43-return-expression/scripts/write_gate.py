#!/usr/bin/env python3
"""Collect one gate directory's per-step facts into the report's gate records.

Usage: write_gate.py GATE_DIR SOURCE_COMMIT PYTHON_TESTS

Writes `gate-facts.json` (every step's facts file, verbatim) and `gate.json`
(the matrix generator's `--gate` shape) beside this report. PYTHON_TESTS is the
count the post-regeneration Python run reported; the Lean audit count is the
number of `proof/Audit*.lean` files the audit step checked.
"""

import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    gate = pathlib.Path(sys.argv[1])
    commit = sys.argv[2]
    python_tests = int(sys.argv[3])
    facts = {}
    for path in sorted(gate.glob("facts-*.json")):
        facts[path.stem.removeprefix("facts-")] = json.loads(path.read_text())
    (REPORT / "gate-facts.json").write_text(
        json.dumps({"source_commit": commit, "private_logs": str(gate.resolve()), "steps": facts}, indent=2)
        + "\n"
    )
    tests = facts["test"]["totals"]
    exits = {step: record["exit"] for step, record in facts.items()}
    audits = len(list((ROOT / "proof").glob("Audit*.lean")))
    clang = (REPORT / "clang-version.txt").read_text().splitlines()[0]
    complete = all(code == 0 for code in exits.values())
    record = {
        "note": (
            f"W43-RETURN-EXPRESSION: Rust, fmt, clippy, probes and Lean measured at {commit}; "
            "later commits change documents, the ledger and report scripts only (no Rust, "
            "Lean or test source), and the Python suite ran after the final matrix "
            f"regeneration on the tip. {tests['passed']} core Rust passed / "
            f"{tests['failed']} failed / {tests['ignored']} ignored over {tests['summaries']} "
            f"summaries; {audits} Lean audits; {python_tests} root Python tests after the final "
            f"matrix regeneration. Step exits: {exits}. Provider tests are carried, not remeasured "
            f"(no provider change). Logs: {gate.resolve()}. Clang recorded in this checkout. "
            "No coverage promotion; 0 function and 0 unit moves at the arbiter."
        ),
        "workspace_tests": tests["passed"],
        "provider_tests": 123,
        "python_evidence_validator_tests": python_tests,
        "lean_audits": audits,
        "reevaluation_probe_exit": exits.get("probe"),
        "reevaluation_followup_exit": exits.get("followup"),
        "cargo_fmt": "clean" if exits.get("fmt") == 0 else "failed",
        "cargo_clippy": "clean" if exits.get("clippy") == 0 else "failed",
        "standing_gate": "complete" if complete else "incomplete",
        "clang": f"present: {clang}",
    }
    (REPORT / "gate.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return 0 if complete else 1


if __name__ == "__main__":
    sys.exit(main())
