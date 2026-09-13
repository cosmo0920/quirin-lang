#!/usr/bin/env python3
"""D02 - generate docs/reviews/r0_3/acceptance_matrix.json.

Every count in a section this generator OWNS is derived from a manifest or a
validated run record that is read here, never hand-copied:

  * CT stage rows/histogram/depth profile/observation counts
        <- docs/reevaluation/ct_manifest.json (schema 2), whose per-stage
           evidence compiler/tests/ct_corpus.rs rederives from its own dispatch,
           summarized through scripts/ct_stage_summary.py and cross-checked
           against every archived per-card ct_stage_evidence.json
        Until card T00-CT-SCHEMA this section read T00's eleven-case
        coverage.json alone and asserted its mapped set equalled the manifest's.
        That assertion stopped holding at T03-CT and the section diverged by
        thirteen cases; the merge below is what ended that.
        The newly-decidable list and its companion reasons map are derived
        here too since card GEN-CTREASONS (follow-on `D02-GEN-CTREASONS`);
        until then the map was hand-maintained commentary this generator
        dropped on every run, so nothing could refuse a stale row.
  * contract clause pin / per-directory counts
        <- docs/reports/remaining-b00/ledger.json (B00)
  * contract corpus module count      <- contract/qcore/*/*.qcore on disk
  * workspace members / crates        <- Cargo.toml, providers/Cargo.toml
  * qcore-schemas Rust API version    <- crates/qcore-schemas/Cargo.toml
  * provider package versions         <- providers/*/provider-template.json
                                         (the manifest template P01-F2 binds)
  * Lean audit file count             <- proof/Audit*.lean

Sections this generator does NOT own - the requirement rows, the unverified-spec
dispositions, the header prose, the named follow-ons and the measured audit
figures - are recorded analysis, not counts. Until card D02-GEN-REST they were
Python literals from the original D02 wave, so a whole-file regeneration
silently reverted every later reconciliation (the named_followups literal held
eleven entries against the committed nineteen, and the gate block hard-coded
provider_tests 47 against the real 67). They are now CARRIED VERBATIM from the
committed matrix, which the integrator owns; this script refuses to run without
it rather than reintroducing a stale copy. `section_ownership` in the emitted
file records who owns each section, so nothing is silently hand-maintained.

Gate measurements are integrator-supplied: pass `--gate PATH` to a JSON object
holding the numbers actually measured at this checkpoint. Without it the
previous matrix's numbers are carried and labelled as carried, so a stale count
is visible in the file rather than invented here.

Usage:
    build_matrix.py ROOT [WORKSPACE_TESTS] [--out PATH] [--previous PATH]
                         [--gate PATH]
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(sys.argv[1]).resolve()


def option(name):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else None


# `--out PATH` writes elsewhere, so the generator can be verified against the
# manifest without overwriting the committed matrix.
COMMITTED = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
OUT = pathlib.Path(option("--out")).resolve() if option("--out") else COMMITTED
# The carried sections come from the committed matrix by default, and are read
# before anything is written, so an in-place regeneration carries its own file.
PREVIOUS = pathlib.Path(option("--previous")).resolve() if option("--previous") else COMMITTED

sys.path.insert(0, str(ROOT / "scripts"))
import ct_stage_summary  # noqa: E402

# ------------------------------------------------------------ section ownership
# `derived`            recomputed here from a manifest, a validated record or
#                      the tree; a hand edit to it is overwritten on the next
#                      regeneration, deliberately.
# `integrator_supplied` a measurement this script cannot take: it comes from
#                      --gate, else it is carried and labelled as carried.
# `hand_maintained`    recorded analysis owned by the integrator and carried
#                      verbatim; this script never authors or reverts it.
DERIVED = "derived"
INTEGRATOR_SUPPLIED = "integrator_supplied"
HAND_MAINTAINED = "hand_maintained"

OWNERSHIP_VOCABULARY = {
    DERIVED: "recomputed by docs/reports/remaining-d02/build_matrix.py from the named manifest, "
    "validated record or working tree; a hand edit here is overwritten on the next regeneration",
    INTEGRATOR_SUPPLIED: "a measurement this script cannot take; supplied through --gate, "
    "otherwise carried from the previous matrix and labelled as carried",
    HAND_MAINTAINED: "recorded analysis owned by the integrator; carried verbatim from the "
    "previous matrix and never authored or reverted by this script",
}

SECTION_OWNERS = {
    "schema_version": (DERIVED, "generator constant"),
    "card": (DERIVED, "generator constant"),
    "generated": (DERIVED, "generator constant - the date the D02 matrix shape was established"),
    "report": (DERIVED, "generator constant"),
    "acceptance_record": (DERIVED, "generator constant"),
    "section_ownership": (DERIVED, "this block"),
    "header": (HAND_MAINTAINED, "vocabulary and framing prose; support_status_vocabulary is "
               "checked against this script's validator and refuses on drift"),
    "versions": ("mixed", "see section_ownership.fields"),
    "dimensions": (DERIVED, "counted from the emitted rows"),
    "derivation": (DERIVED, "generator constant naming its own inputs"),
    "unverified_specs": (HAND_MAINTAINED, "B02's disposition of the supplied revision-0.3 inputs"),
    "requirements": (HAND_MAINTAINED, "R01-R10 and W01-W07 analysis rows; every cell's status is "
                     "revalidated here against the closed class vocabulary"),
    "ct_conformance": (DERIVED, "docs/reevaluation/ct_manifest.json through "
                       "scripts/ct_stage_summary.py, cross-checked against every archived "
                       "per-card ct_stage_evidence.json and docs/reviews/r0_3/acceptance.json"),
    "corpus": ("mixed", "see section_ownership.fields"),
    "workspace": (DERIVED, "Cargo.toml, providers/Cargo.toml"),
    "validation": ("mixed", "see section_ownership.fields"),
    "named_followups": (HAND_MAINTAINED, "the open follow-on ledger; waves add entries to the "
                        "committed matrix and this script carries them"),
    "updated": (INTEGRATOR_SUPPLIED, "the integrator's own note for this checkpoint"),
}

FIELD_OWNERS = {
    "versions.qcore_schemas_rust_api": (DERIVED, "crates/qcore-schemas/Cargo.toml"),
    "versions.provider_packages": (DERIVED, "providers/*/provider-template.json package_version"),
    "versions.*": (HAND_MAINTAINED, "protocol/evidence/schema prose and the version-cause note"),
    "corpus.contract_modules": (DERIVED, "contract/qcore/*/*.qcore on disk"),
    "corpus.contract_modules_pinned_by": (DERIVED, "generator constant"),
    "corpus.contract_clause_pin": (DERIVED, "docs/reports/remaining-b00/ledger.json"),
    "corpus.contract_modules_per_directory": (DERIVED, "docs/reports/remaining-b00/ledger.json"),
    "corpus.libevent_submodule_pin": (DERIVED, "docs/reports/remaining-b00/ledger.json"),
    "corpus.*": (HAND_MAINTAINED, "measured audit figures (cycle19_raw_units and the configured "
                 "audits) whose measurement lives in the card report named beside them"),
    "validation.lean_audits": (DERIVED, "proof/Audit*.lean on disk"),
    "validation.lean_audit_files": (DERIVED, "proof/Audit*.lean on disk"),
    "validation.full_language_conformance": (DERIVED, "generator constant: false"),
    "validation.measurement_provenance": (DERIVED, "records whether the gate numbers beside it "
                                          "were supplied for this run or carried"),
    "validation.*": (INTEGRATOR_SUPPLIED, "standing-gate measurements from --gate, else carried"),
}

# Gate keys this script cannot measure. `lean_audits` is excluded on purpose: it
# is a count of files on disk, so it is derived rather than supplied.
GATE_KEYS = [
    "note",
    "workspace_tests",
    "provider_tests",
    "python_evidence_validator_tests",
    "reevaluation_probe_exit",
    "reevaluation_followup_exit",
    "cargo_fmt",
    "cargo_clippy",
    "standing_gate",
]


def read_json(rel):
    return json.loads((ROOT / rel).read_text(encoding="utf-8"))


if not PREVIOUS.is_file():
    sys.exit(
        f"{PREVIOUS} does not exist. This generator carries the hand-maintained sections "
        "(header, requirements, unverified_specs, named_followups and the measured corpus "
        "figures) from the committed matrix rather than holding a second copy of them, so it "
        "refuses to write a matrix that would silently drop them. Restore the file (git "
        "checkout docs/reviews/r0_3/acceptance_matrix.json) or pass --previous PATH."
    )
previous = json.loads(PREVIOUS.read_text(encoding="utf-8"))

MISSING = [key for key, (owner, _) in SECTION_OWNERS.items() if owner != DERIVED and key not in previous]
if MISSING:
    sys.exit(f"{PREVIOUS} is missing carried section(s): {MISSING}")


def carried(key):
    return previous[key]


gate_path = option("--gate")
if gate_path:
    supplied = json.loads(pathlib.Path(gate_path).read_text(encoding="utf-8"))
    unknown = sorted(set(supplied) - set(GATE_KEYS))
    if unknown:
        sys.exit(f"--gate {gate_path} carries unknown key(s) {unknown}; expected {GATE_KEYS}")
    missing = sorted(set(GATE_KEYS) - set(supplied))
    if missing:
        sys.exit(f"--gate {gate_path} is missing {missing}; a partial gate is never merged")
    GATE = dict(supplied)
    GATE_PROVENANCE = f"integrator_supplied ({gate_path})"
else:
    previous_validation = previous["validation"]
    missing = sorted(key for key in GATE_KEYS if key not in previous_validation)
    if missing:
        sys.exit(f"{PREVIOUS} validation block is missing {missing} and no --gate was given")
    GATE = {key: previous_validation[key] for key in GATE_KEYS}
    GATE_PROVENANCE = (
        "carried_from_previous_matrix: these numbers were NOT measured for this regeneration; "
        "pass --gate PATH with the standing-gate results of this checkpoint to replace them"
    )

# The documented command `build_matrix.py . <workspace_test_count>` stays valid:
# a positional count overrides the workspace_tests entry only.
if len(sys.argv) > 2 and not sys.argv[2].startswith("--"):
    GATE["workspace_tests"] = int(sys.argv[2])
    if not gate_path:
        GATE_PROVENANCE = (
            "workspace_tests supplied positionally; every other gate number carried from the "
            "previous matrix and NOT measured for this regeneration"
        )


coverage = read_json("docs/reports/remaining-t00/coverage.json")
manifest = read_json("docs/reevaluation/ct_manifest.json")
b00 = read_json("docs/reports/remaining-b00/ledger.json")

# ---------------------------------------------------------------- derived counts
contract_modules = len(list((ROOT / "contract/qcore").glob("*/*.qcore")))
lean_audit_files = sorted(p.name for p in (ROOT / "proof").glob("Audit*.lean"))

workspace_toml = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
members = re.findall(r'"([^"]+)"', workspace_toml.split("members = [", 1)[1].split("]", 1)[0])
provider_toml = (ROOT / "providers/Cargo.toml").read_text(encoding="utf-8")
provider_members = re.findall(
    r'"([^"]+)"', provider_toml.split("members = [", 1)[1].split("]", 1)[0]
)
schemas_version = re.search(
    r'^version\s*=\s*"([^"]+)"',
    (ROOT / "crates/qcore-schemas/Cargo.toml").read_text(encoding="utf-8"),
    re.M,
).group(1)

# Provider package versions come from the manifest template embedded in each
# provider executable (P01-F2), which is the artifact `qcorec plugins verify`
# compares an installed manifest against - not from the artifact crate's
# Cargo.toml, which carries a different version.
provider_packages = {}
for template in sorted((ROOT / "providers").glob("qcore-provider-*/provider-template.json")):
    declared = json.loads(template.read_text(encoding="utf-8"))
    provider_packages[declared["plugin_id"]] = declared["package_version"]
assert provider_packages, "no provider manifest template found under providers/"

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
tool_gates = {
    tool
    for tools in ct_summary["tool_scope"].values()
    for tool in tools
    if tool != "none"
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
            # The epoch route counts its controls under its own key: both its
            # accepted cases and its rejections carry them, so `source_reject`
            # would be the wrong name for the CT35/CT36 rows (card T08-SRC-CT).
            "epoch_controls_executed": entry.get("epoch_controls_executed"),
            # The legacy_c route counts its controls under its own key too: a
            # C twin's controls are gateway analyses, and one of them varies the
            # declared contracts rather than the source (card CT-CTWIN).
            "legacy_c_controls_executed": entry.get("legacy_c_controls_executed"),
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
# One authority for the tally (card HYGIENE-14, finding D02-BUCKETS-STALE):
# this generator's counts were the *correct* ones while `acceptance.json`'s
# hand-kept mirror drifted to a sum of 22, so the derivation moved into the
# shared summarizer and both records now read the same function, which refuses
# a tally that does not sum to the number of unmapped cases.
bucket_counts = ct_stage_summary.unmapped_bucket_counts(ROOT, manifest)
assert sum(bucket_counts.values()) == len(unmapped_ids)
# The *label* is derived too since card CT-LABELS (finding D02-BUCKETS-LABELS).
# T00's `blocker` text is frozen at revision ea2af0f, and for a case whose
# blocker has since been repaired it reads as a false statement about the
# implementation. CT34 was the worked example — its frozen text says "no source
# form creates or declares a mutex", which card T03 and then card CT33-RANKS
# made untrue — and card CT34-GROWTH has since supplied the growth form, so the
# wave-13 CT lane mapped CT34 and it leaves this table entirely. What replaced
# the hand-kept `SUPERSEDING_REASONS` reader that used to sit here is the shared
# resolution rule: each unmapped case's current label is the newest CT lane
# record that measured it, else T00's archive, and both halves are published.
# The archived label and blocker stay beside the current ones, so the history
# remains visible rather than being overwritten.
labels = ct_stage_summary.unmapped_labels(ROOT, manifest)
assert set(labels) == set(unmapped_ids)
# CT34 left this table by being mapped (wave-13 CT lane), not by its blocker
# being quietly dropped. Asserting the transition is what keeps "CT34 no longer
# appears here" from reading as an accident.
assert "CT34" not in unmapped_ids, "CT34 is mapped; its frozen blocker text is no longer published"
for case in triage_cases:
    label = labels[case["id"]]
    measured = label["label_source"] != ct_stage_summary.ARCHIVE_SOURCE
    row = {
        "id": case["id"],
        "mapping_status": "unmapped",
        "expected": cases[case["id"]]["expected"],
        "current_bucket": label["label"],
        "current_bucket_source": label["label_source"],
        # T00's archived analysis, kept beside the current label whether or not a
        # lane has since measured a different one.
        "archived_bucket": case["current_bucket"],
        "blocker": case["blocker"],
        "bucket_changed_since_batch2": case.get("bucket_changed_since_batch2", False),
        "stage_derived_status": {stage: {"status": "unexecuted"} for stage in STAGE_ORDER},
        "deepest_executed_stage": None,
    }
    if measured:
        row["blocker_current"] = label["reason"]
        row["blocker_superseded_by"] = label["label_source"]
        row["blocker_measured_by"] = label["measured_by"]
    ct_rows.append(row)
ct_rows.sort(key=lambda row: int(row["id"][2:]))

# The newly-decidable list and its companion reasons map, both derived from the
# same two records (card T00's triage judgment, filtered by the manifest's own
# unmapped set; the CT lanes' and T00's own reasons through the resolution rule).
# Card GEN-CTREASONS, follow-on `D02-GEN-CTREASONS`: the map used to be hand-kept
# commentary this generator dropped on every run, so nothing could refuse a row
# that had gone stale.
newly_decidable = ct_stage_summary.newly_decidable_ids(ROOT, manifest)
ct_reasons = ct_stage_summary.newly_decidable_reasons(ROOT, manifest)
assert list(ct_reasons) == newly_decidable, (list(ct_reasons), newly_decidable)
# Structural, and the direction that was the finding: no row may describe a case
# the manifest reports as mapped.
assert not set(ct_reasons) - set(unmapped_ids), sorted(set(ct_reasons) - set(unmapped_ids))
assert all(not case_id.startswith("_") for case_id in ct_reasons), sorted(ct_reasons)

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
        "its invocation count, its exact target_os gate and the external tools its runner cannot "
        "start without. "
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
    # The tool axis is separate from the host axis: the legacy_c route's stages
    # run on any target and only where Clang exists (card CT-CTWIN).
    "tool_scope": ct_summary["tool_scope"],
    "tool_gates_load_bearing": sorted(tool_gates),
    "stage_histogram": {stage: histogram[stage] for stage in STAGE_ORDER},
    "depth_profile": depth_profile,
    "case_depth": ct_summary["case_depth"],
    "observation_counts": observation_counts,
    # The per-CT rows were built and then only counted; T00.9 asked for them in
    # the record, so they are emitted.
    "ct_rows": ct_rows,
    "executed_totals_from_t00": coverage["executed_totals"],
    "unmapped_bucket_counts": bucket_counts,
    # Which record decided each current label: a CT lane's own `unmapped_reasons`
    # row, or `t00-archive` for a case no lane has re-measured (card CT-LABELS).
    "unmapped_label_sources": {case: entry["label_source"] for case, entry in labels.items()},
    "newly_decidable_not_mapped": newly_decidable,
    # The companion prose of the list above, derived from the same records since
    # card GEN-CTREASONS (follow-on `D02-GEN-CTREASONS`). It was hand-maintained
    # commentary this generator dropped on every regeneration and a later step
    # re-added, which is why one of its rows explained why CT39 was unmapped for
    # four waves after wave 8 mapped it. Its keys are now the list's own ids by
    # construction — so a row can neither describe a case the manifest maps nor
    # a case the list does not name — and each value is that case's resolved
    # label and reason with the record that decided it published beside them.
    "newly_decidable_not_mapped_reasons": {
        "_derivation": (
            "derived by docs/reports/remaining-d02/build_matrix.py through "
            "scripts/ct_stage_summary.py::newly_decidable_reasons (card GEN-CTREASONS, "
            "follow-on D02-GEN-CTREASONS): one row per id in newly_decidable_not_mapped, "
            "carrying that case's resolved bucket label and reason — the newest CT lane "
            "record that measured it, else card T00's frozen archive — and the record that "
            "decided them. A hand edit here is overwritten on the next regeneration, and "
            "scripts/ct_stage_summary.py --check refuses a committed row the derivation "
            "cannot produce as well as a derived row the file does not carry. The "
            "carry-forward step docs/reports/wave16-corpus/readd_ct_reasons.py is retired by "
            "this key existing: its own assertion refuses to run once the generator emits it."
        ),
        **ct_reasons,
    },
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
        "Only CT03, CT17, CT26 and CT34 carry the harness-gated sanitized run — CT34 since card "
        "CT34-SANITIZED, which this note had not been updated for — and the harness refuses an "
        "`executed` sanitized claim from any other case.",
        "known_manifest_overstatements is T00's audit at revision ea2af0f, retained as the "
        "record of what schema 2 repaired; its O1/O2/O3/O4/O7 findings are the ones this "
        "generator's inputs no longer allow.",
        "The unmapped triage rows are recorded analysis, not stage evidence. The counted set is "
        "the manifest's own unmapped set (card HYGIENE-14); each row's current_bucket is the "
        "newest CT lane record that measured that case, else T00's archived audit, with "
        "current_bucket_source naming which and archived_bucket keeping T00's text beside it "
        "(card CT-LABELS). A row whose source is a lane also carries blocker_current and "
        "blocker_measured_by; a row on t00-archive carries only T00's frozen blocker, which for "
        "such a case has not been re-verified against the current implementation.",
        "Thirty of the mapped cases are QCore source; CT40, CT43 and CT45 are C twins analyzed "
        "through the unrestricted-C gateway on the legacy_c route (card CT-CTWIN, joined by "
        "CT40 in the wave-17 CT lane). That is a "
        "different artifact and a different property — a resource projection over declared "
        "contracts, not a source-language check — and their Unknown verdicts are what their "
        "scenarios ask for, each with a control the gateway checks so the refusal is "
        "attributable. None of them adds native, sanitized or proof evidence, and the route "
        "structurally cannot: it analyzes C rather than emitting any. CT40 additionally carries "
        "two controls pinned for NOT moving the verdict — `_Atomic` removed, and the release "
        "removed — which is what bounds its mapping to the publication protocol its `expected` "
        "text names rather than to the atomicity or the premature free in its title.",
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

# The seventeen R/W rows are recorded analysis owned by the integrator, not
# counts this script can take. They are carried verbatim from the committed
# matrix and revalidated here against the closed class vocabulary above, so a
# hand edit that invents a status word or drops an evidence class refuses
# instead of being written.
requirements = carried("requirements")

for row in requirements:
    assert set(row["support"]) == set(EVIDENCE_CLASSES), row["id"]
    for cls, cell in row["support"].items():
        assert cell["status"] in CLASS_VOCABULARY, (row["id"], cls, cell["status"])

# ---------------------------------------------------------------- unverified specs
unverified_specs = carried("unverified_specs")
header = carried("header")
# The class vocabulary published in the header is this script's own validator.
# Carrying the header verbatim would otherwise let a hand edit widen the
# vocabulary the emitted rows are checked against; drift refuses instead.
assert header["support_status_vocabulary"] == CLASS_VOCABULARY, (
    "header.support_status_vocabulary in "
    f"{PREVIOUS} has drifted from build_matrix.py's CLASS_VOCABULARY"
)

# --------------------------------------------------------------------- assembly
section_ownership = {
    "note": "Card D02-GEN-REST. Every top-level key of this file is labelled here, so no section "
    "is silently hand-maintained. `derived` sections are recomputed by the generator on every "
    "run and a hand edit to them is overwritten; `hand_maintained` sections are carried verbatim "
    "from the previous matrix, which is why a regeneration no longer reverts a later "
    "reconciliation; `integrator_supplied` numbers are measurements the generator cannot take.",
    "generator": "docs/reports/remaining-d02/build_matrix.py",
    "carried_from": str(PREVIOUS.relative_to(ROOT)) if PREVIOUS.is_relative_to(ROOT) else str(PREVIOUS),
    "vocabulary": OWNERSHIP_VOCABULARY,
    "sections": {
        key: {"owner": owner, "source": source} for key, (owner, source) in SECTION_OWNERS.items()
    },
    "fields": {
        key: {"owner": owner, "source": source} for key, (owner, source) in FIELD_OWNERS.items()
    },
}

carried_versions = carried("versions")
carried_corpus = carried("corpus")
carried_validation = carried("validation")

matrix = {
    "schema_version": 1,
    "card": "D02",
    "generated": "2026-09-07",
    "report": "docs/reports/remaining-d02/README.md",
    "acceptance_record": "docs/reviews/r0_3/acceptance.json",
    "section_ownership": section_ownership,
    "header": header,
    "versions": {
        # Prose fields are carried; the two version facts the tree can answer
        # are recomputed over them.
        **carried_versions,
        "qcore_schemas_rust_api": schemas_version,
        "provider_packages": provider_packages,
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
        "named_followups": len(carried("named_followups")),
    },
    "derivation": {
        "rule": "no count in a section this file marks `derived` is hand-copied from a status "
        "ledger; each is read from the manifest or validated run record named beside it. "
        "Sections marked `hand_maintained` are recorded analysis carried verbatim from the "
        "previous matrix, and `integrator_supplied` numbers name whether they were measured for "
        "this run or carried - see section_ownership.",
        "sources": {
            "ct_stage_evidence": "docs/reports/remaining-t00/coverage.json",
            "ct_inventory": "docs/reevaluation/ct_manifest.json",
            "checkpoint_and_corpus_reconciliation": "docs/reports/remaining-b00/ledger.json",
            "contract_corpus": "contract/qcore/*/*.qcore on disk",
            "workspace_membership": "Cargo.toml, providers/Cargo.toml",
            "schema_version": "crates/qcore-schemas/Cargo.toml",
            "provider_package_versions": "providers/*/provider-template.json",
            "lean_audit_files": "proof/Audit*.lean",
            "gate_results": "--gate PATH, else carried from the previous matrix",
            "carried_sections": "the previous matrix named in section_ownership.carried_from",
        },
    },
    "unverified_specs": unverified_specs,
    "requirements": requirements,
    "ct_conformance": ct_section,
    "corpus": {
        # The measured audit figures are carried; the four facts read from the
        # tree and the B00 ledger are recomputed over them.
        **carried_corpus,
        "contract_modules": contract_modules,
        "contract_modules_pinned_by": "compiler/tests/qcore_contract_corpus.rs",
        "contract_clause_pin": b00["contract_corpus"]["clause_pin_table"],
        "contract_modules_per_directory": b00["contract_corpus"]["per_directory"],
        "libevent_submodule_pin": b00["submodule"]["declared_pin_sha"],
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
        **{key: carried_validation[key] for key in carried_validation},
        **{key: GATE[key] for key in GATE_KEYS},
        "measurement_provenance": GATE_PROVENANCE,
        "lean_audits": len(lean_audit_files),
        "lean_audit_files": lean_audit_files,
        "full_language_conformance": False,
    },
    "named_followups": carried("named_followups"),
    "updated": carried("updated"),
}

# Every emitted section must be labelled exactly once, so a new section cannot
# arrive without an owner and a stale label cannot outlive its section.
assert set(section_ownership["sections"]) == set(matrix), (
    sorted(set(matrix) - set(section_ownership["sections"])),
    sorted(set(section_ownership["sections"]) - set(matrix)),
)
for key, (owner, _) in SECTION_OWNERS.items():
    assert owner in OWNERSHIP_VOCABULARY or owner == "mixed", (key, owner)
for path, (owner, _) in FIELD_OWNERS.items():
    section, field = path.split(".", 1)
    assert SECTION_OWNERS[section][0] == "mixed", path
    assert field == "*" or field in matrix[section], path
for section, (owner, _) in SECTION_OWNERS.items():
    if owner != "mixed":
        continue
    assert any(path.startswith(section + ".") for path in FIELD_OWNERS), section
    assert f"{section}.*" in FIELD_OWNERS, f"{section} has no catch-all field owner"

OUT.parent.mkdir(parents=True, exist_ok=True)
# The committed matrix has been `indent=1, ensure_ascii=False` since the original
# D02 wave, and this writer's `indent=2, ensure_ascii=True` never matched it: the
# committed file carries literal em dashes, so the ASCII escaping was plainly
# never applied to it. Every "adopt the regenerated matrix" commit therefore
# re-serialised the generator's output by hand, and a regeneration adopted
# verbatim would have reformatted all 11,798 lines and buried its real change.
# Card HYGIENE-16 moves the writer to the committed byte form rather than the
# file to the writer's, so a regeneration is now reviewable as its own diff.
OUT.write_text(
    json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
)
print("wrote", OUT)
print("carried hand-maintained sections from", PREVIOUS)
print("gate provenance:", GATE_PROVENANCE)
print("requirements", matrix["dimensions"]["requirements"])
print("cells", matrix["dimensions"]["requirement_evidence_cells"])
print("ct rows", matrix["dimensions"]["ct_rows"], "stages", matrix["dimensions"]["ct_stage_names"])
print("histogram native_execution", ct_section["stage_histogram"]["native_execution"])
print("depth", depth_profile)
print("observations", observation_counts)
print("buckets", bucket_counts)
print("named followups", matrix["dimensions"]["named_followups"])
print("lean audits", matrix["validation"]["lean_audits"])
print("provider packages", provider_packages)
print("contract modules", contract_modules)
