#!/usr/bin/env python3
"""Create gate.json only after every recorded standing and provider step passed."""

import json
import re
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]
REPORT = REPO / "docs/reports/rk-plugin-interceptor-candidates"
PRIVATE = REPO / "private/rk-interceptor-candidates"


def exits(directory):
    return {path.stem: int(path.read_text().strip()) for path in directory.glob("*.exit")}


def match(pattern, text):
    result = re.search(pattern, text)
    if not result:
        raise ValueError(f"missing gate count: {pattern}")
    return result


def main():
    standing = PRIVATE / "gate-standing-final"
    providers = PRIVATE / "gate-providers-final"
    docs = PRIVATE / "gate-docs-final"
    by_part = {"standing": exits(standing), "providers": exits(providers), "docs": exits(docs)}
    required = {
        "standing": {"submodule", "tests", "fmt", "clippy", "probe", "followup", "lean", "clang-version"},
        "providers": {"check", "tests", "fmt", "clippy", "contracts", "package-build", "python"},
        "docs": {"python", "index"},
    }
    for part, needed in required.items():
        if not needed <= by_part[part].keys():
            raise ValueError(f"missing {part} exits: {sorted(needed - by_part[part].keys())}")
        bad = {name: by_part[part][name] for name in needed if by_part[part][name]}
        if bad:
            raise ValueError(f"nonzero {part} exits: {bad}")
    verification = {name: code for name, code in by_part["providers"].items() if name.startswith("verify-")}
    if len(verification) != 5 or any(verification.values()):
        raise ValueError("all five packages must verify")
    tests = (standing / "tests.log").read_text()
    totals = match(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored",
        tests,
    )
    lean = int(match(r"Passed (\d+) Lean audits", (standing / "lean.log").read_text()).group(1))
    provider_tests = sum(
        int(n) for n in re.findall(r"test result: ok\. (\d+) passed", (providers / "tests.log").read_text())
    )
    script_tests = int(match(r"Ran (\d+) tests", (docs / "python.log").read_text()).group(1))
    provider_python = int(match(r"Ran (\d+) tests", (providers / "python.log").read_text()).group(1))
    clang = (standing / "clang-version.txt").read_text().splitlines()[0]
    observed_source = (standing / "head-standing-tests").read_text().strip()
    note = (
        f"RK-PLUGIN-INTERCEPTOR-CANDIDATES on base 1543259f0; final source {observed_source}; worktree-private logs in "
        "private/rk-interceptor-candidates/gate-standing-final, gate-providers-final, gate-docs-final; "
        f"standing tests {totals.group(1)} units/{totals.group(2)} summaries/"
        f"{totals.group(3)} passed/{totals.group(4)} failed/{totals.group(5)} ignored; "
        f"Lean {lean} audits; provider Rust {provider_tests} passed, provider Python {provider_python} "
        f"and scripts Python {script_tests} tests; every required final gate step exited 0; corrected diagnostic attempts remain in the report. "
        "Source pins and per-route arbiter exits are separately recorded in the card report."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": provider_tests,
        "python_evidence_validator_tests": script_tests,
        "reevaluation_probe_exit": by_part["standing"]["probe"],
        "reevaluation_followup_exit": by_part["standing"]["followup"],
        "cargo_fmt": "clean",
        "cargo_clippy": "clean",
        "standing_gate": "complete",
        "lean_audits": lean,
        "clang": f"present: {clang}",
    }
    (REPORT / "gate.json").write_text(json.dumps(gate, indent=2) + "\n")
    print(json.dumps(gate, indent=2))


if __name__ == "__main__":
    main()
