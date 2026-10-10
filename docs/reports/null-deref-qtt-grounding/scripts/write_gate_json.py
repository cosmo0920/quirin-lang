#!/usr/bin/env python3
"""write_gate_json.py NAME BASE -- NULL-DEREF-QTT-GROUNDING (copied from
NULL-SAFETY-PROPERTY's, without the provider gate: providers/ is unchanged):
gate.json from the standing gate's logs in private/ndqg/ (exits, the runner's
totals line, the Lean audit count, the Python suite's count) and the Clang
recording. Fails if any recorded exit is nonzero, so a red gate never yields a
green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/null-deref-qtt-grounding"


def main() -> int:
    name, base = sys.argv[1], sys.argv[2]
    logs = REPO / "private/ndqg" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    bad = {k: v for k, v in exits.items() if v != "0"}
    if bad:
        print("nonzero exits:", bad)
        return 1
    tests = (logs / "04-tests.log").read_text()
    totals = re.search(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored", tests
    )
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    python = re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text())
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"NULL-DEREF-QTT-GROUNDING card gate on main {base} (base), every code step at "
        f"{head[:9]}, scripts docs/reports/null-deref-qtt-grounding/scripts/gate.sh "
        "(each step under the shared flock) and finish-gate.sh, logs "
        f"private/ndqg/gate-{name} of this worktree. "
        f"gate-{name}: cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit 0 (private/ndqg/tests-gate-{name}); "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed "
        "(QCore.NullDerefGrade with AuditNullDerefGrade and QCore.DerefDecomposition with "
        "AuditDerefDecomposition added); python suite and docs index check per gate/ (08, 09: the "
        "first 08 run failed only on the stale derived Lean count, 308 against 310, and passed after "
        "the derived facts were regenerated and committed), then build_matrix.py . --gate this file "
        "(09a), the python suite (09b) and the docs index (10b, 10c). providers/ unchanged: no "
        "provider gate is due; a providers workspace cargo check --locked exits 0."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": (
            "not run: this card changes nothing under providers/ or crates/qcore-schemas/, "
            "so the provider gate is not due. Stated rather than silently omitted."
        ),
        "python_evidence_validator_tests": int(python.group(1)),
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
