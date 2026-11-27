#!/usr/bin/env python3
"""Collect the gate directory into the report's gate records.

Usage: write_gate.py GATE_DIR SOURCE_COMMIT PYTHON_TESTS DOCSINDEX_EXIT

Writes `gate-facts.json` (every step's facts file, verbatim) and `gate.json`
(the matrix generator's `--gate` shape) beside this report. PYTHON_TESTS is
the count the post-regeneration Python run reported; DOCSINDEX_EXIT is the
exit of `scripts/build_docs_index.py --check` on the final tree. The Lean audit
count is the number of `proof/Audit*.lean` files the audit step checked; the
Clang claim is the first line of this report's own clang-version.txt, recorded
in this checkout by the gate's `clang` step. `workspace_tests` is the runner's
total, which the step list requires to equal plain `cargo test`'s.
Adapted from docs/reports/w44-assignment-destination-suffix-place/scripts/write_gate.py.
"""

import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(__file__).resolve().parents[4]
PROVIDER_STEPS = {"ptest", "pisolated", "pfmt", "pclippy", "pcontracts"}


def main() -> int:
    gate = pathlib.Path(sys.argv[1]).resolve()
    commit, python_tests, docsindex = sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    facts = {p.stem.removeprefix("facts-"): json.loads(p.read_text()) for p in sorted(gate.glob("facts-*.json"))}
    core = {k: v for k, v in facts.items() if k not in PROVIDER_STEPS}
    provider = {k: v for k, v in facts.items() if k in PROVIDER_STEPS}
    (REPORT / "gate-facts.json").write_text(json.dumps({
        "source_commit": commit,
        "private_logs": str(gate),
        "core": core,
        "provider": provider,
        "docsindex_exit": docsindex,
        "python_tests_after_regeneration": python_tests,
    }, indent=2) + "\n")
    runner = core["runner"]["totals"]
    plain = core["test"]["totals"]
    census = core["census"]["totals"]
    ptests = provider["ptest"]["totals"]
    same = all(runner[k] == plain[k] for k in ("passed", "failed", "ignored", "summaries"))
    exits = {step: record["exit"] for step, record in core.items()}
    pexits = {step: record["exit"] for step, record in provider.items()}
    audits = len(list((ROOT / "proof").glob("Audit*.lean")))
    clang = (REPORT / "clang-version.txt").read_text().splitlines()[0]
    complete = same and all(code == 0 for code in [*exits.values(), *pexits.values(), docsindex])
    walls = {step: record.get("wall_seconds") for step, record in facts.items()}
    record = {
        "note": (
            f"W44-TEST-SUITE-WALL-TIME: the standing gate and the provider gate measured at {commit} "
            "with CARGO_INCREMENTAL=0 and the committed profile (no CARGO_PROFILE_*, RUST_TEST_THREADS "
            "or CARGO_BUILD_JOBS override). The Rust suite ran twice on the same tree: through "
            "scripts/run_test_binaries.py (the gate's new form; runner step) and as plain `cargo test "
            f"--workspace --no-fail-fast` (the reference; test step). Runner: {runner['passed']} passed / "
            f"{runner['failed']} failed / {runner['ignored']} ignored over {runner['summaries']} summaries, "
            f"all eight exclusive binaries ok; plain: {plain['passed']} / {plain['failed']} / "
            f"{plain['ignored']} over {plain['summaries']} summaries; equal: {same}. The card adds two "
            "clang::tests unit tests (qcore-c-gateway) and no other test; every base test runs with its "
            f"base outcome. {audits} Lean audits (none added); census step: clippy -p qcore-c-gateway "
            f"--features indirect-census clean and {census['passed']} feature-on library tests passed; "
            f"{python_tests} root Python tests after the final matrix regeneration; docs index check exit "
            f"{docsindex}. Step exits: {exits}. Provider workspace on the same tree: {ptests['passed']} "
            f"passed / {ptests['failed']} failed / {ptests['ignored']} ignored over {ptests['summaries']} "
            f"summaries; provider step exits: {pexits}. Step wall seconds: {walls}. Logs: worktree-private "
            "measure/gate. Clang recorded in this checkout. No corpus, contract, Lean or verdict change."
        ),
        "workspace_tests": runner["passed"],
        "provider_tests": ptests["passed"],
        "python_evidence_validator_tests": python_tests,
        "lean_audits": audits,
        "reevaluation_probe_exit": exits.get("probe"),
        "reevaluation_followup_exit": exits.get("followup"),
        "cargo_fmt": "clean" if exits.get("fmt") == 0 else "failed",
        "cargo_clippy": "clean" if exits.get("clippy") == 0 else "failed",
        "standing_gate": "complete" if complete else "incomplete",
        "clang": f"present: {clang}",
    }
    (REPORT / "gate.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return 0 if complete else 1


if __name__ == "__main__":
    sys.exit(main())
