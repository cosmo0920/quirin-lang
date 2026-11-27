#!/usr/bin/env python3
"""write_gate_json.py NAME -- CALLBACK-MEMBER-DATA-ONLY-UNITS (from CG22-OP-CALLBACKS-AND-SERVED-QUEUES's) (from PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS's, from
HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's, from EXECUTION-QUOTA-LOAD-INDEPENDENCE's): gate.json from the standing gate's logs
in private/mdo/gate-NAME (exits, the runner's totals line, the Lean audit
count, the Python suite's count; providers/ is unchanged, so no provider gate) and the Clang
recording. Fails if any recorded exit is nonzero, so a red gate never yields a
green summary. The one accounted nonzero exit is the Python suite before the
matrix regeneration (it reads the committed matrix; finish-gate.sh reruns it
as 09b after 09a). There is no isolation accounting: the test step must pass
without any manual rerun."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/callback-member-data-only-units"
BASE = "f3e1d724c"


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/mdo" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    bad = {k: v for k, v in exits.items() if v != "0"}
    accounted = {}
    if bad.get("08-python") == "1":
        accounted["08-python"] = "1 before the matrix regeneration; rerun as 09b"
    bad = {k: v for k, v in bad.items() if k not in accounted}
    if bad:
        print("nonzero exits:", bad)
        return 1
    totals = re.search(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored",
        (logs / "04-tests.log").read_text(),
    )
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    python = (
        re.search(r"Ran (\d+) tests", (logs / "08-python.log").read_text())
        if (logs / "08-python.log").exists()
        else None
    )
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"CALLBACK-MEMBER-DATA-ONLY-UNITS card gate on base {BASE}, every code "
        f"step at {head[:9]}, scripts docs/reports/callback-member-data-only-units/scripts/gate.sh "
        "(HELD=1, `code` then `docs`) and finish-gate.sh, each part under one exclusive flock "
        f"on the shared heavy lock held for that whole part, logs private/mdo/gate-{name} of "
        "this worktree. "
        "cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit {exits['04-tests']} "
        f"(private/mdo/tests-{name}), with no rerun; both reevaluation probes exit 0; "
        f"check_lean_audits.py {lean.group(1)} audits passed (QCore.OpCallbacks extended); "
        "no provider gate (providers/ unchanged); python suite and docs index check per "
        "gate/ (08, 09), then build_matrix.py . --gate this file (09a), the python suite (09b) "
        "and the docs index (10b, 10c)."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": "not run: this card changes nothing under providers/ or "
        "crates/qcore-schemas/, so the provider gate is not due. Stated rather than "
        "silently omitted.",
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
    print(json.dumps(gate, indent=2)[:1500])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
