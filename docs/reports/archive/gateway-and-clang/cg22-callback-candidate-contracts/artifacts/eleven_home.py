#!/usr/bin/env python3
"""eleven_home.py -- CG22-CALLBACK-CANDIDATE-CONTRACTS: write the sensitivity
variant of the corpus inputs, the eight-home link map plus the three external
comparators the follow-up names as Safe in another unit, as
artifacts/link-map.eleven-home.json and artifacts/inputs.eleven-home.json.
Rule 4c's stated premise extended by three symbols; not the eight-home
profile's input, and not a change to it."""
import hashlib
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
HERE = pathlib.Path(__file__).resolve().parent
EXTRA = [
    {"function": "rd_kafka_metadata_broker_cmp", "home_unit": "librdkafka/rdkafka_metadata.c"},
    {"function": "rd_kafka_metadata_broker_internal_cmp",
     "home_unit": "librdkafka/rdkafka_metadata.c"},
    {"function": "rd_strcmp3", "home_unit": "librdkafka/rdstring.c"},
]


def main():
    base = json.loads((ROOT / "docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json").read_text())
    base["symbols"] = base["symbols"] + EXTRA
    text = json.dumps(base, indent=2) + "\n"
    (HERE / "link-map.eleven-home.json").write_text(text)
    inputs = json.loads((HERE / "inputs.json").read_text())
    inputs["routes"][0]["link_map"] = {
        "path": "docs/reports/cg22-callback-candidate-contracts/artifacts/link-map.eleven-home.json",
        "sha256": hashlib.sha256(text.encode()).hexdigest(),
    }
    inputs["description"] += (
        " SENSITIVITY VARIANT: the eight-home link map plus the three comparator homes "
        "rd_kafka_metadata_broker_cmp, rd_kafka_metadata_broker_internal_cmp "
        "(librdkafka/rdkafka_metadata.c) and rd_strcmp3 (librdkafka/rdstring.c), rule 4c's "
        "stated premise extended by three symbols; not the eight-home profile's input."
    )
    (HERE / "inputs.eleven-home.json").write_text(json.dumps(inputs, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
