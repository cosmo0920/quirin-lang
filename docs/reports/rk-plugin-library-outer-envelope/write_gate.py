#!/usr/bin/env python3
"""Create gate.json only after every recorded standing and provider step passed."""

import json
import re
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]
REPORT = REPO / "docs/reports/rk-plugin-library-outer-envelope"
PRIVATE = REPO / "private/rk-outer"


def exits(directory):
    return {path.stem: int(path.read_text().strip()) for path in directory.glob("*.exit")}


def match(pattern, text):
    result = re.search(pattern, text)
    if not result:
        raise ValueError(f"missing gate count: {pattern}")
    return result


def main():
    standing = PRIVATE / "gate-standing-v3"
    providers = PRIVATE / "gate-providers-v3"
    docs = PRIVATE / "gate-docs"
    by_part = {"standing": exits(standing), "providers": exits(providers), "docs": exits(docs)}
    required = {
        "standing": {"submodule", "tests", "fmt", "clippy", "probe", "followup", "lean", "clang-version"},
        "providers": {"check", "tests", "fmt", "clippy", "contracts", "package-build", "python"},
        "docs": {"python", "index"},
    }
    for part, needed in required.items():
        if not needed <= by_part[part].keys():
            raise ValueError(f"missing {part} exits: {sorted(needed - by_part[part].keys())}")
        bad = {name: code for name, code in by_part[part].items() if code}
        if bad:
            raise ValueError(f"nonzero {part} exits: {bad}")
    if not any(name.startswith("verify-") for name in by_part["providers"]):
        raise ValueError("no package verification exits")
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
    note = (
        "RK-PLUGIN-LIBRARY-OUTER-ENVELOPE on base fa04aab0f, worktree-private logs in "
        "private/rk-outer/gate-standing-v3, gate-providers-v3, gate-docs; "
        f"standing tests {totals.group(1)} units/{totals.group(2)} summaries/"
        f"{totals.group(3)} passed/{totals.group(4)} failed/{totals.group(5)} ignored; "
        f"Lean {lean} audits; provider Rust {provider_tests} passed, provider Python {provider_python} "
        f"and scripts Python {script_tests} tests; every recorded step exited 0. "
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
