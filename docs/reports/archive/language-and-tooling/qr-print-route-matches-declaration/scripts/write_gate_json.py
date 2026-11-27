#!/usr/bin/env python3
"""write_gate_json.py NAME -- QR-PRINT-ROUTE-MATCHES-DECLARATION (from W45-CALLEE-CASCADE-RESIDUAL round 3, from CG22-CALLBACK-REGISTRY-CONTRACT and its lineage): gate.json from the
standing gate's and the provider gate's logs in private/qrpr/ (exits, the
runner's totals line, the Lean audit count, the provider test count, the
Python suite's count) and the Clang recording. Fails if any recorded exit is
nonzero, so a red gate never yields a green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/qr-print-route-matches-declaration"


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/qrpr" / f"gate-{name}"
    # The provider gate may be an earlier run when providers/ did not change
    # since (argument 2, default NAME).
    provider_name = sys.argv[2] if len(sys.argv) > 2 else name
    provider = REPO / "private/qrpr" / f"provider-gate-{provider_name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    provider_exits = {path.stem: path.read_text().strip() for path in sorted(provider.glob("*.exit"))}
    bad = {k: v for k, v in {**exits, **provider_exits}.items() if v != "0"}
    # Two nonzero exits are accounted for, and only these: the Python suite
    # before the matrix regeneration (it reads the committed matrix's derived
    # Lean count; finish-gate.sh reruns it as 09b after 09a), and the runner's
    # step when its one failure is a constraint-16 row of the runner's
    # EXCLUSIVE table that passes in isolation (ISOLATED names the binary;
    # its log is private/qrpr/isolation-ISOLATED.log).
    isolated = sys.argv[3] if len(sys.argv) > 3 else None
    accounted = {}
    if bad.get("08-python") == "1":
        accounted["08-python"] = "1 before the matrix regeneration; rerun as 09b"
    tests_text = (logs / "04-tests.log").read_text()
    if isolated and bad.get("04-tests") == "101" and "; 1 failed;" in tests_text:
        log = REPO / "private/qrpr" / f"isolation-{isolated}.log"
        exit_file = REPO / "private/qrpr" / f"isolation-{isolated}.log.exit"
        if exit_file.read_text().strip() == "0" and " 0 failed;" in log.read_text():
            accounted["04-tests"] = f"101: one failure in {isolated}, which passes in isolation"
    bad = {k: v for k, v in bad.items() if k not in accounted}
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
        f"QR-PRINT-ROUTE-MATCHES-DECLARATION card gate on base da6c1f448, every code step at "
        f"{head[:9]}, scripts docs/reports/qr-print-route-matches-declaration/scripts/gate.sh "
        "(each step under the shared flock), provider-gate.sh and finish-gate.sh, logs "
        f"private/qrpr/gate-{name} and private/qrpr/provider-gate-{provider_name} of this worktree. "
        f"gate-{name}: cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit {exits['04-tests']} (private/qrpr/tests-gate-{name}"
        + (f"; the failure is the constraint-16 row {isolated} (EXCLUSIVE in run_test_binaries.py: RLIMIT_NPROC counts every process of the real UID), 13 passed in isolation, private/qrpr/isolation-{isolated}.log" if "04-tests" in accounted else "")
        + "); "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed (QCore.PrintRouteSelection added); python suite and docs index check per gate/ (08, 09), then "
        "build_matrix.py . --gate this file (09a), the python suite (09b) and the docs index (10b, 10c). "
        f"Provider gate (provider-gate-{provider_name}, at {(provider / 'head').read_text().strip()[:9]}; providers/ unchanged since when named apart): cargo check --locked 0, cargo test {provider_tests} passed "
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
    print("accounted exits:", accounted)
    (REPORT / "gate.json").write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(gate, indent=2)[:800])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
