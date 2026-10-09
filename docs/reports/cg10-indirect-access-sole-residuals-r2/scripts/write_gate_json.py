#!/usr/bin/env python3
"""write_gate_json.py NAME PROVIDER-NAME -- CG10-INDIRECT-ACCESS-SOLE-RESIDUALS
round 2 (from DEEP-AST-TRANSPORTS-AND-WALL-TIME's): gate.json from the standing
gate's logs in private/cg10r2/gate-NAME (exits, the runner's totals line, the
Lean audit count, the Python suite's count), the provider gate's logs in
private/cg10r2/provider-gate-PROVIDER-NAME (providers/ changes: its cargo test
total and every exit) and the Clang recording. Fails if any recorded exit is
nonzero (the Python suite before the matrix regeneration excepted, see below),
so a red gate never yields a green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/cg10-indirect-access-sole-residuals-r2"


def main() -> int:
    name, provider = sys.argv[1], sys.argv[2]
    logs = REPO / "private/cg10r2" / f"gate-{name}"
    plogs = REPO / "private/cg10r2" / f"provider-gate-{provider}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    pexits = {path.stem: path.read_text().strip() for path in sorted(plogs.glob("*.exit"))}
    # The python suite (08) reads the committed matrix, which counts the Lean
    # audits (303 -> 304): it fails until build_matrix.py (09a) regenerates the
    # matrix with this file, and finish-gate.sh reruns it (09b), the authority.
    bad = {k: v for k, v in exits.items() if v != "0" and k != "08-python"}
    bad.update({f"provider:{k}": v for k, v in pexits.items() if v != "0"})
    if bad:
        print("nonzero exits:", bad)
        return 1
    tests = (logs / "04-tests.log").read_text()
    totals = re.search(r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored", tests)
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    python = re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text()) if (logs / "08-python.log").exists() else None
    provider_tests = sum(int(m.group(1)) for m in re.finditer(
        r"test result: ok\. (\d+) passed", (plogs / "p2-test.log").read_text()))
    provider_python = re.search(r"Ran (\d+) tests", (plogs / "p8-unittest.log").read_text())
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    phead = (plogs / "head").read_text().strip()
    packages = sorted(k[len("p7-verify-"):] for k in pexits if k.startswith("p7-verify-"))
    note = (
        f"CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2 card gate on base 84ec54d1d, every code step at "
        f"{head[:9]}, scripts docs/reports/cg10-indirect-access-sole-residuals-r2/scripts/gate.sh "
        "(each step under the shared flock) and finish-gate.sh, logs "
        f"private/cg10r2/gate-{name} of this worktree. "
        f"gate-{name}: cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit 0 (private/cg10r2/tests-gate-{name}); "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed (with the new "
        f"AuditResidualAccessReadings); python suite before the matrix regeneration 08 = {exits.get('08-python')} "
        "(Lean audits 303 -> 304 in the committed matrix), docs index check 09 = "
        f"{exits.get('09-index')}, then build_matrix.py . --gate this file (09a), the python suite (09b) "
        "and the docs index (10b, 10c). The first gate, g1 (same code but two gateway pins), failed "
        "exactly the two pins rule 14a and rule 14d flip (7,767 passed / 2 failed), updated deliberately "
        f"at f26c53cd7. Provider gate {provider} (providers/ changes) at {phead[:9]}: cargo check --locked, "
        f"cargo test {provider_tests} passed, fmt, clippy, contracts.py check, package build and verify of "
        f"{len(packages)} packages ({', '.join(packages)}), providers Python "
        f"{provider_python.group(1) if provider_python else '?'} tests OK, every exit 0 "
        f"(private/cg10r2/provider-gate-{provider})."
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
