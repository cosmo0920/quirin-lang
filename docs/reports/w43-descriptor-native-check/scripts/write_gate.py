#!/usr/bin/env python3
"""Collect this card's gate step facts into the report's gate records.

Usage: write_gate.py SOURCE_COMMIT GATE_DIR [GATE_DIR ...]

Reads every facts-<step>.json in the given run_gate.py output directories (a
later directory's step replaces an earlier one's, so a documented rerun wins),
writes gate-facts.json (verbatim facts), clang-version.txt / .exit (from the
`clang` step of this checkout) and gate.json in the matrix generator's
`--gate` shape beside this report. Exit 0 only when every recorded step
exited 0.
"""

import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(__file__).resolve().parents[4]


def main() -> int:
    commit = sys.argv[1]
    facts = {}
    sources = {}
    for directory in map(pathlib.Path, sys.argv[2:]):
        for path in sorted(directory.glob("facts-*.json")):
            step = path.stem.removeprefix("facts-")
            facts[step] = json.loads(path.read_text())
            sources[step] = str(directory.resolve())
    clang_dir = pathlib.Path(sources["clang"])
    (REPORT / "clang-version.txt").write_text((clang_dir / "clang.log").read_text())
    (REPORT / "clang-version.exit").write_text((clang_dir / "clang.exit").read_text())
    (REPORT / "gate-facts.json").write_text(json.dumps(
        {"source_commit": commit, "private_logs": sources, "steps": facts}, indent=2) + "\n")
    exits = {step: record["exit"] for step, record in facts.items()}
    tests = facts["test"]["totals"]
    provider = facts["provider_test"]["totals"]
    clang = (REPORT / "clang-version.txt").read_text().splitlines()[0]
    complete = all(code == 0 for code in exits.values())
    record = {
        "note": (
            f"W43-DESCRIPTOR-NATIVE-CHECK: gate measured at {commit}. {tests['passed']} core Rust passed / "
            f"{tests['failed']} failed / {tests['ignored']} ignored over {tests['summaries']} summaries; "
            f"{facts['lean'].get('audits_passed')} Lean audits; {facts['python'].get('tests_run')} root Python "
            f"tests after the final matrix regeneration; docs index check exit {exits.get('docs_index')}. "
            f"Provider workspace {provider['passed']} passed / {provider['failed']} failed / "
            f"{provider['ignored']} ignored over {provider['summaries']} summaries; isolated package mutation "
            f"control exit {exits.get('provider_mutation')}; providers/contracts.py check exit "
            f"{exits.get('contracts_check')}; five package builds/verifications; "
            f"{facts['provider_python'].get('tests_run')} package Python tests. Step exits: {exits}. "
            f"Logs: {sorted(set(sources.values()))}. Clang recorded in this checkout. CARGO_INCREMENTAL=0. "
            "Native observation evidence only; no verdict, unit or function moved at the arbiter."
        ),
        "workspace_tests": tests["passed"],
        "provider_tests": provider["passed"],
        "python_evidence_validator_tests": facts["python"].get("tests_run"),
        "lean_audits": facts["lean"].get("audits_passed"),
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
