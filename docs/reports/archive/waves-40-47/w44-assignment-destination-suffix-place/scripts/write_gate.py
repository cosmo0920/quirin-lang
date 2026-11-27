#!/usr/bin/env python3
"""Collect the core and provider gate directories into the report's gate records.

Usage: write_gate.py CORE_GATE_DIR PROVIDER_GATE_DIR SOURCE_COMMIT PYTHON_TESTS DOCSINDEX_EXIT

Writes `gate-facts.json` (every step's facts file, verbatim) and `gate.json`
(the matrix generator's `--gate` shape) beside this report. PYTHON_TESTS is
the count the post-regeneration Python run reported. DOCSINDEX_EXIT is the
exit of `scripts/build_docs_index.py --check` on the final tree. The Lean audit
count is the number of `proof/Audit*.lean` files the audit step checked.
Adapted from docs/reports/w43-return-expression/scripts/write_gate.py; this
card also measured the provider gate, so its figures are recorded, not carried.
Copied from docs/reports/w44-indirect-chain-access/scripts/write_gate.py by card
W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE, which ran the core and provider steps
in one directory (pass it twice; the steps are split by name) and words its own
note.
"""

import json
import pathlib
import sys

REPORT = pathlib.Path(__file__).resolve().parents[1]
ROOT = pathlib.Path(__file__).resolve().parents[4]


def load(directory):
    return {
        path.stem.removeprefix("facts-"): json.loads(path.read_text())
        for path in sorted(pathlib.Path(directory).glob("facts-*.json"))
    }


def main() -> int:
    PROVIDER_STEPS = {"ptest", "pisolated", "pfmt", "pclippy", "pcontracts", "pbuild", "pverify", "ppython"}
    core = {k: v for k, v in load(sys.argv[1]).items() if k not in PROVIDER_STEPS}
    provider = {k: v for k, v in load(sys.argv[2]).items() if k in PROVIDER_STEPS}
    commit, python_tests, docsindex = sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
    (REPORT / "gate-facts.json").write_text(
        json.dumps(
            {
                "source_commit": commit,
                "private_logs": [str(pathlib.Path(sys.argv[1]).resolve()), str(pathlib.Path(sys.argv[2]).resolve())],
                "core": core,
                "provider": provider,
                "docsindex_exit": docsindex,
                "python_tests_after_regeneration": python_tests,
            },
            indent=2,
        )
        + "\n"
    )
    tests = core["test"]["totals"]
    census = core["census"]["totals"]
    ptests = provider["ptest"]["totals"]
    exits = {step: record["exit"] for step, record in core.items()}
    pexits = {step: record["exit"] for step, record in provider.items()}
    audits = len(list((ROOT / "proof").glob("Audit*.lean")))
    clang = (REPORT / "clang-version.txt").read_text().splitlines()[0]
    complete = all(code == 0 for code in [*exits.values(), *pexits.values(), docsindex])
    record = {
        "note": (
            f"W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: core Rust, fmt, clippy (--workspace), probes, "
            f"Lean and the provider gate measured at {commit} with CARGO_INCREMENTAL=0 and "
            "QCORE_CLANG=/usr/bin/clang. The census-feature step's first attempt at that commit failed "
            "feature-on clippy (manual_pattern_char_comparison in the feature-gated census module; "
            "measure/gate/census-first-attempt.log, exit 101, its 467 tests passing); it was fixed at "
            "95ae39ff, which changes only that compiled-out module, and the step was rerun clean there "
            "together with fmt. Later commits change documents, the ledger, report scripts and evidence "
            "only (no Rust, Lean or test source), and the Python suite ran after the final matrix "
            "regeneration. "
            f"{tests['passed']} core Rust passed / {tests['failed']} failed / {tests['ignored']} ignored "
            f"over {tests['summaries']} summaries (the card adds the 11-test assignment_destination_suffix "
            "binary, three library unit tests and two census unit tests; no existing pin moved); "
            f"{audits} Lean audits (1 added: AuditAssignmentDestinationSuffix); census step: clippy -p "
            f"qcore-c-gateway --features indirect-census clean and {census['passed']} feature-on library "
            f"tests passed; {python_tests} root Python tests after the final matrix regeneration; docs "
            f"index check exit {docsindex}. Step exits: {exits}. Provider workspace on the same tree: "
            f"{ptests['passed']} passed / {ptests['failed']} failed / {ptests['ignored']} ignored over "
            f"{ptests['summaries']} summaries; provider step exits: {pexits}. Logs: worktree-private "
            "measure/gate. Clang recorded in this checkout. Corpus: LZ4_write16 and LZ4_write32 Safe -> "
            "Unknown, unpack_basic_type provisional Unsafe -> Unknown, 0 unit moves, leak 271 -> 269, 0 "
            "violation candidates, every delta attributed."
        ),
        "workspace_tests": tests["passed"],
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
