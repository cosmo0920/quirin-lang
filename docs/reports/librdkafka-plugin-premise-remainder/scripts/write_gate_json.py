#!/usr/bin/env python3
"""write_gate_json.py NAME -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED, from OP-CALLBACK-MEMBER-PATH-RELEASE, from SUPPLIED-OWNED-RETURN-CODE-HANDOFF, from OP-APP-RK-CALLBACKS-VIA-POLL-CB) (from OP-CALLBACK-CANDIDATE-CONTRACTS, from OP-CONSTRUCTOR-DECLARATIONS, from CG22-OP-CALLBACKS-AND-SERVED-QUEUES's, from PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS's, from
HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's, from EXECUTION-QUOTA-LOAD-INDEPENDENCE's): gate.json from the standing gate's logs
in private/lppr/gate-NAME (exits, the runner's totals line, the Lean audit
count, the Python suite's count, the provider gate's counts) and the Clang
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
REPORT = REPO / "docs/reports/librdkafka-plugin-premise-remainder"
BASE = "711287771"


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/lppr" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    bad = {k: v for k, v in exits.items() if v != "0"}
    accounted = {}
    if bad.get("08-python") == "1":
        accounted["08-python"] = "1 before the matrix regeneration; rerun as 09b"
    if bad.get("09-index") == "1":
        # As in LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: the report index may not be
        # regenerated for the new report directory before the gate;
        # finish-gate.sh regenerates it (10b) and checks it again (10c).
        accounted["09-index"] = "1: index stale before its regeneration; regenerated as 10b, checked as 10c"
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
    provider = re.findall(
        r"test result: ok\. (\d+) passed; (\d+) failed", (logs / "12-provider-tests.log").read_text()
    )
    provider_passed = sum(int(p) for p, _ in provider)
    provider_failed = sum(int(f) for _, f in provider)
    provider_python = re.search(r"Ran (\d+) tests", (logs / "13-provider-python.log").read_text())
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    note = (
        f"LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER card gate on base {BASE}, every code "
        f"step at {head[:9]}, scripts docs/reports/librdkafka-plugin-premise-remainder/scripts/gate-whole.sh, "
        "gate.sh and finish-gate.sh, one exclusive flock on the shared heavy lock held for the "
        f"whole gate, logs private/lppr/gate-{name} of this worktree. "
        "cargo check --workspace --all-targets 0; submodule 0; cargo fmt --check 0; "
        "clippy --locked 0; run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit {exits['04-tests']} "
        f"(private/lppr/tests-{name}), with no rerun; both reevaluation probes exit 0; "
        f"check_lean_audits.py {lean.group(1)} audits passed (325: 324 plus AuditNullMemberPremise); "
        "provider gate: providers/package.py build and verify qcore.libevent and qcore.librdkafka 0 / 0 / 0, cargo check --locked in providers/ 0, providers/contracts.py check 0, "
        f"cargo test in providers/ {provider_passed} passed / {provider_failed} failed, "
        f"providers/tests {provider_python.group(1)} OK; python suite and docs index check per "
        "gate/ (08, 09), then build_matrix.py . --gate this file (09a), the python suite (09b) "
        "and the docs index (10b, 10c)."
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": provider_passed,
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
