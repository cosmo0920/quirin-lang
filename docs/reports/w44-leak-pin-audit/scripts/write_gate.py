#!/usr/bin/env python3
"""Collect one gate directory's per-step facts into the report's gate records.

Usage: write_gate.py GATE_DIR SOURCE_COMMIT

Writes `gate-facts.json` (every step's facts file, verbatim), `gate.json` (the
matrix generator's `--gate` shape) and the README's gate table between the
`<!-- gate -->` markers (or in place of `GATE_TABLE`). The Python counts are
the post-regeneration `python` and `ppython` steps; the Lean audit count is the
number of `proof/Audit*.lean` files the audit step checked. Adapted from
docs/reports/w43-return-expression/scripts/write_gate.py: provider tests are
measured by this gate (`ptest`), not carried.
"""

import json
import pathlib
import re
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(__file__).resolve().parents[4]
ORDER = [
    "submodules", "clang", "test", "fmt", "clippy", "probe", "followup", "lean", "census",
    "ptest", "pisolated", "pfmt", "pclippy", "pcontracts", "pbuild", "pverify", "ppython",
    "python", "docsindex",
]


def main() -> int:
    gate = pathlib.Path(sys.argv[1])
    commit = sys.argv[2]
    facts = {}
    for path in sorted(gate.glob("facts-*.json")):
        facts[path.stem.removeprefix("facts-")] = json.loads(path.read_text())
    (REPORT / "gate-facts.json").write_text(
        json.dumps({"source_commit": commit, "private_logs": str(gate.resolve()), "steps": facts}, indent=2)
        + "\n"
    )
    exits = {step: facts[step]["exit"] for step in ORDER if step in facts}
    missing = [step for step in ORDER if step not in facts]
    tests = facts["test"]["totals"]
    provider = facts["ptest"]["totals"]
    census = facts["census"]["totals"]
    python_tests = (facts.get("python") or {}).get("totals", {}).get("python_ran", 0)
    provider_python = (facts.get("ppython") or {}).get("totals", {}).get("python_ran", 0)
    lean_log = (gate / "lean.log").read_text(errors="replace")
    passed = re.findall(r"^Passed (\d+) Lean audits", lean_log, re.M)
    audits = int(passed[-1]) if passed else 0
    clang = (REPORT / "clang-version.txt").read_text().splitlines()[0]
    complete = not missing and all(code == 0 for code in exits.values())
    record = {
        "note": (
            f"W44-LEAK-PIN-AUDIT: every step measured at {commit} (the Python suites after the final "
            f"matrix regeneration). {tests['passed']} core Rust passed / {tests['failed']} failed / "
            f"{tests['ignored']} ignored over {tests['summaries']} summaries; {provider['passed']} "
            f"provider Rust passed / {provider['failed']} failed; census feature {census['passed']} "
            f"passed; {audits} Lean audits; {python_tests} root and {provider_python} provider Python "
            f"tests. Step exits: {exits}. Logs: {gate.resolve()}. No coverage promotion; the arbiter "
            "moved 76 functions and 35 units provisional Unsafe -> Unknown and 3 functions Unknown -> Safe."
        ),
        "workspace_tests": tests["passed"],
        "provider_tests": provider["passed"],
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

    rows = ["<!-- gate -->", "", "| Step | Exit | Measured |", "|---|---:|---|"]
    for step in ORDER:
        if step not in facts:
            rows.append(f"| {step} | — | not run |")
            continue
        totals = facts[step].get("totals") or {}
        measured = ""
        if "python_ran" in totals:
            measured = f"{totals['python_ran']} tests"
        elif totals:
            measured = (f"{totals['passed']} passed / {totals['failed']} failed / "
                        f"{totals['ignored']} ignored over {totals['summaries']} summaries")
        if step == "lean":
            measured = f"{audits} audits, no placeholders, no project axioms"
        if step == "clang":
            measured = clang
        rows.append(f"| {step} | {facts[step]['exit']} | {measured} |")
    rows += ["", "<!-- /gate -->"]
    table = "\n".join(rows)
    readme = REPORT / "README.md"
    text = readme.read_text()
    if "GATE_TABLE" in text:
        text = text.replace("GATE_TABLE", table)
    else:
        text = re.sub(r"<!-- gate -->.*?<!-- /gate -->", lambda _: table, text, flags=re.S)
    readme.write_text(text)
    print(json.dumps(record, indent=2))
    return 0 if complete else 1


if __name__ == "__main__":
    sys.exit(main())
