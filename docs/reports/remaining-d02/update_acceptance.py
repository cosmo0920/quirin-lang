#!/usr/bin/env python3
"""D02 - update docs/reviews/r0_3/acceptance.json where wave evidence changed a row.

Rows whose evidence did not change are left byte-for-byte alone. Each edit below
names the wave report that caused it.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(sys.argv[1]).resolve()
HEAD = sys.argv[2]
WORKSPACE_TESTS = int(sys.argv[3])
PROVIDER_TESTS = int(sys.argv[4])
LEAN_AUDITS = int(sys.argv[5])

path = ROOT / "docs/reviews/r0_3/acceptance.json"
record = json.loads(path.read_text())
matrix = json.loads((ROOT / "docs/reviews/r0_3/acceptance_matrix.json").read_text())

by_id = {finding["id"]: finding for finding in record["findings"]}
matrix_rows = {row["id"]: row for row in matrix["requirements"]}
before = {finding["id"]: json.dumps(finding, sort_keys=True) for finding in record["findings"]}

R = "docs/reports/remaining-"


def extend(finding_id, added_evidence, status=None):
    finding = by_id[finding_id]
    for report in added_evidence:
        if report not in finding["evidence"]:
            finding["evidence"].append(report)
    if status is not None:
        finding["status"] = status
    finding["remaining_boundary"] = matrix_rows[finding_id]["remaining_boundary"]


# ---- rows the wave's evidence changed, with the report that changed each ----
CHANGES = {
    "R01": {
        "evidence": [R + "v01/README.md"],
        "cause": "V01's adversarial pass over the generation-liveness gate: the event-kind x "
        "liveness-state matrix, four closed coverage gaps and four new Lean destroy-arm "
        "theorems. No soundness failure was found and no production code changed.",
    },
    "R02": {
        "evidence": [R + "t00/README.md"],
        "cause": "T00 established that the mapped CT cases' stage claims exceed their executed "
        "evidence (the sequential CFG and its reference execution are not run on any mapped "
        "case's own fixture), and T02 turned 'checked mutex invariants' from absent into a "
        "separate model profile that is not composed with the task ledger. The boundary is "
        "restated accordingly; the status is unchanged.",
    },
    "R03": {
        "evidence": [R + "t01/README.md", R + "t02/README.md", R + "b02/README.md"],
        "status": "checked_profile_added_model_only",
        "cause": "B02 classified R03 unsupported-by-profile because no checked-invariant profile "
        "existed. T01 specified one and T02 implemented CheckedMutexInvariantV1 in "
        "crates/qcore-core/src/mutex_invariant.rs with 48 tests and 31 registered Lean theorems, "
        "leaving TrustedExplicitTrace and its named premises unchanged.",
    },
    "R04": {
        "evidence": [R + "v03/README.md", R + "b02/README.md"],
        "cause": "B02 found that this row's remaining_boundary was a verbatim copy of R02's "
        "task-lifetime text, which says nothing about result typing. V03's 71-probe matrix then "
        "superseded B02's own residual flag and named the four Q500-only rows and the two "
        "acceptance boundaries B-1/B-2. The status is unchanged; the boundary is replaced with "
        "the one measured for this requirement.",
    },
    "R05": {
        "evidence": [R + "v04/README.md", R + "d01/README.md"],
        "cause": "V04's 28-row matrix confirmed the unconditional rule and found three "
        "diagnostic-attribution defects on the rejecting path; D01 repaired F1 and F2 and "
        "retained F3 by decision, and added the direct DuplicateDimension/DuplicateAxis cases "
        "V04 named as missing.",
    },
    "R06": {
        "evidence": [R + "v05/README.md"],
        "cause": "V05 pinned the full body x count x evidence matrix and the Exact->AtMost "
        "mutation in both replay paths, and measured that no source loop is admitted at all, so "
        "the distinction is unreachable from the source route.",
    },
    "R07": {
        "evidence": [R + "v06/README.md"],
        "cause": "V06 located a third signed literal route - the checked-task scalar operand, "
        "with its own range rule - that neither cited test covers, and exercised the i128 route "
        "through its public API from outside the workspace. 128 probe rows, 0 mismatches.",
    },
    "R08": {
        "evidence": [R + "e01/README.md", R + "e02/README.md", R + "g03/README.md"],
        "cause": "E01 audited the obligation inventory of one admitted capability and found the "
        "enumeration boundary E01-B1; E02 completed the replay/cache mutation matrix (52 rows, 0 "
        "accepted mutations) and recorded that no result cache exists on the route; G03 audited "
        "a second pinned source set from configuration alone and named the two generalization "
        "gaps that blocked the stronger candidate.",
    },
    "R09": {
        "evidence": [
            R + "d01/README.md",
            R + "b00/README.md",
            R + "b02/README.md",
            R + "v07/README.md",
        ],
        "cause": "D01 made the diagnostic ownership table canonical in docs/QCORE_SPEC_r0_3.md "
        "L21, resolved the Q700-Q703 drift B02 recorded in favour of the code, recorded the "
        "Q020/Q021 route split V07 found as intentional, and specified the L21.1 reason-code "
        "vocabulary without implementing it. B00's two repository-map documentation findings are "
        "repaired in AGENTS.md by card D02.",
    },
    "R10": {
        "evidence": [
            R + "g01/README.md",
            R + "g02/README.md",
            R + "e02/README.md",
            R + "p01/README.md",
            R + "p02/README.md",
        ],
        "cause": "G01 bound a compile-task identity the resource projection did not record; G02 "
        "repaired a Safe verdict decided by the audit process's working directory and made the "
        "declared input part of the identity; E02 pinned the mutation matrix; P01 audited "
        "provider layering and packaging and found F2; P02 replaced a pure limit declaration "
        "with an enforced Linux execution profile carrying an explicit achieved/unenforced "
        "record.",
    },
}

for finding_id, change in CHANGES.items():
    extend(finding_id, change["evidence"], change.get("status"))

changed = [
    finding_id
    for finding_id, text in before.items()
    if json.dumps(by_id[finding_id], sort_keys=True) != text
]
assert set(changed) == set(CHANGES), (sorted(changed), sorted(CHANGES))

# ------------------------------------------------------------- top-level updates
record["acceptance_matrix"] = "docs/reviews/r0_3/acceptance_matrix.json"

record["counts_are_run_metadata"] = (
    "Every count in this record is run metadata about executed gates and manifests. It is not "
    "proof of specification coverage: projection coverage is not safety coverage, unknowns stay "
    "in the denominator, and a provisional Unsafe is never an upstream defect claim."
)

record["schema_versions"] = matrix["versions"]

ct = matrix["ct_conformance"]
record["ct_evidence"] = {
    "note": "ct_mapped/ct_unmapped above are inventory scalars, not coverage. T00 finding O7: "
    "the scalar treats a source-rejection case with zero native runs and a case with four "
    "reference executions plus four native runs as one identical unit of coverage, and hides "
    "that 0 of 11 mapped cases carry proof or gated sanitized evidence. The stage-derived rows "
    "live in the acceptance matrix.",
    "derived_from": ct["derived_from"],
    "reporting_host": ct["reporting_host"],
    "stage_histogram": ct["stage_histogram"],
    "depth_profile": ct["depth_profile"],
    "observation_counts": ct["observation_counts"],
    "unmapped_bucket_counts": ct["unmapped_bucket_counts"],
    "newly_decidable_not_mapped": ct["newly_decidable_not_mapped"],
    "cause": "docs/reports/remaining-t00/README.md",
}

record["unverified_specs_disposition"] = {
    "authority_note": matrix["unverified_specs"]["authority_note"],
    "source": matrix["unverified_specs"]["source"],
    "location_note": matrix["unverified_specs"]["location_note"],
    "items": {item["item"]: item["disposition"] for item in matrix["unverified_specs"]["items"]},
    "detail": "docs/reviews/r0_3/acceptance_matrix.json -> unverified_specs",
}

record["validation"].update(
    {
        "workspace_tests": WORKSPACE_TESTS,
        "provider_tests": PROVIDER_TESTS,
        "lean_audits": LEAN_AUDITS,
        "latest_integration_evidence": "docs/reports/remaining-d02/README.md",
    }
)

existing_followups = {entry["area"] for entry in record["open_followups"]}
for followup in matrix["named_followups"]:
    area = followup["id"]
    if area in existing_followups:
        continue
    record["open_followups"].append(
        {
            "area": area,
            "status": followup["title"],
            "evidence": followup["source"],
        }
    )

record["latest_verified_implementation_revision"] = HEAD

record["wave_2026_09_07"] = {
    "report": "docs/reports/remaining-d02/README.md",
    "matrix": "docs/reviews/r0_3/acceptance_matrix.json",
    "rows_changed": {finding_id: change["cause"] for finding_id, change in CHANGES.items()},
    "rows_unchanged": [
        finding["id"] for finding in record["findings"] if finding["id"] not in CHANGES
    ],
    "not_re_measured": "the four archived 185-unit Cycle 19 artifacts. C02 touched shared "
    "analysis code and G02 changed the declared input transport, so they would change in both "
    "directions; the pin gate correctly refused to run the audit in this environment and nothing "
    "was adjusted to fit. Raw units remain 2 Safe / 42 provisional Unsafe / 137 Unknown plus "
    "four data-only units, unpromoted, and re-measurement plus deliberate re-pinning is the "
    "named CORPUS-REPIN follow-up.",
}

path.write_text(json.dumps(record, indent=2, ensure_ascii=True) + "\n")
print("rows changed:", ", ".join(sorted(changed)))
print("rows unchanged:", ", ".join(record["wave_2026_09_07"]["rows_unchanged"]) or "(none)")
print("followups:", len(record["open_followups"]))
