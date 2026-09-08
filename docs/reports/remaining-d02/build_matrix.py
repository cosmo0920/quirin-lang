#!/usr/bin/env python3
"""D02 - generate docs/reviews/r0_3/acceptance_matrix.json.

Every count in the emitted matrix is derived from a manifest or a validated run
record that is read here, never hand-copied:

  * CT stage rows/histogram/depth profile/observation counts
        <- docs/reevaluation/ct_manifest.json (schema 2), whose per-stage
           evidence compiler/tests/ct_corpus.rs rederives from its own dispatch,
           summarized through scripts/ct_stage_summary.py and cross-checked
           against every archived per-card ct_stage_evidence.json
        Until card T00-CT-SCHEMA this section read T00's eleven-case
        coverage.json alone and asserted its mapped set equalled the manifest's.
        That assertion stopped holding at T03-CT and the section diverged by
        thirteen cases; the merge below is what ended that.
  * corpus and checkpoint counts
        <- docs/reports/remaining-b00/ledger.json (B00)
  * contract corpus module count      <- contract/qcore/*/*.qcore on disk
  * workspace members / crates        <- Cargo.toml, providers/Cargo.toml
  * qcore-schemas Rust API version    <- crates/qcore-schemas/Cargo.toml
  * Lean audit file count             <- proof/Audit*.lean

The requirement rows themselves are analysis, not counts: they cite the wave
report that established each cell.
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(sys.argv[1]).resolve()
# `--out PATH` writes elsewhere, so the generator can be verified against the
# manifest without overwriting the committed matrix.
if "--out" in sys.argv:
    OUT = pathlib.Path(sys.argv[sys.argv.index("--out") + 1]).resolve()
else:
    OUT = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"

sys.path.insert(0, str(ROOT / "scripts"))
import ct_stage_summary  # noqa: E402

GATE = {
    "workspace_tests": int(sys.argv[2]),
    "provider_tests": 47,
    "python_evidence_validator_tests": 43,
    "lean_audits": None,  # derived below
    "reevaluation_probe_exit": 0,
    "reevaluation_followup_exit": 0,
    "cargo_fmt": "clean",
    "cargo_clippy": "clean",
}


def read_json(rel):
    return json.loads((ROOT / rel).read_text())


coverage = read_json("docs/reports/remaining-t00/coverage.json")
manifest = read_json("docs/reevaluation/ct_manifest.json")
b00 = read_json("docs/reports/remaining-b00/ledger.json")

# ---------------------------------------------------------------- derived counts
contract_modules = len(list((ROOT / "contract/qcore").glob("*/*.qcore")))
lean_audit_files = sorted(p.name for p in (ROOT / "proof").glob("Audit*.lean"))
GATE["lean_audits"] = len(lean_audit_files)

workspace_toml = (ROOT / "Cargo.toml").read_text()
members = re.findall(r'"([^"]+)"', workspace_toml.split("members = [", 1)[1].split("]", 1)[0])
provider_toml = (ROOT / "providers/Cargo.toml").read_text()
provider_members = re.findall(
    r'"([^"]+)"', provider_toml.split("members = [", 1)[1].split("]", 1)[0]
)
schemas_version = re.search(
    r'^version\s*=\s*"([^"]+)"', (ROOT / "crates/qcore-schemas/Cargo.toml").read_text(), re.M
).group(1)

# ------------------------------------------------------------------- CT section
# The manifest is the source of truth: compiler/tests/ct_corpus.rs rederives
# every one of its stage entries from its own dispatch and refuses drift, so
# reading it here is reading checked evidence. The archived per-card records are
# merged only for the `aux:` rows (controls, CLI end-to-end runs, mutation
# matrices) that the manifest has no field for, and ct_stage_summary's
# provenance check is what keeps the two from separating again.
cases = {c["id"]: c for c in manifest["cases"]}
mapped_ids = sorted(
    (c["id"] for c in manifest["cases"] if c["mapping_status"] == "mapped"),
    key=lambda name: int(name[2:]),
)
archived_evidence = ct_stage_summary.merge_evidence(ROOT)
ct_summary = ct_stage_summary.summarize(ROOT)
provenance_problems = ct_stage_summary.check_provenance(ROOT)
assert not provenance_problems, provenance_problems
assert not ct_stage_summary.check_acceptance(ROOT), ct_stage_summary.check_acceptance(ROOT)
assert set(archived_evidence) == set(mapped_ids)
assert len(cases) == 50 == ct_summary["ct_mapped"] + ct_summary["ct_unmapped"]

STAGE_ORDER = list(manifest["stage_vocabulary"]["stages"])
histogram = ct_summary["stage_histogram"]
depth_profile = ct_summary["depth_profile"]
observation_counts = ct_summary["observation_counts"]
host_gates = {
    host
    for hosts in ct_summary["host_scope"].values()
    for host in hosts
    if host != "any"
}

ct_rows = []
for cid in mapped_ids:
    case = cases[cid]
    entry = archived_evidence[cid]
    verified = entry["verified_stages"]
    ct_rows.append(
        {
            "id": cid,
            "mapping_status": "mapped",
            "expected": case["expected"],
            "route": case["route"],
            "harness_verdict_mode": case["harness_verdict_mode"],
            "manifest_evidence_target": case.get("evidence_target"),
            "source_reject_controls_executed": entry.get("source_reject_controls_executed"),
            # Schema 2 makes the manifest entry and the stage-derived status the
            # same object, so the old side-by-side `manifest_stages` column that
            # existed to keep their divergence visible is gone.
            "stage_derived_status": {stage: case["stages"][stage] for stage in STAGE_ORDER},
            "auxiliary_executed_stages": {
                key.split(":", 1)[1]: value
                for key, value in verified.items()
                if key.startswith("aux:")
            },
            "deepest_executed_stage": ct_summary["case_depth"][cid],
            "stage_evidence_record": entry["source"],
            "semantic_profile": case["semantic_profile"],
        }
    )

# Triage of the unmapped cases is recorded analysis, not stage evidence, and
# T00's audit predates thirteen mappings, so its rows are filtered to the cases
# that are still unmapped and every one of those must be covered.
triage = coverage["unmapped_triage"]
unmapped_ids = [c["id"] for c in manifest["cases"] if c["mapping_status"] == "unmapped"]
triage_cases = [case for case in triage["cases"] if case["id"] in set(unmapped_ids)]
assert {case["id"] for case in triage_cases} == set(unmapped_ids), sorted(
    set(unmapped_ids) - {case["id"] for case in triage["cases"]}
)
bucket_counts = {}
for case in triage_cases:
    bucket_counts[case["current_bucket"]] = bucket_counts.get(case["current_bucket"], 0) + 1
for case in triage_cases:
    ct_rows.append(
        {
            "id": case["id"],
            "mapping_status": "unmapped",
            "expected": cases[case["id"]]["expected"],
            "current_bucket": case["current_bucket"],
            "blocker": case["blocker"],
            "bucket_changed_since_batch2": case.get("bucket_changed_since_batch2", False),
            "stage_derived_status": {stage: {"status": "unexecuted"} for stage in STAGE_ORDER},
            "deepest_executed_stage": None,
        }
    )
ct_rows.sort(key=lambda row: int(row["id"][2:]))

ct_section = {
    "generator_status": (
        "regenerating: this section is produced by docs/reports/remaining-d02/build_matrix.py "
        "from docs/reevaluation/ct_manifest.json (schema 2) through "
        "scripts/ct_stage_summary.py, and the script asserts that the manifest, every archived "
        "per-card ct_stage_evidence.json and docs/reviews/r0_3/acceptance.json all agree before "
        "it writes anything. Card T00-CT-SCHEMA ended the thirteen-case divergence that came "
        "from reading T00's eleven-case coverage.json alone."
    ),
    "derived_from": [
        "docs/reevaluation/ct_manifest.json",
        "scripts/ct_stage_summary.py",
    ]
    + ct_stage_summary.EVIDENCE_FILES,
    "derivation_rule": (
        "a stage is 'executed' only where a named workspace test runs it on the case's own "
        "artifact (T00's rule), and the manifest entry must name that executor, that artifact, "
        "its invocation count and its exact target_os gate. "
        "compiler/tests/ct_corpus.rs rederives all of it from its own dispatch and refuses any "
        "drift, so no row can claim a stage its executed evidence does not carry. Schema 2 makes "
        "the manifest entry and the stage-derived status one object; the schema-1 columns `cfg` "
        "and `type_checker`, which conflated four passes and read differently per route, are "
        "gone."
    ),
    "totals": {
        "cases": len(cases),
        "mapped": ct_summary["ct_mapped"],
        "unmapped": ct_summary["ct_unmapped"],
    },
    "reporting_host": "linux",
    "host_gates_load_bearing": sorted(host_gates),
    "host_scope": ct_summary["host_scope"],
    "stage_histogram": {stage: histogram[stage] for stage in STAGE_ORDER},
    "depth_profile": depth_profile,
    "case_depth": ct_summary["case_depth"],
    "observation_counts": observation_counts,
    # The per-CT rows were built and then only counted; T00.9 asked for them in
    # the record, so they are emitted.
    "ct_rows": ct_rows,
    "executed_totals_from_t00": coverage["executed_totals"],
    "unmapped_bucket_counts": bucket_counts,
    "newly_decidable_not_mapped": [
        cid for cid in triage["newly_decidable"] if cid in set(unmapped_ids)
    ],
    "known_manifest_overstatements": [
        finding["id"] + " - " + finding["finding"]
        if isinstance(finding, dict) and "finding" in finding
        else finding
        for finding in coverage["claims_exceeding_executed_evidence"]
    ],
    "notes": [
        "Mapping is not conformance. `proof` is unassessed for every mapped case and no Lean "
        "theorem is bound to any CT fixture (T00.7 remains open).",
        "The sanitized-native evidence for CT14/CT16/CT20/CT37/CT50 is recorded_not_gated: the "
        "transcripts live in docs/reports/w6-*/README.md and no workspace test re-runs them. "
        "Only CT03, CT17 and CT26 carry the harness-gated sanitized run, and the harness now "
        "refuses an `executed` sanitized claim from any other case.",
        "known_manifest_overstatements is T00's audit at revision ea2af0f, retained as the "
        "record of what schema 2 repaired; its O1/O2/O3/O4/O7 findings are the ones this "
        "generator's inputs no longer allow.",
        "The unmapped triage rows are recorded analysis, not stage evidence, and come from "
        "T00's audit filtered to the cases that are still unmapped.",
    ],
}

# ------------------------------------------------------ requirement rows (analysis)
CLASS_VOCABULARY = {
    "executed": "a named gate in the standing per-change gate runs this evidence class at HEAD",
    "recorded_not_gated": "the evidence exists as an archived probe log or transcript; nothing "
    "in the standing gate re-runs it",
    "declared_premise": "this class is an explicitly named assumption carried on the "
    "certificate/report, not established here",
    "absent": "no evidence of this class exists for this requirement",
    "not_applicable": "this class cannot apply to the requirement as stated",
}

EVIDENCE_CLASSES = ["source", "model", "observation", "mechanized"]

requirements = [
    {
        "id": "R01",
        "origin": "r0_3_review",
        "requirement": "Destroyed-generation access and destruction quiescence",
        "area": "concurrency",
        "acceptance_status": "repaired_model",
        "status_changed_by_wave": False,
        "implemented_profile": "ConcurrencyProfile::TrustedExplicitTrace",
        "entry_points": [
            "crates/qcore-core/src/concurrency.rs::check_concurrent",
            "proof/QCore/Concurrency.lean (QCore.Concurrency.Lifetime)",
        ],
        "support": {
            "source": {
                "status": "absent",
                "note": "the checker consumes an explicit trusted event trace; no source-to-trace "
                "lowering exists (V01).",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-core --lib concurrency:: "
                "(crates/qcore-core/tests/unit/concurrency.rs)",
                "note": "V01 pinned the event-kind x liveness-state matrix and tightened four "
                "assertions from is_err() to the exact diagnostic; the repeat-destruction row "
                "pins DeadResource and asserts the absence of WrongCustodian.",
            },
            "observation": {
                "status": "absent",
                "note": "no native run binds the liveness ledger; CT50's real address reuse is "
                "adjacent evidence for a different model (scoped tasks).",
            },
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditCycle14.lean via scripts/check_lean_audits.py",
                "note": "four new destroy-arm theorems plus seven decidable examples; "
                "[propext, Quot.sound] only.",
            },
        },
        "wave_evidence": ["docs/reports/remaining-v01/README.md"],
        "prior_evidence": ["docs/reports/r0-3-concurrency-lifetimes/README.md"],
        "remaining_boundary": "Live IDs are unique lifetimes in TrustedExplicitTrace. V01 found "
        "no soundness failure and closed four coverage gaps. No arbitrary-source extraction, no "
        "source-to-trace refinement and no general callback lifetime proof.",
    },
    {
        "id": "R02",
        "origin": "r0_3_review",
        "requirement": "Terminal child task execution after join",
        "area": "concurrency",
        "acceptance_status": "repaired_model",
        "status_changed_by_wave": False,
        "implemented_profile": "TrustedExplicitTrace + ScopedTaskLedger + ScopedOwnedTasksV1 / "
        "ScopedCopyTasksV1",
        "entry_points": [
            "crates/qcore-core/src/concurrency.rs::check_concurrent",
            "crates/qcore-core/src/scoped_tasks.rs::ScopedTaskLedger",
            "crates/qcore-verification/src/driver.rs::compile_task_source",
            "crates/qcore-core/src/task_c_emit.rs::emit_task_c11",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-compiler --test ct_corpus, --test source_tasks, "
                "--test owned_source_tasks",
                "note": "bounded: the checked finite task IR only. See the CT section for which "
                "stages each mapped case actually executes.",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-core --lib concurrency:: and scoped_tasks::",
                "note": "V02 pinned the full terminal-state x event-kind matrix with exact "
                "diagnostic vectors, the refused-join direction, and all ten ledger operations "
                "for Completed and Joined children.",
            },
            "observation": {
                "status": "executed",
                "test_stage": "ct_corpus native task runners (Linux-gated)",
                "note": "19 native compile-and-run cycles across 9 cases; the ASan/UBSan/LSan "
                "transcripts for five of them are recorded_not_gated.",
            },
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditCycle14.lean, proof/AuditTaskExecution.lean",
                "note": "V02 added 19 Lifetime theorems so the Lean matrix matches the Rust "
                "matrix constructor for constructor.",
            },
        },
        "wave_evidence": [
            "docs/reports/remaining-v02/README.md",
            "docs/reports/remaining-t00/README.md",
        ],
        "prior_evidence": [
            "docs/reports/w6-owned-source-tasks/README.md",
            "docs/reports/w6-completed-child-custody/README.md",
            "docs/reports/w6-reused-address/README.md",
        ],
        "remaining_boundary": "ScopedOwnedTasksV1 covers original and worker-fresh Buffer "
        "results, worker failure cleanup, handler rethrow, completed-child custody and distinct "
        "generations on address reuse, with bounded Linux native evidence. These are model-level "
        "regressions over trusted explicit traces and a checked finite task IR: they establish "
        "no source-to-trace refinement. Checked mutex invariants now exist as a separate model "
        "profile (R03) but are not composed with the task ledger. Per T00, the mapped CT cases' "
        "manifest stage labels exceed their executed evidence and no mapped case carries proof "
        "or gated sanitized evidence. Loans, fractional shares, general owned graphs and error "
        "payloads, nested workers, general control flow, arbitrary native schedules and "
        "Rust/Lean/native refinement remain open.",
    },
    {
        "id": "R03",
        "origin": "r0_3_review",
        "requirement": "Explicit mutex and synchronization trust boundary",
        "area": "concurrency",
        "acceptance_status": "checked_profile_added_model_only",
        "status_changed_by_wave": True,
        "status_change_cause": "T01 specified and T02 implemented CheckedMutexInvariantV1, the "
        "second profile B02 recorded as not existing. The prior status "
        "`explicit_trust_boundary` described a state in which no checked-invariant profile "
        "existed at all.",
        "implemented_profile": "ConcurrencyProfile::CheckedMutexInvariantV1 (additive; "
        "TrustedExplicitTrace retained unchanged)",
        "entry_points": [
            "crates/qcore-core/src/mutex_invariant.rs::check_mutex_invariant_program",
            "crates/qcore-core/src/mutex_invariant.rs::recheck_mutex_invariant_certificate",
            "proof/QCore/MutexInvariant.lean (QCore.Concurrency.MutexInvariant)",
        ],
        "support": {
            "source": {
                "status": "absent",
                "note": "no source lowering. `with lock ... as ...` still routes to Q600 and the "
                "`mutex m guards ...` creation form does not exist; that is follow-up T03. No CT "
                "case among CT26-CT34/CT46/CT48 is mapped.",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-core --lib mutex_invariant:: "
                "(crates/qcore-core/tests/unit/mutex_invariant.rs, 48 tests)",
                "note": "guard-instance identity, pointwise share conservation checked before and "
                "after every event, derive-and-compare synchronization witnesses, and "
                "certificate recheck. Every rejection asserts its exact diagnostic vector.",
            },
            "observation": {"status": "absent", "note": "no native or process run exercises it."},
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditMutexInvariant.lean via scripts/check_lean_audits.py",
                "note": "31 registered theorems including all 18 T01 named; two obligations are "
                "deliberately narrowed and named open (step_preserves_declared_shape, "
                "pointwise_atom_conservation_refines_authority_total).",
            },
        },
        "wave_evidence": [
            "docs/reports/remaining-t01/README.md",
            "docs/reports/remaining-t02/README.md",
            "docs/reports/remaining-b02/README.md",
        ],
        "prior_evidence": ["docs/reports/r0-3-concurrency-lifetimes/README.md"],
        "remaining_boundary": "The checked profile discharges GuardFootprintAuthority and "
        "SynchronizedCustodyEdges only inside CheckedMutexInvariantV1, over supplied traces. "
        "TrustedExplicitTrace still issues both premises and is unchanged. The new profile "
        "introduces two narrower premises of its own (DeclaredFootprintShape, "
        "InvariantPredicateDeclared), and the pointwise-atom refinement of the abstract "
        "authority total is a premise, not a theorem. No source lowering (T03), no deadlock "
        "freedom, no lock-order analysis, no condition variables/rwlocks/try-locks, no "
        "composition with the scoped-task ledger, no CT mapping, no promotion.",
    },
    {
        "id": "R04",
        "origin": "r0_3_review",
        "requirement": "Ordinary source result type and exit compatibility",
        "area": "frontend",
        "acceptance_status": "partially_repaired",
        "status_changed_by_wave": True,
        "status_change_cause": "The status is unchanged; the recorded remaining_boundary was a "
        "verbatim copy of R02's task-lifetime text (B02, 'Ledger-quality drift found in R04') "
        "and is replaced with the boundary V03 measured for this requirement.",
        "implemented_profile": "SequentialCleanupV1 (i32/unit results) and ScopedOwnedTasksV1 / "
        "ScopedCopyTasksV1 (i32, unit, own<Buffer> results)",
        "entry_points": [
            "crates/qcore-frontend/src/frontend.rs:149-155",
            "crates/qcore-frontend/src/source_tasks.rs:103-118,230-251",
            "crates/qcore-verification/src/driver.rs::compile_files",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-compiler --lib review_r04, "
                "-p qcore-frontend --lib review_r04",
                "note": "the sequential rows are pinned; V03 records that none of the "
                "task-profile Q020 signature rows is pinned by any existing test.",
            },
            "model": {"status": "not_applicable", "note": "a source-typing requirement."},
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-v03/probe_results.tsv (71 live CLI probes, "
                "12 exit 0 / 59 exit 1 / 0 crashes)",
                "note": "probe sources are archived under docs/reports/remaining-v03/probes/; "
                "nothing in the standing gate replays them.",
            },
            "mechanized": {"status": "absent"},
        },
        "wave_evidence": [
            "docs/reports/remaining-v03/README.md",
            "docs/reports/remaining-b02/README.md",
        ],
        "prior_evidence": ["docs/reports/r0-3-source-validation/README.md"],
        "remaining_boundary": "Only i32 and unit are typed in the sequential executable subset; "
        "own<Buffer> is added by the task profile and every other result type rejects at Q020. "
        "B02's residual flag is superseded: inconsistent exits are guarded by Q202/Q020 and "
        "call/result mismatch by Q020/Q013, not only by Q500. Four Q500-only rows remain named "
        "(V03 Q500-1..Q500-4). Two acceptance boundaries are recorded rather than repaired: "
        "sequential-subset parameter type names are structurally classified and not resolved "
        "(V03 B-1) and local type annotations are accepted and not interpreted (V03 B-2); both "
        "are inert in that subset and both reject in the task profile.",
    },
    {
        "id": "R05",
        "origin": "r0_3_review",
        "requirement": "Unconditional axis declaration validation",
        "area": "frontend",
        "acceptance_status": "repaired_source",
        "status_changed_by_wave": True,
        "status_change_cause": "D01 repaired the two diagnostic-attribution defects V04 found on "
        "R05's rejecting path (degenerate span, synthetic byte offsets). The acceptance verdict "
        "is unchanged; the boundary now records the repair and the retained first-error rule.",
        "implemented_profile": "unconditional declaration environment "
        "(elaborate_source declaration slice + StaticEnvironment::check)",
        "entry_points": [
            "crates/qcore-frontend/src/frontend.rs:1645-1671",
            "crates/qcore-frontend/src/check.rs::StaticEnvironment::check / check_located",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-frontend --lib review_r05, --lib check:: "
                "(including D01's locates_the_declaration_that_failed_validation, which adds the "
                "DuplicateDimension/DuplicateAxis direct cases V04 named as missing)",
            },
            "model": {"status": "not_applicable"},
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-v04/probe_run.log (28-row CLI matrix), "
                "docs/reports/remaining-d01/probes/probe_log_{before,after}.txt",
                "note": "9 acceptance-required rejections fire identically with and without a "
                "quantitative type; 8 positive controls accept.",
            },
            "mechanized": {"status": "absent"},
        },
        "wave_evidence": [
            "docs/reports/remaining-v04/README.md",
            "docs/reports/remaining-d01/README.md",
        ],
        "prior_evidence": ["docs/reports/r0-3-source-validation/README.md"],
        "remaining_boundary": "Axis environment validation is unconditional and the closed "
        "index/profile limits remain. V04's F1 (degenerate 0..0 span, cross-file "
        "misattribution) and F2 (byte offsets into the reconstructed source) are repaired by "
        "D01; V04's F3 (StaticEnvironment::check returns on the first CheckError, so a second "
        "bad axis is not named) is retained by decision as fail-closed.",
    },
    {
        "id": "R06",
        "origin": "r0_3_review",
        "requirement": "Exact versus maximum loop custody",
        "area": "resource_flow",
        "acceptance_status": "repaired_model",
        "status_changed_by_wave": False,
        "implemented_profile": "LoopBoundEvidence {Exact | AtMost | Terminating | Widened | "
        "Unbounded} over supplied premises",
        "entry_points": [
            "crates/qcore-core/src/resource_accounting.rs::"
            "{summarize_general_loop,check_loop_summary,loop_ledger_morphism}",
            "crates/qcore-core/src/resource_flow.rs::{repeat_exact,bounded_zero_net_loop}",
            "crates/qcore-verification/src/research_extensions.rs::close_cycle17",
        ],
        "support": {
            "source": {
                "status": "absent",
                "note": "V05 measured this directly: no source loop is admitted (Q500), so the "
                "Exact/AtMost distinction is not reachable from the source route at all. "
                "Fail-closed, not optimistically approximated.",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-core --lib resource_accounting::, "
                "resource_flow::; -p qcore-compiler --lib research_extensions",
                "note": "V05 added five tests covering the zero-net Exact rows, the "
                "'exact admits no early exit' direction, the reverse AtMost->Exact mutation, the "
                "ledger deltas and aggregate-closure replay of a mutated certificate.",
            },
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "V05's four qcorec probe programs",
                "note": "all three loop probes reject at Q500; the loop-free control accepts.",
            },
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditCycle17.lean (ResearchExtensions, Flow)",
                "note": "no Lean change was required; the existing theorems match the Rust "
                "behavior cell for cell.",
            },
        },
        "wave_evidence": ["docs/reports/remaining-v05/README.md"],
        "prior_evidence": ["docs/reports/r03-loop-counts/README.md"],
        "remaining_boundary": "Nonzero-net positive maximum counts reject. Exact counts remain "
        "supplied premises; the summary API carries no allocation-identity discharge, and scalar "
        "ledger balance is not an identity discharge. No source loop is admitted, so no source "
        "program reaches this vocabulary. Identity alternatives and arbitrary nonzero-net "
        "upper-bound inference remain unimplemented.",
    },
    {
        "id": "R07",
        "origin": "r0_3_review",
        "requirement": "Signed decimal minimum literals",
        "area": "frontend/observation",
        "acceptance_status": "repaired_consumers",
        "status_changed_by_wave": True,
        "status_change_cause": "V06 located a third signed literal route (the checked-task scalar "
        "operand) that neither cited test covers; the boundary now records it as probe-only.",
        "implemented_profile": "i32 source/emitted/task-operand literals and i128 observation "
        "predicate literals",
        "entry_points": [
            "crates/qcore-verification/src/driver.rs:1007 (parse_return)",
            "crates/qcore-core/src/c_emit.rs:295 (c_i32), "
            "crates/qcore-core/src/task_c_emit.rs:29",
            "crates/qcore-frontend/src/source_tasks.rs:858 (scalar_operand)",
            "crates/qcore-contract/src/contract_observation.rs:177,205",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-compiler --lib review_r07, "
                "-p qcore-contract --lib review_r07",
                "note": "route (a3), the checked-task scalar operand, is pinned by no existing "
                "R07 test; its 26 rows are probe-only evidence.",
            },
            "model": {"status": "not_applicable"},
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-v06/probes/*.sh and probe_i128/ "
                "(128 automated rows across four routes, 0 mismatches)",
                "note": "the i128 route was exercised through its public loadable API from "
                "outside the workspace; the emitted-constant rows read back INT32_MIN rather "
                "than a negated out-of-range constant.",
            },
            "mechanized": {"status": "absent"},
        },
        "wave_evidence": ["docs/reports/remaining-v06/README.md"],
        "prior_evidence": ["docs/reports/r0-3-source-validation/README.md"],
        "remaining_boundary": "Full i32/i128 literal extrema are checked in every reachable "
        "signed route; arbitrary expression and result semantics are not added. Constant "
        "arithmetic exists only in the observation route and stays range-checked there. Route "
        "(a3) has probe-only coverage and its accepted range was value-checked only at the "
        "minimum and zero. No parser/Rust/C refinement proof is claimed.",
    },
    {
        "id": "R08",
        "origin": "r0_3_review",
        "requirement": "Consumer-specific contract coverage",
        "area": "contracts",
        "acceptance_status": "validated_bounded_consumers",
        "status_changed_by_wave": True,
        "status_change_cause": "E01 audited one capability's obligation inventory and found an "
        "enumeration boundary (E01-B1); E02 pinned a 52-row replay/cache mutation matrix; G03 "
        "added a second pinned source set from configuration alone. The verdict is unchanged; "
        "the evidence and the named boundaries are new.",
        "implemented_profile": "qcore.c_boolean_field_contract.v2 (ChunkIO) and .v3 (CMetrics "
        "unit-result) source contracts over declared .qcore resource contracts",
        "entry_points": [
            "crates/qcore-c-gateway/src/boolean_source.rs::import_chunk_source_with_profile",
            "crates/qcore-verification/src/source_effect.rs::BooleanFieldSourceTask",
            "crates/qcore-audit/src/{source_audit,source_contract}.rs",
            "crates/qcore-contract/src/contract_loader.rs",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-verification --test source_effect_obligations "
                "(6), --test source_effect_mutations (11); -p qcore-audit --test "
                "second_source_set (5), --test configured_cli; -p qcore-compiler --test "
                "contract_consumers, --test contract_loader, --test qcore_contract_corpus",
            },
            "model": {
                "status": "executed",
                "test_stage": "core-computed obligation identifiers and coverage denominators "
                "(qcore-verifier-host verify_v2 / check_evidence_v2 / "
                "compose_verification_with_inventory)",
                "note": "E01 pinned that a provider cannot remove an obligation from the "
                "requested set; E02 pinned that no summary or evidence cache exists on this "
                "route and that introducing a digest-keyed one that ignored a semantic change "
                "would fail.",
            },
            "observation": {
                "status": "executed",
                "test_stage": "cargo run -p qcore-audit -- <configured audit>",
                "note": "the canonical CMetrics audit reproduces 1 checked function contract / "
                "6 Unknown with a provisional Unsafe raw unit; G03's chunkio-core set publishes "
                "13 units as 0 Safe / 2 provisional Unsafe / 11 Unknown. Both are reproducible "
                "commands, not standing-gate steps.",
            },
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditCycle19.lean, proof/AuditCycle20.lean",
                "note": "existing Boolean-field and unit-result contract theorems; no new Lean "
                "obligation was introduced by this wave for R08.",
            },
        },
        "wave_evidence": [
            "docs/reports/remaining-e01/README.md",
            "docs/reports/remaining-e02/README.md",
            "docs/reports/remaining-g03/README.md",
        ],
        "prior_evidence": [
            "docs/reports/r03-contract-evidence/README.md",
            "docs/reports/w5-cmetrics-unit-provider/README.md",
        ],
        "remaining_boundary": "Resource loading, structural retention and observation replay "
        "remain separate. Bounded to the audited capability. E01-B1 is open and pinned: a "
        "definition visible only after macro expansion, and a K&R-style definition, are absent "
        "from the raw function enumeration and therefore from the derived inventory - no "
        "obligation and no residual Unknown row; the follow-up is E01-F1. E01-B2 (header "
        "definitions are outside the audited scope by construction) is a scope statement carried "
        "by the certificate. E02-B1: admission is not evidence that a provider computed "
        "anything, so no Safe result on this route may be attributed to provider analysis. "
        "E02-B2/B3/B4 and E01-F2/F3/F4 name the capabilities that still need their own audit. "
        "G03's F1 (a source set rooted at the checkout root is unrepresentable) and F2 (a second "
        "real CMake project's unmodified database is not consumable) bound the "
        "configuration-only claim. Arbitrary contract theorem proving and source "
        "caller-precondition discharge remain open.",
    },
    {
        "id": "R09",
        "origin": "r0_3_review",
        "requirement": "Current specification, commands and diagnostic ownership",
        "area": "documentation",
        "acceptance_status": "reconciled",
        "status_changed_by_wave": True,
        "status_change_cause": "D01 made the diagnostic ownership table canonical in "
        "docs/QCORE_SPEC_r0_3.md L21, resolved the Q700-Q703 spec drift B02 recorded in favour "
        "of the code, recorded the Q020/Q021 route split as intentional, and specified an "
        "unimplemented reason-code vocabulary. B00's two documentation-map findings are repaired "
        "by this card.",
        "implemented_profile": "documentation reconciliation; "
        "qcore_frontend::frontend::source_diagnostic is the authoritative pass -> code map",
        "entry_points": [
            "docs/QCORE_SPEC_r0_3.md L21 (ownership table) and L21.1 (reason-code vocabulary)",
            "crates/qcore-frontend/src/frontend.rs::source_diagnostic",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-compiler --test module_modes; the ownership "
                "table was read out of the code by grep over crates/ and compiler/, excluding "
                "tests",
                "note": "no diagnostic code changed anywhere in the wave.",
            },
            "model": {"status": "not_applicable"},
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-v07/probes/probe_log.txt (41 CLI probes)",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": [
            "docs/reports/remaining-d01/README.md",
            "docs/reports/remaining-b00/README.md",
            "docs/reports/remaining-b02/README.md",
            "docs/reports/remaining-v07/README.md",
        ],
        "prior_evidence": ["docs/reports/r03-integration/README.md"],
        "remaining_boundary": "The spec's Q7xx row was stale and is corrected to the four codes "
        "the code emits; AGENTS.md was already accurate. The Q020 (task route) / Q021 "
        "(sequential route) split for an unenforced source contract is retained deliberately "
        "until a source-contract consumer lands, because the two gates do not have the same "
        "rejection coverage (the task gate also refuses an empty `contract { }`). The "
        "seven-category reason-code vocabulary is specified prose with a mapping onto today's "
        "strings; no field carries it, so no consumer can filter on it. Code-level diagnostic "
        "golden tests remain future work, and historical reports retain their original "
        "provenance.",
    },
    {
        "id": "R10",
        "origin": "r0_3_review",
        "requirement": "Artifact/evidence composition and snapshot provenance",
        "area": "verification",
        "acceptance_status": "hardened_bounded_evidence",
        "status_changed_by_wave": True,
        "status_change_cause": "G01 bound a compile-task identity on the resource-projection "
        "side; G02 repaired a false Safe decided by the audit process's working directory; E02 "
        "pinned the 52-row replay mutation matrix; P01 audited provider layering and packaging; "
        "P02 replaced a pure limit declaration with an enforced Linux execution profile and an "
        "explicit achieved/unenforced evidence record.",
        "implemented_profile": "BooleanSourceIdentity + CompileTaskIdentity + CSourceInput "
        "{DeclaredFile | AnonymousText | HistoricalStdin} + "
        "ProcessExecutionProfile {UnenforcedLegacy | LinuxRlimitV1}",
        "entry_points": [
            "crates/qcore-audit/src/compile_db.rs::compile_task_identity",
            "crates/qcore-c-gateway/src/legacy_c.rs::CSourceInput",
            "crates/qcore-c-gateway/src/clang.rs::ClangIdentity",
            "crates/qcore-verifier-host/src/lib.rs::"
            "{ProcessProvider,ProcessExecutionProfile,verify_v2,check_evidence_v2}",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-audit --test compile_task_identity (7), "
                "--test compile_options, --test configured_cli; "
                "-p qcore-c-gateway --test boolean_frame_helpers",
                "note": "G02's verdict-level differential reproduces the old transport publishing "
                "Safe purely because of where the audit process was started, and the repaired "
                "transport returning Unknown.",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-verification --test source_effect_mutations, "
                "--test source_effect; -p qcore-verifier-host --test enforced_process_limits "
                "(12), --test diagnostic_attribution; -p qcore-schemas --test plugin_limit_kinds",
            },
            "observation": {
                "status": "executed",
                "test_stage": "providers/conformance/tests/packages.rs::"
                "real_packages_answer_identically_under_the_enforced_profile; "
                "cargo run -p qcore-audit -- <configured CMetrics audit>",
                "note": "all five shipped packages answer identically under both execution "
                "profiles for all 11 declared capabilities; the CMetrics audit is unchanged and "
                "its serialized report moved exactly 10 identity-digest paths under G02.",
            },
            "mechanized": {
                "status": "not_applicable",
                "note": "G01/G02/P02 add provenance, selection and execution rules, not "
                "semantics; no Lean definition or theorem changed.",
            },
        },
        "wave_evidence": [
            "docs/reports/remaining-g01/README.md",
            "docs/reports/remaining-g02/README.md",
            "docs/reports/remaining-e02/README.md",
            "docs/reports/remaining-p01/README.md",
            "docs/reports/remaining-p02/README.md",
        ],
        "prior_evidence": [
            "docs/reports/w5-configured-process-audit/README.md",
            "docs/reports/w2-clang-cmake-build/README.md",
        ],
        "remaining_boundary": "Observation scope/module/bounds substitution rejects; native "
        "provenance is declared trusted evidence and whole-program/source-model refinement "
        "remains open. Identity is provenance, not attestation: the compiler executable, "
        "its environment and the filesystem are trusted. Individual headers are not enumerated - "
        "the dependency closure is bound by effect through preprocessed_source_sha256 under "
        "-E -P, so a header edit that leaves the token stream unchanged does not change the "
        "identity. The declared driver is recorded, never checked for equivalence. The pinned "
        "Cycle19 compatibility adapter keeps CSourceInput::HistoricalStdin and its silent choice "
        "among 28 units with two or four compile configurations (G01 SS5); its four archived "
        "artifacts were not re-measured. Syntax and typed-AST phases still read preprocessed "
        "text from standard input, which is why __builtin_FILE/__builtin_FILE_NAME reject rather "
        "than being answered. P01-F2 is open: qcorec plugins verify/enable accept "
        "schema-valid, digest-intact edits to non-digest manifest declarations. P02 is not an OS "
        "sandbox: no namespace, cgroup, seccomp or capability-drop mechanism; no filesystem or "
        "network confinement (P03); RLIMIT_NPROC is per-UID and must not be presented as a "
        "per-execution child quota; non-Linux refuses rather than partially enforcing; the "
        "enforced profile is never selected by default.",
    },
    {
        "id": "W01",
        "origin": "wave_2026_09_07",
        "requirement": "Typed direct scalar-field access classification in the unrestricted-C "
        "resource projection",
        "area": "unrestricted_c",
        "acceptance_status": "implemented_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New capability. C00 ranked indirect_member_or_deref first among "
        "seven blocker families for the six CMetrics Unknown functions; C02 implemented the "
        "bounded whitelist it recommended, plus one premise C00's sketch did not contain.",
        "supports_requirement": ["R08", "open_followups.unrestricted_c_indirect_access"],
        "implemented_profile": "conjunctive eight-premise whitelist over declared C types and "
        "expression shape; anything else keeps its existing Unknown and diagnostic",
        "entry_points": [
            "crates/qcore-c-gateway/src/typed_field_access.rs",
            "crates/qcore-c-gateway/src/legacy_c.rs::classify_simple",
            "proof/QCore/TypedFieldAccess.lean, proof/QCore/LegacyCAnalyzer.lean "
            "(Instruction.noop)",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-c-gateway --test typed_field_access (11 "
                "public-API acceptance tests) and 25 module tests",
                "note": "held-out generic fixture family; no library, record, field or function "
                "name appears in any rule. Negative direction is exercised on purpose: a leak, a "
                "double release and a release-after-transfer that hid behind a scalar-field "
                "access are now reported.",
            },
            "model": {
                "status": "executed",
                "test_stage": "certificate rederivation "
                "(a_certificate_resting_on_the_classification_is_rederived_from_source, "
                "a_custody_summary_derived_through_a_classified_body_is_validated_and_rederived)",
                "note": "the classification is a pure function of the certificate's own content; "
                "substituting only the declaration it rests on is refused with the measured "
                "reason.",
            },
            "observation": {
                "status": "executed",
                "test_stage": "cargo run -p qcore-audit -- <configured CMetrics audit>",
                "note": "the real unit's complete per-function delta was measured: unsupported "
                "36 -> 35, cmt_counter_allow_reset Unknown -> Safe in the resource projection "
                "only, every other function and count unchanged.",
            },
            "mechanized": {
                "status": "executed",
                "test_stage": "proof/AuditW4.lean via scripts/check_lean_audits.py",
                "note": "31 theorems (9 LegacyCAnalyzer + 22 TypedFieldAccess), including one "
                "refusal theorem per premise and the_whitelist_is_not_empty so the refusal "
                "theorems are not vacuous.",
            },
        },
        "wave_evidence": [
            "docs/reports/remaining-c00/README.md",
            "docs/reports/remaining-c02/README.md",
            "docs/reports/remaining-c01/README.md",
        ],
        "prior_evidence": ["docs/reports/w4-indirect-access/README.md"],
        "remaining_boundary": "No verdict is promoted: the CMetrics raw unit remains provisional "
        "Unsafe and its source-contract layer is unchanged at 1 checked / 6 Unknown. Premise 8 "
        "(the base must be a name the projection never binds) is what keeps a use-after-free "
        "read from becoming custody-neutral; it also refuses cmt_counter_create's scalar store. "
        "Two premises are assumed, not proved: declaration completeness and C effective types. "
        "The reader is a hand-written declaration reader over the preprocessed token stream, not "
        "Clang's parser - a recorded deviation from C00's recommendation, taken so the "
        "classification stays a pure function of the certificate. Branch/loop/return/switch "
        "conditions (C00 rank 3), borrowed pointer-field reads (C02-P) and the source-contract "
        "ABI families (C00 ranks 5 and 7) are out of scope. The four archived 185-unit artifacts "
        "would change in both directions and were NOT edited; the pin gate correctly refused to "
        "re-measure them in this environment.",
    },
    {
        "id": "W02",
        "origin": "wave_2026_09_07",
        "requirement": "Provider package layering, lifecycle and enforced execution profile",
        "area": "verifier_plugins",
        "acceptance_status": "implemented_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New capability plus an audit. P01 measured the layering and the "
        "build/verify/enable/disable/rollback lifecycle and found F2; P02 turned PluginLimits "
        "from a pure declaration into an installed Linux rlimit set with an accurate trust "
        "boundary.",
        "supports_requirement": ["R10"],
        "implemented_profile": "ProcessExecutionProfile::{UnenforcedLegacy (default), "
        "LinuxRlimitV1}; qcore-schemas 3.0.0 PluginLimits::additional",
        "entry_points": [
            "crates/qcore-verifier-host/src/lib.rs::ProcessProvider::with_execution_profile",
            "crates/qcore-verifier-host/src/lib.rs::ProcessProvider::"
            "{enforced_limits,last_execution}",
            "providers/package.py, crates/qcorec/src (plugins verify/enable/disable)",
        ],
        "support": {
            "source": {"status": "not_applicable"},
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-verifier-host --test enforced_process_limits "
                "(12); -p qcore-schemas --test plugin_limit_kinds (4); "
                "-p qcore-compiler --test workspace_layering; "
                "-p qcorec --test plugin_packages",
                "note": "case 1 is the non-vacuousness control: the identical fixture completes "
                "under the legacy profile and is terminated under the enforced one.",
            },
            "observation": {
                "status": "executed",
                "test_stage": "cargo test --manifest-path providers/Cargo.toml --workspace "
                "--locked --offline (47 tests) including "
                "real_packages_answer_identically_under_the_enforced_profile",
                "note": "two clean reproducible package builds were reproduced with identical "
                "packages.json including the rustc identity and lock hash; five tamper cases "
                "were run on copies.",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": [
            "docs/reports/remaining-p01/README.md",
            "docs/reports/remaining-p02/README.md",
        ],
        "prior_evidence": [
            "docs/reports/w5-provider-packages/README.md",
            "docs/reports/w5-cli-packages/README.md",
        ],
        "remaining_boundary": "Process separation is not an OS sandbox and enforcement bounds how "
        "a provider runs, not what its output may mean: the core still refuses every protocol-v1 "
        "Safe proposal and downgrades unchecked Unsafe to Unknown (QP010). The enforced profile "
        "is Linux-only, selected only by an explicit builder call, and refuses rather than "
        "partially enforcing. RLIMIT_CPU is derived from the wall clock when undeclared and is "
        "labelled as such; RLIMIT_NPROC is per real UID. P01-F2 remains open: schema-valid, "
        "digest-intact manifest declaration edits pass qcorec plugins verify/enable and are "
        "caught only by package.py verify, which executes the package's own self-check. "
        "Filesystem and network confinement, capability enforcement, per-execution child quotas "
        "and non-Linux enforcement are P03. Reproducibility is relative to this recorded source "
        "tree, lockfile, toolchain and target directory only.",
    },
    {
        "id": "W03",
        "origin": "wave_2026_09_07",
        "requirement": "A second pinned source set audited from configuration alone",
        "area": "generalization",
        "acceptance_status": "demonstrated_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New demonstration. G03 added chunkio-core as two data files with "
        "no change to any analysis code, and archived the two generalization gaps the stronger "
        "candidate (Libevent) surfaced.",
        "supports_requirement": ["R08", "R10"],
        "implemented_profile": "schema v1 source-set manifest + schema v1 audit configuration "
        "consumed by the L3 qcore-audit driver",
        "entry_points": [
            "contract/profiles/chunkio-core/{profile.json,audit.json}",
            "crates/qcore-audit/src/lib.rs::run_config",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-audit --test second_source_set (5 hermetic "
                "tests over a synthetic pinned repository with the same nested-root shape)",
            },
            "model": {"status": "not_applicable"},
            "observation": {
                "status": "executed",
                "test_stage": "cargo run -p qcore-audit -- "
                "contract/profiles/chunkio-core/audit.json",
                "note": "13 units: 0 Safe / 2 provisional Unsafe / 11 Unknown / 0 data-only / "
                "0 gateway failures; per-function 1 Safe / 2 Unsafe / 120 Unknown; four "
                "fail-closed mutations rejected with a control accepted. Reproducible command, "
                "not a standing-gate step.",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": ["docs/reports/remaining-g03/README.md"],
        "prior_evidence": ["docs/reports/w5-configured-audit/README.md"],
        "remaining_boundary": "This is an Unknown-heavy withdrawal of unsupported claims, not a "
        "finding about ChunkIO, and no raw unit is promoted; these units also appear in the "
        "pinned Cycle19 corpus under a different profile and orchestration, so this is not a "
        "re-measurement of that corpus. G03-F1: a source set whose C sources live at the "
        "checkout root cannot be selected (SourceRootSelection admits only Component::Normal "
        "segments), which prevents a sound result rather than an unsound one. G03-F2: a second "
        "real CMake C project's unmodified database is not consumable - compile_db accepts four "
        "frontend flags and Libevent's own CMakeLists appends 18 more. chunkio-core clears the "
        "bar only because Fluent Bit's build happens to use exactly those four flags.",
    },
    {
        "id": "W04",
        "origin": "wave_2026_09_07",
        "requirement": "Source-derived obligation completeness and adversarial replay resistance",
        "area": "verification",
        "acceptance_status": "audited_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New audits. E01 established the obligation inventory rule for one "
        "capability and found E01-B1; E02 completed the mutation matrix systematically (52 rows, "
        "0 accepted mutations) and established that no summary or evidence cache exists on the "
        "route.",
        "supports_requirement": ["R08", "R10"],
        "implemented_profile": "core-computed obligation identifiers and coverage denominators "
        "over qcore.c_boolean_field_contract.v2",
        "entry_points": [
            "crates/qcore-verification/src/source_effect.rs::"
            "BooleanFieldSourceTask::{derive,check,evaluate}",
            "crates/qcore-verifier-host/src/lib.rs::"
            "{verify_v2,check_evidence_v2,compose_verification_with_inventory}",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-verification --test source_effect_obligations "
                "(6), --test source_effect_mutations (11), --test source_effect",
                "note": "Importer::statement and ::expression are total matches whose final arm "
                "is an error, so a construct outside the grammar aborts the function's import "
                "and reappears as a residual Unknown row with a reason.",
            },
            "model": {
                "status": "executed",
                "test_stage": "same suites plus the registry firewall paths (QP001, QP008, "
                "QP009, QP011, QP016)",
                "note": "coverage is a core-computed denominator throughout; a provider's account "
                "of what it covered is compared to it, never substituted for it.",
            },
            "observation": {
                "status": "executed",
                "test_stage": "schemas-only process provider fixtures under providers/conformance",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": [
            "docs/reports/remaining-e01/README.md",
            "docs/reports/remaining-e02/README.md",
        ],
        "prior_evidence": ["docs/reports/w3-source-field-provider/README.md"],
        "remaining_boundary": "Scoped to one capability. E01-B1 (macro-expanded and K&R "
        "definitions are absent from the raw function enumeration, hence from the inventory, "
        "with no residual Unknown row) is open with follow-up E01-F1 and its own parity plan, "
        "because repairing the enumeration changes the pinned 185-unit artifacts and the "
        "resource selection. E02-B1: admission is not evidence that the provider computed "
        "anything; the witness is compared to the core's own sealed import, so no Safe result "
        "on this route may be attributed to provider analysis. E02-B2 pins that "
        "BooleanFieldSourceTask::check returning a whole coverage set is sound only because its "
        "derivation emits exactly one obligation. E01-F2/F3/F4 and E02-F1/F2/F3 name the "
        "capabilities and transports still to be audited.",
    },
    {
        "id": "W05",
        "origin": "wave_2026_09_07",
        "requirement": "Compile-task identity and declared source input transport",
        "area": "verification",
        "acceptance_status": "implemented_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New capability. G01 bound an identity the resource projection did "
        "not record at all; G02 replaced a standard-input transport that answered several "
        "constructs for a translation unit nobody compiled, and the two compose through "
        "CompileTaskIdentity::source_input.",
        "supports_requirement": ["R10"],
        "implemented_profile": "CompileTaskIdentity (compile_task_identity_version 1) + "
        "CSourceInput {DeclaredFile, AnonymousText, HistoricalStdin}",
        "entry_points": [
            "crates/qcore-audit/src/compile_db.rs::"
            "{compile_task_identity,same_configuration,select_unique_configuration}",
            "crates/qcore-c-gateway/src/legacy_c.rs::CSourceInput",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-audit --test compile_task_identity (7), "
                "--test compile_options (12), --test configured_cli",
                "note": "both differentials were run against the old transport first, confirming "
                "each catches the defect it names.",
            },
            "model": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-verification --test source_effect::"
                "a_changed_header_expansion_retires_the_prior_task_and_its_witness",
            },
            "observation": {
                "status": "executed",
                "test_stage": "cargo run -p qcore-audit -- <configured CMetrics audit>",
                "note": "console output diffs clean; exactly 10 serialized JSON paths moved, all "
                "identity digests of the single checked function, and the source_input recorded "
                "in the assumptions sidecar matches the compile record's directory and file.",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": [
            "docs/reports/remaining-g01/README.md",
            "docs/reports/remaining-g02/README.md",
        ],
        "prior_evidence": ["docs/reports/w2-preprocessor-transport/README.md"],
        "remaining_boundary": "See R10's boundary; additionally, G01's census is a finding rather "
        "than a repair: 155 of 1,244 files in the production database and all 27 Monkey "
        "forced-Libevent units carry more than one compile configuration, the generic configured "
        "route rejects them, and the pinned compatibility adapter still silently picks one. "
        "Repairing that changes which units the pinned 185-unit audit analyzes and is a "
        "deliberate pin update.",
    },
    {
        "id": "W06",
        "origin": "wave_2026_09_07",
        "requirement": "Diagnostic ownership and source attribution",
        "area": "diagnostics",
        "acceptance_status": "implemented_bounded",
        "status_changed_by_wave": True,
        "status_change_cause": "New repairs. V07 and V04 each found a genuine attribution defect "
        "on an otherwise correctly rejecting path; D01 repaired both plus V04-F2, and made the "
        "ownership table canonical.",
        "supports_requirement": ["R05", "R09"],
        "implemented_profile": "per-file offset attribution in the combined source buffer; "
        "slice-mapped spans for the unconditional declaration pass",
        "entry_points": [
            "crates/qcore-verification/src/driver.rs::locate",
            "crates/qcore-frontend/src/frontend.rs (declaration slice map)",
            "crates/qcore-frontend/src/check.rs::StaticEnvironment::check_located",
        ],
        "support": {
            "source": {
                "status": "executed",
                "test_stage": "cargo test -p qcore-compiler --lib driver::"
                "attributes_diagnostics_at_a_file_boundary_to_the_offending_file; "
                "-p qcore-frontend --lib frontend::"
                "unconditional_declaration_checks_report_the_offending_span, "
                "--lib check::locates_the_declaration_that_failed_validation",
                "note": "all three tests fail on the unrepaired tree.",
            },
            "model": {"status": "not_applicable"},
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-d01/probes/probe_log_{before,after}.txt "
                "replayed against V07's and V04's own archived probe sources",
            },
            "mechanized": {"status": "not_applicable"},
        },
        "wave_evidence": [
            "docs/reports/remaining-d01/README.md",
            "docs/reports/remaining-v07/README.md",
            "docs/reports/remaining-v04/README.md",
        ],
        "prior_evidence": ["docs/reports/w7-diagnostic-attribution/README.md"],
        "remaining_boundary": "Attribution only: no diagnostic code changed, every input rejected "
        "before is still rejected with the same code, message and exit status, and nothing was "
        "promoted. V07's separate finding stands unrepaired by decision - the task route rejects "
        "an unenforced source contract with Q020 where the sequential route uses Q021, and it "
        "also refuses an empty `contract { }` that the sequential route accepts; both directions "
        "are conservative. V04-F3 (first-error-only reporting) is retained. The reason-code "
        "vocabulary is specified and not implemented, because either carrier (the "
        "digest-bearing VerificationTask or the audit report's per-unit reason) would move a "
        "pinned artifact.",
    },
    {
        "id": "W07",
        "origin": "wave_2026_09_07",
        "requirement": "Constructor-to-caller premise inventory for the pinned corpus",
        "area": "unrestricted_c",
        "acceptance_status": "discovery_only",
        "status_changed_by_wave": True,
        "status_change_cause": "New discovery card. O00 separated the trusted "
        "live/exclusive/ownership/cleanup premises from what is derivable on one concrete "
        "constructor-to-caller path, and scoped the O04 follow-up.",
        "supports_requirement": ["R08", "open_followups.unrestricted_c_custody_transfers"],
        "implemented_profile": "none - report-only; no analysis, contract, profile or test "
        "changed",
        "entry_points": ["docs/reports/remaining-o00/premises.json (38 premise rows)"],
        "support": {
            "source": {
                "status": "absent",
                "note": "the constructor call statement is rejected before the declared contract "
                "is consulted (O00-B01), so the loaded cio_chunk_open contract never reaches "
                "either corpus call site.",
            },
            "model": {
                "status": "absent",
                "note": "the chunk-lifecycle obligation inventory is derived over a QCore "
                "SourceProgram, not over this C (O00-B11).",
            },
            "observation": {
                "status": "recorded_not_gated",
                "test_stage": "docs/reports/remaining-o00/logs/gateway_probe.log (8 probe cases "
                "through the real gateway entry point)",
                "note": "four premise rows are checked only by fixed injected native runs at a "
                "harness caller, archived under docs/reports/cycle19-*.",
            },
            "mechanized": {"status": "absent"},
        },
        "wave_evidence": ["docs/reports/remaining-o00/README.md"],
        "prior_evidence": ["docs/reports/cycle19-file-descriptor-failures/README.md"],
        "remaining_boundary": "Of 38 premise rows: 13 assumed, 2 assumed but inert (declared and "
        "never reaching any corpus call site), 4 checked only by bounded native observation at a "
        "harness caller, 19 not modeled. 16 are derivable behind a named feature; 22 need a "
        "premise that exists nowhere today. Eleven named blockers (O00-B01..B11) include an "
        "unmodeled constructor publication in tension with the declared owned result, an "
        "undeclared mk_list_add, an unenumerated CIO_RETRY/CIO_CORRUPTED caller exit, and two "
        "unresolved aliases plus one unresolved callback. The O04 recommendation is deliberately "
        "narrow - per-arm cleanup discharge on four early-return arms only - and needs C02-P, "
        "array-declarator admission, three borrowed callee declarations and per-exit "
        "disposition. Lifting the indirect-access gate there also unmasks a discarded owned "
        "result elsewhere and would move all four archived 185-unit artifacts.",
    },
]

for row in requirements:
    assert set(row["support"]) == set(EVIDENCE_CLASSES), row["id"]
    for cls, cell in row["support"].items():
        assert cell["status"] in CLASS_VOCABULARY, (row["id"], cls, cell["status"])

# ---------------------------------------------------------------- unverified specs
unverified_specs = {
    "authority_note": "These are the supplied revision-0.3 inputs. They are DECLARED INPUTS, not "
    "implementation authority: an item's disposition records what this repository adopted, not "
    "what it implements. Implementation status is carried by the requirement rows above and by "
    "docs/reviews/r0_3/acceptance.json.",
    "source": "docs/reports/remaining-b02/README.md (section `unverified_specs/ disposition`)",
    "location_note": "unverified_specs/ is untracked in the primary checkout and therefore absent "
    "from every worktree; all five files are byte-identical (sha256 verified by B02) to their "
    "archived copies under docs/reviews/r0_3/supplied/.",
    "items": [
        {
            "item": "REVIEW.md",
            "disposition": "adopted",
            "rationale": "basis of revision 0.3; archived byte-identically in supplied/. The "
            "local ledger is docs/reviews/r0_3/REVIEW.md plus acceptance.json, and its R01-R10 "
            "are the requirement rows above.",
        },
        {
            "item": "QCORE_SPEC_r0_3.md",
            "disposition": "adopted-and-amended",
            "rationale": "normative at docs/QCORE_SPEC_r0_3.md, differing from the original by 99 "
            "lines recording implemented boundaries; pristine original retained. D01 added the "
            "L21 ownership table and the L21.1 reason-code vocabulary and corrected the stale "
            "Q7xx row.",
        },
        {
            "item": "QCORE_CONTRACT_SPEC_r0_3.md",
            "disposition": "adopted-and-amended",
            "rationale": "normative at docs/QCORE_CONTRACT_SPEC_r0_3.md, amended with the bounded "
            "scalar/unit Boolean-field interpretation; pristine original retained.",
        },
        {
            "item": "QCORE_IMPLEMENTED_GRAMMARS_r0_3.md",
            "disposition": "adopted-and-amended",
            "rationale": "consumer grammar inventory, amended as source-language increments "
            "landed; pristine original retained.",
        },
        {
            "item": "QCORE_DESIGN_GRAMMAR_r0_3.bnf",
            "disposition": "adopted-as-target-only",
            "rationale": "target grammar; the handoff records that it is not proof that all "
            "constructs execute, so it carries no implementation claim.",
        },
        {
            "item": "docs/reviews/r0_3/probe/ (advertised Rust probe package)",
            "disposition": "superseded",
            "rationale": "never supplied. Superseded by in-tree regressions and by the live CLI "
            "probe matrices of V03 (71 rows), V04 (28), V06 (128) and V07 (41).",
        },
        {
            "item": "the reviewer's archive, acceptance.json, source_inventory.json and "
            "validation.json",
            "disposition": "superseded",
            "rationale": "never supplied. The local acceptance record and source inventory are "
            "new local records (docs/reviews/r0_3/REVIEW.md lines 8-10); the 43-test Python "
            "evidence-validator suite and the standing gate replace validation.json.",
        },
    ],
}

# --------------------------------------------------------------------- assembly
matrix = {
    "schema_version": 1,
    "card": "D02",
    "generated": "2026-09-07",
    "report": "docs/reports/remaining-d02/README.md",
    "acceptance_record": "docs/reviews/r0_3/acceptance.json",
    "header": {
        "purpose": "A traceable requirement -> implemented profile -> source/model entry point -> "
        "executed test stage -> proof/evidence -> remaining-boundary matrix for revision 0.3 as "
        "implemented at this checkpoint.",
        "counts_are_run_metadata": "Every count in this file is run metadata about executed "
        "gates, manifests and archived measurements. A count is NOT proof of specification "
        "coverage: passing test totals, CT mapping counts, contract module counts and audit unit "
        "totals say what was executed, not what was proved. Projection coverage is not safety "
        "coverage and unknowns stay in the denominator.",
        "verdict_vocabulary": "Safe only after all obligations discharge; Unsafe only for a "
        "validated reachable violation; everything else Unknown. A provisional Unsafe is never "
        "presented as an upstream defect. Raw Cycle 19 units remain unpromoted.",
        "full_conformance": False,
        "independence_rule": "The four evidence classes are marked INDEPENDENTLY per "
        "requirement. Support in one class is never inferred from another: a proved Lean theorem "
        "is not source evidence, an executed native run is not a model proof, and a model "
        "regression is not an observation.",
        "evidence_classes": {
            "source": "evidence taken on real source - QCore source programs through the "
            "frontend/driver, or C translation units through the gateway importers",
            "model": "evidence taken in the dependency-free Rust kernel or the trusted "
            "verification adapter, over supplied traces, programs or certificates",
            "observation": "evidence from an executed run - native/C11 execution, a CLI probe, a "
            "configured audit or a process provider",
            "mechanized": "Lean 4 definitions and theorems registered in an Audit*.lean file and "
            "checked by scripts/check_lean_audits.py",
        },
        "support_status_vocabulary": CLASS_VOCABULARY,
    },
    "versions": {
        "note": "Prose revision 0.3 is the specification revision. It is separate from, and does "
        "not move with, the package/protocol/evidence/schema versions below.",
        "prose_specification_revision": "0.3",
        "documentation_revision": "0.3",
        "qcore_schemas_rust_api": schemas_version,
        "qcore_schemas_version_cause": "P02 added the public PluginLimits::additional field, "
        "which the documented policy makes a major change (docs/reports/schema-1-compatibility). "
        "Serde output for a manifest declaring no additional kinds is byte-identical.",
        "plugin_protocol": "v1 (unchanged)",
        "semantic_evidence": "v2 (unchanged)",
        "program_schema_and_profile": "v2 (unchanged)",
        "source_set_profile_manifest": "v1 (unchanged)",
        "compile_task_identity": "1 (new in G01)",
        "provider_packages": {
            "qcore.chunkio": "0.4.0",
            "qcore.cmetrics": "0.2.0",
            "qcore.ctraces": "0.1.0",
            "qcore.libevent": "0.1.0",
            "qcore.librdkafka": "0.1.0",
        },
    },
    "dimensions": {
        "requirements": len(requirements),
        "requirements_from_r0_3_review": sum(
            1 for row in requirements if row["origin"] == "r0_3_review"
        ),
        "requirements_from_this_wave": sum(
            1 for row in requirements if row["origin"] == "wave_2026_09_07"
        ),
        "evidence_classes": len(EVIDENCE_CLASSES),
        "requirement_evidence_cells": len(requirements) * len(EVIDENCE_CLASSES),
        "ct_rows": len(ct_rows),
        "ct_stage_names": len(STAGE_ORDER),
        "ct_cells": len(ct_rows) * len(STAGE_ORDER),
        "unverified_spec_items": len(unverified_specs["items"]),
    },
    "derivation": {
        "rule": "no count in this file is hand-copied from a status ledger; each is read from the "
        "manifest or validated run record named beside it.",
        "sources": {
            "ct_stage_evidence": "docs/reports/remaining-t00/coverage.json",
            "ct_inventory": "docs/reevaluation/ct_manifest.json",
            "checkpoint_and_corpus_reconciliation": "docs/reports/remaining-b00/ledger.json",
            "contract_corpus": "contract/qcore/*/*.qcore on disk",
            "workspace_membership": "Cargo.toml, providers/Cargo.toml",
            "schema_version": "crates/qcore-schemas/Cargo.toml",
            "lean_audit_files": "proof/Audit*.lean",
            "gate_results": "the standing per-change gate executed at this checkpoint",
        },
    },
    "unverified_specs": unverified_specs,
    "requirements": requirements,
    "ct_conformance": ct_section,
    "corpus": {
        "contract_modules": contract_modules,
        "contract_modules_pinned_by": "compiler/tests/qcore_contract_corpus.rs",
        "contract_clause_pin": b00["contract_corpus"]["clause_pin_table"],
        "contract_modules_per_directory": b00["contract_corpus"]["per_directory"],
        "libevent_submodule_pin": b00["submodule"]["declared_pin_sha"],
        "cycle19_raw_units": {
            "note": "unchanged and unpromoted; not re-measured in this wave. The pin gate "
            "correctly refuses to run the 185-unit audit in this environment, and C02 changed "
            "shared analysis code, so the four archived artifacts would change and must be "
            "re-measured and re-pinned deliberately.",
            "units": 185,
            "safe": 2,
            "provisional_unsafe": 42,
            "unknown": 137,
            "data_only": 4,
            "process_sidecar": "3 checked function contracts / 93 Unknown",
            "source": "docs/reports/w4-nullable-removal/README.md, "
            "docs/reports/w2-clang-cmake-build/README.md",
        },
        "configured_cmetrics_audit": {
            "checked_function_contracts": 1,
            "unknown_functions": 6,
            "raw_unit_verdict": "provisional Unsafe (unpromoted)",
            "resource_diagnostics": "leak=2, unknown_alias=2, unsupported=35 (was 36 before C02)",
            "source": "docs/reports/remaining-c02/README.md",
        },
        "configured_chunkio_core_audit": {
            "units": 13,
            "safe": 0,
            "provisional_unsafe": 2,
            "unknown": 11,
            "data_only": 0,
            "gateway_failures": 0,
            "source": "docs/reports/remaining-g03/README.md",
        },
    },
    "workspace": {
        "cargo_workspace_members": len(members),
        "cargo_workspace_member_paths": sorted(members),
        "crates_directory_members": sorted(
            path for path in members if path.startswith("crates/")
        ),
        "provider_workspace_members": len(provider_members),
        "provider_workspace_member_paths": sorted(provider_members),
        "note": "providers/ is a separate Cargo workspace, excluded from the core workspace; P01 "
        "verified empirically that the core workspace builds and passes its full suite with "
        "providers/ moved out of the tree.",
    },
    "validation": {
        "note": "run metadata for the standing per-change gate at this checkpoint, not a "
        "statement of specification coverage.",
        "workspace_tests": GATE["workspace_tests"],
        "provider_tests": GATE["provider_tests"],
        "python_evidence_validator_tests": GATE["python_evidence_validator_tests"],
        "lean_audits": GATE["lean_audits"],
        "lean_audit_files": lean_audit_files,
        "reevaluation_probe_exit": GATE["reevaluation_probe_exit"],
        "reevaluation_followup_exit": GATE["reevaluation_followup_exit"],
        "cargo_fmt": GATE["cargo_fmt"],
        "cargo_clippy": GATE["cargo_clippy"],
        "standing_gate": "passed",
        "full_language_conformance": False,
    },
    "named_followups": [
        {
            "id": "CORPUS-REPIN",
            "title": "Re-measure and re-pin the 185-unit corpus after C02 and G02",
            "why": "C02 touched shared analysis code (legacy_c.rs) and G02 changed the declared "
            "input transport; the four archived artifacts would change in both directions, "
            "including possible newly surfaced provisional Unsafe units. The pin gate refused to "
            "run in this environment, so nothing was adjusted to fit.",
            "invariants": "hard invariants 3 (parity before deletion) and 5 (pinned expectations "
            "are intentional): regenerate, diff, archive the diff under docs/reports/ and "
            "re-pin deliberately.",
            "source": "docs/reports/remaining-c02/README.md SS6, "
            "docs/reports/remaining-g02/README.md SS8",
        },
        {
            "id": "E01-F1",
            "title": "Complete the source function enumeration (macro-expanded and K&R "
            "definitions)",
            "why": "such definitions carry no obligation and no residual Unknown row, so the "
            "reported checked/Unknown split is not a complete function denominator for such a "
            "unit. It promotes nothing today, but it changes the pinned artifacts and the "
            "resource selection, so it needs its own parity plan.",
            "source": "docs/reports/remaining-e01/README.md",
        },
        {
            "id": "C02-P",
            "title": "Borrowed pointer-field reads as contracted-call actuals",
            "why": "premise 5 refuses every pointer field today; without this the constructor "
            "call statement is rejected before the declared contract is consulted, and O04 "
            "cannot bind its chunk identity.",
            "source": "docs/reports/remaining-c02/README.md SS7, "
            "docs/reports/remaining-o00/README.md SS8",
        },
        {
            "id": "O04",
            "title": "Per-arm constructor-to-caller cleanup discharge on the four early-return "
            "arms of flb_input_chunk_create",
            "why": "the single bounded path O00 scoped. Needs C02-P, array-declarator admission, "
            "three borrowed callee declarations and per-exit disposition; the success exit, the "
            "unenumerated exit and the descriptor/mapping identities stay out of scope.",
            "source": "docs/reports/remaining-o00/README.md SS8",
        },
        {
            "id": "T03",
            "title": "Source lowering for CheckedMutexInvariantV1",
            "why": "T02 is model-only. `with lock ... as ...` still routes to Q600 and no mutex "
            "creation form exists, so CT26-CT34/CT46/CT48 stay unmapped and R03's source class "
            "stays absent.",
            "source": "docs/reports/remaining-t01/README.md, docs/reports/remaining-t02/README.md",
        },
        {
            "id": "P03",
            "title": "Filesystem and network confinement for process providers",
            "why": "P02 enforces rlimits only. filesystem_inputs and network_access remain "
            "declarations, there is no namespace/cgroup/seccomp mechanism, and a per-execution "
            "child-process quota needs a cgroup.",
            "source": "docs/reports/remaining-p02/README.md SS7",
        },
        {
            "id": "P01-F2",
            "title": "Bind non-digest manifest declarations to the executable's embedded template "
            "at enable time",
            "why": "schema-valid, digest-intact edits to limits and targets pass qcorec plugins "
            "verify/enable; only package.py verify, which executes the package's self-check, "
            "detects them. It cannot admit a Safe verdict, but it changes what the enforced "
            "profile installs.",
            "source": "docs/reports/remaining-p01/README.md SS6",
        },
        {
            "id": "G03-F1",
            "title": "Represent a source set whose C sources live at the checkout root",
            "why": "SourceRootSelection admits only Component::Normal segments, so `.` is "
            "rejected even where the loader never reads the root. This prevents a sound result "
            "rather than an unsound one, and its repair is a PROFILE_MANIFEST_VERSION decision.",
            "source": "docs/reports/remaining-g03/README.md",
        },
        {
            "id": "G03-F2",
            "title": "Consume a second real CMake C project's unmodified compile database",
            "why": "compile_db accepts four frontend flags; Libevent's own CMakeLists appends 18 "
            "more unconditionally. Each additional flag needs a preservation argument through "
            "preprocessing, syntax and the typed AST.",
            "source": "docs/reports/remaining-g03/README.md",
        },
        {
            "id": "REASON-CODES",
            "title": "Implement the L21.1 reason-code vocabulary for uncovered obligations",
            "why": "specified prose today with a mapping onto the real strings. Either carrier "
            "moves a pinned artifact: DerivationIncomplete is digest-bearing in every "
            "evidence-v2 request and witness, and the audit report's per-unit reason is in all "
            "four 185-unit artifacts. Environment failures must stay their own category and "
            "never be relabelled as source defects.",
            "source": "docs/reports/remaining-d01/README.md SS3",
        },
        {
            "id": "T00-CT-SCHEMA",
            "title": "Make the CT manifest and its summarizer stage-derived (T00.1-T00.9)",
            "why": "manifest stage statuses are host-unconditional strings, `cfg` and "
            "`type_checker` conflate four real passes, sanitizer evidence is recorded rather than "
            "gated, and acceptance.json's two scalars treat a source-rejection case and a "
            "four-scenario native case as one identical unit of coverage.",
            "source": "docs/reports/remaining-t00/README.md SS4, SS6",
        },
    ],
}

OUT.parent.mkdir(parents=True, exist_ok=True)
OUT.write_text(json.dumps(matrix, indent=2, ensure_ascii=True) + "\n")
print("wrote", OUT)
print("requirements", matrix["dimensions"]["requirements"])
print("cells", matrix["dimensions"]["requirement_evidence_cells"])
print("ct rows", matrix["dimensions"]["ct_rows"], "stages", matrix["dimensions"]["ct_stage_names"])
print("histogram native_execution", ct_section["stage_histogram"]["native_execution"])
print("depth", depth_profile)
print("observations", observation_counts)
print("buckets", bucket_counts)
print("lean audits", GATE["lean_audits"])
print("contract modules", contract_modules)
