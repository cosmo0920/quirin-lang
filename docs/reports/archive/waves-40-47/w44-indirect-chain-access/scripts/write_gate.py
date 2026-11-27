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
    core, provider = load(sys.argv[1]), load(sys.argv[2])
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
            f"W44-INDIRECT-CHAIN-ACCESS: core Rust, fmt, clippy (--workspace), probes, Lean and the "
            f"census-feature step measured at {commit} with CARGO_INCREMENTAL=0; later commits change "
            "documents, the ledger, report scripts and evidence only (no Rust, Lean or test source), "
            "and the Python suite ran after the final matrix regeneration. "
            f"{tests['passed']} core Rust passed / {tests['failed']} failed / {tests['ignored']} ignored "
            f"over {tests['summaries']} summaries (the card adds 12 module tests and the 14-test "
            "indirect_chain_access binary, and four new tests where six existing pins moved "
            f"deliberately); {audits} Lean audits (1 added: AuditIndirectChainAccess); census step: "
            f"clippy -p qcore-c-gateway --features indirect-census clean and {census['passed']} "
            f"feature-on library tests passed; {python_tests} root Python tests after the final "
            f"matrix regeneration; docs index check exit {docsindex}. Step exits: {exits}. Provider "
            f"workspace on the same tree: {ptests['passed']} passed / {ptests['failed']} failed / "
            f"{ptests['ignored']} ignored over {ptests['summaries']} summaries; provider step exits: "
            f"{pexits}. A first core gate run (private/gate) failed to compile "
            "alias_aggregate_probe and is not evidence. Logs: worktree-private private/gate2 and "
            "private/provider-gate. Clang recorded in this checkout. Corpus: 0 function and 0 unit "
            "moves, leak 271 unchanged, 0 violation candidates."
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
