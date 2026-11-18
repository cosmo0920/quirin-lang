#!/usr/bin/env python3
"""twelve_home.py -- CG22-CALLBACK-CANDIDATE-LINK-PREMISE: write the
twelve-home profile's stated link map (contract/evaluations/twelve-home/
link-map.json) as the eight-home map, byte for byte in its premise and its
eight symbols, plus the four comparator homes; and the two measurement
inputs of this card's worktree-private preparation (artifacts/inputs.*.json),
which differ only in the link map.

    twelve_home.py map               write the profile's link map
    twelve_home.py inputs DB1 DB2    write inputs.eight-home.json and
                                     inputs.twelve-home.json with the digests
                                     of the prepared Fluent Bit and Monkey
                                     compile databases
"""
import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
HERE = pathlib.Path(__file__).resolve().parent
EIGHT = "contract/evaluations/eight-home/link-map.json"
TWELVE = "contract/evaluations/twelve-home/link-map.json"
ADDED = [
    {"function": "rd_kafka_metadata_broker_cmp", "home_unit": "librdkafka/rdkafka_metadata.c"},
    {"function": "rd_kafka_metadata_broker_internal_cmp",
     "home_unit": "librdkafka/rdkafka_metadata.c"},
    {"function": "rd_strcmp3", "home_unit": "librdkafka/rdstring.c"},
    {"function": "rd_kafka_assignor_topic_cmp", "home_unit": "librdkafka/rdkafka_assignor.c"},
]


def sha(path):
    return hashlib.sha256((ROOT / path).read_bytes()).hexdigest()


def write_map():
    data = json.loads((ROOT / EIGHT).read_text())
    data["symbols"] = data["symbols"] + ADDED
    target = ROOT / TWELVE
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(data, indent=2) + "\n")
    print(TWELVE, sha(TWELVE))


def write_inputs(fluent_bit_db, monkey_db):
    for name, link_map in (("eight-home", EIGHT), ("twelve-home", TWELVE)):
        inputs = {
            "schema": "quirin-measure-inputs/1",
            "description": (
                "CG22-CALLBACK-CANDIDATE-LINK-PREMISE worktree-private preparation of the "
                "pinned corpus route (artifacts/prep.sh: quirin-audit prepare clang-build --link "
                "with the base 974434463 binary and system Clang 21 of the local Fluent Bit "
                "clone at 66910c10, prepare monkey, providers/package.py build). --inputs-base "
                "and --repository are this card's worktree. Route `corpus` under the "
                f"{name} profile's stated link map; the two inputs files differ only in it."),
            "routes": [{
                "name": "corpus",
                "profile": name,
                "kind": "pinned-corpus",
                "source_commit": "66910c10",
                "inputs_root": "private/corpus-inputs",
                "compile_databases": [
                    {"path": "c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json",
                     "sha256": fluent_bit_db},
                    {"path": "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json",
                     "sha256": monkey_db},
                ],
                "mirror_trees": ["c-gateway-wave-pin/fluent-bit-clang-source",
                                 "c-gateway-wave-pin/monkey", "provider-packages-wave20"],
                "link_map": {"path": link_map, "sha256": sha(link_map)},
                "clang": "/usr/lib/llvm-21/bin/clang",
                "tracked": ["contract/profiles/fluent-bit/audit.json",
                            "contract/profiles/fluent-bit/profile.json",
                            "contract/profiles/fluent-bit/member-facts-scope.json"],
            }],
        }
        (HERE / f"inputs.{name}.json").write_text(json.dumps(inputs, indent=2) + "\n")


def main(argv):
    if argv[1:2] == ["map"]:
        write_map()
    elif argv[1:2] == ["inputs"] and len(argv) == 4:
        write_inputs(argv[2], argv[3])
    else:
        print(__doc__, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
