#!/usr/bin/env python3
"""Publish the merged gate only when every recorded standing step passed."""

import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
GATE_LOG = ROOT / "private/w44xu/gate-final-integration-2"
REPORT = ROOT / "docs/reports/w48-w44-rk-integration"
CARD_GATES = {
    "W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT": ROOT
    / "docs/reports/w48-pointer-return-readonly/integration",
    "W44-CALLEE-CROSS-UNIT-SUMMARY": ROOT
    / "docs/reports/w44-cross-unit-summary/integration",
    "RK-PLUGIN-LIBRARY-OUTER-ENVELOPE": ROOT
    / "docs/reports/rk-plugin-library-outer-envelope/integration",
}


def capture(*args: str) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def match(pattern: str, text: str) -> re.Match[str]:
    found = re.search(pattern, text)
    if found is None:
        raise ValueError(f"missing gate evidence: {pattern}")
    return found


def main() -> None:
    expected = {
        "submodule", "rust-tests", "fmt", "clippy", "reevaluation-probe",
        "reevaluation-followup", "lean-audits", "python-tests",
    }
    exits = {}
    for line in (GATE_LOG / "exits.txt").read_text().splitlines():
        label, code = line.split()
        if label in exits:
            raise ValueError(f"duplicate gate step: {label}")
        exits[label] = int(code)
        if int((GATE_LOG / f"{label}.exit").read_text()) != exits[label]:
            raise ValueError(f"exit file mismatch: {label}")
    if set(exits) != expected or any(exits.values()):
        raise ValueError(f"incomplete or failed standing gate: {exits}")

    measured_tip = (GATE_LOG / "head.txt").read_text().strip()
    if capture("git", "rev-parse", "HEAD") != measured_tip:
        raise ValueError("standing-gate source tip differs from HEAD")
    subprocess.run(
        ["git", "diff", "--quiet", "a0ec4b37aacfee7b8e6ed9b528fdbdf1a1582e61", "HEAD", "--", "providers"],
        cwd=ROOT,
        check=True,
    )
    rust = match(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored",
        (GATE_LOG / "rust-tests.log").read_text(),
    )
    if int(rust.group(4)):
        raise ValueError("Rust test failures")
    lean = int(match(r"Passed (\d+) Lean audits", (GATE_LOG / "lean-audits.log").read_text()).group(1))
    python_log = (GATE_LOG / "python-tests.log").read_text()
    python_tests = int(match(r"Ran (\d+) tests", python_log).group(1))
    if "\nOK\n" not in python_log:
        raise ValueError("Python suite has no OK summary")
    provider = json.loads((ROOT / "docs/reports/rk-plugin-library-outer-envelope/gate.json").read_text())
    clang = subprocess.run(["clang", "--version"], cwd=ROOT, capture_output=True, text=True)
    if clang.returncode:
        raise ValueError("Clang version recording failed")
    banner = clang.stdout.splitlines()[0]
    common = {
        "workspace_tests": int(rust.group(3)),
        "provider_tests": provider["provider_tests"],
        "python_evidence_validator_tests": python_tests,
        "lean_audits": lean,
        "reevaluation_probe_exit": exits["reevaluation-probe"],
        "reevaluation_followup_exit": exits["reevaluation-followup"],
        "cargo_fmt": "clean",
        "cargo_clippy": "clean",
        "standing_gate": "complete",
        "clang": f"present: {banner}",
    }
    note = (
        "Merged W48/W44/RK stage 2 source tip " + measured_tip +
        "; merge commits W48 44e6701d6, W44 868bbcc0f, RK d882f6e00; "
        "full standing gate private/w44xu/gate-final-integration-2: "
        f"{rust.group(1)} units/{rust.group(2)} summaries, {rust.group(3)} Rust passed/"
        f"{rust.group(4)} failed/{rust.group(5)} ignored, {lean} Lean audits, "
        f"{python_tests} Python tests, all eight steps exit 0. "
        "The first integration attempt is retained with a stale matrix Lean-count failure. "
        "Provider tree is byte-identical to stage 2 tip a0ec4b37a; its passing "
        "gate-providers-v3 supplies 133 Rust tests, 25 Python tests and five verified packages. "
        "Integrated corpus, system and slice audits all exit 0 with exact row comparisons "
        "in private/w48-w44-rk-integration/; the report records counts and assumptions."
    )
    for directory, prefix in [(REPORT, ""), *[(path, card + ": ") for card, path in CARD_GATES.items()]]:
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "clang-version.txt").write_text(clang.stdout)
        (directory / "clang-version.exit").write_text("0\n")
        gate = {"note": prefix + note, **common}
        (directory / "gate.json").write_text(json.dumps(gate, indent=1) + "\n")
        print(directory / "gate.json")


if __name__ == "__main__":
    main()
