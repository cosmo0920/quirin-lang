#!/usr/bin/env python3
"""link_premise.py RESOLUTION -- CG22-CALLBACK-CANDIDATE-LINK-PREMISE: write
contract/evaluations/twelve-home/link-premise.json, the twelve-home profile's
stated link premise as data for human approval: the eight-home map it
extends, the four homes it adds (with the invocations that hand them to
`bsearch`/`qsort`), and, for every one of the twelve symbols, what the
`quirin-link-resolution/1` artifact that `quirin-audit prepare clang-build
--link` wrote for this card's preparation (artifacts/prep.sh) records about
the fluent-bit executable's static link: rule 40's decision, the chosen
definition (archive member, binding, visibility, executable or shared
object), how many linked definitions exist, dynamic relocations against the
name, and the linked objects that reference it.

The evidence supports the premise; it does not replace it. The pinned corpus
route reads only the stated map (rule 40 is the `current` route's), and the
artifact establishes neither the call-site type nor the calling convention.
"""
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
EIGHT = "contract/evaluations/eight-home/link-map.json"
TWELVE = "contract/evaluations/twelve-home/link-map.json"
OUT = ROOT / "contract/evaluations/twelve-home/link-premise.json"
INVOCATIONS = {
    "rd_kafka_metadata_broker_cmp": [
        "librdkafka/rdkafka_aux.c::rd_kafka_Node_new_from_brokers (bsearch)"],
    "rd_kafka_metadata_broker_internal_cmp": [
        "librdkafka/rdkafka_aux.c::rd_kafka_Node_new_from_brokers (bsearch)",
        "librdkafka/rdkafka_assignor.c::ut_populate_internal_topic_metadata (bsearch)"],
    "rd_strcmp3": [
        "librdkafka/rdkafka_range_assignor.c::rd_kafka_topic_assignment_state_rack_search (bsearch)",
        "librdkafka/rdkafka_sticky_assignor.c::rd_kafka_partition_internal_find_rack (bsearch)"],
    "rd_kafka_assignor_topic_cmp": [
        "librdkafka/rdkafka_roundrobin_assignor.c::rd_kafka_roundrobin_assignor_assign_cb (qsort)"],
}


def sha(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    resolution_path = pathlib.Path(argv[1])
    raw = resolution_path.read_bytes()
    resolution = json.loads(raw)
    if resolution.get("schema") != "quirin-link-resolution/1":
        print("not a quirin-link-resolution/1 artifact", file=sys.stderr)
        return 1
    eight = json.loads((ROOT / EIGHT).read_text())
    twelve = json.loads((ROOT / TWELVE).read_text())
    base_names = {entry["function"] for entry in eight["symbols"]}
    labels = {entry["id"]: entry["label"] for entry in resolution["inputs"]}
    records = {}
    for symbol in resolution["symbols"]:
        records.setdefault(symbol["name"], []).append(symbol)
    evidence = []
    failures = []
    for entry in twelve["symbols"]:
        name = entry["function"]
        found = records.get(name, [])
        if len(found) != 1:
            failures.append(f"{name}: {len(found)} records in the artifact")
            continue
        record = found[0]
        definition = record["definition"]
        item = {
            "function": name,
            "stated_home_unit": entry["home_unit"],
            "added_by_this_profile": name not in base_names,
            "rule_40_decision": record["decision"],
            "rule_40_reason": record["reason"],
            "linked_home_unit": record["home_unit"],
            "definition": {
                "member": definition["file"],
                "in": definition["in"],
                "binding": definition["binding"],
                "visibility": definition["visibility"],
                "type": definition["type"],
            },
            "linked_definitions": len(record["definitions"]),
            "executable_dynamic_relocations": record["executable"]["dynamic_relocations"],
            "referenced_by": sorted(labels[index] for index in record["referenced_by"]),
        }
        if name in INVOCATIONS:
            item["handed_to_invocations"] = INVOCATIONS[name]
        evidence.append(item)
        stated = entry["home_unit"].split("/")[-1]
        linked = (record["home_unit"] or "").split("/")[-1]
        if (record["decision"] != "derived" or stated != linked
                or definition["in"] != "executable" or len(record["definitions"]) != 1
                or record["executable"]["dynamic_relocations"] != 0):
            failures.append(f"{name}: the artifact does not support the stated home")
    added = [entry for entry in twelve["symbols"] if entry["function"] not in base_names]
    document = {
        "description": (
            "The twelve-home profile's stated link premise (rule 4c), written as data for human "
            "approval by docs/reports/cg22-callback-candidate-link-premise/artifacts/"
            "link_premise.py. The premise text is the eight-home map's, unchanged; the profile "
            "states it for four more symbols. The evidence is one static link of the fluent-bit "
            "executable at the source revision on this card's preparation; it supports the "
            "premise and does not replace it (the pinned corpus route reads only the stated map)."),
        "premise": twelve["premise"],
        "extends": {"path": EIGHT, "sha256": sha(EIGHT)},
        "map": {"path": TWELVE, "sha256": sha(TWELVE)},
        "added_homes": added,
        "approval": ("stated, not proved: the four added homes are a premise a human approves "
                     "(card CG22-CALLBACK-CANDIDATE-LINK-PREMISE asks for that decision)"),
        "link_evidence": {
            "source": ("quirin-audit prepare clang-build --link (base 974434463, system Clang 21) "
                       "of Fluent Bit 66910c10; artifacts/prep.sh"),
            "artifact_schema": resolution["schema"],
            "artifact_sha256": hashlib.sha256(raw).hexdigest(),
            "executable_sha256": resolution["executable"]["sha256"],
            "executable": {key: resolution["executable"]["image"][key]
                           for key in ("is_executable", "pie")},
            "compile_database_sha256": resolution["compile_database"]["sha256"],
            "map_sha256": resolution["link"]["map_sha256"],
            "relinked_equal": resolution["link"]["relinked_equal"],
            "linker": resolution["toolchain"]["linker_version"].splitlines()[0],
            "driver": resolution["toolchain"]["driver_version"].splitlines()[0],
            "not_established": ("the call-site function type and calling convention, and "
                                "interposition by a preloaded object at run time; the executable "
                                "is linked with --export-dynamic"),
            "symbols": evidence,
        },
    }
    OUT.write_text(json.dumps(document, indent=2) + "\n")
    print(OUT.relative_to(ROOT), sha(OUT.relative_to(ROOT)))
    for failure in failures:
        print("REFUSED:", failure, file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
