"""Write contract/evaluations/twelve-home/{evaluation,baseline}.json from the
eight-home record and a summarize output (argv[1]): same route, source,
untracked inputs and contract set; its own link map and link premise."""
import copy, hashlib, json, os, sys
W = '/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a83f807263a0b65f2'
os.chdir(W)
REV = '9744344634a7c6e7141d8cd11c87ce0ad8dd9114'
SOURCE = 'docs/reports/cg22-callback-candidate-link-premise/README.md'


def sha(path):
    return hashlib.sha256(open(path, 'rb').read()).hexdigest()


figures = json.load(open(sys.argv[1]))
baseline = {
    "schema": "quirin-evaluation-baseline/1",
    "profile": "twelve-home",
    "measured_revision": REV,
    "source": SOURCE,
    "figures": figures,
    "artifacts": [],
}
bpath = 'contract/evaluations/twelve-home/baseline.json'
open(bpath, 'w').write(json.dumps(baseline, indent=2) + "\n")

eight = json.load(open('contract/evaluations/eight-home/evaluation.json'))
record = copy.deepcopy(eight)
record["name"] = "twelve-home"
record["description"] = (
    "The eight-home corpus route (the pinned Fluent Bit corpus, 185 units of the fluent-bit "
    "source set's audit.json selection, null and outer projections on, no source contracts or "
    "census) on the same inputs under a wider stated link premise: the eight-home map plus four "
    "external comparator homes (rd_kafka_metadata_broker_cmp, rd_kafka_metadata_broker_internal_cmp, "
    "rd_strcmp3, rd_kafka_assignor_topic_cmp) that rule 4c may then import at their callers' "
    "bsearch/qsort invocations (card CG22-CALLBACK-CANDIDATE-LINK-PREMISE). The added homes and "
    "their link evidence are in link-premise.json; the premise is stated, not proved.")
record["route"]["command"] = record["route"]["command"].replace(
    "QCORE_CROSS_UNIT_LINK_MAP=<link map>",
    "QCORE_CROSS_UNIT_LINK_MAP=contract/evaluations/twelve-home/link-map.json")
tracked = []
for entry in eight["inputs"]["tracked"]:
    if "link map" in entry["role"]:
        continue
    tracked.append(entry)
tracked.append({
    "path": "contract/evaluations/twelve-home/link-map.json",
    "sha256": sha('contract/evaluations/twelve-home/link-map.json'),
    "role": "cross-unit link map (QCORE_CROSS_UNIT_LINK_MAP): contract/evaluations/eight-home/link-map.json's premise and eight symbols plus four comparator homes",
})
tracked.append({
    "path": "contract/evaluations/twelve-home/link-premise.json",
    "sha256": sha('contract/evaluations/twelve-home/link-premise.json'),
    "role": "stated premise for human approval: the added homes and their static-link evidence",
})
record["inputs"]["tracked"] = tracked
record["baseline"] = {"path": bpath, "sha256": sha(bpath)}
record["baseline_history"] = [{
    "sha256": sha(bpath),
    "measured_revision": REV,
    "reason": (
        "first baseline of the profile: base 974434463 measured by card "
        "CG22-CALLBACK-CANDIDATE-LINK-PREMISE with quirin-audit measure on that card's "
        "worktree-private preparation of the pinned corpus inputs (not the build/cycle19 "
        "preparation the untracked inputs name), beside eight-home on the same preparation; "
        "a landing measurement on the recorded inputs re-records it"),
    "source": SOURCE,
}]
record["evidence_version"] = {"analyzer_revision": REV, "semantic_evidence": "v2 (unchanged)"}
order = ["schema", "name", "description", "policy", "route", "source", "inputs", "contract_set",
         "baseline", "baseline_history", "evidence_version"]
record = {key: record[key] for key in order}
open('contract/evaluations/twelve-home/evaluation.json', 'w').write(json.dumps(record, indent=2) + "\n")
print("written")
