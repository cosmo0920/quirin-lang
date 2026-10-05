#!/usr/bin/env python3
"""write_gate_json.py NAME -- W43-ABI-ENUM-AND-CONST-PP (adapted from CALLEE-RETENTION-CLASS1): gate.json from the
standing gate's and the provider gate's logs in private/w43aecp/ (exits, the
runner's totals line, the Lean audit count, the provider test count, the
Python suite's count) and the Clang recording. Fails if any recorded exit is
nonzero, so a red gate never yields a green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/w43-abi-enum-and-const-pp"


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/w43aecp" / f"gate-{name}"
    provider = REPO / "private/w43aecp" / f"provider-gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    provider_exits = {path.stem: path.read_text().strip() for path in sorted(provider.glob("*.exit"))}
    bad = {k: v for k, v in {**exits, **provider_exits}.items() if v != "0"}
    if bad:
        print("nonzero exits:", bad)
        return 1
    tests = (logs / "04-tests.log").read_text()
    totals = re.search(r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored", tests)
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    provider_tests = sum(int(n) for n in re.findall(r"test result: ok\. (\d+) passed", (provider / "p2-test.log").read_text()))
    python = re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text()) if (logs / "08-python.log").exists() else None
    provider_python = re.search(r"Ran (\d+) tests", (provider / "p8-unittest.log").read_text())
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"W43-ABI-ENUM-AND-CONST-PP card gate on base 4f55fa4c4, every code step at {head[:9]}, scripts "
        "docs/reports/w43-abi-enum-and-const-pp/scripts/gate.sh (each step under the shared flock), "
        f"provider-gate.sh and finish-gate.sh, logs private/w43aecp/gate-{name} and "
        f"private/w43aecp/provider-gate-{name} of this worktree. gate-{name}: cargo check --workspace "
        "--all-targets 0; submodule 0; fmt 0; clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit 0 (private/w43aecp/tests-gate-{name}); "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed (291 + "
        "AuditAbiEnumConstPointer + AuditRetentionGuard); python tests 08 exit 0 "
        f"({python.group(1) if python else '?'} OK) on the committed matrix; docs index check 0; then "
        "build_matrix.py . --gate this file (09a), the python suite (09b) and the docs index (10b, 10c). "
        f"Provider gate (provider-gate-{name}): cargo check --locked 0, cargo test {provider_tests} passed "
        "(exit 0), fmt 0, clippy 0, contracts.py check 0, package build 0, package verify of every "
        f"package 0, providers Python suite {provider_python.group(1)} tests OK."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": provider_tests,
        "python_evidence_validator_tests": int(python.group(1)) if python else None,
        "reevaluation_probe_exit": int(exits["05-probe"]),
        "reevaluation_followup_exit": int(exits["06-followup"]),
        "cargo_fmt": "clean",
        "cargo_clippy": "clean",
        "standing_gate": "complete",
        "lean_audits": int(lean.group(1)),
        "clang": f"present: {clang}",
    }
    (REPORT / "gate.json").write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(gate, indent=2)[:800])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
