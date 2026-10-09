#!/usr/bin/env python3
"""write_gate_json.py NAME -- VERDICT-BODY-STANDING-CLAUSE-SCOPE (adapted from
VERDICT-CONTRACT-DEPENDENCIES): gate.json from the standing gate's logs in
private/vbs/gate-NAME (exits, the runner's totals line, the Lean audit count,
the Python suite's count) and the Clang recording. providers/ is unchanged
from the base (`git diff 9564eeb6d -- providers` is empty), so no provider gate
runs; `provider_tests` (a key the matrix requires) is carried from
VERDICT-CONTRACT-DEPENDENCIES's provider gate pg3 and labelled so in the note.
Fails if any recorded exit is nonzero (the Python suite before the matrix
regeneration excepted, see below), so a red gate never yields a green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/verdict-body-standing-clause-scope"
CARRIED_PROVIDER_TESTS = 129


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/vbs" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    # The python suite (08) reads the committed matrix, which counts the Lean
    # audits (299 -> 300): it fails until build_matrix.py (09a) regenerates the
    # matrix with this file, and finish-gate.sh reruns it (09b), the authority.
    bad = {k: v for k, v in exits.items() if v != "0" and k != "08-python"}
    if bad:
        print("nonzero exits:", bad)
        return 1
    tests = (logs / "04-tests.log").read_text()
    totals = re.search(r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored", tests)
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    python = re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text()) if (logs / "08-python.log").exists() else None
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"VERDICT-BODY-STANDING-CLAUSE-SCOPE card gate on base 9564eeb6d, every code step at "
        f"{head[:9]}, scripts docs/reports/verdict-body-standing-clause-scope/scripts/gate.sh "
        "(each step under the shared flock) and finish-gate.sh, logs "
        f"private/vbs/gate-{name} of this worktree. "
        f"gate-{name}: cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit 0 (private/vbs/tests-gate-{name}); "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed (with the new "
        f"AuditBodyClauseStanding); python suite before the matrix regeneration 08 = {exits.get('08-python')} "
        "(Lean audits 299 -> 300 in the committed matrix), docs index check 09 = "
        f"{exits.get('09-index')}, then build_matrix.py . --gate this file (09a), the python suite (09b) "
        "and the docs index (10b, 10c). No provider gate: providers/ is unchanged from the base; "
        f"provider_tests {CARRIED_PROVIDER_TESTS} is CARRIED from VERDICT-CONTRACT-DEPENDENCIES's "
        "provider gate pg3 (at 58bc2b290), not measured here."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": CARRIED_PROVIDER_TESTS,
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
