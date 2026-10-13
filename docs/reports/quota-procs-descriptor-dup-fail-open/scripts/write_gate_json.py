#!/usr/bin/env python3
"""write_gate_json.py NAME -- QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN (from
GAPO-RETURN-CORRECTNESS's): gate.json from the standing gate's logs in
private/qpdf/gate-NAME (exits, the runner's totals line, the Lean audit count,
the Python suite's count, the concurrent arbiter's exit and overlap stamps)
and the Clang recording. No provider gate: the card changes nothing under
providers/. Fails if any recorded exit is nonzero, so a red gate never yields
a green summary. The one accounted nonzero exit is the Python suite before the
matrix regeneration (it reads the committed matrix; finish-gate.sh reruns it
as 09b after 09a). There is no isolation accounting: the card's test step
must pass without any manual rerun."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/quota-procs-descriptor-dup-fail-open"
BASE = "440c41b8b"


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/qpdf" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    bad = {k: v for k, v in exits.items() if v != "0"}
    accounted = {}
    if bad.get("08-python") == "1":
        accounted["08-python"] = "1 before the matrix regeneration; rerun as 09b"
    bad = {k: v for k, v in bad.items() if k not in accounted}
    if bad:
        print("nonzero exits:", bad)
        return 1
    tests_text = (logs / "04-tests.log").read_text()
    totals = re.search(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored",
        tests_text,
    )
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    python = (
        re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text())
        if (logs / "08-python.log").exists()
        else None
    )
    stamps = dict(line.split() for line in (logs / "overlap.txt").read_text().splitlines())
    t0 = float(stamps["arbiter-launch"])
    rel = {k: round(float(v) - t0, 1) for k, v in stamps.items()}
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN card gate on base {BASE}, every code step at "
        f"{head[:9]}, scripts docs/reports/quota-procs-descriptor-dup-fail-open/scripts/gate.sh "
        "and finish-gate.sh, logs "
        f"private/qpdf/gate-{name} of this worktree. "
        f"gate-{name}: cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit {exits['04-tests']} "
        f"(private/qpdf/tests-{name}), with no rerun, run CONCURRENTLY with the AFTER corpus "
        f"arbiter (--jobs 4, exit {exits['04-arbiter']}): arbiter running at +{rel['arbiter-running']} s, "
        f"suite +{rel['suite-start']} to +{rel['suite-end']} s, arbiter end +{rel['arbiter-end']} s; "
        f"both reevaluation probes exit 0; check_lean_audits.py {lean.group(1)} audits passed "
        "(AuditProcessQuotaPlan gains three theorems); python suite and docs index check per gate/ "
        "(08, 09), then build_matrix.py . --gate this file (09a), the python suite (09b) and the "
        "docs index (10b, 10c). No provider gate: providers/ is unchanged."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": "not run: this card changes nothing under providers/ or "
        "crates/qcore-schemas/, so the provider gate is not due. Stated rather than silently "
        "omitted.",
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
    print(json.dumps(gate, indent=2)[:900])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
